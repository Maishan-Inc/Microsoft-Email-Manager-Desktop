//! 后台邮件刷新 + 系统通知。
//!
//! - 解锁后由 `spawn` 启动一个 tokio 循环，每 30s 检查一次；
//! - 对「开启通知」且到达各自轮询间隔的账号，拉取收件箱并与
//!   `mail_activity` 去重，新邮件弹系统通知并 emit `mail:new`；
//! - 首次同步（无 last_sync_at）只建立基线、不通知，避免刷屏。
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
use crate::state::AppState;

const TICK_SECS: u64 = 30;
const FETCH_LIMIT: u32 = 25;

/// 拉取某账号收件箱第一页。
pub async fn fetch_inbox(creds: &AccountCredentials, limit: u32) -> AppResult<Vec<EmailItem>> {
    if normalize_auth_method(&creds.auth_method) == "graph" {
        Ok(graph::list_emails(creds, "inbox", 1, limit).await?.emails)
    } else {
        let token = accounts_auth::get_access_token(creds).await?;
        let email = creds.email.clone();
        let resp = tokio::task::spawn_blocking(move || {
            imap_client::list_blocking(email, token, "inbox".to_string(), 1, limit)
        })
        .await
        .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))??;
        Ok(resp.emails)
    }
}

/// 同步单个账号；返回新邮件数。allow_notify 控制是否弹系统通知。
pub async fn sync_account(app: &AppHandle, email: &str, allow_notify: bool) -> AppResult<usize> {
    let state = app.state::<AppState>();

    // 短暂持锁取凭据与上次同步时间
    let (creds, last_sync) = state.with_vault(|v| {
        let creds = db::get_credentials(&v.conn, &v.key, email)?;
        let last = db::get_last_sync(&v.conn, email)?;
        Ok((creds, last))
    })?;
    let baseline = last_sync.is_none();

    let items = fetch_inbox(&creds, FETCH_LIMIT).await?;

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
                let _ = app
                    .notification()
                    .builder()
                    .title(format!("新邮件 · {email}"))
                    .body(format!("{} — {}", it.from_email, it.subject))
                    .show();
            }
        }
    }

    state.with_vault(|v| {
        db::set_last_sync(&v.conn, email)?;
        db::prune_activity(&v.conn)
    })?;

    if new_count > 0 {
        let _ = app.emit(
            "mail:new",
            serde_json::json!({ "email": email, "count": new_count }),
        );
    }
    Ok(new_count)
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
            if !state.is_unlocked() {
                continue;
            }

            // 读取全局开关与默认间隔（解锁态下可访问 app_meta）
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

            for tgt in targets {
                if is_due(&tgt.last_sync_at, tgt.interval_secs) {
                    if let Err(e) = sync_account(&app, &tgt.email, true).await {
                        log::warn!("后台同步 {} 失败: {e}", tgt.email);
                    }
                }
            }
        }
    });
}
