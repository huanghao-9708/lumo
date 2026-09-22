# -*- coding: utf-8 -*-
"""探查库里 artist 名字里的可疑分隔符 / 超长组合名，定位用户说的「逗号分隔」。"""
import os
import shutil
import sqlite3
import tempfile

db = os.path.join(os.environ["APPDATA"], "com.hao.lumo", "lumo.sqlite")
tmp = os.path.join(tempfile.gettempdir(), "lumo_probe2.sqlite")
shutil.copyfile(db, tmp)

conn = sqlite3.connect(tmp)
conn.isolation_level = None
c = conn.cursor()

print("artist 总数:", c.execute("SELECT COUNT(*) FROM artists").fetchone()[0])

print("=== 名字最长的 25 个 artist（组合名通常很长）===")
for length, name in c.execute("SELECT LENGTH(name), name FROM artists ORDER BY LENGTH(name) DESC LIMIT 25"):
    print("  ", length, repr(name))

print("=== 可疑字符命中（半角/全角/异体）===")
for ch in [",", "，", "､", "﹐", "‚", "、", ";", "；", "|", "｜", "／", "/", "·", "•", "+", "×", "&", "＆", "＆", "&amp;"]:
    n = c.execute("SELECT COUNT(*) FROM artists WHERE name LIKE ?", ("%" + ch + "%",)).fetchone()[0]
    if n:
        print("  ", repr(ch), n)
        for r in c.execute("SELECT name FROM artists WHERE name LIKE ? LIMIT 5", ("%" + ch + "%",)):
            print("       ", repr(r[0]))

print("=== track_count > 0 的 artist 里，名字含空格+斜杠等混排的抽样 ===")
for r in c.execute(
    "SELECT name, track_count FROM artists WHERE track_count > 0 AND (name LIKE '%/%' OR name LIKE '%·%' OR name LIKE '%+%') LIMIT 15"
):
    print("  ", r)

print("=== 前 15 个 artist（按 track_count 降序）===")
for r in c.execute("SELECT name, track_count FROM artists ORDER BY track_count DESC LIMIT 15"):
    print("  ", r)

conn.close()
