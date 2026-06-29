# 02 Spatial System

> **Whitespace Creates Rhythm.** 宁可空，也不要拥挤。

---

## 1. Design Canvas

| 项 | 值 | 说明 |
|---|---|---|
| 基准画布 | **2560 × 1600** | 设计稿基准，HiDPI 5K 档 |
| 推荐分辨率 | **1920+** | 主力用户群 |
| 最佳分辨率 | **2560** | 高保真预览档 |
| 最小支持 | **1280 × 720** | 笔记本入门档，Inspector 应可折叠 |

**缩放策略**：

- 应用使用 CSS px，不随系统 DPI 缩放变化（Tauri WebView 默认行为）
- 1280px 宽度下：Sidebar 240 + Inspector 360 = 600px 已占用，Content 仅剩 680px，**必须支持 Inspector 折叠**
- 1920px+：三栏完整显示
- 2560px+：Content 宽敞，Album Grid 显示 6+ 列

---

## 2. Window Layout

应用采用**五区固定工作台**（Persistent Workspace），所有一级区域固定，避免频繁切换页面。

```
┌────────────┬───────────────────────────────────────────────┬──────────────────────┐
│            │                                               │                      │
│  Sidebar   │                  Top Bar                      │                      │
│  240px     ├───────────────────────────────────────────────┤  Inspector 360px     │
│            │                                               │  (可折叠)            │
│            │                Content Area                   │                      │
│            │                (flex-1)                       │                      │
├────────────┴───────────────────────────────────────────────┴──────────────────────┤
│                         Playback Bar  110px                                        │
└────────────────────────────────────────────────────────────────────────────────────┘
```

### 区域尺寸

| 区域 | 尺寸 | 占比(2560) | 性质 |
|---|---|---|---|
| **Sidebar** | 240 × full | 10% | 固定宽度，不可调 |
| **TopBar** | full × 60 | — | 固定高度，仅覆盖 Content + Inspector |
| **Content** | flex-1 × full | 72% | 自动伸缩 |
| **Inspector** | 360 × full | 18% | 固定宽度，**可折叠** |
| **Playback** | full × 110 | — | 固定高度 |

> TopBar **不覆盖** Sidebar——Sidebar 是独立纵向区域，从顶到底贯通。

---

## 3. 四条一级 Divider

整个界面**只有四条一级 Divider**，全部 `1px` + `--border-color`：

| 编号 | 位置 | 方向 |
|---|---|---|
| A | Sidebar ↔ Content/TopBar | 纵向 |
| B | TopBar ↔ Content | 横向 |
| C | Content ↔ Inspector | 纵向（Inspector 折叠时消失） |
| D | Workspace ↔ Playback | 横向 |

```html
<!-- App.vue 实现 -->
<div class="w-px h-full bg-border-color shrink-0"></div>   <!-- A -->
<div class="h-px w-full bg-border-color shrink-0"></div>   <!-- B -->
<div class="w-px h-full bg-border-color shrink-0" v-if="inspectorVisible"></div>  <!-- C -->
<div class="h-px w-full bg-border-color shrink-0"></div>   <!-- D -->
```

**禁止**：Card 边框作为区域分隔、用阴影分层、用背景色差替代 Divider（Inspector 与 Content 的微弱色差是辅助，不是主要分隔手段）。

### 二级 Divider（可选）

区域内部允许更细的分隔，使用 `--border-color`，仍为 1px：

- Inspector Tab 底部分隔
- 表头与列表之间 `border-b border-border-color`
- Footer Status 顶部 `border-t border-border-color`
- 专辑详情头部与列表之间 `h-px bg-border-color mx-8`

---

## 4. Grid 模数

LUMO 采用**三套并行模数**，各司其职，不强行统一：

### 4.1 间距 / 布局 — 8pt Grid

```
2 · 4 · 8 · 12 · 16 · 20 · 24 · 32 · 40 · 48 · 64 · 80 · 96
```

所有 padding、margin、gap、区域间距必须来自此阶梯。

**禁止**：`17px` `27px` `53px` 等随机值。

### 4.2 字号 — 4pt 模数

```
9 · 10 · 11 · 12 · 13 · 14 · 15 · 18 · 22 · 28 · 32
```

字号不强制 8pt，因为 12/14/16 是桌面正文核心档位，强行 8pt 会跳到 16/24 过稀。

> 详见 [04 Typography](04-typography.md)。

### 4.3 圆角 — 独立模数

```
6 · 8 · 10 · 16 · full
```

圆角与控件语义绑定，不与间距共享模数。

| 圆角 | 语义 |
|---|---|
| 6 | 小控件（Sidebar Item、视图切换内按钮） |
| 8 | 中控件（IconButton、输入框、视图切换容器） |
| 10 | 封面（Album Card、Now Playing、详情封面） |
| 16 | 大容器（未来 Modal） |
| full | 胶囊（Play All）、圆形（Transport Play） |

---

## 5. 各区域内部结构

### 5.1 Sidebar（240px）

```
Logo (pt-8 pb-6)
    ↓
Library 分组
    ├ 全部歌曲
    ├ 专辑
    ├ 艺术家
    ├ 作曲家
    ├ 文件夹
    ├ 最近播放
    └ 播放列表
    ↓
Playlists 分组
    ├ 歌单 1
    ├ 歌单 2
    └ + 新建
    ↓
(底部留白)
```

**职责**：应用导航 + 歌单列表。
**禁止**：搜索、设置、皮肤、播放控制（这些在 TopBar / Content Toolbar / Playback）。

> 详见 [09-components/sidebar-item.md](09-components/sidebar-item.md)。

### 5.2 TopBar（60px）

```
                                    [夜间模式] [右栏] [更多] | [最小] [最大] [关闭]
```

**职责**：应用级操作（主题切换、面板切换、更多、窗口控制）。
**不承担**页面功能（搜索、排序、筛选在 Content Toolbar）。
**拖拽**：整个 TopBar 是窗口拖拽区域（`data-tauri-drag-region`），按钮区 `pointer-events-auto`。

### 5.3 Content Area（flex-1）

统一结构：

```
Page Title (32px, px-8 pt-8)
    ↓
Metadata (text-12, mono)
    ↓
Toolbar (搜索 / 排序 / 筛选 / 视图切换, py-3)
    ↓
List / Grid (flex-1, overflow-y-auto)
    ↓
Footer Status (text-11, mono, border-t)
```

**详情页**（如 AlbumDetail）独占 Content，不显示 Page Title / Toolbar，自带返回按钮 + 头部 + 列表。

### 5.4 Inspector（360px，可折叠）

两个 Tab：

```
[正在播放]  [播放列表]    ← Tab，Accent 下划线
    ↓
Now Playing 模式:
    封面 (aspect-square, radius-10)
    ↓
    曲名 (text-18 bold) + 收藏/更多
    ↓
    艺术家 / 专辑 / 元数据(mono uppercase)
    ↓
    Lyrics (scroll, 当前行 Accent)

Queue 模式:
    队列列表 (序号 + 标题/艺术家 + 时长)
```

**折叠**：< 1280px 或用户手动隐藏时，Inspector 整体 `v-if` 移除，Divider C 同步消失。

### 5.5 Playback Bar（110px）

```
[封面 56×56 + 曲名/波形]      [Transport + 进度条]      [旋钮 + Output]
    280px                       flex-1 居中                flex-shrink-0
```

> 详见 [09-components/playback-bar.md](09-components/playback-bar.md)。

---

## 6. 响应式断点

LUMO 是**桌面优先**应用，断点用于决定 Inspector 是否默认可见、Grid 列数。

| 断点 | Tailwind | 宽度 | 行为 |
|---|---|---|---|
| xs | — | < 1280 | Inspector 默认隐藏；Album Grid 4 列 |
| md | `md` | ≥ 768 | Song Row 显示艺术家列 |
| lg | `lg` | ≥ 1024 | Song Row 显示专辑列 |
| xl | `xl` | ≥ 1280 | Inspector 默认显示；Album Grid 5 列 |
| 2xl | `2xl` | ≥ 1536 | Album Grid 6 列 |

**当前代码用法**（`MainContent.vue`）：

```html
<div class="flex-[1.5] min-w-0 hidden md:block">艺术家</div>   <!-- ≥768 显示 -->
<div class="flex-[1.5] min-w-0 hidden lg:block">专辑</div>     <!-- ≥1024 显示 -->
<div class="w-[56px] hidden xl:block">时长</div>               <!-- ≥1280 显示 -->
```

---

## 7. 内边距规范

| 区域 | 内边距 | 说明 |
|---|---|---|
| Sidebar | `px-5` (20px) 横向，`pt-8 pb-4` 纵向 | Logo 区 `px-8 pt-8 pb-6` |
| TopBar | `px-4` (16px) | — |
| Content Header | `px-8 pt-8` (32px) | Page Title 区 |
| Content Toolbar | `px-8 py-3` | — |
| Content List | `px-8` | 滚动区 |
| Inspector | `px-6` (24px) 或 `px-4` (16px) | Now Playing 用 24，Queue 用 16 |
| Playback | `px-6` (24px) | — |

**规则**：Content 区统一 `px-8`（32px）作为水平基准，Inspector 略窄。

---

## 8. Do & Don't

### Do

- ✅ 区域之间只用 1px Divider 分隔
- ✅ Content 列表用 `px-8` 统一水平边距
- ✅ Inspector 在窄屏下折叠而非挤压 Content
- ✅ 所有间距取自 8pt 阶梯

### Don't

- ❌ 用 Card / 阴影 / 大色块划分区域
- ❌ 在 Sidebar 放搜索框或播放控制
- ❌ 让 TopBar 承担页面级功能
- ❌ 使用 17px / 27px 等非阶梯间距
- ❌ Inspector 折叠后仍保留 Divider C

---

## 9. Tauri 窗口配置规范

LUMO 使用 Tauri 自定义窗口（`decorations: false`），窗口控制由前端 TopBar 实现。

### 9.1 配置（`tauri.conf.json`）

```json
{
  "app": {
    "windows": [{
      "title": "Lumo Player",
      "width": 1600,
      "height": 900,
      "minWidth": 1280,
      "minHeight": 720,
      "center": true,
      "resizable": true,
      "decorations": false
    }]
  }
}
```

### 9.2 各宽度下的 Content 实际宽度

| 窗口宽度 | Content 宽（Inspector 显示） | Content 宽（Inspector 隐藏） |
|---|---|---|
| 1280（最小） | 680px | 1040px |
| 1600（默认） | 1000px | 1360px |
| 1920 | 1320px | 1680px |
| 2560 | 1960px | 2320px |

### 9.3 最小窗口 1280×720 的约束

- 1280 时 Inspector 默认隐藏（`hidden xl:flex`）
- Song Row 显示 # + 标题 + 艺术家（sm=640）
- AlbumGrid 3 列（auto-fill minmax 180px）
- Playback 进度条 `max-w-2xl` 在 680px 中栏中比例合理
- 720 高度下：60(TopBar) + 60(Inspector Tab) + 110(Playback) = 230，Content 可用 490px

---

## 10. 平台差异（v3.0 预留）

| 平台 | 窗口控制位置 | 特殊处理 |
|---|---|---|
| **Windows** | 右上（当前实现） | 关闭按钮 Hover `#E81123` |
| **macOS** | 左上系统红黄绿灯（未来） | `decorations: true` + 自定义 titlebar |
| **Linux** | 右上（同 Windows） | 各 DE 差异，以 GNOME/KDE 为基准 |

当前阶段以 **Windows 优先**，macOS/Linux 窗口控制差异推迟到 v3.0 Multi-platform 时处理。代码层面保留平台检测接口。

---

## 11. Content 容器查询断点

当前 Song Row 断点基于**视口宽度**（Tailwind `sm/md/lg/xl`），但 Song Row 实际可用宽度 = 视口 - Sidebar 240 - Inspector 360。

### 当前策略

| 列 | Tailwind 断点 | 视口阈值 | Content 显示条件 |
|---|---|---|---|
| # | 始终 | — | 始终 |
| 收藏 | 始终 | — | 始终 |
| 标题 | 始终 | — | 始终 |
| 艺术家 | `sm` | ≥ 640 | Content ≥ 400（Inspector 隐藏时） |
| 专辑 | `md` | ≥ 768 | Content ≥ 528 |
| 时长 | `lg` | ≥ 1024 | Content ≥ 784 |
| 格式 | `lg` | ≥ 1024 | Content ≥ 784 |

### 未来方向（v2.1）

迁移到 **CSS Container Queries**：

```css
@container (min-width: 500px) {
  .artist-col { display: block; }
}
@container (min-width: 700px) {
  .album-col { display: block; }
}
@container (min-width: 900px) {
  .duration-col { display: block; }
  .format-col { display: block; }
}
```

当前 Tailwind v4 尚不支持 `@container` 直接绑定 utility class，暂用视口断点近似。

---

## 12. Inspector 封面响应式

Inspector 封面 `aspect-square` 自适应宽度（312px @ 360px），但在小高度屏幕下会压榨 Lyrics 空间。

### 规则

```html
<div class="w-full aspect-square max-h-[40vh] bg-bg-hover rounded-[10px] mb-4 overflow-hidden flex-shrink-0">
```

| 屏幕高度 | 可用 Content 高度 | 封面高度（max-h 前） | 封面高度（max-h 后） | Lyrics 可用 |
|---|---|---|---|---|
| 720（最小） | 490px | 312px (64%) | **288px** (max 40vh) | ~202px |
| 900（默认） | 670px | 312px (47%) | 312px (不受限) | ~358px |
| 1080 | 850px | 312px (37%) | 312px | ~538px |

- `max-h-[40vh]` 确保小屏幕下封面不超过视口 40%
- 大屏幕下无影响（312px < 40vh）

---

## 13. AlbumGrid 自适应列公式

```css
grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
```

| 窗口宽度 | Content 宽（Inspector 隐藏） | 列数 | 封面宽 |
|---|---|---|---|
| 1280 | 1040 | 4-5 | 180-200px |
| 1600（默认） | 1360 | 6 | 200px |
| 1920 | 1680 | 7-8 | 200-210px |
| 2560 | 2320 | 10-11 | 200-210px |

### 规则

- `minmax(180px, 1fr)` 确保封面宽度 180-210px 一致
- 避免固定 5 列在宽屏下标宽过大、窄屏下过小
- 网格间距 `gap-6` (24px) 不随列数变化

---

## 14. Playback 进度条宽度

```html
<div class="w-full flex items-center gap-3 max-w-2xl">
```

| 项 | 值 |
|---|---|
| 最大宽度 | `max-w-2xl` = 42rem = 672px |
| 最小宽度 | 弹性（中栏 flex-1） |
| 适用屏宽 | 1280-2560 |

`max-w-2xl` 在 1280 屏上约占中栏 70%，在 2560 屏上约占 45%，在可用性和留白之间取得平衡。禁用 `max-w-md`（448px，大屏过窄）和 `max-w-full`（1280 屏过长）。

---

*End of Spatial System.*
