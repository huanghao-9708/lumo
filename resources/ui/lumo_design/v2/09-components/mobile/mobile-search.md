# Mobile Search

> 独立搜索页。搜索框 + 防抖 300ms + 分段结果显示。

---

## 1. Overview

对应桌面 `GlobalSearch.vue`（依赖 TopBar 搜索输入 + Tab 式结果）的独立触屏版。

| 项 | 移动端 | 桌面端 |
|---|---|---|
| 搜索框 | **独立输入框**（页面顶部） | 依赖 TopBar 搜索 |
| 结果展示 | **单列分段**（歌曲→专辑→艺术家） | 3 Tab 切换 |
| 防抖 | 300ms | 300ms（同） |
| API | `libraryGetTracks` / `libraryGetAlbums` / `libraryGetArtists` | 同 |

---

## 2. Anatomy

```
┌────────────────────────┐
│ 🔍 搜索歌曲、专辑…  [×] │  h-11, rounded-[10px]
├────────────────────────┤
│                        │
│ ── 歌曲 (5) ──        │  Section 标题 10px uppercase
│ #  歌曲名称      3:42  │  MobileSongRow
│                        │
│ ── 专辑 (2) ──        │
│ [封面] [封面]           │  MobileAlbumCard 2 列
│                        │
│ ── 艺术家 (3) ──       │
│ (头像) 艺术家名   >     │  行列表
│                        │
├────────────────────────┤
│ 共找到 10 个结果        │  Footer Status 11px Mono
└────────────────────────┘
```

---

## 3. States

| 状态 | 展示 |
|---|---|
| **无输入** | 空白提示："输入关键词搜索" + Search 图标 |
| **搜索中** | Loader2 spin |
| **有结果** | 分段（歌曲/专辑/艺术家），每段带计数 |
| **无结果** | "没有找到结果" + Music 图标 |

---

## 4. 交互

| 手势 | 行为 |
|---|---|
| 输入关键词 | 300ms 防抖触发搜索 |
| 点击 × | 清除输入，重置结果 |
| 点击歌曲行 | 播放该歌曲 |
| 点击专辑卡片 | 跳转专辑详情（`uiStore.setMobileTab('library')`） |
| 点击艺术家行 | 跳转艺术家详情 |
| 切换 Tab | 自动聚焦搜索框 |

---

## 5. 结果播放

点击搜索结果中的歌曲：构建 **1 首歌曲的队列**，`playAll([track], 0)`。

> **注意**：不要传搜索结果行索引（1 元素队列 index > 0 会越界）。

---

## 6. Tokens used

| Token | 用途 |
|---|---|
| `--radius-10` | 搜索框圆角 |
| `--text-mobile-body` | 搜索框字号 15px |
| `--text-10` | Section 标题 |
| `--brand-orange` | Focus 描边 `border-brand-orange/50` |

---

## 7. Do & Don't

### Do

- ✅ 独立搜索输入框
- ✅ 分段显示（非 Tab，单列更友好）
- ✅ 自动聚焦（进入搜索 Tab 时）
- ✅ 点击结果后切换 Tab 到 library

### Don't

- ❌ 依赖桌面 TopBar 搜索输入
- ❌ Tab 式结果（小屏太窄）
- ❌ 传错误索引给 playAll

---

*End of Mobile Search.*
