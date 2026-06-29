# Tab

> 下划线 Accent Tab。Inspector 顶部"正在播放 / 播放列表"切换。

---

## 1. Overview

LDL 的 Tab 是**极简下划线式**：无背景，仅靠 Accent 2px 下划线 + 文字色变化表达激活态。

---

## 2. Anatomy

```
┌──────────────────────────────────────────────────────────┐
│                                                          │  h-60px（与 TopBar 对齐）
│              正在播放    播放列表                          │
│              ────────                                     │  ← Accent 2px 下划线
│ ──────────────────────────────────────────────────────── │  ← 底部 1px Divider
└──────────────────────────────────────────────────────────┘
```

---

## 3. Sizes

| 项 | 值 |
|---|---|
| 容器高度 | 60px（与 TopBar 同高） |
| Tab 文字 | 13px |
| 下划线高度 | 2px |
| 下划线位置 | 容器底部 `-1px`（压在 Divider 上） |
| Tab 间距 | `gap-10` (40px) |

---

## 4. States

| 状态 | 文字色 | 字重 | 下划线 |
|---|---|---|---|
| Active | `text-brand-orange` | Medium (500) | 2px `bg-brand-orange` |
| Inactive | `text-text-muted` | Regular | 无 |
| Inactive Hover | `text-text-primary` | Regular | 无 |

---

## 5. 参考代码

```html
<div class="relative h-[60px] flex-shrink-0" data-tauri-drag-region>
  <!-- 底部 1px Divider -->
  <div class="absolute bottom-0 left-0 w-full h-px bg-border-color"></div>

  <!-- Tab 组（居中） -->
  <div class="flex items-end justify-center gap-10 h-full">
    <button
      class="relative pb-3 text-[13px] transition-colors-smooth"
      :class="tab === 'now-playing' ? 'font-medium text-brand-orange' : 'text-text-muted hover:text-text-primary'"
      @click="tab = 'now-playing'"
      :aria-selected="tab === 'now-playing'"
      role="tab"
    >
      正在播放
      <!-- Accent 下划线（Active 时显示） -->
      <div v-if="tab === 'now-playing'" class="absolute bottom-[-1px] left-0 w-full h-[2px] bg-brand-orange z-10"></div>
    </button>
    <button
      class="relative pb-3 text-[13px] transition-colors-smooth"
      :class="tab === 'queue' ? 'font-medium text-brand-orange' : 'text-text-muted hover:text-text-primary'"
      @click="tab = 'queue'"
      :aria-selected="tab === 'queue'"
      role="tab"
    >
      播放列表
      <div v-if="tab === 'queue'" class="absolute bottom-[-1px] left-0 w-full h-[2px] bg-brand-orange z-10"></div>
    </button>
  </div>
</div>
```

### 关键细节

1. **下划线 `bottom-[-1px]`**：压在底部 Divider 上，视觉合并为一条粗线
2. **下划线 `z-10`**：高于 Divider（`--z-sticky`），确保 Accent 覆盖
3. **`pb-3` (12px)**：文字底部留白，与下划线间距
4. **`justify-center`**：Tab 组居中（Inspector 360px 宽）
5. **`gap-10` (40px)**：两个 Tab 间距

---

## 6. Accent 使用

Tab 是 Accent 的**合法场景**（Active 状态指示）：

- Active 文字 `text-brand-orange`
- Active 下划线 `bg-brand-orange`

**与 One Accent Rule 的关系**：Inspector 中 Tab Accent 与 Now Playing 曲名的 Accent **不同时出现**——Tab Active 时显示的是面板，曲名 Accent 只在 Now Playing Tab 内。若 Tab 切到 Queue，Now Playing 曲名不可见，Accent 焦点转移到 Queue 当前播放项。

---

## 7. 未来：二级 Tab（详情页）

专辑/艺术家详情页内的"全部歌曲 / 全部专辑"二级 Tab（预留）：

```
全部歌曲   全部专辑
─────────
```

- 与 Inspector Tab 同样式
- 容器高度可降至 40px
- 位于详情页头部下方

---

## 8. Tokens used

| Token | 用途 |
|---|---|
| `--brand-orange` | Active 文字 + 下划线 |
| `--text-muted` | Inactive 文字 |
| `--text-primary` | Inactive Hover 文字 |
| `--border-color` | 底部 Divider |

---

## 9. Do & Don't

### Do

- ✅ 下划线 `bottom-[-1px]` 压在 Divider 上
- ✅ Active 用 Medium + Accent，不用 Bold
- ✅ Tab 组居中
- ✅ `role="tab"` + `aria-selected`

### Don't

- ❌ Active 用 Bold（应用 Medium）
- ❌ 给 Tab 加背景色（应无背景）
- ❌ 下划线用 1px（应 2px 突出）
- ❌ Inactive 用 `text-text-secondary`（应 Muted 更安静）

---

*End of Tab.*
