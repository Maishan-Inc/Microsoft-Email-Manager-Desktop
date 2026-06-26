//! 后台邮件刷新 + 系统通知。
//!
//! - 解锁后由 `spawn` 启动一个 tokio 循环，每 5s 检查一次；
//! - **解锁态**：对「开启后台刷新（铃铛）」的账号拉取收件箱并去重，新邮件弹系统
//!   通知并 emit `mail:new`；同时为这些账号 + 当前打开的邮箱「武装」锁后监视
//!   （缓存短期 access_token 与已见 id）。
//! - **锁定态（令牌即焚）**：用已缓存的短期令牌继续拉取被监视邮箱；令牌约 50 分钟
//!   后过期即停。新邮件只发**最小化**通知（邮箱+发件人，不含主题），并写入锁屏列表。
//! - 网络调用不跨 vault 锁：取凭据时短暂持锁，抓取在锁外。

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::accounts_auth;
use crate::db;
use crate::error::{AppError, AppResult};
use crate::graph;
use crate::imap_client;
use crate::models::{normalize_auth_method, AccountCredentials, EmailItem};
use crate::state::{AppState, LockedMail};

const TICK_SECS: u64 = 5;
const FETCH_LIMIT: u32 = 25;
/// 锁后令牌存活上限（秒）：超过即认为 access_token 失效，停止该邮箱的锁后监视。
const TOKEN_TTL_SECS: i64 = 50 * 60;

fn now_unix() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

/// 主题里是否疑似含验证码（仅基于主题，不泄露主题本身，只回布尔）。
fn subject_has_code(subject: &str) -> bool {
    let lower = subject.to_lowercase();
    let kw = ["验证码", "校验码", "动态码", "verification", "code", "otp", "passcode", "one-time", "安全码", "登录码"];
    let has_kw = kw.iter().any(|k| lower.contains(k));
    let has_digit = subject.bytes().any(|b| b.is_ascii_digit());
    // 独立的 6 位数字串（最常见的一次性验证码）
    let six = subject
        .as_bytes()
        .split(|b| !b.is_ascii_digit())
        .any(|seg| seg.len() == 6);
    (has_kw && has_digit) || six
}

/// 取 access_token 并拉收件箱（解锁态用：之后可武装锁后监视）。
async fn get_token_and_inbox(
    creds: &AccountCredentials,
    limit: u32,
) -> AppResult<(String, Vec<EmailItem>)> {
    let token = accounts_auth::get_access_token(creds).await?;
    let items = fetch_inbox_with_token(&creds.email, &token, &creds.auth_method, limit).await?;
    Ok((token, items))
}

/// 用已有 access_token 拉收件箱（锁后场景：无需 refresh_token）。
pub async fn fetch_inbox_with_token(
    email: &str,
    token: &str,
    auth_method: &str,
    limit: u32,
) -> AppResult<Vec<EmailItem>> {
    if normalize_auth_method(auth_method) == "graph" {
        graph::list_inbox_with_token(token, limit).await
    } else {
        let email = email.to_string();
        let token = token.to_string();
        tokio::task::spawn_blocking(move || {
            imap_client::list_blocking(email, token, "inbox".to_string(), 1, limit)
        })
        .await
        .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))?
        .map(|resp| resp.emails)
    }
}

/// 同步单个账号（解锁态）；返回新邮件数。allow_notify 控制是否弹系统通知。
/// 同时刷新锁后监视（令牌 + 已见 id 基线）。
pub async fn sync_account(app: &AppHandle, email: &str, allow_notify: bool) -> AppResult<usize> {
    let state = app.state::<AppState>();
    let icon_path = notify_icon(app);

    let (creds, last_sync) = state.with_vault(|v| {
        let creds = db::get_credentials(&v.conn, &v.key, email)?;
        let last = db::get_last_sync(&v.conn, email)?;
        Ok((creds, last))
    })?;
    let baseline = last_sync.is_none();

    let (token, items) = get_token_and_inbox(&creds, FETCH_LIMIT).await?;

    let mut new_count = 0usize;
    for it in &items {
        let inserted = state.with_vault(|v| {
            db::record_activity(
                &v.conn,
                email,
                &it.message_id,
                &it.subject,
                &it.from_email,
                &it.date,
                !baseline,
            )
        })?;
        if inserted && !baseline {
            new_count += 1;
            if allow_notify {
                let mut builder = app
                    .notification()
                    .builder()
                    .title(format!("新邮件 · {email}"))
                    .body(format!("{} — {}", it.from_email, it.subject));
                if let Some(p) = &icon_path {
                    builder = builder.icon(p.clone());
                }
                let _ = builder.show();
            }
        }
    }

    state.with_vault(|v| {
        db::set_last_sync(&v.conn, email)?;
        db::prune_activity(&v.conn)
    })?;

    // 武装锁后监视：缓存短期令牌 + 当前收件箱 id 作为基线
    let ids: Vec<String> = items.iter().map(|i| i.message_id.clone()).collect();
    state.arm_watch(email, &creds.auth_method, token, now_unix(), ids);

    if new_count > 0 {
        let _ = app.emit(
            "mail:new",
            serde_json::json!({ "email": email, "count": new_count }),
        );
    }
    Ok(new_count)
}

/// 仅武装锁后监视（解锁态，用于「当前打开但未开铃铛」的邮箱；不弹通知、不写活动）。
async fn arm_only(app: &AppHandle, email: &str) {
    let state = app.state::<AppState>();
    let creds = match state.with_vault(|v| db::get_credentials(&v.conn, &v.key, email)) {
        Ok(c) => c,
        Err(_) => return,
    };
    if let Ok((token, items)) = get_token_and_inbox(&creds, FETCH_LIMIT).await {
        let ids: Vec<String> = items.iter().map(|i| i.message_id.clone()).collect();
        state.arm_watch(email, &creds.auth_method, token, now_unix(), ids);
    }
}

/// 手动同步全部账号（不弹通知，仅刷新统计）。返回新邮件总数。
pub async fn sync_all(app: &AppHandle) -> AppResult<usize> {
    let state = app.state::<AppState>();
    let emails = state.with_vault(|v| db::list_account_emails(&v.conn))?;
    let mut total = 0usize;
    for email in emails {
        match sync_account(app, &email, false).await {
            Ok(n) => total += n,
            Err(e) => log::warn!("同步 {email} 失败: {e}"),
        }
    }
    Ok(total)
}

/// 锁定态：用缓存的短期令牌继续拉取被监视邮箱，发最小化通知 + 写锁屏列表。
async fn locked_tick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let icon_path = notify_icon(app);
    let now = now_unix();

    for (email, auth_method, token, captured) in state.watch_snapshot() {
        if now - captured > TOKEN_TTL_SECS {
            state.drop_watch(&email); // 令牌即焚：过期停更
            continue;
        }
        let items = match fetch_inbox_with_token(&email, &token, &auth_method, FETCH_LIMIT).await {
            Ok(i) => i,
            Err(_) => continue, // 令牌失效/网络错误：静默跳过
        };
        for it in &items {
            // 仅「新」邮件（基线之外）才提醒
            if state.watch_mark_seen(&email, &it.message_id) {
                let has_code = subject_has_code(&it.subject);
                state.push_locked_item(LockedMail {
                    email: email.clone(),
                    from: it.from_email.clone(),
                    message_id: it.message_id.clone(),
                    received_at: it.date.clone(),
                    has_code,
                });
                // 锁态通知：只含邮箱 + 发件人，不含主题
                let mut builder = app
                    .notification()
                    .builder()
                    .title(format!("新邮件 · {email}"))
                    .body(it.from_email.clone());
                if let Some(p) = &icon_path {
                    builder = builder.icon(p.clone());
                }
                let _ = builder.show();
            }
        }
        let _ = app.emit("mail:locked-new", serde_json::json!({ "email": email }));
    }
}

fn notify_icon(app: &AppHandle) -> Option<String> {
    app.path()
        .resource_dir()
        .ok()
        .map(|d| d.join("icons/128x128.png"))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
}

fn is_due(last: &Option<String>, interval: i64) -> bool {
    match last {
        None => true,
        Some(s) => {
            use time::format_description::well_known::Rfc3339;
            match time::OffsetDateTime::parse(s, &Rfc3339) {
                Ok(t) => (time::OffsetDateTime::now_utc() - t).whole_seconds() >= interval,
                Err(_) => true,
            }
        }
    }
}

/// 启动后台轮询循环（应用启动时调用一次）。
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(TICK_SECS)).await;

            let state = app.state::<AppState>();

            // 锁定态：令牌即焚续期
            if !state.is_unlocked() {
                locked_tick(&app).await;
                continue;
            }

            // 解锁态：读取全局开关与默认间隔
            let (enabled, default_secs) = match state.with_vault(|v| {
                let enabled = db::get_meta(&v.conn, "bg_refresh_enabled")?
                    .map(|s| s != "0")
                    .unwrap_or(true);
                let secs = db::get_meta(&v.conn, "bg_refresh_interval_secs")?
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(db::DEFAULT_POLL_SECS);
                Ok((enabled, secs))
            }) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if !enabled {
                continue;
            }

            let targets = match state.with_vault(|v| db::list_notify_targets(&v.conn, default_secs))
            {
                Ok(t) => t,
                Err(_) => continue,
            };

            let notify_emails: Vec<String> = targets.iter().map(|t| t.email.clone()).collect();
            for tgt in targets {
                if is_due(&tgt.last_sync_at, tgt.interval_secs) {
                    if let Err(e) = sync_account(&app, &tgt.email, true).await {
                        log::warn!("后台同步 {} 失败: {e}", tgt.email);
                    }
                }
            }

            // 当前打开的邮箱（未开铃铛时也武装锁后监视）
            if let Some(active) = state.active_mailbox() {
                if !notify_emails.contains(&active) {
                    arm_only(&app, &active).await;
                }
            }
        }
    });
}
