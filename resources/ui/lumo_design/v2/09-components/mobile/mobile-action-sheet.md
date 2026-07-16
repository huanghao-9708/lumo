# Mobile ActionSheet

> 底部操作菜单。Teleport to body + 遮罩 + 上滑动画。

---

## 1. Overview

移动端长按等操作触发的底部菜单，无桌面端对应组件。

---

## 2. Anatomy

```
┌─────────────────────────┐
│                         │
│     (半透明遮罩)          │  bg-black/30
│                         │
├─────────────────────────┤
│  操作 1                  │  48px 行，15px Medium
│  操作 2                  │
│  ───────────────         │  h-px bg-border-color mx-4
│  操作 3（危险/红色）      │  text-status-error（v2.1）
│  ───────────────         │
│  取消                    │  text-text-secondary
└─────────────────────────┘
    rounded-t-[16px] + safe-area-inset-bottom
```

---

## 3. 动画

| 元素 | 动画 |
|---|---|
| 遮罩 | `opacity 0 → 1`，250ms ease-out |
| 面板 | `translateY(100%) → 0`，250ms `cubic-bezier(0.4,0,0.2,1)` |
| 退出 | 同上反向 |

---

## 4. Props

```ts
defineProps<{
  visible: boolean;
  actions: ActionItem[];
}>();

interface ActionItem {
  label: string;
  onClick: () => void;
  danger?: boolean;    // text-status-error
  disabled?: boolean;  // opacity-40 + cursor-not-allowed
}
```

## 5. Events

```ts
emit: { (e: 'close'): void }
```

---

## 6. 触发方式

通过 MobileSongRow 的长按（500ms touch-hold）触发：

```
@touchstart → start 500ms timer
@touchmove  → cancel timer
@touchend   → cancel timer（短按）
timer fires → emit('longPress', trackId) → open ActionSheet
```

长按后跳过同次 click，避免同时触发播放和菜单。

---

## 7. 安全区

```css
padding-bottom: env(safe-area-inset-bottom);
```

---

## 8. Tokens used

| Token | 用途 |
|---|---|
| `--radius-16` | 面板顶部圆角 |
| `--status-error` | danger 项文字色 |
| `--border-color` | Divider |
| `--text-primary` / `--text-secondary` | 操作项 / 取消按钮 |
| `--list-hover` | active 态背景 |

---

## 9. Do & Don't

### Do

- ✅ `Teleport to body`（脱离父容器层级）
- ✅ 遮罩 + 上滑动画（250ms）
- ✅ Danger 项用 `--status-error`
- ✅ `safe-area-inset-bottom`
- ✅ 点击遮罩或取消按钮关闭
- ✅ `close()` 的 250ms setTimeout 在 `onBeforeUnmount` 清理

### Don't

- ❌ 阴影（用 Divider + 圆角分层）
- ❌ 弹簧/缩放动画
- ❌ 透传点击到背景（遮罩拦截）

---

*End of Mobile ActionSheet.*
