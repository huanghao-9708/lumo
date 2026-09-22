use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use lofty::tag::ItemKey;
use std::path::Path;

// ===================== 多艺人标签解析（扫描 + 迁移共用一份） =====================
//
// 历史教训：扫描侧（services::library）与存量迁移侧（db::V5）各抄了一份分隔符清单，
// 结果后来发现曲库里还有用 `|` 分隔的合作标签时，只改一边就会两边行为不一致。
// 现在唯一的清单 + 唯一的拆分函数都放在这里。

/// 多艺人标签的分隔符集合。
pub const ARTIST_SEPARATORS: &[char] = &['&', '＆', ';', '；', '、', '，', ',', '|', '｜'];

/// 连接词（需自成词、大小写不敏感）：`feat.` / `ft.` / `featuring`
const COLLAB_KEYWORDS: &[&str] = &["featuring", "feat.", "feat", "ft.", "ft"];

/// 归一化艺人/标题字符串：去掉首尾空白、折叠中间多个空白为单个空格。
/// 仅用于展示与去重的"原值"清理；做唯一键时再额外 `.to_lowercase()`。
pub fn normalize_artist_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.trim().chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

/// 把「多位艺人」标签切成单个艺人名。
///
/// 支持的分隔符：`&` `＆` `;` `；` `、` `，` `,` `|` `｜`，以及连接词
/// `feat.` / `ft.` / `featuring`（大小写不敏感、需自成词）。
///
/// ```text
/// "A & B"       -> ["A", "B"]
/// "A|B"         -> ["A", "B"]     // 竖线分隔（部分下载源）
/// "A feat. B"   -> ["A", "B"]
/// "Kraftwerk"   -> ["Kraftwerk"]  // 内含 "ft" 但不是独立词，不切
/// ```
pub fn split_artist_names(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;

    while i < raw.len() {
        let ch = raw[i..].chars().next().expect("char boundary");

        if ARTIST_SEPARATORS.contains(&ch) {
            flush_part(&mut out, &mut buf);
            i += ch.len_utf8();
            continue;
        }

        // 连接词必须自成词：前面是空白/串首，后面是空白/串尾，
        // 否则 "Kraftwerk"、"Defeats" 这类名字会被切坏。
        let at_word_start = buf.is_empty() || buf.ends_with(char::is_whitespace);
        if let Some(len) = match_collab_keyword(&raw[i..], at_word_start) {
            flush_part(&mut out, &mut buf);
            i += len;
            continue;
        }

        buf.push(ch);
        i += ch.len_utf8();
    }

    flush_part(&mut out, &mut buf);
    out
}

fn flush_part(out: &mut Vec<String>, buf: &mut String) {
    let part = strip_trailing_collab_keyword(buf.trim());
    if !part.is_empty() {
        out.push(part.to_string());
    }
    buf.clear();
}

/// 去掉结尾残留的连接词：`"Frank Ocean feat."` → `"Frank Ocean"`。
///
/// 来源标签把分隔符弄丢的情况很常见（如 "Frank Ocean feat.|James Blake"），
/// 切分后前半段会留下一个光秃秃的 "feat."。这里顺手清掉，
/// 但要求连接词前面是空白，避免误伤 "Defeat" 这类正常词尾。
fn strip_trailing_collab_keyword(part: &str) -> &str {
    let trimmed = part.trim_end();
    for kw in COLLAB_KEYWORDS {
        let n = kw.len();
        if trimmed.len() <= n {
            continue;
        }
        let cut = trimmed.len() - n;
        if !trimmed.is_char_boundary(cut) || !trimmed[cut..].eq_ignore_ascii_case(kw) {
            continue;
        }
        // 连接词前面必须是空白，否则可能是 "Defeat" 这类正常词尾
        let prefix = &trimmed[..cut];
        if !prefix.ends_with(char::is_whitespace) {
            continue;
        }
        let head = prefix.trim_end();
        if !head.is_empty() {
            return head;
        }
    }
    part
}

/// 命中连接词时返回其字节长度（调用方据此跳过）。
fn match_collab_keyword(rest: &str, at_word_start: bool) -> Option<usize> {
    if !at_word_start {
        return None;
    }
    for kw in COLLAB_KEYWORDS {
        let n = kw.len();
        if rest.len() < n || !rest.is_char_boundary(n) {
            continue;
        }
        if !rest[..n].eq_ignore_ascii_case(kw) {
            continue;
        }
        let after = &rest[n..];
        if after.is_empty() || after.starts_with(char::is_whitespace) {
            return Some(n);
        }
    }
    None
}

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

pub fn extract_metadata_from_reader<R: std::io::Read + std::io::Seek>(
    reader: R,
) -> Result<AudioMetadata, String> {
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

    if let Some(tag) = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_common_separators() {
        assert_eq!(split_artist_names("A & B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A&B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A|B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A | B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A｜B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A、B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A;B;C"), vec!["A", "B", "C"]);
        assert_eq!(split_artist_names("A，B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A＆B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A|B|C|D"), vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn splits_collab_keywords_case_insensitively() {
        assert_eq!(split_artist_names("A feat. B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A Feat. B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A FEAT. B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A ft. B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A ft B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("A featuring B"), vec!["A", "B"]);
        assert_eq!(split_artist_names("feat. B"), vec!["B"]);
    }

    #[test]
    fn keeps_single_artists_intact() {
        assert_eq!(split_artist_names("Kraftwerk"), vec!["Kraftwerk"]);
        assert_eq!(split_artist_names("The Soft Machine"), vec!["The Soft Machine"]);
        assert_eq!(split_artist_names("Defeats"), vec!["Defeats"]);
        assert_eq!(split_artist_names("  A  B  "), vec!["A  B"]);
        // 空串 / 纯分隔符不产生空艺人
        assert!(split_artist_names("").is_empty());
        assert!(split_artist_names(" | & ").is_empty());
    }

    #[test]
    fn mixed_separators_and_whitespace() {
        assert_eq!(split_artist_names(" A | B feat. C & D "), vec!["A", "B", "C", "D"]);
    }

    /// 来源标签把分隔符丢掉时（"Frank Ocean feat.|James Blake"）后半段会残留一个光秃秃的连接词
    #[test]
    fn strips_dangling_collab_keyword_at_part_end() {
        assert_eq!(
            split_artist_names("Frank Ocean feat.|James Blake"),
            vec!["Frank Ocean", "James Blake"]
        );
        assert_eq!(
            split_artist_names("Frank Ocean feat|James Blake"),
            vec!["Frank Ocean", "James Blake"]
        );
        assert_eq!(
            split_artist_names("Frank Ocean featuring|Andre 3000"),
            vec!["Frank Ocean", "Andre 3000"]
        );
        // 正常词尾不能被误切
        assert_eq!(split_artist_names("Defeat"), vec!["Defeat"]);
    }

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize_artist_name("  A   B "), "A B");
    }
}
