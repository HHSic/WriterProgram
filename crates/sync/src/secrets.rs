//! Where sign-ins are kept. The app stores them in the system's credential
//! store (Windows 자격 증명 관리자, macOS·iOS 키체인, Android 키스토어), never
//! in the project folder or a settings file; tests keep them in memory.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::Result;

pub trait Secrets: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>>;
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
}

/// Secrets kept in memory only (tests, or when the system store is missing).
#[derive(Default)]
pub struct MemorySecrets(Mutex<HashMap<String, String>>);

impl Secrets for MemorySecrets {
    fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(key)
            .cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(key.into(), value.into());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).remove(key);
        Ok(())
    }
}
