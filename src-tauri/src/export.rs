//! 导出：账号导出为 JSON / CSV，可选是否包含敏感凭据。
//! 不含凭据的导出可安全分享；含凭据的导出请用户自行妥善保管。

use serde::Serialize;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::state::Vault;

#[derive(Serialize)]
pub struct ExportRow {
    pub email: String,
    pub auth_method: String,
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub status: String,
    pub category_key: Option<String>,
    pub tag_keys: Vec<String>,
    pub created_at: String,
}

/// 收集导出行（含凭据时解密 refresh_token）
fn collect_rows(vault: &Vault, include_credentials: bool) -> AppResult<Vec<ExportRow>> {
    let accounts = db::list_accounts(&vault.conn, &vault.key)?;
    let mut rows = Vec::with_capacity(accounts.len());
    for a in accounts {
        let refresh_token = if include_credentials {
            let creds = db::get_credentials(&vault.conn, &vault.key, &a.email)?;
            Some(creds.refresh_token)
        } else {
            None
        };
        rows.push(ExportRow {
            email: a.email,
            auth_method: a.auth_method,
            client_id: a.client_id,
            refresh_token,
            status: a.status,
            category_key: a.category_key,
            tag_keys: a.tag_keys,
            created_at: a.created_at,
        });
    }
    Ok(rows)
}

pub fn build_json(vault: &Vault, include_credentials: bool) -> AppResult<String> {
    let rows = collect_rows(vault, include_credentials)?;
    Ok(serde_json::to_string_pretty(&rows)?)
}

pub fn build_csv(vault: &Vault, include_credentials: bool) -> AppResult<String> {
    let rows = collect_rows(vault, include_credentials)?;
    let mut wtr = csv::Writer::from_writer(Vec::new());

    let mut header = vec![
        "email",
        "auth_method",
        "client_id",
        "status",
        "category_key",
        "tag_keys",
        "created_at",
    ];
    if include_credentials {
        header.insert(3, "refresh_token");
    }
    wtr.write_record(&header)
        .map_err(|e| AppError::Io(e.to_string()))?;

    for r in rows {
        let tags = r.tag_keys.join("|");
        let category = r.category_key.unwrap_or_default();
        let mut record = vec![
            r.email,
            r.auth_method,
            r.client_id,
            r.status,
            category,
            tags,
            r.created_at,
        ];
        if include_credentials {
            record.insert(3, r.refresh_token.unwrap_or_default());
        }
        wtr.write_record(&record)
            .map_err(|e| AppError::Io(e.to_string()))?;
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| AppError::Io(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| AppError::Parse(e.to_string()))
}

/// 生成导出内容并写入指定文件。
/// - format: "json" | "csv"
/// - include_credentials: 是否包含 refresh_token
/// - encrypt_with: 若提供，则用主密钥对内容整体 AES-GCM 加密后写出（含凭据时建议开启）
pub fn export_to_file(
    vault: &Vault,
    path: &str,
    format: &str,
    include_credentials: bool,
    encrypt: bool,
) -> AppResult<()> {
    let content = match format {
        "csv" => build_csv(vault, include_credentials)?,
        _ => build_json(vault, include_credentials)?,
    };

    if encrypt {
        // 整体加密：文件以 "MEMENC1\n" 开头，后接 base64(nonce||ct)。
        // 可用同一主密码在本应用内解密恢复。
        let enc = vault.key.encrypt(content.as_bytes())?;
        let payload = format!("MEMENC1\n{enc}");
        std::fs::write(path, payload)?;
    } else {
        std::fs::write(path, content)?;
    }
    Ok(())
}
