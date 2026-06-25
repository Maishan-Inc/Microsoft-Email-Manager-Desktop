//! 账号业务：批量导入解析、连接测试。

use crate::accounts_auth::get_access_token;
use crate::error::{AppError, AppResult};
use crate::models::{normalize_auth_method, AccountCredentials};

/// 解析单行批量导入文本。分隔符 `----`。
///
/// - 3 段：`邮箱----刷新令牌----客户端ID`（IMAP 标准）
/// - 4 段 + imap：`邮箱----占位密码----刷新令牌----客户端ID`（兼容旧格式）
/// - 4 段 + graph：`邮箱----密码----client_id----刷新令牌`（Graph / Outlook_OA2）
pub fn parse_import_line(line: &str, auth_method: &str) -> AppResult<AccountCredentials> {
    let auth_method = normalize_auth_method(auth_method);
    let parts: Vec<&str> = line.split("----").map(|s| s.trim()).collect();

    let creds = match parts.len() {
        3 => AccountCredentials {
            email: parts[0].to_string(),
            refresh_token: parts[1].to_string(),
            client_id: parts[2].to_string(),
            auth_method: auth_method.clone(),
        },
        4 => {
            if auth_method == "graph" {
                // 邮箱----密码----client_id----refresh_token
                AccountCredentials {
                    email: parts[0].to_string(),
                    client_id: parts[2].to_string(),
                    refresh_token: parts[3].to_string(),
                    auth_method: auth_method.clone(),
                }
            } else {
                // 邮箱----占位密码----刷新令牌----客户端ID
                AccountCredentials {
                    email: parts[0].to_string(),
                    refresh_token: parts[2].to_string(),
                    client_id: parts[3].to_string(),
                    auth_method: auth_method.clone(),
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
