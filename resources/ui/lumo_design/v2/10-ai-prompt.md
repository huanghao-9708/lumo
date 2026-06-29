# 10 AI Prompt Specification

> 今后所有 AI（ChatGPT、Claude、Midjourney、Figma AI、v0 等）生成 LUMO 界面时，都应遵循以下统一提示词。

---

## 1. 基础提示词（通用）

用于生成任意 LUMO 界面的基础提示词。复制粘贴即可：

```text
LUMO Design Language v2.0, Warm Industrial Minimalism, Scandinavian Editorial UI, Swiss Grid System, Teenage Engineering inspired, Braun industrial aesthetics.

Warm ivory background (#F7F5F1 canvas, #FBFAF7 content), content-first layout, persistent desktop workspace.

Three-column layout: fixed left sidebar (240px), fixed right inspector panel (360px, collapsible), top application toolbar (60px), bottom playback bar (110px), content area auto-flex.

Ultra-thin 1px dividers (#E8E5DE / rgba(232,229,222,0.7)) for region separation. No cards, no shadows (except album-card hover overlay), no gradients, no glassmorphism, no neumorphism.

Typography-driven hierarchy: Inter for sans, IBM Plex Mono for numbers/metadata. Page Title 32px bold, H1 28px, H2 22px, body 13px, metadata 10px mono uppercase with tracking.

Spacious 8pt grid for spacing (8/16/24/32/48/64), 4pt for font sizes, independent radius scale (6/8/10/16px).

Monochrome interface with a single muted orange accent (#E28A23) used ONLY for: now-playing indicator, selected state, progress bar, active tab, focus ring, favorited heart. Transport play button is black (bg #111), NOT accent.

Icons: lucide outline style, 1.5px stroke, 16-18px, currentColor inheritance. Play/pause/skip icons use fill-current, functional icons stay outline.

Motion: 150/200/250ms only, cubic-bezier(0.4,0,0.2,1). No bounce, spring, zoom, rotation.

Premium Hi-Fi desktop music application, timeless, quiet, information-first, professional, warm, focused.
```

---

## 2. 场景化提示词

### 2.1 列表页（全部歌曲 / 最近播放）

```text
LUMO Design Language v2.0. A full-song list table view: 32px bold page title top-left, 12px mono metadata below (e.g. "643 首歌曲"), toolbar with search input (32px height, left search icon, placeholder) and list/grid view toggle.

Sticky 10px uppercase mono table header (# 序号 / 标题 / 艺术家 / 专辑 / 时长 / 格式). 40px-height rows, no horizontal lines, alternating via whitespace only. Current playing row: #F0ECE4 background, 2px orange left bar, semibold orange title, spinning loader icon in index column. Hover: #F5F2EB background, index number swaps to play icon, heart and more buttons fade in.

Footer: 11px mono muted status bar with border-top. Empty state: centered Music icon (32px disabled) + "没有找到歌曲".
```

### 2.2 专辑网格页

```text
LUMO Design Language v2.0. Album grid view: 32px bold "专辑" title, 12px mono "643 张专辑" metadata. 5-column responsive grid, 24px gap, 32px horizontal padding.

Each card: 1:1 square cover (rounded 10px, #FBFAF7 placeholder bg), 15px medium album title below, 13px muted "artist · year". On hover: dark 20% overlay, centered 40px circular orange play button with soft shadow (the only shadow exception). Double-click cover to play.

No card shadows by default. IntersectionObserver infinite scroll. Footer status bar.
```

### 2.3 专辑详情页

```text
LUMO Design Language v2.0. Album detail page: back button top-left (12px muted "返回" with arrow). Header: 180px square cover (rounded 10px) left, 28px bold album title + 14px artist + 11px mono uppercase metadata ("1983 · 8 TRACKS · 41 分钟") right. Two pill buttons: black "播放全部" (Play icon fill) and outlined "随机播放" (Shuffle icon).

1px divider below header. Track list: 40px rows, sticky 10px mono header, current playing row with 2px orange left bar. No horizontal lines. Footer status.
```

### 2.4 Inspector Now Playing

```text
LUMO Design Language v2.0. Right inspector panel (360px, #F7F5F1 bg). Top: two centered tabs "正在播放" / "播放列表" with 2px orange underline on active, 1px divider below.

Now Playing: large square cover (aspect-square, rounded 10px), 18px bold title with heart (18px, orange fill if favorited) and more buttons right, 14px artist, 13px muted album, 10px mono uppercase metadata line.

Lyrics section: "Lyrics" 10px uppercase semibold label, 13px lines with 1.8 leading. Current line: orange medium. Past lines: muted. Future lines: secondary. Click to seek. Auto-scroll to center active line.
```

### 2.5 Playback Bar

```text
LUMO Design Language v2.0. Bottom playback bar (110px, #F7F5F1 bg). Three columns:

Left (280px): 56px square cover (rounded 6px) + 13px semibold title + 11px muted "artist · album" + 9px mono uppercase format + 16px-tall deterministic waveform (48 bars, 2px wide, orange=played, muted-30%=unplayed).

Center (flex-1): Transport row — 16px mode toggle (orange when active), 18px skip-back (black, hover orange), 48px circular play/pause button (black bg #111, white icon, 20px fill), 18px skip-forward, 16px more. Below: 10px mono time labels + 3px progress track (border-solid bg, orange fill, 10px orange draggable dot on hover).

Right: physical volume knob (52px, 11 ticks ring -135° to +135°, orange=active ticks, inset shadow knob body, orange pointer, "Volume" 9px bold uppercase label, 8px mono "0 / 45 / 100") + Output selector (9px label + 12px medium "Built-in Output" with chevron).
```

### 2.6 暗色模式

```text
LUMO Design Language v2.0, DARK MODE. Same layout, warm-dark palette (NOT cold zinc): canvas #1C1A17, content #23211D, divider #36332D, primary text #F2EFE9 (warm white), secondary #A9A49A, muted #7A756B. Accent #E28A23 unchanged. Hover backgrounds use rgba(255,255,255,0.05). Shadows deeper (rgba(0,0,0,0.4)). All other rules identical.
```

---

## 3. 生成规则

### 必须遵循

1. **仅用上述调色板**，不引入新颜色
2. **Accent 仅用于允许场景**（playing/selected/progress/active/focus/heart）
3. **Transport Play 用黑底**，不用 Accent
4. **1px Divider 分区**，不用 Card / 阴影 / 渐变
5. **字号来自阶梯**（9/10/11/12/13/14/15/18/22/28/32）
6. **间距来自 8pt**（8/16/24/32/48/64/80）
7. **圆角来自阶梯**（6/8/10/16/full）
8. **Lucide 风格图标**，1.5px stroke
9. **动效 150/200/250ms**，无 bounce/spring
10. **暗色用暖深**（#1C1A17），不用冷灰

### 禁止

- Glassmorphism / 毛玻璃
- Neumorphism / 凸起凹陷
- Aurora / Mesh 渐变
- 多个强调色
- Card 阴影分层
- 大色块 Accent 背景
- Bounce / Spring 动效
- 填充式拟物图标
- 冷灰暗色（zinc/neutral/slate）

---

## 4. 验证清单

生成后自检：

- [ ] 整个界面是否只有一种 Accent (#E28A23)？
- [ ] Transport Play 按钮是否黑色（不是 Accent）？
- [ ] 区域分隔是否用 1px Divider（不是 Card/阴影）？
- [ ] 字号是否来自 9-32 阶梯？
- [ ] 间距是否 8pt 倍数？
- [ ] 图标是否 lucide 线框 1.5px stroke？
- [ ] 暗色是否暖深 (#1C1A17)？
- [ ] 是否无渐变 / 无毛玻璃 / 无拟物？
- [ ] 是否无 bounce / spring 动效？
- [ ] 是否每视图最多一个 Accent 焦点？

---

*End of AI Prompt Specification.*
