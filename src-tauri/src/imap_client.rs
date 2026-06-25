//! IMAP（XOAUTH2）取邮件：同步 `imap` crate + native-tls，运行在 spawn_blocking 线程上。
//! 移植自 main.py 的 _sync_list_emails / _sync_get_email_details。

use std::net::TcpStream;

use mail_parser::MessageParser;
use native_tls::TlsStream;

use crate::error::{AppError, AppResult};
use crate::models::{EmailDetails, EmailItem, EmailListResponse};

const IMAP_SERVER: &str = "outlook.live.com";
const IMAP_PORT: u16 = 993;

type ImapSession = imap::Session<TlsStream<TcpStream>>;

/// XOAUTH2 认证器（imap crate 约定）
struct XOAuth2 {
    user: String,
    access_token: String,
}

impl imap::Authenticator for XOAuth2 {
    type Response = String;
    fn process(&self, _challenge: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

fn connect_session(email: &str, token: &str) -> AppResult<ImapSession> {
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| AppError::Imap(format!("TLS 初始化失败: {e}")))?;
    let client = imap::connect((IMAP_SERVER, IMAP_PORT), IMAP_SERVER, &tls)
        .map_err(|e| AppError::Imap(format!("连接失败: {e}")))?;
    let auth = XOAuth2 {
        user: email.to_string(),
        access_token: token.to_string(),
    };
    client
        .authenticate("XOAUTH2", &auth)
        .map_err(|(e, _client)| AppError::Imap(format!("XOAUTH2 认证失败: {e}")))
}

fn folders_for(view: &str) -> Vec<&'static str> {
    match view {
        "inbox" => vec!["INBOX"],
        "junk" => vec!["Junk"],
        _ => vec!["INBOX", "Junk"],
    }
}

/// 解析邮件头（或完整邮件）的展示字段
fn parse_overview(raw: &[u8]) -> (String, String, String) {
    let parser = MessageParser::default();
    let Some(msg) = parser.parse(raw) else {
        return ("(无主题)".into(), "(未知发件人)".into(), String::new());
    };
    let subject = msg.subject().unwrap_or("(无主题)").to_string();
    let from = msg
        .from()
        .and_then(|a| a.first())
        .map(format_addr)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "(未知发件人)".into());
    let date = msg.date().map(|d| d.to_rfc3339()).unwrap_or_default();
    (subject, from, date)
}

fn format_addr(addr: &mail_parser::Addr) -> String {
    let name = addr.name().unwrap_or("").trim();
    let address = addr.address().unwrap_or("").trim();
    match (name.is_empty(), address.is_empty()) {
        (false, false) => format!("{name} <{address}>"),
        (true, false) => address.to_string(),
        (false, true) => name.to_string(),
        _ => String::new(),
    }
}

/// 取邮件列表（阻塞）
pub fn list_blocking(
    email: String,
    token: String,
    view: String,
    page: u32,
    page_size: u32,
) -> AppResult<EmailListResponse> {
    let mut session = connect_session(&email, &token)?;

    // 收集所有 (folder, seq)，seq 倒序近似新邮件在前
    let mut metas: Vec<(&'static str, u32)> = Vec::new();
    for f in folders_for(&view) {
        if session.examine(f).is_err() {
            continue;
        }
        let seqs = match session.search("ALL") {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut ids: Vec<u32> = seqs.into_iter().collect();
        ids.sort_unstable_by(|a, b| b.cmp(a));
        for id in ids {
            metas.push((f, id));
        }
    }

    let total = metas.len() as u32;
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(metas.len());
    let page_metas: &[(&'static str, u32)] = if start < metas.len() {
        &metas[start..end]
    } else {
        &[]
    };

    // 按文件夹分组（页内文件夹块连续，至多两组）
    let mut groups: Vec<(&'static str, Vec<u32>)> = Vec::new();
    for (f, seq) in page_metas {
        match groups.last_mut() {
            Some(last) if last.0 == *f => last.1.push(*seq),
            _ => groups.push((*f, vec![*seq])),
        }
    }

    let mut emails: Vec<EmailItem> = Vec::new();
    for (folder, seqs) in groups {
        if session.examine(folder).is_err() {
            continue;
        }
        let seq_set = seqs
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let fetches = match session.fetch(&seq_set, "(FLAGS RFC822.HEADER)") {
            Ok(f) => f,
            Err(e) => return Err(AppError::Imap(format!("FETCH 失败: {e}"))),
        };

        let mut group_items: Vec<(u32, EmailItem)> = Vec::new();
        for fe in fetches.iter() {
            let seq = fe.message;
            let is_read = fe
                .flags()
                .iter()
                .any(|fl| matches!(fl, imap::types::Flag::Seen));
            let header = fe.header().unwrap_or(&[]);
            let (subject, from_email, date) = parse_overview(header);
            group_items.push((
                seq,
                EmailItem {
                    message_id: format!("{folder}-{seq}"),
                    folder: folder.to_string(),
                    subject,
                    from_email,
                    date,
                    is_read,
                    has_attachments: false,
                },
            ));
        }
        // 维持 seq 倒序
        group_items.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        emails.extend(group_items.into_iter().map(|(_, item)| item));
    }

    let _ = session.logout();

    Ok(EmailListResponse {
        email_id: email,
        folder_view: view,
        page,
        page_size,
        total_emails: total,
        emails,
    })
}

/// 取邮件详情（阻塞）。message_id 形如 `INBOX-123`。
pub fn detail_blocking(
    email: String,
    token: String,
    message_id: String,
) -> AppResult<EmailDetails> {
    let (folder, seq) = message_id
        .split_once('-')
        .ok_or_else(|| AppError::Parse("message_id 格式错误".into()))?;

    let mut session = connect_session(&email, &token)?;
    session
        .examine(folder)
        .map_err(|e| AppError::Imap(format!("选择文件夹失败: {e}")))?;

    let fetches = session
        .fetch(seq, "RFC822")
        .map_err(|e| AppError::Imap(format!("FETCH 失败: {e}")))?;
    let fe = fetches
        .iter()
        .next()
        .ok_or_else(|| AppError::NotFound(message_id.clone()))?;
    let raw = fe
        .body()
        .or_else(|| fe.text())
        .ok_or_else(|| AppError::Imap("邮件无正文".into()))?;

    let parser = MessageParser::default();
    let msg = parser
        .parse(raw)
        .ok_or_else(|| AppError::Parse("邮件解析失败".into()))?;

    let subject = msg.subject().unwrap_or("(无主题)").to_string();
    let from_email = msg
        .from()
        .and_then(|a| a.first())
        .map(format_addr)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "(未知发件人)".into());
    let to_email = msg
        .to()
        .and_then(|a| a.first())
        .map(format_addr)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "(未知收件人)".into());
    let date = msg.date().map(|d| d.to_rfc3339()).unwrap_or_default();
    let body_plain = msg.body_text(0).map(|c| c.into_owned());
    let body_html = msg.body_html(0).map(|c| c.into_owned());

    let _ = session.logout();

    Ok(EmailDetails {
        message_id,
        subject,
        from_email,
        to_email,
        date,
        body_plain: body_plain.filter(|s| !s.is_empty()),
        body_html: body_html.filter(|s| !s.is_empty()),
    })
}

/// 健康探测：OAuth + IMAP NOOP
pub fn probe_blocking(email: String, token: String) -> AppResult<()> {
    let mut session = connect_session(&email, &token)?;
    session
        .noop()
        .map_err(|e| AppError::Imap(format!("NOOP 失败: {e}")))?;
    let _ = session.logout();
    Ok(())
}
