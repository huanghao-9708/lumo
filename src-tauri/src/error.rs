use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Pool error: {0}")]
    Pool(#[from] r2d2::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

// 实现 Serialize 以便可以作为 Result 的 E 被直接通过 Tauri 返回给前端
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Internal(s.to_string())
    }
}

impl AppError {
    /// 故障的粗类别，用于自动切歌的错误去重（CR-002）。
    ///
    /// 为什么按变体而不是按消息：消息里带路径、HTTP 状态等易变内容，
    /// 同一次故障重复出现时文本可能微差，按文本去重会刷屏；按变体则
    /// 「网络断了」和「文件找不到」在同一首上算两件事，各自都会通知一次。
    pub fn retry_class(&self) -> &'static str {
        match self {
            AppError::Database(_) => "database",
            AppError::Io(_) => "io",
            AppError::Network(_) => "network",
            AppError::Pool(_) => "pool",
            AppError::Internal(_) => "internal",
            AppError::NotFound(_) => "not_found",
        }
    }
}
