# Artist Card

> 艺术家列表页的网格项组件。

---

## 1. Overview

艺术家卡片用于在 6 列自适应网格中展示单名艺术家。设计稿要求**几何交替**排版：奇数位正方形圆角，偶数位圆形，赋予界面如实体艺术杂志般的排版韵律。

---

## 2. Anatomy

```
┌──────────────────────┐
│                      │
│    [User / Avatar]   │  aspect-square · bg-gradient
│                      │  rounded-[10px] / rounded-full
│                      │  (交替)
│                      │
├──────────────────────┤
│  艺术家名             │  15px Primary · medium · truncate
│  X 首歌曲             │  13px Secondary · truncate
└──────────────────────┘
```

---

## 3. States

| 状态 | 头像区 | 文字 |
|---|---|---|
| Default | 渐变背景 + User 图标 + `bg-black/10` 蒙层 | Primary/Secondary 色 |
| Hover | `bg-black/20` | 不变 |
| 空态 | `from-gray-500 to-gray-700` fallback | "还没有扫描到艺术家" |

---

## 4. Geometric Alternation

```html
:class="index % 2 === 0 ? 'rounded-[10px]' : 'rounded-full'"
```

| 索引（0-based） | 头像形状 | 示例 |
|---|---|---|
| 偶数（0, 2, 4...） | 正方形圆角 `rounded-[10px]` | Ludovico Einaudi |
| 奇数（1, 3, 5...） | 圆形 `rounded-full` | Sigur Rós |

---

## 5. 头像占位方案

由于后端不存储艺术家图片，使用确定性颜色渐变 + User 图标：

```html
<div class="w-full aspect-square bg-gradient-to-br {{ avatarColor }}">
  <User class="w-[40px] h-[40px] text-white/60" />
</div>
```

`avatarColor` 由 `getDeterministicColor(artistName)` 生成，确保同一艺术家颜色一致。

---

## 6. Tokens used

| Token | 用途 |
|---|---|
| `--text-primary` | 艺术家名 |
| `--text-secondary` | 歌曲数 |
| `--text-disabled` | 空态图标 |
| `--text-muted` | 空态文字 |
| `--radius-10` | 头像圆角 |
| `--list-hover` | Hover 蒙层 |
| `gap-6` | 网格间距 |

---

## 7. Do & Don't

### Do

- ✅ 交替圆角/圆形赋予视觉韵律
- ✅ User 图标占位，统一风格
- ✅ Hover 加深蒙层而非改变形状
- ✅ 自适应列数 `auto-fill, minmax(180px, 1fr)`

### Don't

- ❌ 使用真实用户图片（后端未支持）
- ❌ 所有头像统一形状（破坏交替设计）
- ❌ 在卡片内放操作按钮（遵循"Show on Hover"原则）
- ❌ 文字换行（一律 truncate）

---

*End of Artist Card.*
