# 01 Design Tokens

> **Everything is Tokenized.** 禁止 Magic Number，禁止 Hard Code。

本文是 LUMO 所有视觉值的**唯一注册表**。任何颜色、字号、圆角、间距、动画时长必须先在此注册 token，再在 `style.css` 的 `:root` / `@theme` 中落地，最后在组件中以 Tailwind utility 引用。

---

## 1. Token 命名规则

| 前缀 | 含义 | 示例 |
|---|---|---|
| `--bg-*` | 背景色 | `--bg-canvas` `--bg-content` `--bg-hover` `--bg-active` |
| `--text-*` | 文字色 | `--text-primary` `--text-secondary` `--text-muted` `--text-disabled` `--text-inverse` |
| `--border-*` | 边框/分割线 | `--border-solid` `--border-color` |
| `--btn-*` | 按钮专用 | `--btn-hover` |
| `--list-*` | 列表行专用 | `--list-hover` `--list-selected` |
| `--text-{N}` | 字号(px) | `--text-9` `--text-12` `--text-13` … `--text-32` |
| `--radius-{N}` | 圆角(px) | `--radius-6` `--radius-8` `--radius-10` `--radius-16` |
| `--space-{N}` | 间距(px) | `--space-2` `--space-4` … `--space-64`（8pt 阶梯） |
| `--duration-*` | 动画时长 | `--duration-150` `--duration-200` `--duration-250` |
| `--shadow-*` | 阴影（例外） | `--shadow-overlay` |
| `--scrollbar-*` | 滚动条 | `--scrollbar-thumb` `--scrollbar-thumb-hover` |
| `--status-*` | 状态色 | `--status-info` `--status-success` `--status-warning` `--status-error` |

**规则**：

1. token 名用 kebab-case，值用 CSS 变量
2. 暗色 token **同名覆盖**，不新增 `--*-dark` 后缀
3. 字号 / 圆角 / 间距 token 直接用像素数字后缀，便于记忆
4. 新增 token 必须先在本文注册，再在 `style.css` 落地

---

## 2. Color Tokens

### 2.1 背景色 Background

| Token | 亮色 | 暗色 | 用途 |
|---|---|---|---|
| `--bg-canvas` | `#F7F5F1` | `#1C1A17` | 应用底色（Sidebar / TopBar / Playback / Inspector） |
| `--bg-content` | `#FBFAF7` | `#23211D` | 内容区底色（仅 Content Area） |
| `--bg-hover` | `rgba(0,0,0,0.04)` | `rgba(255,255,255,0.05)` | 通用 Hover（按钮、图标按钮） |
| `--bg-active` | `rgba(226,138,35,0.10)` | `rgba(226,138,35,0.16)` | Accent 激活态背景（夜间模式按钮） |
| `--btn-hover` | `#F1EFE9` | `#2C2924` | 按钮专用 Hover |
| `--list-hover` | `#F5F2EB` | `#2A2722` | 列表行 Hover |
| `--list-selected` | `#F0ECE4` | `#322E28` | 列表行 Selected |

### 2.2 文字色 Text

| Token | 亮色 | 暗色 | 用途 | 对比度(vs canvas) |
|---|---|---|---|---|
| `--text-primary` | `#111111` | `#F2EFE9` | 主文字、标题 | 18.9:1 ✓ AAA |
| `--text-secondary` | `#5F5F5F` | `#A9A49A` | 副文字、艺术家名 | 7.1:1 ✓ AAA |
| `--text-muted` | `#8B8B8B` | `#7A756B` | 元数据、caption、计数 | 3.9:1（大字 AA，正文需 ≥14px） |
| `--text-disabled` | `#BDBDBD` | `#514E47` | 禁用态、空态图标 | 1.7:1（仅非文字图形） |
| `--text-inverse` | `#FFFFFF` | `#111111` | 反色（Primary 按钮文字） | — |

> **v1.0 修正**：`--text-secondary` 由 `#5E5E5E` 改为 `#5F5F5F`（代码现状，差异微小，以代码为准）；`--text-muted` 由 `#999999` 改为 `#8B8B8B`（#999999 对比度 2.8:1 不达 AA）。

### 2.3 边框 / 分割线 Divider

| Token | 亮色 | 暗色 | 用途 |
|---|---|---|---|
| `--border-solid` | `#E8E5DE` | `#36332D` | 实线边框（输入框、按钮描边、进度条底） |
| `--border-color` | `rgba(232,229,222,0.7)` | `rgba(255,255,255,0.07)` | 一级 Divider（区域分隔，半透明更柔和） |

### 2.4 Accent

| Token | 亮色 | 暗色 | 用途 |
|---|---|---|---|
| `--brand-orange` | `#E28A23` | `#E28A23` | 唯一强调色，暗色下**不变** |

**Accent 使用边界**：见 [03 Color System](03-color.md) §3。

### 2.5 状态色 Status（v2.0 新增）

| Token | 亮色 | 暗色 | 用途 |
|---|---|---|---|
| `--status-info` | `#4A7FB8` | `#6B9FD4` | 扫描中、WebDAV 同步中 |
| `--status-success` | `#5B8C5A` | `#7AB07A` | 完成、在线 |
| `--status-warning` | `#C08A3E` | `#D9A85C` | 警告、文件丢失 |
| `--status-error` | `#C24E4E` | `#D96B6B` | 错误、播放失败 |

**规则**：状态色**低饱和**，不抢 Accent；仅用于状态点（6px 圆点）、边框指示、Toast 图标，**不用于大色块**。

### 2.6 滚动条

| Token | 亮色 | 暗色 |
|---|---|---|
| `--scrollbar-thumb` | `#D8D4CB` | `#3F3C35` |
| `--scrollbar-thumb-hover` | `#BCB7AB` | `#534F47` |

---

## 3. Typography Tokens

### 3.1 字号阶梯

LUMO 字号采用 **4pt 模数**（不是 8pt，因为 12/14/16/20 是桌面正文常用值，强行 8pt 会跳到 16/24 过于稀疏）。

| Token | 值 | 用途 |
|---|---|---|
| `--text-9` | 9px | 版本号、旋钮刻度数字、Output 标签 |
| `--text-10` | 10px | 表头、Section 标题、Lyrics 标签、元数据 uppercase |
| `--text-11` | 11px | 计数徽章、Footer Status、加载提示 |
| `--text-12` | 12px | 搜索框、Empty State、空态副文、Caption |
| `--text-13` | 13px | **正文默认**：Song Row 标题、Sidebar Item、Inspector 信息 |
| `--text-14` | 14px | Inspector 副标题、专辑详情艺术家 |
| `--text-15` | 15px | Album Card 标题 |
| `--text-18` | 18px | Inspector Now Playing 曲名 |
| `--text-22` | 22px | H2 |
| `--text-28` | 28px | H1（专辑详情页） |
| `--text-32` | 32px | **Page Title**（列表页顶部标题） |

### 3.2 字族

| Token | 值 |
|---|---|
| `--font-sans` | `"Inter", "SF Pro Display", "Helvetica Now", "MiSans", "HarmonyOS Sans", "OPPOSans", sans-serif` |
| `--font-mono` | `"IBM Plex Mono", "JetBrains Mono", monospace` |

### 3.3 字重

| 名称 | 值 | 用途 |
|---|---|---|
| Regular | 400 | 正文 |
| Medium | 500 | Selected 项、强调正文、Tab 激活 |
| Semibold | 600 | 当前播放曲名、按钮文字 |
| Bold | 700 | Page Title、Logo、Section 标题 |

### 3.4 行高 / 字距

| Token | 值 | 用途 |
|---|---|---|
| `--leading-tight` | 1.1 | 大标题（Page Title / H1 / H2） |
| `--leading-snug` | 1.3 | 紧凑标题（Inspector 曲名） |
| `--leading-normal` | 1.5 | 默认正文 |
| `--leading-relaxed` | 1.625 | 元数据 mono |
| `--leading-loose` | 1.8 | Lyrics 行间距 |
| `--tracking-tight` | -0.01em | 大标题 |
| `--tracking-normal` | 0 | 正文 |
| `--tracking-wide` | 0.05em | Caption |
| `--tracking-wider` | 0.1em | Section 标题 |
| `--tracking-widest` | 0.15em | Logo、uppercase 元数据 |

> 详见 [04 Typography](04-typography.md)。

---

## 4. Radius Tokens

圆角采用**独立模数**（6/8/10/16），不强制 8pt：

| Token | 值 | 用途 |
|---|---|---|
| `--radius-6` | 6px | 小控件：Sidebar Item、视图切换按钮、播放浮层小按钮 |
| `--radius-8` | 8px | 中控件：IconButton、输入框、视图切换容器 |
| `--radius-10` | 10px | 封面：Album Card、Now Playing 大封面、专辑详情封面 |
| `--radius-16` | 16px | 大容器（未来 Modal / Card） |
| `--radius-full` | 9999px | 胶囊按钮（Play All）、圆形按钮（Transport Play） |

---

## 5. Spacing Tokens

间距严格 **8pt Grid**：

| Token | 值 | 用途 |
|---|---|---|
| `--space-2` | 2px | 列表项之间微间距 |
| `--space-4` | 4px | 图标与文字微间距 |
| `--space-8` | 8px | 紧凑内边距、小间距 |
| `--space-12` | 12px | Sidebar Section 间距 |
| `--space-16` | 16px | 标准内边距、封面与文字间距 |
| `--space-20` | 20px | — |
| `--space-24` | 24px | Album Grid 封面间距、Section 间距 |
| `--space-32` | 32px | 大区间距 |
| `--space-40` | 40px | — |
| `--space-48` | 48px | 页面顶部留白（pt-12） |
| `--space-64` | 64px | — |
| `--space-80` | 80px | 空态垂直留白 |
| `--space-96` | 96px | — |

**布局专用常量**（非 token，写死在组件）：

| 常量 | 值 | 说明 |
|---|---|---|
| Sidebar width | 240px | 固定 |
| TopBar height | 60px | 固定 |
| Inspector width | 360px | 固定，可折叠 |
| Playback height | 110px | 固定 |

---

## 6. Motion Tokens

| Token | 值 | 用途 |
|---|---|---|
| `--duration-150` | 150ms | 快速：旋钮指针、Hover 图标显隐 |
| `--duration-200` | 200ms | 默认：颜色、背景、边框过渡 |
| `--duration-250` | 250ms | 慢速：Tab 切换、视图切换 |
| `--ease-smooth` | `cubic-bezier(0.4, 0, 0.2, 1)` | 唯一规范曲线 |

> 详见 [07 Motion](07-motion.md)。

---

## 7. Shadow Tokens（例外）

LDL 默认无阴影。以下为**受控例外**：

| Token | 亮色 | 暗色 | 用途 |
|---|---|---|---|
| `--shadow-overlay` | `0 4px 12px rgba(0,0,0,0.15)` | `0 4px 12px rgba(0,0,0,0.4)` | AlbumCard Hover 播放浮层按钮 |
| `--shadow-knob-inset` | `inset 0 1px 2px rgba(0,0,0,0.06)` | `inset 0 1px 2px rgba(0,0,0,0.3)` | 音量旋钮内凹质感 |

> 详见 [06 Elevation](06-elevation.md)。

---

## 8. z-index 阶梯

| Token | 值 | 用途 |
|---|---|---|
| `--z-base` | 0 | 默认 |
| `--z-sticky` | 10 | 表头 sticky、Inspector Tab 下划线 |
| `--z-sidebar` | 20 | — |
| `--z-topbar` | 30 | — |
| `--z-overlay` | 40 | Hover 浮层（AlbumCard 播放按钮） |
| `--z-modal` | 50 | Modal / Dialog（未来） |
| `--z-toast` | 60 | Toast（未来） |

---

## 9. Tailwind `@theme` 映射

`style.css` 中通过 `@theme` 将 CSS 变量映射为 Tailwind utility。**组件中只使用 Tailwind utility，不直接 `var(--*)`。**

```css
@theme {
  --font-sans: ...;
  --font-mono: ...;

  --color-brand-orange: var(--brand-orange);

  --color-bg-canvas:   var(--bg-canvas);
  --color-bg-content:  var(--bg-content);
  --color-bg-hover:    var(--bg-hover);
  --color-bg-active:   var(--bg-active);

  --color-btn-hover:     var(--btn-hover);
  --color-list-hover:    var(--list-hover);
  --color-list-selected: var(--list-selected);

  --color-text-primary:   var(--text-primary);
  --color-text-secondary: var(--text-secondary);
  --color-text-muted:     var(--text-muted);
  --color-text-disabled:  var(--text-disabled);
  --color-text-inverse:   var(--text-inverse);

  --color-border-color: var(--border-color);
  --color-border-solid: var(--border-solid);
}
```

**映射后的 utility 用法**：

| 写法 | 生成自 |
|---|---|
| `bg-bg-canvas` | `--color-bg-canvas` |
| `text-text-primary` | `--color-text-primary` |
| `bg-brand-orange` | `--color-brand-orange` |
| `border-border-color` | `--color-border-color` |

字号 / 圆角 / 间距目前直接在组件用 Tailwind 任意值（`text-[13px]` `rounded-[10px]`），未来注册为 `@theme` 阶梯后可改用 `text-text-13` `rounded-radius-10`。

---

## 10. 魔数 → Token 迁移表

下表列出当前代码中的魔数及其目标 token。**文档批准后，`style.css` 对齐轮次逐项迁移。**

| 当前魔数 | 出现位置 | 目标 token | 迁移后写法 |
|---|---|---|---|
| `text-[9px]` | Logo 版本号、旋钮标签 | `--text-9` | `text-text-9` |
| `text-[10px]` | 表头、Section 标题 | `--text-10` | `text-text-10` |
| `text-[11px]` | 计数、Footer | `--text-11` | `text-text-11` |
| `text-[12px]` | 搜索框、Empty State | `--text-12` | `text-text-12` |
| `text-[13px]` | Song Row、Sidebar | `--text-13` | `text-text-13` |
| `text-[14px]` | Inspector 副标题 | `--text-14` | `text-text-14` |
| `text-[15px]` | Album Card 标题 | `--text-15` | `text-text-15` |
| `text-[18px]` | Inspector 曲名 | `--text-18` | `text-text-18` |
| `text-[28px]` | H1 | `--text-28` | `text-text-28` |
| `text-[32px]` | Page Title | `--text-32` | `text-text-32` |
| `rounded-[6px]` | Sidebar Item | `--radius-6` | `rounded-radius-6` |
| `rounded-[8px]` | IconButton、输入框 | `--radius-8` | `rounded-radius-8` |
| `rounded-[10px]` | 封面 | `--radius-10` | `rounded-radius-10` |
| `shadow-lg` | AlbumCard 浮层 | `--shadow-overlay` | `shadow-shadow-overlay` |

---

## 11. token ↔ code 同步工作流

```
1. 在 01-tokens.md 注册新 token（本文档是 canonical）
2. 在 style.css :root 加亮色值
3. 在 style.css [data-theme="dark"] 加暗色值
4. 在 style.css @theme 加 Tailwind 映射
5. 在组件中用 Tailwind utility 引用
6. 删除旧的 magic number / hard code
```

**单向流**：文档 → CSS → 组件。代码不回写文档，文档变更后代码对齐。

---

*End of Tokens.*
