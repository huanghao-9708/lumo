use serde::{Deserialize, Serialize};

/// 音乐来源配置，代表用户添加的一个根目录或远程源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// 来源的唯一自增 ID
    pub id: i64,
    /// 来源名称（如：本地音乐库、我的 NAS）
    pub name: String,
    /// 来源类型：'local' (本地路径) 或 'webdav' (远程 WebDAV)
    pub kind: String,
    /// 根路径或根 URI（如 D:\music 或 https://nas.local/music）
    pub root_uri: String,
    /// 额外的 JSON 配置项
    pub config_json: String,
    /// 凭据引用（钥匙串条目或机器绑定密文）。安全收口（P1-08）：永不序列化到前端，
    /// 前端展示改用 username 字段（解析出的用户名部分）。
    #[serde(skip_serializing)]
    pub credential_ref: Option<String>,
    /// 来源用户名（从 credential_ref 的用户名部分解出，供前端展示）
    #[serde(default)]
    pub username: Option<String>,
    /// 是否启用该来源（如果不启用，则该来源下的歌曲不显示）
    pub enabled: bool,
    /// 最后一次扫描的时间戳
    pub last_scan_at: Option<String>,
    /// 最后一次扫描遇到的错误信息
    pub last_error: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 艺人实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    /// 艺人的唯一自增 ID
    pub id: i64,
    /// 艺人原始名称
    pub name: String,
    /// 用于去重和模糊匹配的归一化名称（如全小写、去空格）
    pub normalized_name: String,
    /// 用于排序的名称（如拼音或拼音首字母，预留字段）
    pub sort_name: Option<String>,
    /// 艺人类型：'person' (个人), 'group' (乐队/组合) 等
    pub kind: String,
    /// MusicBrainz ID，用于日后联网获取高清头像和详细信息
    pub mbid: Option<String>,
}

/// 专辑实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    /// 专辑的唯一自增 ID
    pub id: i64,
    /// 专辑原始标题
    pub title: String,
    /// 归一化标题（小写等，用于防重复插入）
    pub normalized_title: String,
    /// 用于按字母排序的标题（预留）
    pub sort_title: Option<String>,
    /// 专辑艺人的 ID（外键关联 artists 表）
    pub album_artist_id: Option<i64>,
    /// 专辑类型：'album' (正式专辑), 'single' (单曲), 'ep' (迷你专辑) 等
    pub album_type: String,
    /// 具体发布日期
    pub release_date: Option<String>,
    /// 发布年份
    pub release_year: Option<i64>,
    /// 该专辑包含的总碟片数
    pub total_discs: Option<i64>,
    /// 专辑封面图片在 artwork 表中的 ID
    pub cover_artwork_id: Option<i64>,
}

/// 歌曲实体（代表用户眼中的“一首歌”，而非具体文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    /// 歌曲的唯一自增 ID
    pub id: i64,
    /// 歌曲原标题
    pub title: String,
    /// 归一化标题
    pub normalized_title: String,
    /// 用于排序的标题
    pub sort_title: Option<String>,
    /// 所属专辑的 ID
    pub album_id: Option<i64>,
    /// 所属碟片号
    pub disc_no: Option<i64>,
    /// 在专辑或碟片中的音轨号
    pub track_no: Option<i64>,
    /// 歌曲年份
    pub year: Option<i64>,
    /// 该歌曲对应的首选或主要媒体文件 ID（一首歌可能在多处有备份）
    pub primary_file_id: Option<i64>,
    /// 用户对歌曲的星级评分（0-5）
    pub rating: Option<i64>,
    /// 歌曲完整播放的次数
    pub play_count: i64,
    /// 歌曲被手动切歌/跳过的次数
    pub skip_count: i64,
    /// 上次播放的时间戳
    pub last_played_at: Option<String>,
    /// 歌曲首次被添加到系统的时间
    pub added_at: String,
}

/// 媒体文件实体（代表硬盘或 WebDAV 上的实际音频文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    /// 文件的唯一自增 ID
    pub id: i64,
    /// 文件所属的来源 ID
    pub source_id: i64,
    /// 关联的歌曲实体 ID
    pub track_id: Option<i64>,
    /// 相对于 root_uri 的相对路径
    pub relative_path: String,
    /// 归一化路径，结合 source_id 作为唯一键防止重复扫描
    pub normalized_path: String,
    /// 文件名
    pub file_name: String,
    /// 文件扩展名（如 mp3, flac）
    pub file_ext: Option<String>,
    /// 文件大小（字节数）
    pub file_size: Option<i64>,
    /// 文件的最后修改时间
    pub modified_at: Option<String>,
    /// 歌曲的播放时长（毫秒）
    pub duration_ms: Option<i64>,
    /// 音频比特率（bps）
    pub bitrate: Option<i64>,
    /// 音频采样率（Hz）
    pub sample_rate: Option<i64>,
    /// 音频位深（如 16, 24）
    pub bit_depth: Option<i64>,
    /// 声道数（1 为单声道，2 为立体声）
    pub channels: Option<i64>,
    /// 文件的可用性状态：'available' (可用), 'missing' (文件丢失), 等
    pub availability: String,
}

/// 传输给前端的歌曲数据传输对象 (Data Transfer Object)
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrackDTO {
    /// 歌曲 ID
    pub id: i64,
    /// 歌曲标题
    pub title: String,
    /// 首要艺人 ID（用于跳转）
    pub artist_id: Option<i64>,
    /// 格式化后拼接好的所有艺人名称（如 "Artist A, Artist B"）
    pub artist_name: Option<String>,
    /// 所属专辑 ID（用于跳转）
    pub album_id: Option<i64>,
    /// 专辑标题
    pub album_title: Option<String>,
    /// 时长（毫秒）
    pub duration_ms: Option<i64>,
    /// 格式扩展名（如 MP3, FLAC）
    pub format: Option<String>,
    /// 用于调用播放接口的物理文件 ID
    pub media_file_id: i64,
    /// 用户是否已将此歌曲标记为“我喜欢”
    pub is_favorite: bool,
    /// 封面图片 ID
    pub cover_artwork_id: Option<i64>,
    /// 最近播放时间戳（仅最近播放查询返回）
    pub last_played_at: Option<String>,
    /// 文件大小（字节）
    pub file_size: Option<i64>,
    /// 来源类型："local" | "webdav"，用于前端离线降级判断
    pub source_kind: String,
}

/// 传输给前端的专辑数据传输对象
#[derive(Debug, serde::Serialize)]
pub struct AlbumDTO {
    /// 专辑 ID
    pub id: i64,
    /// 专辑标题
    pub title: String,
    /// 专辑艺人名称
    pub artist_name: Option<String>,
    /// 封面图片 ID
    pub cover_artwork_id: Option<i64>,
    /// 该专辑下包含的歌曲总数
    pub track_count: i64,
    /// 200x200 JPEG 缩略图的 base64 data URL（若有）。
    /// 前端可直接用作 `<img src>`，无需再发 `lumo://artwork` 请求。
    /// 仅当扫描期已生成缩略图时才有值；否则为 None，前端 fallback 到原协议。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_thumbnail_base64: Option<String>,
}

/// 传输给前端的艺人数据传输对象
#[derive(Debug, serde::Serialize)]
pub struct ArtistDTO {
    /// 艺人 ID
    pub id: i64,
    /// 艺人名称
    pub name: String,
    /// 该艺人参与的歌曲总数
    pub track_count: i64,
    /// 艺人头像在 artwork 表中的 ID
    pub avatar_artwork_id: Option<i64>,
}

/// 传输给前端的歌单数据传输对象
#[derive(Debug, serde::Serialize)]
pub struct PlaylistDTO {
    /// 歌单 ID
    pub id: i64,
    /// 歌单名称
    pub name: String,
    /// 歌单简介
    pub description: Option<String>,
    /// 歌单内的歌曲总数
    pub track_count: i64,
}

/// 传输给前端的艺人统计数据对象
#[derive(Debug, serde::Serialize)]
pub struct ArtistStatsDTO {
    /// 该艺人参与的歌曲总数
    pub track_count: i64,
    /// 该艺人参与的专辑总数
    pub album_count: i64,
}

/// 传输给前端的物理音频文件详细元数据传输对象
#[derive(Debug, serde::Serialize)]
pub struct TrackFileInfoDTO {
    pub id: i64,
    pub source_id: i64,
    pub track_id: i64,
    pub path: String,
    pub relative_path: String,
    pub file_name: String,
    pub file_ext: Option<String>,
    pub file_size: Option<i64>,
    pub modified_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub bitrate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub bit_depth: Option<i64>,
    pub channels: Option<i64>,
    pub format: Option<String>,
    pub source_kind: String,
}

/// 传输给前端的文件夹内容结构
#[derive(Debug, serde::Serialize)]
pub struct FolderEntryDTO {
    /// 文件夹或文件名称
    pub name: String,
    /// 是否为文件夹
    pub is_dir: bool,
    /// 绝对路径或相对于系统的归一化路径
    pub path: String,
    /// 该目录下的音频文件总数（仅 is_dir=true 时有效）
    pub audio_count: Option<i64>,
    /// 如果是音频文件且被索引，则附带其完整歌曲信息
    pub track: Option<TrackDTO>,
}

/// `library_get_artists` 的分页返回包装
#[derive(Debug, serde::Serialize)]
pub struct ArtistListResult {
    /// 当前页的艺人列表
    pub artists: Vec<ArtistDTO>,
    /// 艺人总数（不分页）
    pub total: i64,
}

/// `library_get_folder_contents` 的返回包装。
/// 包含当前页的条目和总条目数，前端据此判断是否还有更多内容需要滚动加载。
#[derive(Debug, serde::Serialize)]
pub struct FolderContentsResult {
    /// 当前页的文件夹条目（已排序：目录优先、再按名称）
    pub entries: Vec<FolderEntryDTO>,
    /// 该文件夹下的总条目数（不随分页变化），前端用 `entries.len() < total` 判断是否到末尾
    pub total: usize,
}

/// 文件浏览器 - 目录树节点
#[derive(Debug, serde::Serialize)]
pub struct DirectoryNodeDTO {
    /// 目录名
    pub name: String,
    /// 完整路径（相对于 source root 的路径）
    pub path: String,
    /// 该目录（含子目录）下的音频文件总数
    pub audio_count: i64,
    /// 是否有子目录
    pub has_subdirs: bool,
}

/// `library_get_folder_children` 的返回包装
#[derive(Debug, serde::Serialize)]
pub struct FolderChildrenResult {
    /// 子目录列表
    pub children: Vec<DirectoryNodeDTO>,
    /// 源根路径（前端用于面包屑显示）
    pub source_root: String,
}

/// `library_get_folder_tracks` 的返回包装
#[derive(Debug, serde::Serialize)]
pub struct FolderTracksResult {
    /// 当前页的歌曲列表
    pub tracks: Vec<TrackDTO>,
    /// 总歌曲数
    pub total: i64,
}

/// `library_get_counts` 的统一返回
#[derive(Debug, serde::Serialize)]
pub struct LibraryCounts {
    pub tracks: i64,
    pub favorite_tracks: i64,
    pub favorite_albums: i64,
    pub favorite_artists: i64,
    pub recently_played: i64,
}

/// 跨设备数据同步配置（对应的前端配置表 sync_config）
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SyncConfigDTO {
    pub enabled: bool,
    pub webdav_url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub remote_path: Option<String>,
    pub last_sync_at: Option<String>,
    pub last_sync_direction: Option<String>,
}

/// 同步结果
#[derive(Debug, serde::Serialize)]
pub struct SyncResult {
    pub bytes_uploaded: u64,
    pub timestamp: String,
}

/// 远程检查结果
#[derive(Debug, serde::Serialize)]
pub struct RemoteCheckResult {
    pub has_data: bool,
    pub remote_size: Option<u64>,
    pub last_modified: Option<String>,
}

/// WebDAV 目录项（用于文件夹浏览器）
#[derive(Debug, Clone, serde::Serialize)]
pub struct WebdavEntry {
    pub name: String,
    pub is_dir: bool,
    pub path: String,
}

/// `library_get_stats` 的返回：首页统计卡数据。
/// 一条 SQL 聚合曲库规模与收听行为，一次 IPC 返回。
/// 「今日/近7天」按本地时区口径过滤（play_history.played_at 是 UTC 朴素串，
/// `datetime('now','localtime','start of day','utc')` = 本地今日零点的 UTC 时刻）。
#[derive(Debug, serde::Serialize)]
pub struct LibraryStats {
    /// 曲库歌曲总数
    pub track_count: i64,
    /// 专辑总数
    pub album_count: i64,
    /// 艺人总数
    pub artist_count: i64,
    /// 累计听歌时长（毫秒，SUM(play_history.play_duration_ms)）
    pub total_listen_ms: i64,
    /// 今日听歌时长（毫秒）
    pub today_listen_ms: i64,
    /// 近 7 天听歌时长（毫秒）
    pub week_listen_ms: i64,
    /// 累计听歌次数（SUM(tracks.play_count)）
    pub total_play_count: i64,
    /// 今日播放次数（play_history 当日流水条数）
    pub today_play_count: i64,
    /// 歌单数
    pub playlist_count: i64,
    /// 收藏的专辑数
    pub favorite_album_count: i64,
    /// 收藏的艺人数
    pub favorite_artist_count: i64,
    /// 喜欢的歌曲数
    pub favorite_track_count: i64,
}

/// 排行榜歌曲：标准 TrackDTO 13 列之上多一列 play_count（第 14 列）。
/// serde flatten 使 JSON 表现为 TrackDTO 全部字段 + play_count。
#[derive(Debug, serde::Serialize)]
pub struct RankedTrackDTO {
    #[serde(flatten)]
    pub track: TrackDTO,
    /// 播放次数
    pub play_count: i64,
}

/// 排行榜艺人
#[derive(Debug, serde::Serialize)]
pub struct RankedArtistDTO {
    pub id: i64,
    pub name: String,
    /// 名下所有歌曲累计播放次数
    pub play_count: i64,
    /// 名下歌曲总数（artists.track_count 冗余字段）
    pub track_count: i64,
    pub avatar_artwork_id: Option<i64>,
}

/// 排行榜专辑
#[derive(Debug, serde::Serialize)]
pub struct RankedAlbumDTO {
    pub id: i64,
    pub title: String,
    /// 专辑艺人（album_artists GROUP_CONCAT）
    pub artist_name: Option<String>,
    pub cover_artwork_id: Option<i64>,
    /// 专辑内所有歌曲累计播放次数
    pub play_count: i64,
}

/// `library_get_insights` 的返回：首页 8 个查询的结果一次 IPC 打包，
/// 避免逐个查询造成 IPC 拥堵。
#[derive(Debug, serde::Serialize)]
pub struct LibraryInsights {
    /// 播放最多的歌曲榜
    pub top_played_tracks: Vec<RankedTrackDTO>,
    /// 最近播放榜
    pub recent_played_tracks: Vec<RankedTrackDTO>,
    /// 最近添加榜
    pub recent_added_tracks: Vec<RankedTrackDTO>,
    /// 我喜欢的音乐榜
    pub favorite_tracks: Vec<RankedTrackDTO>,
    /// 听得最多的艺人榜
    pub top_played_artists: Vec<RankedArtistDTO>,
    /// 播放最多的专辑榜
    pub top_played_albums: Vec<RankedAlbumDTO>,
    /// 今日播放次数（play_history 流水口径）
    pub today_play_count: i64,
    /// 上次听歌（最近一条有播放时间的曲目，含 last_played_at）
    pub last_played: Option<RankedTrackDTO>,
}

/// 封面后台拉取完成事件（`album-cover-fetched` / `artist-cover-fetched`）的 payload。
/// 第七轮：封面拉取脱离 IPC channel 后，前端靠此事件更新对应条目的封面 id。
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoverFetchedEvent {
    /// 专辑 id 或艺人 id
    pub target_id: i64,
    /// 新写入的封面/头像 artwork id
    pub artwork_id: i64,
    /// 200x200 缩略图的 data URL（v1.8.1）：随事件下发，前端网格即时更新
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_thumbnail_base64: Option<String>,
}

/// 单次封面拉取的内部结果（impl → 命令层，非序列化）
#[derive(Debug)]
pub struct FetchedCover {
    pub artwork_id: i64,
    pub thumbnail_base64: Option<String>,
}

/// `library_get_startup_bundle` 的返回：App.vue 启动所需数据一次 IPC 打包。
/// 启动 IPC 从 ~7 个降到 2 个（本命令 + source_list，后者凭据解析在 scanner 模块）。
#[derive(Debug, serde::Serialize)]
pub struct StartupBundle {
    /// 曲库与收藏计数（同 library_get_counts）
    pub counts: LibraryCounts,
    /// 歌单列表
    pub playlists: Vec<PlaylistDTO>,
    /// 专辑网格第一页（30 条，内联缩略图；与前端 albumsPageSize 一致）
    pub albums: Vec<AlbumDTO>,
    /// 专辑总数
    pub album_total: i64,
    /// 艺人第一页（50 条；与前端 artistsLimit 一致）
    pub artists: Vec<ArtistDTO>,
    /// 艺人总数
    pub artist_total: i64,
    /// 持久化的播放队列（恢复会话用，同 library_get_play_queue）
    pub play_queue: Vec<TrackDTO>,
}

/// AI 设置（ai_settings 表）。API key 永不序列化出后端，前端只见 has_key。
#[derive(Debug, serde::Serialize)]
pub struct AiSettingsDTO {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    pub temperature: f64,
    pub has_key: bool,
}

/// AI 推荐歌单结果。source="fallback" 时 degraded_reason 说明降级原因。
#[derive(Debug, serde::Serialize)]
pub struct AiPlaylistResult {
    /// 歌单名（AI 起名；兜底时为规则名）
    pub name: String,
    /// 一句话简介
    pub description: String,
    /// "ai" | "fallback"
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degraded_reason: Option<String>,
    pub tracks: Vec<AiRankedTrackDTO>,
}

/// AI 推荐歌单条目：TrackDTO + 一句话推荐理由
#[derive(Debug, serde::Serialize)]
pub struct AiRankedTrackDTO {
    #[serde(flatten)]
    pub track: TrackDTO,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// AI 连接测试结果
#[derive(Debug, serde::Serialize)]
pub struct AiTestConnectionResult {
    pub ok: bool,
    pub message: String,
    pub latency_ms: u64,
}
