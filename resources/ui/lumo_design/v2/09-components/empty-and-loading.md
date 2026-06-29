# Empty State & Loading

> 空态、加载态、错误态。专业产品的分水岭。

---

## 1. Overview

LDL 的状态反馈遵循**安静原则**：不抢戏，但清晰传达"发生了什么 / 该做什么"。三种核心状态：

| 状态 | 用途 | 视觉 |
|---|---|---|
| Empty | 无数据 | 大图标 + 短文案 |
| Loading | 加载中 | Spinner + 短文案 |
| Error | 出错 | 短文案（未来加图标） |

---

## 2. Empty State

### Anatomy

```
              [图标 32px / 40px]
              没有找到歌曲
              （可选副文案）
```

### 参考代码

```html
<div class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted">
  <Music class="w-8 h-8 text-text-disabled" aria-hidden="true" />
  <span class="text-[12px]">没有找到歌曲</span>
</div>
```

### 规格

| 项 | 值 |
|---|---|
| 容器 | `flex flex-col items-center justify-center` |
| 垂直留白 | `py-20` (80px) |
| 元素间距 | `gap-3` (12px) |
| 图标尺寸 | `w-8 h-8` (32px) 或 `w-10 h-10` (40px) |
| 图标颜色 | `text-text-disabled` |
| 文案字号 | 12px |
| 文案颜色 | `text-text-muted` |

### 图标选择

| 场景 | 图标 |
|---|---|
| 歌曲空 | `Music` |
| 专辑空 | `Disc3` |
| 队列空 | `ListMusic` |
| 未在播放 | `Disc3` |
| 通用空 | `LayoutGrid` |

### 文案规范

- 简短：4-8 字
- 不指责用户：用"没有找到"而非"你还没有"
- 不加感叹号
- 可选副文案：`text-[11px] text-text-muted/70`（更淡）

```html
<div class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted px-8">
  <LayoutGrid class="w-8 h-8 text-text-disabled" />
  <p class="text-[13px]">{{ pageTitle }}视图</p>
  <p class="text-[11px] text-text-muted/70">该页面的视图将在后续迭代中接入。</p>
</div>
```

---

## 3. Loading State

### 3.1 首次加载（全屏占位）

```html
<div class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted" role="status" aria-live="polite">
  <Loader2 class="w-5 h-5 animate-spin text-brand-orange" aria-hidden="true" />
  <span class="text-[12px]">加载中…</span>
</div>
```

| 项 | 值 |
|---|---|
| Spinner | `Loader2` + `animate-spin` |
| Spinner 尺寸 | `w-5 h-5` (20px) |
| Spinner 颜色 | `text-brand-orange`（加载是动态状态，Accent 合法） |
| 文案 | "加载中…" / "加载专辑…" / "加载更多…" |
| 文案字号 | 12px Muted |

### 3.2 增量加载（列表底部）

```html
<div v-if="isLoading && tracks.length > 0" class="flex items-center justify-center py-4 text-text-muted">
  <Loader2 class="w-3.5 h-3.5 animate-spin mr-2" aria-hidden="true" />
  <span class="text-[11px]">加载更多…</span>
</div>
```

- Spinner 更小：`w-3.5 h-3.5` (14px)
- 文案 11px
- `py-4` (16px) 上下留白
- **不加 Accent**（增量加载是常态，不抢戏）

### 3.3 加载与 Accent

首次加载用 `text-brand-orange` Spinner，增量加载用默认 Muted。原因：

- 首次加载是"页面入场"，Accent Spinner是唯一动态焦点
- 增量加载是"已有内容基础上的补充"，不应抢已有内容

---

## 4. Error State

### 当前实现

```html
<div class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted" role="alert">
  <span class="text-[12px]">加载失败，请稍后重试</span>
</div>
```

### 未来增强（v2.1）

加入 `--status-error` 图标 + 重试按钮：

```html
<div class="flex flex-col items-center justify-center py-20 gap-3 text-text-muted" role="alert">
  <AlertCircle class="w-8 h-8 text-status-error" aria-hidden="true" />
  <span class="text-[12px]">加载失败，请稍后重试</span>
  <button class="text-[12px] text-brand-orange hover:underline">重试</button>
</div>
```

- 图标用 `--status-error`（不抢 Accent，状态色专用）
- 重试按钮用 Ghost + Accent 文字

---

## 5. 无更多内容

```html
<div v-if="!hasMoreAlbums && albums.length > 0" class="flex items-center justify-center py-6 text-text-muted">
  <span class="text-[11px]">已显示全部 {{ totalCount.toLocaleString() }} 张专辑</span>
</div>
```

- 11px Muted
- `py-6` (24px) 留白
- 不加图标，纯文案

---

## 6. 占位态（未实现视图）

```html
<div class="flex-1 flex flex-col items-center justify-center gap-3 text-text-muted px-8">
  <LayoutGrid class="w-8 h-8 text-text-disabled" aria-hidden="true" />
  <p class="text-[13px]">{{ pageTitle }}视图</p>
  <p class="text-[11px] text-text-muted/70">该页面的视图将在后续迭代中接入。</p>
</div>
```

- 与 Empty State 同结构
- 副文案说明原因（"后续迭代接入"）

---

## 7. Tokens used

| Token | 用途 |
|---|---|
| `--text-disabled` | 空态图标 |
| `--text-muted` | 状态文案 |
| `--text-muted/70` | 副文案（更淡） |
| `--brand-orange` | 首次加载 Spinner |
| `--status-error` | 错误态图标（未来） |

---

## 8. Do & Don't

### Do

- ✅ 空态图标用 Disabled 色（不抢戏）
- ✅ 首次加载 Spinner 用 Accent，增量加载不用
- ✅ 文案简短（4-8 字）
- ✅ `role="status"` / `role="alert"` + `aria-live`

### Don't

- ❌ 空态图标用 Accent（应 Disabled）
- ❌ 错误态用 Accent 图标（应用 `--status-error`）
- ❌ 文案加感叹号
- ❌ 空态铺满全屏（应 `py-20` 居中区块）

---

*End of Empty State & Loading.*
