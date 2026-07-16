# Mobile Now Playing

> 全屏沉浸式播放页。单列纵向布局 + 封面取色背景 + 下滑手势收起。

---

## 1. Overview

对应桌面 `NowPlayingImmersive.vue`（三栏布局）的移动端单列适配。

| 项 | 移动端 | 桌面端 |
|---|---|---|
| 布局 | **单列纵向**（封面→信息→进度→控制→歌词） | 三栏（信息/封面/歌词） |
| 封面尺寸 | `w-[75%] max-w-[280px]` | `min(58vh, 500px)` |
| Transport gap | `gap-8` (32px) | `gap-7` (28px) |
| Play 按钮 | 56px | 50px |
| 音量 | **系统音量键接管** | 物理旋钮 |
| 退出 | 下滑手势 / ⌄ 按钮 | 点击歌名 / Esc |
| 动画 | 复用 `np-drawer`（底部上滑） | 同 |

---

## 2. Anatomy

```
┌───────────────────────┐
│  ⌄ 收起               │  safe-area-top + 56px
├───────────────────────┤
│                       │
│    ┌────────────┐     │
│    │    封面     │     │  aspect-square, max-w-[280px]
│    └────────────┘     │
│                       │
│  歌曲名称              │  20px Bold 白色
│  艺术家 · 专辑         │  14px 白色/80%
│                       │
│  ═══════●═══════      │  <input type="range"> 橙 thumb
│  1:23        3:42     │  10px Mono
│                       │
│  🔀  ⏮  ▶  ⏭  ♡     │  5 按钮等距，Play 56px 白底黑标
│                       │
│  ── 歌词 ──          │  LyricsView variant="immersive"
│  前一句 / 当前 / 后    │  当前行 Accent
│                       │
└───────────────────────┘
```

---

## 3. 背景系统

复用桌面 `useCoverColor` 三层叠加：

1. **模糊封面**：`scale-150 blur-[80px] opacity-50`
2. **主色叠加**：`mixBlendMode: color` + `opacity: 0.55`
3. **暗化层**：`bg-black/35`

取色失败 → 兜底色 `#2A2722`。

---

## 4. 手势

| 手势 | 行为 |
|---|---|
| 下滑 `deltaY > 80px` | 收起 Now Playing |
| 点击 ⌄ 按钮 | 收起 |
| 触摸拖拽进度条 | seek |
| `@touchstart` / `@touchend` / `@touchmove` | 下滑检测 |

---

## 5. Tokens used

| Token | 用途 |
|---|---|
| `--brand-orange` | 进度条 thumb、当前歌词行 |
| `--radius-10` | 封面圆角 |
| `--shadow-mobile-overlay` | 顶部浮层分隔（例外 #5） |
| `--text-mobile-title` | 曲名 20px |

---

## 6. Do & Don't

### Do

- ✅ 单列纵向布局，触摸友好
- ✅ 复用桌面取色系统
- ✅ 下滑手势 + ⌄ 按钮双通道退出
- ✅ Transport 56px 白底 Play（Primary 黑，非 Accent）

### Don't

- ❌ 三栏布局（手机屏幕太窄）
- ❌ 音量旋钮（系统音量键接管）
- ❌ Play 按钮用 Accent（违反 One Accent Rule）

---

*End of Mobile Now Playing.*
