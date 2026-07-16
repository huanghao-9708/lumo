# Mobile Tab Bar

> 底部 4 Tab 导航栏（56px），替代桌面 Sidebar。

---

## 1. Overview

| 项 | 移动端 | 桌面端 |
|---|---|---|
| 导航方式 | **底部 Tab Bar** (56px) | 左侧 Sidebar (240px) |
| Tab 数量 | 4 个 | 7 个 Library + 3 个 Favorites + Playlists |
| 图标尺寸 | 22px (`--icon-tab`) | 16px |

---

## 2. Anatomy

```
┌──────────┬──────────┬──────────┬──────────┐
│   曲库   │   搜索   │   收藏   │   设置   │
│ Library  │  Search  │   Favs   │ Settings │
└──────────┴──────────┴──────────┴──────────┘
     56px + safe-area-inset-bottom
```

---

## 3. Tab 映射

| Tab | `activeMobileTab` | 对应 `activeLibraryTab` | 图标 |
|---|---|---|---|
| 曲库 | `'library'` | `'全部歌曲'` (default) / 专辑等 | `LibraryBig` |
| 搜索 | `'search'` | —（独立 MobileSearch 页） | `Search` |
| 收藏 | `'favorites'` | `'喜欢的音乐'` (default) | `Heart` |
| 设置 | `'settings'` | `'设置'` | `Settings` |

---

## 4. States

| 状态 | 图标 + 文字色 |
|---|---|
| Default | `text-text-muted` |
| Active | `text-brand-orange` + `font-medium` + `aria-current="page"` |

---

## 5. 安全区

```css
padding-bottom: env(safe-area-inset-bottom);
```

---

## 6. Tokens used

| Token | 用途 |
|---|---|
| `--height-mobile-tabbar` | 高度 56px |
| `--icon-tab` | 图标 22px |
| `--text-11` | 标签字号 |
| `--brand-orange` | Active 态 |
| `--text-muted` | Default 态 |

---

## 7. Do & Don't

### Do

- ✅ 4 Tab（不多不少，触屏友好）
- ✅ Accent 激活态
- ✅ `safe-area-inset-bottom`
- ✅ `aria-current="page"` + `aria-label`

### Don't

- ❌ 超过 5 个 Tab（太挤）
- ❌ 图标小于 22px（触屏难点）
- ❌ 文字用 Accent（仅图标用）

---

*End of Mobile Tab Bar.*
