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
    /// 大体积传输专用客户端（整文件下载、DB 快照上传）：保留连接超时与 keepalive，但
    /// **不在客户端上设总超时**——总超时挂到客户端会同时套住 PUT 的整个请求体，GB 级快照
    /// 会被固定值掐断在半程。终止条件改由每个请求按 `bulk_budget` 单独施加（见其文档）。
    pub bulk_client: Client,
    /// 本次传输的终止预算（CR-007）。不写成常量而放进实例：停滞路径的回归测试要把窗口
    /// 缩到毫秒级，否则每个用例都得等满 90 秒。
    bulk_budget: BulkBudget,
    pub base_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// 大体积传输（整文件下载、DB 快照上传/下载）的终止条件（CR-007）。
///
/// `RequestBuilder::timeout` 在 reqwest 里是**整次传输的总预算**：请求体侧由
/// `execute_request` 一次性建 deadline 并驱动完整个 body，响应体侧由
/// `async_impl::body::response` 用 `total_timeout` 包住整个 body 流。所以它一定终止得掉，
/// 但也一定要求预算随体积走——固定值会把慢速大文件掐死在半程。
/// 因此上传与下载分别配置换算阈值（审查报告建议 4：不共用一个粗粒度策略），
/// 体积已知时按「体积 / 最小假设吞吐」给足时间，未知时走兜底预算，两端都有硬上限。
///
/// TCP keepalive 仍然保留：它是 OS 层的补充，能提前探出「对端完全不再回包」，
/// 但探不到「链路活着、ACK 照回、应用层不再吐字节」的服务器。
///
/// 本期未覆盖（记在 I3 执行记录的残余缺口里）：
/// 1. 「真·空闲超时」——blocking API 没有 `read_timeout`（只有 async `ClientBuilder` 有），
///    所以低于最小假设吞吐的极慢链路会被当成停滞掐断，而不是按进展放行；
/// 2. 用户主动取消（取消令牌）：blocking 请求一旦发出就无法中途唤醒，需要整条链路 async 化。
#[derive(Debug, Clone, Copy)]
struct BulkBudget {
    /// 下载总预算的换算参数。
    download: TransferBudget,
    /// 上传总预算的换算参数。
    upload: TransferBudget,
}

/// 「体积 → 传输总时长」的换算参数，集中成一处便于核对口径，
/// 也让停滞路径的回归测试能换成毫秒级值——生产下限 60s 起，逐个用例等下去不现实。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TransferBudget {
    /// 最小假设吞吐：低于它就把这条链路判为「传不完」而不是「慢」。
    min_bytes_per_sec: u64,
    /// 与体积无关的固定开销：连接、TLS 握手、服务端接收与落盘排队。
    allowance: Duration,
    /// 总预算下限：体积再小也要留出握手与服务端提交时间。
    min: Duration,
    /// 总预算上限：不封顶就等于没有终止条件。
    max: Duration,
    /// 体积未知（拿不到 Content-Length / metadata）时的兜底预算。
    unknown_size: Duration,
}

impl TransferBudget {
    /// 按体积推导本次传输的总预算：固定开销 + 体积 / 最小假设吞吐，再夹到 `[min, max]`。
    fn total_for(&self, expected_bytes: Option<u64>) -> Duration {
        let Some(bytes) = expected_bytes else {
            return self.unknown_size.clamp(self.min, self.max);
        };
        // 先夹住换算结果再加固定开销：`u64::MAX` 字节会算出天文秒数，直接相加会溢出 panic。
        let rate = self.min_bytes_per_sec.max(1) as f64;
        let transfer = Duration::from_secs_f64(bytes as f64 / rate).min(self.max);
        (self.allowance + transfer).clamp(self.min, self.max)
    }
}

/// 与体积无关的固定开销：连接、TLS 握手、服务端接收与落盘排队。
const BULK_TRANSFER_ALLOWANCE: Duration = Duration::from_secs(30);

/// 体积未知时的兜底总预算（云端恢复在开工前拿不到快照体积）。
const BULK_UNKNOWN_SIZE_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// 传输总预算下限：小文件也要留够握手与服务端提交时间。
const BULK_TIMEOUT_MIN: Duration = Duration::from_secs(60);

/// 上传的最小假设吞吐：32 KiB/s ≈ 256 kbps，覆盖弱网移动链路上持续慢速的上传。
pub const UPLOAD_MIN_BYTES_PER_SEC: u64 = 32 * 1024;

/// 下载的最小假设吞吐：128 KiB/s ≈ 1 Mbps，下行链路通常比上行快一档。
pub const DOWNLOAD_MIN_BYTES_PER_SEC: u64 = 128 * 1024;

/// 上传总预算上限：按 32 KiB/s 换算，它锁定本方案愿意等待的最慢上传约为 225 MiB。
const UPLOAD_TIMEOUT_MAX: Duration = Duration::from_secs(2 * 60 * 60);

/// 下载总预算上限：整首曲目通常几十 MB，1 小时封顶已覆盖极端慢速下行。
const DOWNLOAD_TIMEOUT_MAX: Duration = Duration::from_secs(60 * 60);

/// 生产上传预算。
const UPLOAD_BUDGET: TransferBudget = TransferBudget {
    min_bytes_per_sec: UPLOAD_MIN_BYTES_PER_SEC,
    allowance: BULK_TRANSFER_ALLOWANCE,
    min: BULK_TIMEOUT_MIN,
    max: UPLOAD_TIMEOUT_MAX,
    unknown_size: BULK_UNKNOWN_SIZE_TIMEOUT,
};

/// 生产下载预算。
const DOWNLOAD_BUDGET: TransferBudget = TransferBudget {
    min_bytes_per_sec: DOWNLOAD_MIN_BYTES_PER_SEC,
    allowance: BULK_TRANSFER_ALLOWANCE,
    min: BULK_TIMEOUT_MIN,
    max: DOWNLOAD_TIMEOUT_MAX,
    unknown_size: BULK_UNKNOWN_SIZE_TIMEOUT,
};

impl Default for BulkBudget {
    fn default() -> Self {
        Self {
            download: DOWNLOAD_BUDGET,
            upload: UPLOAD_BUDGET,
        }
    }
}

// 预算常量的大小关系是这套口径的一部分：写反了会让下限高于上限、兜底预算超过封顶，
// 或者上下行用同一个阈值而失去意义。放进编译期断言，而不是等测试跑挂才发现。
const _: () = {
    assert!(UPLOAD_MIN_BYTES_PER_SEC < DOWNLOAD_MIN_BYTES_PER_SEC);
    assert!(BULK_TIMEOUT_MIN.as_secs() < BULK_UNKNOWN_SIZE_TIMEOUT.as_secs());
    assert!(BULK_UNKNOWN_SIZE_TIMEOUT.as_secs() < DOWNLOAD_TIMEOUT_MAX.as_secs());
    assert!(DOWNLOAD_TIMEOUT_MAX.as_secs() < UPLOAD_TIMEOUT_MAX.as_secs());
};

impl WebdavClient {
    pub fn new(base_url: String, username: Option<String>, password: Option<String>) -> Self {
        // 联网行为: §2E §2F —— 本客户端被曲库与备份两条链路共用，目标主机一律来自用户自填地址。
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
        // 联网行为: §2E §2F —— 大文件传输客户端，同样只连用户自填的 WebDAV 地址。
        let bulk_client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            // 不在客户端上设 timeout()：它会连带套住 PUT 的整个请求体（见 BulkBudget 文档）。
            // 终止条件按请求逐个施加——下载用空闲窗口、上传用按体积推导的总预算。
            // keepalive 只是 OS 层补充：能兜住「对端完全不再回包」，兜不住应用层停滞。
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
            bulk_budget: BulkBudget::default(),
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

    /// 传输超时的可读文案（CR-007）。点明「等了多久」与「传输没完成」，
    /// 让用户区分"链路太慢"与笼统的"网络请求失败"。
    fn bulk_stall_message(what: &str, budget: Duration) -> String {
        format!(
            "{}超时：服务器在 {} 秒内没有完成传输，请稍后重试或换用更好的网络",
            what,
            // 测试会注入亚秒预算，生产值恒 >= 60s；取 max 只为不把"0 秒"甩给用户看
            budget.as_secs().max(1)
        )
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

        // 联网行为: §2E §2F
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

        // 联网行为: §2E §2F
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
        // 联网行为: §2E §2F
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
    /// 走 bulk_client + 按体积推导的总预算（CR-007）：`timeout` 覆盖整次传输，
    /// 所以体积已知就按「体积 / 最小假设吞吐」给足时间（慢速但持续有进展不会被误杀），
    /// 体积未知走兜底预算；无论哪种，挂死的连接都会在明确时间内返回可读错误。
    /// `expected_bytes` 传 DB 记录或 PROPFIND 得到的远端文件大小。
    /// 返回写入的字节数。
    pub fn download_to_file(
        &self,
        file_url: &str,
        dest: &Path,
        expected_bytes: Option<u64>,
    ) -> Result<u64, String> {
        // 只重试"还没开始传字节"的失败：状态码 429/5xx 与连接层错误。
        // 半路断开的响应体不能在这里重连（没有 Range 续传），交给上层的临时文件校验兜底。
        const DL_BASE: Duration = Duration::from_millis(500);
        const DL_MAX: Duration = Duration::from_secs(8);
        const DL_RETRY_AFTER_CAP: Duration = Duration::from_secs(15);
        let budget = self.bulk_budget.download.total_for(expected_bytes);
        let mut retries: u32 = 0;
        let mut resp = loop {
            // 联网行为: §2E §2F
            let req = self.apply_auth(self.bulk_client.get(file_url).timeout(budget));
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
                // 只重连"握手阶段就失败"的错误（含 connect 超时，单次最多 10s）。
                // 预算耗尽的停滞不重试：那说明服务器已经接了请求却不给数据，
                // 重试只会把最坏等待时间乘以 (1+重试次数)，把前端"处理中"挂死更久（CR-007）。
                Err(e) if e.is_connect() && retries < DOWNLOAD_MAX_RETRIES => {
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
                Err(e) if e.is_timeout() => return Err(Self::bulk_stall_message("下载", budget)),
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
        // 总预算同样覆盖响应体读取（reqwest 用 total_timeout 包住整个 body 流），
        // 所以「服务器吐完一半就沉默」会在预算到点时返回可读错误，而不是把线程挂到永远。
        // 半截文件由调用方按 Err 分支删除（缓存侧删 .tmp，恢复侧删 .download）。
        let bytes = resp.copy_to(&mut file).map_err(|e| {
            if e.is_timeout() {
                Self::bulk_stall_message("下载", budget)
            } else {
                format!("Download write failed: {}", e)
            }
        })?;
        Ok(bytes)
    }

    /// MKCOL：创建远程目录（同步目录初始化、文件夹浏览器新建文件夹用）。
    /// 对已存在的目录返回 405，视为成功（幂等）。
    pub fn mkcol(&self, path_url: &str) -> Result<(), String> {
        let req = self.apply_auth(
            // 联网行为: §2E §2F
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
    ///
    /// 终止条件与下载不同（CR-007 建议 4：上传/下载不共用一个粗粒度策略）。带请求体的请求
    /// 在 blocking reqwest 里只在发送前建立一次 deadline，并驱动完整个 body，所以这里的
    /// `timeout` 是**覆盖整次上传的总预算**——必须按体积换算，固定值会把慢速大快照掐死。
    /// `expected_bytes` 取调用方已经知道的体积（快照的 file_size、校验和的字节数）；
    /// 传 None 时走体积未知的兜底预算。
    ///
    /// body 收 `Into<Body>` 而不是 `Vec<u8>`：上百 MB 至 GiB 级的快照读进内存再克隆一份，
    /// 峰值内存就是「两份完整数据库」，低内存设备上进程会被系统直接杀掉（CR-004）。
    /// 传 `File` 时 reqwest 按 metadata 推出 content-length 并边读边发。
    pub fn put_file<B: Into<reqwest::blocking::Body>>(
        &self,
        file_url: &str,
        body: B,
        expected_bytes: Option<u64>,
    ) -> Result<(), String> {
        let budget = self.bulk_budget.upload.total_for(expected_bytes);
        let req = self.apply_auth(
            // 联网行为: §2E §2F
            self.bulk_client
                .put(file_url)
                .body(body.into())
                .timeout(budget),
        );
        let resp = req.send().map_err(|e| {
            if e.is_timeout() {
                Self::bulk_stall_message("上传", budget)
            } else {
                format!("PUT request failed: {}", e)
            }
        })?;
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
            // 联网行为: §2E §2F
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
        // 联网行为: §2E §2F
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
        // 联网行为: §2E §2F
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
                // 联网行为: §2E §2F
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

    /// CR-007：大文件传输的终止条件。
    ///
    /// 夹具全部走 loopback 上的自建故障服务器（审查报告建议 5「连接建立后不再传输数据」）：
    /// 只完成 TCP 握手就沉默的连接，keepalive 探不到、请求会永远挂着，正是这条整改要治的形态。
    /// 用例把预算注入成毫秒级——终止条件与具体数值无关，等满生产的 90s 只是让套件变慢。
    mod bulk_transfer_termination {
        use super::*;
        use crate::db::test_util::TempDir;
        use std::io::Write;
        use std::net::{TcpListener, TcpStream};
        use std::time::Instant;

        /// 接受连接后一个字节都不读、也不回应的服务器。
        fn silent_server() -> String {
            listen_loopback(|listener| {
                let mut held = Vec::new();
                for stream in listener.incoming().flatten() {
                    // 握住连接不关闭：关掉会让客户端看到 EOF，测的就不是停滞而是断链
                    held.push(stream);
                }
            })
        }

        /// 每隔 `gap` 才吐 1 字节的慢速服务器：用来同时验证"预算内放行"和"超预算掐断"。
        fn trickling_server(bytes: usize, gap: Duration) -> String {
            listen_loopback(move |listener| {
                let Some(Ok(mut stream)) = listener.incoming().next() else {
                    return;
                };
                // 先把请求读干净再回应：带着未读数据关闭套接字，Windows 会发 RST 而不是 FIN，
                // 客户端可能因此丢掉已经收到的响应体，测的就不是慢速而是断链了。
                if drain_request(&mut stream).is_err() {
                    return;
                }
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes
                );
                if stream.write_all(head.as_bytes()).is_err() {
                    return;
                }
                for _ in 0..bytes {
                    std::thread::sleep(gap);
                    if stream.write_all(b"x").is_err() {
                        return;
                    }
                }
                // 写完即关闭：Content-Length 已满足，这里是正常结束而不是截断
            })
        }

        /// 读到请求头结束标记（`\r\n\r\n`）为止。
        fn drain_request(stream: &mut TcpStream) -> io::Result<()> {
            let mut buf = [0u8; 512];
            let mut seen = Vec::new();
            loop {
                let n = stream.read(&mut buf)?;
                if n == 0 {
                    return Ok(());
                }
                seen.extend_from_slice(&buf[..n]);
                if seen.windows(4).any(|w| w == b"\r\n\r\n") {
                    return Ok(());
                }
            }
        }

        /// 起一个只监听 127.0.0.1:0 的线程级夹具，返回 base URL。
        /// 处理线程随测试进程结束一起退出，连接由调用方在闭包里逐个接手。
        fn listen_loopback<F>(handler: F) -> String
        where
            F: FnOnce(TcpListener) + Send + 'static,
        {
            let listener = TcpListener::bind("127.0.0.1:0").expect("loopback 端口应可绑定");
            let addr = listener.local_addr().expect("loopback 地址应可查询");
            std::thread::spawn(move || handler(listener));
            format!("http://{}", addr)
        }

        /// 毫秒级的传输预算：终止条件与具体数值无关，生产的 60s 起步等下去只是让套件变慢。
        fn client_at(base: &str) -> WebdavClient {
            // reqwest 以 rustls-no-provider 构建，客户端在测试进程里也要先装好加密后端（ADR-1）
            crate::install_crypto_provider();
            let fast = TransferBudget {
                min_bytes_per_sec: 1_000_000,
                allowance: Duration::from_millis(50),
                min: Duration::from_millis(250),
                max: Duration::from_secs(2),
                unknown_size: Duration::from_millis(250),
            };
            let mut client = WebdavClient::new(base.to_string(), None, None);
            client.bulk_budget.download = fast;
            client.bulk_budget.upload = fast;
            client
        }

        /// 生产默认值必须是文档口径里的那几个常量：改动会同时影响备份、缓存与恢复路径。
        /// （常量之间的大小关系另有编译期断言把关。）
        #[test]
        fn default_budget_matches_the_documented_constants() {
            let budget = BulkBudget::default();
            assert_eq!(budget.upload, UPLOAD_BUDGET);
            assert_eq!(budget.download, DOWNLOAD_BUDGET);
        }

        #[test]
        fn transfer_budget_grows_with_size_and_is_bounded() {
            // 4 MiB / 32 KiB·s⁻¹ = 128s，再加固定开销
            assert_eq!(
                UPLOAD_BUDGET.total_for(Some(4 * 1024 * 1024)),
                BULK_TRANSFER_ALLOWANCE + Duration::from_secs(128)
            );
            // 同一份 4 MiB 走下载预算：下行假设吞吐更高，预算更短
            assert_eq!(
                DOWNLOAD_BUDGET.total_for(Some(4 * 1024 * 1024)),
                BULK_TRANSFER_ALLOWANCE + Duration::from_secs(32)
            );
            // 256 MiB 快照 → 8192s，但被上传封顶夹住
            assert_eq!(
                UPLOAD_BUDGET.total_for(Some(256 * 1024 * 1024)),
                UPLOAD_TIMEOUT_MAX
            );
            // 体积再小也有下限（小文件也要留够握手与服务端提交时间），
            // 体积再大也有封顶（含 u64::MAX 这种荒谬输入，不能溢出 panic）
            assert_eq!(UPLOAD_BUDGET.total_for(Some(0)), BULK_TIMEOUT_MIN);
            assert_eq!(UPLOAD_BUDGET.total_for(Some(1)), BULK_TIMEOUT_MIN);
            assert_eq!(UPLOAD_BUDGET.total_for(Some(64 * 1024)), BULK_TIMEOUT_MIN);
            assert_eq!(UPLOAD_BUDGET.total_for(Some(u64::MAX)), UPLOAD_TIMEOUT_MAX);
            assert_eq!(
                DOWNLOAD_BUDGET.total_for(Some(u64::MAX)),
                DOWNLOAD_TIMEOUT_MAX
            );
            // 体积未知走兜底预算，而不是"没有预算"
            assert_eq!(UPLOAD_BUDGET.total_for(None), BULK_UNKNOWN_SIZE_TIMEOUT);
            assert_eq!(DOWNLOAD_BUDGET.total_for(None), BULK_UNKNOWN_SIZE_TIMEOUT);
            // 单调：更大的体积绝不能拿到更短的预算
            for budget in [UPLOAD_BUDGET, DOWNLOAD_BUDGET] {
                let mut last = Duration::ZERO;
                for bytes in [0u64, 1, 4096, 1 << 20, 1 << 26, 1 << 30, u64::MAX] {
                    let total = budget.total_for(Some(bytes));
                    assert!(
                        total >= last,
                        "{} 字节的预算 {:?} 短于上一档 {:?}",
                        bytes,
                        total,
                        last
                    );
                    last = total;
                }
            }
        }

        #[test]
        fn stalled_download_fails_within_its_total_budget() {
            let dir = TempDir::new("bulk_stalled_download");
            let dest = dir.path().join("partial.bin");
            let base = silent_server();
            let client = client_at(&base);

            let started = Instant::now();
            let err = client
                .download_to_file(&format!("{}/stalled", base), &dest, Some(4096))
                .expect_err("服务器接受连接后不再发字节，下载必须失败");
            let elapsed = started.elapsed();

            assert!(err.contains("超时"), "文案要让用户看出是超时：{}", err);
            assert!(err.contains("下载"), "文案要指明失败阶段：{}", err);
            // 预算耗尽即返回，不再叠加重试：停滞重试只会把前端"处理中"挂得更久
            assert!(
                elapsed < Duration::from_secs(5),
                "停滞下载耗时 {:?}，终止条件未生效",
                elapsed
            );
            // 验收点「不留下本地半成品」：连文件都不该创建（响应头都没等到）
            assert!(!dest.exists(), "停滞下载不应落地任何本地文件");
        }

        #[test]
        fn stalled_upload_fails_within_the_total_budget() {
            let base = silent_server();
            let client = client_at(&base);

            let started = Instant::now();
            let err = client
                .put_file(&format!("{}/staged", base), vec![7u8; 4096], Some(4096))
                .expect_err("服务器不再收字节，上传必须失败");
            let elapsed = started.elapsed();

            assert!(err.contains("超时"), "文案要让用户看出是超时：{}", err);
            assert!(err.contains("上传"), "文案要指明失败阶段：{}", err);
            assert!(
                elapsed < Duration::from_secs(5),
                "停滞上传耗时 {:?}，终止条件未生效",
                elapsed
            );
        }

        #[test]
        fn unknown_size_transfers_also_get_a_deadline() {
            let base = silent_server();
            let client = client_at(&base);
            let started = Instant::now();
            let err = client
                .put_file(&format!("{}/staged", base), vec![7u8; 64], None)
                .expect_err("体积未知的上传同样要有终止条件");
            assert!(err.contains("超时"), "文案要让用户看出是超时：{}", err);

            let dir = TempDir::new("bulk_unknown_size_download");
            let err = client
                .download_to_file(
                    &format!("{}/stalled", base),
                    &dir.path().join("out.bin"),
                    None,
                )
                .expect_err("体积未知的下载同样要有终止条件");
            assert!(err.contains("超时"), "文案要让用户看出是超时：{}", err);
            // 必须早于 OS 兜底：bulk_client 的 tcp_keepalive 是 30s，若测到 30s 上下，
            // 说明是内核把连接掐了而不是应用层预算在生效（keepalive 兜不住应用层停滞）。
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "体积未知的传输耗时 {:?}，终止条件未生效",
                started.elapsed()
            );
        }

        /// 验收点「慢但持续有进展不被误杀」：预算按体积给足时间后，
        /// 每 40ms 才吐 1 字节的下载必须完整走完（12 字节约 0.5s，远超 250ms 的下限预算）。
        #[test]
        fn trickling_download_inside_its_budget_completes() {
            let dir = TempDir::new("bulk_trickle_download");
            let dest = dir.path().join("slow.bin");
            let base = trickling_server(12, Duration::from_millis(40));
            let mut client = client_at(&base);
            // 12 字节的 trickle ≈ 0.5s：给它 2s 总预算
            client.bulk_budget.download.allowance = Duration::from_secs(2);
            client.bulk_budget.download.min = Duration::from_secs(2);

            let bytes = client
                .download_to_file(&format!("{}/slow", base), &dest, Some(12))
                .expect("预算内的慢速下载不能被掐断");
            assert_eq!(bytes, 12);
            assert_eq!(
                std::fs::metadata(&dest).expect("文件应存在").len(),
                12,
                "下载内容必须完整落地"
            );
        }

        /// 取舍的另一面，写成测试固定住：低于最小假设吞吐（这里是"2s 传 12 字节"≈ 6 B/s）
        /// 的传输被判定为传不完并掐断——blocking API 没有 read_timeout，做不到按进展放行。
        /// 掐断时必须在预算内返回，并且**已写的半截文件由调用方删除**（本层保留半成品）。
        #[test]
        fn download_below_the_assumed_throughput_is_cut_off_at_its_budget() {
            let dir = TempDir::new("bulk_trickle_timeout");
            let dest = dir.path().join("half.bin");
            let base = trickling_server(40, Duration::from_millis(100));
            let mut client = client_at(&base);
            client.bulk_budget.download.allowance = Duration::from_millis(500);
            client.bulk_budget.download.min = Duration::from_millis(500);
            client.bulk_budget.download.max = Duration::from_secs(1);

            let started = Instant::now();
            let err = client
                .download_to_file(&format!("{}/slow", base), &dest, Some(40))
                .expect_err("远低于假设吞吐的传输应当被预算掐断");
            assert!(err.contains("超时"), "文案要让用户看出是超时：{}", err);
            // 半途停滞与「连不上服务器」必须是两条文案：这里已经拿到响应头并收到过字节，
            // 若退化成通用的"连接超时"，用户会去查地址和证书，而实际问题是链路太慢。
            assert!(err.contains("下载"), "文案要指明失败阶段：{}", err);
            assert!(err.contains("秒"), "文案要让用户看出等了多久：{}", err);
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "掐断必须及时，实际耗时 {:?}",
                started.elapsed()
            );
        }
    }
}
