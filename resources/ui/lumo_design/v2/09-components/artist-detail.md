# Artist Detail

> 歌手详情页的 Editorial Layout 规范。

---

## 1. Overview

歌手详情页采用**杂志式排版（Editorial Layout）**，左侧大尺寸肖像 + 右侧传记/统计 + 下方二级 Tab 分流为"全部歌曲"与"全部专辑"。

---

## 2. Anatomy

```
┌────────────────────────────────────────────────────┐
│  ┌──────────┐                                      │
│  │          │  艺术家名 (28px Bold)                 │
│  │ Portrait │  X TRACKS · Y ALBUMS (11px Mono)     │
│  │  180×180 │                                      │
│  │  rounded │  [▶ 播放全部]  [🔀 随机播放]         │
│  └──────────┘                                      │
├────────────────────────────────────────────────────┤
│  全部歌曲  │  全部专辑                              │  ← 二级 Tab（下划线 Accent）
├────────────────────────────────────────────────────┤
│  #  │  ♥  │  标题  │  专辑          │  时长  │  ⋯  │
│  01 │     │  Song  │  Album         │  03:45 │     │
│  02 │     │  Song  │  Album         │  04:12 │     │
│  ...                                              │
└────────────────────────────────────────────────────┘
```

---

## 3. 头部布局

```html
<div class="flex items-start gap-8">
  <!-- 左：肖像 -->
  <div class="w-[180px] h-[180px] rounded-[10px] overflow-hidden bg-gradient-to-br">
    <User class="w-[48px] h-[48px] text-white/60" />
  </div>

  <!-- 右：信息 -->
  <div class="flex-1 min-w-0 pt-2">
    <h1 class="text-[28px] font-bold">{{ artist.name }}</h1>
    <p class="text-[11px] text-text-muted font-mono uppercase tracking-wider">
      {{ stats.track_count }} TRACKS · {{ stats.album_count }} ALBUMS
    </p>
    <div class="flex items-center gap-3">
      <PrimaryButton label="播放全部" />
      <SecondaryButton label="随机播放" />
    </div>
  </div>
</div>
```

---

## 4. 二级 Tab 系统

```html
<div class="flex items-center gap-8 px-8 pt-4 pb-0">
  <button class="text-[13px] pb-2 border-b-2"
    :class="activeTab === 'tracks'
      ? 'text-text-primary border-brand-orange font-medium'
      : 'text-text-muted border-transparent'" />
  <button class="text-[13px] pb-2 border-b-2"
    :class="activeTab === 'albums'
      ? 'text-text-primary border-brand-orange font-medium'
      : 'text-text-muted border-transparent'" />
</div>
```

| Tab | 内容 | 数据源 |
|---|---|---|
| 全部歌曲 | Song Row 列表 | `currentArtistDetails.tracks` |
| 全部专辑 | AlbumGrid 网格 | `currentArtistDetails.albums` |

---

## 5. States

| 状态 | 显示 |
|---|---|
| Loading | `Loader2` 旋转图标 + "加载艺术家…" |
| 空数据 | "暂无歌曲" / "暂无专辑" |
| 无选中 | "选择一位艺术家查看详情" |

---

## 6. Tokens used

| Token | 用途 |
|---|---|
| `--text-primary` | 艺术家名 |
| `--text-secondary` | 统计行 |
| `--text-muted` | Tab 非选中态 |
| `--brand-orange` | Tab 下划线 Accent |
| `--list-hover` | 行 Hover |
| `--list-selected` | 当前播放行 |

---

## 7. Do & Don't

### Do

- ✅ 大尺寸肖像 + 信息区左图文排版
- ✅ 二级 Tab 仅用下划线 Accent 区分
- ✅ 操作按钮对齐 AlbumDetail 规范

### Don't

- ❌ 在头部放搜索框
- ❌ 使用 Card/阴影划分 Tab 内容
- ❌ 超出显示区域不 truncate

---

*End of Artist Detail.*
