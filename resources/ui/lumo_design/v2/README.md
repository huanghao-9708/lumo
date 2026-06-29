# LUMO Design Language (LDL) v2.0

> **Warm Industrial Minimalism**
>
> *Quiet · Focused · Timeless*

---

## 这是什么

LDL v2.0 是 LUMO 产品的**唯一设计规范来源（Single Source of Truth）**。所有设计稿、组件、代码实现、AI 生成页面均必须遵循本规范。

v2.0 相对 v1.0 的主要变更见 [`00-foundation.md` Changelog](00-foundation.md#changelog)。

---

## 文档索引

### Foundation & Systems（主手册）

| 文件 | 内容 |
|---|---|
| [00-foundation.md](00-foundation.md) | 宪法10条 / 人格 / DNA / 目标 / 路线图 |
| [01-tokens.md](01-tokens.md) | 完整 token 表（亮+暗）+ 命名规则 + Tailwind 映射 + 魔数迁移 |
| [02-spatial.md](02-spatial.md) | 画布 / 五大区域 / 栅格模数 / 响应式 |
| [03-color.md](03-color.md) | 调色板 / 用法规则 / Accent 边界 / 状态色 / 暗色 / AA |
| [04-typography.md](04-typography.md) | 字族 / 字重 / 字号 / 行高 / 字距 / 等宽用法 |
| [05-iconography.md](05-iconography.md) | lucide / stroke / 尺寸 / currentColor / 线框 vs 填充 |
| [06-elevation.md](06-elevation.md) | z-index 阶梯 / 无阴影规则 / 受控例外 |
| [07-motion.md](07-motion.md) | 150/200/250 场景映射 / 规范曲线 / reduced-motion |
| [08-accessibility.md](08-accessibility.md) | 键盘 / focus ring / AA 对比 / aria |

### Components（组件规范）

每个组件文件统一结构：Overview / Anatomy / Sizes / States / Tokens used / Do & Don't / 参考代码。

| 文件 | 组件 |
|---|---|
| [09-components/button.md](09-components/button.md) | Primary / Secondary / Ghost / Transport Play |
| [09-components/icon-button.md](09-components/icon-button.md) | 32×32 / 28×28 / Active / 危险 / 视图切换组 |
| [09-components/input.md](09-components/input.md) | 搜索框 / Focus / 防抖 |
| [09-components/sidebar-item.md](09-components/sidebar-item.md) | NavItem / PlaylistItem / 计数徽章 / Accent 点 |
| [09-components/song-row.md](09-components/song-row.md) | 4 态 / 序号 / 收藏 / 虚拟列表 |
| [09-components/album-card.md](09-components/album-card.md) | 网格 / Hover 浮层（阴影例外） |
| [09-components/tab.md](09-components/tab.md) | 下划线 Accent Tab |
| [09-components/empty-and-loading.md](09-components/empty-and-loading.md) | 空态 / 加载 / 错误 / 无更多 |
| [09-components/playback-bar.md](09-components/playback-bar.md) | 进度条 / 波形 / 旋钮 / Transport |
| [09-components/inspector.md](09-components/inspector.md) | Now Playing / Queue / Lyrics |

### Reference

| 文件 | 内容 |
|---|---|
| [10-ai-prompt.md](10-ai-prompt.md) | AI 生成界面的提示词规范 |
| [11-brand.md](11-brand.md) | LUMO wordmark / 命名规范 |

---

## 快速入门

### 我是设计师

1. 读 [00-foundation.md](00-foundation.md) 理解宪法与哲学
2. 读 [01-tokens.md](01-tokens.md) 获取所有视觉值
3. 按 [03-color.md](03-color.md) / [04-typography.md](04-typography.md) 建立调色板与字号
4. 查 `09-components/` 对应组件规范

### 我是开发者

1. 读 [00-foundation.md](00-foundation.md) §9 Code Follows Design 理解信息流
2. 读 [01-tokens.md](01-tokens.md) §9 Tailwind `@theme` 映射，知道哪些 utility 可用
3. 查 `09-components/` 对应组件的"参考代码"段
4. 遵守 [01-tokens.md](01-tokens.md) §10 魔数迁移表，逐步消除 magic number

### 我在用 AI 生成 UI

1. 复制 [10-ai-prompt.md](10-ai-prompt.md) §1 基础提示词
2. 按场景追加 §2 场景化提示词
3. 生成后用 §4 验证清单自检

---

## token ↔ code 同步工作流

LDL 是 canonical，代码随后对齐。**单向流**：

```
1. 在 01-tokens.md 注册新 token（canonical）
2. 在 style.css :root 加亮色值
3. 在 style.css [data-theme="dark"] 加暗色值
4. 在 style.css @theme 加 Tailwind 映射
5. 在组件中用 Tailwind utility 引用
6. 删除旧的 magic number / hard code
```

代码不回写文档，文档变更后代码对齐。

---

## 当前代码对齐状态

v2.0 文档定稿后，`style.css` 需单独一轮 PR 对齐以下项：

- [ ] `--text-muted` 代码已是 `#8B8B8B`（v1.0 文档 #999999 已修正）
- [ ] `--text-secondary` 代码 `#5F5F5F`（文档对齐）
- [ ] 字号 token 注册（`--text-9` … `--text-32`）+ `@theme` 映射
- [ ] 圆角 token 注册（`--radius-6/8/10/16`）+ `@theme` 映射
- [ ] 状态色 token 注册（`--status-*`）+ `@theme` 映射
- [ ] 阴影 token 注册（`--shadow-overlay` / `--shadow-knob-inset`）
- [ ] z-index token 注册
- [ ] `prefers-reduced-motion` 媒体查询
- [ ] `button:focus-visible` Accent outline

> 详见 [01-tokens.md §10](01-tokens.md#10-魔数--token-迁移表)。

---

## 版本历史

| 版本 | 日期 | 说明 |
|---|---|---|
| v1.0 | 2026-06 | Foundation，存档于 `../LUMO Design Language（LDL） v1.0.md` |
| **v2.0** | **2026-06** | **当前版本。补齐暗色/Iconography/Elevation/A11y/Motion/状态色/10组件** |

---

## 相关文档

- [LUMO 产品愿景](../../VISION.md) — 产品哲学与路线图
- [AI 协作规范](../../agent.md) — 项目开发规范
- [v1.0 存档](../LUMO Design Language（LDL） v1.0.md) — 历史版本
- [图片设计说明](../lumo_picture_design.md) — 8 张高保真预览图说明

---

*End of README.*
