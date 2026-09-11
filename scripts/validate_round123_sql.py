# -*- coding: utf-8 -*-
"""前三轮 SQL 改动的真实库验证（只读模式）。

验证内容：
1. library_get_stats 的聚合 SQL：可执行、耗时、数值口径（今日=本地时区）。
2. 两步法排行榜（Top Track / Recent Play / Recent Added / Favorite / Last Played）：
   耗时 + 与"旧版单步相关子查询 SQL"的 (id, play_count) 等价性比对。
3. 艺人榜 / 专辑榜两步法：等价性 + 耗时。
4. 今日播放次数 / 今日听歌时长口径抽检。
"""
import os
import sqlite3
import time
import json

APPDATA = os.environ.get("APPDATA", "")
DB = os.path.join(APPDATA, "com.hao.lumo", "lumo.sqlite")
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "validate_round123_out.txt")

lines = []
def log(s=""):
    lines.append(str(s))

if not os.path.exists(DB):
    log(f"DB NOT FOUND: {DB}")
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    raise SystemExit(1)

conn = sqlite3.connect(f"file:{DB}?mode=ro", uri=True)
conn.row_factory = sqlite3.Row
log(f"DB: {DB}")

# ---------- 0. 库规模 ----------
for t in ("tracks", "albums", "artists", "play_history", "playlists", "favorite_tracks"):
    n = conn.execute(f"SELECT COUNT(*) FROM {t}").fetchone()[0]
    log(f"count {t}: {n}")

# ---------- 1. stats SQL（与 commands/library.rs 完全一致） ----------
STATS_SQL = """
SELECT
    (SELECT COUNT(*) FROM tracks) AS track_count,
    (SELECT COUNT(*) FROM albums) AS album_count,
    (SELECT COUNT(*) FROM artists) AS artist_count,
    (SELECT COALESCE(SUM(play_duration_ms), 0) FROM play_history) AS total_listen_ms,
    (SELECT COALESCE(SUM(play_duration_ms), 0) FROM play_history
     WHERE played_at >= datetime('now','localtime','start of day','utc')) AS today_listen_ms,
    (SELECT COALESCE(SUM(play_duration_ms), 0) FROM play_history
     WHERE played_at >= datetime('now','localtime','start of day','-6 days','utc')) AS week_listen_ms,
    (SELECT COALESCE(SUM(play_count), 0) FROM tracks) AS total_play_count,
    (SELECT COUNT(*) FROM play_history
     WHERE played_at >= datetime('now','localtime','start of day','utc')) AS today_play_count,
    (SELECT COUNT(*) FROM playlists) AS playlist_count,
    (SELECT COUNT(*) FROM favorite_albums) AS favorite_album_count,
    (SELECT COUNT(*) FROM favorite_artists) AS favorite_artist_count,
    (SELECT COUNT(*) FROM favorite_tracks) AS favorite_track_count
"""
t0 = time.perf_counter()
stats = dict(conn.execute(STATS_SQL).fetchone())
dt = (time.perf_counter() - t0) * 1000
log(f"\n[stats] {dt:.2f} ms")
log(json.dumps(stats, ensure_ascii=False))

# 今日口径抽检：today_listen_ms 应等于按本地今日零点 UTC 时刻过滤的 SUM
local_midnight_utc = conn.execute(
    "SELECT datetime('now','localtime','start of day','utc')").fetchone()[0]
log(f"local midnight (UTC naive): {local_midnight_utc}")
manual_today = conn.execute(
    "SELECT COALESCE(SUM(play_duration_ms),0) FROM play_history WHERE played_at >= ?",
    (local_midnight_utc,)).fetchone()[0]
log(f"today_listen_ms check: sql={stats['today_listen_ms']} manual={manual_today} match={stats['today_listen_ms']==manual_today}")

# ---------- 2. 两步法 Top Track vs 旧版单步 SQL ----------
OLD_TOP_TRACK = """
SELECT t.id, t.play_count FROM tracks t
WHERE t.play_count > 0
ORDER BY t.play_count DESC, t.id ASC
LIMIT ?
"""
NEW_TOP_TRACK_STEP1 = OLD_TOP_TRACK  # 第一步同形（只取 ID，无子查询）

t0 = time.perf_counter()
old_top = conn.execute(OLD_TOP_TRACK, (30,)).fetchall()
old_dt = (time.perf_counter() - t0) * 1000
t0 = time.perf_counter()
ids = [r[0] for r in conn.execute(NEW_TOP_TRACK_STEP1, (30,)).fetchall()]
dt_step1 = (time.perf_counter() - t0) * 1000
ph = ",".join("?" * len(ids))
DETAIL_SQL = f"""
SELECT
    t.id,
    t.title,
    (SELECT artist_id FROM track_artists WHERE track_id = t.id ORDER BY position LIMIT 1) AS artist_id,
    (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name,
    t.album_id,
    al.title AS album_title,
    m.duration_ms,
    m.file_ext,
    m.id AS media_file_id,
    ft.track_id IS NOT NULL AS is_favorite,
    al.cover_artwork_id,
    m.file_size,
    (SELECT s.kind FROM sources s JOIN media_files mf ON mf.source_id = s.id WHERE mf.id = m.id) AS source_kind,
    t.play_count,
    t.last_played_at
FROM tracks t
LEFT JOIN albums al ON t.album_id = al.id
JOIN media_files m ON m.id = COALESCE(t.primary_file_id, (SELECT mf.id FROM media_files mf WHERE mf.track_id = t.id ORDER BY mf.id LIMIT 1))
LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
WHERE t.id IN ({ph})
ORDER BY t.play_count DESC, t.id ASC
"""
t0 = time.perf_counter()
detail_rows = conn.execute(DETAIL_SQL, ids).fetchall()
dt_step2 = (time.perf_counter() - t0) * 1000
new_top = [(r["id"], r["play_count"]) for r in detail_rows]
old_pairs = [(r["id"], r["play_count"]) for r in old_top]
log(f"\n[top track 30] old_single={old_dt:.1f} ms | two_step: s1={dt_step1:.2f}+s2={dt_step2:.2f}={dt_step1+dt_step2:.2f} ms | equal={old_pairs == new_top}")
if old_pairs != new_top:
    log(f"  old={old_pairs}")
    log(f"  new={new_top}")
# 附带检查：13+2 列每行都取到且 source_kind 非空
bad = [r["id"] for r in detail_rows if not r["source_kind"] or r["media_file_id"] is None]
log(f"  rows_with_null_source_kind_or_media: {bad}")

# 旧版「带相关子查询的完整列表」形态（文档中 6.7s 的真实形态：SELECT 里 4 个子查询 + WHERE 全表扫）
OLD_FULL_TOP_TRACK = """
SELECT
    t.id,
    t.title,
    (SELECT artist_id FROM track_artists WHERE track_id = t.id ORDER BY position LIMIT 1) AS artist_id,
    (SELECT GROUP_CONCAT(a.name, ', ') FROM track_artists ta JOIN artists a ON ta.artist_id = a.id WHERE ta.track_id = t.id ORDER BY ta.position) AS artist_name,
    t.album_id,
    al.title AS album_title,
    m.duration_ms,
    m.file_ext,
    m.id AS media_file_id,
    ft.track_id IS NOT NULL AS is_favorite,
    al.cover_artwork_id,
    m.file_size,
    (SELECT s.kind FROM sources s JOIN media_files mf ON mf.source_id = s.id WHERE mf.id = m.id) AS source_kind
FROM tracks t
LEFT JOIN albums al ON t.album_id = al.id
JOIN media_files m ON m.id = COALESCE(t.primary_file_id, (SELECT mf.id FROM media_files mf WHERE mf.track_id = t.id ORDER BY mf.id LIMIT 1))
LEFT JOIN favorite_tracks ft ON t.id = ft.track_id
WHERE t.play_count > 0
ORDER BY t.play_count DESC, t.id ASC
LIMIT ?
"""
t0 = time.perf_counter()
old_full = conn.execute(OLD_FULL_TOP_TRACK, (30,)).fetchall()
old_full_dt = (time.perf_counter() - t0) * 1000
log(f"  old_full_form(4 correlated subqueries, LIMIT 30): {old_full_dt:.1f} ms | equal_ids={[(r['id']) for r in old_full] == [i for i,_ in new_top]}")

# ---------- 3. 艺人榜 / 专辑榜等价性 ----------
OLD_TOP_ARTIST = """
SELECT ta.artist_id AS id, SUM(t.play_count) AS play_count
FROM track_artists ta
JOIN tracks t ON t.id = ta.track_id
WHERE t.play_count > 0
GROUP BY ta.artist_id
ORDER BY play_count DESC, ta.artist_id ASC
LIMIT ?
"""
t0 = time.perf_counter()
old_artists = [(r["id"], r["play_count"]) for r in conn.execute(OLD_TOP_ARTIST, (5,)).fetchall()]
old_art_dt = (time.perf_counter() - t0) * 1000

ids_a = [i for i, _ in old_artists] if old_artists else []
new_artists = []
if ids_a:
    ph = ",".join("?" * len(ids_a))
    ART_DETAIL = f"""
    SELECT ar.id, ar.name, ar.track_count, ar.avatar_artwork_id,
        (SELECT COALESCE(SUM(t.play_count), 0) FROM track_artists ta JOIN tracks t ON t.id = ta.track_id WHERE ta.artist_id = ar.id) AS play_count
    FROM artists ar
    WHERE ar.id IN ({ph})
    ORDER BY play_count DESC, ar.id ASC
    """
    t0 = time.perf_counter()
    rows = conn.execute(ART_DETAIL, ids_a).fetchall()
    art_detail_dt = (time.perf_counter() - t0) * 1000
    new_artists = [(r["id"], r["play_count"]) for r in rows]
    names = [(r["name"], r["play_count"], r["track_count"]) for r in rows]
log(f"[top artist 5] old={old_art_dt:.1f} ms, detail={art_detail_dt if ids_a else 0:.2f} ms | equal={old_artists == new_artists}")
if ids_a:
    log(f"  {names}")

OLD_TOP_ALBUM = """
SELECT al.id AS id, SUM(t.play_count) AS play_count
FROM albums al
JOIN tracks t ON t.album_id = al.id
WHERE t.play_count > 0
GROUP BY al.id
ORDER BY play_count DESC, al.id ASC
LIMIT ?
"""
t0 = time.perf_counter()
old_albums = [(r["id"], r["play_count"]) for r in conn.execute(OLD_TOP_ALBUM, (5,)).fetchall()]
old_alb_dt = (time.perf_counter() - t0) * 1000

ids_al = [i for i, _ in old_albums] if old_albums else []
new_albums = []
if ids_al:
    ph = ",".join("?" * len(ids_al))
    ALB_DETAIL = f"""
    SELECT al.id, al.title,
        (SELECT GROUP_CONCAT(aa2.name, ', ') FROM album_artists aa1 JOIN artists aa2 ON aa1.artist_id = aa2.id WHERE aa1.album_id = al.id ORDER BY aa1.position) AS artist_name,
        al.cover_artwork_id,
        (SELECT COALESCE(SUM(t.play_count), 0) FROM tracks t WHERE t.album_id = al.id) AS play_count
    FROM albums al
    WHERE al.id IN ({ph})
    ORDER BY play_count DESC, al.id ASC
    """
    t0 = time.perf_counter()
    rows = conn.execute(ALB_DETAIL, ids_al).fetchall()
    alb_detail_dt = (time.perf_counter() - t0) * 1000
    new_albums = [(r["id"], r["play_count"]) for r in rows]
    alb_names = [(r["title"], r["artist_name"], r["play_count"]) for r in rows]
log(f"[top album 5] old={old_alb_dt:.1f} ms, detail={alb_detail_dt if ids_al else 0:.2f} ms | equal={old_albums == new_albums}")
if ids_al:
    log(f"  {alb_names}")

# ---------- 4. 其余榜单两步法耗时 ----------
def timed(sql, params=()):
    t0 = time.perf_counter()
    rows = conn.execute(sql, params).fetchall()
    return (time.perf_counter() - t0) * 1000, rows

RECENT_PLAY_IDS = "SELECT t.id FROM tracks t WHERE t.last_played_at IS NOT NULL ORDER BY t.last_played_at DESC, t.id DESC LIMIT ?1"
RECENT_ADDED_IDS = "SELECT t.id FROM tracks t ORDER BY t.added_at DESC, t.id DESC LIMIT ?1"
FAV_IDS = "SELECT ft.track_id FROM favorite_tracks ft ORDER BY ft.favorited_at DESC, ft.track_id DESC LIMIT ?1"
LAST_ID = "SELECT id FROM tracks WHERE last_played_at IS NOT NULL ORDER BY last_played_at DESC, id DESC LIMIT 1"
TODAY_CNT = "SELECT COUNT(*) FROM play_history WHERE played_at >= datetime('now','localtime','start of day','utc')"

for label, sql, params in [
    ("recent_play step1", RECENT_PLAY_IDS, (5,)),
    ("recent_added step1", RECENT_ADDED_IDS, (5,)),
    ("favorite step1", FAV_IDS, (5,)),
    ("last_played id", LAST_ID, ()),
    ("today_play_count", TODAY_CNT, ()),
]:
    dt, rows = timed(sql, params)
    log(f"[{label}] {dt:.2f} ms -> {[tuple(r) for r in rows[:5]]}")

# favorite 详情步（按 ft.favorited_at 复现顺序）
ids_f = [r[0] for r in conn.execute(FAV_IDS, (5,)).fetchall()]
if ids_f:
    ph5 = ",".join("?" * len(ids_f))
    FAV_DETAIL_ORDER = DETAIL_SQL.replace(",".join("?" * len(ids)), ph5).replace(
        "ORDER BY t.play_count DESC, t.id ASC", "ORDER BY ft.favorited_at DESC, t.id DESC")
    dt, rows = timed(FAV_DETAIL_ORDER, ids_f)
    log(f"[favorite step2 order-by-favorited_at] {dt:.2f} ms -> {[(r['id'], r['title']) for r in rows]}")
    # 校验顺序与 favorited_at DESC 一致
    fa = [r[0] for r in conn.execute(
        f"SELECT ft.track_id FROM favorite_tracks ft JOIN tracks t ON t.id = ft.track_id WHERE ft.track_id IN ({ph5}) ORDER BY ft.favorited_at DESC, ft.track_id DESC", ids_f).fetchall()]
    log(f"  order_match={fa == [r['id'] for r in rows]}")

# EXPLAIN QUERY PLAN：确认第一步走索引/无全表+子查询放大
log("\n[EXPLAIN QUERY PLAN top_played step1]")
for r in conn.execute("EXPLAIN QUERY PLAN " + OLD_TOP_TRACK, (5,)):
    log(f"  {tuple(r)}")
log("[EXPLAIN QUERY PLAN detail]")
for r in conn.execute("EXPLAIN QUERY PLAN " + DETAIL_SQL.replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1).replace("?", "1", 1)):
    log(f"  {tuple(r)}")

conn.close()
with open(OUT, "w", encoding="utf-8") as f:
    f.write("\n".join(lines))
print("done")
