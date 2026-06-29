# 00 Foundation

> **Warm Industrial Minimalism**
>
> *Quiet · Focused · Timeless*

---

## Document Information

| 字段 | 值 |
|---|---|
| **Version** | LDL v2.0 |
| **Status** | Canonical（唯一规范来源） |
| **Owner** | LUMO Design Team |
| **Supersedes** | LDL v1.0（2026-06，存档于 `../LUMO Design Language（LDL） v1.0.md`） |
| **Purpose** | 建立一套可持续演进十年以上的设计语言，作为 LUMO 产品**唯一的视觉、交互与工程规范（Single Source of Truth）**。所有设计稿、组件、代码实现、AI 生成页面均必须遵循本规范。 |

---

## Design Constitution（设计宪法）

LUMO 的设计语言建立在以下十条原则之上。**任何新的设计决策都不得违背这些原则。** 当原则之间发生冲突时，按编号靠前者优先。

### ① Content First

内容永远是产品的核心。界面必须服务于内容，而不是争夺用户注意力。

```
Content → Information → Interaction → Visual → Decoration
```

任何新增功能都必须遵循这一层级。Decoration 永远是最后考虑的。

---

### ② Typography Before Decoration

优先使用字体、字号、字重和排版建立层级。避免依赖阴影、渐变或大面积背景色。

> 见 [04 Typography](04-typography.md)。

---

### ③ Whitespace Creates Rhythm

留白创造节奏。宁可空，也不要拥挤。间距永远来自 [01 Tokens](01-tokens.md) 的 spacing 阶梯，不允许 magic number。

> 见 [02 Spatial System](02-spatial.md)。

---

### ④ Divider Over Card

优先使用 **1px Divider** 划分区域。避免大量 Card、复杂背景、Glassmorphism。

整个界面只有四条**一级 Divider**：

```
Sidebar │ Content
TopBar ──────
Content │ Inspector
Playback ────
```

> 见 [06 Elevation](06-elevation.md)。

---

### ⑤ One Accent Rule

整个界面**仅允许一种主强调色**：`#E28A23`（Warm Orange）。

Accent **仅用于**：

- 当前播放（Now Playing 指示）
- 当前选中（Selected 指示点 / Tab 下划线）
- Progress（进度条 / 波形已播放段 / 旋钮刻度）
- Active（激活态图标，如夜间模式开启）
- Focus（输入框聚焦描边）

**禁止**：

- 多个 Accent 同时出现在同一视图
- 用 Accent 做大色块背景
- 用 Accent 做正文文字色
- Transport 控件（Play/Skip）用 Accent —— Transport 是中性控件，使用 Primary 黑

> 见 [03 Color System](03-color.md)。

---

### ⑥ Persistent Workspace

所有一级区域**固定**：Sidebar 240px / TopBar 60px / Inspector 360px / Playback 110px / Content 自动伸缩。

避免频繁切换页面、避免弹窗式工作流。详情页通过 Content 内部视图切换实现，不弹窗。

> 见 [02 Spatial System](02-spatial.md)。

---

### ⑦ Everything is Tokenized

任何颜色、字号、圆角、间距、动画时长均必须来自 [Design Token](01-tokens.md)。

**禁止 Magic Number。禁止 Hard Code。**

新增视觉值时，先在 `01-tokens.md` 注册 token，再在 `style.css` 的 `:root` / `@theme` 中落地，最后在组件中使用 Tailwind utility（如 `bg-bg-canvas`）。

---

### ⑧ Accessibility by Default

所有组件默认满足：

- Keyboard Navigation
- WCAG AA（对比度 4.5:1 正文 / 3:1 大字与图形）
- Focus Visible
- Screen Reader（aria-label / role）

Accessibility 不是补充功能，而是默认能力。

> 见 [08 Accessibility](08-accessibility.md)。

---

### ⑨ Code Follows Design

LDL 是唯一规范来源。信息流单向：

```
LDL Document (canonical)
    ↓
CSS Variables (style.css :root)
    ↓
@theme → Tailwind utility 映射
    ↓
Vue Components (Tailwind class)
    ↓
Application
```

**任何实现不得偏离 LDL。** 如有冲突，以 LDL 为准，随后更新代码对齐。

---

### ⑩ Timeless Over Trend

LUMO 不追逐流行设计。设计目标是：**十年以后依然耐看。**

禁用：Bounce / Spring / Zoom / Rotation 动画、Glassmorphism、Neumorphism、Aurora 渐变、Mesh 渐变。

---

## Product Personality

LUMO 的品牌人格：

```
Quiet        安静
Warm         温暖
Professional 专业
Reliable     可靠
Calm         平静
Focused      专注
Timeless     耐看
```

---

## Visual DNA

LUMO 的视觉语言来自以下设计流派的融合：

| 流派 | 贡献 |
|---|---|
| **Scandinavian Minimalism** | 暖白底色、克制色彩、功能至上 |
| **Swiss Grid** | 8pt 栅格、严格对齐、排版驱动层级 |
| **Braun Industrial Design** | 物理控件质感（音量旋钮）、工业克制 |
| **Editorial Layout** | 杂志感排版、巨幅肖像、字号对比韵律 |
| **Teenage Engineering** | 实体硬件感、极简面板、精密刻度 |
| **Japanese Minimalism** | 留白即设计、负空间呼吸 |

这些共同形成：**Warm Industrial Minimalism**。

---

## Core Keywords

```
Warm
Industrial
Editorial
Minimal
Premium
Hi-Fi
Desktop First
Long-term
```

---

## Single Source of Truth

整个产品只有一个设计规范来源：

```
LDL Document
    ↓
Figma Variables（未来）
    ↓
CSS Variables（style.css）
    ↓
Vue Components
    ↓
Application
```

任何实现不得偏离 LDL。如有冲突，以 LDL 为准。

---

## Document Structure

v2.0 采用 **Hybrid 结构**：一本主手册（Foundation + Tokens + 全部 Systems）+ `09-components/` 子目录（每个组件一文件）。

| 文件 | 内容 |
|---|---|
| `00-foundation.md` | 宪法 / 人格 / DNA / 目标 / 路线图（本文） |
| `01-tokens.md` | 完整 token 表（亮+暗）+ 命名规则 + Tailwind 映射 + 魔数迁移 |
| `02-spatial.md` | 画布 / 五大区域 / 栅格模数 / 响应式 |
| `03-color.md` | 调色板 / 用法规则 / Accent 边界 / 状态色 / 暗色 / AA |
| `04-typography.md` | 字族 / 字重 / 字号 / 行高 / 字距 / 等宽用法 |
| `05-iconography.md` | lucide / stroke / 尺寸 / currentColor / 线框 vs 填充 |
| `06-elevation.md` | z-index 阶梯 / 无阴影规则 / 例外 |
| `07-motion.md` | 150/200/250 场景映射 / 规范曲线 / reduced-motion |
| `08-accessibility.md` | 键盘 / focus ring / AA / aria |
| `09-components/*.md` | 每个组件：Anatomy / Sizes / States / Tokens / Do&Don't / 代码片段 |
| `10-ai-prompt.md` | AI 生成界面的提示词规范 |
| `11-brand.md` | LUMO wordmark / logo 用法 |

---

## Version Roadmap

| 版本 | 范围 | 状态 |
|---|---|---|
| **v2.0** | Foundation / Tokens / 全部 Systems（含 Dark Mode、Motion、A11y、Iconography、Elevation）/ 核心组件（Button×3、Input、SongRow、AlbumCard、Tab、EmptyState、PlaybackBar、Inspector） | **当前** |
| v2.1 | Patterns（列表/网格/详情模板）/ Charts（播放统计）/ Template 整合 |
| v2.2 | Plugin SDK 设计规范 / 高级组件（Modal、Menu、Toast、Dialog、Command Palette） |
| v3.0 | Multi-platform（Windows / macOS / Linux 差异）/ Touch Device / 缩放策略完整化 |

---

## Changelog

### v2.0 — 2026-06

相对 v1.0 的主要变更：

- **新增** 暗色模式完整规范（v1.0 完全缺失，代码已有）
- **新增** Iconography / Elevation / Accessibility / Motion 完整章节（v1.0 目录有但正文缺失）
- **新增** 状态色系统（info / success / warning / error）
- **新增** 完整 Design Token 表 + Tailwind `@theme` 映射 + 魔数迁移表
- **新增** 10 个核心组件的完整规范（Anatomy / States / Do&Don't / 代码片段）
- **修正** Page Title 字号：42px → **32px**（60px TopBar 下 42 过大）
- **修正** Muted 文字色：#999999 → **#8B8B8B**（AA 对比度不达标）
- **修正** 章节编号错乱（v1.0 目录 14 章与正文 13 章不符）
- **澄清** 8pt Grid 矛盾：布局/间距=8pt，字号=4pt 模数，圆角独立模数
- **澄清** 动效曲线：统一为 `cubic-bezier(0.4, 0, 0.2, 1)`
- **澄清** Play 主按钮用 Primary 黑，不用 Accent（符合 One Accent Rule）
- **引入** AlbumCard Hover 播放浮层为受控阴影例外

---

*End of Foundation.*
