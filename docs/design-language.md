# Lumo 设计语言规范与实现文档 (LDL)

> **版本**: 2.1.1 (LDL v2.0)  
> **状态**: 正式规范与代码落地实现对齐  
> **单一事实源代码**: [style.css](file:///c:/Users/hao/RustroverProjects/lumo/src/style.css)  
> **上游设计文档库**: [resources/ui/lumo_design/v2/](file:///c:/Users/hao/RustroverProjects/lumo/resources/ui/lumo_design/v2/README.md)

---

## 1. 文档概述与设计哲学

### 1.1 核心设计定位
**Lumo（轻音）** 的设计语言体系被命名为 **LDL (LUMO Design Language)**。其核心设计哲学为：
> **Warm Industrial Minimalism（温暖工业极简主义）**  
> *核心口号: Quiet · Focused · Timeless（安静 · 专注 · 恒久）*  
> *产品愿景: Less Interface, More Music（更少界面侵扰，更多音乐本真）*

LDL 汲取了 20 世纪中叶瑞士国际平面风格（Swiss Style / International Typographic Style）与博朗（Braun）工业设计的实用主义克制精髓，拒绝当下流媒体客户端普遍存在的过度社交化、视觉喧嚣和流光渐变泛滥，致力于构建一个如实体高级音响和黑胶唱片唱机般历久弥新、专注本地曲库的沉浸音乐工作台。

```
┌─────────────────────────────────────────────────────────────────┐
│                     LDL 设计哲学三大支柱                        │
├───────────────────┬─────────────────────┬───────────────────────┤
│    Quiet (安静)   │   Focused (专注)    │   Timeless (恒久)     │
│  默认拒绝视觉杂音 │  音乐与元数据优先   │  经得起时间考验的美学 │
│  无投影、无大渐变 │  单强调色高信息密度 │  坚守暖白/暖黑/中性阶梯│
└───────────────────┴─────────────────────┴───────────────────────┘
```

### 1.2 LDL 设计宪法十大原则

1. **Content First（内容第一）**：专辑封面、曲目名称、艺术家与音频音质元数据是界面绝对的主角，所有控件均为次级支撑。
2. **Typography Before Decoration（排版先于装饰）**：依赖严谨的字体阶梯、字重与字距构建信息层级，严禁添加无关的边框装饰花纹或多余的浮层胶囊。
3. **Whitespace Creates Rhythm（留白构建节奏）**：采用严格的 8pt 空间模数，留白不仅是距离，更是界面呼吸与音乐节奏的延展。
4. **Divider Over Card（线胜于卡）**：大面积内容列表采用 1px 精细分割线切分层级，严禁使用大阴影、厚卡片把列表切割得四分五裂。
5. **One Accent Rule（单强调色法则）**：全系统日夜模式恒定仅允许唯一强调色暖橘（`#E28A23`），强调色仅用于标明“正在播放”、“激活选中”、“播放进度”与“聚焦”，严禁滥用于大面积背景。
6. **Quiet Controls（克制控件）**：按钮、图标平时保持低对比度静默（Muted），仅在鼠标悬停（Hover）或激活（Active）时优雅亮起。
7. **Mono for Numbers（数字强制等宽）**：所有播放进度（`03:42 / 05:18`）、采样率（`44.1 kHz`）、比特率（`320 kbps`）、文件大小与序号必须强制使用等宽字体（`tabular-nums`），杜绝数字变动跳动。
8. **Touch Target Rule（触控安全基线）**：桌面端保持紧凑高信息密度，移动端（视口 < 768px 或 Android 设备）触控目标必须大于等于 44px（行高 56px）。
9. **Code Follows Design（代码严格对齐设计）**：代码中的样式只消费 LDL Tokens，绝对清零硬编码魔数（Magic Numbers），样式变量与设计规范保持单向流同步。
10. **Accessible by Default（默认无障碍）**：全功能支持键盘 Tab 与快捷键导航，遵守 WCAG AA/AAA 对比度标准，严密支持系统级防眩晕减弱动效（`prefers-reduced-motion`）。

### 1.3 品牌标识体系 (Brand Identity)
LUMO 品牌标识由排版字标（Wordmark）构成，严禁使用复杂的拟物化图形或炫光彩标：
- **主标**：`LUMO`（20px Bold，字间距 `tracking-[0.15em]`）
- **副标**：`LOCAL MUSIC SYSTEM`（9px Mono，`text-text-muted`）
- **版本事实源**：`v2.1.1`（9px Mono，自动同步全局配置 [appInfo.ts](file:///c:/Users/hao/RustroverProjects/lumo/src/config/appInfo.ts)）

---

## 2. 空间布局系统 (Spatial System)

### 2.1 窗口与画布基准
- **基准设计画布**：2560 × 1600（视网膜级设计基线）
- **桌面默认窗口尺寸**：1200 × 720 逻辑像素（居中启动）
- **桌面最小约束窗口**：1024 × 640 逻辑像素（[tauri.conf.json](file:///c:/Users/hao/RustroverProjects/lumo/src-tauri/tauri.conf.json) 强制门禁，防止布局崩溃）
- **迷你播放栏模式规格**：560 × 96 逻辑像素（极简悬浮小窗）
- **移动端视口分界断点**：`< 768px`（触发 [MobileLayout.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/mobile/MobileLayout.vue)）

### 2.2 桌面五区固定工作台架构 (Five-Region Workspace)

桌面端在 [App.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/App.vue) 中采用精密的五区工作台布局：

```
┌────────────────────────────────────────────────────────────────────────┐
│  Region 01        │ Region 02: TopBar (高 60px)                        │
│  SidebarLeft      ├────────────────────────────────────────────────────┤
│  左侧导航栏       │ Region 03: MainContent         │ Region 04         │
│  (固定宽 220px)   │ 主内容区                       │ SidebarRight      │
│                   │ (自适应弹性填充 flex-1)        │ Inspector 检查器  │
│                   │                                │ (浮层宽 320px)    │
├───────────────────┴────────────────────────────────┴───────────────────┤
│  Region 05: BottomPlayer 底栏播放器 (固定高 92px)                      │
└────────────────────────────────────────────────────────────────────────┘
```

| 区域代号 | 组件文件 | 尺寸规范 | 核心职责 |
|---|---|---|---|
| **Region 01** | [SidebarLeft.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/layout/SidebarLeft.vue) | 宽 `220px` (自 v2.0 优化，原 240px) | 窗口拖拽区、品牌 Logo、曲库分面入口、自定义歌单列表 |
| **Region 02** | [TopBar.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/layout/TopBar.vue) | 高 `60px` | 历史导航（前进/后退/主页）、全局搜索框、极简/迷你模式切换、日夜切换、窗口控制 |
| **Region 03** | [MainContent.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/layout/MainContent.vue) | 自适应 `flex-1` | 承载专辑网格、曲目列表、艺术家墙、文件夹树、歌单详情等业务视图 |
| **Region 04** | [SidebarRight.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/layout/SidebarRight.vue) | 宽 `320px` (浮层覆盖模式) | Inspector 检查器，切换展示正在播放（封面+歌词）与播放队列，不挤占内容区宽度 |
| **Region 05** | [BottomPlayer.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/layout/BottomPlayer.vue) | 高 `92px` (自 v2.0 优化，原 110px) | 左侧曲目与收藏、绝对居中的播放控制与时间进度、右侧物理质感音量旋钮 |
| **全屏沉浸层** | [NowPlayingImmersive.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/layout/NowPlayingImmersive.vue) | `w-screen h-screen` (z-index: 200) | 抽屉式向上平滑滑出的全屏黑胶大屏，呼吸光晕场（Aura Halo）与 rAF 同步全屏歌词 |

### 2.3 4 条核心分割线系统 (The 4 Dividers)
LDL 严禁使用粗重或带模糊的边框，全系统仅使用 4 条 1px 极细实线切分宏观结构：
- **Divider A**：侧边栏与主区之间的垂直分割线（1px，`bg-border-color`）
- **Divider B**：顶栏与主内容区之间的水平分割线（1px，`bg-border-color`）
- **Divider C**：主内容区与右侧 Inspector 之间的垂直分割线（1px，`bg-border-color`）
- **Divider D**：上部工作区与底部播放栏之间的贯穿水平分割线（1px，`bg-border-color`）

---

## 3. 色彩体系与双主题机制 (Color System & Themes)

### 3.1 核心调色板与严格无冷灰原则 (No Cold Gray)
传统数字音乐软件多使用偏蓝、偏青的冷灰（如 Tailwind 默认的 `zinc`、`slate`、`coolGray`），会带来冰冷、廉价的电子设备感。LDL 严禁出现冷灰，所有中性色均落在**暖色中性调（Warm Neutral Ladder）**上：

```
亮色主调 (Warm White)                 暗色主调 (Warm Industrial Dark)
┌──────────────────────────────┐     ┌──────────────────────────────┐
│ --bg-canvas:   #F7F5F1       │     │ --bg-canvas:   #1C1A17       │
│ --bg-content:  #FBFAF7       │     │ --bg-content:  #23211D       │
│ --border-solid:#E8E5DE       │     │ --border-solid:#36332D       │
│ --text-primary:#111111       │     │ --text-primary:#F2EFE9       │
└──────────────────────────────┘     └──────────────────────────────┘
```

#### 暖中性阶梯（Tailwind v4 `@theme` 绑定）
项目将骨架屏、占位图与中性渐变全部收敛映射到暖灰阶梯：
- `--color-warm-300`: `#d6d3d1`
- `--color-warm-400`: `#a8a29e`
- `--color-warm-500`: `#78716c`
- `--color-warm-600`: `#57534e`
- `--color-warm-700`: `#44403c`
- `--color-warm-800`: `#292524`

### 3.2 唯一强调色法则 (One Accent Rule)
- **色值**: `--brand-orange: #E28A23`（日间模式与夜间模式完全恒定，不随主题变化）
- **语义**: 象征老式晶体管收音机的琥珀色背光刻度盘与温暖的模拟音频指示灯。
- **允许使用场景 (受严格门禁保护)**：
  1. 当前正在播放曲目的行指示（TrackRow 左侧 2px 竖条）
  2. 播放进度条已播放长度滑轨
  3. 侧边栏/顶栏 Active 激活态下划线或指示小点（6px 橙色实心圆点）
  4. 已收藏状态的心形图标高亮（`text-brand-orange fill-brand-orange`）
  5. 物理键盘 Tab 导航时的 `:focus-visible` 2px 聚焦指示环
  6. 底部音量旋钮的动态刻度弧线
- **绝对禁忌场景**：
  - 严禁作为大面积卡片或视图的底色；
  - 严禁作为通用 Primary 按钮的填充色（Primary 按钮在 LDL 中规范为黑底白字胶囊）；
  - 严禁与辅助状态色同时混搭出现。

### 3.3 文字色彩层级与 WCAG 对比度

| 语义变量 | 亮色模式色值 | 暗色模式色值 | WCAG 对比度 | 适用场景 |
|---|---|---|---|---|
| `--text-primary` | `#111111` | `#F2EFE9` | **15.2:1 (AAA)** | 歌曲名、专辑标题、艺人名、大标题 |
| `--text-secondary` | `#5F5F5F` | `#A9A49A` | **5.8:1 (AA)** | 副标题、导航激活文字、音质格式说明 |
| `--text-muted` | `#8B8B8B` | `#7A756B` | **3.2:1 (元数据)** | 歌曲时长、音轨号、年代、次要操作图标 |
| `--text-disabled` | `#BDBDBD` | `#514E47` | -- | 禁用控件、骨架屏底层、未支持格式 |
| `--text-inverse` | `#FFFFFF` | `#111111` | **18.1:1 (AAA)** | 胶囊按钮内部反色文字、深色标签 |

### 3.4 状态色体系 (Status Colors)
状态色用于扫描结果、错误提示、云端同步反馈，夜间模式需经过明度提亮，防止在暗底上难以辨识：
- **Info (提示)**: 亮色 `#4A7FB8` / 暗色 `#6B9FD4`
- **Success (成功)**: 亮色 `#5B8C5A` / 暗色 `#7AB07A`
- **Warning (警告)**: 亮色 `#C08A3E` / 暗色 `#D9A85C`
- **Error (异常)**: 亮色 `#C24E4E` / 暗色 `#D96B6B`

---

## 4. 全量 Design Tokens 注册表 (Tokens Registry)

以下内容完整反映 [src/style.css](file:///c:/Users/hao/RustroverProjects/lumo/src/style.css) 中的实际实现：

### 4.1 字号模数阶梯 (Typography Tokens)
遵循 4pt 字阶模数体系，严禁在业务组件中随手写死 `text-[17px]` 等任意像素值：

| Token 名称 | 尺寸值 | 行高 | 典型使用场景 |
|---|---|---|---|
| `--text-9` | `9px` | 12px | 版本号、副标辅助 Mono 标签 |
| `--text-10` | `10px` | 14px | 歌曲行音质格式徽章（FLAC/96kHz）、小脚标 |
| `--text-11` | `11px` | 16px | 表头元数据字段名、迷你时间刻度 |
| `--text-12` | `12px` | 16px | 侧边栏菜单项、辅助说明、空态提示 |
| `--text-13` | `13px` | 18px | 桌面标准正文、曲目副标题（艺人/专辑） |
| `--text-14` | `14px` | 20px | 桌面歌曲列表曲名、常规输入框文字 |
| `--text-15` | `15px` | 22px | 移动端正文（触屏阅读略放大） |
| `--text-18` | `18px` | 24px | 卡片二级标题、统计数值 |
| `--text-22` | `22px` | 28px | 详情页中标题、弹窗主标题 |
| `--text-28` | `28px` | 34px | 桌面业务视图大标题（Page Title） |
| `--text-32` | `32px` | 40px | 首页大型欢迎标语、全屏黑胶大曲名 |

### 4.2 空间与尺寸 Tokens (Spatial Tokens)

```css
/* 桌面端区域尺寸 */
--width-sidebar: 220px;          /* 左侧导航栏 */
--width-inspector: 320px;        /* 右侧 Inspector 抽屉浮层 */
--height-topbar: 60px;           /* 顶部控制栏 */
--height-playback-bar: 92px;     /* 底部播放控制条 */
--height-track-row: 40px;        /* 单曲固定行高 (虚拟列表核心基准) */
--size-play-cover: 48px;         /* 底栏封面正方形尺寸 */
--size-play-button: 46px;        /* 底栏核心播放大按钮外圆 */

/* 移动端专属触控尺寸 (断点 < 768px) */
--touch-min: 44px;               /* 触控点击最小热区 (Apple HIG) */
--touch-row: 56px;               /* 移动端歌曲列表触控行高 */
--height-mobile-header: 56px;    /* 移动端顶栏高度 */
--height-mobile-tabbar: 56px;    /* 移动端底部 4-Tab 切换栏 */
--height-mobile-miniplayer: 64px;/* 移动端悬浮 Mini 播放条 */
--icon-tab: 22px;                /* 移动端 Tab 图标尺寸 */
--icon-transport-mobile: 24px;   /* 移动端切歌图标尺寸 */
--icon-play-mobile: 28px;        /* 移动端播放主图标尺寸 */
```

### 4.3 圆角系统 (Border Radius)
LDL 坚持工业级严谨微圆角，杜绝过大或全圆（Pill 仅用于独立胶囊按钮）：
- `rounded-none`: `0px`（五区工作台外壳、分割线）
- `rounded-sm`: `4px`（音质徽章、进度条轨道滑块、滚动条滑块）
- `rounded-md`: `6px`（输入框、菜单下拉项、图标悬停底衬）
- `rounded-lg`: `8px`（弹窗卡片、对话框）
- `rounded-xl`: `10px`（专辑封面、艺人封面、黑胶大卡片）
- `rounded-full`: `9999px`（Primary 胶囊按钮、音量旋钮、Transport 播放圆按钮）

---

## 5. 字体排印规范 (Typography)

### 5.1 字体栈声明 (Font Families)
前端在 [index.html](file:///c:/Users/hao/RustroverProjects/lumo/index.html) 引入 Web 字体并在 [style.css](file:///c:/Users/hao/RustroverProjects/lumo/src/style.css) 中声明两套核心字族：

```css
/* 无衬线西文与中文优雅混排栈 */
--font-sans: "Inter", "SF Pro Display", "Helvetica Now", "MiSans", "HarmonyOS Sans", "OPPOSans", sans-serif;

/* 高精工业等宽字体栈 (强制用于数字、时间、技术元数据) */
--font-mono: "IBM Plex Mono", "JetBrains Mono", monospace;
```

### 5.2 杂志感跳跃对比 (Editorial Rhythm)
LDL 强调通过字阶的大跨度对比营造出杂志版面般的阅读节奏：
- 避免全界面平铺同字号文字；
- 页面标题直接使用 `28px Bold`，紧跟 `12px Mono` 的副元数据（曲目总数、时长总计）；
- 歌手详情页头部采用艺术级巨幅肖像配合重度对齐的纯文字排印。

### 5.3 等宽数字使用规范 (Tabular Numbers)
界面中所有与时间、数据变动相关的数值，必须应用 `tabular-nums` 或 `font-mono`：
- 当前播放时长 / 总时长：`02:15 / 04:32`
- 音频参数：`FLAC · 24bit / 96.0kHz · 2840kbps`
- 歌曲行序号：`01`, `02`, `12`（右对齐或补零对齐）

---

## 6. 图标与媒体资产系统 (Iconography & Assets)

### 6.1 图标库选型与描边规范
- **图标引擎**：完全统一采用 `lucide-vue-next` (v0.577.0)。
- **统一线条粗细**：所有常规图标强制设定 `stroke-width="1.5"`，保持轻盈线框美学。
- **色彩继承原则**：图标一律使用 `currentColor` 继承父级文字色彩阶梯（`text-text-muted`，Hover 时过渡到 `text-text-primary`）。
- **实心填充例外**：全系统仅在表示“媒体播放”状态（Play/Pause 三角）及“已收藏心形”时允许启用 `fill-current`。

### 6.2 常用图标规格矩阵

| 尺寸等级 | 像素值 | 使用场景示例 |
|---|---|---|
| **Micro** | `12px` | 歌曲音质类型标志（Hi-Res 箭头）、小排序指示 |
| **Small** | `14px` | 搜索栏清除按钮、列表行次级操作菜单、快捷键提示 |
| **Medium** | `16px` | 侧边栏导航条目、表头列配置齿轮、音量小喇叭 |
| **Regular** | `18px` | 顶栏历史导航键（ArrowLeft / ArrowRight）、Inspector 切换键 |
| **Large** | `20px` | 底部播放栏上一曲/下一曲、循环模式、随机播放 |
| **Hero** | `24px / 28px` | 核心播放大按钮主三角图标、全屏沉浸大屏控制 |

---

## 7. 层级深度、阴影与动效规范 (Elevation, Motion & A11y)

### 7.1 无阴影规则与 5 个受控阴影例外 (The 5 Controlled Shadows)
LDL 严格践行“Divider Over Card”原则，全系统默认**彻底禁用阴影（Flat Design）**，仅允许通过色层差异（Canvas `#F7F5F1` vs Content `#FBFAF7`）及 1px 分割线表现结构。

全系统仅严格容许以下 **5 个受控阴影例外**：
1. **专辑卡片 Hover 播放浮层**：`shadow-md`（悬浮在封面右下角的 38px 橙底播放圆形浮动按钮）
2. **Now Playing 沉浸页大封面浮层**：`shadow-2xl`（配合 Aura Halo 光晕脱离背景）
3. **Inspector 抽屉式侧边栏浮层**：`shadow-lg`（覆盖在主内容区上方时的微弱侧阴影）
4. **底栏物理音量旋钮内凹阴影**：`inset 0 1px 2px rgba(0,0,0,0.12)`（表现微拟物实体触感）
5. **移动端 Now Playing 浮层顶部阴影**：`--shadow-mobile-overlay: 0 -4px 16px rgba(0, 0, 0, 0.12)`（覆盖层自底向上拉起时的边界反差）

### 7.2 动效节奏与时长曲线
LDL 严禁使用任何弹簧物理震颤（Spring）、回弹晃动（Bounce）或花哨的缩放（Zoom）。仅允许三档极简时长：

```css
/* LDL 动效核心参数 */
--duration-fast: 150ms;   /* 按钮悬停、图标高亮等微交互 */
--duration-normal: 200ms; /* 背景变色、日夜主题切换、常规组件展开 */
--duration-slow: 250ms;   /* 视图切换、Now Playing 全屏抽屉滑入滑出 */

/* 唯一允许动效贝塞尔曲线 (标准优雅缓出) */
transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
```

### 7.3 可访问性与减弱动效 (A11y & Reduced Motion)
项目完整遵守系统级无障碍配置：

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
> **设计例外说明**：即便在 `prefers-reduced-motion` 激活状态下，`Loader2` 的匀速旋转（`animate-spin`）仍然例外保留，因为旋转是用户获知“后台正在扫描/加载”的**唯一生命体征反馈**。

### 7.4 极简体验模式覆盖 (DM-04 Minimal Experience)
在用户或系统触发极简体验模式（`data-experience="minimal"`）时：
- 全局强制关闭毛玻璃效果：`backdrop-filter: none !important;`
- 消除复杂的呼吸光晕与非必要过渡：`animation-duration: 0.01ms !important;`
- 保留最核心的状态变更（选中高亮、禁用灰色、加载转圈）。

---

## 8. 组件视觉与交互设计规范 (Components Specs)

### 8.1 按钮组件体系 (Buttons)
1. **Primary Button（主操作按钮）**：
   - 视觉形态：高对比度纯黑底白字（暗色下纯白底黑字）圆角胶囊（`rounded-full`），高度 32px 或 36px。
   - 规则：**同一视图内至多允许出现一个 Primary 按钮**，防止操作视觉争抢。
2. **Secondary Button（次操作按钮）**：
   - 视觉形态：1px 细线描边（`border-border-solid`），背景透明，Hover 时亮起暖白底。
3. **Ghost / Icon Button（图标按钮）**：
   - 视觉形态：平时无边框、无背景；尺寸 32×32 像素（图标 16/18px）；Hover 时呈现微暖背景衬底（`bg-btn-hover`，`rounded-md`）。
4. **Transport Play Button（主播放大键）**：
   - 底部播放栏核心控制键，46px 圆形黑色实体，图标绝对居中，具备微按压反馈（`active:scale-95`）。

### 8.2 歌曲列表系统 (TrackRow Specs)
歌曲行组件 [TrackRow.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/shared/trackList/TrackRow.vue) 是整个播放器使用频率最高的 UI 单元：
- **行高恒定**：桌面端固定 `40px`，移动端固定 `56px`。
- **四态交互规范**：
  1. *Default（默认态）*：透明底，文字按层级呈中性色。
  2. *Hover（悬停态）*：整行呈现 `--list-hover` 暖灰底衬，左侧序号渐变为播放播放小图标。
  3. *Selected（选中态）*：整行呈现 `--list-selected` 选中底色，多选勾选框高亮。
  4. *Playing（正在播放态）*：加挂 `.playing-row` 类，行左侧绝对定位绘制 **2px 暖橘色竖条**，曲目标题变为 `text-brand-orange`，序号处展示律动等化器指示。
- **列收纳响应式规则**：当内容区物理宽度收窄时，按优先级从右向左逐级隐藏：大小 → 流派 → 年份 → 音频参数（码率/采样率） → 专辑名，确保核心曲名与时长始终可见。

### 8.3 专辑网格与卡片 (AlbumCard Specs)
- **卡片宽高比**：严格 1:1 正方形封面。
- **圆角参数**：`rounded-[10px]`。
- **Hover 浮动操作**：鼠标悬停在卡片上时，右下角淡入 38px 暖橘底色圆形播放按钮（带 `shadow-md`），点击立即开始串联播放该专辑所有音轨。
- **极简模式降级**：在极简体验模式下，网格可无缝回退为纯文字高密度排版 [MinimalEntityList.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/content/MinimalEntityList.vue)。

### 8.4 物理质感音量旋钮 (Analog Volume Knob)
位于底部播放栏右侧，致敬经典实体功放设备：
- 平时显示紧凑的音量图标与数值；
- 展开时呈现具备 SVG 刻度圆弧的旋钮面板；
- 支持线性拖拽手势与鼠标滚轮微调（步进 1%~5%）；
- 刻度线使用 `--brand-orange` 与暖中性灰平滑过渡。

---

## 9. 移动端 (Android / 移动视口) 专属适配体系

移动端在完全继承 LDL v2.0 色彩与排版 Token 的基础上，针对触控特性进行了精准重构：
1. **触控行高扩展**：歌曲列表行由 40px 扩展至 56px，确保拇指盲点无误触。
2. **底部安全区避让**：全界面使用 `env(safe-area-inset-bottom)` 与 `env(safe-area-inset-top)`，完美避让刘海屏与系统手势底横条。
3. **底部 4-Tab 架构**：[MobileTabBar.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/mobile/MobileTabBar.vue) 固定高度 56px，提供曲库、搜索、收藏、设置 4 大根入口。
4. **移动 Mini 播放条**：[MobileLayout.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/mobile/MobileLayout.vue) 常驻 64px 悬浮播放条，点击向上平滑拖拽唤出全屏 [MobileNowPlaying.vue](file:///c:/Users/hao/RustroverProjects/lumo/src/components/mobile/MobileNowPlaying.vue)。
