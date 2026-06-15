use lofty::probe::Probe;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use lofty::tag::ItemKey;
use std::path::Path;

/// 从音频文件中解析出的基础元数据
#[derive(Debug, Default)]
pub struct AudioMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    /// 专辑艺人（AlbumArtist 标签），用于整张专辑的归类，缺失时回退到 artist
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub bit_rate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub channels: Option<i64>,
    pub picture_data: Option<Vec<u8>>,
    pub picture_mime: Option<String>,
    pub lyrics: Option<String>,
}

/// 解析单个音频文件，失败时返回 Err（由调用方决定跳过/标记 error）。
pub fn extract_metadata<P: AsRef<Path>>(path: P) -> Result<AudioMetadata, String> {
    let tagged_file = match Probe::open(path.as_ref()) {
        Ok(probe) => match probe.read() {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to read file: {}", e)),
        },
        Err(e) => return Err(format!("Failed to open file: {}", e)),
    };
    extract_metadata_inner(tagged_file)
}

pub fn extract_metadata_from_reader<R: std::io::Read + std::io::Seek>(reader: R) -> Result<AudioMetadata, String> {
    let tagged_file = match Probe::new(reader).guess_file_type() {
        Ok(probe) => match probe.read() {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to read stream: {}", e)),
        },
        Err(e) => return Err(format!("Failed to guess type for stream: {}", e)),
    };
    extract_metadata_inner(tagged_file)
}

fn extract_metadata_inner(tagged_file: lofty::file::TaggedFile) -> Result<AudioMetadata, String> {
    let mut metadata = AudioMetadata::default();

    let properties = tagged_file.properties();
    metadata.duration_ms = Some(properties.duration().as_millis() as i64);
    // lofty 0.21 的 `audio_bitrate()` 返回单位是 kbps (u32)，这里乘以 1000 转为 bps 入库
    metadata.bit_rate = properties.audio_bitrate().map(|b| b as i64 * 1000);
    metadata.sample_rate = properties.sample_rate().map(|s| s as i64);
    metadata.channels = properties.channels().map(|c| c as i64);

    if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
        metadata.title = tag.title().map(|s| s.into_owned());
        metadata.artist = tag.artist().map(|s| s.into_owned());
        // lofty 0.21 的 Accessor trait 未提供 album_artist 访问器，
        // 这里通过 ItemKey 直接查询 tag items（对应 ID3 的 TPE2 / Vorbis 的 ALBUMARTIST）。
        // 用于把整张专辑归到一个艺人名下，避免合辑被拆碎。
        for item in tag.items() {
            if item.key() == &ItemKey::AlbumArtist {
                if let Some(txt) = item.value().text() {
                    metadata.album_artist = Some(txt.to_string());
                }
                break;
            }
        }
        metadata.album = tag.album().map(|s| s.into_owned());

        if let Some(pic) = tag.pictures().first() {
            metadata.picture_data = Some(pic.data().to_vec());
            metadata.picture_mime = pic.mime_type().map(|m| m.to_string());
        }

        // 尝试从 tag items 里面查找 Lyrics 键，取出内嵌歌词
        for item in tag.items() {
            if item.key() == &lofty::tag::ItemKey::Lyrics {
                if let Some(txt) = item.value().text() {
                    metadata.lyrics = Some(txt.to_string());
                    break;
                }
            }
        }
    }

    Ok(metadata)
}
