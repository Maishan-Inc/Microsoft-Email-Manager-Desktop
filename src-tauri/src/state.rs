//! 应用状态：持有已解锁的数据库连接与派生密钥。锁定时清空。

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::crypto::DerivedKey;
use crate::error::{AppError, AppResult};

pub struct Vault {
    pub conn: Connection,
    pub key: DerivedKey,
}

pub struct AppState {
    pub db_path: PathBuf,
    pub vault: Mutex<Option<Vault>>,
}

impl AppState {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            vault: Mutex::new(None),
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

    pub fn lock_vault(&self) {
        *self.vault.lock().unwrap() = None;
    }

    pub fn set_vault(&self, vault: Vault) {
        *self.vault.lock().unwrap() = Some(vault);
    }
}
