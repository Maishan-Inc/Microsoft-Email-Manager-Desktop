//! 加密模块：Argon2id 从主密码派生 32 字节密钥，AES-256-GCM 加密敏感字段。
//!
//! 设计：
//! - 主密码不落盘。仅落盘随机 salt 与一个「校验密文」。
//! - 派生密钥常驻内存（解锁期间），退出/锁定时 zeroize 清零。
//! - 每个敏感字段单独加密：输出 = base64(nonce(12) || ciphertext+tag)。

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use zeroize::Zeroize;

use crate::error::{AppError, AppResult};

/// 用于校验主密码是否正确的固定明文（解锁时解密比对）。
const VERIFIER_PLAINTEXT: &[u8] = b"MEM_VERIFY_OK_v1";

/// 派生密钥的安全封装：Drop 时清零。
pub struct DerivedKey([u8; 32]);

impl DerivedKey {
    fn cipher(&self) -> Aes256Gcm {
        let key = Key::<Aes256Gcm>::from_slice(&self.0);
        Aes256Gcm::new(key)
    }

    /// 加密明文 -> base64(nonce || ct)
    pub fn encrypt(&self, plaintext: &[u8]) -> AppResult<String> {
        let cipher = self.cipher();
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| AppError::Crypto(e.to_string()))?;
        let mut out = Vec::with_capacity(12 + ct.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ct);
        Ok(B64.encode(out))
    }

    /// 解密 base64(nonce || ct) -> 明文
    pub fn decrypt(&self, encoded: &str) -> AppResult<Vec<u8>> {
        let raw = B64
            .decode(encoded)
            .map_err(|e| AppError::Crypto(format!("base64: {e}")))?;
        if raw.len() < 12 + 16 {
            return Err(AppError::Crypto("密文长度异常".into()));
        }
        let (nonce_bytes, ct) = raw.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let cipher = self.cipher();
        cipher
            .decrypt(nonce, ct)
            .map_err(|_| AppError::Crypto("认证失败（密钥错误或数据损坏）".into()))
    }

    pub fn encrypt_str(&self, s: &str) -> AppResult<String> {
        self.encrypt(s.as_bytes())
    }

    pub fn decrypt_str(&self, encoded: &str) -> AppResult<String> {
        let bytes = self.decrypt(encoded)?;
        String::from_utf8(bytes).map_err(|e| AppError::Crypto(e.to_string()))
    }
}

impl Drop for DerivedKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Argon2id 参数：内存 64MiB、迭代 3、并行 1。对桌面端足够强且不卡顿。
fn argon2() -> Argon2<'static> {
    let params = Params::new(64 * 1024, 3, 1, Some(32)).expect("argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// 由主密码 + salt 派生 32 字节密钥
pub fn derive_key(password: &str, salt: &[u8]) -> AppResult<DerivedKey> {
    let mut key = [0u8; 32];
    argon2()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| AppError::Crypto(e.to_string()))?;
    Ok(DerivedKey(key))
}

/// 生成 16 字节随机 salt
pub fn new_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// 生成校验密文（首次设置主密码时调用）
pub fn make_verifier(key: &DerivedKey) -> AppResult<String> {
    key.encrypt(VERIFIER_PLAINTEXT)
}

/// 校验主密码：解密校验密文并比对
pub fn verify(key: &DerivedKey, verifier: &str) -> AppResult<()> {
    match key.decrypt(verifier) {
        Ok(pt) if pt == VERIFIER_PLAINTEXT => Ok(()),
        _ => Err(AppError::BadPassword),
    }
}

pub fn b64_encode(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

pub fn b64_decode(s: &str) -> AppResult<Vec<u8>> {
    B64.decode(s).map_err(|e| AppError::Crypto(e.to_string()))
}
