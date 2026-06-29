# Button

> 三种按钮：Primary / Secondary / Ghost。Transport Play 按钮单列规范。

---

## 1. Overview

LDL 按钮系统克制：**整个界面优先用 Ghost**，Primary 极少，Secondary 用于次级操作。每页面**最多一个 Primary**。

| 类型 | 用途 | 视觉 |
|---|---|---|
| Primary | 当前页面的主要动作（播放全部） | 黑底白字胶囊 |
| Secondary | 次级操作（随机播放） | 描边胶囊 |
| Ghost | 工具栏、TopBar、图标按钮 | 透明底，Hover 微背景 |
| Transport Play | 播放/暂停主按钮 | 黑底圆形（不是 Accent） |

---

## 2. Primary Button

### Anatomy

```
┌──────────────────────────────┐
│  [icon]  播放全部            │  黑底(#111) · 白字 · 胶囊 · h-34px
└──────────────────────────────┘
```

### Sizes

| 尺寸 | 高度 | 内边距 | 字号 | 圆角 |
|---|---|---|---|---|
| 标准 | 34px | `px-5` (20px) | 13px | `rounded-full` |
| 小（未来） | 28px | `px-4` | 12px | `rounded-full` |

### States

| 状态 | 背景 | 文字 | 处理 |
|---|---|---|---|
| Default | `bg-text-primary` (#111) | `text-bg-canvas` (#F7F5F1) | — |
| Hover | `opacity-90` | — | `transition-opacity` |
| Disabled | `opacity-50` | — | `disabled` 属性 |
| Focus | — | — | `outline: 2px Accent`（未来） |

### 参考代码

```html
<button
  class="h-[34px] px-5 rounded-full bg-text-primary text-bg-canvas text-[13px] font-medium flex items-center gap-2 hover:opacity-90 transition-opacity"
  @click="playAll"
>
  <Play class="w-[14px] h-[14px] fill-current" />
  播放全部
</button>
```

### Do & Don't

- ✅ 每页面最多一个 Primary
- ✅ 用 Primary 黑（`bg-text-primary`），**不用 Accent**
- ❌ 用 Accent 做 Primary 底色（违反 One Accent）
- ❌ Primary 加阴影

---

## 3. Secondary Button

### Anatomy

```
┌──────────────────────────────┐
│  [icon]  随机播放            │  描边 · 透明底 · 胶囊 · h-34px
└──────────────────────────────┘
```

### States

| 状态 | 背景 | 边框 | 文字 |
|---|---|---|---|
| Default | 透明 | `border-border-solid` | `text-text-primary` |
| Hover | `bg-list-hover` | — | — |
| Disabled | `opacity-50` | — | — |

### 参考代码

```html
<button
  class="h-[34px] px-4 rounded-full border border-border-solid text-[13px] font-medium text-text-primary flex items-center gap-2 hover:bg-list-hover transition-colors-smooth"
  @click="shufflePlay"
>
  <Shuffle class="w-[14px] h-[14px]" />
  随机播放
</button>
```

---

## 4. Ghost Button（文字版）

用于 Inspector、未来 Modal 的取消按钮等。

### States

| 状态 | 背景 | 文字 |
|---|---|---|
| Default | 透明 | `text-text-secondary` |
| Hover | `bg-btn-hover` | `text-text-primary` |

```html
<button class="px-3 py-1.5 rounded-[6px] text-[13px] text-text-secondary hover:bg-btn-hover hover:text-text-primary transition-colors-smooth">
  取消
</button>
```

---

## 5. Transport Play Button（特例）

播放/暂停主按钮是**圆形**，使用 Primary 黑，**不用 Accent**。

### Anatomy

```
    ┌──────┐
    │  ▶   │  48×48 圆形 · 黑底 · 白图标
    └──────┘
```

### Sizes

| 尺寸 | 直径 | 图标 |
|---|---|---|
| 标准 | 48px | 20px |

### States

| 状态 | 背景 | 图标 | 处理 |
|---|---|---|---|
| Default | `bg-text-primary` | `text-bg-canvas` | — |
| Hover | `opacity-90` | — | `transition-opacity` |
| Disabled | `opacity-50` | — | `!playerStore.currentTrack` |

### 参考代码

```html
<button
  class="w-[48px] h-[48px] rounded-full bg-text-primary text-bg-canvas flex items-center justify-center hover:opacity-90 transition-opacity"
  :disabled="!playerStore.currentTrack"
  @click="playerStore.togglePlay()"
>
  <Pause v-if="playerStore.isPlaying" class="w-[20px] h-[20px] fill-current" />
  <Play v-else class="w-[20px] h-[20px] fill-current ml-0.5" />
</button>
```

### 为什么不用 Accent

Transport 是**中性控件**，每秒都在用。Accent 留给"当前状态指示"（playing/selected/progress），如果 Play 按钮用 Accent，会与当前播放行的 Accent 指示冲突，违反 One Accent Rule。

---

## 6. Transport Skip Buttons

上一首 / 下一首是图标按钮，Hover 时转 Accent。

```html
<button
  class="text-text-primary hover:text-brand-orange transition-colors-smooth"
  @click="playerStore.prevTrack()"
  title="上一首"
  aria-label="上一首"
>
  <SkipBack class="w-[18px] h-[18px] fill-current" />
</button>
```

- Default：`text-text-primary`
- Hover：`text-brand-orange`（Hover 强调，允许场景）

---

## 7. Tokens used

| Token | 按钮 |
|---|---|
| `--text-primary` | Primary 底、Skip Default 文字 |
| `--bg-canvas` | Primary 文字反色 |
| `--border-solid` | Secondary 边框 |
| `--list-hover` | Secondary Hover |
| `--btn-hover` | Ghost Hover |
| `--brand-orange` | Skip Hover |
| `--radius-full` | Primary / Secondary / Transport |

---

*End of Button.*
