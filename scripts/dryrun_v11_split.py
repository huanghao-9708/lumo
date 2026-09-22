# -*- coding: utf-8 -*-
"""V11 迁移预演：只读真实库，看看哪些艺人会被拆分（不改数据）。

用 Python 复刻 services/metadata.rs 的 split_artist_names，与 Rust 侧行为对齐，
先在真库上验证「粗筛 SQL + 拆分逻辑」的实际结果。
"""
import os
import shutil
import sqlite3
import tempfile

SEPARATORS = set('&＆;；、，,|｜')
KEYWORDS = ["featuring", "feat.", "feat", "ft.", "ft"]


def split_artist_names(raw: str):
    out, buf, i = [], "", 0
    while i < len(raw):
        ch = raw[i]
        if ch in SEPARATORS:
            flush(out, buf)
            buf = ""
            i += 1
            continue
        at_word_start = (buf == "") or buf[-1].isspace()
        kw_len = match_keyword(raw[i:], at_word_start)
        if kw_len:
            flush(out, buf)
            buf = ""
            i += kw_len
            continue
        buf += ch
        i += 1
    flush(out, buf)
    return out


def match_keyword(rest: str, at_word_start: bool):
    if not at_word_start:
        return 0
    for kw in KEYWORDS:
        if not rest.lower().startswith(kw):
            continue
        after = rest[len(kw):]
        if after == "" or after[0].isspace():
            return len(kw)
    return 0


def flush(out, buf):
    part = strip_trailing_keyword(buf.strip())
    if part:
        out.append(part)


def strip_trailing_keyword(part: str):
    """镜像 Rust 的 strip_trailing_collab_keyword。"""
    trimmed = part.rstrip()
    for kw in KEYWORDS:
        n = len(kw)
        if len(trimmed) <= n:
            continue
        if trimmed[len(trimmed) - n:].lower() != kw:
            continue
        prefix = trimmed[: len(trimmed) - n]
        if not prefix or not prefix[-1].isspace():
            continue
        head = prefix.rstrip()
        if head:
            return head
    return part


db = os.path.join(os.environ["APPDATA"], "com.hao.lumo", "lumo.sqlite")
tmp = os.path.join(tempfile.gettempdir(), "lumo_v11_dryrun.sqlite")
shutil.copyfile(db, tmp)          # 只读预演：拷贝一份，绝不碰原库
print("副本:", tmp)

conn = sqlite3.connect(tmp)
cur = conn.cursor()

sql = "SELECT id, name FROM artists WHERE 0 = 1"
for ch in SEPARATORS:
    sql += " OR name LIKE '%{}%'".format(ch)
sql += " OR name LIKE '%feat%' OR name LIKE '%ft.%' OR name LIKE '%ft %'"

rows = cur.execute(sql).fetchall()
print("粗筛命中 artist 行数:", len(rows))

splittable = []
for aid, name in rows:
    parts = split_artist_names(name)
    if len(parts) > 1:
        splittable.append((aid, name, parts))

print("真正会被拆分的:", len(splittable))
for aid, name, parts in splittable:
    refs = cur.execute(
        "SELECT COUNT(*) FROM track_artists WHERE artist_id = ?", (aid,)
    ).fetchone()[0]
    print("  [{}] {!r} -> {} (挂 {} 首)".format(aid, name, parts, refs))

print("--- 粗筛但不会被拆（应为 0 误伤）---")
for aid, name in rows:
    parts = split_artist_names(name)
    if len(parts) == 1:
        print("  保留: {!r}".format(name))

conn.close()
os.remove(tmp)
print("已删除副本")
