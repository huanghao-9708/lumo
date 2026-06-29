# Folder Tree

> 文件夹导航页的双栏树状工作区规范。

---

## 1. Overview

文件夹视图专为本地 Hi-Fi 音乐发烧友设计的物理文件管理视图。采用**双栏布局**：左栏为多层级文件夹树，右栏为当前文件夹内的歌曲列表。

---

## 2. Anatomy

```
┌─────────────────────────────────────────────────────────┐
│  文件浏览器                    │  Local Music  /  Jazz  │
│  ┌─────────────────────────┐   │                          │
│  │  Local Music ▼          │   │  #  │  ♥  │ 标题  │ ...  │
│  │─────────────────────────│   │  01 │     │ Song  │      │
│  │  ▸  Music               │   │  02 │     │ Song  │      │
│  │    ▸  Classical         │   │  ...                      │
│  │      ▸  Beethoven       │   │                          │
│  │      ▸  Mozart          │   │                          │
│  │    ▸  Jazz              │   │                          │
│  └─────────────────────────┘   └──────────────────────────┘
│  ← 280px →                    ← flex-1 →
└─────────────────────────────────────────────────────────┘
```

---

## 3. 左栏：文件夹树

### 元素规范

| 元素 | 尺寸 | 样式 |
|---|---|---|
| 折叠箭头 | 14×14 | `ChevronRight` / `rotate-90`（展开时） |
| 文件夹图标 | 16×16 | `Folder`（Lucide，1px stroke） |
| 文件夹名 | 13px Primary | `truncate` |
| 行高 | ~32px | `py-[6px]` + icon |
| 选中态 | `bg-list-selected` | 当前打开的文件夹 |

### 交互模型

1. 用户选择数据源（顶部的 `<select>` 下拉框）
2. 自动加载根目录内容（`fetchFolderContents(sourceId, '')`）
3. 点击文件夹 → 展开子目录（`expandedPaths` set 控制展开状态）
4. 展开新文件夹时调用 `fetchFolderContents(sourceId, path)` 获取子级
5. 选中态跟随导航（`selectedFolderPath`）

> **注意**：当前 `FolderEntry` 是扁平列表，没有嵌套结构。树状展开通过 `expandedPaths: Set<string>` 在组件内维护，每次展开/点击都会重新 fetch 数据。未来版本可引入层级缓存。

---

## 4. 右栏：歌曲列表

右栏展示当前文件夹下所有可播放的音频文件（`is_dir === false && track !== null`），采用标准 Song Row 布局。

### 列定义

| 列 | 显示规则 | 说明 |
|---|---|---|
| # | 始终 | 序号 / 播放图标 |
| ♥ | 始终 | 收藏 |
| 标题 | 始终 | 曲名 |
| 艺术家 | `sm` (≥640) | — |
| 大小 | `md` (≥768) | 当前返回 `--`（占位） |
| 时长 | `md` (≥768) | — |
| ⋯ | 始终 | Hover 显示 |

### 面包屑

```
Local Music  /  Music  /  Jazz
```

- 根节点可点击（回到根目录）
- 中间路径可点击（跳到该层级）
- 11px `font-mono` `text-text-muted`

---

## 5. 数据源选择

```html
<select class="w-full h-[32px] px-2 text-[12px] bg-bg-canvas border border-border-color rounded-[6px] text-text-primary">
  <option v-for="s in sources" :value="s.id">{{ s.name }}</option>
</select>
```

| 属性 | 值 |
|---|---|
| 高度 | 32px |
| 字号 | 12px |
| 圆角 | `rounded-[6px]` |
| 背景 | `bg-bg-canvas` |
| 边框 | `border-border-color` |

---

## 6. Tokens used

| Token | 用途 |
|---|---|
| `--bg-canvas` | 树背景 |
| `--bg-content` | 右栏背景 |
| `--border-color` | 双栏 Divider + 边框 |
| `--list-selected` | 选中文件夹 |
| `--list-hover` | 文件夹 Hover |
| `--text-primary` | 文件夹/曲名 |
| `--text-muted` | 图标/面包屑/辅助文字 |

---

## 7. Do & Don't

### Do

- ✅ 双栏分割，左栏固定 280px
- ✅ 面包屑导航可点击跳转层级
- ✅ 自动选择第一个数据源
- ✅ 空态明确区分"无数据源"和"无歌曲"

### Don't

- ❌ 在左栏显示歌曲（仅在右栏显示）
- ❌ 使用复杂嵌套组件（当前用扁平列表）
- ❌ 在文件夹树里播放音乐（在右栏双击播放）

---

*End of Folder Tree.*
