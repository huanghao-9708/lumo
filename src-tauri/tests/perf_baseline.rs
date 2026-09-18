//! 曲库规模性能基准（I3/DATA-005）。
//!
//! 默认 `#[ignore]`：它要生成 3 万行数据，不该拖慢每次 `cargo test`。
//! 运行：`cargo test --release --test perf_baseline -- --ignored --nocapture`
//!
//! 覆盖范围**只有数据库/查询层**：列表分页、深分页、关键词搜索、扫描前的全量缓存加载、
//! `integrity_check`、库体积。冷启动、首播、切歌、内存与安装包体积**不在这里测**，
//! 未测的项一律不得写进对外声明（见 `PERFORMANCE_BASELINE.md` §4）。
//!
//! 这里调用的是 `TrackRepo::get_tracks_paginated` 等**生产代码本身**而不是复制一份 SQL，
//! 否则基准会通过而真实路径早已劣化。

use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri_app_lib::db::init_db;
use tauri_app_lib::repositories::track_repo::TrackRepo;

/// 每个指标预热 1 次后再跑 5 次，报告中位数与最大值（样本量小，P95 与最大值同义）。
const RUNS: usize = 5;

/// 临时库目录，Drop 时删除；基准中途 assert 失败也不会留垃圾文件。
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("lumo-perf-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("无法创建基准临时目录");
        Self(dir)
    }

    fn db_path(&self) -> PathBuf {
        self.0.join("lumo.db")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn summarize(samples: &[Duration]) -> (f64, f64) {
    let mut v: Vec<Duration> = samples.to_vec();
    v.sort_unstable();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    (ms(v[v.len() / 2]), ms(v[v.len() - 1]))
}

fn measure<F: FnMut()>(runs: usize, mut f: F) -> (f64, f64) {
    f(); // 预热：查询计划与页缓存先就位，冷读单独看最大值列
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = Instant::now();
        f();
        samples.push(start.elapsed());
    }
    summarize(&samples)
}

/// 生成接近真实曲库的标题：中英混合、长度不一。全 ASCII 会让 LIKE 扫描便宜得不真实。
fn title(i: usize) -> String {
    const CN: [&str; 8] = [
        "夜航船",
        "山海",
        "南方奶奶",
        "雨后城市",
        "折叠时间",
        "雾中灯塔",
        "旧海报",
        "北回归线",
    ];
    const EN: [&str; 6] = [
        "Midnight Ferry",
        "Paper Lanterns",
        "Slow Static",
        "Northern Lights",
        "Quiet Machine",
        "Harbour Light",
    ];
    format!("{} {}", CN[i % CN.len()], EN[i % EN.len()])
}

/// 播种 N 首：每 8 首一张专辑、每首一个艺人、约 5% 收藏、统一 flac 与时长。
fn seed(conn: &Connection, n: usize) {
    conn.execute(
        "INSERT INTO sources (id, name, kind, root_uri) VALUES (1, 'bench', 'local', 'X:/bench')",
        [],
    )
    .unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    for i in 0..n {
        let artist_id = (i % (n / 6).max(1)) as i64 + 1;
        let album_id = (i / 8) as i64 + 1;
        tx.execute(
            "INSERT OR IGNORE INTO artists (id, name, normalized_name) VALUES (?1, ?2, ?2)",
            params![artist_id, format!("艺人 {:02}", artist_id)],
        )
        .unwrap();
        tx.execute(
            "INSERT OR IGNORE INTO albums (id, title, normalized_title) VALUES (?1, ?2, ?2)",
            params![album_id, format!("专辑 {:04}", album_id)],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO tracks (id, title, normalized_title, album_id, added_at)
             VALUES (?1, ?2, ?2, ?3, datetime('2026-01-01'))",
            params![i as i64 + 1, title(i), album_id],
        )
        .unwrap();
        let path = format!("Music/{:02}/{:06}.flac", album_id % 32, i);
        tx.execute(
            "INSERT INTO media_files (track_id, source_id, relative_path, normalized_path,
                                       file_name, file_ext, file_size, modified_at, duration_ms)
             VALUES (?1, 1, ?2, ?3, ?4, 'flac', ?5, '2026-01-01', 210000)",
            params![
                i as i64 + 1,
                path,
                path.to_lowercase(),
                format!("{:06}.flac", i),
                20_000_000i64 + i as i64
            ],
        )
        .unwrap();
        // primary_file_id 在真实扫描里由标签解析回填，列表查询的 COALESCE 依赖它
        tx.execute(
            "UPDATE tracks SET primary_file_id = id WHERE id = ?1",
            params![i as i64 + 1],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO track_artists (track_id, artist_id, position) VALUES (?1, ?2, 0)",
            params![i as i64 + 1, artist_id],
        )
        .unwrap();
        if i % 20 == 0 {
            tx.execute(
                "INSERT INTO favorite_tracks (track_id) VALUES (?1)",
                params![i as i64 + 1],
            )
            .unwrap();
        }
    }
    tx.commit().unwrap();
}

/// 与 `services::scanner` 扫描前加载的字段完全一致：增量判定要把全库路径载入内存。
fn load_scan_cache(conn: &Connection) -> usize {
    let mut stmt = conn
        .prepare(
            "SELECT normalized_path, modified_at, file_size, availability
             FROM media_files WHERE source_id = 1",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            let p: String = r.get(0)?;
            let m: Option<String> = r.get(1)?;
            let s: Option<i64> = r.get(2)?;
            let a: String = r.get(3)?;
            Ok((p, m, s, a))
        })
        .unwrap();
    rows.count()
}

/// 归因用：同一排序条件下的裸查询（不带任何相关子查询）。
/// 与"首屏列表"对比即可判定成本来自排序/索引还是来自 SELECT 里的子查询。
fn bare_sorted_ids(conn: &Connection, limit: u32, offset: u32) -> usize {
    let mut stmt = conn
        .prepare("SELECT t.id FROM tracks t ORDER BY t.added_at DESC, t.id ASC LIMIT ?1 OFFSET ?2")
        .unwrap();
    let ids: Vec<i64> = stmt
        .query_map(params![limit, offset], |r| r.get::<_, i64>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert_eq!(ids.len() as u32, limit);
    ids.len()
}

fn integrity_ok(conn: &Connection) {
    let verdict: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(verdict, "ok");
}

fn file_size_mb(path: &Path) -> f64 {
    std::fs::metadata(path)
        .map(|m| m.len() as f64 / 1_048_576.0)
        .unwrap_or(0.0)
}

/// `rustc --version` 的单行输出；拿不到就写 unknown，不编造基准元数据。
fn toolchain() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[test]
#[ignore = "曲库规模基准需要 release：cargo test --release --test perf_baseline -- --ignored --nocapture"]
fn library_scale_baseline() {
    println!("\n=== Lumo 曲库规模基准（数据库/查询层） ===");
    println!(
        "预热 1 次 + 计时 {} 次；profile={}；toolchain={}；os={} {}；并行度={}",
        RUNS,
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        toolchain(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0),
    );

    for size in [1_000usize, 10_000, 30_000] {
        let dir = TempDir::new(&size.to_string());
        let db_path = dir.db_path();
        let pool = init_db(db_path.clone()).unwrap();
        let conn = pool.get().unwrap();

        let seed_start = Instant::now();
        seed(&conn, size);
        let seed_ms = seed_start.elapsed().as_secs_f64() * 1000.0;
        conn.execute("VACUUM", []).unwrap();

        let (first_page, first_page_max) = measure(RUNS, || {
            assert_eq!(
                TrackRepo::get_tracks_paginated(&conn, 200, 0, None)
                    .unwrap()
                    .len(),
                200
            );
        });
        let (deep_page, deep_page_max) = measure(RUNS, || {
            let offset = (size - 200) as u32;
            assert_eq!(
                TrackRepo::get_tracks_paginated(&conn, 200, offset, None)
                    .unwrap()
                    .len(),
                200
            );
        });
        let (search, search_max) = measure(RUNS, || {
            let hits =
                TrackRepo::get_tracks_paginated(&conn, 200, 0, Some("夜航".to_string())).unwrap();
            assert!(!hits.is_empty());
        });
        let (scan_cache, scan_cache_max) = measure(RUNS, || {
            assert_eq!(load_scan_cache(&conn), size);
        });
        let (integrity, integrity_max) = measure(RUNS, || integrity_ok(&conn));
        let (bare, bare_max) = measure(RUNS, || {
            assert_eq!(bare_sorted_ids(&conn, 200, 0), 200);
        });

        println!("--- {} 首 ---", size);
        println!("  建库导入（含事务提交）        : {:>9.0} ms", seed_ms);
        println!(
            "  首屏列表 200 条              : {:>9.1} / {:>7.1} ms (中位/最大)",
            first_page, first_page_max
        );
        println!(
            "  深分页 offset=N-200          : {:>9.1} / {:>7.1} ms",
            deep_page, deep_page_max
        );
        println!(
            "  关键词搜索 LIKE（命中集大）  : {:>9.1} / {:>7.1} ms",
            search, search_max
        );
        println!(
            "  扫描前全量缓存加载 {} 行   : {:>9.1} / {:>7.1} ms",
            size, scan_cache, scan_cache_max
        );
        println!(
            "  PRAGMA integrity_check       : {:>9.1} / {:>7.1} ms",
            integrity, integrity_max
        );
        println!(
            "  [归因] 同排序裸取 200 id      : {:>9.1} / {:>7.1} ms",
            bare, bare_max
        );
        println!(
            "  库体积（VACUUM 后）          : {:>9.2} MB",
            file_size_mb(&db_path)
        );
    }
    println!("=== 基准结束 ===\n");
}
