//! Tauri 命令：前端通过 invoke 调用。
//! 约定：加密/解密等 CPU 操作用同步命令；网络操作用 async 且不跨 await 持锁。

use serde::Serialize;
use tauri::State;

use crate::accounts;
use crate::accounts_auth;
use crate::crypto;
use crate::db;
use crate::error::{AppError, AppResult};
use crate::export;
use crate::graph;
use crate::imap_client;
use crate::models::{
    normalize_auth_method, AccountCredentials, AccountInfo, ClassificationOption, EmailDetails,
    EmailListResponse,
};
use crate::state::{AppState, Vault};

#[derive(Serialize)]
pub struct AppStatus {
    pub initialized: bool,
    pub unlocked: bool,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub added: usize,
    pub total: usize,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct Catalog {
    pub categories: Vec<ClassificationOption>,
    pub tags: Vec<ClassificationOption>,
}

// ---------- 解锁 / 主密码 ----------

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> AppResult<AppStatus> {
    let conn = db::open(&state.db_path)?;
    Ok(AppStatus {
        initialized: db::is_initialized(&conn)?,
        unlocked: state.is_unlocked(),
    })
}

#[tauri::command]
pub fn setup_master_password(state: State<'_, AppState>, password: String) -> AppResult<()> {
    if password.len() < 8 {
        return Err(AppError::Other("主密码至少 8 位".into()));
    }
    let conn = db::open(&state.db_path)?;
    if db::is_initialized(&conn)? {
        return Err(AppError::AlreadyInitialized);
    }
    let salt = crypto::new_salt();
    let key = crypto::derive_key(&password, &salt)?;
    db::init_master(&conn, &key, &salt)?;
    state.set_vault(Vault { conn, key });
    Ok(())
}

#[tauri::command]
pub fn unlock(state: State<'_, AppState>, password: String) -> AppResult<()> {
    let conn = db::open(&state.db_path)?;
    let salt_b64 = db::get_meta(&conn, "master_salt")?.ok_or(AppError::BadPassword)?;
    let salt = crypto::b64_decode(&salt_b64)?;
    let key = crypto::derive_key(&password, &salt)?;
    let verifier = db::get_meta(&conn, "verifier")?.ok_or(AppError::BadPassword)?;
    crypto::verify(&key, &verifier)?;
    state.set_vault(Vault { conn, key });
    Ok(())
}

#[tauri::command]
pub fn lock(state: State<'_, AppState>) -> AppResult<()> {
    state.lock_vault();
    Ok(())
}

// ---------- 账号 ----------

#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> AppResult<Vec<AccountInfo>> {
    state.with_vault(|v| db::list_accounts(&v.conn, &v.key))
}

#[tauri::command]
pub fn add_account(
    state: State<'_, AppState>,
    creds: AccountCredentials,
    category_key: Option<String>,
    tag_keys: Vec<String>,
) -> AppResult<()> {
    state.with_vault(|v| {
        db::upsert_account(&v.conn, &v.key, &creds, category_key.as_deref(), &tag_keys)
    })
}

#[tauri::command]
pub fn delete_account(state: State<'_, AppState>, email: String) -> AppResult<()> {
    state.with_vault(|v| db::delete_account(&v.conn, &email))
}

#[tauri::command]
pub fn update_classification(
    state: State<'_, AppState>,
    email: String,
    category_key: Option<String>,
    tag_keys: Vec<String>,
) -> AppResult<()> {
    state.with_vault(|v| {
        db::update_classification(&v.conn, &email, category_key.as_deref(), &tag_keys)
    })
}

#[tauri::command]
pub fn import_accounts(
    state: State<'_, AppState>,
    text: String,
    auth_method: String,
) -> AppResult<ImportResult> {
    let (creds_list, mut errors) = accounts::parse_import_bulk(&text, &auth_method);
    let mut added = 0usize;
    state.with_vault(|v| {
        for c in &creds_list {
            match db::upsert_account(&v.conn, &v.key, c, None, &[]) {
                Ok(_) => added += 1,
                Err(e) => errors.push(format!("{}: {}", c.email, e)),
            }
        }
        Ok(())
    })?;
    Ok(ImportResult {
        added,
        total: creds_list.len(),
        errors,
    })
}

/// 测试一组凭据（添加前校验）
#[tauri::command]
pub async fn test_credentials(creds: AccountCredentials) -> AppResult<()> {
    accounts::test_connection(&creds).await
}

/// 测试已存账号连接
#[tauri::command]
pub async fn test_account(state: State<'_, AppState>, email: String) -> AppResult<()> {
    let creds = state.with_vault(|v| db::get_credentials(&v.conn, &v.key, &email))?;
    accounts::test_connection(&creds).await
}

// ---------- 分类 / 标签 ----------

#[tauri::command]
pub fn get_catalog(state: State<'_, AppState>) -> AppResult<Catalog> {
    state.with_vault(|v| {
        let (categories, tags) = db::list_catalog(&v.conn)?;
        Ok(Catalog { categories, tags })
    })
}

#[tauri::command]
pub fn add_category(state: State<'_, AppState>, opt: ClassificationOption) -> AppResult<()> {
    state.with_vault(|v| db::add_option(&v.conn, "categories", &opt))
}

#[tauri::command]
pub fn add_tag(state: State<'_, AppState>, opt: ClassificationOption) -> AppResult<()> {
    state.with_vault(|v| db::add_option(&v.conn, "tags", &opt))
}

#[tauri::command]
pub fn delete_category(state: State<'_, AppState>, key: String) -> AppResult<()> {
    state.with_vault(|v| db::delete_option(&v.conn, "categories", &key))
}

#[tauri::command]
pub fn delete_tag(state: State<'_, AppState>, key: String) -> AppResult<()> {
    state.with_vault(|v| db::delete_option(&v.conn, "tags", &key))
}

// ---------- 邮件 ----------

/// 取邮件列表。folder: "inbox" | "junk" | "all"
#[tauri::command]
pub async fn list_emails(
    state: State<'_, AppState>,
    email: String,
    folder: String,
    page: u32,
    page_size: u32,
) -> AppResult<EmailListResponse> {
    let creds = state.with_vault(|v| db::get_credentials(&v.conn, &v.key, &email))?;
    let page = page.max(1);
    let page_size = page_size.clamp(1, 100);

    if normalize_auth_method(&creds.auth_method) == "graph" {
        graph::list_emails(&creds, &folder, page, page_size).await
    } else {
        let token = accounts_auth::get_access_token(&creds).await?;
        let email = creds.email.clone();
        tokio::task::spawn_blocking(move || {
            imap_client::list_blocking(email, token, folder, page, page_size)
        })
        .await
        .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))?
    }
}

/// 取邮件详情
#[tauri::command]
pub async fn get_email_details(
    state: State<'_, AppState>,
    email: String,
    message_id: String,
) -> AppResult<EmailDetails> {
    let creds = state.with_vault(|v| db::get_credentials(&v.conn, &v.key, &email))?;
    if normalize_auth_method(&creds.auth_method) == "graph" {
        graph::get_details(&creds, &message_id).await
    } else {
        let token = accounts_auth::get_access_token(&creds).await?;
        let email = creds.email.clone();
        tokio::task::spawn_blocking(move || {
            imap_client::detail_blocking(email, token, message_id)
        })
        .await
        .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))?
    }
}

// ---------- 导出 ----------

/// 导出账号到文件。format: "json" | "csv"；含凭据时建议 encrypt=true。
#[tauri::command]
pub fn export_accounts(
    state: State<'_, AppState>,
    path: String,
    format: String,
    include_credentials: bool,
    encrypt: bool,
) -> AppResult<()> {
    state.with_vault(|v| {
        export::export_to_file(v, &path, &format, include_credentials, encrypt)
    })
}

// ---------- 健康检查 ----------

#[derive(Serialize)]
pub struct HealthResult {
    pub email: String,
    pub score: i64,
    pub summary: String,
}

/// 检查单个账号健康度：OAuth 刷新 + (Graph 探测 / IMAP NOOP)，结果写库。
#[tauri::command]
pub async fn check_account_health(
    state: State<'_, AppState>,
    email: String,
) -> AppResult<HealthResult> {
    let creds = state.with_vault(|v| db::get_credentials(&v.conn, &v.key, &email))?;

    // 1) OAuth 刷新
    let token = match accounts_auth::get_access_token(&creds).await {
        Ok(t) => t,
        Err(e) => {
            let (score, summary) = (20i64, format!("OAuth 刷新失败: {e}"));
            state.with_vault(|v| db::set_health(&v.conn, &email, score, &summary))?;
            return Ok(HealthResult { email, score, summary });
        }
    };

    // 2) 协议探测
    let (score, summary) = if normalize_auth_method(&creds.auth_method) == "graph" {
        match graph::probe(&token).await {
            Ok(_) => (100, "OAuth 与 Graph 均正常".to_string()),
            Err(e) => (60, format!("OAuth 正常，但 Graph 请求失败: {e}")),
        }
    } else {
        let email2 = creds.email.clone();
        let res = tokio::task::spawn_blocking(move || imap_client::probe_blocking(email2, token))
            .await
            .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))?;
        match res {
            Ok(_) => (100, "OAuth 与 IMAP 均正常".to_string()),
            Err(e) => (60, format!("OAuth 正常，但 IMAP 连接失败: {e}")),
        }
    };

    state.with_vault(|v| db::set_health(&v.conn, &email, score, &summary))?;
    Ok(HealthResult { email, score, summary })
}
