我觉得可以直接把它定义成一套自己的设计体系，而不是简单地说"参考Teenage Engineering"。

因为现在你的 LUMO 已经有自己的特点了：

* 比 TE 更现代
* 比 Apple Music 更克制
* 比 Notion 更有温度
* 比 Linear 更适合长时间使用

我建议直接命名为：

> **LUMO Design Language（LDL） v1.0**

副标题：

> **Warm Industrial Minimalism**

---

# LUMO Design Language（LDL）

## Design Philosophy（设计理念）

> **Quiet. Focused. Timeless.**

LUMO 不追求视觉冲击，而追求一种可以长时间使用、不疲劳的专业体验。

设计原则：

* 信息第一，装饰第二
* 排版代替边框
* 留白代替颜色
* 层级代替阴影
* 内容始终是主角

一句话：

> **Less Interface, More Music.**

---

# 一、整体布局（Layout）

你说得对，我前面的布局规范确实有问题。

因为**你现在的 LUMO 已经不是传统的 Sidebar + Content + Player 三栏应用了**，而是更接近专业创作软件（如 Logic Pro、DaVinci Resolve、Linear Desktop）的布局体系。

你的 UI 实际上有 **5 个一级区域 + 1 个固定播放器**，每个区域都有独立职责。

建议把 Layout 章节重新写成下面这样。

---

# LUMO Design Language

# Layout System（布局系统）

## Layout Philosophy（布局理念）

LUMO 采用 **Persistent Workspace（持续工作空间）**。

整个应用不存在弹窗式工作流。

所有主要功能均保持可见，避免界面切换造成的信息中断。

整个界面由 **六个固定区域（Persistent Regions）** 组成。

```text
┌────────────┬───────────────────────────────────────────────┬──────────────────────────┐
│            │                                               │                          │
│ Sidebar    │                Top Bar                        │                          │
│            ├───────────────────────────────────────────────┤ Inspector Panel          │
│            │                                               │                          │
│            │             Content Area                      │                          │
│            │                                               │                          │
├────────────┴───────────────────────────────────────────────┴──────────────────────────┤
│                            Playback Bar                                           │
└────────────────────────────────────────────────────────────────────────────────────┘
```

整个布局始终保持固定。

任何页面都不得改变整体结构。

---

# Region 01

## Sidebar（导航栏）

宽度：

```text
240px
```

高度：

整个窗口

永远位于最左侧。

**注意：**

Sidebar 是整个应用最顶层区域。

它**覆盖 Top Bar 的高度**。

因此：

顶部横线不能穿过 Sidebar。

导航栏左侧形成一个完整独立区域。

即：

```
┌──────────┬──────────────────────────────
│ Sidebar  │ TopBar
│          │──────────────
│          │ Content
```

而不是：

```
──────────────
Sidebar
──────────────
```

这是整个设计的重要特征。

---

### Sidebar 职责

只负责：

一级导航

例如：

Library

Playlist

Folder

Artist

Album

Composer

Recent

不承担：

搜索

播放器

工具按钮

状态信息

设置

所有工具行为全部交给 Top Bar。

---

# Region 02

## Top Bar（顶部工具栏）

高度：

```text
60px
```

Top Bar 不属于 Sidebar。

Top Bar 从 Content 左边开始。

一直延伸到 Inspector。

即：

```
Sidebar │ TopBar──────────────Inspector
```

顶部栏只承担：

窗口级操作。

---

### Top Bar 包含

右侧：

```
Theme

Skin

Setting

More

—

Minimize

Maximize

Close
```

绝不出现：

页面搜索

筛选

分类

这些属于 Content。

Top Bar 永远不参与页面业务。

它属于：

Application Chrome。

---

# Region 03

## Content Area（内容区）

这是整个软件最大的区域。

承担：

数据浏览

数据管理

数据编辑

例如：

歌曲列表

专辑

歌手

文件

播放历史

NAS

等等。

Content 永远不承担：

播放信息

歌词

播放队列

这些全部放在 Inspector。

---

Content 内部分层：

```
Page Title

↓

Metadata

↓

Toolbar（当前页面工具）

↓

Table/List/Grid

↓

Status Bar
```

例如：

```
全部歌曲

12,483 首歌曲

──────────────────

视图切换

──────────────────

歌曲列表

──────────────────

统计信息
```

注意：

Toolbar 属于页面。

不是 Top Bar。

以后：

搜索

筛选

排序

切换视图

都放这里。

---

# Region 04

## Inspector Panel（右侧信息面板）

宽度：

```text
360px
```

作用：

上下文信息。

始终跟随当前选中对象。

而不是当前页面。

例如：

当前歌曲

当前专辑

当前歌手

等等。

它是：

Context Panel。

---

Inspector 分成两个模式。

### Mode A

Now Playing

```
Tab

↓

Album Cover

↓

Song Information

↓

Lyrics
```

---

### Mode B

Queue

```
Tab

↓

Play Queue
```

两者切换：

顶部 Tab。

不是两个区域同时显示。

---

# Region 05

## Playback Bar（播放器）

固定：

底部

高度：

110px

承担：

播放控制。

仅包含：

Album

Play

Timeline

Volume

Output

绝不承担：

歌词

播放列表

播放信息

这些全部属于 Inspector。

---

# Region 06

## Divider System（分割系统）

整个 UI 的层级完全依靠：

**Divider（分割线）**

而不是：

阴影

卡片

背景色

整个应用只有四条主分割线。

---

### Divider A

Sidebar

↓

Content

```
│
```

从窗口顶部一直到底部。

---

### Divider B

Top Bar

↓

Content

```
──────────────
```

只存在于：

Content + Inspector。

不会穿过 Sidebar。

---

### Divider C

Content

↓

Inspector

```
│
```

贯穿：

Top Bar

Content

一直到底部播放器。

---

### Divider D

Playback

↓

Workspace

```
──────────────
```

贯穿整个窗口。

---

# 整个布局的层级（Hierarchy）

最后，把整个层级明确下来：

```text
Window
│
├── Sidebar（应用导航）
│
├── Workspace
│   │
│   ├── Top Bar（窗口级工具）
│   │
│   ├── Content Area（页面内容）
│   │    ├── Page Header
│   │    ├── Metadata
│   │    ├── Page Toolbar
│   │    ├── List / Grid
│   │    └── Footer Status
│   │
│   └── Inspector（上下文信息）
│        ├── Now Playing
│        ├── Lyrics
│        └── Queue（Tab 切换）
│
└── Playback Bar（全局播放器）
```

**这一版会比之前专业得多，也更符合 Figma、Sketch 等大型桌面软件的设计规范。** 它不仅定义了布局，还定义了**每个区域的职责边界**，后续无论增加功能还是让 AI 生成新页面，都能保持整个 LUMO 的一致性。


# 二、栅格（Grid）

统一采用 **8pt Grid**。

```
4px
8px
16px
24px
32px
40px
48px
64px
80px
96px
```

所有尺寸必须来自这个体系。

例如：

```
Sidebar

240px

Inspector

360px

TopBar

60px

Bottom Player

110px
```

---

# 三、颜色规范（Color）

## Background

Warm White

```
Canvas

#F7F5F1
```

Content

```
#FBFAF7
```

Sidebar

```
#F7F5F1
```

Inspector

```
#F7F5F1
```

不要出现纯白。

不要出现冷灰。

---

## Divider

所有边框统一：

```
#E8E5DE
```

透明度：

```
70%
```

宽度：

```
1px
```

禁止：

阴影

卡片

粗边框

---

## Text

Primary

```
#111111
```

Secondary

```
#5F5F5F
```

Muted

```
#8B8B8B
```

Disabled

```
#BDBDBD
```

---

## Accent

整个系统只有一种强调色：

```
#E28A23
```

仅用于：

❤

播放

进度条

歌词高亮

Hover

禁止：

绿色

蓝色

紫色

红色按钮

---

# 四、Typography

推荐字体：

中文：

```
MiSans

HarmonyOS Sans

OPPOSans
```

英文：

```
Inter

SF Pro Display

Helvetica Now
```

数字：

```
IBM Plex Mono

JetBrains Mono
```

---

字号：

```
Page Title

42

Section

20

Body

15

Secondary

13

Caption

11
```

行高：

```
150%
```

---

# 五、圆角

整个系统只有三种：

```
6px

10px

16px
```

Album Cover：

```
10px
```

按钮：

```
8px
```

播放按钮：

```
100%
```

---

# 六、图标

统一：

Stroke

2px

Rounded

24×24

推荐：

Lucide

Phosphor

Remix Icon

不要：

拟物

彩色

填充风格

---

# 七、间距（Spacing）

所有距离采用统一体系：

```
4
8
12
16
24
32
48
64
```

例如：

标题

↓

24

统计信息

↓

32

列表

↓

24

Footer

---

# 八、边框

永远只有：

```
1px
```

颜色：

```
#E8E5DE
```

不要：

阴影

玻璃

磨砂

渐变

卡片

整个UI依靠：

留白

分割线

字体

建立层级。

---

# 九、按钮

按钮默认：

透明

Hover：

```
#F1EFE9
```

Active：

Accent

不要：

立体按钮

胶囊按钮

发光按钮

---

# 十、列表

Hover：

浅米色

```
#F5F2EB
```

Selected：

```
#F0ECE4
```

Current Playing：

左侧：

```
2px Accent
```

不要蓝色高亮。

---

# 十一、动画

整个系统动画非常慢。

推荐：

```
150ms

200ms

250ms
```

Curve：

```
ease-out
```

不要：

弹簧

缩放

Bounce

旋转

---

# 十二、留白原则

任何页面：

至少

30%

为空白。

永远不要：

把空间填满。

宁愿空。

---

# 十三、组件原则

每个页面只允许出现：

```
一个 Accent Color

一个 Primary Button

一个 Focus Area
```

其它全部保持克制。

---

# 十四、视觉关键词（Prompt）

以后无论是生成 UI，还是让 AI 帮你设计，都可以直接使用下面这一段，它几乎就是 **LUMO Design Language** 的完整视觉描述：

```text
Warm Industrial Minimalism, Scandinavian Editorial UI, Swiss Grid System, Teenage Engineering inspired, Braun industrial aesthetics, large breathing space, warm ivory background (#F7F5F1), ultra-thin 1px dividers (#E8E5DE), monochrome interface with a single muted orange accent (#E28A23), typography-driven hierarchy, no cards, no shadows, no glassmorphism, no gradients, clean modular layout, precise spacing on an 8pt grid, quiet premium desktop application, timeless and functional design, information-first, subtle micro-interactions, soft rounded corners (6–10px), minimal iconography, editorial composition, professional Hi-Fi audio software aesthetic.
```

---

## LUMO 的设计原则（十诫）

1. **内容永远比装饰重要。**
2. **留白创造层级，而不是阴影。**
3. **一种强调色胜过多种颜色。**
4. **排版就是界面。**
5. **用细线分隔区域，而不是卡片。**
6. **保持温暖中性的色调，避免纯白和高饱和。**
7. **每个页面只保留一个视觉焦点。**
8. **交互应安静、流畅，不喧宾夺主。**
9. **所有元素遵循统一的 8pt 栅格与间距体系。**
10. **让界面隐退，让音乐成为主角。**

如果坚持这一套规范，LUMO 会形成非常统一且具有辨识度的品牌设计语言，而不是仅仅模仿某一种现有风格。

模板实例:![](./LUMO_全部歌曲.png)