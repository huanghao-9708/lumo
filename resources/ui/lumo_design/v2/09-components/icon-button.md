# Icon Button

> 纯图标按钮。TopBar、Inspector、Toolbar 的主力控件。

---

## 1. Overview

Icon Button 是 LDL 中数量最多的控件。**克制是核心**：默认低存在感，Hover 才显现。

---

## 2. Sizes

| 尺寸 | 按钮尺寸 | 图标尺寸 | 圆角 | 用途 |
|---|---|---|---|---|
| 标准 | 32×32 (`w-8 h-8`) | 18px | `rounded-[8px]` | TopBar、Inspector |
| 小 | 28×28 (`w-7 h-7`) | 14px | `rounded-[6px]` | 视图切换内按钮 |
| 特小 | 24×24（未来） | 12px | `rounded-[6px]` | 极少 |

### 图标与按钮比例

- 32px 按钮 → 18px 图标（图标占 56%）
- 28px 按钮 → 14px 图标（图标占 50%）
- 图标 `flex items-center justify-center` 居中

---

## 3. States

### 3.1 默认 Ghost Icon Button

| 状态 | 图标色 | 背景 | 处理 |
|---|---|---|---|
| Default | `text-text-secondary` | 透明 | — |
| Hover | `text-text-primary` | `bg-bg-hover` | `transition-colors-smooth` |
| Disabled | `text-text-disabled` | — | `disabled` + `cursor-not-allowed` |

```html
<button
  class="w-8 h-8 flex items-center justify-center rounded-[8px] text-text-secondary hover:text-text-primary hover:bg-bg-hover transition-colors-smooth"
  title="更多"
  aria-label="更多"
>
  <MoreHorizontal class="w-[18px] h-[18px]" aria-hidden="true" />
</button>
```

### 3.2 Muted Icon Button（低存在感）

用于次要操作（更多、面板切换未激活）：

| 状态 | 图标色 | 背景 |
|---|---|---|
| Default | `text-text-muted` | 透明 |
| Hover | `text-text-primary` | `bg-bg-hover` |

```html
<button class="w-8 h-8 flex items-center justify-center rounded-[8px] text-text-muted hover:text-text-primary hover:bg-bg-hover transition-colors-smooth">
```

### 3.3 Active Icon Button（Accent 激活态）

用于状态切换按钮（夜间模式开启、面板已显示）：

| 状态 | 图标色 | 背景 |
|---|---|---|
| Default（激活） | `text-brand-orange` | `bg-bg-active` (Accent /10) |
| Hover | — | — |

```html
<button
  class="w-8 h-8 flex items-center justify-center rounded-[8px] transition-colors-smooth"
  :class="uiStore.isDarkMode
    ? 'text-brand-orange bg-bg-active'
    : 'text-text-secondary hover:text-text-primary hover:bg-bg-hover'"
  :title="uiStore.isDarkMode ? '切换到日间模式' : '切换到夜间模式'"
  :aria-label="uiStore.isDarkMode ? '切换到日间模式' : '切换到夜间模式'"
  @click="uiStore.toggleDarkMode()"
>
  <Moon v-if="!uiStore.isDarkMode" class="w-[18px] h-[18px]" aria-hidden="true" />
  <Sun v-else class="w-[18px] h-[18px]" aria-hidden="true" />
</button>
```

### 3.4 危险 Icon Button

仅用于窗口关闭按钮：

| 状态 | 图标色 | 背景 |
|---|---|---|
| Default | `text-text-secondary` | 透明 |
| Hover | `text-white` | `bg-[#E81123]`（Windows 标准红） |

```html
<button
  class="w-8 h-8 flex items-center justify-center rounded-[8px] text-text-secondary hover:text-white hover:bg-[#E81123] transition-colors-smooth"
  title="关闭"
  aria-label="关闭窗口"
  @click="close"
>
  <X class="w-4 h-4" aria-hidden="true" />
</button>
```

> `#E81123` 是 Windows 标准关闭红，**不纳入 token 系统**，作为平台约定写死。

---

## 4. 窗口控制按钮组

TopBar 右侧的最小化 / 最大化 / 关闭：

```
[─] [□] [✕]
```

| 按钮 | 图标 | 尺寸 | Hover |
|---|---|---|---|
| 最小化 | `Minus` `w-4 h-4` | 32×32 | `bg-bg-hover` |
| 最大化 | `Square` `w-3.5 h-3.5` | 32×32 | `bg-bg-hover` |
| 关闭 | `X` `w-4 h-4` | 32×32 | `bg-[#E81123]` + 白字 |

```html
<div class="flex items-center gap-1 pointer-events-auto ml-1">
  <button @click="minimize" class="w-8 h-8 ..."> <Minus class="w-4 h-4" /> </button>
  <button @click="toggleMaximize" class="w-8 h-8 ..."> <Square class="w-3.5 h-3.5" /> </button>
  <button @click="close" class="w-8 h-8 ... hover:bg-[#E81123] hover:text-white"> <X class="w-4 h-4" /> </button>
</div>
```

---

## 5. 视图切换按钮组

Content Toolbar 的列表 / 网格切换：

```
┌─────────────┐
│ [☰] [▦]     │  容器 bg-bg-canvas + border · p-[2px]
└─────────────┘
```

```html
<div class="flex items-center gap-0 bg-bg-canvas border border-border-color rounded-[8px] p-[2px]">
  <button
    class="w-7 h-7 flex items-center justify-center rounded-[6px] transition-colors-smooth"
    :class="viewMode === 'list' ? 'bg-list-selected text-text-primary' : 'text-text-muted hover:text-text-primary'"
    @click="viewMode = 'list'"
    title="列表视图"
    aria-label="切换到列表视图"
  >
    <List class="w-[14px] h-[14px]" aria-hidden="true" />
  </button>
  <button
    class="w-7 h-7 flex items-center justify-center rounded-[6px] transition-colors-smooth"
    :class="viewMode === 'grid' ? 'bg-list-selected text-text-primary' : 'text-text-muted hover:text-text-primary'"
    @click="viewMode = 'grid'"
    title="网格视图"
    aria-label="切换到网格视图"
  >
    <LayoutGrid class="w-[14px] h-[14px]" aria-hidden="true" />
  </button>
</div>
```

- 容器 28px 高，内按钮 28×28
- 激活：`bg-list-selected` + `text-text-primary`（不用 Accent）
- 未激活：`text-text-muted` + Hover `text-text-primary`

---

## 6. Tokens used

| Token | 用途 |
|---|---|
| `--bg-hover` | Ghost Hover 背景 |
| `--bg-active` | Active Accent 背景 |
| `--text-secondary` | Default 图标色 |
| `--text-muted` | 低存在感图标色 |
| `--text-primary` | Hover 图标色 |
| `--brand-orange` | Active 图标色 |
| `--list-selected` | 视图切换激活背景 |
| `--border-solid` / `--border-color` | 视图切换容器边框 |
| `--radius-8` / `--radius-6` | 圆角 |

---

## 7. Do & Don't

### Do

- ✅ 32×32 标准，图标 18px
- ✅ 必须有 `title` + `aria-label`
- ✅ 图标 `aria-hidden="true"`
- ✅ 激活态用 `bg-bg-active` + Accent 图标

### Don't

- ❌ 图标按钮加文字（变成 Ghost Button）
- ❌ Active 态用纯 Accent 实色背景（太抢眼，应用 /10 透明）
- ❌ 自定义按钮尺寸（用标准 32 / 28）
- ❌ 关闭按钮 Hover 用 Accent（应用 Windows 红）

---

*End of Icon Button.*
