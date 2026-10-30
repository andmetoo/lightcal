//! Authenticode-pinned portable Windows updates. No elevation and no signing keys.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Package deployment owns MSIX binaries, including sideloaded packages.
pub fn package_managed() -> bool {
    #[cfg(windows)]
    {
        let mut length = 0;
        let status = unsafe {
            windows_sys::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName(
                &mut length,
                std::ptr::null_mut(),
            )
        };
        // Unknown errors fail closed; only APPMODEL_ERROR_NO_PACKAGE permits portable replacement.
        status != 15700
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn ensure_portable_update_allowed(managed: bool) -> Result<(), String> {
    if managed {
        Err("This MSIX installation is updated by Microsoft Store or its package deployment channel; portable EXE replacement is disabled".into())
    } else {
        Ok(())
    }
}

pub fn certificate_pin() -> Result<String, String> {
    normalize_pin(option_env!("LIGHTCAL_UPDATE_CERT_SHA256").unwrap_or(""))
}
fn normalize_pin(raw: &str) -> Result<String, String> {
    let pin = raw.replace([' ', ':'], "").to_ascii_uppercase();
    if pin.len() != 64 || !pin.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Automatic installation is unavailable: build with LIGHTCAL_UPDATE_CERT_SHA256 set to the SHA-256 fingerprint of your SimpleSign signing certificate".into());
    }
    Ok(pin)
}

#[derive(Serialize, Deserialize)]
pub struct InstallPlan {
    target: PathBuf,
    candidate: PathBuf,
    expected_version: String,
    sha256: String,
    parent_pid: u32,
}

pub fn stage(data_dir: &Path) -> Result<PathBuf, String> {
    ensure_portable_update_allowed(package_managed())?;
    let pin = certificate_pin()?;
    if !cfg!(all(windows, target_arch = "x86_64")) {
        return Err("Automatic installation currently supports Windows x64 portable builds".into());
    }
    let (status, bytes) = crate::net::get(
        "https://api.github.com/repos/andmetoo/lightcal/releases/latest",
        None,
        2 * 1024 * 1024,
    )?;
    if status != 200 {
        return Err(format!("No downloadable stable update (HTTP {status})"));
    }
    let release: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid release metadata")?;
    let version = release["tag_name"]
        .as_str()
        .ok_or("Release has no version")?
        .trim_start_matches('v');
    validate_version(version)?;
    if release["draft"].as_bool() == Some(true) || release["prerelease"].as_bool() == Some(true) {
        return Err("Only stable releases can be installed".into());
    }
    let asset = release["assets"]
        .as_array()
        .ok_or("Release has no assets")?
        .iter()
        .find(|a| a["name"].as_str() == Some("lightcal.exe"))
        .ok_or("The release needs a signed lightcal.exe asset for automatic installation")?;
    let address = asset["browser_download_url"]
        .as_str()
        .ok_or("Update asset has no download address")?;
    let payload = crate::net::download_release_asset(address)?;
    if payload.get(..2) != Some(b"MZ") {
        return Err("Update asset is not a Windows executable".into());
    }
    let sha = format!("{:x}", Sha256::digest(&payload));
    if let Some(digest) = asset["digest"].as_str() {
        if digest != format!("sha256:{sha}") {
            return Err("GitHub asset digest does not match the downloaded update".into());
        }
    }
    let stage_dir = data_dir
        .join("updates")
        .join(crate::calendars::random_key()?);
    std::fs::create_dir_all(&stage_dir).map_err(|_| "Could not create update staging directory")?;
    let candidate = stage_dir.join("candidate.exe");
    std::fs::write(&candidate, payload).map_err(|_| "Could not stage the update executable")?;
    verify_signed_file(&candidate, version, &pin)?;
    let target = std::env::current_exe()
        .map_err(|_| "Could not locate the running executable")?
        .canonicalize()
        .map_err(|_| "Could not resolve installation path")?;
    // A trusted current installation is required, preventing an unsigned bootstrap from asserting trust.
    verify_signed_file(&target, env!("CARGO_PKG_VERSION"), &pin)?;
    let plan = InstallPlan {
        target,
        candidate,
        expected_version: version.into(),
        sha256: sha,
        parent_pid: std::process::id(),
    };
    let plan_path = stage_dir.join("install.json");
    std::fs::write(
        &plan_path,
        serde_json::to_vec(&plan).map_err(|_| "Could not encode update plan")?,
    )
    .map_err(|_| "Could not write update plan")?;
    Ok(plan_path)
}

fn validate_version(version: &str) -> Result<(), String> {
    let next = crate::updater::SemVer::parse(version).ok_or("Invalid update version")?;
    let current = crate::updater::SemVer::parse(env!("CARGO_PKG_VERSION"))
        .ok_or("Invalid installed version")?;
    if next <= current {
        return Err("Update version must be newer than this installation".into());
    }
    Ok(())
}

#[cfg(windows)]
fn verify_signed_file(file: &Path, version: &str, pin: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    // Arguments travel via process environment, never through interpolated PowerShell code.
    let script = r#"$ErrorActionPreference='Stop'; $sig=Get-AuthenticodeSignature -LiteralPath $env:LIGHTCAL_VERIFY_FILE; if($sig.Status -ne 'Valid' -or $null -eq $sig.SignerCertificate){exit 11}; $sha=[Security.Cryptography.SHA256]::Create(); $pin=[BitConverter]::ToString($sha.ComputeHash($sig.SignerCertificate.RawData)).Replace('-',''); if($pin -ne $env:LIGHTCAL_VERIFY_PIN){exit 12}; $v=[Diagnostics.FileVersionInfo]::GetVersionInfo($env:LIGHTCAL_VERIFY_FILE).ProductVersion; if($v -ne $env:LIGHTCAL_VERIFY_VERSION){exit 13}"#;
    let powershell = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .ok_or("Windows system directory is unavailable")?
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut child = std::process::Command::new(powershell)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .env("LIGHTCAL_VERIFY_FILE", file)
        .env("LIGHTCAL_VERIFY_PIN", pin)
        .env("LIGHTCAL_VERIFY_VERSION", version)
        .creation_flags(0x08000000)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| "Could not invoke Windows signature verification")?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| "Windows signature verification failed")?
        {
            if !status.success() {
                return Err("Update rejected: invalid Authenticode trust, wrong signing certificate, or mismatched signed version".into());
            }
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Windows signature verification timed out; update cancelled".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
#[cfg(not(windows))]
fn verify_signed_file(_: &Path, _: &str, _: &str) -> Result<(), String> {
    Err("Windows signature verification is unavailable".into())
}

pub fn launch_helper(plan_path: &Path) -> Result<(), String> {
    let helper = plan_path
        .parent()
        .ok_or("Invalid update staging path")?
        .join("installer.exe");
    std::fs::copy(
        std::env::current_exe().map_err(|_| "Could not locate updater helper")?,
        &helper,
    )
    .map_err(|_| "Could not stage updater helper")?;
    let mut child = std::process::Command::new(helper)
        .arg("--apply-update")
        .arg(plan_path)
        .spawn()
        .map_err(|_| "Could not start updater helper")?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if plan_path.with_extension("ready").exists() {
            return Ok(());
        }
        if child
            .try_wait()
            .map_err(|_| "Could not check updater helper")?
            .is_some()
        {
            return Err("Updater helper rejected the plan before installation; the current application remains running".into());
        }
        if std::time::Instant::now() >= deadline {
            return Err(
                "Updater helper did not become ready; the current application remains running"
                    .into(),
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

pub fn apply(plan_path: &Path) -> Result<(), String> {
    let bytes = read_plan(plan_path)?;
    if bytes.len() > 16384 {
        return Err("Update plan is too large".into());
    }
    let plan: InstallPlan = serde_json::from_slice(&bytes).map_err(|_| "Invalid update plan")?;
    let pin = certificate_pin()?;
    validate_version(&plan.expected_version)?;
    if plan.target.file_name().and_then(|n| n.to_str()) != Some("lightcal.exe")
        || !plan.target.is_absolute()
    {
        return Err("Unsupported update target; use a portable lightcal.exe installation".into());
    }
    let expected_candidate = plan_path
        .parent()
        .ok_or("Invalid update plan directory")?
        .join("candidate.exe")
        .canonicalize()
        .map_err(|_| "Missing update candidate")?;
    if plan
        .candidate
        .canonicalize()
        .map_err(|_| "Invalid update candidate")?
        != expected_candidate
    {
        return Err("Update candidate is outside its staging directory".into());
    }
    verify_payload(&plan.candidate, &plan.sha256)?;
    verify_signed_file(&plan.candidate, &plan.expected_version, &pin)?;
    wait_for_parent(plan.parent_pid, &plan_path.with_extension("ready"))?;
    verify_signed_file(&plan.target, env!("CARGO_PKG_VERSION"), &pin)?;
    let parent = plan
        .target
        .parent()
        .ok_or("Invalid installation directory")?;
    let suffix = crate::calendars::random_key()?;
    let replacement = parent.join(format!("lightcal-{suffix}.new.exe"));
    let backup = parent.join(format!("lightcal-{suffix}.previous.exe"));
    std::fs::copy(&plan.candidate, &replacement)
        .map_err(|_| "Installation directory is not writable; portable update cannot proceed")?;
    verify_payload(&replacement, &plan.sha256)?;
    verify_signed_file(&replacement, &plan.expected_version, &pin)?;
    replace(&plan.target, &replacement, &backup)?;
    match std::process::Command::new(&plan.target)
        .arg("--tray")
        .spawn()
    {
        Ok(mut process) => {
            std::thread::sleep(std::time::Duration::from_secs(3));
            if process
                .try_wait()
                .map_err(|_| "Could not check updated process")?
                .is_some()
            {
                replace(
                    &plan.target,
                    &backup,
                    &parent.join(format!("lightcal-{suffix}.failed.exe")),
                )?;
                let _ = std::process::Command::new(&plan.target)
                    .arg("--tray")
                    .spawn();
                return Err(
                    "Updated application exited immediately; restored the previous executable"
                        .into(),
                );
            }
            // Keep the previous executable for owner recovery; clean staged credentials-free files later.
            let _ = std::fs::write(
                plan_path.with_extension("result.txt"),
                "Installed and restarted successfully",
            );
            Ok(())
        }
        Err(_) => {
            replace(
                &plan.target,
                &backup,
                &parent.join(format!("lightcal-{suffix}.failed.exe")),
            )?;
            let _ = std::process::Command::new(&plan.target)
                .arg("--tray")
                .spawn();
            Err("Could not restart update; previous executable restored".into())
        }
    }
}

fn verify_payload(path: &Path, expected: &str) -> Result<(), String> {
    let metadata = std::fs::metadata(path).map_err(|_| "Missing staged update")?;
    if !metadata.is_file() || metadata.len() > 128 * 1024 * 1024 {
        return Err("Invalid staged update size/type".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "Could not read staged update")?;
    if format!("{:x}", Sha256::digest(bytes)) != expected {
        return Err("Staged update changed after verification".into());
    }
    Ok(())
}

#[cfg(windows)]
fn wait_for_parent(pid: u32, ready: &Path) -> Result<(), String> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    };
    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
    if handle.is_null() {
        return Err("Could not wait for the running installation; update cancelled".into());
    }
    if std::fs::write(ready, b"ready").is_err() {
        unsafe {
            CloseHandle(handle);
        }
        return Err("Could not acknowledge updater readiness".into());
    }
    let status = unsafe { WaitForSingleObject(handle, 60_000) };
    unsafe {
        CloseHandle(handle);
    }
    if status != 0 {
        return Err("Running application did not exit; update cancelled".into());
    }
    Ok(())
}
#[cfg(not(windows))]
fn wait_for_parent(_: u32, _: &Path) -> Result<(), String> {
    Err("Windows installer helper is unavailable".into())
}

pub fn recover_after_failure(plan_path: &Path) {
    let attempt = || -> Result<(), String> {
        let bytes = read_plan(plan_path)?;
        let plan: InstallPlan =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid recovery plan")?;
        if plan.target.file_name().and_then(|s| s.to_str()) != Some("lightcal.exe") {
            return Err("Invalid recovery target".into());
        }
        verify_signed_file(&plan.target, env!("CARGO_PKG_VERSION"), &certificate_pin()?)?;
        std::process::Command::new(plan.target)
            .arg("--tray")
            .spawn()
            .map_err(|_| "Could not restart the previous version")?;
        Ok(())
    };
    let _ = attempt();
}

fn read_plan(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|_| "Could not read update plan")?;
    let mut bytes = Vec::new();
    file.take(16385)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read update plan")?;
    if bytes.len() > 16384 {
        return Err("Update plan is too large".into());
    }
    Ok(bytes)
}

#[cfg(windows)]
fn replace(target: &Path, new: &Path, backup: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let wide = |p: &Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let (target, new, backup) = (wide(target), wide(new), wide(backup));
    let ok = unsafe {
        windows_sys::Win32::Storage::FileSystem::ReplaceFileW(
            target.as_ptr(),
            new.as_ptr(),
            backup.as_ptr(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if ok == 0 {
        return Err(
            "Atomic executable replacement failed; previous installation was retained".into(),
        );
    }
    Ok(())
}
#[cfg(not(windows))]
fn replace(_: &Path, _: &Path, _: &Path) -> Result<(), String> {
    Err("Windows atomic replacement is unavailable".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pin_and_version_checks_fail_closed() {
        assert!(ensure_portable_update_allowed(true).is_err());
        assert!(ensure_portable_update_allowed(false).is_ok());
        for pin in ["", "DEADBEEF", "not a certificate"] {
            assert!(normalize_pin(pin).is_err());
        }
        assert_eq!(normalize_pin(&"ab".repeat(32)).unwrap(), "AB".repeat(32));
        assert!(validate_version(env!("CARGO_PKG_VERSION")).is_err());
        assert!(validate_version("0.0.0").is_err());
        assert!(validate_version("999.0.0-beta").is_err());
    }
    #[test]
    fn changed_download_is_rejected_in_temporary_directory() {
        let dir = std::env::temp_dir().join(format!(
            "lightcal-update-test-{}",
            crate::calendars::random_key().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("candidate.exe");
        std::fs::write(&file, b"original").unwrap();
        let hash = format!("{:x}", Sha256::digest(b"original"));
        assert!(verify_payload(&file, &hash).is_ok());
        std::fs::write(&file, b"changed").unwrap();
        assert!(verify_payload(&file, &hash).is_err());
        std::fs::remove_file(file).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    #[cfg(windows)]
    fn unsigned_payload_is_rejected_by_windows_without_touching_an_installation() {
        let dir = std::env::temp_dir().join(format!(
            "lightcal-signature-test-{}",
            crate::calendars::random_key().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let candidate = dir.join("unsigned.exe");
        std::fs::write(&candidate, b"MZ deliberately unsigned fixture").unwrap();
        assert!(verify_signed_file(&candidate, "999.0.0", &"0".repeat(64)).is_err());
        std::fs::remove_file(candidate).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    #[cfg(windows)]
    fn atomic_replacement_and_rollback_preserve_the_original_in_isolated_directory() {
        let dir = std::env::temp_dir().join(format!(
            "lightcal-replace-test-{}",
            crate::calendars::random_key().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("lightcal.exe");
        let next = dir.join("next.exe");
        let backup = dir.join("previous.exe");
        let failed = dir.join("failed.exe");
        std::fs::write(&target, b"old").unwrap();
        std::fs::write(&next, b"new").unwrap();
        replace(&target, &next, &backup).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert_eq!(std::fs::read(&backup).unwrap(), b"old");
        replace(&target, &backup, &failed).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        assert_eq!(std::fs::read(&failed).unwrap(), b"new");
        for file in [target, failed] {
            std::fs::remove_file(file).unwrap();
        }
        std::fs::remove_dir(dir).unwrap();
    }
}
