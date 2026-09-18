use crate::services::backoff::{exp_backoff, is_retryable_status, retry_after_delay};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::blocking::Client;
use reqwest::header::RANGE;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize)]
pub struct WebdavFile {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub last_modified: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebdavProbeResult {
    pub ok: bool,
    pub latency_ms: u64,
    pub server_header: Option<String>,
    pub status_code: Option<u16>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct WebdavClient {
    pub client: Client,
    /// 大体积传输专用客户端（整文件下载、DB 快照上传）：保留连接超时，但**不设总超时**——
    /// reqwest 的总超时覆盖整个响应体，大文件传输会被 60s 掐断。
    /// 挂死的连接改由 TCP keepalive 探测（blocking 客户端没有 read_timeout，只有这一层能兜住
    /// "连上了但不再传字节"）。
    pub bulk_client: Client,
    pub base_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl WebdavClient {
    pub fn new(base_url: String, username: Option<String>, password: Option<String>) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|e| {
                // 退回到无超时的 Client 会让请求可能永久挂住，只有在 TLS 后端缺失时才会走到这里；
                // 保留可用性，但必须留痕，否则线上表现为"点了没反应"。
                tracing::error!("无法构建带超时的 HTTP 客户端，退回默认客户端: {}", e);
                Client::new()
            });
        let bulk_client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            // 不用 timeout()：它覆盖整个响应体，GB 级快照/曲库文件会被掐断。
            // keepalive 让"连上后对端不再发字节"的死连接由 OS 探测出来。
            .tcp_keepalive(Duration::from_secs(30))
            .tcp_keepalive_interval(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|e| {
                tracing::error!("无法构建大文件传输 HTTP 客户端，退回默认客户端: {}", e);
                Client::new()
            });
        Self {
            client,
            bulk_client,
            base_url: base_url.trim_end_matches('/').to_string(),
            username,
            password,
        }
    }

    /// 把 reqwest 错误翻译成用户可读文案（错误提示会经 AppError 透出到前端 toast）。
    pub fn describe_reqwest_error(e: &reqwest::Error) -> String {
        if e.is_timeout() {
            "连接超时：服务器长时间未响应".to_string()
        } else if e.is_connect() {
            "无法连接服务器：请检查网络或服务器地址".to_string()
        } else {
            format!("网络请求失败: {}", e)
        }
    }

    /// 拼接 base_url 与子路径。统一替代原先散落各处的 unwrap 链——
    /// base_url 来自用户输入、subpath 来自远端 XML，非法输入应返回错误而不是 panic。
    pub fn build_url(&self, subpath: &str) -> Result<String, String> {
        if subpath.starts_with("http://") || subpath.starts_with("https://") {
            return Ok(subpath.to_string());
        }
        let base = reqwest::Url::parse(&format!("{}/", self.base_url))
            .map_err(|e| format!("无效的 WebDAV 服务器地址 \"{}\": {}", self.base_url, e))?;
        let joined = base
            .join(subpath)
            .map_err(|e| format!("无效的 WebDAV 文件路径 \"{}\": {}", subpath, e))?;
        Ok(joined.to_string())
    }

    // helper to add auth
    fn apply_auth(
        &self,
        req: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            req.basic_auth(u, Some(p))
        } else {
            req
        }
    }

    /// [MA3 A3-1] 测试 WebDAV 服务器连接连通性与权限
    pub fn probe_connection(&self) -> WebdavProbeResult {
        let start = std::time::Instant::now();
        let url = if self.base_url.is_empty() {
            "http://localhost".to_string()
        } else {
            format!("{}/", self.base_url)
        };

        let req = self
            .client
            .request(
                reqwest::Method::from_bytes(b"PROPFIND").unwrap_or(reqwest::Method::GET),
                &url,
            )
            .header("Depth", "0");
        let req = self.apply_auth(req);

        match req.send() {
            Ok(resp) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let status = resp.status();
                let server = resp
                    .headers()
                    .get("server")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string());

                if status.is_success() || status.as_u16() == 207 {
                    WebdavProbeResult {
                        ok: true,
                        latency_ms,
                        server_header: server,
                        status_code: Some(status.as_u16()),
                        error: None,
                    }
                } else {
                    let err_msg = match status.as_u16() {
                        401 => "认证失败：用户名或密码错误 (401 Unauthorized)".to_string(),
                        403 => "拒绝访问：权限不足 (403 Forbidden)".to_string(),
                        404 => "路径不存在：请检查服务器地址路径 (404 Not Found)".to_string(),
                        _ => format!("服务器返回状态码: {}", status),
                    };
                    WebdavProbeResult {
                        ok: false,
                        latency_ms,
                        server_header: server,
                        status_code: Some(status.as_u16()),
                        error: Some(err_msg),
                    }
                }
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let err_str = e.to_string();
                let friendly_err = if e.is_timeout() {
                    "连接超时：请检查网络或服务器地址是否可达".to_string()
                } else if e.is_connect() {
                    "连接失败：无法连接到目标服务器或域名解析失败".to_string()
                } else {
                    format!("网络请求失败: {}", err_str)
                };
                WebdavProbeResult {
                    ok: false,
                    latency_ms,
                    server_header: None,
                    status_code: None,
                    error: Some(friendly_err),
                }
            }
        }
    }

    pub fn propfind(&self, subpath: &str) -> Result<Vec<WebdavFile>, String> {
        let url = self.build_url(subpath)?;

        let req = self
            .client
            .request(
                reqwest::Method::from_bytes(b"PROPFIND").unwrap_or(reqwest::Method::GET),
                &url,
            )
            .header("Depth", "1");

        let req = self.apply_auth(req);

        let resp = req
            .send()
            .map_err(|e| WebdavClient::describe_reqwest_error(&e))?;
        if !resp.status().is_success() {
            let s = resp.status().as_u16();
            let msg = match s {
                401 => "认证失败：用户名或密码错误 (401)".to_string(),
                403 => "权限不足 (403)".to_string(),
                404 => "目录不存在 (404)".to_string(),
                _ => format!("PROPFIND 失败 (HTTP {})", s),
            };
            return Err(msg);
        }

        let xml = resp.text().map_err(|e| e.to_string())?;
        // 解析失败必须报错而非返回半截列表：调用方据此置 scan_failed，
        // 否则截断的 PROPFIND 会让"本次未遍历到"的文件被批量误标 missing。
        let parsed = self.parse_propfind(&xml)?;

        let req_url_path = reqwest::Url::parse(&url)
            .map_err(|e| format!("无效的请求地址: {}", e))?
            .path()
            .to_string();

        let mut results = Vec::new();
        for file in parsed {
            // 跳过代表当前目录自身的条目；路径无法解析的条目直接丢弃（不 panic）
            let file_url_path = match self.build_url(&file.path) {
                Ok(u) => u,
                Err(_) => continue,
            };
            let Ok(file_url) = reqwest::Url::parse(&file_url_path) else {
                continue;
            };

            if file_url.path().trim_end_matches('/') == req_url_path.trim_end_matches('/') {
                continue;
            }
            results.push(file);
        }

        Ok(results)
    }

    /// 用真实文件的 URL 发一次 `Range: bytes=0-0` 小请求探测分段下载支持。
    /// 206 = 支持；200 = 服务器明确忽略 Range（不支持）；
    /// 认证失败 / 网络错误等返回 Err（带分类后的可读文案），调用方不落库。
    pub fn probe_range_support(&self, file_url: &str) -> Result<bool, String> {
        let req = self.apply_auth(self.client.get(file_url).header(RANGE, "bytes=0-0"));
        let resp = match req.send() {
            Ok(r) => r,
            Err(e) => return Err(WebdavClient::describe_reqwest_error(&e)),
        };
        match resp.status().as_u16() {
            206 => Ok(true),
            200 => Ok(false),
            401 => Err("认证失败：用户名或密码错误 (401)".to_string()),
            403 => Err("权限不足：对该文件没有访问权限 (403)".to_string()),
            404 => Err("文件不存在 (404)".to_string()),
            s => Err(format!("服务器返回异常状态 (HTTP {})", s)),
        }
    }

    fn parse_propfind(&self, xml: &str) -> Result<Vec<WebdavFile>, String> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut files = Vec::new();

        let mut current_href = String::new();
        let mut current_len: u64 = 0;
        let mut current_is_dir = false;
        let mut current_mtime = String::new();

        let mut inside_tag = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);
                    inside_tag = local_name.to_string();

                    if inside_tag == "response" {
                        current_href = String::new();
                        current_len = 0;
                        current_is_dir = false;
                        current_mtime = String::new();
                    } else if inside_tag == "collection" {
                        current_is_dir = true;
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);
                    if local_name == "collection" {
                        current_is_dir = true;
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = String::from_utf8_lossy(e.as_ref()).into_owned();
                    match inside_tag.as_str() {
                        "href" => current_href = text,
                        "getcontentlength" => current_len = text.parse().unwrap_or(0),
                        "getlastmodified" => current_mtime = text,
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);

                    if local_name == "response" {
                        files.push(WebdavFile {
                            path: current_href.clone(),
                            is_dir: current_is_dir,
                            size: current_len,
                            last_modified: current_mtime.clone(),
                        });
                    }
                    inside_tag = String::new();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    // 关键：XML 中途出错（响应被截断、编码异常、服务端返回 HTML 错误页）
                    // 必须向上报错，让扫描判定为失败，不能把"已解析到的部分"当完整列表。
                    return Err(format!("PROPFIND 响应解析失败：{}", e));
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(files)
    }

    /// 用单个 GET 请求下载完整文件到指定本地路径（带认证）。
    /// 用于云端文件透明缓存与「服务器不支持 Range」时的整文件降级：
    /// 播放 WebDAV 歌曲时后台异步拉取完整文件，
    /// 下次播放同一首歌即可命中本地缓存，实现「零网络请求」秒开。
    /// 走 bulk_client（不设总超时，靠 TCP keepalive 探测死连接），大文件不会被 60s 掐断。
    /// 返回写入的字节数。
    pub fn download_to_file(&self, file_url: &str, dest: &Path) -> Result<u64, String> {
        // 只重试"还没开始传字节"的失败：状态码 429/5xx 与连接层错误。
        // 半路断开的响应体不能在这里重连（没有 Range 续传），交给上层的临时文件校验兜底。
        const DL_BASE: Duration = Duration::from_millis(500);
        const DL_MAX: Duration = Duration::from_secs(8);
        const DL_RETRY_AFTER_CAP: Duration = Duration::from_secs(15);
        let mut retries: u32 = 0;
        let mut resp = loop {
            let req = self.apply_auth(self.bulk_client.get(file_url));
            match req.send() {
                Ok(r) => {
                    let status = r.status();
                    if !is_retryable_status(status) {
                        break r;
                    }
                    if retries >= DOWNLOAD_MAX_RETRIES {
                        break r;
                    }
                    retries += 1;
                    let delay = retry_after_delay(r.headers(), DL_RETRY_AFTER_CAP)
                        .unwrap_or_else(|| exp_backoff(retries, DL_BASE, DL_MAX));
                    // 不打告警级日志：整首下载失败由调用方统一记录，这里只是常规重试
                    tracing::debug!(
                        "下载遇到 HTTP {}，{}ms 后第 {}/{} 次重试",
                        status,
                        delay.as_millis(),
                        retries,
                        DOWNLOAD_MAX_RETRIES
                    );
                    std::thread::sleep(delay);
                }
                Err(e) if (e.is_connect() || e.is_timeout()) && retries < DOWNLOAD_MAX_RETRIES => {
                    retries += 1;
                    let delay = exp_backoff(retries, DL_BASE, DL_MAX);
                    tracing::debug!(
                        "下载连接失败，{}ms 后第 {}/{} 次重试",
                        delay.as_millis(),
                        retries,
                        DOWNLOAD_MAX_RETRIES
                    );
                    std::thread::sleep(delay);
                }
                Err(e) => return Err(WebdavClient::describe_reqwest_error(&e)),
            }
        };
        if !resp.status().is_success() {
            let s = resp.status().as_u16();
            let msg = match s {
                401 => "下载失败：认证失败 (401)".to_string(),
                403 => "下载失败：权限不足 (403)".to_string(),
                _ => format!("下载失败: HTTP {}", s),
            };
            return Err(msg);
        }
        let mut file =
            File::create(dest).map_err(|e| format!("Failed to create cache file: {}", e))?;
        let bytes = resp
            .copy_to(&mut file)
            .map_err(|e| format!("Download write failed: {}", e))?;
        Ok(bytes)
    }

    /// MKCOL：创建远程目录（同步目录初始化、文件夹浏览器新建文件夹用）。
    /// 对已存在的目录返回 405，视为成功（幂等）。
    pub fn mkcol(&self, path_url: &str) -> Result<(), String> {
        let req = self.apply_auth(
            self.client
                .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), path_url),
        );
        let resp = req
            .send()
            .map_err(|e| format!("MKCOL request failed: {}", e))?;
        let status = resp.status();
        // 201 Created 或 405 Method Not Allowed（目录已存在）都视为成功
        if status.is_success() || status.as_u16() == 405 {
            Ok(())
        } else {
            Err(format!("MKCOL failed: HTTP {}", status))
        }
    }

    /// PUT：上传文件内容到指定 URL（上传 DB 快照用）。
    /// 走 bulk_client：快照可能有上百 MB，60s 总超时会把慢速上传掐断在半程。
    ///
    /// body 收 `Into<Body>` 而不是 `Vec<u8>`：上百 MB 至 GiB 级的快照读进内存再克隆一份，
    /// 峰值内存就是「两份完整数据库」，低内存设备上进程会被系统直接杀掉（CR-004）。
    /// 传 `File` 时 reqwest 按 metadata 推出 content-length 并边读边发。
    pub fn put_file<B: Into<reqwest::blocking::Body>>(
        &self,
        file_url: &str,
        body: B,
    ) -> Result<(), String> {
        let req = self.apply_auth(self.bulk_client.put(file_url).body(body.into()));
        let resp = req
            .send()
            .map_err(|e| format!("PUT request failed: {}", e))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("PUT failed: HTTP {}", resp.status()))
        }
    }

    /// MOVE：把远程 src 重命名/移动到 dst（RFC 4918 §9.10）。
    /// 用于「先 PUT 到临时名、再原子替换正式名」的上传流程。
    /// `Overwrite: T` 允许目标已存在时覆盖；201/204 视为成功。
    pub fn move_file(&self, src_url: &str, dst_url: &str) -> Result<(), String> {
        let req = self.apply_auth(
            self.client
                .request(reqwest::Method::from_bytes(b"MOVE").unwrap(), src_url)
                .header("Destination", dst_url)
                .header("Overwrite", "T"),
        );
        let resp = req
            .send()
            .map_err(|e| format!("MOVE request failed: {}", e))?;
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(format!("MOVE failed: HTTP {}", status))
        }
    }

    /// 读取小体积文本资源（校验和 sidecar 等），最多 max_bytes 字节。
    /// 404 返回 Ok(None)——调用方据此区分「资源不存在」与「读取失败」。
    pub fn fetch_text(&self, file_url: &str, max_bytes: u64) -> Result<Option<String>, String> {
        let req = self.apply_auth(self.client.get(file_url));
        let resp = req
            .send()
            .map_err(|e| WebdavClient::describe_reqwest_error(&e))?;
        let status = resp.status();
        if status.as_u16() == 404 || status.as_u16() == 403 {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(format!("GET failed: HTTP {}", status));
        }
        let mut buf = Vec::new();
        resp.take(max_bytes)
            .read_to_end(&mut buf)
            .map_err(|e| format!("读取响应失败: {}", e))?;
        Ok(Some(String::from_utf8_lossy(&buf).into_owned()))
    }

    /// DELETE：删除远程文件（清理旧快照用）。
    pub fn delete(&self, file_url: &str) -> Result<(), String> {
        let req = self.apply_auth(self.client.delete(file_url));
        let resp = req
            .send()
            .map_err(|e| format!("DELETE request failed: {}", e))?;
        // 204 No Content 或 404 Not Found 都视为成功
        let status = resp.status();
        if status.is_success() || status.as_u16() == 404 {
            Ok(())
        } else {
            Err(format!("DELETE failed: HTTP {}", status))
        }
    }
}

/// 分段读取的重试预算：最多 3 次、单次等待 100ms 起指数递增、封顶 1s（总等待约 0.7s）。
/// `read` 跑在解码线程上，等太久等于把播放卡死——宁可让这一首失败并由上层提示。
const RANGE_MAX_RETRIES: u32 = 3;
const RANGE_RETRY_BASE: Duration = Duration::from_millis(100);
const RANGE_RETRY_MAX: Duration = Duration::from_millis(1_000);
/// 采纳服务端 `Retry-After` 的上限：不超过它原样等待，超过它只等满该时长再重试。
const RANGE_RETRY_AFTER_CAP: Duration = Duration::from_secs(5);
/// 整文件下载的重试次数（只覆盖状态码与连接层失败，不覆盖断流续传）。
const DOWNLOAD_MAX_RETRIES: u32 = 3;

pub struct HttpRangeReader {
    client: Client,
    pub url: String,
    username: Option<String>,
    password: Option<String>,
    offset: u64,
    length: u64,
    current_resp: Option<reqwest::blocking::Response>,
    resp_offset: u64,
}

impl HttpRangeReader {
    pub fn new(webdav: &WebdavClient, url: String, length: u64) -> Self {
        Self {
            client: webdav.client.clone(),
            url,
            username: webdav.username.clone(),
            password: webdav.password.clone(),
            offset: 0,
            length,
            current_resp: None,
            resp_offset: 0,
        }
    }

    fn apply_auth(
        &self,
        req: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            req.basic_auth(u, Some(p))
        } else {
            req
        }
    }

    /// 消耗一次重试预算并等待。返回 false 表示预算已耗尽，调用方应当放弃。
    /// `delay_override` 用于服务端明确要求的等待（Retry-After），否则走指数退避。
    fn wait_before_retry(
        &mut self,
        retries: &mut u32,
        reason: &str,
        delay_override: Option<Duration>,
    ) -> bool {
        if *retries >= RANGE_MAX_RETRIES {
            return false;
        }
        *retries += 1;
        let delay = delay_override
            .unwrap_or_else(|| exp_backoff(*retries, RANGE_RETRY_BASE, RANGE_RETRY_MAX));
        tracing::warn!(
            "HttpRangeReader {} ({}), {}ms 后第 {}/{} 次重试",
            reason,
            self.url,
            delay.as_millis(),
            *retries,
            RANGE_MAX_RETRIES
        );
        self.current_resp = None;
        std::thread::sleep(delay);
        true
    }
}

impl Read for HttpRangeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.offset >= self.length {
            return Ok(0);
        }

        let mut retries: u32 = 0;
        loop {
            // Forward seek optimization (up to 256KB)
            if self.current_resp.is_some()
                && self.offset > self.resp_offset
                && self.offset - self.resp_offset <= 256 * 1024
            {
                let mut skip = self.offset - self.resp_offset;
                let mut dummy = [0u8; 8192];
                let mut success = true;
                while skip > 0 {
                    let to_read = std::cmp::min(skip, dummy.len() as u64) as usize;
                    match self
                        .current_resp
                        .as_mut()
                        .unwrap()
                        .read(&mut dummy[..to_read])
                    {
                        Ok(0) => {
                            success = false;
                            break;
                        }
                        Ok(n) => {
                            skip -= n as u64;
                            self.resp_offset += n as u64;
                        }
                        Err(_) => {
                            success = false;
                            break;
                        }
                    }
                }
                if !success {
                    self.current_resp = None;
                }
            }

            if self.current_resp.is_none() || self.offset != self.resp_offset {
                let range_val = format!("bytes={}-{}", self.offset, self.length - 1);
                let mut req = self.client.get(&self.url).header(RANGE, range_val);
                req = self.apply_auth(req);

                let resp = match req.send() {
                    Ok(r) => r,
                    Err(e) => {
                        // 连接层失败（DNS/TLS 握手/超时）通常是瞬时的：在同一 read() 内
                        // 退避重试，预算耗尽后沿用原有的错误文案向上抛。
                        if (e.is_connect() || e.is_timeout())
                            && self.wait_before_retry(&mut retries, "连接失败", None)
                        {
                            continue;
                        }
                        tracing::error!("HttpRangeReader fetch failed for url: {}", self.url);
                        return Err(io::Error::other(WebdavClient::describe_reqwest_error(&e)));
                    }
                };

                let status = resp.status();
                if status == reqwest::StatusCode::PARTIAL_CONTENT {
                    // 严格校验 Content-Range 起始偏移：服务端返回错误分段会导致解码错乱
                    if let Some(start) = resp
                        .headers()
                        .get(reqwest::header::CONTENT_RANGE)
                        .and_then(|v| v.to_str().ok())
                        .and_then(parse_content_range_start)
                    {
                        if start != self.offset {
                            tracing::error!(
                                "HttpRangeReader Content-Range mismatch for {}: expected offset {}, server sent {}",
                                self.url, self.offset, start
                            );
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!(
                                    "WebDAV 服务器返回的分段起始位置不匹配 (期望 {}, 实际 {})",
                                    self.offset, start
                                ),
                            ));
                        }
                    }
                    self.current_resp = Some(resp);
                    self.resp_offset = self.offset;
                } else if status.as_u16() == 200 && self.offset == 0 {
                    // 服务器忽略 Range 但请求起点本来就是 0：整条 200 响应可当作全量流使用，
                    // 不再直接报错（不支持 Range 的来源会在能力探测后改走整文件下载，这里是兜底）
                    tracing::warn!(
                        "WebDAV server ignored Range (HTTP 200); using full stream for {}",
                        self.url
                    );
                    self.current_resp = Some(resp);
                    self.resp_offset = 0;
                } else if is_retryable_status(status)
                    && self.wait_before_retry(
                        &mut retries,
                        &format!("服务器繁忙 (HTTP {})", status.as_u16()),
                        retry_after_delay(resp.headers(), RANGE_RETRY_AFTER_CAP),
                    )
                {
                    continue;
                } else if is_retryable_status(status) {
                    // 重试预算已耗尽：这是服务端瞬时故障/限流，不是"不支持分段读取"
                    tracing::error!(
                        "HttpRangeReader giving up on {} after {} retries (HTTP {})",
                        self.url,
                        RANGE_MAX_RETRIES,
                        status
                    );
                    return Err(io::Error::other(format!(
                        "WebDAV 服务器暂时不可用 (HTTP {})，已重试仍未恢复",
                        status
                    )));
                } else {
                    tracing::error!(
                        "HttpRangeReader fetch failed for url: {} with status: {}",
                        self.url,
                        status
                    );
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("WebDAV 服务器不支持分段读取 (HTTP {})", status),
                    ));
                }
            }

            // 先取结果再处理：wait_before_retry 需要 &mut self 丢弃当前响应，
            // 不能在一个仍借用 current_resp 的 if-let 作用域里调用。
            let stream_result = match self.current_resp.as_mut() {
                Some(resp) => resp.read(buf),
                None => continue,
            };
            match stream_result {
                Ok(0) => {
                    if self.offset >= self.length {
                        return Ok(0);
                    }
                    // 服务端提前断流：带退避地在同一 offset 重连，预算耗尽才报 EOF
                    if !self.wait_before_retry(&mut retries, "响应提前结束", None) {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Premature EOF from server",
                        ));
                    }
                    continue;
                }
                Ok(n) => {
                    self.offset += n as u64;
                    self.resp_offset += n as u64;
                    return Ok(n);
                }
                Err(e) => {
                    if !self.wait_before_retry(&mut retries, "数据流读取失败", None) {
                        tracing::error!("HTTP stream read error: {}", e);
                        return Err(e);
                    }
                    continue;
                }
            }
        }
    }
}

impl Seek for HttpRangeReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_offset = match pos {
            SeekFrom::Start(o) => o as i64,
            SeekFrom::Current(o) => self.offset as i64 + o,
            SeekFrom::End(o) => self.length as i64 + o,
        };

        if new_offset < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid seek to negative offset",
            ));
        }

        self.offset = new_offset as u64;
        Ok(self.offset)
    }
}

/// 解析 Content-Range 头的起始偏移（"bytes 0-1023/1465152" → 0）。
/// 通配形式（"bytes */123"）或格式异常返回 None，调用方跳过校验。
fn parse_content_range_start(value: &str) -> Option<u64> {
    let rest = value.trim().strip_prefix("bytes")?.trim();
    let first = rest.split('-').next()?.trim();
    first.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_range_start_parses_standard_header() {
        assert_eq!(parse_content_range_start("bytes 0-1023/1465152"), Some(0));
        assert_eq!(
            parse_content_range_start("bytes 1024-2047/1465152"),
            Some(1024)
        );
    }

    #[test]
    fn content_range_start_handles_wildcard_and_garbage() {
        assert_eq!(parse_content_range_start("bytes */1465152"), None);
        assert_eq!(parse_content_range_start("garbage"), None);
    }
}
