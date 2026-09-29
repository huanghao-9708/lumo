# Song Row

> 列表行四态：Default / Hover / Selected / Playing。LDL 最高频组件。

> **2026-09 重构**：桌面端所有歌曲列表由共享组件驱动——
> 列定义 `src/components/shared/trackList/columns.ts`（单一来源）、
> 表头 `TrackListHeader.vue`、歌曲行 `TrackRow.vue`。
> 列显隐由**实际容器宽度**（ResizeObserver）决定，不再使用视口 `sm/md/lg` 断点。
> 本文档描述列定义与状态规范；实现细节以 `columns.ts` 为准。

---

## 1. Overview

Song Row 是音乐播放器的核心组件。每行 40px 高（token `--height-track-row` / `TRACK_ROW_HEIGHT`），
承载：序号 / 收藏 / 标题 / 艺术家 / 专辑 / 时长 / 音频信息 / 可选列（年份·流派·大小）/ 更多操作。
页面上下文可隐藏冗余列（专辑详情无专辑列、艺人详情无艺术家列、最近播放追加播放时间列）。

---

## 2. Anatomy

```
┌──┬──┬──────────────────────┬──────────────┬──────────────┬──────┬────────┬────┬────┬────┬──┐
│ #│♥ │ 标题                  │ 艺术家        │ 专辑          │ 时长 │ 音频    │年份│流派│大小│⋯ │
│40│32│ flex-[2]              │ flex-[1.5]    │ flex-[1.5]    │ 56   │ 104    │ 52 │ 96 │ 72 │32│
└──┴──┴──────────────────────┴──────────────┴──────────────┴──────┴────────┴────┴────┴────┴──┘
   ←──────────────────────────── 40px 高 ────────────────────────────────────────────────→
```

### 列定义（columns.ts 单一来源）

| 列 | 宽度/伸缩 | 对齐 | 收纳次序 | 说明 |
|---|---|---|---|---|
| # 序号 | 40px | 中 | 核心列 | 多选态变复选框；播放态变均衡器 |
| ♥ 收藏 | 32px | 中 | 核心列 | Hover 显；多选态变复选框 |
| 标题 | `flex-[2]`，最小 150px | 左 | 核心列 | 截断 + `title` 悬停看全名 |
| 艺术家 | `flex-[1.5]`，最小 96px | 左 | 6 | 艺人详情页隐藏 |
| 专辑 | `flex-[1.5]`，最小 96px | 左 | 5 | 专辑详情页隐藏 |
| 时长 | 56px | 右 | 7 | 数字右对齐 mono；缺失 `—` |
| 音频 | 104px | 左 | 4 | `FLAC` / `MP3 · 320 kbps` / `FLAC · 24bit/96kHz`，只描述可验证事实 |
| 年份 | 52px | 右 | 3 | 可选列（默认关）；歌曲年份优先、专辑年份兜底 |
| 流派 | 96px | 左 | 1 | 可选列（默认关）；多流派 `; ` 连接、截断 + tooltip |
| 大小 | 72px | 右 | 2 | 可选列（默认关）；当前首选音源大小 |
| 播放时间 | 88px | 右 | 页面专属 | 仅最近播放；不被列设置移除 |
| ⋯ 更多 | 32px | 中 | 核心列 | Hover 显；不可播行由 CloudOff 占位 |

**收纳次序**（容器变窄时先隐藏）：流派 → 大小 → 年份 → 音频信息 → 专辑 → 艺术家 → 时长。
用户在表头「显示列」菜单显式启用的列优先于宽度收纳（标题收缩让位，不丢用户选择）。
无标签缺失值统一显示 `—`。

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

## 5. 表头（TrackListHeader）

sticky 表头，与行由**同一份列配置**驱动（`columnCellStyle` 共用，天然对齐）：

- 10px uppercase `tracking-wider` Muted
- `border-b border-border-color` 底分隔
- `sticky top-0 bg-bg-content z-10`（`--z-sticky`）
- 数字列（时长/年份/大小/播放时间）右对齐，文本列左对齐，图标操作区宽度固定
- 右端「显示列」菜单（Settings2 图标）：**详细视图**开关（一次开启全部可选列）+
  年份/流派/大小逐列勾选；偏好持久化在 localStorage（`lumo_track_columns`），全视图共享
- 尾部 `#trailing` 插槽承载页面专属控件（如批量选择入口）

**列偏好规则**：显式勾选 = 钉住显示（优先于宽度收纳）；显式取消 = 永不渲染；
未设置的列跟随详细视图开关与默认值。

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
