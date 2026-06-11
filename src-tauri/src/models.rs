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
    /// 用于在操作系统的钥匙串中获取密码的引用标识
    pub credential_ref: Option<String>,
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
#[derive(Debug, serde::Serialize)]
pub struct TrackDTO {
    /// 歌曲 ID
    pub id: i64,
    /// 歌曲标题
    pub title: String,
    /// 格式化后拼接好的所有艺人名称（如 "Artist A, Artist B"）
    pub artist_name: Option<String>,
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
