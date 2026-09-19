// License + device binding.
//
// DEVIATION FROM SPEC (documented): the original verifies Ed25519-signed keys
// against oopsie.bio's private key, which we do not possess. This local rebuild
// therefore grants a valid "team" license by default so the app runs, while
// keeping the faithful activate/deactivate/device-id surface. The device
// fingerprint algorithm (SHA-256 over the "oopsie-device-v2|" salt) is real.
use crate::models::LicenseStatus;
use crate::storage::data_root;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
struct LocalLicense {
    active: bool,
    #[serde(default)]
    key: Option<String>,
}

fn license_file() -> std::path::PathBuf {
    data_root().join("license.json")
}

fn read_local() -> LocalLicense {
    match std::fs::read_to_string(license_file()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or(LocalLicense { active: true, key: None }),
        Err(_) => LocalLicense { active: true, key: None }, // default: granted
    }
}

fn write_local(l: &LocalLicense) -> Result<(), String> {
    let _ = std::fs::create_dir_all(data_root());
    std::fs::write(license_file(), serde_json::to_string(l).unwrap()).map_err(|e| e.to_string())
}

// raw OS machine id (Windows MachineGuid; else hostname)
fn machine_id() -> Option<String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("reg")
            .args(["query", r"HKLM\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"])
            .creation_flags(0x0800_0000)
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            if line.contains("MachineGuid") {
                if let Some(g) = line.split_whitespace().last() {
                    if g.len() > 8 {
                        return Some(g.to_string());
                    }
                }
            }
        }
    }
    std::env::var("COMPUTERNAME").ok().or_else(|| std::env::var("HOSTNAME").ok())
}

// device_fingerprint = hex(SHA-256("oopsie-device-v2|" + machine_id))
pub fn device_fingerprint() -> String {
    match machine_id() {
        Some(id) => {
            let mut h = Sha256::new();
            h.update(b"oopsie-device-v2|");
            h.update(id.as_bytes());
            hex::encode(h.finalize())
        }
        None => "INVALID-DEVICE:no-stable-machine-id".to_string(),
    }
}

pub fn status() -> LicenseStatus {
    let l = read_local();
    if l.active {
        LicenseStatus {
            valid: true,
            license_type: Some("team".into()),
            expires_at: None,
            device_bound: true,
            team_id: None,
            message: Some("Team license active".into()),
        }
    } else {
        LicenseStatus {
            valid: false,
            license_type: None,
            expires_at: None,
            device_bound: false,
            team_id: None,
            message: Some("No license activated".into()),
        }
    }
}

pub fn activate(key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Invalid license format".into());
    }
    write_local(&LocalLicense { active: true, key: Some(key.to_string()) })
}

pub fn deactivate() -> Result<(), String> {
    write_local(&LocalLicense { active: false, key: None })
}
