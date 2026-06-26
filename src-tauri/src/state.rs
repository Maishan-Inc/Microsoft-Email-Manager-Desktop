//! 应用状态：持有已解锁的数据库连接与派生密钥。锁定时清空。
//!
//! 另含「锁后监视」结构（令牌即焚）：锁定**不**清空，解锁/退出才清空。
//! 仅保留各被监视邮箱的短期 access_token（约 50 分钟），过期即停；
//! 用于锁定后仍能拉取新邮件并发最小化通知（只含邮箱+发件人）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;
use serde::Serialize;

use crate::crypto::DerivedKey;
use crate::error::{AppError, AppResult};

pub struct Vault {
    pub conn: Connection,
    pub key: DerivedKey,
}

/// 锁后监视的单个邮箱：短期令牌 + 已见邮件 id（用于判新）。
pub struct WatchEntry {
    pub email: String,
    pub auth_method: String,
    pub access_token: String,
    pub captured_unix: i64,
    pub seen_ids: std::collections::HashSet<String>,
}

/// 锁定期间到达的新邮件（最小信息，供锁屏列表显示；不含主题/正文）。
#[derive(Clone, Serialize)]
pub struct LockedMail {
    pub email: String,
    pub from: String,
    pub message_id: String,
    pub received_at: String,
    pub has_code: bool,
}

pub struct AppState {
    pub db_path: PathBuf,
    pub vault: Mutex<Option<Vault>>,
    /// 已用密码/恢复路径解出 DEK，但尚未通过 2FA 校验的中间态。
    pub pending: Mutex<Option<Vault>>,
    /// 锁后监视集合（email -> 短期令牌+已见 id）。锁定不清，解锁清空。
    pub watch: Mutex<HashMap<String, WatchEntry>>,
    /// 当前在邮件页打开的邮箱（纳入锁后监视）。
    pub active_mailbox: Mutex<Option<String>>,
    /// 锁定期间累计的新邮件（供锁屏列表）。
    pub locked_items: Mutex<Vec<LockedMail>>,
}

impl AppState {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            vault: Mutex::new(None),
            pending: Mutex::new(None),
            watch: Mutex::new(HashMap::new()),
            active_mailbox: Mutex::new(None),
            locked_items: Mutex::new(Vec::new()),
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.vault.lock().unwrap().is_some()
    }

    /// 在已解锁的 vault 上执行闭包；未解锁返回 Locked
    pub fn with_vault<T>(&self, f: impl FnOnce(&Vault) -> AppResult<T>) -> AppResult<T> {
        let guard = self.vault.lock().unwrap();
        let vault = guard.as_ref().ok_or(AppError::Locked)?;
        f(vault)
    }

    /// 锁定：清空 vault/pending，但**保留**锁后监视（令牌即焚）。
    pub fn lock_vault(&self) {
        *self.vault.lock().unwrap() = None;
        *self.pending.lock().unwrap() = None;
    }

    pub fn set_vault(&self, vault: Vault) {
        *self.vault.lock().unwrap() = Some(vault);
        *self.pending.lock().unwrap() = None;
        // 解锁后不再需要锁后监视与锁屏列表
        self.clear_post_lock();
    }

    /// 暂存待 2FA 校验的 vault
    pub fn set_pending(&self, vault: Vault) {
        *self.pending.lock().unwrap() = Some(vault);
    }

    /// 取出 pending（取走所有权）
    pub fn take_pending(&self) -> Option<Vault> {
        self.pending.lock().unwrap().take()
    }

    // ---------- 锁后监视（令牌即焚） ----------

    pub fn set_active_mailbox(&self, email: Option<String>) {
        *self.active_mailbox.lock().unwrap() = email;
    }
    pub fn active_mailbox(&self) -> Option<String> {
        self.active_mailbox.lock().unwrap().clone()
    }

    /// 解锁态下刷新某邮箱的令牌与基线 id（替换 seen_ids 为当前收件箱快照）。
    pub fn arm_watch(
        &self,
        email: &str,
        auth_method: &str,
        access_token: String,
        captured_unix: i64,
        ids: Vec<String>,
    ) {
        let mut w = self.watch.lock().unwrap();
        w.insert(
            email.to_string(),
            WatchEntry {
                email: email.to_string(),
                auth_method: auth_method.to_string(),
                access_token,
                captured_unix,
                seen_ids: ids.into_iter().collect(),
            },
        );
    }

    /// 当前监视集合的精简快照（email, auth_method, token, captured_unix）。
    pub fn watch_snapshot(&self) -> Vec<(String, String, String, i64)> {
        self.watch
            .lock()
            .unwrap()
            .values()
            .map(|e| (e.email.clone(), e.auth_method.clone(), e.access_token.clone(), e.captured_unix))
            .collect()
    }

    /// 标记某邮件为已见；返回 true 表示这是「新」邮件（之前未见）。
    pub fn watch_mark_seen(&self, email: &str, message_id: &str) -> bool {
        let mut w = self.watch.lock().unwrap();
        match w.get_mut(email) {
            Some(e) => e.seen_ids.insert(message_id.to_string()),
            None => false,
        }
    }

    pub fn drop_watch(&self, email: &str) {
        self.watch.lock().unwrap().remove(email);
    }

    pub fn push_locked_item(&self, item: LockedMail) {
        let mut v = self.locked_items.lock().unwrap();
        // 去重 + 限量
        if v.iter().any(|x| x.message_id == item.message_id && x.email == item.email) {
            return;
        }
        v.insert(0, item);
        v.truncate(50);
    }

    pub fn locked_items(&self) -> Vec<LockedMail> {
        self.locked_items.lock().unwrap().clone()
    }

    pub fn clear_post_lock(&self) {
        self.watch.lock().unwrap().clear();
        self.locked_items.lock().unwrap().clear();
    }
}
