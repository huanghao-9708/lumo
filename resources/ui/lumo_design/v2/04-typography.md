# 04 Typography

> **Typography Before Decoration.** 优先用字体、字号、字重建立层级，不靠阴影与颜色。

LDL 是 **Typography-Driven** 的设计系统。视觉层级主要由字号对比、字重差异、字距控制建立，颜色和背景只起辅助作用。

---

## 1. 字族 Font Family

| Token | 栈 | 用途 |
|---|---|---|
| `--font-sans` | `"Inter", "SF Pro Display", "Helvetica Now", "MiSans", "HarmonyOS Sans", "OPPOSans", sans-serif` | 所有正文、标题、UI 文字 |
| `--font-mono` | `"IBM Plex Mono", "JetBrains Mono", monospace` | 数字、元数据、技术信息、Logo 副标 |

### 字族选择理由

- **Inter**：英文 UI 黄金标准，字面宽均匀，小字号清晰
- **MiSans / HarmonyOS Sans**：中文回退，现代无衬线，与 Inter 视觉重量匹配
- **IBM Plex Mono**：等宽，数字对齐优秀，带工业感（呼应 Braun / Teenage Engineering）

### 字体加载策略

- 优先使用系统已安装字体，减少打包体积
- Inter / IBM Plex Mono 通过 CSS `@font-face` 或 `fontsource` 按需加载（未来优化）
- 中文依赖系统 MiSans / HarmonyOS Sans，不打包中文字体（体积过大）

---

## 2. 字号阶梯 Type Scale

LUMO 字号采用 **4pt 模数**，11 档：

| Token | 字号 | 用途 | 字重 | 行高 |
|---|---|---|---|---|
| `--text-9` | 9px | 版本号、旋钮刻度、Output 标签 | Bold | tight |
| `--text-10` | 10px | 表头、Section 标题、uppercase 元数据 | Semibold / Medium | snug |
| `--text-11` | 11px | 计数徽章、Footer Status、加载提示 | Mono Regular | normal |
| `--text-12` | 12px | 搜索框、Empty State、Caption | Regular | normal |
| `--text-13` | 13px | **正文默认**：Song Row、Sidebar、Inspector | Regular / Medium | normal |
| `--text-14` | 14px | Inspector 副标题、专辑详情艺术家 | Regular | snug |
| `--text-15` | 15px | Album Card 标题 | Medium | tight |
| `--text-18` | 18px | Inspector Now Playing 曲名 | Bold | tight |
| `--text-22` | 22px | H2 | Bold | tight |
| `--text-28` | 28px | H1（专辑详情页标题） | Bold | tight |
| `--text-32` | 32px | **Page Title**（列表页顶部） | Bold | tight |

### Editorial 模数比

LDL 借鉴杂志排版，字号跳跃明显以建立强对比：

```
Page Title 32  ─┐
                │ 1.14x
H1         28  ─┤
                │ 1.27x
H2         22  ─┤
                │ 1.22x
Body       18  ─┤
                │ 1.2x
           15  ─┤
                │ 1.15x
           14  ─┤
                │ 1.08x
Default    13  ─┤ ← 正文基准
                │ 1.08x
Caption    12  ─┘
```

**规则**：相邻层级比 1.08–1.27x，跨越式对比（32 → 13 = 2.46x）用于 Page Title 与正文的强分隔。

---

## 3. 字重 Weight

| 名称 | 值 | 用途 |
|---|---|---|
| Regular | 400 | 正文默认 |
| Medium | 500 | Selected 项文字、Tab 激活、强调正文 |
| Semibold | 600 | 当前播放曲名、Primary 按钮文字 |
| Bold | 700 | Page Title、H1、H2、Logo、Section 标题 |

### 字重使用规则

- **不跳级**：正文 Regular → 强调 Medium，不直接到 Bold
- **不滥用 Bold**：Bold 仅用于标题与 Logo，正文最高 Semibold
- **Selected 用 Medium**：选中项文字 `font-medium`，不用 Bold（保持安静）

---

## 4. 行高 Line Height

| Token | 值 | 用途 |
|---|---|---|
| `--leading-tight` | 1.1 | 大标题（32 / 28 / 22） |
| `--leading-snug` | 1.3 | 紧凑标题（18 / 15） |
| `--leading-normal` | 1.5 | 默认正文（13 / 14）— `body` 全局设置 |
| `--leading-relaxed` | 1.625 | 元数据 mono |
| `--leading-loose` | 1.8 | Lyrics 行间距 |

> `body { line-height: 1.5; }` 全局默认，组件按需覆盖。

---

## 5. 字距 Letter Spacing

| Token | 值 | 用途 |
|---|---|---|
| `--tracking-tight` | -0.01em | 大标题（32 / 28）`tracking-tight` |
| `--tracking-normal` | 0 | 正文默认 |
| `--tracking-wide` | 0.05em | Caption |
| `--tracking-wider` | 0.1em | Section 标题（`tracking-wider`） |
| `--tracking-widest` | 0.15em | Logo、uppercase 元数据（`tracking-widest` / `tracking-[0.15em]`） |

### uppercase 配对规则

**uppercase 文字必须配 `tracking-wider` 或 `tracking-widest`**，否则字母挤在一起难读。

```html
<!-- ✅ 正确 -->
<h2 class="text-[10px] font-semibold uppercase tracking-widest">Library</h2>
<p class="text-[10px] font-mono uppercase tracking-wider">1983 · 8 TRACKS · 41 MIN</p>

<!-- ❌ 错误：uppercase 无 tracking -->
<h2 class="text-[10px] uppercase">Library</h2>
```

---

## 6. 等宽字体（Mono）用法

`IBM Plex Mono` 用于所有**数字 / 技术信息 / 元数据**，提供工业感与对齐：

| 场景 | 字号 | 示例 |
|---|---|---|
| 时长 | 12 | `03:42` |
| 序号 | 12 | `01` `02`… |
| 计数徽章 | 11 | `643` `982` |
| 元数据行 | 10 | `1983 · 8 TRACKS · 41 MIN` |
| Footer Status | 11 | `643 首歌曲` |
| 比特率 | 10 | `24bit / 96kHz` |
| 格式 | 10 | `FLAC` |
| 版本号 | 9 | `v1.0.0` |
| 音量 | 8 | `0  45  100` |

### Mono 使用规则

1. **数字一律 `tabular-nums`**：`class="font-mono tabular-nums"`，确保等宽对齐
2. **技术信息用 `uppercase`**：格式、比特率、TRACKS、Volume 标签
3. **正文不用 Mono**：歌名、艺术家、专辑名用 sans
4. **元数据 mono 行配 `tracking-wider`**：`uppercase tracking-wider leading-relaxed`

```html
<!-- 标准元数据行 -->
<p class="text-[10px] text-text-muted font-mono uppercase tracking-wider leading-relaxed">
  {{ fileInfoText }}
</p>
```

---

## 7. 排版层级应用

### 7.1 Page Title（列表页顶部）

```html
<h1 class="text-[32px] font-bold text-text-primary tracking-tight leading-none mb-2">
  {{ pageTitle }}
</h1>
<p class="text-[12px] text-text-muted leading-relaxed font-mono">{{ metaText }}</p>
```

- 32px Bold + `tracking-tight` + `leading-none`
- 下方 12px mono 元数据

### 7.2 H1（详情页）

```html
<h1 class="text-[28px] font-bold text-text-primary tracking-tight leading-tight mb-1">
  {{ album.title }}
</h1>
```

### 7.3 Section 标题（Sidebar 分组）

```html
<h2 class="px-3 text-[10px] font-semibold text-text-muted mb-2 uppercase tracking-widest">
  Library
</h2>
```

### 7.4 正文行（Song Row）

```html
<span class="text-[13px] text-text-primary font-medium truncate">{{ song.title }}</span>
<span class="text-[13px] text-text-secondary truncate">{{ song.artist }}</span>
<span class="text-[12px] font-mono text-text-muted tabular-nums">{{ song.duration }}</span>
```

### 7.5 当前播放强化

```html
<span class="text-[13px] font-semibold text-brand-orange">{{ song.title }}</span>
```

- Regular → Semibold（不跳到 Bold）
- 颜色 → Accent（仅当前播放行）

---

## 8. 中英文混排

- 中文与英文/数字之间**不强制加空格**（Inter 与 MiSans 间距已足够）
- 中文标题用 Bold 时，英文部分同样 Bold（不混用字重）
- 中文不用斜体（`italic`），斜体仅用于英文歌词暂无提示等极少场景

---

## 9. 截断与换行

| 场景 | 处理 |
|---|---|
| Song Row 标题 | `truncate`（单行省略） |
| Album Card 标题 | `truncate` |
| Inspector 曲名 | `truncate` |
| 歌词 | 多行，`leading-loose`，不截断 |
| 元数据 | 单行，`truncate` |

```html
<span class="text-[13px] truncate block">{{ song.title }}</span>
```

---

## 10. Do & Don't

### Do

- ✅ 用字号对比建立层级（32 → 13）
- ✅ Mono + tabular-nums + uppercase 用于数字与技术信息
- ✅ uppercase 配 tracking-wider / tracking-widest
- ✅ 当前播放用 Semibold + Accent，不用 Bold

### Don't

- ❌ 用颜色替代字号建立层级
- ❌ 正文用 Bold（最高 Semibold）
- ❌ uppercase 不加 tracking
- ❌ 数字不用 mono / tabular-nums
- ❌ 在同一视图用超过 3 种字重

---

*End of Typography.*
