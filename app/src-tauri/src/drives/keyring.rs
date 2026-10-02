//! The sign-ins (refresh tokens) in the system's credential store.

use writer_sync::secrets::Secrets;

/// Sign-ins in the system's credential store. A value longer than one
/// credential can hold (Windows: 2,560 bytes) is split over several.
pub struct KeyringSecrets;

const SERVICE: &str = "WriterProgram";
/// Characters per credential (stored as UTF-16).
const CHUNK: usize = 1000;

fn entry(key: &str) -> writer_sync::Result<keyring_core::Entry> {
    keyring_core::Entry::new(SERVICE, key)
        .map_err(|e| writer_sync::Error::Invalid(format!("자격 증명 저장소를 쓰지 못함 ({e})")))
}

fn store_err(e: keyring_core::Error) -> writer_sync::Error {
    writer_sync::Error::Invalid(format!("자격 증명 저장소를 쓰지 못함 ({e})"))
}

impl Secrets for KeyringSecrets {
    fn get(&self, key: &str) -> writer_sync::Result<Option<String>> {
        let head = match entry(key)?.get_password() {
            Ok(v) => v,
            Err(keyring_core::Error::NoEntry) => return Ok(None),
            Err(e) => return Err(store_err(e)),
        };
        let Some(count) = head
            .strip_prefix("chunks:")
            .and_then(|n| n.parse::<usize>().ok())
        else {
            return Ok(Some(head));
        };
        let mut value = String::new();
        for i in 1..=count {
            value.push_str(
                &entry(&format!("{key}#{i}"))?
                    .get_password()
                    .map_err(store_err)?,
            );
        }
        Ok(Some(value))
    }

    fn set(&self, key: &str, value: &str) -> writer_sync::Result<()> {
        self.delete(key)?;
        let chars: Vec<char> = value.chars().collect();
        if chars.len() <= CHUNK {
            return entry(key)?.set_password(value).map_err(store_err);
        }
        let parts: Vec<String> = chars.chunks(CHUNK).map(|c| c.iter().collect()).collect();
        for (i, part) in parts.iter().enumerate() {
            entry(&format!("{key}#{}", i + 1))?
                .set_password(part)
                .map_err(store_err)?;
        }
        entry(key)?
            .set_password(&format!("chunks:{}", parts.len()))
            .map_err(store_err)
    }

    fn delete(&self, key: &str) -> writer_sync::Result<()> {
        let head = match entry(key)?.get_password() {
            Ok(v) => v,
            Err(keyring_core::Error::NoEntry) => return Ok(()),
            Err(e) => return Err(store_err(e)),
        };
        if let Some(count) = head
            .strip_prefix("chunks:")
            .and_then(|n| n.parse::<usize>().ok())
        {
            for i in 1..=count {
                let _ = entry(&format!("{key}#{i}"))?.delete_credential();
            }
        }
        match entry(key)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(e) => Err(store_err(e)),
        }
    }
}

/// Picks the system's credential store for keyring. Call once at start-up.
pub fn init_secrets() {
    #[cfg(windows)]
    if let Ok(store) = windows_native_keyring_store::Store::new() {
        keyring_core::set_default_store(store);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_sign_ins_are_split_and_joined() {
        keyring_core::set_default_store(keyring_core::mock::Store::new().unwrap());
        let s = KeyringSecrets;
        assert_eq!(s.get("t/short").unwrap(), None);
        s.set("t/short", "RT1").unwrap();
        assert_eq!(s.get("t/short").unwrap().as_deref(), Some("RT1"));

        // Microsoft's refresh tokens can be longer than one credential holds.
        let long: String = "가나다라마바사아자차카타파하0123456789".repeat(120);
        s.set("t/long", &long).unwrap();
        assert_eq!(s.get("t/long").unwrap().as_deref(), Some(long.as_str()));
        s.set("t/long", "short again").unwrap();
        assert_eq!(s.get("t/long").unwrap().as_deref(), Some("short again"));
        s.delete("t/long").unwrap();
        s.delete("t/short").unwrap();
        assert_eq!(s.get("t/long").unwrap(), None);
        assert_eq!(s.get("t/short").unwrap(), None);
    }
}
