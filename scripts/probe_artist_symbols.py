# -*- coding: utf-8 -*-
"""列出 artist 名字里出现过的所有非字母数字字符（按频次），找出还没覆盖的分隔符。"""
import os
import shutil
import sqlite3
import tempfile
from collections import Counter

db = os.path.join(os.environ["APPDATA"], "com.hao.lumo", "lumo.sqlite")
tmp = os.path.join(tempfile.gettempdir(), "lumo_probe3.sqlite")
shutil.copyfile(db, tmp)

conn = sqlite3.connect(tmp)
c = conn.cursor()

names = [r[0] for r in c.execute("SELECT name FROM artists")]
conn.close()

counter = Counter()
for name in names:
    for ch in name:
        if not ch.isalnum() and not ch.isspace():
            counter[ch] += 1

print("=== artist 名里出现过的符号（按频次降序）===")
for ch, n in counter.most_common(60):
    print("  {!r} U+{:04X} x{}".format(ch, ord(ch), n))

print()
print("=== 含这些符号、且可能是组合名的样本 ===")
for ch, _ in counter.most_common(30):
    rows = [r[0] for r in c.execute("SELECT name FROM artists WHERE name LIKE ? LIMIT 6", ("%" + ch + "%",))]
    print("  {!r}: {}".format(ch, rows))
