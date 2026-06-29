# 08 Accessibility

> **Accessibility by Default.** 不是补充功能，而是默认能力。

LUMO 定位为"专业"音乐管理工具，无障碍是成熟度标志。所有组件默认满足：键盘导航、WCAG AA、Focus Visible、Screen Reader。

---

## 1. 键盘导航

### 1.1 全局快捷键（已实现于 `App.vue`）

| 键 | 行为 | 备注 |
|---|---|---|
| `Space` | 播放 / 暂停 | 输入框聚焦时除外 |
| `Ctrl + →` | 下一首 | |
| `Ctrl + ←` | 上一首 | |
| `→` | 快进 5 秒 | |
| `←` | 快退 5 秒 | |
| `↑` | 音量 +5 | |
| `↓` | 音量 -5 | |

### 1.2 Tab 顺序

```
Sidebar Items → TopBar Buttons → Content Toolbar → List Items → Inspector Tabs → Playback Controls
```

- 所有可交互元素必须 `tabindex="0"`（默认）或可聚焦
- 装饰性图标 `tabindex="-1"` 或 `aria-hidden="true"`
- 禁用元素 `tabindex="-1"` + `disabled`

### 1.3 输入框保护

全局键盘事件**必须排除输入框**：

```js
const activeEl = document.activeElement;
if (activeEl && (
  activeEl.tagName === 'INPUT' ||
  activeEl.tagName === 'TEXTAREA' ||
  activeEl.getAttribute('contenteditable') === 'true'
)) {
  return; // 不拦截
}
```

### 1.4 双击播放

Song Row 双击播放（`@dblclick`）是桌面应用约定，但**必须同时支持键盘**：

- Enter / Space 在聚焦的 Song Row 上触发播放（未来实现 `@keydown.enter` `@keydown.space`）
- 当前仅双击，键盘支持列为 v2.1 补全项

---

## 2. Focus Ring

### 2.1 原则

- **Focus 必须可见**（Focus Visible）
- 焦点指示不能依赖颜色变化（色盲用户不可见）
- 不用 `outline: none` 全局禁用，仅在特定控件用替代焦点样式

### 2.2 输入框 Focus

```css
input:focus, textarea:focus, select:focus {
  outline: none;
}
```

输入框用**边框色变化**替代 outline：

```html
<input class="border border-border-color focus:border-brand-orange/50 transition-colors-smooth" />
```

- focus 时 border 变 Accent 半透明，200ms 过渡
- 这是 Accent 的合法用法（Focus 是允许场景）

### 2.3 按钮 Focus

按钮当前依赖浏览器默认 outline。**未来规范**：

```css
button:focus-visible {
  outline: 2px solid var(--brand-orange);
  outline-offset: 2px;
}
```

- `:focus-visible` 仅键盘聚焦时显示，鼠标点击不显示
- 2px Accent outline + 2px offset
- token 对齐轮次落地

### 2.4 拖拽区域

进度条 / 音量旋钮是鼠标拖拽控件，键盘用户需替代方案：

- 进度条：聚焦后 `←` `→` 调整 5 秒（已通过全局快捷键实现）
- 音量旋钮：聚焦后 `↑` `↓` 调整（已通过全局快捷键实现）
- 旋钮需 `tabindex="0"` + `role="slider"` + `aria-valuenow`（未来补全）

---

## 3. 颜色对比度（WCAG AA）

### 3.1 要求

| 等级 | 要求 | LUMO 应用 |
|---|---|---|
| AA 正文 | ≥ 4.5:1 | < 14px 文字 |
| AA 大字 | ≥ 3:1 | ≥ 14px 或粗体 |
| AA 图形 | ≥ 3:1 | 图标、状态点 |

### 3.2 当前组合审计

| 组合 | 对比度 | 等级 | 状态 |
|---|---|---|---|
| Primary `#111` on Canvas `#F7F5F1` | 18.9:1 | AAA | ✓ |
| Secondary `#5F5F5F` on Canvas | 7.1:1 | AAA | ✓ |
| Muted `#8B8B8B` on Canvas | 3.9:1 | AA Large | ⚠️ 仅 ≥14px |
| Accent `#E28A23` on Canvas | 3.4:1 | AA Large | ⚠️ 仅图标/≥18px |
| Disabled `#BDBDBD` on Canvas | 1.7:1 | — | ❌ 仅非文字 |

### 3.3 规则

- **正文（< 14px）**：必须 Primary 或 Secondary
- **Muted**：仅用于 ≥14px 辅助文字，或非文字（图标、计数）
- **Accent 文字**：仅用于当前播放曲名（≥13px + semibold 视觉补偿）或图标
- **Disabled**：不用于文字，仅用于非关键图形

### 3.4 v1.0 修正

v1.0 的 `--text-muted: #999999` 对比度仅 2.8:1，**不达 AA**。v2.0 改为 `#8B8B8B`（3.9:1，达 AA Large）。

---

## 4. ARIA 与 Screen Reader

### 4.1 基本原则

- 语义化 HTML 优先（`<button>` `<nav>` `<ul>` `<a>`），不滥用 `<div>`
- 装饰性图标 `aria-hidden="true"`
- 图标按钮必须有 `aria-label` 或 `title`

### 4.2 图标按钮

```html
<button
  class="w-8 h-8 ..."
  :title="uiStore.isDarkMode ? '切换到日间模式' : '切换到夜间模式'"
  :aria-label="uiStore.isDarkMode ? '切换到日间模式' : '切换到夜间模式'"
  @click="uiStore.toggleDarkMode()"
>
  <Moon class="w-[18px] h-[18px]" aria-hidden="true" />
</button>
```

- `title` 提供鼠标悬浮提示
- `aria-label` 提供屏幕阅读器标签
- 图标 `aria-hidden="true"` 避免重复朗读

### 4.3 列表语义

```html
<!-- Sidebar -->
<nav aria-label="库导航">
  <ul>
    <li><a href="#" aria-current="page">{{ item.label }}</a></li>
  </ul>
</nav>

<!-- Song Row（未来语义化） -->
<tr> 或 <li role="row" aria-selected="true">
```

### 4.4 当前播放状态

```html
<span :aria-label="isPlaying ? '正在播放' : '已暂停'">
  <Loader2 v-if="isPlaying" class="animate-spin" aria-hidden="true" />
  <Play v-else aria-hidden="true" />
</span>
```

### 4.5 进度条 / 旋钮

未来需补全：

```html
<!-- 进度条 -->
<div role="slider" tabindex="0"
  aria-label="播放进度"
  aria-valuemin="0" :aria-valuemax="durationMs" :aria-valuenow="progressMs"
  aria-valuetext="{{ currentTimeText }} / {{ totalTimeText }}">

<!-- 音量旋钮 -->
<div role="slider" tabindex="0"
  aria-label="音量"
  aria-valuemin="0" aria-valuemax="100" :aria-valuenow="volume"
  aria-valuetext="音量 {{ volume }}">
```

### 4.6 空态 / 加载态

```html
<div role="status" aria-live="polite">
  <Loader2 class="animate-spin" aria-hidden="true" />
  <span>加载中…</span>
</div>

<div role="alert" v-if="isError">
  加载失败，请稍后重试
</div>
```

---

## 5. 桌面应用专属约定

### 5.1 窗口拖拽

TopBar 是拖拽区域（`data-tauri-drag-region`），但按钮必须 `pointer-events-auto` 且 `-webkit-app-region: no-drag`：

```css
[data-tauri-drag-region] {
  -webkit-app-region: drag;
  user-select: none;
}
button, input, a, select, textarea {
  -webkit-app-region: no-drag;
}
```

### 5.2 双击 vs 单击

- Song Row：**双击**播放（单击仅选中/聚焦）
- AlbumCard 封面：**单击**进入详情，**双击**直接播放
- 桌面用户习惯双击，但需为键盘用户提供 Enter 替代（v2.1）

### 5.3 滚轮调节

音量旋钮支持滚轮（`@wheel`），桌面应用特有交互：

```html
<div @wheel="onKnobWheel" title="拖动 / 滚轮调节音量">
```

- 滚轮上 = +5，下 = -5
- `e.preventDefault()` 防止页面滚动

---

## 6. Do & Don't

### Do

- ✅ 所有可交互元素键盘可达
- ✅ 图标按钮有 `aria-label` / `title`
- ✅ 装饰图标 `aria-hidden="true"`
- ✅ 输入框 focus 用边框色变化
- ✅ 支持 `prefers-reduced-motion`

### Don't

- ❌ 全局 `outline: none` 禁用焦点
- ❌ Muted 色用于 < 14px 正文
- ❌ 用颜色单独表达状态（需配图标/文字）
- ❌ div 滥用替代 button / a
- ❌ 进度条 / 旋钮无 ARIA slider 角色

---

*End of Accessibility.*
