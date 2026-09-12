//! AI 推荐歌单服务（PRD-AI推荐歌单 2026-09-12）。
//!
//! 两段式架构，AI 永不直接面对全库：
//!   ① SQL 候选检索（本文件 `retrieve_candidates`，毫秒级，200-500 首）
//!   ② 一次 LLM 调用：从候选中选 20-30 首 + 起名 + 每首一句话理由，
//!      输出 ID 必须属于候选集（`parse_and_validate` 硬校验，防幻觉）。
//!
//! 隐私红线：prompt 只含 id/标题/艺人/播放次数；歌词、音频、文件路径永不进入请求。
//! 模型接入走用户自备的 OpenAI 兼容接口（云端 API 或本地 Ollama 皆可），默认关闭。

use crate::error::AppError;
use crate::models::{AiRankedTrackDTO, TrackDTO};
use crate::services::secret::SecretStore;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

/// 候选池上限（≈15-20k tokens 的紧凑列表，14B 级本地模型可承受）
const CANDIDATE_LIMIT: usize = 500;
/// AI 返回曲目数下限：低于此值视为输出质量不足，触发重试/兜底
const MIN_TRACKS: usize = 15;
/// AI 返回曲目数上限（防止超长输出）
const MAX_TRACKS: usize = 40;
/// 一次生成的 LLM 超时
const LLM_TIMEOUT_SECS: u64 = 90;

pub const ERR_NOT_ENABLED: &str = "AI 推荐未开启，请在「设置 → AI 推荐」中配置并开启";

/// 候选歌曲（prompt 紧凑行：id|标题|艺人|播放次数）
#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub play_count: i64,
}

/// 内部设置（含解出的明文 key；key 只在内存，绝不落库/落日志）
#[derive(Debug, Clone)]
pub struct AiSettings {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    pub temperature: f64,
    pub credential_ref: Option<String>,
}

pub struct AiService;

impl AiService {
    // ================= 设置 =================

    pub fn load_settings(conn: &Connection) -> rusqlite::Result<AiSettings> {
        let row = conn
            .query_row(
                "SELECT enabled, base_url, model, temperature, credential_ref FROM ai_settings WHERE id = 1",
                [],
                |row| {
                    Ok(AiSettings {
                        enabled: row.get::<_, i64>(0)? != 0,
                        base_url: row.get(1)?,
                        model: row.get(2)?,
                        temperature: row.get(3)?,
                        credential_ref: row.get(4)?,
                    })
                },
            )
            .optional()?;
        Ok(row.unwrap_or(AiSettings {
            enabled: false,
            base_url: String::new(),
            model: String::new(),
            temperature: 0.8,
            credential_ref: None,
        }))
    }

    /// 保存设置。api_key 语义：None=不变，Some("")=清除，Some(k)=重新存储。
    pub fn save_settings(
        conn: &Connection,
        app_dir: &Path,
        enabled: bool,
        base_url: &str,
        model: &str,
        temperature: f64,
        api_key: Option<String>,
    ) -> Result<(), AppError> {
        let mut cred: Option<String> = conn
            .query_row("SELECT credential_ref FROM ai_settings WHERE id = 1", [], |r| r.get(0))
            .optional()?
            .flatten();

        if let Some(key_input) = api_key {
            if key_input.is_empty() {
                // 清除
                if let Some(old) = cred.take() {
                    Self::delete_stored_key(app_dir, &old);
                }
            } else {
                // 覆盖旧条目（先删后存，避免钥匙串孤儿）
                if let Some(old) = &cred {
                    Self::delete_stored_key(app_dir, old);
                }
                cred = Some(Self::store_key(app_dir, &key_input)?);
            }
        }

        conn.execute(
            "UPDATE ai_settings
             SET enabled = ?1, base_url = ?2, model = ?3, temperature = ?4, credential_ref = ?5, updated_at = datetime('now')
             WHERE id = 1",
            params![enabled as i64, base_url.trim(), model.trim(), temperature, cred],
        )?;
        Ok(())
    }

    fn store_key(app_dir: &Path, key: &str) -> Result<String, AppError> {
        #[cfg(not(target_os = "android"))]
        {
            let _ = app_dir;
            let uuid = hex::encode(rand::random::<[u8; 16]>());
            crate::services::secret::keyring_set(&uuid, key)
                .map_err(|e| AppError::Internal(format!("API Key 写入系统钥匙串失败: {}", e)))?;
            Ok(format!("kr:{}", uuid))
        }
        #[cfg(target_os = "android")]
        {
            let store = crate::services::secret::DefaultSecretStore::new(Some(app_dir));
            store.seal(key).map_err(|e| AppError::Internal(format!("API Key 加密存储失败: {}", e)))
        }
    }

    fn delete_stored_key(app_dir: &Path, cred: &str) {
        #[cfg(not(target_os = "android"))]
        let _ = app_dir;
        #[cfg(not(target_os = "android"))]
        if let Some(uuid) = cred.strip_prefix("kr:") {
            crate::services::secret::keyring_delete(uuid);
        }
        #[cfg(target_os = "android")]
        let _ = (app_dir, cred);
    }

    fn load_key(app_dir: &Path, cred: &str) -> Option<String> {
        #[cfg(not(target_os = "android"))]
        if let Some(uuid) = cred.strip_prefix("kr:") {
            return crate::services::secret::keyring_get(uuid).ok();
        }
        if cred.starts_with("v2:seal:") {
            let store = crate::services::secret::DefaultSecretStore::new(Some(app_dir));
            return store.open(cred).ok();
        }
        None
    }

    // ================= ① 候选检索 =================

    /// 按模式检索候选。短语/种子信息在这里转成 SQL 检索条件；
    /// 流派与年代字段在当前曲库为空，不参与检索（见 PRD 背景）。
    pub fn retrieve_candidates(
        conn: &Connection,
        mode: &str,
        phrase: Option<&str>,
        seed_track_id: Option<i64>,
    ) -> rusqlite::Result<Vec<Candidate>> {
        let mut ids: Vec<i64> = Vec::new();
        let mut seen: HashSet<i64> = HashSet::new();
        let push = |pool: Vec<i64>, ids: &mut Vec<i64>, seen: &mut HashSet<i64>| {
            for id in pool {
                if seen.insert(id) {
                    ids.push(id);
                    if ids.len() >= CANDIDATE_LIMIT {
                        return;
                    }
                }
            }
        };

        match mode {
            "seed" => {
                let seed_id = seed_track_id.unwrap_or(0);
                // 同艺人的曲目为主锚点
                let artist_ids: Vec<i64> = conn
                    .prepare("SELECT artist_id FROM track_artists WHERE track_id = ?1")?
                    .query_map(params![seed_id], |r| r.get(0))?
                    .filter_map(Result::ok)
                    .collect();
                if !artist_ids.is_empty() {
                    let placeholders = artist_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                    let sql = format!(
                        "SELECT DISTINCT ta.track_id FROM track_artists ta
                         WHERE ta.artist_id IN ({placeholders}) AND ta.track_id != {seed_id}
                         LIMIT 250"
                    );
                    push(
                        conn.prepare(&sql)?
                            .query_map(rusqlite::params_from_iter(artist_ids.iter()), |r| r.get(0))?
                            .filter_map(Result::ok)
                            .collect(),
                        &mut ids,
                        &mut seen,
                    );
                }
                // 同专辑
                let same_album: Vec<i64> = conn
                    .prepare(
                        "SELECT id FROM tracks
                         WHERE album_id = (SELECT album_id FROM tracks WHERE id = ?1)
                           AND id != ?1 LIMIT 60",
                    )?
                    .query_map(params![seed_id], |r| r.get(0))?
                    .filter_map(Result::ok)
                    .collect();
                push(same_album, &mut ids, &mut seen);
                // 种子标题关键词扩散
                if let Ok(seed_title) = conn.query_row("SELECT title FROM tracks WHERE id = ?1", params![seed_id], |r| r.get::<_, String>(0)) {
                    let terms = extract_keywords(&seed_title);
                    if !terms.is_empty() {
                        push(like_search(conn, &terms, 100)?, &mut ids, &mut seen);
                    }
                }
            }
            "phrase" => {
                // 关键词 LIKE 命中优先（流派为空，退化为标题/艺人文本匹配）
                if let Some(p) = phrase {
                    let terms = extract_keywords(p);
                    if !terms.is_empty() {
                        push(like_search(conn, &terms, 300)?, &mut ids, &mut seen);
                    }
                }
            }
            _ => {} // "recent"：无前置池
        }

        // 公共底池：最近播放 → 收藏 → 高频 → 随机分层补齐
        if ids.len() < CANDIDATE_LIMIT {
            let recent: Vec<i64> = conn
                .prepare("SELECT track_id FROM play_history WHERE track_id IS NOT NULL ORDER BY played_at DESC, id DESC LIMIT 120")?
                .query_map([], |r| r.get(0))?
                .filter_map(Result::ok)
                .collect();
            push(recent, &mut ids, &mut seen);
        }
        if ids.len() < CANDIDATE_LIMIT {
            let favs: Vec<i64> = conn
                .prepare("SELECT track_id FROM favorite_tracks ORDER BY favorited_at DESC LIMIT 120")?
                .query_map([], |r| r.get(0))?
                .filter_map(Result::ok)
                .collect();
            push(favs, &mut ids, &mut seen);
        }
        if ids.len() < CANDIDATE_LIMIT {
            let top: Vec<i64> = conn
                .prepare("SELECT id FROM tracks WHERE play_count > 0 ORDER BY play_count DESC, id ASC LIMIT 200")?
                .query_map([], |r| r.get(0))?
                .filter_map(Result::ok)
                .collect();
            push(top, &mut ids, &mut seen);
        }
        if ids.len() < CANDIDATE_LIMIT {
            // 曲库 32k 级 RANDOM() 全扫 ≈ 数十毫秒，每次生成一次可接受
            let need = CANDIDATE_LIMIT - ids.len();
            let rand_pool: Vec<i64> = conn
                .prepare("SELECT id FROM tracks ORDER BY RANDOM() LIMIT ?1")?
                .query_map(params![need as i64], |r| r.get(0))?
                .filter_map(Result::ok)
                .collect();
            push(rand_pool, &mut ids, &mut seen);
        }

        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // 回填标题/艺人/播放次数（保持池序）
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT t.id, t.title,
                    COALESCE((SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id), '') AS artist,
                    t.play_count
             FROM tracks t WHERE t.id IN ({placeholders})"
        );
        let mut map: std::collections::HashMap<i64, (String, String, i64)> = std::collections::HashMap::new();
        conn.prepare(&sql)?
            .query_map(rusqlite::params_from_iter(ids.iter()), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?))
            })?
            .filter_map(Result::ok)
            .for_each(|(id, title, artist, pc)| {
                map.insert(id, (title, artist, pc));
            });

        Ok(ids
            .into_iter()
            .filter_map(|id| {
                map.remove(&id).map(|(title, artist, pc)| Candidate {
                    id,
                    title,
                    artist,
                    play_count: pc,
                })
            })
            .collect())
    }

    // ================= ② prompt 组装 =================

    pub fn build_prompt(mode: &str, phrase: Option<&str>, seed: Option<&Candidate>, candidates: &[Candidate]) -> (String, String) {
        let system = "你是一个本地音乐库的私人 DJ 助手。用户会给你一个候选歌曲列表和生成情境，你从中挑选并编排一份歌单。\n\
规则：\n\
1. 只能输出候选列表中存在的 id，绝对不能编造候选之外的歌曲或 id。\n\
2. 挑选 20 到 30 首，按播放顺序排列，第一首要抓耳。\n\
3. name：歌单名，12 字以内；description：一句话简介，30 字以内。\n\
4. 每首给 reason：不超过 16 字的入选理由，要具体（点出风格、氛围或与情境的关联），不要空话。\n\
5. 宁缺毋滥：候选里不合适的就少选，但不要少于 15 首。\n\
6. 只输出一个 JSON 对象，不要输出任何其他文字：\n\
{\"name\":\"...\",\"description\":\"...\",\"tracks\":[{\"id\":123,\"reason\":\"...\"}]}";

        let mut user = String::new();
        match mode {
            "seed" => {
                if let Some(s) = seed {
                    user.push_str(&format!("生成情境：以歌曲「{} - {}」为锚点，扩展一份气质相近、适合连续收听的歌单，可以跨艺人但保持风格连贯。\n\n", s.title, s.artist));
                }
            }
            "phrase" => {
                user.push_str(&format!("生成情境：用户说——「{}」。从候选中筛出最符合这句话的歌；若完全贴合的不足，可放宽到氛围相近的。\n\n", phrase.unwrap_or("")));
            }
            _ => {
                user.push_str("生成情境：基于用户近期的收听口味，生成一份既有熟悉感又有新意的日常歌单；不要全选最近播过的，适当从候选其他部分补充。\n\n");
            }
        }
        user.push_str(&format!("候选歌曲（id|标题|艺人|播放次数），共 {} 首：\n", candidates.len()));
        for c in candidates {
            user.push_str(&format!("{}|{}|{}|{}\n", c.id, c.title, c.artist, c.play_count));
        }
        (system.to_string(), user)
    }

    // ================= ③ LLM 调用 =================

    pub async fn call_llm(settings: &AiSettings, app_dir: &Path, system: &str, user: &str) -> Result<String, String> {
        let key = settings
            .credential_ref
            .as_deref()
            .and_then(|cred| Self::load_key(app_dir, cred));

        let base = settings.base_url.trim().trim_end_matches('/');
        if base.is_empty() || settings.model.trim().is_empty() {
            return Err("模型服务未配置完整（Base URL / 模型名）".to_string());
        }
        let url = format!("{}/chat/completions", base);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(LLM_TIMEOUT_SECS))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let send = |json_mode: bool| {
            let mut body = serde_json::json!({
                "model": settings.model.trim(),
                "messages": [
                    {"role": "system", "content": system},
                    {"role": "user", "content": user}
                ],
                "temperature": settings.temperature,
            });
            if json_mode {
                body["response_format"] = serde_json::json!({"type": "json_object"});
            }
            let mut req = client.post(&url).json(&body);
            if let Some(k) = &key {
                req = req.bearer_auth(k);
            }
            req
        };

        // 优先带 response_format=json_object；部分兼容实现不支持时（400）退化为纯提示词约束
        let resp = send(true).send().await;
        let resp = match resp {
            Ok(r) if r.status() == reqwest::StatusCode::BAD_REQUEST => send(false).send().await,
            other => other,
        }
        .map_err(|e| format!("请求模型服务失败: {}", e))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
        if !status.is_success() {
            let brief: String = text.chars().take(200).collect();
            return Err(format!("模型服务返回 {}：{}", status, brief));
        }

        let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("响应不是合法 JSON: {}", e))?;
        let content = value["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| "响应缺少 choices[0].message.content".to_string())?
            .to_string();
        Ok(content)
    }

    // ================= ④ 解析与校验 =================

    /// 解析 LLM 输出并硬校验：返回的 id 必须在候选集中。失败返回 None（调用方重试/兜底）。
    pub fn parse_and_validate(
        raw: &str,
        candidates: &[Candidate],
    ) -> Option<(String, String, Vec<(i64, String)>)> {
        let json_text = extract_json(raw)?;
        let value: serde_json::Value = serde_json::from_str(&json_text).ok()?;

        let name = value["name"].as_str().unwrap_or("AI 歌单").trim().to_string();
        let description = value["description"].as_str().unwrap_or("").trim().to_string();
        let arr = value["tracks"].as_array()?;

        let valid: HashSet<i64> = candidates.iter().map(|c| c.id).collect();
        let mut picked: Vec<(i64, String)> = Vec::new();
        let mut seen: HashSet<i64> = HashSet::new();
        for item in arr {
            let id = item["id"].as_i64().or_else(|| item["id"].as_str().and_then(|s| s.parse().ok()));
            let Some(id) = id else { continue };
            if !valid.contains(&id) || !seen.insert(id) {
                continue;
            }
            let reason = item["reason"]
                .as_str()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            picked.push((id, reason.unwrap_or_default()));
            if picked.len() >= MAX_TRACKS {
                break;
            }
        }

        if picked.len() < MIN_TRACKS || name.is_empty() {
            return None;
        }
        Some((truncate_str(&name, 40), truncate_str(&description, 120), picked))
    }

    // ================= ⑤ 兜底与回表 =================

    /// 规则兜底歌单：AI 不可用/解析失败时仍给一份可用的结果。
    pub fn fallback(conn: &Connection, mode: &str, seed_track_id: Option<i64>) -> rusqlite::Result<(String, String, Vec<TrackDTO>)> {
        let (name, description, tracks) = match mode {
            "seed" => {
                let artist_id: Option<i64> = conn
                    .prepare("SELECT artist_id FROM track_artists WHERE track_id = ?1 ORDER BY position LIMIT 1")?
                    .query_row(params![seed_track_id.unwrap_or(0)], |r| r.get(0))
                    .optional()?;
                match artist_id {
                    Some(aid) => {
                        let tracks = crate::repositories::artist_repo::ArtistRepo::get_artist_tracks(conn, aid, 30, 0)?;
                        (
                            "同好之声".to_string(),
                            "AI 暂不可用，已按种子歌曲艺人自动兜底".to_string(),
                            tracks,
                        )
                    }
                    _ => Self::fallback(conn, "recent", None)?,
                }
            }
            "phrase" => {
                let tracks = crate::repositories::track_repo::TrackRepo::get_recently_added_tracks(conn, 30)?;
                ("最新入库".to_string(), "AI 暂不可用，已按最近添加自动兜底".to_string(), tracks)
            }
            _ => {
                let tracks = crate::repositories::track_repo::TrackRepo::get_recently_played_tracks(conn, 30)?;
                ("最近在听".to_string(), "AI 暂不可用，已按最近播放自动兜底".to_string(), tracks)
            }
        };
        Ok((name, description, tracks))
    }

    /// 把选中的 id 按选择顺序回表为完整 TrackDTO。
    pub fn finalize_tracks(conn: &Connection, picked: &[(i64, String)]) -> rusqlite::Result<Vec<AiRankedTrackDTO>> {
        if picked.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<i64> = picked.iter().map(|(id, _)| *id).collect();
        let by_id: std::collections::HashMap<i64, TrackDTO> = crate::repositories::track_repo::TrackRepo::get_tracks_by_ids(conn, &ids)?
            .into_iter()
            .map(|t| (t.id, t))
            .collect();
        Ok(picked
            .iter()
            .filter_map(|(id, reason)| {
                by_id.get(id).map(|t| AiRankedTrackDTO {
                    track: t.clone(),
                    reason: if reason.is_empty() { None } else { Some(reason.clone()) },
                })
            })
            .collect())
    }

    // ================= 连接测试 =================

    pub async fn test_connection(settings: &AiSettings, app_dir: &Path) -> crate::models::AiTestConnectionResult {
        let started = std::time::Instant::now();
        let base = settings.base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return crate::models::AiTestConnectionResult { ok: false, message: "请先填写 Base URL".to_string(), latency_ms: 0 };
        }
        let key = settings
            .credential_ref
            .as_deref()
            .and_then(|cred| Self::load_key(app_dir, cred));

        let client = match reqwest::Client::builder().timeout(Duration::from_secs(10)).build() {
            Ok(c) => c,
            Err(e) => return crate::models::AiTestConnectionResult { ok: false, message: format!("HTTP 客户端创建失败: {}", e), latency_ms: 0 },
        };
        let mut req = client.get(format!("{}/models", base));
        if let Some(k) = &key {
            req = req.bearer_auth(k);
        }
        match req.send().await {
            Ok(resp) => {
                let latency = started.elapsed().as_millis() as u64;
                if resp.status().is_success() {
                    crate::models::AiTestConnectionResult { ok: true, message: format!("连接成功（{} ms）", latency), latency_ms: latency }
                } else {
                    crate::models::AiTestConnectionResult { ok: false, message: format!("服务返回 {}（检查 Base URL 与 Key）", resp.status()), latency_ms: latency }
                }
            }
            Err(e) => crate::models::AiTestConnectionResult { ok: false, message: format!("连接失败: {}", e), latency_ms: started.elapsed().as_millis() as u64 },
        }
    }
}

/// 从短语提取检索关键词：CJK 连续段取整段（2-4 字）或二元组（更长时），
/// 西文/数字按非字母数字切词（≥2 字符）。最多 12 个词，供标题/艺人 LIKE 检索。
fn extract_keywords(phrase: &str) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    let mut push_unique = |t: &str, terms: &mut Vec<String>| {
        if !t.is_empty() && !terms.iter().any(|x| x == t) && terms.len() < 12 {
            terms.push(t.to_string());
        }
    };

    let mut cjk_run = String::new();
    for ch in phrase.chars() {
        if is_cjk(ch) {
            cjk_run.push(ch);
            continue;
        }
        if !cjk_run.is_empty() {
            flush_cjk_run(&cjk_run, &mut terms, &mut push_unique);
            cjk_run.clear();
        }
    }
    if !cjk_run.is_empty() {
        flush_cjk_run(&cjk_run, &mut terms, &mut push_unique);
    }

    for word in phrase.split(|c: char| !(c.is_alphanumeric() && !is_cjk(c))) {
        let w = word.trim();
        if w.chars().count() >= 2 && !w.chars().all(is_cjk) {
            push_unique(w, &mut terms);
        }
    }

    terms
}

fn flush_cjk_run(run: &str, terms: &mut Vec<String>, push_unique: &mut dyn FnMut(&str, &mut Vec<String>)) {
    let chars: Vec<char> = run.chars().collect();
    if (2..=4).contains(&chars.len()) {
        push_unique(run, terms);
        return;
    }
    if chars.len() > 4 {
        // 整段太长无法作为检索词，退化为二元组
        for w in chars.windows(2) {
            let t: String = w.iter().collect();
            push_unique(&t, terms);
        }
    }
    // 单字 CJK 段（1 字）不做检索
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x3040..=0x309F | 0x30A0..=0x30FF | 0xF900..=0xFAFF)
}

/// LIKE 检索标题/艺人（关键词做 LIKE 转义，OR 连接）
fn like_search(conn: &Connection, terms: &[String], limit: i64) -> rusqlite::Result<Vec<i64>> {
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    // 艺人名不内联（tracks 无 artist_name 列）：先在 artists 表按名检索，
    // 再经 track_artists 索引取曲目；标题直接 LIKE。每个词产生 2 组参数。
    let clauses = terms
        .iter()
        .map(|_| "(t.title LIKE ? OR t.id IN (SELECT ta.track_id FROM track_artists ta WHERE ta.artist_id IN (SELECT id FROM artists WHERE name LIKE ?)))")
        .collect::<Vec<_>>()
        .join(" OR ");
    let sql = format!(
        "SELECT t.id FROM tracks t
         WHERE {clauses}
         ORDER BY t.play_count DESC, t.id ASC
         LIMIT ?"
    );
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    for t in terms {
        let pattern = format!("%{}%", crate::repositories::escape_like(t));
        param_values.push(Box::new(pattern.clone()));
        param_values.push(Box::new(pattern));
    }
    param_values.push(Box::new(limit));
    let refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();
    let ids: Vec<i64> = conn.prepare(&sql)?
        .query_map(refs.as_slice(), |r| r.get::<_, i64>(0))?
        .filter_map(Result::ok)
        .collect();
    Ok(ids)
}

/// 从 LLM 输出中剥出 JSON（容忍 ```json 围栏与前后杂文）
fn extract_json(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end < start {
        return None;
    }
    Some(trimmed[start..=end].to_string())
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        s.chars().take(max_chars).collect()
    }
}

impl AiSettings {
    /// 校验必填项，返回可读错误（None 表示通过）
    pub fn validate(&self) -> Option<String> {
        if !self.enabled {
            return Some(ERR_NOT_ENABLED.to_string());
        }
        if self.base_url.trim().is_empty() || self.model.trim().is_empty() {
            return Some("模型服务未配置完整，请在「设置 → AI 推荐」中填写 Base URL 与模型名".to_string());
        }
        None
    }
}
