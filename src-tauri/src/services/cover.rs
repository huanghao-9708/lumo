//! 在线封面/头像检索服务（v1.8.1）。
//!
//! 数据源链（按本机可达性与 CJK 曲库命中率实测排序，2026-09-12）：
//!   1. 网易云音乐开放接口（music.163.com）——免费无鉴权，CJK 命中率最高，
//!      专辑 `?param=1500y1500`（实测 ~1.8MB 原图级）/ 歌手 `?param=1200y1200`
//!   2. iTunes Search API——兜底；artworkUrl100 的 `100x100bb` 后缀可替换为
//!      `100000x100000bb` 直接取源图最大分辨率（此前固定 600x600，画质偏低）
//!
//! Deezer 在部分网络不可达（实测超时），不纳入。
//! 隐私（P0-07）：仅在用户显式开启「在线封面匹配」后由调用方触发，
//! 请求只包含专辑/艺人名称。

use serde_json::Value;

const NETEASE_REFERER: &str = "https://music.163.com";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) LumoPlayer";
const IMAGE_TIMEOUT_SECS: u64 = 20;

/// 一次成功的封面命中
#[derive(Debug)]
pub struct CoverHit {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

pub struct CoverService;

impl CoverService {
    fn client() -> Result<reqwest::Client, String> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(IMAGE_TIMEOUT_SECS))
            .user_agent(UA)
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))
    }

    /// 网易云 GET（带 Referer/UA；CDN 与 API 均要求浏览器式头）
    async fn netease_get_json(client: &reqwest::Client, url: &str) -> Option<Value> {
        let resp = client
            .get(url)
            .header("Referer", NETEASE_REFERER)
            .send()
            .await
            .ok()?;
        resp.error_for_status().ok()?.json::<Value>().await.ok()
    }

    fn https_upgraded(url: &str) -> String {
        url.replacen("http://", "https://", 1)
    }

    /// 下载图片（返回 bytes + mime）
    async fn fetch_image(client: &reqwest::Client, url: &str) -> Option<CoverHit> {
        let resp = client.get(url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let mime_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .split(';')
            .next()
            .unwrap_or("image/jpeg")
            .trim()
            .to_string();
        let bytes = resp.bytes().await.ok()?;
        // 过小视为占位图/错误页
        if bytes.len() < 2_000 {
            return None;
        }
        Some(CoverHit { bytes: bytes.to_vec(), mime_type })
    }

    /// 专辑封面：网易云（1500x1500）→ iTunes（最大分辨率）
    pub async fn search_album_cover(album_title: &str, artist_name: Option<&str>) -> Option<CoverHit> {
        let client = Self::client().ok()?;

        // 1) 网易云：专辑搜索 type=10，取第一个有图的
        let mut q = album_title.to_string();
        if let Some(a) = artist_name {
            if !a.trim().is_empty() {
                q = format!("{} {}", a.trim(), album_title);
            }
        }
        let url = url::Url::parse_with_params(
            "https://music.163.com/api/search/get",
            &[("s", q.as_str()), ("type", "10"), ("limit", "5")],
        )
        .ok()?;
        if let Some(json) = Self::netease_get_json(&client, url.as_str()).await {
            let albums = json["result"]["albums"].as_array();
            if let Some(albums) = albums {
                for al in albums {
                    let pic = al["picUrl"].as_str().map(Self::https_upgraded);
                    if let Some(pic) = pic {
                        let sized = format!("{}?param=1500y1500", pic);
                        if let Some(hit) = Self::fetch_image(&client, &sized).await {
                            tracing::info!("[cover] album 网易云命中: {} -> {}KB", al["name"].as_str().unwrap_or(""), hit.bytes.len() / 1024);
                            return Some(hit);
                        }
                    }
                }
            }
        }

        // 2) iTunes 兜底（最大分辨率）
        let url = url::Url::parse_with_params(
            "https://itunes.apple.com/search",
            &[("term", q.as_str()), ("entity", "album"), ("limit", "1")],
        )
        .ok()?;
        let json: Value = client.get(url).send().await.ok()?.json().await.ok()?;
        let artwork = json["results"][0]["artworkUrl100"].as_str()?;
        if artwork.is_empty() {
            return None;
        }
        let big = artwork.replace("100x100bb", "100000x100000bb");
        let hit = Self::fetch_image(&client, &big).await?;
        tracing::info!("[cover] album iTunes 兜底: {}KB", hit.bytes.len() / 1024);
        Some(hit)
    }

    /// 歌手头像：网易云（type=100 搜索 → 歌手详情 picUrl 1200x1200）→ iTunes 兜底
    pub async fn search_artist_cover(artist_name: &str) -> Option<CoverHit> {
        let client = Self::client().ok()?;

        let url = url::Url::parse_with_params(
            "https://music.163.com/api/search/get",
            &[("s", artist_name), ("type", "100"), ("limit", "1")],
        )
        .ok()?;
        if let Some(json) = Self::netease_get_json(&client, url.as_str()).await {
            let artist_id = json["result"]["artists"][0]["id"].as_i64();
            if let Some(id) = artist_id {
                if let Some(detail) = Self::netease_get_json(&client, &format!("https://music.163.com/api/artist/{}", id)).await {
                    // 详情 picUrl 是大图；img1v1Url 兜底（可能仅 130px）
                    let pic = detail["artist"]["picUrl"]
                        .as_str()
                        .or_else(|| detail["artist"]["img1v1Url"].as_str())
                        .map(Self::https_upgraded);
                    if let Some(pic) = pic {
                        let sized = format!("{}?param=1200y1200", pic);
                        if let Some(hit) = Self::fetch_image(&client, &sized).await {
                            tracing::info!("[cover] artist 网易云命中: {} -> {}KB", artist_name, hit.bytes.len() / 1024);
                            return Some(hit);
                        }
                    }
                }
            }
        }

        // iTunes 兜底：歌手实体出图率低，沿用「取其专辑封面」策略（原实现行为）
        let url = url::Url::parse_with_params(
            "https://itunes.apple.com/search",
            &[("term", artist_name), ("entity", "album"), ("limit", "1")],
        )
        .ok()?;
        let json: Value = client.get(url).send().await.ok()?.json().await.ok()?;
        let artwork = json["results"][0]["artworkUrl100"].as_str()?;
        if artwork.is_empty() {
            return None;
        }
        let big = artwork.replace("100x100bb", "100000x100000bb");
        let hit = Self::fetch_image(&client, &big).await?;
        tracing::info!("[cover] artist iTunes 兜底: {}KB", hit.bytes.len() / 1024);
        Some(hit)
    }
}
