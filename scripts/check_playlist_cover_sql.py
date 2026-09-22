# -*- coding: utf-8 -*-
"""只读验证新的歌单查询 SQL（列名 + 封面解析结果）。"""
import os
import sqlite3

db = os.path.join(os.environ["APPDATA"], "com.hao.lumo", "lumo.sqlite")
print("DB:", db, "exists:", os.path.exists(db))

SQL = """
                SELECT
                    p.id,
                    p.name,
                    p.description,
                    COUNT(pi.id) AS track_count,
                    COALESCE(
                        (SELECT al.cover_artwork_id
                           FROM playlist_items pi2
                           JOIN tracks t2 ON t2.id = pi2.track_id
                           JOIN albums al ON al.id = t2.album_id
                          WHERE pi2.playlist_id = p.id
                            AND al.cover_artwork_id IS NOT NULL
                          ORDER BY pi2.position ASC
                          LIMIT 1),
                        (SELECT al2.cover_artwork_id
                           FROM playlist_items pi3
                           JOIN tracks t3 ON t3.id = pi3.track_id
                           LEFT JOIN albums al2 ON al2.id = t3.album_id
                          WHERE pi3.playlist_id = p.id
                          ORDER BY pi3.position ASC
                          LIMIT 1)
                    ) AS cover_artwork_id
                FROM playlists p
                LEFT JOIN playlist_items pi ON p.id = pi.playlist_id
                GROUP BY p.id
                ORDER BY p.created_at ASC
"""

conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
cur = conn.cursor()

rows = cur.execute(SQL).fetchall()
print(f"playlists = {len(rows)}")
for pid, name, desc, cnt, cover in rows:
    thumb = None
    if cover is not None:
        r = cur.execute("SELECT length(thumbnail_blob) FROM artwork WHERE id = ?1", (cover,)).fetchone()
        thumb = r[0] if r and r[0] is not None else "NULL-BLOB"
    print(f"  id={pid} count={cnt} cover_artwork_id={cover} thumb_bytes={thumb} name={name}")

# 抽查第一个歌单：第一首曲目是不是真的对应这个封面
first = cur.execute("""
    SELECT pi.position, t.title, t.album_id, al.title, al.cover_artwork_id
      FROM playlist_items pi
      JOIN tracks t ON t.id = pi.track_id
      LEFT JOIN albums al ON al.id = t.album_id
     WHERE pi.playlist_id = (SELECT id FROM playlists ORDER BY created_at ASC LIMIT 1)
     ORDER BY pi.position ASC LIMIT 5
""").fetchall()
print("首个歌单前 5 首：")
for pos, title, alid, altitle, cov in first:
    print(f"  pos={pos} track={title} album={altitle} album_cover={cov}")

conn.close()
