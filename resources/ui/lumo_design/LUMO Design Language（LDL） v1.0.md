# LUMO Design Language（LDL） v1.0

> **Warm Industrial Minimalism**
>
> *Quiet • Focused • Timeless*

---

# 目录（Table of Contents）

```
01. Design Philosophy
02. Design Principles
03. Spatial System
04. Layout System
05. Grid System
06. Color System
07. Typography
08. Iconography
09. Elevation & Divider
10. Component Specification
11. Motion System
12. Interaction Rules
13. Design Tokens
14. AI Prompt Specification
```

---

# 01 Design Philosophy（设计理念）

## Vision

打造一个能够陪伴用户数十年的音乐管理工具。

不是炫酷。

不是潮流。

而是：

> 安静、耐看、专业。

一句话：

> **Less Interface, More Music.**

---

## Core Keywords

```
Warm

Minimal

Industrial

Editorial

Scandinavian

Swiss

Professional

Timeless
```

---

## Design Goals

优先级：

```
Content

↓

Information

↓

Interaction

↓

Decoration
```

界面永远不要抢内容。

---

# 02 Design Principles（设计原则）

### ① Typography First

使用排版建立层级。

不是颜色。

不是卡片。

---

### ② Whitespace First

使用留白建立节奏。

不是阴影。

---

### ③ Divider First

区域之间全部依靠：

```
1px Divider
```

不要：

Card

Glass

Shadow

---

### ④ One Accent

整个系统只有一种强调色。

```
#E28A23
```

任何页面：

只能出现一个视觉焦点。

---

### ⑤ Quiet Interface

所有控件都应该：

安静。

克制。

低存在感。

---

# 03 Spatial System（空间系统）

## Design Canvas

基准：

```
2560 × 1600
```

推荐：

```
1920+

最佳：

2560
```

---

## Window Layout

```
┌────────────┬───────────────────────────────────────────────┬──────────────────────┐
│            │                                               │                      │
│ Sidebar    │                  Top Bar                      │                      │
│            ├───────────────────────────────────────────────┤ Inspector Panel      │
│            │                                               │                      │
│            │                Content Area                   │                      │
│            │                                               │                      │
├────────────┴───────────────────────────────────────────────┴──────────────────────┤
│                         Playback Bar                                              │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## Region Size

### Sidebar

```
240px
```

固定宽度。

占：

10%

---

### Inspector

```
360px
```

固定宽度。

占：

18%

---

### Content

自动伸缩。

占：

72%

---

### Top Bar

```
60px
```

仅覆盖：

Content + Inspector。

Sidebar 独立。

---

### Playback

```
110px
```

固定高度。

---

# 04 Layout System（布局系统）

## Sidebar

结构：

```
Logo

↓

Library

↓

Playlists

↓

Blank Space
```

职责：

应用导航。

禁止：

搜索

设置

皮肤

模式

播放控制

---

## Top Bar

右上：

```
Skin

Theme

Settings

More

—

Minimize

Maximize

Close
```

职责：

应用级操作。

不承担页面功能。

---

## Content

结构：

```
Page Title

↓

Metadata

↓

Toolbar

↓

List/Grid

↓

Status
```

Toolbar：

负责：

搜索

排序

筛选

视图切换

这些全部属于页面。

不是 Top Bar。

---

## Inspector

两个模式。

### Now Playing

```
Tab

↓

Album

↓

Song Info

↓

Lyrics
```

---

### Queue

```
Tab

↓

Queue
```

切换：

Tab。

---

## Playback

左：

Album

中：

Transport

右：

Volume

---

# 05 Grid System

统一：

```
8pt Grid
```

所有尺寸来自：

```
8

16

24

32

40

48

64

80

96
```

禁止：

17px

27px

53px

这种随机值。

---

# 06 Color System

## Canvas

```
#F7F5F1
```

---

Content

```
#FBFAF7
```

---

Divider

```
#E8E5DE
```

---

Primary

```
#111111
```

---

Secondary

```
#5E5E5E
```

---

Muted

```
#999999
```

---

Accent

```
#E28A23
```

整个系统：

唯一强调色。

---

# 07 Typography

推荐：

中文：

```
MiSans

HarmonyOS Sans
```

英文：

```
Inter
```

数字：

```
IBM Plex Mono
```

字号：

```
Title

40

H1

28

H2

22

Body

15

Caption

12
```

---

# 08 Divider System

整个 UI

只有：

四条一级 Divider。

```
Sidebar │ Content

TopBar ─────

Content │ Inspector

Playback ───
```

全部：

```
1px

#E8E5DE
```

---

# 09 Component Specification（组件规范）

建议定义完整组件库：

### Navigation

* Sidebar Item
* Sidebar Group
* Navigation Badge

---

### Buttons

* Icon Button
* Primary Button
* Secondary Button
* Ghost Button

---

### Table

* Row
* Hover
* Selected
* Playing

---

### Music

* Song Row
* Album Card
* Artist Card
* Queue Item
* Playlist Item

---

### Player

* Timeline
* Volume
* Waveform
* Play Button
* Album Cover

---

### Inspector

* Lyrics
* Metadata
* Queue
* Audio Info

---

### Feedback

* Empty State
* Loading
* Toast
* Dialog

---

# 10 Motion

动画：

```
150ms

200ms

250ms
```

Curve：

```
Ease-Out
```

禁止：

Bounce

Spring

Zoom

Rotation

---

# 11 Interaction

Hover：

```
#F2EFE8
```

Selected：

```
#ECE7DE
```

Current Playing：

```
左侧

2px Accent
```

---

# 12 Design Tokens

统一 Token：

```
Radius

6

8

10

16

Spacing

8

16

24

32

48

64

Shadow

None

Border

#E8E5DE
```

---

# 13 AI Prompt Specification（AI生成规范）

今后所有 AI（ChatGPT、Claude、Midjourney、Figma AI 等）生成界面时，都应遵循以下统一提示词：

```text
LUMO Design Language v1.0, Warm Industrial Minimalism, Scandinavian Editorial UI, Swiss Grid System, Teenage Engineering inspired, Braun industrial aesthetics, warm ivory background (#F7F5F1), content-first layout, persistent desktop workspace, three-column layout with a fixed left sidebar (240px), fixed right inspector panel (360px), top application toolbar (60px), bottom playback bar (110px), ultra-thin 1px dividers (#E8E5DE), no cards, no shadows, no gradients, no glassmorphism, typography-driven hierarchy, spacious 8pt grid, monochrome interface with a single muted orange accent (#E28A23), soft rounded corners (6–10px), clean modular spacing, premium Hi-Fi desktop application, timeless, quiet, information-first.
```

---

## 我还有一个建议

这份规范目前属于 **v1.0（Foundation）**，建议继续扩展为完整的 **LUMO Design System**，最终规模控制在 **150–200 页**。内容不仅包括基础设计语言，还会覆盖每一个组件（Button、Table、Lyrics、Album、Queue、Dialog、Toast、播放器等）、交互状态、设计 Token，以及开发规范（CSS Variables / Figma Variables），真正达到 **Apple HIG** 或 **Material Design** 的专业程度。对于你希望长期迭代的 LUMO 来说，这样的设计系统会极大提升一致性和开发效率。
