# 06 Elevation

> **Divider Over Card.** 默认无阴影，用 1px Divider 分层。

LDL 是**扁平 + Divider 驱动**的设计系统。深度感来自色温微差与 Divider，不来自阴影。

---

## 1. 核心原则

```
Default:  No Shadow
Depth:    Canvas / Content 色温微差 + 1px Divider
Exception: 仅 4 个受控例外允许阴影
```

**禁用**：

- Card 阴影
- Glassmorphism（毛玻璃 + 模糊 + 阴影）
- Neumorphism（凸起 / 凹陷阴影）
- 大面积投影作为分层手段

---

## 2. 深度策略

LUMO 的深度由三层叠加构成，**均无阴影**：

### 层 1：色温微差

```
Canvas   #F7F5F1（Sidebar/TopBar/Playback/Inspector）
Content  #FBFAF7（内容区，略亮略暖）
```

Inspector 与 Content 的微弱色差在视觉上产生"Content 是被托起的纸面"的层次感。

### 层 2：1px Divider

四条一级 Divider（详见 [02 Spatial](02-spatial.md) §3）明确划分区域边界。

### 层 3：交互态背景

Hover / Selected / Active 通过背景色变化表达"被选中"的状态，无需阴影。

---

## 3. 阴影例外（受控）

以下 **4 个场景**允许阴影，使用专用 token，**不允许自创阴影值**：

### 3.1 AlbumCard Hover 播放浮层

| 项 | 值 |
|---|---|
| Token | `--shadow-overlay` |
| 亮色 | `0 4px 12px rgba(0,0,0,0.15)` |
| 暗色 | `0 4px 12px rgba(0,0,0,0.4)` |
| 用途 | 网格封面 Hover 时浮现的圆形播放按钮 |

```html
<div class="w-10 h-10 rounded-full bg-brand-orange text-white shadow-[0_4px_12px_rgba(0,0,0,0.15)]">
  <Play class="w-4 h-4 fill-current" />
</div>
```

**理由**：浮层叠在封面之上，需要深度提示让用户感知"可点击"。是 LDL 中极少数的 Skeuomorphic 例外。

### 3.2 音量旋钮内凹质感

| 项 | 值 |
|---|---|
| Token | `--shadow-knob-inset` |
| 亮色 | `inset 0 1px 2px rgba(0,0,0,0.06)` |
| 暗色 | `inset 0 1px 2px rgba(0,0,0,0.3)` |
| 用途 | Playback Bar 物理音量旋钮的内凹质感 |

```html
<div class="rounded-full bg-bg-content border border-border-solid shadow-[inset_0_1px_2px_rgba(0,0,0,0.06)]"></div>
```

**理由**：旋钮是 LDL 唯一的物理 Skeuomorphic 控件，inset 阴影模拟实体凹陷。

### 3.3 Modal / Dialog（未来）

| 项 | 值 |
|---|---|
| Token | `--shadow-modal`（待注册） |
| 用途 | 模态对话框 |

### 3.4 Toast（未来）

| 项 | 值 |
|---|---|
| Token | `--shadow-toast`（待注册） |
| 用途 | 顶部/底部短暂提示 |

---

## 4. z-index 阶梯

LDL 的 z-index 严格分层，**不允许中间值**：

| Token | 值 | 用途 |
|---|---|---|
| `--z-base` | 0 | 默认所有内容 |
| `--z-sticky` | 10 | sticky 表头、Inspector Tab 下划线 |
| `--z-sidebar` | 20 | Sidebar（保证不被 Content 覆盖） |
| `--z-topbar` | 30 | TopBar（窗口控制始终可点击） |
| `--z-overlay` | 40 | Hover 浮层（AlbumCard 播放按钮） |
| `--z-modal` | 50 | Modal / Dialog（未来） |
| `--z-toast` | 60 | Toast（未来，最高） |

### 当前代码用法

```html
<!-- 表头 sticky -->
<div class="sticky top-0 bg-bg-content z-10">...</div>

<!-- Inspector Tab 下划线 -->
<div class="absolute bottom-[-1px] z-10 bg-brand-orange h-[2px]"></div>
```

### 规则

- 同层内用 DOM 顺序，不手动加 z-index
- 跨层用 token 值，不自创数字
- Modal / Toast 出现时需覆盖所有其他层

---

## 5. 边框 vs 阴影

LDL 用**边框**替代阴影定义控件边界：

| 控件 | 处理 |
|---|---|
| 输入框 | `border border-border-color`，focus 时 `border-brand-orange/50` |
| Secondary 按钮 | `border border-border-solid` |
| 视图切换容器 | `border border-border-color` |
| 旋钮主体 | `border border-border-solid` + inset 阴影（例外） |
| 进度条底 | 无边框，纯背景色 `bg-border-solid` |

**规则**：需要边界感时优先 `1px border`，不优先 `box-shadow`。

---

## 6. 暗色下的调整

暗色模式下，阴影更深（透明度增大），因为深色背景上的浅阴影不可见：

| Token | 亮色 | 暗色 |
|---|---|---|
| `--shadow-overlay` | `rgba(0,0,0,0.15)` | `rgba(0,0,0,0.4)` |
| `--shadow-knob-inset` | `rgba(0,0,0,0.06)` | `rgba(0,0,0,0.3)` |

**规则**：暗色阴影透明度约为亮色的 2.5 倍，确保可见。

---

## 7. Do & Don't

### Do

- ✅ 用 Canvas / Content 色温微差 + Divider 分层
- ✅ 阴影仅用于 4 个受控例外
- ✅ 阴影值来自 token，不自创
- ✅ z-index 严格用阶梯值

### Don't

- ❌ 给 Card / 列表行 / 按钮加阴影
- ❌ 用 Glassmorphism / Neumorphism
- ❌ 自创 z-index 中间值（如 `z-[15]` `z-[45]`）
- ❌ 用阴影替代边框或 Divider

---

*End of Elevation.*
