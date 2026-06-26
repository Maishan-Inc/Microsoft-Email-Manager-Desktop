//! Microsoft Graph API 取邮件（移植自 main.py 的 list_graph_emails / get_graph_email_details）。

use serde_json::Value;

use crate::accounts_auth::get_access_token;
use crate::error::{AppError, AppResult};
use crate::models::{AccountCredentials, EmailDetails, EmailItem, EmailListResponse};

const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";

/// folder 关键字 -> (Graph 文件夹键, 展示名)
fn normalize_folder(folder: &str) -> (&'static str, &'static str) {
    if folder == "junk" {
        ("junkemail", "Junk")
    } else {
        ("inbox", "INBOX")
    }
}

fn build_message_id(folder_key: &str, graph_id: &str) -> String {
    format!("graph:{folder_key}:{graph_id}")
}

/// 解析 `graph:{folder}:{id}` -> (folder, id)
fn parse_message_id(message_id: &str) -> AppResult<(String, String)> {
    let rest = message_id
        .strip_prefix("graph:")
        .ok_or_else(|| AppError::Parse("非 Graph message_id".into()))?;
    let mut it = rest.splitn(2, ':');
    let folder = it.next().unwrap_or_default().to_string();
    let id = it
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Parse("Graph message_id 格式错误".into()))?
        .to_string();
    Ok((folder, id))
}

fn format_address(addr: Option<&Value>) -> String {
    let Some(ea) = addr.and_then(|a| a.get("emailAddress")) else {
        return String::new();
    };
    let address = ea.get("address").and_then(|v| v.as_str()).unwrap_or("").trim();
    let name = ea.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
    match (name.is_empty(), address.is_empty()) {
        (false, false) => format!("{name} <{address}>"),
        (true, false) => address.to_string(),
        (false, true) => name.to_string(),
        _ => String::new(),
    }
}

fn format_recipients(list: Option<&Value>) -> String {
    let Some(arr) = list.and_then(|v| v.as_array()) else {
        return String::new();
    };
    arr.iter()
        .map(|r| format_address(Some(r)))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

fn strip_html(s: &str) -> String {
    // 简单去标签：用于 HTML 正文回退为纯文本
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

async fn graph_get(
    client: &reqwest::Client,
    token: &str,
    path: &str,
    query: &[(&str, String)],
) -> AppResult<Value> {
    let resp = client
        .get(format!("{GRAPH_BASE}{path}"))
        .bearer_auth(token)
        .query(query)
        .send()
        .await?;
    if !resp.status().is_success() {
        let code = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let detail = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| {
                v.get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or(body);
        return Err(AppError::Network(format!("Graph HTTP {code}: {detail}")));
    }
    Ok(resp.json::<Value>().await?)
}

fn message_to_item(msg: &Value, folder_key: &str) -> Option<EmailItem> {
    let graph_id = msg.get("id").and_then(|v| v.as_str())?.trim();
    if graph_id.is_empty() {
        return None;
    }
    let (norm_key, display) = normalize_folder(folder_key);
    let from_email = format_address(msg.get("from"));
    Some(EmailItem {
        message_id: build_message_id(norm_key, graph_id),
        folder: display.to_string(),
        subject: msg
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("(无主题)")
            .to_string(),
        from_email: if from_email.is_empty() {
            "(未知发件人)".to_string()
        } else {
            from_email
        },
        date: msg
            .get("receivedDateTime")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        is_read: msg.get("isRead").and_then(|v| v.as_bool()).unwrap_or(false),
        has_attachments: msg
            .get("hasAttachments")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// 取单个文件夹的邮件与总数
async fn list_folder(
    client: &reqwest::Client,
    token: &str,
    folder: &str,
    page: u32,
    page_size: u32,
    skip_override: Option<u32>,
    top_override: Option<u32>,
) -> AppResult<(Vec<EmailItem>, u32)> {
    let (folder_key, _display) = normalize_folder(folder);

    // 文件夹元信息（总数）
    let meta = graph_get(
        client,
        token,
        &format!("/me/mailFolders/{folder_key}"),
        &[("$select", "id,displayName,totalItemCount".to_string())],
    )
    .await?;
    let total = meta
        .get("totalItemCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let top = top_override.unwrap_or(page_size);
    let mut query: Vec<(&str, String)> = vec![
        (
            "$select",
            "id,subject,from,receivedDateTime,isRead,hasAttachments".to_string(),
        ),
        ("$orderby", "receivedDateTime DESC".to_string()),
        ("$top", top.to_string()),
    ];
    let skip = skip_override.unwrap_or_else(|| if page > 1 { (page - 1) * page_size } else { 0 });
    if skip > 0 {
        query.push(("$skip", skip.to_string()));
    }

    let payload = graph_get(
        client,
        token,
        &format!("/me/mailFolders/{folder_key}/messages"),
        &query,
    )
    .await?;

    let items = payload
        .get("value")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| message_to_item(m, folder))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok((items, total))
}

/// 取邮件列表（inbox / junk / all）
pub async fn list_emails(
    creds: &AccountCredentials,
    folder: &str,
    page: u32,
    page_size: u32,
) -> AppResult<EmailListResponse> {
    let token = get_access_token(creds).await?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    if folder == "inbox" || folder == "junk" {
        let (emails, total) =
            list_folder(&client, &token, folder, page, page_size, None, None).await?;
        return Ok(EmailListResponse {
            email_id: creds.email.clone(),
            folder_view: folder.to_string(),
            page,
            page_size,
            total_emails: total,
            emails,
        });
    }

    // all：合并 inbox + junk
    let fetch_limit = page * page_size;
    let (inbox, inbox_total) =
        list_folder(&client, &token, "inbox", 1, page_size, Some(0), Some(fetch_limit)).await?;
    let (junk, junk_total) =
        list_folder(&client, &token, "junk", 1, page_size, Some(0), Some(fetch_limit)).await?;

    let mut all: Vec<EmailItem> = inbox.into_iter().chain(junk).collect();
    all.sort_by(|a, b| b.date.cmp(&a.date));

    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(all.len());
    let emails = if start < all.len() {
        all[start..end].to_vec()
    } else {
        Vec::new()
    };

    Ok(EmailListResponse {
        email_id: creds.email.clone(),
        folder_view: folder.to_string(),
        page,
        page_size,
        total_emails: inbox_total + junk_total,
        emails,
    })
}

/// 取邮件详情
pub async fn get_details(creds: &AccountCredentials, message_id: &str) -> AppResult<EmailDetails> {
    let (_folder, graph_id) = parse_message_id(message_id)?;
    let token = get_access_token(creds).await?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let encoded_id: String =
        url_encode(&graph_id);
    let payload = graph_get(
        &client,
        &token,
        &format!("/me/messages/{encoded_id}"),
        &[(
            "$select",
            "id,subject,from,toRecipients,receivedDateTime,body".to_string(),
        )],
    )
    .await?;

    let body = payload.get("body");
    let content = body
        .and_then(|b| b.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let content_type = body
        .and_then(|b| b.get("contentType"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    let (body_html, body_plain) = if content_type == "html" {
        let plain = strip_html(&content);
        (Some(content), Some(plain))
    } else {
        (None, Some(content))
    };

    let from_email = format_address(payload.get("from"));
    let to_email = format_recipients(payload.get("toRecipients"));

    Ok(EmailDetails {
        message_id: message_id.to_string(),
        subject: payload
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("(无主题)")
            .to_string(),
        from_email: if from_email.is_empty() {
            "(未知发件人)".to_string()
        } else {
            from_email
        },
        to_email: if to_email.is_empty() {
            "(未知收件人)".to_string()
        } else {
            to_email
        },
        date: payload
            .get("receivedDateTime")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        body_plain: body_plain.filter(|s| !s.is_empty()),
        body_html: body_html.filter(|s| !s.is_empty()),
    })
}

/// 健康探测：用已有 access_token 请求收件箱元信息
pub async fn probe(token: &str) -> AppResult<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    graph_get(
        &client,
        token,
        "/me/mailFolders/inbox",
        &[("$select", "id,displayName,totalItemCount".to_string())],
    )
    .await
    .map(|_| ())
}

/// 用已有 access_token 直接拉收件箱（锁后令牌即焚场景，无需 refresh_token）。
pub async fn list_inbox_with_token(token: &str, limit: u32) -> AppResult<Vec<EmailItem>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let (items, _total) = list_folder(&client, token, "inbox", 1, limit, None, None).await?;
    Ok(items)
}

/// 最小 URL 百分号编码（仅对 path 段中的非安全字符），避免引入额外依赖。
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
