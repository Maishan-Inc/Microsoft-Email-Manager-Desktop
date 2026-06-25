//! SQLite 存储层（rusqlite bundled）。敏感字段加密后再入库。

use rusqlite::{params, Connection, OptionalExtension};

use crate::crypto::{self, DerivedKey};
use crate::error::{AppError, AppResult};
use crate::models::{AccountCredentials, AccountInfo, ClassificationOption};

const SCHEMA_VERSION: i64 = 2;

/// 默认后台刷新间隔（秒）
pub const DEFAULT_POLL_SECS: i64 = 300;

/// 打开数据库并执行迁移
pub fn open(path: &std::path::Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS app_meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS accounts (
            email             TEXT PRIMARY KEY,
            auth_method       TEXT NOT NULL DEFAULT 'imap',
            client_id_enc     TEXT NOT NULL,
            refresh_token_enc TEXT NOT NULL,
            status            TEXT NOT NULL DEFAULT 'active',
            category_key      TEXT,
            tag_keys          TEXT NOT NULL DEFAULT '[]',
            health_score      INTEGER NOT NULL DEFAULT 0,
            health_summary    TEXT NOT NULL DEFAULT '未检查',
            health_checked_at TEXT,
            notify_enabled    INTEGER NOT NULL DEFAULT 0,
            poll_interval_secs INTEGER,
            last_sync_at      TEXT,
            created_at        TEXT NOT NULL,
            updated_at        TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS categories (
            key        TEXT PRIMARY KEY,
            name_zh    TEXT NOT NULL,
            name_en    TEXT NOT NULL,
            remark     TEXT,
            created_at TEXT
        );

        CREATE TABLE IF NOT EXISTS tags (
            key        TEXT PRIMARY KEY,
            name_zh    TEXT NOT NULL,
            name_en    TEXT NOT NULL,
            remark     TEXT,
            created_at TEXT
        );

        CREATE TABLE IF NOT EXISTS email_tags (
            email      TEXT NOT NULL,
            message_id TEXT NOT NULL,
            tag_keys   TEXT NOT NULL DEFAULT '[]',
            PRIMARY KEY (email, message_id)
        );

        CREATE TABLE IF NOT EXISTS mail_activity (
            email      TEXT NOT NULL,
            message_id TEXT NOT NULL,
            subject    TEXT NOT NULL DEFAULT '',
            from_email TEXT NOT NULL DEFAULT '',
            received_at TEXT NOT NULL DEFAULT '',
            seen_at    TEXT NOT NULL,
            is_new     INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (email, message_id)
        );
        CREATE INDEX IF NOT EXISTS idx_mail_activity_seen ON mail_activity(seen_at);
        "#,
    )?;

    // 旧库（v1）升级：幂等补列。
    ensure_column(conn, "accounts", "notify_enabled", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "accounts", "poll_interval_secs", "INTEGER")?;
    ensure_column(conn, "accounts", "last_sync_at", "TEXT")?;

    set_meta(conn, "schema_version", &SCHEMA_VERSION.to_string())?;
    Ok(())
}

/// 若列不存在则 ALTER 添加（SQLite 无 IF NOT EXISTS 列语法）。
fn ensure_column(conn: &Connection, table: &str, col: &str, decl: &str) -> AppResult<()> {
    let exists: bool = {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let names = stmt.query_map([], |r| r.get::<_, String>(1))?;
        let mut found = false;
        for n in names {
            if n? == col {
                found = true;
                break;
            }
        }
        found
    };
    if !exists {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {col} {decl}"), [])?;
    }
    Ok(())
}

// ---------- app_meta ----------

pub fn get_meta(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let v = conn
        .query_row(
            "SELECT value FROM app_meta WHERE key = ?1",
            params![key],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(v)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_meta(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// 是否已设置主密码（新模型：存在密码包装的 DEK）
pub fn is_initialized(conn: &Connection) -> AppResult<bool> {
    Ok(get_meta(conn, "dek_wrapped_pw")?.is_some())
}

// ---------- accounts ----------

fn now_iso() -> String {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

/// 新增或更新账号（凭据加密入库）
pub fn upsert_account(
    conn: &Connection,
    key: &DerivedKey,
    creds: &AccountCredentials,
    category_key: Option<&str>,
    tag_keys: &[String],
) -> AppResult<()> {
    let client_id_enc = key.encrypt_str(&creds.client_id)?;
    let refresh_token_enc = key.encrypt_str(&creds.refresh_token)?;
    let tags_json = serde_json::to_string(tag_keys)?;
    let now = now_iso();

    conn.execute(
        r#"
        INSERT INTO accounts
            (email, auth_method, client_id_enc, refresh_token_enc, status,
             category_key, tag_keys, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, ?7, ?7)
        ON CONFLICT(email) DO UPDATE SET
            auth_method       = excluded.auth_method,
            client_id_enc     = excluded.client_id_enc,
            refresh_token_enc = excluded.refresh_token_enc,
            category_key      = excluded.category_key,
            tag_keys          = excluded.tag_keys,
            updated_at        = excluded.updated_at
        "#,
        params![
            creds.email,
            creds.auth_method,
            client_id_enc,
            refresh_token_enc,
            category_key,
            tags_json,
            now,
        ],
    )?;
    Ok(())
}

/// 读取账号凭据（解密）
pub fn get_credentials(
    conn: &Connection,
    key: &DerivedKey,
    email: &str,
) -> AppResult<AccountCredentials> {
    let row = conn
        .query_row(
            "SELECT auth_method, client_id_enc, refresh_token_enc FROM accounts WHERE email = ?1",
            params![email],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(email.to_string()))?;

    Ok(AccountCredentials {
        email: email.to_string(),
        auth_method: row.0,
        client_id: key.decrypt_str(&row.1)?,
        refresh_token: key.decrypt_str(&row.2)?,
    })
}

/// 列出账号信息（不含敏感字段）
pub fn list_accounts(conn: &Connection, key: &DerivedKey) -> AppResult<Vec<AccountInfo>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT email, auth_method, client_id_enc, status, category_key, tag_keys,
               health_score, health_summary, health_checked_at, created_at, updated_at,
               notify_enabled, poll_interval_secs, last_sync_at
        FROM accounts ORDER BY created_at DESC
        "#,
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,         // email
            r.get::<_, String>(1)?,         // auth_method
            r.get::<_, String>(2)?,         // client_id_enc
            r.get::<_, String>(3)?,         // status
            r.get::<_, Option<String>>(4)?, // category_key
            r.get::<_, String>(5)?,         // tag_keys json
            r.get::<_, i64>(6)?,            // health_score
            r.get::<_, String>(7)?,         // health_summary
            r.get::<_, Option<String>>(8)?, // health_checked_at
            r.get::<_, String>(9)?,         // created_at
            r.get::<_, String>(10)?,        // updated_at
            r.get::<_, i64>(11)?,           // notify_enabled
            r.get::<_, Option<i64>>(12)?,   // poll_interval_secs
            r.get::<_, Option<String>>(13)?, // last_sync_at
        ))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let r = row?;
        let tag_keys: Vec<String> = serde_json::from_str(&r.5).unwrap_or_default();
        // client_id 非高敏感，解密后展示便于核对；失败则留空
        let client_id = key.decrypt_str(&r.2).unwrap_or_default();
        out.push(AccountInfo {
            email: r.0,
            auth_method: r.1,
            client_id,
            status: r.3,
            category_key: r.4,
            tag_keys,
            health_score: r.6,
            health_summary: r.7,
            health_checked_at: r.8,
            created_at: r.9,
            updated_at: r.10,
            notify_enabled: r.11 != 0,
            poll_interval_secs: r.12,
            last_sync_at: r.13,
        });
    }
    Ok(out)
}

/// 设置某账号的通知开关与（可选）轮询间隔。
pub fn set_account_notify(
    conn: &Connection,
    email: &str,
    enabled: bool,
    interval_secs: Option<i64>,
) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE accounts SET notify_enabled = ?2, poll_interval_secs = ?3, updated_at = ?4 WHERE email = ?1",
        params![email, enabled as i64, interval_secs, now_iso()],
    )?;
    if n == 0 {
        return Err(AppError::NotFound(email.to_string()));
    }
    Ok(())
}

/// 后台轮询目标：开启通知的账号 + 其间隔 + 上次同步时间。
pub struct NotifyTarget {
    pub email: String,
    pub interval_secs: i64,
    pub last_sync_at: Option<String>,
}

pub fn list_notify_targets(conn: &Connection, default_secs: i64) -> AppResult<Vec<NotifyTarget>> {
    let mut stmt = conn.prepare(
        "SELECT email, poll_interval_secs, last_sync_at FROM accounts WHERE notify_enabled = 1",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(NotifyTarget {
            email: r.get::<_, String>(0)?,
            interval_secs: r.get::<_, Option<i64>>(1)?.unwrap_or(default_secs).max(30),
            last_sync_at: r.get::<_, Option<String>>(2)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// 所有账号邮箱（供手动「刷新统计」遍历）。
pub fn list_account_emails(conn: &Connection) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT email FROM accounts ORDER BY created_at DESC")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// 记录一条邮件活动；已存在则忽略。返回是否为新插入。
pub fn record_activity(
    conn: &Connection,
    email: &str,
    message_id: &str,
    subject: &str,
    from_email: &str,
    received_at: &str,
    is_new: bool,
) -> AppResult<bool> {
    let n = conn.execute(
        "INSERT OR IGNORE INTO mail_activity
            (email, message_id, subject, from_email, received_at, seen_at, is_new)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![email, message_id, subject, from_email, received_at, now_iso(), is_new as i64],
    )?;
    Ok(n > 0)
}

pub fn set_last_sync(conn: &Connection, email: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE accounts SET last_sync_at = ?2 WHERE email = ?1",
        params![email, now_iso()],
    )?;
    Ok(())
}

pub fn get_last_sync(conn: &Connection, email: &str) -> AppResult<Option<String>> {
    let v: Option<Option<String>> = conn
        .query_row(
            "SELECT last_sync_at FROM accounts WHERE email = ?1",
            params![email],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?;
    Ok(v.flatten())
}

/// 清理 7 天前的邮件活动，避免无限增长。
pub fn prune_activity(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "DELETE FROM mail_activity WHERE seen_at < datetime('now', '-7 days')",
        [],
    )?;
    Ok(())
}

/// 仪表盘统计：账号数 / 健康聚合 / 当日新邮件 / 最近活动。
pub fn dashboard_stats(conn: &Connection) -> AppResult<crate::models::DashboardStats> {
    let account_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))?;
    let healthy_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM accounts WHERE health_checked_at IS NOT NULL AND health_score >= 100",
        [],
        |r| r.get(0),
    )?;
    let unchecked_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM accounts WHERE health_checked_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    let health_avg: f64 = conn
        .query_row(
            "SELECT AVG(health_score) FROM accounts WHERE health_checked_at IS NOT NULL",
            [],
            |r| r.get::<_, Option<f64>>(0),
        )?
        .unwrap_or(0.0);
    let today_mail: i64 = conn.query_row(
        "SELECT COUNT(*) FROM mail_activity WHERE is_new = 1 AND substr(seen_at,1,10) = strftime('%Y-%m-%d','now')",
        [],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT email, message_id, subject, from_email, received_at
         FROM mail_activity ORDER BY seen_at DESC LIMIT 12",
    )?;
    let recent = stmt
        .query_map([], |r| {
            Ok(crate::models::MailActivityItem {
                email: r.get(0)?,
                message_id: r.get(1)?,
                subject: r.get(2)?,
                from_email: r.get(3)?,
                received_at: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(crate::models::DashboardStats {
        account_count,
        health_avg: health_avg.round() as i64,
        healthy_count,
        unchecked_count,
        today_mail,
        recent,
    })
}

pub fn delete_account(conn: &Connection, email: &str) -> AppResult<()> {
    let n = conn.execute("DELETE FROM accounts WHERE email = ?1", params![email])?;
    if n == 0 {
        return Err(AppError::NotFound(email.to_string()));
    }
    conn.execute("DELETE FROM email_tags WHERE email = ?1", params![email])?;
    Ok(())
}

pub fn update_classification(
    conn: &Connection,
    email: &str,
    category_key: Option<&str>,
    tag_keys: &[String],
) -> AppResult<()> {
    let tags_json = serde_json::to_string(tag_keys)?;
    let n = conn.execute(
        "UPDATE accounts SET category_key = ?2, tag_keys = ?3, updated_at = ?4 WHERE email = ?1",
        params![email, category_key, tags_json, now_iso()],
    )?;
    if n == 0 {
        return Err(AppError::NotFound(email.to_string()));
    }
    Ok(())
}

pub fn set_health(
    conn: &Connection,
    email: &str,
    score: i64,
    summary: &str,
) -> AppResult<()> {
    conn.execute(
        "UPDATE accounts SET health_score = ?2, health_summary = ?3, health_checked_at = ?4 WHERE email = ?1",
        params![email, score, summary, now_iso()],
    )?;
    Ok(())
}

// ---------- classifications ----------

pub fn list_catalog(conn: &Connection) -> AppResult<(Vec<ClassificationOption>, Vec<ClassificationOption>)> {
    Ok((read_options(conn, "categories")?, read_options(conn, "tags")?))
}

fn read_options(conn: &Connection, table: &str) -> AppResult<Vec<ClassificationOption>> {
    let sql = format!(
        "SELECT key, name_zh, name_en, remark, created_at FROM {table} ORDER BY created_at ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |r| {
        Ok(ClassificationOption {
            key: r.get(0)?,
            name_zh: r.get(1)?,
            name_en: r.get(2)?,
            remark: r.get(3)?,
            created_at: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn add_option(
    conn: &Connection,
    table: &str,
    opt: &ClassificationOption,
) -> AppResult<()> {
    if table != "categories" && table != "tags" {
        return Err(AppError::Other("非法分类表".into()));
    }
    let sql = format!(
        "INSERT INTO {table}(key, name_zh, name_en, remark, created_at) VALUES(?1,?2,?3,?4,?5)
         ON CONFLICT(key) DO UPDATE SET name_zh=excluded.name_zh, name_en=excluded.name_en, remark=excluded.remark"
    );
    conn.execute(
        &sql,
        params![opt.key, opt.name_zh, opt.name_en, opt.remark, opt.created_at],
    )?;
    Ok(())
}

pub fn delete_option(conn: &Connection, table: &str, key: &str) -> AppResult<()> {
    if table != "categories" && table != "tags" {
        return Err(AppError::Other("非法分类表".into()));
    }
    let sql = format!("DELETE FROM {table} WHERE key = ?1");
    conn.execute(&sql, params![key])?;
    Ok(())
}

/// 初始化主密码：写入 salt 与校验密文
pub fn init_master(conn: &Connection, key: &DerivedKey, salt: &[u8]) -> AppResult<()> {
    if is_initialized(conn)? {
        return Err(AppError::AlreadyInitialized);
    }
    let verifier = crypto::make_verifier(key)?;
    set_meta(conn, "master_salt", &crypto::b64_encode(salt))?;
    set_meta(conn, "verifier", &verifier)?;
    Ok(())
}
