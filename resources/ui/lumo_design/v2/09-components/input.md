# Input

> 搜索框是 Content Toolbar 的核心控件。

---

## 1. Overview

LDL 当前只有一个输入场景：**搜索框**。未来扩展设置页表单时，本规范适用。

---

## 2. Search Input

### Anatomy

```
┌──────────────────────────────────────────┐
│  [🔍]  搜索歌曲、艺术家、专辑…           │  h-32px · 左图标 · placeholder
└──────────────────────────────────────────┘
```

### Sizes

| 尺寸 | 高度 | 字号 | 圆角 |
|---|---|---|---|
| 标准 | 32px (`h-[32px]`) | 12px | `rounded-[8px]` |

### States

| 状态 | 背景 | 边框 | 文字 | placeholder |
|---|---|---|---|---|
| Default | `bg-bg-canvas` | `border-border-color` | `text-text-primary` | `text-text-muted` |
| Focus | `bg-bg-canvas` | `border-brand-orange/50` | `text-text-primary` | `text-text-muted` |
| Disabled | — | `border-border-color` | `text-text-disabled` | — |

### 参考代码

```html
<div class="relative flex-1 max-w-[280px]">
  <!-- 左侧搜索图标，绝对定位 -->
  <Search
    class="w-[14px] h-[14px] text-text-muted absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none"
    aria-hidden="true"
  />
  <input
    v-model="searchInput"
    @input="onSearchInput"
    type="text"
    placeholder="搜索歌曲、艺术家、专辑…"
    class="w-full h-[32px] pl-8 pr-3 text-[12px] bg-bg-canvas border border-border-color rounded-[8px] text-text-primary placeholder:text-text-muted transition-colors-smooth focus:border-brand-orange/50"
    aria-label="搜索"
  />
</div>
```

### 关键细节

1. **左侧图标绝对定位**：`absolute left-3 top-1/2 -translate-y-1/2`，`pointer-events-none` 不挡点击
2. **输入框左内边距**：`pl-8` (32px) 给图标留位
3. **Focus 描边**：`focus:border-brand-orange/50`（Accent 半透明，合法 Focus 用法）
4. **去 outline**：`style.css` 全局 `input:focus { outline: none; }`，用边框替代
5. **防抖**：250ms 防抖后触发搜索（`setTimeout`）

### 防抖实现

```js
const searchInput = ref('');
let searchTimer: ReturnType<typeof setTimeout> | null = null;
function onSearchInput() {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    playerStore.searchQuery = searchInput.value;
    loadForCurrentTab();
  }, 250);
}
onBeforeUnmount(() => { if (searchTimer) clearTimeout(searchTimer); });
```

---

## 3. 未来：表单 Input（设置页）

未来设置页的文本输入框规范（预留）：

| 尺寸 | 高度 | 字号 | 圆角 |
|---|---|---|---|
| 标准 | 36px | 13px | `rounded-[8px]` |

```html
<input
  type="text"
  class="w-full h-9 px-3 text-[13px] bg-bg-canvas border border-border-color rounded-[8px] text-text-primary placeholder:text-text-muted transition-colors-smooth focus:border-brand-orange/50"
/>
```

---

## 4. Tokens used

| Token | 用途 |
|---|---|
| `--bg-canvas` | 输入框背景 |
| `--border-color` | 默认边框 |
| `--brand-orange` | Focus 边框（/50 透明） |
| `--text-primary` | 输入文字 |
| `--text-muted` | placeholder + 搜索图标 |
| `--radius-8` | 圆角 |

---

## 5. Do & Don't

### Do

- ✅ 左图标 + `pl-8` 留位
- ✅ Focus 用 `border-brand-orange/50`（Accent 合法场景）
- ✅ 250ms 防抖
- ✅ `aria-label="搜索"`

### Don't

- ❌ 用 `bg-bg-content`（应 Canvas，与内容区区分）
- ❌ Focus 用纯 Accent 实色边框（太抢眼，应用 /50）
- ❌ 搜索图标不设 `pointer-events-none`（会挡输入框点击）
- ❌ placeholder 用 `text-text-primary`（应 Muted）

---

*End of Input.*
