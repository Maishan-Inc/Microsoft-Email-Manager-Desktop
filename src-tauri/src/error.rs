use serde::Serialize;

/// 统一错误类型，可直接序列化给前端
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("加密/解密失败: {0}")]
    Crypto(String),
    #[error("尚未解锁，请先输入主密码")]
    Locked,
    #[error("主密码错误")]
    BadPassword,
    #[error("已初始化，无法重复设置主密码")]
    AlreadyInitialized,
    #[error("网络错误: {0}")]
    Network(String),
    #[error("Microsoft 认证失败: {0}")]
    Auth(String),
    #[error("IMAP 错误: {0}")]
    Imap(String),
    #[error("未找到账号: {0}")]
    NotFound(String),
    #[error("数据格式错误: {0}")]
    Parse(String),
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Parse(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
