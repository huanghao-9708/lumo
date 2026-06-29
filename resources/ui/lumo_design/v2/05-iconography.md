# 05 Iconography

> 线框至上，克制填色。图标是辅助，不是装饰。

---

## 1. 图标库

LUMO 使用 **[lucide-vue-next](https://lucide.dev)** 作为唯一图标库。

| 项 | 值 |
|---|---|
| 包 | `lucide-vue-next` |
| 风格 | 极简线框（Outline），Phosphor/Lucide 系 |
| 引入 | 按需 `import { Play, Heart } from 'lucide-vue-next'` |

### 选择理由

- 线框风格，与 Warm Industrial Minimalism 一致
- Stroke 宽度统一可控
- Vue 3 原生支持，tree-shaking 友好
- 不使用填充式 / 拟物图标

---

## 2. Stroke 宽度

| 场景 | Stroke | 说明 |
|---|---|---|
| 默认 | **1.5px** | lucide 默认，所有 UI 图标 |
| 强调 | 2px | 仅用于需要强突出的图标（极少用） |

**规则**：整个应用**统一 1.5px**，不混用。lucide 默认即 1.5，无需额外设置。

```html
<!-- ✅ 默认 1.5px stroke -->
<Play class="w-[16px] h-[16px]" />

<!-- ❌ 不要手动改 stroke -->
<Play :stroke-width="3" class="w-[16px] h-[16px]" />
```

---

## 3. 尺寸阶梯

| 尺寸 | 用途 |
|---|---|
| 12px | 极小：表头 Clock 图标、更多操作 |
| 14px | 小：搜索图标、Song Row 序号位 Play、Heart |
| 16px | **默认**：Sidebar Item、Transport 模式按钮、Heart |
| 18px | 中：TopBar 按钮、Inspector Heart/More、Transport Skip |
| 20px | 大：Transport Play/Pause、空态主图标 |
| 24px | 特大（预留）：Modal 标题 |
| 32px / 40px | 空态/占位：Disc3 等示意图标（`w-8 h-8` / `w-10 h-10`，用 `--text-disabled` 色） |

### 尺寸使用规则

- 同一控件组内**尺寸统一**
- Transport 按钮组：Mode 16 / Skip 18 / Play 20（主次分明）
- TopBar 按钮组：全部 18
- Sidebar Item：全部 16

---

## 4. 颜色继承 currentColor

lucide 图标默认 `stroke="currentColor"`，**通过父级文字色控制**，不直接设 stroke 色。

```html
<!-- ✅ 通过 text-* 控制颜色 -->
<button class="text-text-muted hover:text-text-primary">
  <Play class="w-4 h-4" />
</button>

<!-- ❌ 不直接设颜色属性 -->
<Play class="w-4 h-4" color="#5F5F5F" />
```

### 图标配色规则

| 状态 | 颜色 |
|---|---|
| 默认 | `text-text-muted` |
| Hover | `text-text-primary` |
| Selected/Active | `text-brand-orange` |
| 禁用 | `text-text-disabled` |
| 当前播放 | `text-brand-orange` |
| 收藏激活 | `text-brand-orange` + `fill-current` |

---

## 5. 线框 vs 填充

LDL 默认**线框（Outline）**。填充（`fill-current`）仅用于以下场景：

| 场景 | 图标 | 处理 |
|---|---|---|
| Transport Play/Pause/Skip | Play Pause SkipBack SkipForward | `fill-current` |
| 收藏激活 | Heart | `fill-current` + `text-brand-orange` |
| 当前播放序号位 | Play | `fill-current` + `text-brand-orange` |
| AlbumCard Hover 播放浮层 | Play | `fill-current` + 白色 |

### 规则

- **播放相关图标一律 fill**：Play / Pause / SkipBack / SkipForward，因为它们是"动作实体"
- **功能图标保持 outline**：Heart（未收藏）/ Search / More / Settings / Moon / Sun
- **Heart 双态**：未收藏 outline + muted，已收藏 fill + Accent

```html
<!-- 收藏激活 -->
<Heart class="w-[14px] h-[14px] text-brand-orange fill-current" />

<!-- 未收藏 -->
<Heart class="w-[14px] h-[14px] text-text-disabled" />

<!-- 播放图标 -->
<Play class="w-[20px] h-[20px] fill-current" />
```

---

## 6. 图标 + 文字组合

### 6.1 左图标 + 文字（Sidebar Item）

```html
<a class="flex items-center px-3 py-[7px]">
  <component :is="item.icon" class="w-[16px] h-[16px] mr-3 flex-shrink-0" />
  <span class="text-[13px] flex-1">{{ item.label }}</span>
</a>
```

- 图标 `mr-3` (12px) 与文字间距
- 图标 `flex-shrink-0`，文字 `flex-1 truncate`

### 6.2 左图标 + 标签（Playback Volume）

```html
<span class="flex items-center gap-1">
  <Volume class="w-[11px] h-[11px]" />
  Volume
</span>
```

- 小图标 + 紧凑标签，`gap-1` (4px)

### 6.3 图标按钮（纯图标）

```html
<button class="w-8 h-8 flex items-center justify-center rounded-[8px]">
  <Moon class="w-[18px] h-[18px]" />
</button>
```

- 按钮固定尺寸，图标 `flex` 居中

---

## 7. 常用图标清单

### Navigation

| 图标 | 用途 |
|---|---|
| `Activity` | 全部歌曲 |
| `Disc` / `Disc3` | 专辑 |
| `User` | 艺术家 |
| `Heart` | 收藏 / 作曲家 |
| `Folder` | 文件夹 |
| `Clock` | 最近播放 / 时长 |
| `ListMusic` | 播放列表 |
| `List` | 列表视图 |
| `LayoutGrid` | 网格视图 |

### Playback

| 图标 | 用途 |
|---|---|
| `Play` | 播放（fill） |
| `Pause` | 暂停（fill） |
| `SkipBack` | 上一首（fill） |
| `SkipForward` | 下一首（fill） |
| `Shuffle` | 随机 |
| `Repeat` | 循环 |
| `Repeat1` | 单曲循环 |
| `Volume` / `Volume1` / `Volume2` | 音量三档 |
| `ChevronDown` | 展开 / Output 选择 |

### Actions

| 图标 | 用途 |
|---|---|
| `Search` | 搜索 |
| `Plus` | 新建 |
| `MoreHorizontal` | 更多操作 |
| `ArrowLeft` | 返回 |
| `PanelRight` | 右栏切换 |
| `Sun` / `Moon` | 主题切换 |
| `Minus` / `Square` / `X` | 窗口控制 |
| `Loader2` | 加载中（`animate-spin`） |

### Feedback

| 图标 | 用途 |
|---|---|
| `Disc3` | 空态占位（专辑/播放） |
| `Music` | 空态占位（歌曲） |
| `ListMusic` | 空态占位（队列） |

---

## 8. 空态图标

空态用大尺寸 + Disabled 色，不带交互：

```html
<Music class="w-8 h-8 text-text-disabled" />
<Disc3 class="w-10 h-10 text-text-disabled" />
```

- 尺寸：`w-8 h-8` (32px) 或 `w-10 h-10` (40px)
- 颜色：`text-text-disabled`（不抢注意力）
- 不加 fill，保持 outline

---

## 9. Do & Don't

### Do

- ✅ 统一 lucide 图标库，不混用其他库
- ✅ 1.5px stroke，currentColor 继承
- ✅ 播放图标 fill-current，功能图标 outline
- ✅ 图标尺寸与控件尺寸匹配（按钮 32px → 图标 18px）

### Don't

- ❌ 使用填充式 / 拟物 / 彩色图标
- ❌ 手动改 stroke-width
- ❌ 图标大于控件（如 24px 图标塞进 32px 按钮会很挤，应用 18px）
- ❌ 用颜色属性 `color="..."`，应用 text-* class
- ❌ 在空态用 Accent 色图标（应 Disabled 色）

---

*End of Iconography.*
