# Global Search

> 全局搜索页，从 TopBar 搜索框触发。

## Trigger

TopBar 搜索框 `v-model="playerStore.globalSearchQuery"`，输入即激活。

## Anatomy

```
┌──────────────────────────────────────────────────────────┐
│  搜索                                                      │
│  找到 N 个结果 / 输入关键词开始搜索                          │
│                                                           │
│  歌曲(3) ┃ 专辑(2) ┃ 艺术家(1)                              │
│  ─────────────────────────────────────────────────────    │
│                                                           │
│  [歌曲 Tab 内容]                                           │
│  # │ 标题            │ 艺术家      │ 专辑         │ 时长    │
│  01 │ Track Title    │ Artist      │ Album        │ 03:45  │
│                                                           │
│  [专辑 Tab 内容]  ┌─────────┐ ┌─────────┐                │
│                   │ Disc3   │ │ Disc3   │                │
│                   │ Album 1 │ │ Album 2 │                │
│                                                           │
│  [艺术家 Tab 内容] ┌─────────┐ ┌─────────┐                │
│                    │  User   │ │  User   │                │
│                    │ Artist1 │ │ Artist2 │                │
└──────────────────────────────────────────────────────────┘
```

## Search Logic

- 300ms 防抖
- 并行调用 `libraryGetTracks(20, 0, q)` / `libraryGetAlbums(12, 0, q)` / `libraryGetArtists(12, 0, q)`
- 清空搜索框时恢复原视图

## Interaction

| 结果类型 | 点击行为 |
|---|---|
| 歌曲 | 双击播放 |
| 专辑 | 跳转专辑详情（`activeAlbumId` + `activeLibraryTab='专辑'` + 清空搜索）|
| 艺术家 | 跳转歌手详情（`activeArtistId` + `activeLibraryTab='艺术家'` + 清空搜索）|

## States

| 状态 | 显示 |
|---|---|
| 无输入 | Search 图标 + "在 TopBar 搜索框输入关键词" |
| 搜索中 | Loader2 旋转 |
| 有结果 | 分 tab 显示 |
| 无结果 | "没有找到{歌曲/专辑/艺术家}" |
