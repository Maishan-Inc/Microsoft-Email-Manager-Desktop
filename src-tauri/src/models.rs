use serde::{Deserialize, Serialize};

pub const DEFAULT_AUTH_METHOD: &str = "imap";

/// 规范化接入方式
pub fn normalize_auth_method(m: &str) -> String {
    let m = m.trim().to_lowercase();
    match m.as_str() {
        "graph" => "graph".to_string(),
        "imap" | "oauth2" | "" => "imap".to_string(),
        other => other.to_string(),
    }
}

/// 账号凭据（含敏感字段，仅在内存/网络调用时存在明文）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountCredentials {
    pub email: String,
    pub refresh_token: String,
    pub client_id: String,
    #[serde(default = "default_auth")]
    pub auth_method: String,
}

fn default_auth() -> String {
    DEFAULT_AUTH_METHOD.to_string()
}

/// 账号对外信息（不含 refresh_token）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub email: String,
    pub auth_method: String,
    pub client_id: String,
    pub status: String,
    pub category_key: Option<String>,
    pub tag_keys: Vec<String>,
    pub health_score: i64,
    pub health_summary: String,
    pub health_checked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 分类 / 标签定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationOption {
    pub key: String,
    pub name_zh: String,
    pub name_en: String,
    pub remark: Option<String>,
    pub created_at: Option<String>,
}

/// 邮件列表项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailItem {
    pub message_id: String,
    pub folder: String,
    pub subject: String,
    pub from_email: String,
    pub date: String,
    pub is_read: bool,
    pub has_attachments: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailListResponse {
    pub email_id: String,
    pub folder_view: String,
    pub page: u32,
    pub page_size: u32,
    pub total_emails: u32,
    pub emails: Vec<EmailItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailDetails {
    pub message_id: String,
    pub subject: String,
    pub from_email: String,
    pub to_email: String,
    pub date: String,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
}
