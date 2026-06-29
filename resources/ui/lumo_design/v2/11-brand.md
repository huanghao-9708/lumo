# 11 Brand

> LUMO wordmark 与品牌标识用法。

---

## 1. Wordmark

LUMO 的品牌标识是**纯文字 wordmark**，无图形 Logo。

### 构成

```
LUMO                      ← 20px Bold, tracking-[0.15em], Primary
LOCAL MUSIC SYSTEM        ← 9px Mono uppercase, tracking-widest, Muted
v1.0.0                    ← 9px Mono, Muted/60
```

### 参考代码

```html
<div class="px-8 pt-8 pb-6 cursor-pointer" data-tauri-drag-region>
  <h1 class="text-xl font-bold tracking-[0.15em] text-text-primary mb-2">LUMO</h1>
  <p class="text-[9px] tracking-widest text-text-muted font-mono uppercase leading-tight">Local Music System</p>
  <p class="text-[9px] text-text-muted/60 font-mono mt-0.5">v1.0.0</p>
</div>
```

### 规格

| 元素 | 字号 | 字重 | 字距 | 颜色 | 字体 |
|---|---|---|---|---|---|
| LUMO | 20px (`text-xl`) | Bold | `0.15em` | `text-text-primary` | Sans |
| 副标 | 9px | Regular | `widest` (0.1em) | `text-text-muted` | Mono uppercase |
| 版本 | 9px | Regular | 0 | `text-text-muted/60` | Mono |

### 位置

- **唯一位置**：Sidebar 顶部
- `px-8 pt-8 pb-6`（32px 横向，32px 顶部，24px 底部）
- `data-tauri-drag-region`（作为窗口拖拽区域）
- `cursor-pointer`（未来可点击打开关于页）

---

## 2. 字距原理

`tracking-[0.15em]` (3.75px @ 20px) 赋予 LUMO 字母间呼吸感，呼应 Editorial / Swiss 排版传统。大写字母 + 宽字距 = 工业感（Braun / Teenage Engineering 标识风格）。

---

## 3. 暗色模式

暗色下 wordmark 自动适配，**不特殊处理**：

| 元素 | 亮色 | 暗色 |
|---|---|---|
| LUMO | `#111111` | `#F2EFE9`（暖白） |
| 副标 | `#8B8B8B` | `#7A756B` |
| 版本 | `#8B8B8B` /60 | `#7A756B` /60 |

通过 `text-text-primary` / `text-text-muted` token 自动切换。

---

## 4. 禁用场景

- ❌ 不在 Sidebar 以外区域显示 wordmark
- ❌ 不修改字号 / 字重 / 字距
- ❌ 不加图形图标替代文字
- ❌ 不用 Accent 色
- ❌ 不加阴影 / 发光 / 渐变

---

## 5. 未来：关于页

点击 wordmark 未来可打开关于页（预留）：

- 显示完整版本号 / 构建信息
- 显示技术栈（Tauri / Vue / Rust）
- 显示 LDL 版本
- 遵循 VISION.md 愿景宣言

---

## 6. 命名规范

| 项 | 写法 | 说明 |
|---|---|---|
| 应用名 | **LUMO** | 全大写， Bold |
| 中文 | **轻音** | 与 LUMO 等价 |
| 简称 | Lumo | 句首或正常语境 |
| 文件名 | lumo | 小写 |
| 包名 | lumo | 小写 |
| 文档 | LUMO | 全大写 |

---

*End of Brand.*
