/// 文件优先级评分模块

/// 计算文件的优先级评分。分数越高，越优先被选为主文件 (primary_file_id)
/// 本地文件优先级高于 WebDAV。
/// 无损格式优先级高于有损格式。
pub fn file_priority_score(source_kind: &str, file_ext: &str) -> i32 {
    let kind_score = match source_kind {
        "local" => 100,
        _ => 0, // webdav
    };
    
    let format_score = match file_ext.to_lowercase().as_str() {
        "flac" | "wav" | "alac" | "ape" => 50,
        "m4a" | "aac" => 30,
        "mp3" | "ogg" => 10,
        _ => 0,
    };
    
    kind_score + format_score
}
