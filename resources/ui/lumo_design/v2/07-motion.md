# 07 Motion

> **Timeless Over Trend.** 动效克制，不抢戏。禁用 Bounce / Spring / Zoom / Rotation。

LDL 的动效哲学：**动效是为状态变化服务的，不是为炫技。** 所有动效必须安静、快速、可预测。

---

## 1. 时长阶梯

仅允许三档时长：

| Token | 值 | 场景 |
|---|---|---|
| `--duration-150` | 150ms | **快速**：旋钮指针、Hover 图标显隐、opacity 变化 |
| `--duration-200` | 200ms | **默认**：颜色、背景、边框过渡（最常用） |
| `--duration-250` | 250ms | **慢速**：Tab 切换、视图切换、主题切换 |

### 选择规则

- 默认用 200ms
- 纯 opacity / transform 用 150ms（更跟手）
- 大面积 / 状态切换用 250ms

---

## 2. 规范曲线

**唯一允许的曲线**：

| Token | 值 | 说明 |
|---|---|---|
| `--ease-smooth` | `cubic-bezier(0.4, 0, 0.2, 1)` | Material 标准 ease-in-out，感官中性 |

```css
.transition-smooth {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}
.transition-colors-smooth {
  transition: color 0.2s ease-out, background-color 0.2s ease-out, border-color 0.2s ease-out;
}
```

> **v1.0 修正**：v1.0 文档写 "Ease-Out"，但代码实际用 `cubic-bezier(0.4,0,0.2,1)`（ease-in-out）。v2.0 以代码为准，统一为此曲线。颜色过渡用 `ease-out`（更自然）。

---

## 3. 禁用动效

| 禁用 | 原因 |
|---|---|
| Bounce | 不安静，抢戏 |
| Spring | 物理感过强，不符合 Warm Industrial 克制 |
| Zoom | 缩放引起布局抖动 |
| Rotation | 旋转动画（Loader2 spin 是唯一例外） |
| Parallax | 视差滚动，过度装饰 |
| Stagger | 列表项依次出现，延迟感差 |

### 唯一允许的 keyframe 动画

| 动画 | 用途 | 实现 |
|---|---|---|
| `animate-spin` | Loader2 加载旋转 | Tailwind 内置 |

```html
<Loader2 class="w-5 h-5 animate-spin text-brand-orange" />
```

---

## 4. 场景映射

### 4.1 Hover 状态（200ms）

```html
<button class="transition-colors-smooth hover:bg-bg-hover">
```

- 颜色 / 背景过渡：200ms ease-out
- 所有 Hover 用 `.transition-colors-smooth`（仅过渡颜色，避免位置属性过渡）

### 4.2 Selected 状态（200ms）

```html
<a class="transition-colors-smooth bg-list-selected">
```

- 同 Hover，200ms 颜色过渡

### 4.3 图标显隐（150ms）

```html
<div class="opacity-0 group-hover:opacity-100 transition-opacity" style="transition-duration: 150ms;">
```

- Song Row 的 More 按钮、未收藏 Heart：Hover 时 opacity 0 → 60/100
- 150ms，跟手快

### 4.4 旋钮指针（150ms）

```html
<div class="transition-transform" style="transition-duration: 150ms; transition-timing-function: cubic-bezier(0.4,0,0.2,1);">
```

- 拖拽时禁用过渡（`transition-none`），释放后 150ms 归位
- 见 `BottomPlayer.vue:282-284`

### 4.5 Tab 切换（250ms）

- Tab 激活下划线出现：250ms（未来可加 slide 动画）
- 当前实现为瞬间显示，未来优化

### 4.6 主题切换（200ms）

```css
body {
  transition: background-color 0.2s ease-out, color 0.2s ease-out;
}
```

- 亮/暗切换时 body 背景与文字色 200ms 过渡
- 不过渡所有元素（性能差），仅 body + 各区域背景

### 4.7 进度条（无过渡）

```html
<div class="bg-brand-orange" :style="{ width: progressPercent + '%' }"></div>
```

- 进度条 width 跟随播放进度实时变化，**不加 transition**（每秒更新会卡）
- Hover 时拖拽点 opacity 150ms 显隐

---

## 5. transition 工具类

LDL 在 `style.css` 定义两个工具类，组件中优先使用：

| 类 | 用途 | 定义 |
|---|---|---|
| `.transition-smooth` | 通用过渡（all） | `transition: all 0.2s cubic-bezier(0.4,0,0.2,1)` |
| `.transition-colors-smooth` | 仅颜色过渡 | `transition: color/background/border 0.2s ease-out` |

```html
<!-- ✅ 优先用工具类 -->
<button class="transition-colors-smooth hover:bg-bg-hover">

<!-- ❌ 不内联重复定义 -->
<button class="transition-all duration-200 ease-in-out hover:bg-bg-hover">
```

---

## 6. prefers-reduced-motion

**必须支持**。尊重用户系统设置，禁用所有非必要动效：

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

### 例外（保留）

- `animate-spin`（Loader2）：可降速但保留，因为它是"正在加载"的唯一信号
- 进度条 width 实时变化：无 transition，不受影响

### 实现位置

在 `style.css` 末尾添加此媒体查询（token 对齐轮次落地）。

---

## 7. Do & Don't

### Do

- ✅ 默认 200ms，快速 150ms，慢速 250ms
- ✅ 用 `.transition-colors-smooth` 工具类
- ✅ 拖拽时 `transition-none`，释放后恢复
- ✅ 支持 `prefers-reduced-motion`

### Don't

- ❌ 用 Bounce / Spring / Zoom / Rotation
- ❌ 动效超过 250ms（太慢）
- ❌ 进度条 width 加 transition
- ❌ 用动效替代状态指示（应靠颜色/图标）
- ❌ 列表项 stagger 依次出现

---

*End of Motion.*
