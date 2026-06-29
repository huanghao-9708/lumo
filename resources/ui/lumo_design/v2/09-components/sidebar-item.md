# Sidebar Item

> 导航项 + 歌单项 + 计数徽章 + Accent 指示点。

---

## 1. Overview

Sidebar Item 是左侧导航的主力元素。两种变体：

| 变体 | 用途 |
|---|---|
| NavItem | Library 一级导航（全部歌曲、专辑、艺术家…） |
| PlaylistItem | Playlists 分组下的歌单 |

---

## 2. NavItem

### Anatomy

```
┌────────────────────────────────────────────┐
│ [icon]  全部歌曲              643  •       │  左图标 + 文字 + 计数 + Accent 点
└────────────────────────────────────────────┘
```

### Sizes

| 尺寸 | 高度 | 内边距 | 圆角 |
|---|---|---|---|
| 标准 | ~30px (`py-[7px]`) | `px-3` | `rounded-[6px]` |

### 结构

```
[icon 16px] --mr-3--> [label 13px flex-1] [count 11px mono] [• 6px Accent]
```

### States

| 状态 | 背景 | 图标色 | 文字 | 计数 | Accent 点 |
|---|---|---|---|---|---|
| Default | 透明 | `text-text-muted` | `text-text-primary` Regular | `text-text-muted` | 无 |
| Hover | `bg-list-hover` | `text-text-muted` | `text-text-primary` | `text-text-muted` | 无 |
| Selected | `bg-list-selected` | `text-brand-orange` | `text-text-primary` Medium | `text-text-secondary` | 6px Accent 圆点 |

### 参考代码

```html
<a
  href="#"
  class="flex items-center px-3 py-[7px] rounded-[6px] transition-colors-smooth"
  :class="isActive(item.key)
    ? 'bg-list-selected text-text-primary'
    : 'text-text-primary hover:bg-list-hover'"
  @click.prevent="selectNav(item.key)"
  :aria-current="isActive(item.key) ? 'page' : undefined"
>
  <component
    :is="item.icon"
    class="w-[16px] h-[16px] mr-3 flex-shrink-0"
    :class="isActive(item.key) ? 'text-brand-orange' : 'text-text-muted'"
    aria-hidden="true"
  />
  <span
    class="text-[13px] flex-1"
    :class="isActive(item.key) ? 'font-medium' : ''"
  >{{ item.label }}</span>
  <span
    v-if="item.count !== null && item.count > 0"
    class="text-[11px] font-mono tabular-nums"
    :class="isActive(item.key) ? 'text-text-secondary' : 'text-text-muted'"
  >{{ item.count.toLocaleString() }}</span>
  <div
    v-if="isActive(item.key)"
    class="w-[6px] h-[6px] rounded-full bg-brand-orange ml-2 flex-shrink-0"
  ></div>
</a>
```

### Selected 态的 Accent 使用

Selected 态用**两处 Accent**，但不违反 One Accent Rule：

1. **图标色** → `text-brand-orange`（图标是状态指示，允许）
2. **6px 圆点** → `bg-brand-orange`（明确"当前选中"指示）

这两处是同一语义（"当前选中"）的不同表达，视为一个焦点。

### 计数徽章规则

- 仅当 `count > 0` 时显示
- 11px mono + `tabular-nums` 等宽对齐
- Default: `text-text-muted`
- Selected: `text-text-secondary`（提亮一级）

---

## 3. PlaylistItem

### Anatomy

```
┌────────────────────────────────────────────┐
│ [List icon]  Focus Flow              12    │  歌单图标 + 名称 + 曲目数
└────────────────────────────────────────────┘
```

### 与 NavItem 的差异

| 项 | NavItem | PlaylistItem |
|---|---|---|
| 图标 | 各项不同（Disc/User/Folder…） | 统一 `List` 图标 |
| 计数 | 项目总数 | 曲目数 |
| Accent 点 | Selected 时显示 | **不显示**（避免过度） |
| Selected 图标色 | Accent | `text-text-muted`（保持安静） |

### 参考代码

```html
<a
  href="#"
  class="flex items-center px-3 py-[7px] rounded-[6px] transition-colors-smooth"
  :class="playerStore.activePlaylistId === pl.id
    ? 'bg-list-selected text-text-primary'
    : 'text-text-primary hover:bg-list-hover'"
  @click.prevent="selectPlaylist(pl.id)"
>
  <List class="w-[16px] h-[16px] mr-3 text-text-muted flex-shrink-0" aria-hidden="true" />
  <span class="text-[13px] flex-1 truncate">{{ pl.name }}</span>
  <span class="text-[11px] font-mono text-text-muted tabular-nums">{{ pl.count }}</span>
</a>
```

---

## 4. 新建播放列表项

特殊样式：Muted 文字 + Plus 图标，Hover 提亮。

```html
<a
  href="#"
  class="flex items-center px-3 py-[7px] rounded-[6px] text-text-muted hover:bg-list-hover hover:text-text-primary transition-colors-smooth"
  @click.prevent="playerStore.isCreatePlaylistModalOpen = true"
>
  <Plus class="w-[16px] h-[16px] mr-3 flex-shrink-0" aria-hidden="true" />
  <span class="text-[13px] flex-1">新建播放列表</span>
</a>
```

- Default: `text-text-muted`（低存在感，引导但不抢）
- Hover: `bg-list-hover` + `text-text-primary`

---

## 5. Section 标题

分组标题（Library / Playlists）：

```html
<h2 class="px-3 text-[10px] font-semibold text-text-muted mb-2 uppercase tracking-widest">
  Library
</h2>
```

- 10px Semibold + `uppercase` + `tracking-widest` + Muted
- `mb-2` (8px) 与列表项间距
- `px-3` 与列表项左对齐

---

## 6. 列表间距

```html
<ul class="space-y-[2px]">
```

- 列表项之间 `2px` 微间距（`space-y-[2px]`）
- 分组之间 `mb-6` (24px)

---

## 7. Tokens used

| Token | 用途 |
|---|---|
| `--list-hover` | Hover 背景 |
| `--list-selected` | Selected 背景 |
| `--text-primary` | 文字 |
| `--text-muted` | 图标、计数、Section 标题 |
| `--text-secondary` | Selected 计数 |
| `--brand-orange` | Selected 图标色 + Accent 点 |
| `--radius-6` | 圆角 |

---

## 8. Do & Don't

### Do

- ✅ Selected 用图标 Accent + 6px 圆点双重指示
- ✅ 计数用 mono + tabular-nums
- ✅ Section 标题 uppercase + tracking-widest
- ✅ PlaylistItem 不加 Accent 点（克制）

### Don't

- ❌ Selected 用 Bold（应用 Medium）
- ❌ 给列表项加左边框（用 6px 圆点）
- ❌ 计数用 sans 字体（应用 mono）
- ❌ Section 标题不加 tracking

---

*End of Sidebar Item.*
