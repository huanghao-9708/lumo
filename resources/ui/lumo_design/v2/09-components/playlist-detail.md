# Playlist Detail

> 歌单详情页，复用 AlbumDetail 布局。

## Anatomy

```
┌──────────────────────────────────────────────────┐
│  ┌─────────┐  歌单名称                            │
│  │         │  X TRACKS · X 小时 X 分钟             │
│  │  List   │  [▶ 播放全部] [🔀 随机播放]            │
│  │  Music  │                                       │
│  │  (180px)│                                       │
│  └─────────┘                                       │
├──────────────────────────────────────────────────┤
│  # │ ♥ │ 标题            │ 艺术家      │ 时长      │
├──────────────────────────────────────────────────┤
│  01 │ ♥ │ Track Title    │ Artist      │ 03:45    │
│  02 │   │ Another Track  │ Artist      │ 04:12    │
├──────────────────────────────────────────────────┤
│  X 首曲目 · X 小时 X 分钟              双击播放    │
└──────────────────────────────────────────────────┘
```

## States

| 状态 | 显示 |
|---|---|
| 加载中 | Loader2 居中旋转 |
| 空列表 | "该歌单暂无曲目" |
| 正常 | 封面 + 信息 + 按钮 + 轨道列表 |

## Tokens used

Same as AlbumDetail + `currentPlaylistDetails` store data.

## Store data structure

```ts
currentPlaylistDetails: {
  id: number;
  name: string;
  description?: string | null;
  count: number;
  tracks: Track[];
  isLoadingTracks: boolean;
}
```

## 路线图

- v2.1: 歌单封面（自定义上传 / 自动从曲目封面生成）
- v2.2: 歌单编辑（删除曲目/排序/重命名）
