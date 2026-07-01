use reqwest::blocking::Client;
use reqwest::header::RANGE;
use std::io::{self, Read, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;
use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Clone)]
pub struct WebdavFile {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub last_modified: String,
}

#[derive(Clone)]
pub struct WebdavClient {
    pub client: Client,
    pub base_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl WebdavClient {
    pub fn new(base_url: String, username: Option<String>, password: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            username,
            password,
        }
    }
    
    // helper to add auth
    fn apply_auth(&self, req: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            req.basic_auth(u, Some(p))
        } else {
            req
        }
    }

    pub fn propfind(&self, subpath: &str) -> Result<Vec<WebdavFile>, String> {
        let url = if subpath.starts_with("http://") || subpath.starts_with("https://") {
            subpath.to_string()
        } else {
            let base = reqwest::Url::parse(&format!("{}/", self.base_url)).unwrap();
            base.join(subpath).unwrap().to_string()
        };
        
        let req = self.client.request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "1");
            
        let req = self.apply_auth(req);
        
        let resp = req.send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("PROPFIND failed: {}", resp.status()));
        }

        let xml = resp.text().map_err(|e| e.to_string())?;
        let parsed = self.parse_propfind(&xml);
        
        let mut results = Vec::new();
        for file in parsed {
            let file_url_path = reqwest::Url::parse(&format!("{}/", self.base_url))
                .unwrap()
                .join(&file.path)
                .unwrap()
                .path()
                .to_string();
            let req_url_path = reqwest::Url::parse(&url).unwrap().path().to_string();
            
            if file_url_path.trim_end_matches('/') == req_url_path.trim_end_matches('/') {
                continue;
            }
            results.push(file);
        }
        
        Ok(results)
    }

    fn parse_propfind(&self, xml: &str) -> Vec<WebdavFile> {
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
                    let local_name = tag_name.split(':').last().unwrap_or(&tag_name);
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
                    let local_name = tag_name.split(':').last().unwrap_or(&tag_name);
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
                    let local_name = tag_name.split(':').last().unwrap_or(&tag_name);
                    
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
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        files
    }

    /// 用单个 GET 请求下载完整文件到指定本地路径（带认证）。
    /// 用于云端文件透明缓存：播放 WebDAV 歌曲时后台异步拉取完整文件，
    /// 下次播放同一首歌即可命中本地缓存，实现「零网络请求」秒开。
    /// 返回写入的字节数。
    pub fn download_to_file(&self, file_url: &str, dest: &Path) -> Result<u64, String> {
        let req = self.apply_auth(self.client.get(file_url));
        let mut resp = req.send().map_err(|e| format!("Download request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Download failed: HTTP {}", resp.status()));
        }
        let mut file = File::create(dest).map_err(|e| format!("Failed to create cache file: {}", e))?;
        let bytes = resp.copy_to(&mut file).map_err(|e| format!("Download write failed: {}", e))?;
        Ok(bytes)
    }
}

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
    
    fn apply_auth(&self, req: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            req.basic_auth(u, Some(p))
        } else {
            req
        }
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

        let mut retries = 0;
        loop {
            // Forward seek optimization (up to 256KB)
            if self.current_resp.is_some() && self.offset > self.resp_offset && self.offset - self.resp_offset <= 256 * 1024 {
                let mut skip = self.offset - self.resp_offset;
                let mut dummy = [0u8; 8192];
                let mut success = true;
                while skip > 0 {
                    let to_read = std::cmp::min(skip, dummy.len() as u64) as usize;
                    match self.current_resp.as_mut().unwrap().read(&mut dummy[..to_read]) {
                        Ok(0) => { success = false; break; }
                        Ok(n) => {
                            skip -= n as u64;
                            self.resp_offset += n as u64;
                        }
                        Err(_) => { success = false; break; }
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
                        tracing::error!("HttpRangeReader fetch failed for url: {}", self.url);
                        return Err(io::Error::new(io::ErrorKind::Other, format!("HTTP Error: {}", e)));
                    }
                };
                
                if !resp.status().is_success() {
                    tracing::error!("HttpRangeReader fetch failed for url: {} with status: {}", self.url, resp.status());
                    return Err(io::Error::new(io::ErrorKind::Other, format!("HTTP Error: {}", resp.status())));
                }

                self.current_resp = Some(resp);
                self.resp_offset = self.offset;
            }

            if let Some(resp) = self.current_resp.as_mut() {
                match resp.read(buf) {
                    Ok(0) => {
                        if self.offset < self.length {
                            self.current_resp = None;
                            retries += 1;
                            if retries > 3 {
                                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Premature EOF from server"));
                            }
                            continue;
                        } else {
                            return Ok(0);
                        }
                    }
                    Ok(n) => {
                        self.offset += n as u64;
                        self.resp_offset += n as u64;
                        return Ok(n);
                    }
                    Err(e) => {
                        tracing::warn!("HTTP stream read error: {}, reconnecting...", e);
                        self.current_resp = None;
                        retries += 1;
                        if retries > 3 {
                            return Err(e);
                        }
                        continue;
                    }
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
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid seek to negative offset"));
        }

        self.offset = new_offset as u64;
        Ok(self.offset)
    }
}
