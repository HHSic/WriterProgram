//! Check the actual uploaded Windows installer with the same public key that
//! installed copies trust, before making the release public.
use std::{error::Error, path::Path};

use base64::{Engine, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use serde_json::Value;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=3).contains(&args.len()) {
        return Err(
            "usage: verify_release <vX.Y.Z> <downloaded-assets-folder> [tauri.conf.json]".into(),
        );
    }
    let version = args[0].strip_prefix('v').ok_or("expected a version tag")?;
    let folder = Path::new(&args[1]);
    let config_path = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("app/src-tauri/tauri.conf.json");
    let config: Value = serde_json::from_slice(&std::fs::read(config_path)?)?;
    let latest: Value = serde_json::from_slice(&std::fs::read(folder.join("latest.json"))?)?;
    if latest["version"] != version || config["version"] != version {
        return Err("update metadata, tag and app versions do not match".into());
    }
    chrono::DateTime::parse_from_rfc3339(
        latest["pub_date"].as_str().ok_or("missing release date")?,
    )?;
    let name = format!("WriterProgram_{version}_x64-setup.exe");
    let bytes = std::fs::read(folder.join(&name))?;
    if !bytes.starts_with(b"MZ") || bytes.len() < 1024 {
        return Err("missing or invalid Windows installer".into());
    }
    let encoded_signature = std::fs::read_to_string(folder.join(format!("{name}.sig")))?;
    let signature = Signature::decode(&String::from_utf8(
        STANDARD.decode(encoded_signature.trim())?,
    )?)?;
    let key = PublicKey::decode(&String::from_utf8(
        STANDARD.decode(
            config["plugins"]["updater"]["pubkey"]
                .as_str()
                .ok_or("missing updater public key")?,
        )?,
    )?)?;
    key.verify(&bytes, &signature, false)?;
    let expected_url = format!(
        "https://github.com/HHSic/WriterProgram/releases/download/{}/{name}",
        args[0]
    );
    for platform in ["windows-x86_64", "windows-x86_64-nsis"] {
        let target = &latest["platforms"][platform];
        if target["url"] != expected_url
            || target["signature"].as_str() != Some(encoded_signature.trim())
        {
            return Err(format!(
                "{platform}: update URL or signature does not match the installer"
            )
            .into());
        }
    }
    println!(
        "Verified {}: installer, update metadata and signature match.",
        args[0]
    );
    Ok(())
}
