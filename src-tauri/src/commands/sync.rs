use crate::db::DbState;
use crate::error::AppError;
use crate::ipc_trace;
use crate::models::{RemoteCheckResult, SyncConfigDTO, SyncResult};
use crate::services::sync::SyncService;
use crate::services::webdav::WebdavFile;
use std::path::{Path, PathBuf};
use tauri::{Manager, State};

// ========================= 配置读写 =========================

#[tauri::command]
pub fn sync_get_config(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
) -> Result<SyncConfigDTO, AppError> {
    let conn = db_state.db.get()?;
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let machine_key = crate::commands::scanner::derive_credential_key(&app_dir);
    let config = SyncService::get_config(&conn, &machine_key)?;
    Ok(config)
}

#[tauri::command]
pub fn sync_save_config(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
    config: SyncConfigDTO,
) -> Result<(), AppError> {
    let conn = db_state.db.get()?;
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let machine_key = crate::commands::scanner::derive_credential_key(&app_dir);
    SyncService::save_config(&conn, &config, &machine_key)?;
    Ok(())
}

// ========================= 文件夹浏览器 =========================

/// 浏览 WebDAV 目录树（PROPFIND 过滤后仅返回目录）。
/// url / username / password 来自同步配置，前端在调用前已从 config 读取。
#[tauri::command]
pub fn sync_browse_webdav(
    url: String,
    username: Option<String>,
    password: Option<String>,
    path: String,
) -> Result<Vec<WebdavFile>, AppError> {
    let config = SyncConfigDTO {
        enabled: true,
        webdav_url: Some(url),
        username,
        password,
        remote_path: None,
        last_sync_at: None,
        last_sync_direction: None,
    };
    let files = SyncService::browse(&config, &path)?;
    Ok(files)
}

/// 在 WebDAV 上新建文件夹（文件夹浏览器内使用）。
#[tauri::command]
pub fn sync_create_folder(
    url: String,
    username: Option<String>,
    password: Option<String>,
    path: String,
) -> Result<(), AppError> {
    let config = SyncConfigDTO {
        enabled: true,
        webdav_url: Some(url),
        username,
        password,
        remote_path: None,
        last_sync_at: None,
        last_sync_direction: None,
    };
    SyncService::create_remote_folder(&config, &path)?;
    Ok(())
}

// ========================= 同步操作 =========================

/// live 库整库替换的四个动作。真实实现走 rusqlite 的在线 Backup API；
/// 单测用替身按脚本放行/报错，为的是覆盖「替换失败且回滚同样失败」这条分支——
/// 它是本应用唯一会造成不可逆用户数据损失的路径，不该只靠人工演练兜（CR-001）。
trait LiveDatabase {
    /// 把当前 live 库完整复制到 `backup`，作为回滚依据。
    fn snapshot_to(&mut self, backup: &Path) -> Result<(), String>;
    /// 用 `source` 整库替换 live 库。
    fn replace_with(&mut self, source: &Path) -> Result<(), String>;
    /// 用 `backup` 把 live 库还原回替换前的状态。
    fn restore_from(&mut self, backup: &Path) -> Result<(), String>;
    /// 记录本次恢复时间与方向。
    fn mark_restored(&mut self, at: &str) -> Result<(), String>;
}

struct LiveBackupRestorer<'a> {
    conn: &'a mut rusqlite::Connection,
}

impl LiveDatabase for LiveBackupRestorer<'_> {
    fn snapshot_to(&mut self, backup: &Path) -> Result<(), String> {
        self.conn
            .backup(rusqlite::DatabaseName::Main, backup, None)
            .map_err(|e| e.to_string())
    }

    fn replace_with(&mut self, source: &Path) -> Result<(), String> {
        self.conn
            .restore(rusqlite::DatabaseName::Main, source, Some(|_| {}))
            .map_err(|e| e.to_string())
    }

    fn restore_from(&mut self, backup: &Path) -> Result<(), String> {
        self.conn
            .restore(rusqlite::DatabaseName::Main, backup, Some(|_| {}))
            .map_err(|e| e.to_string())
    }

    fn mark_restored(&mut self, at: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE sync_config SET last_sync_at = ?1, last_sync_direction = 'download' WHERE id = 1",
                rusqlite::params![at],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// 整库恢复的结果。三个变体的真正区别只有一件事：**本轮的恢复前副本保不保留**。
#[derive(Debug)]
enum RestoreOutcome {
    /// 恢复成功，副本已无保留价值（本轮副本已由 [`restore_with_backup`] 清理）。
    Success,
    /// 失败，但 live 库状态可信（替换从未成功且已回滚成功，或副本压根没建成）：副本已清理。
    Failed { message: String },
    /// 替换失败且回滚同样失败：这份副本是用户唯一的历史数据，必须保留。
    Preserved {
        message: String,
        backup_path: PathBuf,
    },
}

/// 已下载快照的作用域清理器。
///
/// 恢复命令在下载完成后还有校验、连接池取连接、在线备份与替换等多个可能提前返回的步骤；
/// 把删除动作绑到 Drop，才能保证任何 `?` 都不会把一份完整数据库遗留在数据目录里。
struct DownloadedSnapshotGuard(PathBuf);

impl DownloadedSnapshotGuard {
    fn new(path: PathBuf) -> Self {
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for DownloadedSnapshotGuard {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.0) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("恢复下载临时文件清理失败 {}: {}", self.0.display(), e);
            }
        }
    }
}

/// 把已校验的快照灌进 live 库，并为失败留下退路。
///
/// 副本删除口径（CR-001）：只有恢复成功、或失败后**已用副本回滚成功**，才允许删除本轮副本；
/// 双失败一律保留，且把真实路径写进给用户看的文案。每轮副本文件名唯一，
/// 因此任何一次失败恢复都不会覆盖上一次留下的救援副本。
fn restore_with_backup(
    db: &mut dyn LiveDatabase,
    app_dir: &Path,
    temp_path: &Path,
) -> RestoreOutcome {
    let backup_path = SyncService::rescue_backup_path(app_dir);
    if let Err(e) = db.snapshot_to(&backup_path) {
        // 在线 Backup API 直接写目标文件；磁盘满或 I/O 中断时可能先留下半截文件再报错。
        // live 库尚未被碰过，这种文件没有救援价值，必须删掉，不能被后续列表冒充成可恢复副本。
        discard_rescue_copy(&backup_path);
        return RestoreOutcome::Failed {
            message: format!("备份当前数据库失败，本地库未做任何改动: {}", e),
        };
    }

    let replaced = db
        .replace_with(temp_path)
        .and_then(|()| db.mark_restored(&restored_timestamp()));
    let Err(cause) = replaced else {
        discard_rescue_copy(&backup_path);
        return RestoreOutcome::Success;
    };

    // 回滚失败是这条链路上最危险的分支：live 库可能停在半恢复状态，
    // 而唯一能救回的副本就在 backup_path。绝不能把它的失败吞掉。
    match db.restore_from(&backup_path) {
        Ok(()) => {
            discard_rescue_copy(&backup_path);
            RestoreOutcome::Failed {
                message: format!("恢复数据库失败，已回滚到恢复前的本地副本: {}", cause),
            }
        }
        Err(e) => RestoreOutcome::Preserved {
            message: format!(
                "恢复数据库失败（{}），自动回滚也失败（{}）。请先停止使用备份恢复功能：恢复前的副本仍保留在 {}，可手动还原",
                cause,
                e,
                backup_path.display()
            ),
            backup_path,
        },
    }
}

/// 清理本轮不再有价值的副本：恢复成功、回滚成功，或副本创建中途失败时调用。
/// 删除失败就原地保留并告警：宁可多占磁盘，也不谎称文件已经被清理掉了。
fn discard_rescue_copy(path: &Path) {
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("恢复前副本清理失败，仍保留在 {}: {}", path.display(), e);
        }
    }
    // SQLite 在异常退出路径上可能留下辅助文件；它们与不完整主文件一样没有恢复价值。
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let sidecar = PathBuf::from(sidecar);
        if let Err(e) = std::fs::remove_file(&sidecar) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("恢复前副本辅助文件清理失败 {}: {}", sidecar.display(), e);
            }
        }
    }
}

fn restored_timestamp() -> String {
    use chrono::Utc;
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 把「数据目录里还留着几份没处理的恢复前副本」写成一句用户看得懂的话。
/// 救援副本只能由成功流程或用户明确操作删除，所以这里只告知位置，绝不清理。
/// `already_named` 是本轮文案里已经点名的那份，不重复列。
fn rescue_copies_hint(app_dir: &Path, already_named: Option<&Path>) -> String {
    let copies: Vec<PathBuf> = SyncService::list_rescue_backups(app_dir)
        .into_iter()
        .filter(|path| Some(path.as_path()) != already_named)
        .collect();
    match copies.len() {
        0 => String::new(),
        1 => format!(
            "。另有 1 份此前失败留下的恢复前副本尚未处理，可手动还原或删除：{}",
            copies[0].display()
        ),
        n => format!(
            "。另有 {} 份此前失败留下的恢复前副本尚未处理，可手动还原或删除：{}",
            n,
            copies
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("、")
        ),
    }
}

/// 立即同步上传：VACUUM INTO → PUT 到远程路径。
#[tauri::command]
pub fn sync_upload_now(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
) -> Result<SyncResult, AppError> {
    let _trace = ipc_trace!("sync_upload_now");
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let conn = db_state.db.get()?;
    let machine_key = crate::commands::scanner::derive_credential_key(&app_dir);
    let config = SyncService::get_config(&conn, &machine_key)?;
    if !config.enabled {
        return Err(AppError::Internal(
            "同步未启用，请在设置中配置并启用".to_string(),
        ));
    }
    let result = SyncService::sync_upload(&conn, &app_dir, &config)?;
    Ok(result)
}

/// Restore a validated snapshot into the live pool without replacing Tauri-managed state.
#[tauri::command]
pub fn sync_restore_now(
    app: tauri::AppHandle,
    db_state: State<'_, DbState>,
) -> Result<String, AppError> {
    let _trace = ipc_trace!("sync_restore_now");
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let machine_key = crate::commands::scanner::derive_credential_key(&app_dir);
    let config = {
        let conn = db_state.db.get()?;
        SyncService::get_config(&conn, &machine_key)?
    };
    if !config.enabled {
        return Err(AppError::Internal(
            "同步未启用，请在设置中配置并启用".to_string(),
        ));
    }

    let temp_snapshot =
        DownloadedSnapshotGuard::new(SyncService::sync_download_to_temp(&app_dir, &config)?);
    // 空文件 / HTML 错误页 / 非 Lumo 库 / 版本越界 / integrity_check 不通过
    // 全部由 validate_snapshot 拒绝，此时本地库尚未被触碰。
    if let Err(e) = SyncService::validate_snapshot(temp_snapshot.path()) {
        return Err(AppError::Internal(e));
    }

    // 上一次失败留下的救援副本既不删也不覆盖（CR-001）：本轮副本另起一个唯一文件名，
    // 开工前只把它们记进日志，收尾时随结果文案一并告知用户。
    let pending_before = SyncService::list_rescue_backups(&app_dir);
    if !pending_before.is_empty() {
        tracing::warn!(
            "恢复前发现 {} 份未处理的救援副本，本次不会覆盖：{}",
            pending_before.len(),
            pending_before
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("、")
        );
    }

    // 在线 Backup API：既保持受管 DbState 有效，又给回滚留一份副本。
    // 整库替换 + 回滚 + 本轮副本的取舍全在 restore_with_backup 里，三种结局对应三种话术。
    let outcome = {
        let mut conn = db_state.db.get()?;
        let mut restorer = LiveBackupRestorer { conn: &mut conn };
        restore_with_backup(&mut restorer, &app_dir, temp_snapshot.path())
    };

    match &outcome {
        RestoreOutcome::Success => Ok(format!(
            "数据库已通过校验并从云端恢复成功{}",
            rescue_copies_hint(&app_dir, None)
        )),
        RestoreOutcome::Failed { message } => {
            tracing::warn!("从云端恢复未完成（本地库仍可用）：{}", message);
            Err(AppError::Internal(format!(
                "{}{}",
                message,
                rescue_copies_hint(&app_dir, None)
            )))
        }
        RestoreOutcome::Preserved {
            message,
            backup_path,
        } => {
            // 保留：它此刻是用户唯一的本地历史数据。
            tracing::error!(
                "从云端恢复失败且自动回滚失败，救援副本保留于 {}",
                backup_path.display()
            );
            Err(AppError::Internal(format!(
                "{}{}",
                message,
                rescue_copies_hint(&app_dir, Some(backup_path))
            )))
        }
    }
}

/// 检查云端是否有同步数据（首次开启同步时检测用）。
#[tauri::command]
pub fn sync_check_remote(
    url: String,
    username: Option<String>,
    password: Option<String>,
    path: String,
) -> Result<RemoteCheckResult, AppError> {
    let config = SyncConfigDTO {
        enabled: true,
        webdav_url: Some(url),
        username,
        password,
        remote_path: Some(path),
        last_sync_at: None,
        last_sync_direction: None,
    };
    let result = SyncService::check_remote(&config)?;
    Ok(result)
}

// ========================= 测试 =========================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_util::TempDir;
    use rusqlite::Connection;
    use std::cell::RefCell;

    /// 每一步的预设结果。全部 `Ok` 即「一切顺利」，改哪一步就模拟哪一步失败。
    #[derive(Clone)]
    struct Script {
        snapshot: Result<(), String>,
        replace: Result<(), String>,
        rollback: Result<(), String>,
        mark: Result<(), String>,
    }

    impl Script {
        fn all_ok() -> Self {
            Self {
                snapshot: Ok(()),
                replace: Ok(()),
                rollback: Ok(()),
                mark: Ok(()),
            }
        }
    }

    /// live 库替身：按脚本放行/报错，并留下动作轨迹。
    /// `snapshot_to` 会把一份带轮次标记的内容写到副本路径，用来证明
    /// 「后一次失败没有覆盖前一次留下的救援副本」。
    struct FakeDb {
        script: Script,
        rounds: RefCell<u32>,
        actions: RefCell<Vec<String>>,
    }

    impl FakeDb {
        fn new(script: Script) -> Self {
            Self {
                script,
                rounds: RefCell::new(0),
                actions: RefCell::new(Vec::new()),
            }
        }

        fn record(&self, action: &str) {
            self.actions.borrow_mut().push(action.to_string());
        }
    }

    impl LiveDatabase for FakeDb {
        fn snapshot_to(&mut self, backup: &Path) -> Result<(), String> {
            self.record(&format!("snapshot:{}", backup.display()));
            if let Err(e) = self.script.snapshot.clone() {
                // 模拟 SQLite 已创建并写入部分目标文件后才遇到磁盘/I/O 错误。
                std::fs::write(backup, "partial rescue copy").map_err(|e| e.to_string())?;
                return Err(e);
            }
            let round = {
                let mut r = self.rounds.borrow_mut();
                *r += 1;
                *r
            };
            std::fs::write(backup, format!("rescue copy #{}", round)).map_err(|e| e.to_string())?;
            Ok(())
        }

        fn replace_with(&mut self, source: &Path) -> Result<(), String> {
            self.record(&format!("replace:{}", source.display()));
            self.script.replace.clone()
        }

        fn restore_from(&mut self, backup: &Path) -> Result<(), String> {
            self.record(&format!("rollback:{}", backup.display()));
            self.script.rollback.clone()
        }

        fn mark_restored(&mut self, at: &str) -> Result<(), String> {
            self.record(&format!("mark:{}", at));
            self.script.mark.clone()
        }
    }

    fn rollback_failed_script() -> Script {
        Script {
            replace: Err("服务端把库换成了半截文件".to_string()),
            ..Script::all_ok()
        }
    }

    fn double_failure_script() -> Script {
        Script {
            rollback: Err("磁盘已满".to_string()),
            ..rollback_failed_script()
        }
    }

    /// 伪造一份「已通过校验、等着灌进 live 库的下载快照」。
    fn downloaded_snapshot(dir: &TempDir) -> PathBuf {
        let temp = dir.path().join("lumo.sqlite.1-1.download");
        std::fs::write(&temp, "snapshot bytes").unwrap();
        temp
    }

    fn run(dir: &TempDir, script: Script) -> RestoreOutcome {
        let temp = downloaded_snapshot(dir);
        restore_with_backup(&mut FakeDb::new(script), dir.path(), &temp)
    }

    /// 恢复成功 → 本轮副本由成功流程清理，不留垃圾。
    #[test]
    fn success_discards_this_rounds_copy() {
        let dir = TempDir::new("restore_ok");
        let outcome = run(&dir, Script::all_ok());
        assert!(
            matches!(&outcome, RestoreOutcome::Success),
            "恢复成功却返回了失败结果: {:?}",
            outcome
        );
        assert!(
            SyncService::list_rescue_backups(dir.path()).is_empty(),
            "成功恢复后不应留下恢复前副本"
        );
    }

    /// 替换失败但回滚成功 → live 库可信，副本随失败流程清理，文案说明已回滚。
    #[test]
    fn rollback_success_discards_copy_and_reports_it() {
        let dir = TempDir::new("restore_rolled_back");
        let outcome = run(&dir, rollback_failed_script());
        let RestoreOutcome::Failed { message } = &outcome else {
            panic!("应返回可清理的失败结果: {:?}", outcome);
        };
        assert!(message.contains("已回滚"), "文案失真: {}", message);
        assert!(
            SyncService::list_rescue_backups(dir.path()).is_empty(),
            "回滚成功后不应继续占着整库副本"
        );
    }

    /// CR-001 的核心分支：替换失败**且**回滚也失败 → 副本必须保留，路径必须真实可查。
    #[test]
    fn double_failure_keeps_the_copy_and_names_its_path() {
        let dir = TempDir::new("restore_double_fail");
        let outcome = run(&dir, double_failure_script());
        let RestoreOutcome::Preserved {
            message,
            backup_path,
        } = &outcome
        else {
            panic!("双失败必须返回保留结果: {:?}", outcome);
        };
        assert!(backup_path.exists(), "唯一的历史数据被删掉了");
        assert_eq!(
            std::fs::read_to_string(backup_path).unwrap(),
            "rescue copy #1"
        );
        assert!(
            message.contains(&backup_path.display().to_string()),
            "文案没给出副本路径: {}",
            message
        );
        assert!(message.contains("自动回滚也失败"), "文案失真: {}", message);
    }

    /// 连开三次都失败，第三次的救援副本不会盖掉前两次；三份内容各自独立可查。
    #[test]
    fn repeated_failed_restores_never_overwrite_earlier_copies() {
        let dir = TempDir::new("restore_repeat");
        let temp = downloaded_snapshot(&dir);
        // 同一个替身跑三轮：轮次标记才能区分「第几次的副本」
        let mut db = FakeDb::new(double_failure_script());
        for _ in 0..3 {
            let outcome = restore_with_backup(&mut db, dir.path(), &temp);
            assert!(
                matches!(&outcome, RestoreOutcome::Preserved { .. }),
                "双失败必须保留副本: {:?}",
                outcome
            );
        }
        let copies = SyncService::list_rescue_backups(dir.path());
        assert_eq!(copies.len(), 3, "每轮失败都应留下各自独立的副本");
        let mut contents: Vec<String> = copies
            .iter()
            .map(|p| std::fs::read_to_string(p).unwrap())
            .collect();
        contents.sort();
        assert_eq!(
            contents,
            vec![
                "rescue copy #1".to_string(),
                "rescue copy #2".to_string(),
                "rescue copy #3".to_string()
            ],
            "后一次失败覆盖了前一次的历史数据"
        );
        // 失败流程绝不替用户删任何东西：救援副本的删除只能由成功流程或用户明确操作触发
        assert!(copies.iter().all(|p| p.exists()));
    }

    /// 副本都没建成时，不能谎称「副本保留在某处」，也不能去替换 live 库。
    #[test]
    fn snapshot_failure_claims_no_rescue_copy() {
        let dir = TempDir::new("restore_no_snapshot");
        let temp = downloaded_snapshot(&dir);
        let mut db = FakeDb::new(Script {
            snapshot: Err("磁盘不可用".to_string()),
            ..Script::all_ok()
        });
        let outcome = restore_with_backup(&mut db, dir.path(), &temp);
        let RestoreOutcome::Failed { message } = &outcome else {
            panic!("副本没建成应返回失败结果: {:?}", outcome);
        };
        assert!(message.contains("未做任何改动"), "文案失真: {}", message);
        assert!(!message.contains("回滚"), "文案失真: {}", message);
        assert!(SyncService::list_rescue_backups(dir.path()).is_empty());
        let actions = db.actions.borrow();
        assert!(
            actions.iter().all(|a| a.starts_with("snapshot:")),
            "副本没建成就不该再动 live 库: {:?}",
            actions
        );
    }

    #[test]
    fn downloaded_snapshot_guard_removes_file_on_early_return() {
        let dir = TempDir::new("restore_download_guard");
        let path = dir.path().join("lumo.sqlite.test.download");
        std::fs::write(&path, "complete downloaded database").unwrap();

        {
            let guard = DownloadedSnapshotGuard::new(path.clone());
            assert!(guard.path().exists());
            // 模拟连接池获取失败等任意 `?` 提前返回：离开作用域即清理。
        }

        assert!(!path.exists(), "提前返回后不能遗留完整下载快照");
    }

    /// `mark_restored` 失败同样必须回滚到刚建的副本：否则 live 库被换掉、时间戳没记上，也没人还原它。
    #[test]
    fn mark_failure_rolls_back_to_the_copy_it_just_made() {
        let dir = TempDir::new("restore_mark_fails");
        let temp = downloaded_snapshot(&dir);
        let mut db = FakeDb::new(Script {
            mark: Err("只读事务".to_string()),
            ..Script::all_ok()
        });
        let outcome = restore_with_backup(&mut db, dir.path(), &temp);
        let RestoreOutcome::Failed { message } = &outcome else {
            panic!("mark 失败应回滚并返回失败: {:?}", outcome);
        };
        let actions = db.actions.borrow();
        let backup = actions
            .iter()
            .find_map(|a| a.strip_prefix("snapshot:"))
            .expect("应先建副本");
        assert!(
            actions.contains(&format!("rollback:{}", backup)),
            "回滚目标不是本轮副本: {:?}",
            actions
        );
        assert!(message.contains("已回滚"), "文案失真: {}", message);
        assert!(!Path::new(backup).exists(), "回滚成功的副本应当清理掉");
    }

    /// 成功文案也要如实告知：上一次失败留下的副本没被本次操作动过。
    #[test]
    fn rescue_copies_hint_lists_leftover_copies_only() {
        let dir = TempDir::new("restore_hint");
        let leftover = dir.path().join("lumo.sqlite.1-1.restore_bak");
        std::fs::write(&leftover, "old history").unwrap();

        let outcome = run(&dir, Script::all_ok());
        assert!(matches!(&outcome, RestoreOutcome::Success), "{:?}", outcome);
        let hint = rescue_copies_hint(dir.path(), None);
        assert!(
            hint.contains(&leftover.display().to_string()),
            "文案: {}",
            hint
        );
        assert_eq!(std::fs::read_to_string(&leftover).unwrap(), "old history");

        // 本轮保留的那份不重复列（它已经写在 message 里）
        let kept = run(&dir, double_failure_script());
        let RestoreOutcome::Preserved { backup_path, .. } = &kept else {
            panic!("应保留: {:?}", kept);
        };
        let hint = rescue_copies_hint(dir.path(), Some(backup_path));
        assert!(
            !hint.contains(&backup_path.display().to_string()),
            "文案: {}",
            hint
        );
        assert_eq!(hint.matches("lumo.sqlite.").count(), 1, "文案: {}", hint);
    }

    /// 真实 rusqlite 链路：副本必须是「能独立打开、能读回原数据」的 SQLite 文件，
    /// 否则错误文案里那句「可手动还原」就是空话。
    #[test]
    fn real_backup_copy_is_an_independently_openable_database() {
        let dir = TempDir::new("restore_real_copy");
        let mut conn = Connection::open(dir.db_path()).unwrap();
        conn.execute_batch(
            "CREATE TABLE tracks (id INTEGER PRIMARY KEY, title TEXT);
             INSERT INTO tracks (title) VALUES ('before restore');",
        )
        .unwrap();

        let backup = SyncService::rescue_backup_path(dir.path());
        LiveBackupRestorer { conn: &mut conn }
            .snapshot_to(&backup)
            .expect("真实备份应当成功");

        let reopened = Connection::open(&backup).unwrap();
        let title: String = reopened
            .query_row("SELECT title FROM tracks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(title, "before restore");
    }

    /// 真实 rusqlite 链路 + 一份坏快照：替换失败 → 回滚成功 → 本地数据原封不动。
    #[test]
    fn real_restore_of_invalid_snapshot_rolls_back_and_keeps_data() {
        let dir = TempDir::new("restore_real_rollback");
        let mut conn = Connection::open(dir.db_path()).unwrap();
        conn.execute_batch(
            "CREATE TABLE tracks (id INTEGER PRIMARY KEY, title TEXT);
             INSERT INTO tracks (title) VALUES ('keep me');",
        )
        .unwrap();

        // 非 SQLite 内容：替换阶段就会失败，从而走真实的回滚分支
        let garbage = dir.path().join("lumo.sqlite.9-9.download");
        std::fs::write(&garbage, b"definitely not a sqlite database").unwrap();

        let outcome = restore_with_backup(
            &mut LiveBackupRestorer { conn: &mut conn },
            dir.path(),
            &garbage,
        );
        let RestoreOutcome::Failed { message } = &outcome else {
            panic!("坏快照应替换失败并回滚: {:?}", outcome);
        };
        assert!(message.contains("已回滚"), "文案失真: {}", message);
        let title: String = conn
            .query_row("SELECT title FROM tracks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(title, "keep me", "回滚后本地数据必须还在");
        assert!(
            SyncService::list_rescue_backups(dir.path()).is_empty(),
            "回滚成功的副本应由失败流程清理"
        );
    }

    /// 救援副本的识别不能把别的文件当成它——误判会导致误删或误报。
    #[test]
    fn list_rescue_backups_only_matches_our_own_pattern() {
        let dir = TempDir::new("rescue_listing");
        // 前两个是本轮起的唯一名；最后一个是 v1.8.1 及更早的固定名遗留副本，
        // 老用户数据目录里可能真有，必须一起告知而不是当作陌生文件忽略。
        let wanted = [
            "lumo.sqlite.1-100.restore_bak",
            "lumo.sqlite.2-200.restore_bak",
            "lumo.sqlite.restore_bak",
        ];
        let not_wanted = [
            "lumo.sqlite",
            "lumo.sqlite.sha256",
            "lumo.sqlite.1-1.download",
            "other.sqlite.1-1.restore_bak",
            "lumo.sqlite.1-1.restore_bak.tmp",
        ];
        for name in wanted.iter().chain(not_wanted.iter()) {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        let names: Vec<String> = SyncService::list_rescue_backups(dir.path())
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        // 按文件名排序：越早的一轮越靠前，用户看日志时顺序稳定
        assert_eq!(
            names,
            wanted.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        );
    }

    /// 副本名唯一是「不覆盖上一次救援数据」的前提。
    #[test]
    fn rescue_backup_paths_are_unique() {
        let dir = TempDir::new("rescue_names");
        assert_ne!(
            SyncService::rescue_backup_path(dir.path()),
            SyncService::rescue_backup_path(dir.path())
        );
    }
}
