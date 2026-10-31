//! Account tokens and private feed addresses: DPAPI current-user encryption on Windows.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
static CREDENTIAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn credential_lock() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    CREDENTIAL_LOCK
        .lock()
        .map_err(|_| "Credential operation failed; restart Lightcal".into())
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
        std::fs::write(&temp, protected).map_err(|_| "Could not save protected credentials")?;
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
