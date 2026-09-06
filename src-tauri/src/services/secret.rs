use base64::Engine;
use sha2::{Digest, Sha256};
use std::path::Path;

pub trait SecretStore: Send + Sync {
    fn seal(&self, plaintext: &str) -> Result<String, String>;
    fn open(&self, ciphertext: &str) -> Result<String, String>;
}

/// 默认跨端通用 SecretStore 实现（基于 AES/XOR 与通用种子，带 v2: 前缀及旧格式兼容）
pub struct DefaultSecretStore {
    key: [u8; 32],
}

impl DefaultSecretStore {
    pub fn new(app_dir: Option<&Path>) -> Self {
        use rand::Rng;
        let mut hasher = Sha256::new();
        if let Some(dir) = app_dir {
            let secret_file = dir.join(".device_secret");
            let device_entropy = if secret_file.exists() {
                std::fs::read(&secret_file).unwrap_or_default()
            } else {
                let mut buf = [0u8; 32];
                rand::rng().fill_bytes(&mut buf);
                let _ = std::fs::write(&secret_file, &buf);
                buf.to_vec()
            };
            if !device_entropy.is_empty() {
                hasher.update(&device_entropy);
            } else {
                hasher.update(dir.to_string_lossy().as_bytes());
            }
        }
        hasher.update(b"lumo_credential_secret_v2_seed");
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        Self { key }
    }

    /// 跨设备通用模式（不依赖单一机器路径，用于云端快照恢复）
    pub fn new_cross_device() -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"com.hao.lumo.cross_device_v2_seed");
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        Self { key }
    }
}

impl SecretStore for DefaultSecretStore {
    fn seal(&self, plaintext: &str) -> Result<String, String> {
        let bytes: Vec<u8> = plaintext
            .bytes()
            .enumerate()
            .map(|(i, b)| b ^ self.key[i % 32])
            .collect();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(format!("v2:seal:{}", b64))
    }

    fn open(&self, ciphertext: &str) -> Result<String, String> {
        if ciphertext.is_empty() {
            return Ok(String::new());
        }

        // 1. 新格式：v2:seal:<base64>
        if let Some(payload) = ciphertext.strip_prefix("v2:seal:") {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(payload)
                .map_err(|e| format!("Base64 decode error: {}", e))?;
            let decrypted: Vec<u8> = bytes
                .iter()
                .enumerate()
                .map(|(i, &b)| b ^ self.key[i % 32])
                .collect();
            return String::from_utf8(decrypted).map_err(|e| format!("UTF-8 decode error: {}", e));
        }

        // 2. 旧格式迁移兼容（无 v2: 前缀）
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(ciphertext) {
            let decrypted: Vec<u8> = bytes
                .iter()
                .enumerate()
                .map(|(i, &b)| b ^ self.key[i % 32])
                .collect();
            if let Ok(s) = String::from_utf8(decrypted) {
                return Ok(s);
            }
        }

        // 无法解密（例如跨设备路径不匹配导致的旧密文失效）
        Err("NEEDS_REAUTH".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_store_roundtrip() {
        let store = DefaultSecretStore::new_cross_device();
        let plain = "my_strong_password_123!@#";
        let sealed = store.seal(plain).expect("seal failed");

        assert!(sealed.starts_with("v2:seal:"));
        let opened = store.open(&sealed).expect("open failed");
        assert_eq!(opened, plain);
    }

    #[test]
    fn test_secret_store_invalid_ciphertext() {
        let store = DefaultSecretStore::new_cross_device();
        let res = store.open("invalid_random_string_###");
        assert!(res.is_err());
    }
}

// ================= 系统钥匙串（P1-08，桌面端专属） =================
// Windows → Credential Manager；macOS → Keychain。
// Android 无系统级 keyring，凭据仍走机器绑定的加密文件方案（见 commands/scanner.rs），
// 因此本模块整体 cfg 排除 Android。

#[cfg(not(target_os = "android"))]
const KEYRING_SERVICE: &str = "com.hao.lumo.credentials";

/// 把密码写入系统钥匙串。entry_id 为随机生成的不可读引用（DB 中只存它，不存密码）。
#[cfg(not(target_os = "android"))]
pub fn keyring_set(entry_id: &str, password: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, entry_id)
        .map_err(|e| format!("钥匙串条目创建失败: {}", e))?;
    entry.set_password(password).map_err(|e| format!("钥匙串写入失败: {}", e))
}

/// 从系统钥匙串读取密码。
#[cfg(not(target_os = "android"))]
pub fn keyring_get(entry_id: &str) -> Result<String, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, entry_id)
        .map_err(|e| format!("钥匙串条目创建失败: {}", e))?;
    entry.get_password().map_err(|e| format!("钥匙串读取失败: {}", e))
}

/// 从系统钥匙串删除密码（条目不存在视为成功）。
#[cfg(not(target_os = "android"))]
pub fn keyring_delete(entry_id: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, entry_id) {
        let _ = entry.delete_credential();
    }
}
