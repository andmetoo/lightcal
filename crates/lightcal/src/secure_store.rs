//! Account tokens and private feed addresses: DPAPI current-user encryption on Windows.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
static CREDENTIAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serializes credential file I/O. The mutex protects ordering only, never in-memory state, so a
/// panic in another holder (now caught instead of aborting) must not disable it for good.
pub fn credential_lock() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    Ok(CREDENTIAL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshProvider {
    Microsoft,
    Google,
}

/// Applies a rotated refresh token to freshly loaded credentials, changing only that token.
/// Returns false (and changes nothing) when the account was disconnected or reconnected since
/// the refresh started, so a slow refresh can never undo a Disconnect.
pub fn merge_rotated_token(
    stored: &mut Credentials,
    provider: RefreshProvider,
    started_with: &str,
    rotated: &str,
) -> bool {
    let slot = match provider {
        RefreshProvider::Microsoft => &mut stored.microsoft_refresh_token,
        RefreshProvider::Google => &mut stored.google_refresh_token,
    };
    if slot.as_deref() != Some(started_with) {
        return false;
    }
    *slot = Some(rotated.to_string());
    true
}

/// Takes the credential lock only for this local read-merge-write, never across network calls.
/// Returns whether the rotated token was stored.
pub fn save_rotated_token(
    store: &SecureStore,
    provider: RefreshProvider,
    started_with: &str,
    rotated: &str,
) -> Result<bool, String> {
    let _guard = credential_lock()?;
    let mut stored = store.load()?;
    if !merge_rotated_token(&mut stored, provider, started_with, rotated) {
        return Ok(false);
    }
    store.save(&stored)?;
    Ok(true)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct IcsSubscription {
    pub id: String,
    pub name: String,
    pub url: String,
    pub color: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Credentials {
    pub microsoft_refresh_token: Option<String>,
    #[serde(default)]
    pub microsoft_client_id: Option<String>,
    #[serde(default)]
    pub google_refresh_token: Option<String>,
    #[serde(default)]
    pub google_client_id: Option<String>,
    #[serde(default)]
    pub google_client_secret: Option<String>,
    #[serde(default)]
    pub ics: Vec<IcsSubscription>,
}

pub struct SecureStore {
    path: PathBuf,
}

impl SecureStore {
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join("credentials.dpapi"),
        }
    }
    pub fn load(&self) -> Result<Credentials, String> {
        match std::fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&unprotect(&bytes)?)
                .map_err(|_| "Stored credentials are invalid; reconnect the account".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Credentials::default()),
            Err(_) => Err("Could not read protected credentials".into()),
        }
    }
    pub fn save(&self, credentials: &Credentials) -> Result<(), String> {
        let bytes = serde_json::to_vec(credentials)
            .map_err(|_| "Could not encode credentials".to_string())?;
        let protected = protect(&bytes)?;
        let dir = self.path.parent().ok_or("Invalid credential directory")?;
        std::fs::create_dir_all(dir).map_err(|_| "Could not create credential directory")?;
        let temp = dir.join(format!(
            "credentials-{}.tmp",
            crate::calendars::random_key()?
        ));
        let written = (|| -> std::io::Result<()> {
            use std::io::Write;
            let mut file = std::fs::File::create(&temp)?;
            file.write_all(&protected)?;
            file.sync_all()
        })();
        if written.is_err() {
            let _ = std::fs::remove_file(&temp);
            return Err("Could not save protected credentials".into());
        }
        if std::fs::rename(&temp, &self.path).is_err() {
            let _ = std::fs::remove_file(temp);
            return Err("Could not replace protected credentials".into());
        }
        Ok(())
    }
}

#[cfg(windows)]
fn transform(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(bytes.len()).map_err(|_| "Credentials are too large")?,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // DPAPI copies the borrowed input and allocates output with LocalAlloc.
    let ok = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err("Windows could not protect/unlock credentials for this user".into());
    }
    let result =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData as *mut _);
    }
    Ok(result)
}
#[cfg(windows)]
fn protect(bytes: &[u8]) -> Result<Vec<u8>, String> {
    transform(bytes, true)
}
#[cfg(windows)]
fn unprotect(bytes: &[u8]) -> Result<Vec<u8>, String> {
    transform(bytes, false)
}
#[cfg(not(windows))]
fn protect(_: &[u8]) -> Result<Vec<u8>, String> {
    Err("Protected credential storage is not implemented on this platform".into())
}
#[cfg(not(windows))]
fn unprotect(_: &[u8]) -> Result<Vec<u8>, String> {
    Err("Protected credential storage is not implemented on this platform".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_lock_recovers_after_a_holder_panics() {
        let poisoner = std::thread::spawn(|| {
            let _guard = credential_lock().unwrap();
            panic!("simulated provider panic while holding the credential lock");
        });
        assert!(poisoner.join().is_err());
        assert!(credential_lock().is_ok());
    }

    fn connected() -> Credentials {
        Credentials {
            microsoft_refresh_token: Some("ms-old".into()),
            google_refresh_token: Some("g-old".into()),
            google_client_id: Some("client".into()),
            ics: vec![IcsSubscription {
                id: "ics:1".into(),
                name: "Feed".into(),
                url: "https://example.com/a.ics".into(),
                color: "#10B981".into(),
            }],
            ..Credentials::default()
        }
    }

    #[test]
    fn rotation_changes_only_the_refreshed_token() {
        let mut stored = connected();
        assert!(merge_rotated_token(
            &mut stored,
            RefreshProvider::Microsoft,
            "ms-old",
            "ms-new"
        ));
        assert_eq!(stored.microsoft_refresh_token.as_deref(), Some("ms-new"));
        assert_eq!(stored.google_refresh_token.as_deref(), Some("g-old"));
        assert_eq!(stored.google_client_id.as_deref(), Some("client"));
        assert_eq!(stored.ics.len(), 1);
        assert!(merge_rotated_token(
            &mut stored,
            RefreshProvider::Google,
            "g-old",
            "g-new"
        ));
        assert_eq!(stored.google_refresh_token.as_deref(), Some("g-new"));
        assert_eq!(stored.microsoft_refresh_token.as_deref(), Some("ms-new"));
    }

    #[test]
    fn rotation_does_not_resurrect_or_overwrite_a_changed_account() {
        let mut removed = connected();
        removed.microsoft_refresh_token = None;
        assert!(!merge_rotated_token(
            &mut removed,
            RefreshProvider::Microsoft,
            "ms-old",
            "ms-new"
        ));
        assert!(removed.microsoft_refresh_token.is_none());
        let mut reconnected = connected();
        reconnected.google_refresh_token = Some("g-other".into());
        assert!(!merge_rotated_token(
            &mut reconnected,
            RefreshProvider::Google,
            "g-old",
            "g-new"
        ));
        assert_eq!(reconnected.google_refresh_token.as_deref(), Some("g-other"));
    }

    #[cfg(windows)]
    #[test]
    fn saved_rotation_merges_into_the_stored_file_and_skips_after_disconnect() {
        let dir = std::env::temp_dir().join(format!(
            "lightcal-rotation-{}",
            crate::calendars::random_key().unwrap()
        ));
        let store = SecureStore::new(&dir);
        store.save(&connected()).unwrap();
        assert!(
            save_rotated_token(&store, RefreshProvider::Microsoft, "ms-old", "ms-new").unwrap()
        );
        let loaded = store.load().unwrap();
        assert_eq!(loaded.microsoft_refresh_token.as_deref(), Some("ms-new"));
        assert_eq!(loaded.google_refresh_token.as_deref(), Some("g-old"));
        let mut disconnected = loaded;
        disconnected.google_refresh_token = None;
        store.save(&disconnected).unwrap();
        assert!(!save_rotated_token(&store, RefreshProvider::Google, "g-old", "g-new").unwrap());
        assert!(store.load().unwrap().google_refresh_token.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
