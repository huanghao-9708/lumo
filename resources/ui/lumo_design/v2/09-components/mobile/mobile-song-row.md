# Mobile Song Row

> 移动端核心组件。56px 触控行，单击播放，长按菜单。

---

## 1. Overview

移动端歌曲行，对应桌面 Song Row（40px）的触屏适配版。

| 项 | 移动端 | 桌面端 |
|---|---|---|
| 行高 | **56px** (`--touch-row`) | 40px |
| 交互 | **单击播放** | 双击播放 |
| Hover | **无**（active:bg-list-hover 反馈） | group-hover 显隐 |
| 收藏 | **始终可见** | 未收藏 Hover 显 |

---

## 2. Anatomy

```
┌────┬──────────────────────┬──────┬────┐
│ #  │ 标题                  │ 时长 │ ♥  │
│ 36 │ 艺术家                 │  48  │ 44 │
└────┴──────────────────────┴──────┴────┘
           ← 56px 高 →
```

### 列宽

| 列 | 宽度 | 说明 |
|---|---|---|
| # 序号 | 36px | Playing → Loader2/Play；其余 → 两位数字 mono |
| 标题/艺术家 | flex-1 | 双行：15px Medium + 13px Secondary |
| 时长 | 48px | 12px mono tabular-nums |
| 收藏 ♥ | 44px | 始终可见，已收藏 Accent fill，未收藏 muted outline |

---

## 3. States

### 3.1 Default

- 序号：两位数字，12px mono Muted
- 标题：15px Medium Primary
- 艺术家：13px Regular Secondary
- 收藏：未收藏 muted outline，已收藏 Accent fill

### 3.2 Playing（当前播放）

| 元素 | Playing 态 |
|---|---|
| 行背景 | `bg-list-selected` |
| 左侧竖条 | 2px Accent（`.playing-row::before`，复用桌面） |
| 序号 | Loader2(spin) 或 Play(fill) Accent |
| 标题 | `text-brand-orange font-semibold`（不跳到 Bold） |

### 3.3 Long Press（长按 500ms）

触发 ActionSheet 底部菜单，可选操作：

- 收藏 / 取消收藏
- 查看专辑（跳转专辑详情）
- 查看艺术家（跳转艺术家详情）

---

## 4. Props

```ts
defineProps<{
  track: Track;
  index: number;
  isPlaying: boolean;
  isCurrent: boolean;
}>();
```

## 5. Events

```ts
emit: {
  (e: 'play', index: number): void;
  (e: 'toggleFav', trackId: number): void;
  (e: 'longPress', trackId: number): void;
}
```

---

## 6. Tokens used

| Token | 用途 |
|---|---|
| `--touch-row` | 行高 56px |
| `--text-mobile-body` | 标题 15px |
| `--text-13` | 艺术家字号 |
| `--text-12` | 序号/时长 |
| `--list-selected` | Playing 背景 |
| `--brand-orange` | Playing 指示 |
| `--text-primary` / `--text-secondary` / `--text-muted` / `--text-disabled` | 文字色 |

---

## 7. Do & Don't

### Do

- ✅ 56px 行高，触摸目标 ≥ 44px
- ✅ 单击播放（非双击）
- ✅ 收藏始终可见
- ✅ 长按出 ActionSheet
- ✅ Playing 用 Semibold + Accent（不跳到 Bold）

### Don't

- ❌ 用 Hover 显隐（无 Hover）
- ❌ Playing 标题用 Bold（应用 Semibold）
- ❌ 忽略长按手势

---

*End of Mobile Song Row.*
