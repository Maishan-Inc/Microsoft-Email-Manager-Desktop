//! Tauri 命令：前端通过 invoke 调用。
//! 约定：加密/解密等 CPU 操作用同步命令；网络操作用 async 且不跨 await 持锁。

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::accounts;
use crate::accounts_auth;
use crate::background;
use crate::crypto;
use crate::db;
use crate::error::{AppError, AppResult};
use crate::export;
use crate::graph;
use crate::imap_client;
use crate::security;
use crate::models::{
    normalize_auth_method, AccountCredentials, AccountInfo, ClassificationOption, DashboardStats,
    EmailDetails, EmailListResponse,
};
use crate::state::{AppState, LockedMail, Vault};

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

/// 内部：创建 DEK 并用密码包装，写入 salt_pw / dek_wrapped_pw，返回 DEK。
fn init_dek_with_password(conn: &rusqlite::Connection, password: &str) -> AppResult<crypto::DerivedKey> {
    let dek = crypto::random_dek();
    let salt = crypto::new_salt();
    let kek = crypto::derive_key(password, &salt)?;
    db::set_meta(conn, "salt_pw", &crypto::b64_encode(&salt))?;
    db::set_meta(conn, "dek_wrapped_pw", &crypto::wrap_dek(&kek, &dek)?)?;
    Ok(dek)
}

fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

#[tauri::command]
pub async fn setup_master_password(state: State<'_, AppState>, password: String) -> AppResult<()> {
    if password.len() < 8 {
        return Err(AppError::Other("主密码至少 8 位".into()));
    }
    // Argon2 派生很重，放到阻塞线程，避免主线程卡死（窗口「未响应」）。
    let db_path = state.db_path.clone();
    let vault = tokio::task::spawn_blocking(move || -> AppResult<Vault> {
        let conn = db::open(&db_path)?;
        if db::is_initialized(&conn)? {
            return Err(AppError::AlreadyInitialized);
        }
        let dek = init_dek_with_password(&conn, &password)?;
        db::set_meta(&conn, "auth_mode", "password_only")?;
        Ok(Vault { conn, key: dek })
    })
    .await
    .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))??;
    state.set_vault(vault);
    Ok(())
}

#[derive(Serialize)]
pub struct UnlockResult {
    pub needs_2fa: bool,
}

#[tauri::command]
pub async fn unlock(state: State<'_, AppState>, password: String) -> AppResult<UnlockResult> {
    let db_path = state.db_path.clone();
    // 开锁的 Argon2 派生放到阻塞线程，避免 UI 卡死。
    let (vault, needs_2fa) = tokio::task::spawn_blocking(move || -> AppResult<(Vault, bool)> {
        let conn = db::open(&db_path)?;
        let salt_b64 = db::get_meta(&conn, "salt_pw")?.ok_or(AppError::BadPassword)?;
        let salt = crypto::b64_decode(&salt_b64)?;
        let kek = crypto::derive_key(&password, &salt)?;
        let wrapped = db::get_meta(&conn, "dek_wrapped_pw")?.ok_or(AppError::BadPassword)?;
        // GCM 认证失败即密码错误
        let dek = crypto::unwrap_dek(&kek, &wrapped).map_err(|_| AppError::BadPassword)?;

        let auth_mode = db::get_meta(&conn, "auth_mode")?.unwrap_or_else(|| "password_only".into());
        let has_totp = db::get_meta(&conn, "totp_secret_enc")?.is_some();
        let needs_2fa = auth_mode != "password_only" && has_totp;
        Ok((Vault { conn, key: dek }, needs_2fa))
    })
    .await
    .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))??;

    if needs_2fa {
        state.set_pending(vault);
        Ok(UnlockResult { needs_2fa: true })
    } else {
        state.set_vault(vault);
        Ok(UnlockResult { needs_2fa: false })
    }
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
    category_key: Option<String>,
    tag_keys: Vec<String>,
) -> AppResult<ImportResult> {
    let (creds_list, mut errors) = accounts::parse_import_bulk(&text, &auth_method);
    let mut added = 0usize;
    state.with_vault(|v| {
        for c in &creds_list {
            match db::upsert_account(&v.conn, &v.key, c, category_key.as_deref(), &tag_keys) {
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

// ---------- 通知 / 后台刷新 / 仪表盘 / 设置 ----------

/// 设置某账号是否开启新邮件系统通知（及可选轮询间隔秒）。
#[tauri::command]
pub fn set_account_notify(
    state: State<'_, AppState>,
    email: String,
    enabled: bool,
    interval_secs: Option<i64>,
) -> AppResult<()> {
    state.with_vault(|v| db::set_account_notify(&v.conn, &email, enabled, interval_secs))
}

/// 仪表盘统计数据。
#[tauri::command]
pub fn dashboard_stats(state: State<'_, AppState>) -> AppResult<DashboardStats> {
    state.with_vault(|v| db::dashboard_stats(&v.conn))
}

/// 手动同步全部账号收件箱（刷新统计，不弹通知）。返回新邮件总数。
#[tauri::command]
pub async fn sync_mail_now(app: tauri::AppHandle) -> AppResult<usize> {
    background::sync_all(&app).await
}

#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    pub bg_refresh_enabled: bool,
    pub bg_refresh_interval_secs: i64,
    pub auto_lock_mins: i64,
    pub block_remote_images: bool,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<AppSettings> {
    state.with_vault(|v| {
        let bg_refresh_enabled = db::get_meta(&v.conn, "bg_refresh_enabled")?
            .map(|s| s != "0")
            .unwrap_or(true);
        let bg_refresh_interval_secs = db::get_meta(&v.conn, "bg_refresh_interval_secs")?
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(db::DEFAULT_POLL_SECS);
        let auto_lock_mins = db::get_meta(&v.conn, "auto_lock_mins")?
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(30);
        let block_remote_images = db::get_meta(&v.conn, "block_remote_images")?
            .map(|s| s != "0")
            .unwrap_or(true);
        Ok(AppSettings {
            bg_refresh_enabled,
            bg_refresh_interval_secs,
            auto_lock_mins,
            block_remote_images,
        })
    })
}

#[tauri::command]
pub fn set_settings(state: State<'_, AppState>, settings: AppSettings) -> AppResult<()> {
    state.with_vault(|v| {
        db::set_meta(
            &v.conn,
            "bg_refresh_enabled",
            if settings.bg_refresh_enabled { "1" } else { "0" },
        )?;
        db::set_meta(
            &v.conn,
            "bg_refresh_interval_secs",
            &settings.bg_refresh_interval_secs.max(5).to_string(),
        )?;
        db::set_meta(&v.conn, "auto_lock_mins", &settings.auto_lock_mins.max(0).to_string())?;
        db::set_meta(&v.conn, "block_remote_images", if settings.block_remote_images { "1" } else { "0" })?;
        Ok(())
    })
}

// ---------- 首启引导 / 安全配置 ----------

#[derive(Serialize)]
pub struct OnboardingStatus {
    pub agreement_accepted: bool,
    pub initialized: bool,
    pub onboarding_completed: bool,
    pub tutorial_seen: bool,
    pub auth_mode: String,
}

#[tauri::command]
pub fn onboarding_status(state: State<'_, AppState>) -> AppResult<OnboardingStatus> {
    let conn = db::open(&state.db_path)?;
    Ok(OnboardingStatus {
        agreement_accepted: db::get_meta(&conn, "agreement_accepted_at")?.is_some(),
        initialized: db::is_initialized(&conn)?,
        onboarding_completed: db::get_meta(&conn, "onboarding_completed")?.as_deref() == Some("1"),
        tutorial_seen: db::get_meta(&conn, "tutorial_seen")?.as_deref() == Some("1"),
        auth_mode: db::get_meta(&conn, "auth_mode")?.unwrap_or_default(),
    })
}

#[tauri::command]
pub fn accept_agreement(state: State<'_, AppState>) -> AppResult<()> {
    let conn = db::open(&state.db_path)?;
    db::set_meta(&conn, "agreement_accepted_at", &now_rfc3339())?;
    Ok(())
}

#[tauri::command]
pub fn complete_onboarding(state: State<'_, AppState>) -> AppResult<()> {
    let conn = db::open(&state.db_path)?;
    db::set_meta(&conn, "onboarding_completed", "1")?;
    Ok(())
}

#[tauri::command]
pub fn set_tutorial_seen(state: State<'_, AppState>) -> AppResult<()> {
    let conn = db::open(&state.db_path)?;
    db::set_meta(&conn, "tutorial_seen", "1")?;
    Ok(())
}

#[derive(Serialize)]
pub struct TotpSetup {
    pub secret: String,
    pub otpauth_uri: String,
    pub qr_svg: String,
}

/// 生成候选 TOTP 密钥与二维码（未落库，前端持有到 complete_setup）。
#[tauri::command]
pub fn generate_totp() -> AppResult<TotpSetup> {
    let secret = security::totp_secret_new();
    let otpauth_uri = security::totp_uri(&secret, "Microsoft Email Manager", "vault");
    let qr_svg = security::qr_svg(&otpauth_uri)?;
    Ok(TotpSetup {
        secret,
        otpauth_uri,
        qr_svg,
    })
}

/// 校验候选 TOTP 密钥的验证码（设置过程中用）。
#[tauri::command]
pub fn verify_totp_code(secret: String, code: String) -> AppResult<bool> {
    Ok(security::totp_verify(&secret, &code))
}

#[derive(Serialize)]
pub struct MnemonicGen {
    pub words: Vec<String>,
}

#[tauri::command]
pub fn generate_mnemonic() -> AppResult<MnemonicGen> {
    Ok(MnemonicGen {
        words: security::mnemonic_new()?,
    })
}

#[derive(Deserialize)]
pub struct SecuritySetup {
    pub password: String,
    pub totp_secret: Option<String>,
    pub auth_mode: String,
    pub mnemonic: Option<String>,
}

/// 一次性原子完成安全配置：建 DEK，密码（+助记词）包装，存 TOTP 与 auth_mode，解锁。
#[tauri::command]
pub async fn complete_setup(state: State<'_, AppState>, setup: SecuritySetup) -> AppResult<()> {
    if setup.password.len() < 8 {
        return Err(AppError::Other("主密码至少 8 位".into()));
    }
    let db_path = state.db_path.clone();
    // 多次 Argon2（密码 + 助记词）放到阻塞线程，避免向导卡死。
    let vault = tokio::task::spawn_blocking(move || -> AppResult<Vault> {
        let conn = db::open(&db_path)?;
        if db::is_initialized(&conn)? {
            return Err(AppError::AlreadyInitialized);
        }
        let dek = init_dek_with_password(&conn, &setup.password)?;

        // 助记词恢复包装
        if let Some(phrase) = setup.mnemonic.as_ref().filter(|p| !p.trim().is_empty()) {
            let salt = crypto::new_salt();
            let kek = crypto::derive_key(phrase.trim(), &salt)?;
            db::set_meta(&conn, "salt_mn", &crypto::b64_encode(&salt))?;
            db::set_meta(&conn, "dek_wrapped_mn", &crypto::wrap_dek(&kek, &dek)?)?;
        }

        // TOTP 密钥（DEK 加密存储）
        let has_totp = setup
            .totp_secret
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if let Some(secret) = setup.totp_secret.as_ref().filter(|s| !s.trim().is_empty()) {
            db::set_meta(&conn, "totp_secret_enc", &dek.encrypt_str(secret)?)?;
        }

        let auth_mode = if has_totp {
            if setup.auth_mode.is_empty() {
                "password_2fa".to_string()
            } else {
                setup.auth_mode.clone()
            }
        } else {
            "password_only".to_string()
        };
        db::set_meta(&conn, "auth_mode", &auth_mode)?;
        Ok(Vault { conn, key: dek })
    })
    .await
    .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))??;

    state.set_vault(vault);
    Ok(())
}

/// 解锁第二步：校验 2FA，通过后将 pending 提升为已解锁。
#[tauri::command]
pub fn verify_2fa(state: State<'_, AppState>, code: String) -> AppResult<()> {
    let vault = state.take_pending().ok_or(AppError::Locked)?;
    let res = (|| -> AppResult<bool> {
        let enc = db::get_meta(&vault.conn, "totp_secret_enc")?
            .ok_or_else(|| AppError::Other("未配置 2FA".into()))?;
        let secret = vault.key.decrypt_str(&enc)?;
        Ok(security::totp_verify(&secret, &code))
    })();
    match res {
        Ok(true) => {
            state.set_vault(vault);
            Ok(())
        }
        Ok(false) => {
            state.set_pending(vault);
            Err(AppError::Other("验证码不正确".into()))
        }
        Err(e) => {
            state.set_pending(vault);
            Err(e)
        }
    }
}

/// 用恢复助记词解出 DEK 并解锁（之后建议立即 reset_password）。
#[tauri::command]
pub async fn recover_with_mnemonic(state: State<'_, AppState>, words: String) -> AppResult<()> {
    if !security::mnemonic_valid(&words) {
        return Err(AppError::Other("助记词格式不正确".into()));
    }
    let db_path = state.db_path.clone();
    let vault = tokio::task::spawn_blocking(move || -> AppResult<Vault> {
        let conn = db::open(&db_path)?;
        let salt_b64 = db::get_meta(&conn, "salt_mn")?
            .ok_or_else(|| AppError::Other("未配置恢复助记词".into()))?;
        let salt = crypto::b64_decode(&salt_b64)?;
        let kek = crypto::derive_key(words.trim(), &salt)?;
        let wrapped = db::get_meta(&conn, "dek_wrapped_mn")?
            .ok_or_else(|| AppError::Other("未配置恢复助记词".into()))?;
        let dek = crypto::unwrap_dek(&kek, &wrapped).map_err(|_| AppError::Other("助记词不正确".into()))?;
        Ok(Vault { conn, key: dek })
    })
    .await
    .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))??;
    state.set_vault(vault);
    Ok(())
}

/// 重设主密码（用当前 DEK 重新做密码包装）。
#[tauri::command]
pub async fn reset_password(state: State<'_, AppState>, new_password: String) -> AppResult<()> {
    if new_password.len() < 8 {
        return Err(AppError::Other("主密码至少 8 位".into()));
    }
    // 先离开主线程做 Argon2 派生，再回到锁内做快速的包装写库。
    let salt = crypto::new_salt();
    let salt_vec = salt.to_vec();
    let np = new_password.clone();
    let kek = tokio::task::spawn_blocking(move || crypto::derive_key(&np, &salt_vec))
        .await
        .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))??;
    state.with_vault(|v| {
        db::set_meta(&v.conn, "salt_pw", &crypto::b64_encode(&salt))?;
        db::set_meta(&v.conn, "dek_wrapped_pw", &crypto::wrap_dek(&kek, &v.key)?)?;
        Ok(())
    })
}

#[derive(Serialize)]
pub struct CredsReveal {
    pub email: String,
    pub client_id: String,
    pub refresh_token: String,
    pub combined: String,
}

/// 二次校验所需材料：2FA 走动态码（已解出的 TOTP 密钥），否则走主密码（salt + 包装）。
enum VerifyMaterial {
    Totp(String),
    Pw { salt: Vec<u8>, wrapped: String },
}

/// 在锁内收集校验材料（不做重计算）。
fn gather_verify_material(v: &Vault) -> AppResult<VerifyMaterial> {
    let auth_mode = db::get_meta(&v.conn, "auth_mode")?.unwrap_or_else(|| "password_only".into());
    let totp_enc = db::get_meta(&v.conn, "totp_secret_enc")?;
    if auth_mode != "password_only" && totp_enc.is_some() {
        let totp = v.key.decrypt_str(&totp_enc.unwrap())?;
        Ok(VerifyMaterial::Totp(totp))
    } else {
        let salt_b64 = db::get_meta(&v.conn, "salt_pw")?.ok_or(AppError::BadPassword)?;
        let salt = crypto::b64_decode(&salt_b64)?;
        let wrapped = db::get_meta(&v.conn, "dek_wrapped_pw")?.ok_or(AppError::BadPassword)?;
        Ok(VerifyMaterial::Pw { salt, wrapped })
    }
}

/// 校验密钥/动态码；密码路径的 Argon2 放阻塞线程。
async fn check_secret(material: VerifyMaterial, secret: &str) -> AppResult<bool> {
    Ok(match material {
        VerifyMaterial::Totp(totp) => security::totp_verify(&totp, secret.trim()),
        VerifyMaterial::Pw { salt, wrapped } => {
            let s = secret.to_string();
            let kek = tokio::task::spawn_blocking(move || crypto::derive_key(&s, &salt))
                .await
                .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))??;
            crypto::unwrap_dek(&kek, &wrapped).is_ok()
        }
    })
}

/// 揭示某账号的完整凭据。需二次校验：开启 2FA 时校验动态码，否则校验主密码。
#[tauri::command]
pub async fn reveal_credentials(
    state: State<'_, AppState>,
    email: String,
    secret: String,
) -> AppResult<CredsReveal> {
    let material = state.with_vault(gather_verify_material)?;
    if !check_secret(material, &secret).await? {
        return Err(AppError::Other("验证失败".into()));
    }
    state.with_vault(|v| {
        let creds = db::get_credentials(&v.conn, &v.key, &email)?;
        let combined = format!("{}----{}", creds.refresh_token, creds.client_id);
        Ok(CredsReveal {
            email: creds.email,
            client_id: creds.client_id,
            refresh_token: creds.refresh_token,
            combined,
        })
    })
}

/// 二次身份校验（用于敏感设置变更前确认身份）：有 2FA 校验动态码，否则校验主密码。
#[tauri::command]
pub async fn verify_auth(state: State<'_, AppState>, secret: String) -> AppResult<()> {
    let material = state.with_vault(gather_verify_material)?;
    if check_secret(material, &secret).await? {
        Ok(())
    } else {
        Err(AppError::Other("验证失败".into()))
    }
}

/// 重新生成恢复助记词：先二次校验（密码/2FA），再用当前 DEK 重新包装并落库。返回新助记词。
#[tauri::command]
pub async fn regenerate_mnemonic(
    state: State<'_, AppState>,
    secret: String,
) -> AppResult<MnemonicGen> {
    let material = state.with_vault(gather_verify_material)?;
    if !check_secret(material, &secret).await? {
        return Err(AppError::Other("验证失败".into()));
    }
    let words = security::mnemonic_new()?;
    let phrase = words.join(" ");
    // Argon2 离开主线程
    let salt = crypto::new_salt();
    let salt_vec = salt.to_vec();
    let kek = tokio::task::spawn_blocking(move || crypto::derive_key(phrase.trim(), &salt_vec))
        .await
        .map_err(|e| AppError::Other(format!("任务执行失败: {e}")))??;
    state.with_vault(|v| {
        db::set_meta(&v.conn, "salt_mn", &crypto::b64_encode(&salt))?;
        db::set_meta(&v.conn, "dek_wrapped_mn", &crypto::wrap_dek(&kek, &v.key)?)?;
        Ok(())
    })?;
    Ok(MnemonicGen { words })
}

/// 在已解锁状态下开关 2FA：Some(secret) = 开启（存 TOTP 密钥 + password_2fa）；None = 关闭。
#[tauri::command]
pub fn set_two_factor(state: State<'_, AppState>, totp_secret: Option<String>) -> AppResult<()> {
    state.with_vault(|v| {
        match totp_secret.as_ref().filter(|s| !s.trim().is_empty()) {
            Some(secret) => {
                db::set_meta(&v.conn, "totp_secret_enc", &v.key.encrypt_str(secret.trim())?)?;
                db::set_meta(&v.conn, "auth_mode", "password_2fa")?;
            }
            None => {
                db::del_meta(&v.conn, "totp_secret_enc")?;
                db::set_meta(&v.conn, "auth_mode", "password_only")?;
            }
        }
        Ok(())
    })
}

/// 当前认证模式（供前端决定揭示凭据时提示密码还是动态码）。
#[tauri::command]
pub fn auth_mode_info(state: State<'_, AppState>) -> AppResult<bool> {
    state.with_vault(|v| {
        let auth_mode = db::get_meta(&v.conn, "auth_mode")?.unwrap_or_else(|| "password_only".into());
        let has_totp = db::get_meta(&v.conn, "totp_secret_enc")?.is_some();
        Ok(auth_mode != "password_only" && has_totp)
    })
}

/// 用系统默认浏览器打开外部链接（仅允许 http/https，过滤注入字符）。
#[tauri::command]
pub fn open_url(url: String) -> AppResult<()> {
    let safe = (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() < 2048
        && !url.chars().any(|c| {
            matches!(c, '"' | '\'' | ' ' | '\n' | '\r' | '\t' | '&' | '|' | ';' | '<' | '>' | '^' | '`' | '$' | '(' | ')')
        });
    if !safe {
        return Err(AppError::Other("非法链接".into()));
    }

    #[cfg(target_os = "windows")]
    let spawned = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", &url])
        .spawn();
    #[cfg(target_os = "macos")]
    let spawned = std::process::Command::new("open").arg(&url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let spawned = std::process::Command::new("xdg-open").arg(&url).spawn();

    spawned.map(|_| ()).map_err(|e| AppError::Io(e.to_string()))
}

/// 记录「当前在邮件页打开的邮箱」，纳入锁后监视（令牌即焚）。传 null 清除。
#[tauri::command]
pub fn set_active_mailbox(state: State<'_, AppState>, email: Option<String>) -> AppResult<()> {
    state.set_active_mailbox(email);
    Ok(())
}

/// 锁定期间累计的新邮件（最小信息：邮箱+发件人+message_id+是否含验证码）。
/// 锁屏列表使用，**无需解锁**即可读取。
#[tauri::command]
pub fn locked_items(state: State<'_, AppState>) -> AppResult<Vec<LockedMail>> {
    Ok(state.locked_items())
}
