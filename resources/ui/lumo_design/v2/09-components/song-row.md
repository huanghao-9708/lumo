# Song Row

> 列表行四态：Default / Hover / Selected / Playing。LDL 最高频组件。

---

## 1. Overview

Song Row 是音乐播放器的核心组件。每行 40px 高，承载：序号 / 收藏 / 标题 / 艺术家 / 专辑 / 时长 / 格式 / 更多操作。

---

## 2. Anatomy

```
┌──┬──┬──────────────────────┬──────────────┬──────────────┬──────┬────┬──┐
│ #│♥ │ 标题                  │ 艺术家        │ 专辑          │ 时长 │格式│⋯ │
│10│8 │ flex-[2]              │ flex-[1.5]    │ flex-[1.5]    │ 56   │50 │8 │
└──┴──┴──────────────────────┴──────────────┴──────────────┴──────┴────┴──┘
   ←──── 40px 高 ────→
```

### 列宽分配

| 列 | 宽度 | 响应 |
|---|---|---|
| # 序号 | 40px (`w-10`) | 始终显示 |
| ♥ 收藏 | 32px (`w-8`) | 始终显示 |
| 标题 | `flex-[2]` | 始终显示 |
| 艺术家 | `flex-[1.5]` | `hidden md:block`（≥768） |
| 专辑 | `flex-[1.5]` | `hidden lg:block`（≥1024） |
| 时长 | 56px (`w-[56px]`) | `hidden xl:block`（≥1280） |
| 格式 | 50px (`w-[50px]`) | `hidden xl:block`（≥1280） |
| ⋯ 更多 | 32px (`w-8`) | 始终显示（Hover 显） |

---

## 3. States

### 3.1 Default

```html
<div class="flex items-center hover:bg-list-hover transition-colors-smooth group cursor-pointer relative" style="height: 40px;">
```

- 背景：透明
- Hover：`bg-list-hover`

### 3.2 Hover

- 行背景：`bg-list-hover`
- 序号：默认数字 → Hover 时 Play 图标（`group-hover:hidden` / `group-hover:block`）
- 收藏：未收藏 Heart `opacity-0` → `group-hover:opacity-60`
- 更多：`opacity-0` → `group-hover:opacity-100`

```html
<!-- 序号 Hover 切换 -->
<span class="text-text-muted group-hover:hidden tabular-nums">{{ String(index + 1).padStart(2, '0') }}</span>
<Play class="w-[12px] h-[12px] fill-current mx-auto hidden group-hover:block text-text-secondary" />

<!-- 未收藏 Heart Hover 显 -->
<Heart class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity" />

<!-- 更多 Hover 显 -->
<div class="opacity-0 group-hover:opacity-100 transition-opacity">
  <MoreHorizontal class="w-4 h-4 text-text-muted" />
</div>
```

### 3.3 Selected（未来列表选中态）

预留：未来支持单击选中时，行背景 `bg-list-selected`。

### 3.4 Playing（当前播放）

```html
<div
  class="flex items-center ... relative"
  :class="{ 'playing-row bg-list-selected': isPlayingTrack(song.id) }"
>
```

| 元素 | Playing 态处理 |
|---|---|
| 行背景 | `bg-list-selected` |
| 左侧竖条 | 2px Accent（`.playing-row::before`） |
| 序号位 | 替换为 Loader2(spin) 或 Play(fill) |
| 标题 | `text-brand-orange font-semibold` |
| 收藏 | 正常显示 |

### 左侧 2px Accent 竖条

CSS 实现（`style.css`）：

```css
.playing-row {
  position: relative;
}
.playing-row::before {
  content: "";
  position: absolute;
  left: 0; top: 0; bottom: 0;
  width: 2px;
  background: var(--brand-orange);
  border-radius: 0 1px 1px 0;
  z-index: 1;
}
```

### 序号位 Playing 态

```html
<div class="w-10 text-center shrink-0 text-[12px] font-mono">
  <span v-if="isPlayingTrack(song.id)" class="text-brand-orange inline-flex items-center justify-center">
    <Loader2 v-if="playerStore.isPlaying" class="w-[14px] h-[14px] animate-spin" />
    <Play v-else class="w-[12px] h-[12px] fill-current" />
  </span>
  <template v-else>
    <span class="text-text-muted group-hover:hidden tabular-nums">{{ String(index + 1).padStart(2, '0') }}</span>
    <Play class="w-[12px] h-[12px] fill-current mx-auto hidden group-hover:block text-text-secondary" />
  </template>
</div>
```

- 正在播放且 isPlaying=true：Loader2 旋转
- 正在播放但暂停：Play fill Accent
- 非 Playing：数字（Hover 时切 Play 图标）

### 标题 Playing 态

```html
<span class="text-[13px] truncate block"
  :class="isPlayingTrack(song.id) ? 'text-brand-orange font-semibold' : 'text-text-primary font-medium'">
  {{ song.title }}
</span>
```

- Regular/Medium → **Semibold**（不跳到 Bold）
- `text-text-primary` → `text-brand-orange`

---

## 4. 收藏 Heart

### 双态

```html
<!-- 已收藏 -->
<Heart
  v-if="song.isFavorite"
  class="w-[14px] h-[14px] text-brand-orange fill-current cursor-pointer"
  @click="toggleFav(song.id, $event)"
/>

<!-- 未收藏（Hover 显） -->
<Heart
  v-else
  class="w-[14px] h-[14px] text-text-disabled opacity-0 group-hover:opacity-60 transition-opacity hover:!opacity-100 hover:!text-brand-orange cursor-pointer"
  @click="toggleFav(song.id, $event)"
/>
```

| 状态 | 图标 | 颜色 | 透明度 |
|---|---|---|---|
| 已收藏 | `fill-current` | `text-brand-orange` | 100% |
| 未收藏 Default | outline | `text-text-disabled` | 0%（隐藏） |
| 未收藏 Hover | outline | `text-text-disabled` | 60% |
| 未收藏 Hover Heart | outline | `text-brand-orange` | 100% |

- `@click` + `$event.stopPropagation()` 防止触发行点击
- `transition-opacity` 150ms 显隐

---

## 5. 表头

sticky 表头，与行对齐：

```html
<div class="flex items-center text-[10px] text-text-muted uppercase tracking-wider py-2 border-b border-border-color sticky top-0 bg-bg-content z-10">
  <div class="w-10 text-center shrink-0">#</div>
  <div class="w-8 shrink-0"></div>
  <div class="flex-[2] min-w-0 pl-1">标题</div>
  <div class="flex-[1.5] min-w-0 hidden md:block">艺术家</div>
  <div class="flex-[1.5] min-w-0 hidden lg:block">专辑</div>
  <div class="w-[56px] text-right shrink-0 hidden xl:block">时长</div>
  <div class="w-[50px] text-center shrink-0 hidden xl:block">格式</div>
  <div class="w-8 shrink-0"></div>
</div>
```

- 10px uppercase `tracking-wider` Muted
- `border-b border-border-color` 底分隔
- `sticky top-0 bg-bg-content z-10`（`--z-sticky`）
- 列宽与行完全对齐

---

## 6. 虚拟列表

Song Row 使用虚拟列表（`useVirtualList`）处理大量数据：

```html
<div :style="{ height: totalHeight + 'px', position: 'relative' }">
  <div :style="{ transform: `translateY(${offsetY}px)` }">
    <div v-for="{ index, data: song } in visibleItems" :key="song.id" ...>
```

- 行高固定 40px（`ROW_HEIGHT = 40`）
- `buffer: 8` 预渲染上下 8 行
- `translateY` 偏移可视区

> 虚拟列表实现见 `composables/useVirtualList.ts`。

---

## 7. Footer Status

列表底部统计行：

```html
<div class="flex items-center justify-between py-4 border-t border-border-color mt-2 text-[11px] text-text-muted font-mono">
  <span>{{ trackCount.toLocaleString() }} 首歌曲</span>
  <span>双击播放</span>
</div>
```

- 11px mono Muted
- `border-t` 顶分隔（二级 Divider）
- 左：数量，右：操作提示

---

## 8. Tokens used

| Token | 用途 |
|---|---|
| `--list-hover` | Hover 背景 |
| `--list-selected` | Playing 背景 |
| `--text-primary` | 标题 Default |
| `--text-secondary` | 艺术家、专辑、Hover Play 图标 |
| `--text-muted` | 序号、时长、格式、表头 |
| `--text-disabled` | 未收藏 Heart |
| `--brand-orange` | Playing 竖条、标题、已收藏 Heart |
| `--border-color` | 表头底边、Footer 顶边 |

---

## 9. Do & Don't

### Do

- ✅ Playing 行用 `bg-list-selected` + 2px Accent 竖条 + Semibold 标题
- ✅ Hover 时序号切 Play 图标、收藏/更多渐显
- ✅ 双击播放（`@dblclick`）
- ✅ 表头 sticky + z-10

### Don't

- ❌ Playing 标题用 Bold（应用 Semibold）
- ❌ 给行加横线分隔（用留白和 Hover 背景区分）
- ❌ 序号不用 mono / tabular-nums（数字会跳动）
- ❌ 收藏 Heart 默认就显示（未收藏应 Hover 才显，保持安静）

---

*End of Song Row.*
