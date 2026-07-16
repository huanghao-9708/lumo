# Mobile Mini Player

> 底部迷你播放条（64px）。显示当前曲目，点击展开 Now Playing。

---

## 1. Overview

对应桌面 Playback Bar（110px）的精简触屏版。

| 项 | 移动端 | 桌面端 |
|---|---|---|
| 高度 | **64px** | 110px |
| 封面 | 40×40，`rounded-[6px]` | 56×56 |
| Transport | 三按钮（⏮ ▶ ⏭） | 五按钮 + 进度条 |
| 进度条 | **无**（空间不足） | 有 |
| 音量 | **无** | 物理旋钮 |
| 点击封面/标题 | 展开 Now Playing | 展开沉浸式 |

---

## 2. Anatomy

```
┌─────────────────────────────────────────────┐
│ [封面]  歌曲名称              ⏮   ▶   ⏭   │
│  40px   艺术家                                 │
└─────────────────────────────────────────────┘
   ← 整条 64px 高，全程可点击 →                     
```

---

## 3. 显示条件

- **仅当** `playerStore.currentTrack !== null` 时显示
- 切换曲目时无缝更新
- 与 Content 之间用 `h-px bg-border-color` 分隔

---

## 4. 交互

| 手势 | 行为 |
|---|---|
| 点击整条 | `uiStore.openImmersiveView()` → 展开 Now Playing |
| 点击 ⏮ | `playerStore.prevTrack()` |
| 点击 ▶/⏸ | `playerStore.togglePlay()` |
| 点击 ⏭ | `playerStore.nextTrack()` |
| Transport 按钮 `@click.stop` | 防止冒泡触发展开 |

---

## 5. Tokens used

| Token | 用途 |
|---|---|
| `--height-mobile-miniplayer` | 高度 64px |
| `--radius-6` | 封面圆角 |
| `--text-mobile-body` | 曲名 15px |
| `--text-13` | 艺术家 |
| `--bg-hover` | 封面加载底 |
| `--border-color` | 顶部分隔线 |

---

## 6. Do & Don't

### Do

- ✅ 封面真实图片（`useArtworkSrc`），非占位图标
- ✅ 整条可点击展开
- ✅ 三按钮 Transport（空间刚好）
- ✅ `@click.stop` 隔离按钮与行点击

### Don't

- ❌ 进度条（空间不足，Now Playing 看）
- ❌ 音量控件（系统音量键）
- ❌ Hover 交互（触摸）
- ❌ Accent 用于 Transport（Primary 黑）

---

*End of Mobile Mini Player.*
