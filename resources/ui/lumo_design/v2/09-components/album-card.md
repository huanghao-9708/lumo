# Album Card

> 网格封面卡片。Hover 播放浮层是 LDL 唯一阴影例外。

---

## 1. Overview

Album Card 用于专辑网格视图（`AlbumGrid.vue`）。5 列网格，1:1 方形封面，下方标题 + 艺术家。

---

## 2. Anatomy

```
┌──────────────────┐
│                  │
│    [封面 1:1]     │  aspect-square · rounded-[10px] · Hover 浮层播放按钮
│                  │
└──────────────────┘
  专辑名称           15px Medium
  艺术家 · 年份      13px Muted
```

---

## 3. Grid 规格

| 项 | 值 |
|---|---|
| 列数 | 5（默认）/ 6（≥1536，未来） |
| 间距 | `gap-6` (24px) |
| 容器内边距 | `px-8` (32px) |
| 封面比例 | `aspect-square` (1:1) |
| 封面圆角 | `rounded-[10px]` |
| 封面间距 | `mb-3` (12px，封面到文字) |

```html
<div class="grid gap-6 pb-6" style="grid-template-columns: repeat(5, 1fr);">
```

---

## 4. 封面

### 4.1 默认

```html
<div class="relative w-full aspect-square rounded-[10px] overflow-hidden bg-bg-hover mb-3" @click="selectAlbum(album)">
  <img v-if="getCoverSrc(album)" :src="getCoverSrc(album)" :alt="album.title" class="w-full h-full object-cover" loading="lazy" />
  <div v-else class="w-full h-full flex items-center justify-center">
    <Disc3 class="w-10 h-10 text-text-disabled" />
  </div>
</div>
```

- `bg-bg-hover` 作为图片加载前底色
- 无封面时显示 `Disc3` 占位图标（`text-text-disabled`）
- `loading="lazy"` 懒加载
- `object-cover` 填充

### 4.2 封面来源优先级

```js
function getCoverSrc(album) {
  if (album.cover_thumb) return album.cover_thumb;        // 内联 base64（优先）
  if (album.cover_artwork_id) return getArtworkUrl(...);  // lumo://artwork 协议
  return '';                                               // 无封面
}
```

---

## 5. Hover 播放浮层（阴影例外）

### Anatomy

```
┌──────────────────┐
│                  │
│   ╔══════╗       │  封面 Hover 时：
│   ║  ▶   ║       │  - 黑色半透明蒙层 0 → 20%
│   ╚══════╝       │  - 圆形 Accent 播放按钮
│                  │  - shadow-overlay（受控阴影例外）
└──────────────────┘
```

### 参考代码

```html
<div
  class="absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors-smooth flex items-center justify-center opacity-0 group-hover:opacity-100"
  @click.stop="playAlbum(album)"
>
  <div class="w-10 h-10 rounded-full bg-brand-orange text-white flex items-center justify-center shadow-[0_4px_12px_rgba(0,0,0,0.15)]">
    <Play class="w-4 h-4 fill-current ml-0.5" />
  </div>
</div>
```

### 状态

| 元素 | Default | Hover |
|---|---|---|
| 蒙层 | `bg-black/0` + `opacity-0` | `bg-black/20` + `opacity-100` |
| 播放按钮 | 同蒙层隐藏 | 显示，`bg-brand-orange` + 白图标 |

### 为什么用 Accent

AlbumCard Hover 浮层播放按钮**用 Accent**（与 Transport 主按钮用黑不同）。原因：

- 浮层按钮是"卡片级动作入口"，需要视觉吸引
- 它是临时出现（仅 Hover），不构成"常驻 Accent 焦点"
- 与当前播放行的 Accent 不冲突（Hover 离开即消失）

### 为什么用阴影（例外）

- 浮层叠在封面之上，需深度提示"可点击"
- 使用 `--shadow-overlay` token，不自创阴影值
- 暗色下阴影加深（`rgba(0,0,0,0.4)`）
- 详见 [06 Elevation](06-elevation.md) §3.1

### 交互

- 单击封面：进入专辑详情（`@click="selectAlbum"`）
- 双击封面：直接播放第一首（`@dblclick="playAlbum"`）
- 单击浮层播放按钮：播放（`@click.stop="playAlbum"`，阻止冒泡到封面）

---

## 6. 文字区

```html
<p
  class="text-[15px] font-medium text-text-primary truncate mb-0.5"
  :class="playerStore.activeAlbumId === album.id ? 'text-brand-orange' : ''"
  @click="selectAlbum(album)"
>{{ album.title }}</p>
<p class="text-[13px] text-text-muted truncate">{{ album.artist }}<span v-if="album.year"> · {{ album.year }}</span></p>
```

| 元素 | 字号 | 字重 | 颜色 |
|---|---|---|---|
| 专辑名 | 15px | Medium | `text-text-primary` |
| 选中专辑名 | 15px | Medium | `text-brand-orange`（当前查看的专辑） |
| 艺术家 | 13px | Regular | `text-text-muted` |
| 年份 | 13px | Regular | `text-text-muted`（`· ` 分隔） |

- `truncate` 单行省略
- `mb-0.5` (2px) 标题与副标题间距

---

## 7. 卡片容器

```html
<div class="group cursor-pointer min-w-0" @dblclick="playAlbum(album)">
```

- `group` 启用 group-hover
- `cursor-pointer` 桌面点击提示
- `min-w-0` 防止 flex 子项溢出

---

## 8. 加载 / 错误 / 空态

网格级状态（非单卡片）：

```html
<!-- 加载 -->
<div v-if="isLoading && albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
  <Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
  <span class="text-[12px]">加载专辑…</span>
</div>

<!-- 空态 -->
<div v-else-if="albums.length === 0" class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
  <Disc3 class="w-8 h-8 text-text-disabled" />
  <span class="text-[12px]">没有找到专辑</span>
</div>
```

> 详见 [empty-and-loading.md](empty-and-loading.md)。

---

## 9. 无限加载

IntersectionObserver 触发：

```html
<div ref="sentinelRef" class="h-px" />
```

```js
observer = new IntersectionObserver(
  (entries) => {
    if (entries[0]?.isIntersecting && !isLoading.value && hasMoreAlbums.value) {
      playerStore.fetchAlbums(false);
    }
  },
  { root: gridContainer.value, rootMargin: '200px' }
);
```

- 哨兵 `h-px` 不可见
- `rootMargin: '200px'` 提前 200px 触发

---

## 10. Footer Status

```html
<div class="flex items-center justify-between py-4 border-t border-border-color mt-2 text-[11px] text-text-muted font-mono">
  <span>{{ totalCount.toLocaleString() }} 张专辑</span>
  <span>双击播放</span>
</div>
```

---

## 11. Tokens used

| Token | 用途 |
|---|---|
| `--bg-hover` | 封面底色 |
| `--text-primary` | 专辑名 |
| `--text-muted` | 艺术家、年份 |
| `--text-disabled` | 无封面占位图标 |
| `--brand-orange` | Hover 浮层按钮、选中专辑名 |
| `--shadow-overlay` | Hover 浮层阴影（例外） |
| `--radius-10` | 封面圆角 |
| `--border-color` | Footer 顶边 |

---

## 12. Do & Don't

### Do

- ✅ 1:1 方形封面 + `rounded-[10px]`
- ✅ Hover 浮层用 `--shadow-overlay`（受控例外）
- ✅ `loading="lazy"` 懒加载
- ✅ 无封面用 Disc3 + Disabled 色

### Don't

- ❌ 给卡片整体加阴影（仅 Hover 浮层按钮可）
- ❌ Hover 浮层按钮用黑色（应用 Accent）
- ❌ 封面用非 1:1 比例
- ❌ 网格间距不用 8pt 阶梯

---

*End of Album Card.*
