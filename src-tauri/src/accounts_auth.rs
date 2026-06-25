//! Microsoft OAuth2 token 刷新（移植自原 main.py:get_access_token）。

use crate::error::{AppError, AppResult};
use crate::models::{normalize_auth_method, AccountCredentials};

const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const COMMON_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const IMAP_SCOPE: &str = "https://outlook.office.com/IMAP.AccessAsUser.All offline_access";
const GRAPH_SCOPE: &str = "https://graph.microsoft.com/Mail.Read offline_access";

/// 用 refresh_token 换取 access_token；按多 URL × 多 scope 顺序重试。
pub async fn get_access_token(creds: &AccountCredentials) -> AppResult<String> {
    let auth_method = normalize_auth_method(&creds.auth_method);

    let base: Vec<(&str, &str)> = vec![
        ("client_id", creds.client_id.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", creds.refresh_token.as_str()),
    ];

    // 每个 scope 变体（None 表示不带 scope）
    let (token_urls, scopes): (Vec<&str>, Vec<Option<&str>>) = if auth_method == "graph" {
        (
            vec![TOKEN_URL, COMMON_TOKEN_URL],
            vec![
                Some(GRAPH_SCOPE),
                Some("offline_access openid profile email https://graph.microsoft.com/Mail.Read"),
                Some("offline_access https://graph.microsoft.com/.default"),
                None,
            ],
        )
    } else {
        (vec![TOKEN_URL], vec![Some(IMAP_SCOPE), None])
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut last_err = String::from("认证失败");

    for url in &token_urls {
        for scope in &scopes {
            let mut form = base.clone();
            if let Some(s) = scope {
                form.push(("scope", s));
            }
            let resp = match client.post(*url).form(&form).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_err = e.to_string();
                    continue;
                }
            };

            if resp.status().is_success() {
                let data: serde_json::Value = resp.json().await?;
                if let Some(tok) = data.get("access_token").and_then(|v| v.as_str()) {
                    return Ok(tok.to_string());
                }
                last_err = "响应中缺少 access_token".into();
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                last_err = format!("HTTP {status}: {}", extract_error_detail(&body));
            }
        }
    }

    Err(AppError::Auth(last_err))
}

fn extract_error_detail(body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(desc) = v.get("error_description").and_then(|x| x.as_str()) {
            return desc.lines().next().unwrap_or(desc).to_string();
        }
        if let Some(err) = v.get("error").and_then(|x| x.as_str()) {
            return err.to_string();
        }
    }
    body.chars().take(200).collect()
}
