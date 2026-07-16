# Mobile Album Card

> 2 列网格封面卡片。常驻播放按钮，单击进详情。

---

## 1. Overview

对应桌面 AlbumCard（5 列 + Hover 浮层按钮）的移动端适配。

| 项 | 移动端 | 桌面端 |
|---|---|---|
| 列数 | **2 列**（手机）/ 3 列（平板） | 自适应（minmax 180px） |
| 间距 | `gap-4` (16px) | `gap-6` (24px) |
| 播放按钮 | **常驻可见**（右上角） | Hover 浮层 |
| 播放按钮颜色 | `bg-text-primary/70`（半透黑） | `bg-brand-orange` |

---

## 2. Anatomy

```
┌──────────────┐
│         [▶]  │  常驻播放按钮（右上 32×32 圆形）
│              │
│    封面       │  aspect-square · rounded-[10px]
│              │
└──────────────┘
  专辑名称       15px Medium
  艺术家 · 年份  13px Muted
```

---

## 3. States

### 3.1 Default

- 封面：`rounded-[10px]`，`bg-bg-hover` 加载底
- 播放按钮：`w-8 h-8` 圆形，半透黑底 + 白 Play 图标

### 3.2 Active（当前查看的专辑）

- 标题：`text-brand-orange`（选中态）

---

## 4. 交互

| 手势 | 行为 |
|---|---|
| 单击封面 | 进入专辑详情（`playerStore.activeAlbumId = album.id`） |
| 单击播放按钮 | 直接播放整张专辑（`playAll`） |
| `@click.stop` | 播放按钮阻止冒泡到封面 |

---

## 5. Props

```ts
defineProps<{ album: Album }>();
```

## 6. Events

```ts
emit: {
  (e: 'select', album: Album): void;
}
```

---

## 7. Tokens used

| Token | 用途 |
|---|---|
| `--radius-10` | 封面圆角 |
| `--text-15` | 专辑名 |
| `--text-13` | 艺术家/年份 |
| `--bg-hover` | 封面占位底 |
| `--brand-orange` | 选中标题 |
| `--text-disabled` | 无封面 Disc3 占位 |

---

## 8. Do & Don't

### Do

- ✅ 2 列网格，`gap-4`
- ✅ 播放按钮常驻（不依赖 Hover）
- ✅ 单击封面进详情，单击按钮播放
- ✅ `loading="lazy"` 懒加载

### Don't

- ❌ 用 Accent 做播放按钮（移动端用半透黑，桌面用 Accent——场景不同）
- ❌ 5+ 列（小屏放不下）

---

*End of Mobile Album Card.*
