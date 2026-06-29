# Inspector Panel

> 右侧信息面板。两个 Tab：Now Playing / Queue。

---

## 1. Overview

Inspector 是 360px 宽的固定右侧面板，可折叠。承载当前播放详情与播放队列。

```
┌──────────────────────────────┐
│   [正在播放]  [播放列表]      │  Tab 60px
├──────────────────────────────┤
│                              │
│  Now Playing / Queue 内容     │  flex-1 overflow-y-auto
│                              │
└──────────────────────────────┘
```

> Tab 规范见 [tab.md](tab.md)。本文聚焦 Tab 以下内容。

---

## 2. Now Playing 模式

### Anatomy

```
┌──────────────────────────────┐
│                              │
│      [封面 aspect-square]     │  rounded-[10px]
│                              │
├──────────────────────────────┤
│  曲名 18px Bold    [♥] [⋯]   │
│  艺术家 14px                  │
│  专辑 13px Muted              │
│  元数据 10px Mono uppercase   │
├──────────────────────────────┤
│  LYRICS                      │  Section 标题
│  歌词行 13px                  │
│  歌词行 13px (Accent 当前行)  │
│  歌词行 13px                  │
└──────────────────────────────┘
```

### 封面

```html
<div class="w-full aspect-square bg-bg-hover rounded-[10px] mb-4 overflow-hidden flex-shrink-0 flex items-center justify-center">
  <img v-if="coverSrc" :src="coverSrc" class="w-full h-full object-cover" alt="cover" />
  <Disc3 v-else class="w-10 h-10 text-text-disabled" aria-hidden="true" />
</div>
```

- `aspect-square` 自适应宽度（360px - 48px padding = 312px）
- `rounded-[10px]`
- `mb-4` (16px) 与下方信息间距
- `bg-bg-hover` 加载底色

### 曲目信息

```html
<div class="mb-4 flex-shrink-0">
  <div class="flex items-center justify-between mb-0.5">
    <h2 class="text-[18px] font-bold text-text-primary truncate pr-4 leading-tight">{{ currentTrack.title }}</h2>
    <div class="flex items-center gap-2 flex-shrink-0">
      <button :class="currentTrack.isFavorite ? 'text-brand-orange' : 'text-text-muted hover:text-text-primary'">
        <Heart class="w-[18px] h-[18px]" :class="currentTrack.isFavorite ? 'fill-current' : ''" />
      </button>
      <button class="text-text-muted hover:text-text-primary transition-colors-smooth">
        <MoreHorizontal class="w-[18px] h-[18px]" />
      </button>
    </div>
  </div>
  <p class="text-[14px] text-text-primary mb-0 truncate">{{ currentTrack.artist }}</p>
  <p class="text-[13px] text-text-muted mb-1.5 truncate">{{ currentTrack.album }}</p>
  <p class="text-[10px] text-text-muted font-mono uppercase tracking-wider leading-relaxed">{{ fileInfoText }}</p>
</div>
```

| 元素 | 字号 | 字重 | 颜色 |
|---|---|---|---|
| 曲名 | 18px | Bold | Primary |
| 艺术家 | 14px | Regular | Primary |
| 专辑 | 13px | Regular | Muted |
| 元数据 | 10px | Mono | Muted uppercase |

### 元数据行格式

```
1983 · 8 TRACKS · 03:42 · FLAC · 24bit / 96kHz
```

- ` · ` 分隔
- mono uppercase + `tracking-wider` + `leading-relaxed`
- 包含：年份 · 曲数 · 时长 · 格式 · 位深/采样率

### 收藏 + 更多

- Heart 18px，已收藏 `fill-current` + Accent
- More 18px，Ghost 按钮
- `gap-2` (8px) 间距
- `flex-shrink-0` 不被曲名挤压

### 无曲目占位

```html
<div class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted">
  <Disc3 class="w-8 h-8 text-text-disabled" aria-hidden="true" />
  <span class="text-[12px]">未在播放</span>
</div>
```

---

## 3. Lyrics

### 结构

```html
<div ref="lyricsContainer" class="flex-1 overflow-y-auto min-h-0">
  <h3 class="text-[10px] font-semibold text-text-muted uppercase tracking-widest mb-3">Lyrics</h3>
  <div v-if="lyrics.length === 0" class="text-[13px] text-text-muted/70 italic">暂无歌词</div>
  <div class="space-y-3">
    <p
      v-for="(line, i) in lyrics"
      :key="i"
      :data-active-lyric="i === activeLyricIndex"
      class="text-[13px] leading-[1.8] transition-colors-smooth cursor-pointer"
      :class="i === activeLyricIndex
        ? 'text-brand-orange font-medium'
        : i < activeLyricIndex
          ? 'text-text-muted'
          : 'text-text-secondary hover:text-text-primary'"
      @click="playerStore.seek((line.time || 0) * 1000)"
    >
      {{ line.text }}
    </p>
  </div>
</div>
```

### 歌词行三态

| 状态 | 颜色 | 字重 | 说明 |
|---|---|---|---|
| 当前行 | `text-brand-orange` | Medium | Accent 高亮 |
| 已唱过 | `text-text-muted` | Regular | 弱化（过去时） |
| 未唱 | `text-text-secondary` | Regular | 正常可读 |
| 未唱 Hover | `text-text-primary` | Regular | 提亮 |

### 行间距

- `space-y-3` (12px) 行间距
- `leading-[1.8]`（`--leading-loose`）行高，透气感

### 自动滚动

```js
function scrollToActiveLyric() {
  const el = lyricsContainer.value.querySelector('[data-active-lyric="true"]');
  if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' });
}
watch(() => playerStore.activeLyricIndex, () => {
  nextTick(scrollToActiveLyric);
});
```

- 当前行变化时，`scrollIntoView` 居中
- `behavior: 'smooth'` 平滑滚动

### 点击跳转

- 点击任意歌词行 → `seek(line.time * 1000)`
- `cursor-pointer` 提示可点击

### 无歌词

```html
<div class="text-[13px] text-text-muted/70 italic">暂无歌词</div>
```

- 13px，Muted 70%（更淡）
- `italic`（LDL 极少用斜体，此处例外）

---

## 4. Queue 模式

### Anatomy

```
┌──────────────────────────────┐
│  播放队列 · 12                │  Section 标题 + 计数
├──────────────────────────────┤
│ 01  曲名                      │
│     艺术家            03:42  │
│ 02  曲名                      │
│     艺术家            04:15  │
│ ...                          │
└──────────────────────────────┘
```

### Section 标题

```html
<h3 class="text-[10px] font-semibold text-text-muted uppercase tracking-widest mb-3 px-2">
  播放队列 · {{ queue.length }}
</h3>
```

### Queue Item

```html
<li
  v-for="(t, i) in queue"
  :key="t.id"
  class="flex items-center gap-2 px-2 py-[7px] rounded-[6px] cursor-pointer transition-colors-smooth group relative"
  :class="i === currentIndex ? 'bg-list-selected playing-row' : 'hover:bg-list-hover'"
  @click="playerStore.playQueue(playerStore.queue, i)"
>
  <span class="w-5 text-[11px] font-mono text-text-muted tabular-nums shrink-0">{{ String(i + 1).padStart(2, '0') }}</span>
  <div class="min-w-0 flex-1">
    <p class="text-[12px] truncate" :class="i === currentIndex ? 'text-brand-orange font-medium' : 'text-text-primary'">{{ t.title }}</p>
    <p class="text-[11px] text-text-muted truncate">{{ t.artist }}</p>
  </div>
  <span class="text-[11px] font-mono text-text-muted tabular-nums shrink-0">{{ t.duration }}</span>
</li>
```

### Queue Item 状态

| 元素 | Default | 当前播放 |
|---|---|---|
| 行背景 | 透明 / Hover `bg-list-hover` | `bg-list-selected` + `playing-row` |
| 左竖条 | 无 | 2px Accent（`playing-row::before`） |
| 序号 | 11px mono Muted | 11px mono Muted |
| 曲名 | 12px Primary | 12px Accent Medium |
| 艺术家 | 11px Muted | 11px Muted |
| 时长 | 11px mono Muted | 11px mono Muted |

### 与 Song Row 的差异

| 项 | Song Row | Queue Item |
|---|---|---|
| 高度 | 40px | ~30px (`py-[7px]`) |
| 序号宽 | 40px | 20px (`w-5`) |
| 曲名字号 | 13px | 12px |
| 列 | 标题/艺术家/专辑/时长/格式 | 标题+艺术家/时长 |
| 收藏 | 有 | 无 |
| 更多 | Hover 显 | 无 |

Queue 更紧凑，因为 Inspector 空间有限。

### 空队列

```html
<div class="flex flex-col items-center justify-center py-16 gap-3 text-text-muted">
  <ListMusic class="w-8 h-8 text-text-disabled" aria-hidden="true" />
  <span class="text-[12px]">队列为空</span>
</div>
```

---

## 5. Inspector 折叠

```html
<div v-if="uiStore.isRightSidebarVisible" class="hidden xl:flex w-[360px] ...">
```

- `v-if` 整体移除（包括 Divider C）
- `hidden xl:flex`：< 1280px 默认隐藏
- 用户可通过 TopBar 右栏按钮手动切换

---

## 6. Tokens used

| Token | 用途 |
|---|---|
| `--bg-canvas` | Inspector 底色 |
| `--bg-hover` | 封面底色 |
| `--text-primary` | 曲名、艺术家、Queue 曲名 |
| `--text-secondary` | 未唱歌词 |
| `--text-muted` | 专辑、元数据、已唱歌词、Section 标题、序号 |
| `--text-disabled` | 无封面/空队列图标 |
| `--brand-orange` | 当前行歌词、当前 Queue 曲名、Playing 竖条、已收藏 Heart |
| `--list-hover` | Queue Hover |
| `--list-selected` | 当前 Queue 行 |
| `--radius-10` | 封面圆角 |
| `--radius-6` | Queue Item 圆角 |

---

## 7. Do & Don't

### Do

- ✅ 当前行歌词 Accent + Medium，已唱过 Muted，未唱 Secondary
- ✅ 歌词 `leading-[1.8]` 透气
- ✅ 自动滚动 `scrollIntoView center`
- ✅ Queue 比 Song Row 更紧凑

### Don't

- ❌ 当前行歌词用 Bold（应用 Medium）
- ❌ 歌词行距用 `leading-normal`（应用 1.8 透气）
- ❌ Queue Item 高度与 Song Row 相同（应更紧凑）
- ❌ Inspector 折叠后保留 Divider C

---

*End of Inspector.*
