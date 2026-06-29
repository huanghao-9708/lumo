# Playback Bar

> 底部播放栏。LDL 最复杂的区域：进度条 / 确定性波形 / 物理音量旋钮 / Transport。

---

## 1. Overview

Playback Bar 是 110px 高的固定底部区域，分三栏：

```
┌──────────────────┬──────────────────────────────┬──────────────────────┐
│  封面 + 曲名 +    │     Transport + 进度条        │   旋钮 + Output      │
│  波形             │     (居中)                    │                      │
│  280px            │     flex-1                    │   flex-shrink-0      │
└──────────────────┴──────────────────────────────┴──────────────────────┘
```

---

## 2. 左栏：曲目信息 + 波形

### Anatomy

```
┌──────┐  曲名 13px Semibold
│封面   │  艺术家 · 专辑  11px Muted
│56×56 │  格式 9px Mono uppercase
└──────┘  ▁▂▃▅▇▅▃▂▁  确定性波形 16px 高
```

### 封面

```html
<div class="w-[56px] h-[56px] bg-bg-hover rounded-[6px] overflow-hidden flex-shrink-0 mr-3 flex items-center justify-center">
  <img v-if="coverSrc" :src="coverSrc" class="w-full h-full object-cover" alt="cover" />
  <Disc3 v-else class="w-5 h-5 text-text-disabled" aria-hidden="true" />
</div>
```

- 56×56，`rounded-[6px]`
- `bg-bg-hover` 加载底色
- 无封面 Disc3 占位

### 曲目信息

```html
<span class="text-[13px] font-semibold text-text-primary truncate leading-tight">{{ currentTrack.title }}</span>
<span class="text-[11px] text-text-muted truncate mt-0.5">{{ artist }} · {{ album }}</span>
<span class="text-[9px] text-text-muted font-mono mt-0.5 uppercase tracking-wider">{{ format }}</span>
```

| 元素 | 字号 | 字重 | 颜色 |
|---|---|---|---|
| 曲名 | 13px | Semibold | Primary |
| 艺术家·专辑 | 11px | Regular | Muted |
| 格式 | 9px | Mono Bold | Muted uppercase |

### 确定性波形

基于 track id 的固定种子生成，**不随播放实时变化**（性能优先）：

```html
<div class="flex items-end h-4 gap-[1px] mt-1.5">
  <div
    v-for="(h, i) in waveform"
    :key="i"
    class="w-[2px] rounded-t-sm transition-colors-smooth"
    :class="i < playedBarCount ? 'bg-brand-orange' : 'bg-text-muted/30'"
    :style="{ height: `${h * 100}%` }"
  ></div>
</div>
```

| 项 | 值 |
|---|---|
| 容器高度 | `h-4` (16px) |
| 条数 | 48 |
| 条宽 | 2px |
| 间距 | `gap-[1px]` |
| 已播放 | `bg-brand-orange`（Progress Accent） |
| 未播放 | `bg-text-muted/30`（30% 透明 Muted） |
| 生成 | 确定性 PRNG（seed = track id） |

> 波形是"装饰性进度指示"，已播放段用 Accent 是合法 Progress 用法。

### 未播放态

```html
<div class="flex flex-col justify-center min-w-0">
  <span class="text-[13px] text-text-muted">未在播放</span>
  <span class="text-[11px] text-text-disabled">选择一首歌曲开始</span>
</div>
```

---

## 3. 中栏：Transport + 进度条

### Transport 按钮组

```
[模式]  [⏮]  [▶]  [⏭]  [更多]
 16      18   48   18    16
```

```html
<div class="flex items-center gap-7 mb-2">
  <!-- 播放模式 -->
  <button :class="modeActive ? 'text-brand-orange' : 'text-text-muted hover:text-text-primary'" @click="cycleMode">
    <component :is="modeIcon" class="w-[16px] h-[16px]" />
  </button>
  <!-- 上一首 -->
  <button class="text-text-primary hover:text-brand-orange" @click="prevTrack">
    <SkipBack class="w-[18px] h-[18px] fill-current" />
  </button>
  <!-- Play/Pause（圆形 Primary 黑） -->
  <button class="w-[48px] h-[48px] rounded-full bg-text-primary text-bg-canvas flex items-center justify-center hover:opacity-90">
    <Pause v-if="isPlaying" class="w-[20px] h-[20px] fill-current" />
    <Play v-else class="w-[20px] h-[20px] fill-current ml-0.5" />
  </button>
  <!-- 下一首 -->
  <button class="text-text-primary hover:text-brand-orange" @click="nextTrack">
    <SkipForward class="w-[18px] h-[18px] fill-current" />
  </button>
  <!-- 更多 -->
  <button class="text-text-muted hover:text-text-primary">
    <ChevronDown class="w-[16px] h-[16px]" />
  </button>
</div>
```

| 按钮 | 尺寸 | Default | Hover | Active |
|---|---|---|---|---|
| 模式 | 16px | Muted | Primary | `text-brand-orange`（modeActive） |
| 上一首 | 18px fill | Primary | Accent | — |
| Play/Pause | 48px 圆形 | Primary 黑底 | `opacity-90` | — |
| 下一首 | 18px fill | Primary | Accent | — |
| 更多 | 16px | Muted | Primary | — |

- `gap-7` (28px) 按钮间距
- Play 按钮 `ml-0.5` 视觉居中（三角形光学中心偏左）

### 播放模式

```js
const modes = ['normal', 'repeat', 'repeat-one', 'shuffle'];
```

| 模式 | 图标 | 激活色 |
|---|---|---|
| normal | Repeat | Muted（未激活） |
| repeat | Repeat | Accent |
| repeat-one | Repeat1 | Accent |
| shuffle | Shuffle | Accent |

### 进度条

```html
<div class="w-full flex items-center gap-3 max-w-md">
  <span class="text-[10px] font-mono text-text-muted w-9 text-right tabular-nums">{{ currentTimeText }}</span>
  <div
    ref="progressRef"
    class="flex-1 h-[3px] bg-border-solid rounded-full relative group cursor-pointer"
    @mousedown="onProgressDown"
  >
    <!-- 已播放填充 -->
    <div class="absolute left-0 top-0 h-full bg-brand-orange rounded-full" :style="{ width: progressPercent + '%' }"></div>
    <!-- 拖拽点（Hover 显） -->
    <div
      class="absolute top-1/2 -translate-y-1/2 w-[10px] h-[10px] bg-brand-orange rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
      :style="{ left: progressPercent + '%', marginLeft: '-5px' }"
    ></div>
  </div>
  <span class="text-[10px] font-mono text-text-muted w-9 text-left tabular-nums">{{ totalTimeText }}</span>
</div>
```

| 项 | 值 |
|---|---|
| 轨道高度 | 3px |
| 轨道背景 | `bg-border-solid` |
| 已播放 | `bg-brand-orange`（Progress Accent） |
| 拖拽点 | 10×10 圆形 Accent，Hover 显 |
| 时间 | 10px mono `tabular-nums`，`w-9` 固定宽 |
| 格式 | `MM:SS`（`String.padStart(2,'0')`） |
| 宽度 | `max-w-md` (28rem) |

### 拖拽交互

```js
function seekFromEvent(clientX) {
  const rect = progressRef.value.getBoundingClientRect();
  const pct = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
  playerStore.seek(Math.floor(pct * playerStore.durationMs));
}
```

- `mousedown` 开始拖拽 + 立即 seek
- `mousemove`（window 监听）持续 seek
- `mouseup`（window 监听）结束

---

## 4. 右栏：音量旋钮 + Output

### 物理音量旋钮

LDL 唯一的 Skeuomorphic 控件，致敬 Braun / Teenage Engineering 工业美学。

#### Anatomy

```
        Volume
   ┌─────────────┐
   │  ╱╲╱╲╱╲╱╲   │  ← 11 个刻度环（-135° ~ +135°）
   │   ┌───┐    │
   │   │ ● │    │  ← 旋钮主体（bg-content + inset 阴影）
   │   └───┘    │
   │    ▌       │  ← Accent 指针
   └─────────────┘
    0   45   100
```

#### 刻度环

```html
<svg class="absolute inset-0 w-full h-full pointer-events-none" viewBox="0 0 52 52">
  <g
    v-for="i in KNOB_TICKS"
    :key="i"
    :transform="`rotate(${-135 + ((i - 1) / (KNOB_TICKS - 1)) * 270} 26 26)`"
  >
    <line
      x1="26" y1="3.5" x2="26" :y2="i === 1 || i === KNOB_TICKS ? 7 : 6"
      :stroke="(((i - 1) / (KNOB_TICKS - 1)) * 100) <= volume ? '#E28A23' : 'rgba(139,139,139,0.45)'"
      :stroke-width="i === 1 || i === KNOB_TICKS ? 1.5 : 1"
      stroke-linecap="round"
    />
  </g>
</svg>
```

| 项 | 值 |
|---|---|
| 刻度数 | 11（0~10） |
| 角度范围 | -135° ~ +135°（270°） |
| 已激活刻度 | `#E28A23`（Progress Accent） |
| 未激活刻度 | `rgba(139,139,139,0.45)`（Muted 45%） |
| 端点加粗 | 1.5px（首尾刻度） |
| 中间刻度 | 1px |

#### 旋钮主体

```html
<div class="absolute inset-[10px] rounded-full bg-bg-content border border-border-solid shadow-[inset_0_1px_2px_rgba(0,0,0,0.06)]"></div>
```

- 52×52 容器，主体 `inset-[10px]` → 32×32
- `bg-bg-content` + `border-border-solid`
- `--shadow-knob-inset`（受控阴影例外）

#### 指针

```html
<div
  class="absolute inset-0 transition-transform"
  style="transition-duration: 150ms; transition-timing-function: cubic-bezier(0.4,0,0.2,1);"
  :style="{ transform: `rotate(${knobAngle}deg)` }"
  :class="isDraggingKnob ? 'transition-none' : ''"
>
  <div class="absolute left-1/2 top-[5px] -translate-x-1/2 w-[2px] h-[7px] bg-brand-orange rounded-full"></div>
</div>
```

- 指针角度：`-135 + (volume/100) * 270`
- 150ms 过渡（拖拽时 `transition-none`）
- 2×7px Accent 竖条

#### 标签与数字

```html
<span class="text-[9px] font-bold text-text-primary uppercase tracking-widest mb-1.5 flex items-center gap-1">
  <component :is="volumeIcon" class="w-[11px] h-[11px]" aria-hidden="true" />
  Volume
</span>
<div class="flex justify-between w-14 mt-1 text-[8px] text-text-muted font-mono tabular-nums">
  <span>0</span>
  <span class="text-text-primary font-bold">{{ volume }}</span>
  <span>100</span>
</div>
```

#### 交互

- **拖拽**：`mousedown` 计算 pointer 相对中心角度 → 音量
- **滚轮**：`@wheel` 上 +5 / 下 -5
- **角度计算**：`atan2` → 归一化 → 限制 [-135°, 135°] → 百分比

```js
function setVolumeFromPointer(clientX, clientY) {
  const rect = knobRef.value.getBoundingClientRect();
  let deg = Math.atan2(clientY - cy, clientX - cx) * 180 / Math.PI;
  deg = deg + 90;  // 正上方=0
  deg = Math.max(-135, Math.min(135, deg));
  const pct = (deg + 135) / 270;
  playerStore.setVolume(Math.round(pct * 100));
}
```

### Output 选择器

```html
<div class="flex flex-col items-start">
  <span class="text-[9px] font-bold text-text-primary uppercase tracking-widest mb-1.5">Output</span>
  <button class="flex items-center gap-1 text-[12px] font-medium text-text-primary hover:text-brand-orange transition-colors-smooth">
    Built-in Output
    <ChevronDown class="w-3 h-3 text-text-muted" />
  </button>
</div>
```

- 标签 9px Bold uppercase
- 选择器 12px Medium，Hover Accent

---

## 5. Tokens used

| Token | 用途 |
|---|---|
| `--bg-hover` | 封面底色 |
| `--text-primary` | 曲名、Play 按钮、Skip Default |
| `--text-muted` | 副信息、时间、模式 Default |
| `--text-disabled` | 无封面占位 |
| `--brand-orange` | 进度条、波形已播放、旋钮刻度/指针、模式 Active、Skip Hover |
| `--border-solid` | 进度条轨道、旋钮边框 |
| `--bg-content` | 旋钮主体 |
| `--shadow-knob-inset` | 旋钮内凹（例外） |

---

## 6. Do & Don't

### Do

- ✅ Play 按钮用 Primary 黑（不用 Accent）
- ✅ 旋钮 inset 阴影用 token（受控例外）
- ✅ 进度条拖拽点 Hover 才显
- ✅ 旋钮支持拖拽 + 滚轮

### Don't

- ❌ Play 按钮用 Accent（违反 One Accent）
- ❌ 进度条 width 加 transition（每秒更新会卡）
- ❌ 旋钮用 box-shadow 外阴影（应用 inset）
- ❌ 波形用实时音频分析（性能差，用确定性种子）

---

*End of Playback Bar.*
