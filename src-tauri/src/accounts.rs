//! 账号业务：批量导入解析、连接测试。

use crate::accounts_auth::get_access_token;
use crate::error::{AppError, AppResult};
use crate::models::AccountCredentials;

/// 解析单行批量导入文本。分隔符 `----`。
///
/// - 3 段：`邮箱----刷新令牌----客户端ID`（IMAP / OAuth2 标准）
/// - 4 段 + imap：`邮箱----占位密码----刷新令牌----客户端ID`（兼容旧格式）
/// - 4 段 + graph/oauth2：`邮箱----密码----client_id----刷新令牌`（Graph / Outlook_OA2）
///
/// `oauth2` 走 IMAP(XOAUTH2) 取邮件，但字段顺序与 Graph 相同。
pub fn parse_import_line(line: &str, auth_method: &str) -> AppResult<AccountCredentials> {
    let raw = auth_method.trim().to_lowercase();
    // 落库的方法标签：graph / oauth2 / imap（oauth2 由 normalize 映射为 imap 取邮件）
    let store_method = match raw.as_str() {
        "graph" => "graph",
        "oauth2" => "oauth2",
        _ => "imap",
    };
    let client_id_then_token = raw == "graph" || raw == "oauth2";
    let parts: Vec<&str> = line.split("----").map(|s| s.trim()).collect();

    let creds = match parts.len() {
        3 => AccountCredentials {
            email: parts[0].to_string(),
            refresh_token: parts[1].to_string(),
            client_id: parts[2].to_string(),
            auth_method: store_method.to_string(),
        },
        4 => {
            if client_id_then_token {
                // 邮箱----密码----client_id----refresh_token
                AccountCredentials {
                    email: parts[0].to_string(),
                    client_id: parts[2].to_string(),
                    refresh_token: parts[3].to_string(),
                    auth_method: store_method.to_string(),
                }
            } else {
                // 邮箱----占位密码----刷新令牌----客户端ID
                AccountCredentials {
                    email: parts[0].to_string(),
                    refresh_token: parts[2].to_string(),
                    client_id: parts[3].to_string(),
                    auth_method: store_method.to_string(),
                }
            }
        }
        n => {
            return Err(AppError::Parse(format!(
                "无法识别的格式（{n} 段）：{line}"
            )))
        }
    };

    if creds.email.is_empty() || creds.refresh_token.is_empty() || creds.client_id.is_empty() {
        return Err(AppError::Parse(format!("字段为空：{line}")));
    }
    if !creds.email.contains('@') {
        return Err(AppError::Parse(format!("邮箱格式错误：{}", creds.email)));
    }
    Ok(creds)
}

/// 批量解析，返回成功项与逐行错误信息。
pub fn parse_import_bulk(
    text: &str,
    auth_method: &str,
) -> (Vec<AccountCredentials>, Vec<String>) {
    let mut ok = Vec::new();
    let mut errors = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        match parse_import_line(line, auth_method) {
            Ok(c) => ok.push(c),
            Err(e) => errors.push(format!("第 {} 行：{}", i + 1, e)),
        }
    }
    (ok, errors)
}

/// 连接测试：能成功换取 access_token 即视为凭据有效。
pub async fn test_connection(creds: &AccountCredentials) -> AppResult<()> {
    get_access_token(creds).await.map(|_| ())
}
