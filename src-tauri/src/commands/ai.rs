//! AI 推荐歌单命令（PRD-AI推荐歌单 2026-09-12）。
//!
//! 编排：prepare（同步 DB）→ call_llm（异步，可取消）→ parse_and_validate
//! （失败重试一次）→ finalize / fallback。所有 DB 访问都在同步块内完成，
//! rusqlite Connection 非 Send，不能跨 await 持有。

use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use crate::models::{AiPlaylistResult, AiRankedTrackDTO, AiSettingsDTO, AiTestConnectionResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use tokio::sync::Notify;

/// 进行中的生成请求：request_id → 取消信号。生成结束（成功/失败/取消）即移除。
static CANCEL_MAP: Mutex<Option<HashMap<String, Arc<Notify>>>> = Mutex::new(None);

#[tauri::command]
pub fn ai_get_settings(db_state: State<'_, DbState>) -> Result<AiSettingsDTO, AppError> {
    let _trace = ipc_trace!("ai_get_settings");
    let conn = db_state.db.get()?;
    let s = crate::services::ai::AiService::load_settings(&conn)?;
    Ok(AiSettingsDTO {
        enabled: s.enabled,
        base_url: s.base_url,
        model: s.model,
        temperature: s.temperature,
        has_key: s.credential_ref.is_some(),
    })
}

/// 保存设置。api_key 语义：null=保持不变，""=清除已存 Key，其他=重新存储。
#[tauri::command]
pub fn ai_save_settings(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
    enabled: bool,
    base_url: String,
    model: String,
    temperature: Option<f64>,
    api_key: Option<String>,
) -> Result<AiSettingsDTO, AppError> {
    let _trace = ipc_trace!("ai_save_settings");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let conn = db_state.db.get()?;
    crate::services::ai::AiService::save_settings(
        &conn,
        &app_dir,
        enabled,
        &base_url,
        &model,
        temperature.unwrap_or(0.8),
        api_key,
    )?;
    let s = crate::services::ai::AiService::load_settings(&conn)?;
    Ok(AiSettingsDTO {
        enabled: s.enabled,
        base_url: s.base_url,
        model: s.model,
        temperature: s.temperature,
        has_key: s.credential_ref.is_some(),
    })
}

#[tauri::command(async)]
pub async fn ai_test_connection(app: tauri::AppHandle, db_state: State<'_, DbState>) -> Result<AiTestConnectionResult, AppError> {
    let _trace = ipc_trace!("ai_test_connection");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let settings = {
        let conn = db_state.db.get()?;
        crate::services::ai::AiService::load_settings(&conn)?
    };
    Ok(crate::services::ai::AiService::test_connection(&settings, &app_dir).await)
}

/// 生成 AI 推荐歌单。`request_id` 由前端生成，用于 `ai_cancel_generate` 取消。
#[tauri::command(async)]
pub async fn ai_generate_playlist(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
    request_id: String,
    mode: String,
    phrase: Option<String>,
    seed_track_id: Option<i64>,
) -> Result<AiPlaylistResult, AppError> {
    let _trace = ipc_trace!("ai_generate_playlist");
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let notify = Arc::new(Notify::new());
    CANCEL_MAP
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?
        .get_or_insert_with(HashMap::new)
        .insert(request_id.clone(), notify.clone());

    let result = generate_inner(&db_state, &app_dir, &notify, &mode, phrase.as_deref(), seed_track_id).await;

    CANCEL_MAP
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?
        .get_or_insert_with(HashMap::new)
        .remove(&request_id);

    result
}

/// 取消进行中的生成。发出取消信号后，生成侧以「已取消」错误返回。
#[tauri::command]
pub fn ai_cancel_generate(request_id: String) -> Result<(), AppError> {
    let _trace = ipc_trace!("ai_cancel_generate");
    if let Some(notify) = CANCEL_MAP
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?
        .get_or_insert_with(HashMap::new)
        .get(&request_id)
        .cloned()
    {
        notify.notify_waiters();
    }
    Ok(())
}

async fn generate_inner(
    db_state: &State<'_, DbState>,
    app_dir: &std::path::Path,
    notify: &Arc<Notify>,
    mode: &str,
    phrase: Option<&str>,
    seed_track_id: Option<i64>,
) -> Result<AiPlaylistResult, AppError> {
    // ===== Phase 1（同步）：设置校验 + 候选检索 + prompt 组装 =====
    let prepared = {
        let conn = db_state.db.get()?;
        let settings = crate::services::ai::AiService::load_settings(&conn)?;
        if let Some(err) = settings.validate() {
            return Err(AppError::Internal(err));
        }
        let candidates = crate::services::ai::AiService::retrieve_candidates(&conn, mode, phrase, seed_track_id)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let seed = if mode == "seed" {
            candidates.iter().find(|c| Some(c.id) == seed_track_id).cloned()
        } else {
            None
        };
        let (system, user) = crate::services::ai::AiService::build_prompt(mode, phrase, seed.as_ref(), &candidates);
        (settings, candidates, system, user)
    };
    let (settings, candidates, system, user) = prepared;

    if candidates.is_empty() {
        return Ok(make_fallback(db_state, mode, seed_track_id, "曲库为空，没有可推荐的候选")?);
    }

    // ===== Phase 2（异步）：LLM 调用 + 解析校验，失败重试一次，全程可取消 =====
    let mut picked: Option<(String, String, Vec<(i64, String)>)> = None;
    let mut last_err = String::new();

    for attempt in 0..2 {
        let call = crate::services::ai::AiService::call_llm(&settings, app_dir, &system, &user);
        let raw = tokio::select! {
            r = call => r,
            _ = notify.notified() => {
                return Err(AppError::Internal("已取消".to_string()));
            }
        };
        match raw.and_then(|text| {
            crate::services::ai::AiService::parse_and_validate(&text, &candidates)
                .ok_or_else(|| "输出未能通过校验（JSON 不合法或命中候选不足）".to_string())
        }) {
            Ok(p) => {
                picked = Some(p);
                break;
            }
            Err(e) => {
                last_err = e;
                tracing::warn!("[ai] 生成第 {} 次尝试失败: {}", attempt + 1, last_err);
            }
        }
    }

    // ===== Phase 3（同步）：回表或兜底 =====
    match picked {
        Some((name, description, tracks)) => {
            let conn = db_state.db.get()?;
            let finalized = crate::services::ai::AiService::finalize_tracks(&conn, &tracks)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            if finalized.is_empty() {
                return Ok(make_fallback(db_state, mode, seed_track_id, "AI 结果回表为空")?);
            }
            Ok(AiPlaylistResult { name, description, source: "ai".to_string(), degraded_reason: None, tracks: finalized })
        }
        None => Ok(make_fallback(db_state, mode, seed_track_id, &format!("AI 输出未通过校验（{}），已降级为规则歌单", last_err))?),
    }
}

fn make_fallback(
    db_state: &State<'_, DbState>,
    mode: &str,
    seed_track_id: Option<i64>,
    reason: &str,
) -> Result<AiPlaylistResult, AppError> {
    let conn = db_state.db.get()?;
    let (name, description, tracks) = crate::services::ai::AiService::fallback(&conn, mode, seed_track_id)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    tracing::warn!("[ai] 降级为规则歌单: {}", reason);
    Ok(AiPlaylistResult {
        name,
        description,
        source: "fallback".to_string(),
        degraded_reason: Some(reason.to_string()),
        tracks: tracks.into_iter().map(|t| AiRankedTrackDTO { track: t, reason: None }).collect(),
    })
}
