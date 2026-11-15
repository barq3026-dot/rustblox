// Local accounts: usernames + salted password hashes, stored in %APPDATA%\Rustblox.
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const HASH_ROUNDS: u32 = 25_000;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Stats {
    pub mined: u64,
    pub placed: u64,
    pub play_seconds: u64,
}

impl Stats {
    pub fn playtime(&self) -> String {
        let total = self.play_seconds;
        let h = total / 3600;
        let m = (total % 3600) / 60;
        if h > 0 {
            format!("{h} ч {m} мин")
        } else {
            format!("{m} мин")
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserRecord {
    pub display_name: String,
    pub salt: String,
    pub hash: String,
    pub created_at: u64,
    pub last_login: u64,
    pub stats: Stats,
}

#[derive(Serialize, Deserialize, Default)]
pub struct UserStore {
    pub users: BTreeMap<String, UserRecord>,
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn data_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("Rustblox")
    } else {
        PathBuf::from("rustblox_data")
    }
}

fn hex(bytes: &[u8]) -> String {
    const CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(CHARS[(b >> 4) as usize] as char);
        out.push(CHARS[(b & 0x0f) as usize] as char);
    }
    out
}

fn hash_password(salt: &str, password: &str) -> String {
    let salt_bytes = salt.as_bytes();
    let mut hasher = Sha256::new();
    hasher.update(salt_bytes);
    hasher.update(password.as_bytes());
    let mut digest = hasher.finalize();

    for _ in 0..HASH_ROUNDS {
        let mut h = Sha256::new();
        h.update(digest);
        h.update(salt_bytes);
        digest = h.finalize();
    }
    hex(&digest)
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn validate_username(name: &str) -> Result<(), String> {
    if name.len() < 3 || name.len() > 16 {
        return Err("Никнейм: от 3 до 16 символов".into());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Никнейм: только латиница, цифры, _ и -".into());
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 4 {
        return Err("Пароль: минимум 4 символа".into());
    }
    if password.len() > 64 {
        return Err("Пароль: максимум 64 символа".into());
    }
    Ok(())
}

impl UserStore {
    pub fn load(path: &Path) -> Self {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|err| {
                eprintln!("[auth] не удалось прочитать {}: {err}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, bytes)?;
        if path.exists() {
            let _ = fs::remove_file(path);
        }
        fs::rename(tmp, path)
    }

    pub fn register(&mut self, name: &str, password: &str) -> Result<String, String> {
        let name = name.trim();
        validate_username(name)?;
        validate_password(password)?;
        let key = name.to_lowercase();
        if self.users.contains_key(&key) {
            return Err("Этот никнейм уже занят".into());
        }

        let mut salt_bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt_bytes);
        let salt = hex(&salt_bytes);
        let hash = hash_password(&salt, password);
        let now = now_unix();

        self.users.insert(
            key.clone(),
            UserRecord {
                display_name: name.to_string(),
                salt,
                hash,
                created_at: now,
                last_login: now,
                stats: Stats::default(),
            },
        );
        Ok(key)
    }

    pub fn login(&mut self, name: &str, password: &str) -> Result<String, String> {
        let key = name.trim().to_lowercase();
        if key.is_empty() {
            return Err("Введите никнейм".into());
        }
        let Some(record) = self.users.get(&key) else {
            return Err("Аккаунт не найден. Зарегистрируйтесь.".into());
        };
        let hash = hash_password(&record.salt, password);
        if !constant_time_eq(&hash, &record.hash) {
            return Err("Неверный пароль".into());
        }
        if let Some(record) = self.users.get_mut(&key) {
            record.last_login = now_unix();
        }
        Ok(key)
    }

    pub fn record(&self, key: &str) -> Option<&UserRecord> {
        self.users.get(key)
    }

    pub fn update_stats(&mut self, key: &str, stats: Stats) {
        if let Some(record) = self.users.get_mut(key) {
            record.stats = stats;
            record.last_login = now_unix();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_salted_and_verifies() {
        let a = hash_password("salt-a", "hunter2");
        let b = hash_password("salt-b", "hunter2");
        assert_ne!(a, b);
        assert_eq!(a, hash_password("salt-a", "hunter2"));
        assert_ne!(a, hash_password("salt-a", "wrong"));
    }

    #[test]
    fn register_and_login_flow() {
        let mut store = UserStore::default();
        let key = store.register("Slava", "1234").unwrap();
        assert_eq!(key, "slava");
        assert!(store.register("slava", "9999").is_err());
        assert!(store.login("SLAVA", "bad").is_err());
        assert_eq!(store.login("SLAVA", "1234").unwrap(), "slava");
        let record = store.record("slava").unwrap();
        assert_eq!(record.display_name, "Slava");
        assert!(!record.hash.contains("1234"));
    }

    #[test]
    fn validation_rules() {
        let mut store = UserStore::default();
        assert!(store.register("ab", "1234").is_err());
        assert!(store.register("bad name", "1234").is_err());
        assert!(store.register("good_name", "1").is_err());
        assert!(store.register("good_name", "1234").is_ok());
    }

    #[test]
    fn store_roundtrip() {
        let mut store = UserStore::default();
        store.register("tester", "pass1234").unwrap();
        let dir = std::env::temp_dir().join(format!("rustblox_test_{}", now_unix()));
        let path = dir.join("users.json");
        store.save(&path).unwrap();
        let loaded = UserStore::load(&path);
        assert!(loaded.record("tester").is_some());
        let _ = fs::remove_dir_all(dir);
    }
}
