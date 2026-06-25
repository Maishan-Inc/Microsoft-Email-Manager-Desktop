//! 安全辅助：TOTP（自实现 HMAC-SHA1，RFC 6238）、Base32、二维码 SVG、BIP39 助记词。

use hmac::{Hmac, Mac};
use rand::RngCore;
use sha1::Sha1;

use crate::error::{AppError, AppResult};

type HmacSha1 = Hmac<Sha1>;

// ---------- Base32 (RFC 4648, 无填充) ----------

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub fn base32_encode(data: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for &b in data {
        buffer = (buffer << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize] as char);
    }
    out
}

pub fn base32_decode(s: &str) -> Option<Vec<u8>> {
    let mut buffer = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::new();
    for c in s.trim().chars() {
        if c == '=' || c == ' ' {
            continue;
        }
        let up = c.to_ascii_uppercase() as u8;
        let val = ALPHABET.iter().position(|&x| x == up)? as u32;
        buffer = (buffer << 5) | val;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

// ---------- TOTP ----------

fn hotp(secret: &[u8], counter: u64) -> u32 {
    let mut mac = <HmacSha1 as Mac>::new_from_slice(secret).expect("hmac key length");
    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();
    let offset = (result[19] & 0x0f) as usize;
    let bin = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32) << 16)
        | ((result[offset + 2] as u32) << 8)
        | (result[offset + 3] as u32);
    bin % 1_000_000
}

fn now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 生成新的 TOTP 密钥（base32，160-bit）。
pub fn totp_secret_new() -> String {
    let mut bytes = [0u8; 20];
    rand::thread_rng().fill_bytes(&mut bytes);
    base32_encode(&bytes)
}

/// 构造 otpauth:// URI（供二维码与验证器）。
pub fn totp_uri(secret_b32: &str, issuer: &str, account: &str) -> String {
    let enc = |s: &str| s.replace(' ', "%20");
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits=6&period=30",
        enc(issuer),
        enc(account),
        secret_b32,
        enc(issuer)
    )
}

/// 校验 6 位 TOTP，容忍 ±1 个时间窗（30s）。
pub fn totp_verify(secret_b32: &str, code: &str) -> bool {
    let secret = match base32_decode(secret_b32) {
        Some(s) if !s.is_empty() => s,
        _ => return false,
    };
    let code: u32 = match code.trim().parse() {
        Ok(n) => n,
        Err(_) => return false,
    };
    let counter = now_secs() / 30;
    for c in [counter.wrapping_sub(1), counter, counter + 1] {
        if hotp(&secret, c) == code {
            return true;
        }
    }
    false
}

// ---------- 二维码 ----------

/// 把文本渲染为 SVG 二维码字符串。
pub fn qr_svg(text: &str) -> AppResult<String> {
    use qrcode::render::svg;
    use qrcode::QrCode;
    let code =
        QrCode::new(text.as_bytes()).map_err(|e| AppError::Other(format!("二维码生成失败: {e}")))?;
    let image = code
        .render::<svg::Color>()
        .min_dimensions(180, 180)
        .quiet_zone(true)
        .build();
    Ok(image)
}

// ---------- BIP39 助记词 ----------

/// 生成 12 词助记词（128-bit 熵）。
pub fn mnemonic_new() -> AppResult<Vec<String>> {
    let mut entropy = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut entropy);
    let m = bip39::Mnemonic::from_entropy(&entropy)
        .map_err(|e| AppError::Other(format!("助记词生成失败: {e}")))?;
    Ok(m.to_string().split_whitespace().map(|s| s.to_string()).collect())
}

/// 校验助记词是否合法（BIP39 校验和）。
pub fn mnemonic_valid(phrase: &str) -> bool {
    bip39::Mnemonic::parse(phrase.trim()).is_ok()
}
