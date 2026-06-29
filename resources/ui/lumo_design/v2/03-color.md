# 03 Color System

> **One Accent Rule.** 整个界面仅允许一种主强调色。

本文定义 LUMO 的完整调色板、用法规则、Accent 边界、状态色与暗色模式。所有色值以 [01 Tokens](01-tokens.md) 为 canonical 注册表。

---

## 1. 调色板

### 1.1 亮色（Light）

```
Canvas    #F7F5F1   ████████  应用底色（暖白）
Content   #FBFAF7   ████████  内容区底色（更暖更亮）
Divider   #E8E5DE   ████████  分割线 / 实线边框
Primary   #111111   ████████  主文字 / Primary 按钮底
Secondary #5F5F5F   ████████  副文字
Muted     #8B8B8B   ████████  元数据 / caption
Disabled  #BDBDBD   ████████  禁用 / 空态图标
Accent    #E28A23   ████████  唯一强调色（暖橘）
```

### 1.2 暗色（Dark）

```
Canvas    #1C1A17   ████████  应用底色（暖深，不是冷 zinc）
Content   #23211D   ████████  内容区底色
Divider   #36332D   ████████  分割线 / 实线边框
Primary   #F2EFE9   ████████  主文字（暖白）
Secondary #A9A49A   ████████  副文字
Muted     #7A756B   ████████  元数据
Disabled  #514E47   ████████  禁用
Accent    #E28A23   ████████  暗色下不变
```

**暗色原则**：偏暖的深色（#1C1A17 带 8° 黄），**不是**冷的 `zinc-900` / `neutral-900`。保持 Warm Industrial 调性。

---

## 2. 色彩层级用法

### 2.1 背景层级

```
Canvas  →  应用骨架（Sidebar / TopBar / Playback / Inspector）
   ↓
Content  →  内容区（仅 MainContent 内部）
   ↓
Hover / Selected  →  交互态覆盖
```

**色温微差分层**：Content `#FBFAF7` 比 Canvas `#F7F5F1` 略亮略暖，无需阴影即产生层次。这是 LDL 的核心深度策略。

### 2.2 文字层级

| 层级 | Token | 用途 | 字号要求 |
|---|---|---|---|
| 1 Primary | `--text-primary` | 标题、曲名、主操作 | 任意 |
| 2 Secondary | `--text-secondary` | 副标题、艺术家、专辑 | ≥ 13px |
| 3 Muted | `--text-muted` | 元数据、caption、计数 | **≥ 14px** 或非正文 |
| 4 Disabled | `--text-disabled` | 禁用文字、空态图标 | 仅非关键信息 |

> ⚠️ Muted `#8B8B8B` 对 Canvas 对比度 3.9:1，**正文不达 AA（4.5:1）**。仅用于 ≥14px 的辅助文字或非文字图形。小字用 Secondary。

---

## 3. Accent 使用边界

**Accent `#E28A23` 是整个系统唯一强调色。** 严格遵守以下边界：

### ✅ 允许（ALLOWED）

| 场景 | 用法 | 示例 |
|---|---|---|
| 当前播放指示 | 行左侧 2px 竖条、曲名着色、序号图标 | Song Row / Queue Item |
| 当前选中 | Sidebar Item 指示点（6px 圆点）、图标着色 | Sidebar |
| Tab 激活 | 下划线 2px、文字着色 | Inspector Tab |
| Progress | 进度条填充、波形已播放段、旋钮刻度已激活段 | Playback |
| Active 状态 | 夜间模式按钮激活图标 | TopBar |
| Focus | 输入框聚焦描边 `border-brand-orange/50` | Search Input |
| 收藏 | Heart 图标 `fill-current` | Song Row / Inspector |
| Hover 强调 | 文字 `hover:text-brand-orange` | Skip 按钮、Output 选择器 |

### ❌ 禁止（FORBIDDEN）

| 场景 | 原因 | 应改用 |
|---|---|---|
| Transport 主按钮（Play）底色 | 违反 One Accent；Transport 是中性控件 | `bg-text-primary`（黑） |
| 大色块背景 | 抢注意力，违反 Content First | — |
| 正文文字色 | 对比度不足，违反 A11y | `--text-primary` |
| 多个 Accent 同框 | 违反 One Accent | 每视图仅一个焦点 |
| 普通按钮 Hover 背景 | 太抢眼 | `--bg-hover` / `--btn-hover` |

### Accent 透明度变体

| 写法 | 用途 |
|---|---|
| `bg-brand-orange` | 实色：指示点、进度条 |
| `bg-brand-orange/10` (亮) / `/16` (暗) | Active 背景：夜间模式按钮激活底 |
| `border-brand-orange/50` | Focus 描边 |
| `text-brand-orange` | 文字着色：当前播放曲名、激活图标 |

---

## 4. 状态色（v2.0 新增）

LUMO 是音乐播放器，需要表达扫描 / 同步 / 错误 / 丢失等状态。状态色**低饱和**，不抢 Accent，仅用于状态点与图标。

| Token | 亮色 | 暗色 | 语义 | 用法 |
|---|---|---|---|---|
| `--status-info` | `#4A7FB8` | `#6B9FD4` | 扫描中、WebDAV 同步中 | 6px 圆点、Spinner、边框 |
| `--status-success` | `#5B8C5A` | `#7AB07A` | 完成、在线、已索引 | 6px 圆点、图标 |
| `--status-warning` | `#C08A3E` | `#D9A85C` | 文件丢失、格式不支持 | 6px 圆点、图标 |
| `--status-error` | `#C24E4E` | `#D96B6B` | 播放失败、扫描错误 | 6px 圆点、Toast 图标 |

### 状态点规范

```
6px 圆点 · radius-full · 状态色实色
可选：外部 2px 同色 /20 透明环（呼吸效果，仅 info）
```

### 禁止

- ❌ 状态色用于大色块背景
- ❌ 状态色用于正文文字（对比度不足）
- ❌ 状态色与 Accent 同时出现在同一控件

---

## 5. 交互态色

| 状态 | 亮色 | 暗色 | 用法 |
|---|---|---|---|
| Hover（通用） | `--bg-hover` (rgba 0.04) | `rgba(255,255,255,0.05)` | IconButton、TopBar 按钮 |
| Hover（按钮） | `--btn-hover` `#F1EFE9` | `#2C2924` | Ghost Button hover |
| Hover（列表） | `--list-hover` `#F5F2EB` | `#2A2722` | Song Row、Sidebar Item |
| Selected | `--list-selected` `#F0ECE4` | `#322E28` | Sidebar 当前项、当前播放行 |
| Active（Accent） | `--bg-active` (Accent /10) | (Accent /16) | 夜间模式按钮激活 |

**层级强度**：Hover < Selected < Active。Hover 最淡，Selected 明显但安静，Active 带 Accent 暖意。

---

## 6. 暗色模式

### 6.1 切换机制

- `html` 元素 `data-theme="dark"` 属性切换
- `style.css` 中 `[data-theme="dark"] { ... }` 覆盖 `:root` token
- 过渡：`background-color 0.2s ease-out, color 0.2s ease-out`（`body` 上）

### 6.2 暗色调性原则

1. **暖深不冷**：Canvas `#1C1A17`（带黄），禁用 `zinc-900` `#18181B`（冷紫）
2. **Accent 不变**：`#E28A23` 在暖深色上可见度足够，不调亮
3. **透明度调整**：Hover / Active 的透明度在暗色下略增（0.04 → 0.05，/10 → /16），补偿深色背景的对比衰减
4. **边框更淡**：`--border-color` 暗色用 `rgba(255,255,255,0.07)`，比亮色 `rgba(232,229,222,0.7)` 更含蓄

### 6.3 暗色独有注意

- AlbumCard Hover 浮层的 `shadow-overlay` 在暗色下加深（`rgba(0,0,0,0.4)`）
- 旋钮 `--shadow-knob-inset` 在暗色下加深（`rgba(0,0,0,0.3)`）
- `--text-disabled` 暗色 `#514E47` 极暗，仅用于非关键图形，不用于文字

---

## 7. 对比度要求（WCAG AA）

| 组合 | 对比度 | 等级 |
|---|---|---|
| Primary on Canvas (亮) | 18.9:1 | AAA |
| Secondary on Canvas (亮) | 7.1:1 | AAA |
| Muted on Canvas (亮) | 3.9:1 | AA Large（≥14px 或粗体） |
| Accent on Canvas (亮) | 3.4:1 | AA Large（仅≥18px 或图标） |
| Primary on Dark Canvas | 16.8:1 | AAA |

**规则**：

- 正文（< 14px）必须用 Primary 或 Secondary
- Muted 仅用于 ≥14px 的辅助文字，或非文字图形（图标、计数）
- Accent 不用于正文文字；用于曲名时需 ≥13px + semibold（视觉补偿）
- Disabled 不参与对比度要求（仅非关键信息）

> 详见 [08 Accessibility](08-accessibility.md)。

---

## 8. 色彩 Do & Don't

### Do

- ✅ 用 Canvas / Content 色温微差分层，不用阴影
- ✅ 每个视图只有一个 Accent 焦点
- ✅ 状态色仅用于状态点与图标
- ✅ 正文用 Primary / Secondary，Muted 留给辅助

### Don't

- ❌ 用 Accent 做 Transport 主按钮
- ❌ 用 #999999（v1.0 遗留，对比度不足）
- ❌ 在暗色用冷灰（zinc / neutral / slate）
- ❌ 同时出现两个强调色
- ❌ 用背景色差替代 Divider 作为主要分隔手段

---

## 9. 色彩使用速查

```
背景：     bg-bg-canvas / bg-bg-content / bg-bg-hover / bg-list-hover / bg-list-selected
文字：     text-text-primary / text-text-secondary / text-text-muted / text-text-disabled
边框：     border-border-color / border-border-solid
强调：     bg-brand-orange / text-brand-orange / border-brand-orange
状态：     bg-status-info / bg-status-success / bg-status-warning / bg-status-error
反色：     text-text-inverse（在 Primary 黑底上）
```

---

*End of Color System.*
