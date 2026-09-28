//! Places to keep projects (저장 위치): folders a sync program keeps the same
//! on the writer's other devices (OneDrive, Google Drive, Dropbox, iCloud
//! Drive), and a folder on this computer only.
//!
//! A project inside one of those folders goes to the other devices by itself;
//! the app only needs to cope with changes arriving (copies.rs). Detection
//! looks where each program keeps its folder:
//!
//! | program | Windows | macOS |
//! |---|---|---|
//! | OneDrive | `%OneDrive%`, `%OneDriveConsumer%`, `%OneDriveCommercial%` | `~/Library/CloudStorage/OneDrive-*` |
//! | Google Drive | `X:\My Drive` (its own drive letter), `~\My Drive` | `~/Library/CloudStorage/GoogleDrive-*/My Drive` |
//! | Dropbox | `info.json` under `%APPDATA%` or `%LOCALAPPDATA%\Dropbox` | `~/.dropbox/info.json` |
//! | iCloud Drive | `~\iCloudDrive` | `~/Library/Mobile Documents/com~apple~CloudDocs` |
//!
//! Google Drive names its folder in the computer's language ("내 드라이브").

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::project::APP_NAME;
use crate::store::read_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Service {
    Onedrive,
    Googledrive,
    Dropbox,
    Icloud,
    /// This computer only.
    Local,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub service: Service,
    /// Name to show, e.g. "OneDrive", "OneDrive - 한빛출판", "Google Drive".
    pub label: String,
    /// The folder the program keeps in step (for `Local`, the suggested
    /// folder's parent).
    pub root: String,
    /// Where new projects go in this place.
    pub suggested: String,
}

/// What detection needs to know about the computer, so it can be tested.
#[derive(Debug, Clone, Default)]
pub struct Env {
    pub home: Option<PathBuf>,
    /// The Documents folder (it may itself be inside OneDrive).
    pub documents: Option<PathBuf>,
    /// Environment variables: OneDrive*, APPDATA, LOCALAPPDATA.
    pub vars: HashMap<String, String>,
    /// Roots of local drives (Windows), where Google Drive puts its own.
    pub drives: Vec<PathBuf>,
}

impl Env {
    pub fn current(documents: Option<PathBuf>) -> Env {
        let vars = [
            "OneDrive",
            "OneDriveConsumer",
            "OneDriveCommercial",
            "APPDATA",
            "LOCALAPPDATA",
        ]
        .into_iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v)))
        .collect();
        Env {
            home: std::env::home_dir(),
            documents,
            vars,
            drives: local_drives(),
        }
    }

    fn var(&self, key: &str) -> Option<PathBuf> {
        self.vars
            .get(key)
            .filter(|v| !v.trim().is_empty())
            .map(PathBuf::from)
    }
}

#[cfg(windows)]
fn local_drives() -> Vec<PathBuf> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    // Removable, fixed and RAM disks: Google Drive's drive counts as fixed.
    // Network drives are left out: looking into a disconnected one can hang.
    const LOCAL_TYPES: [u32; 3] = [2, 3, 6];
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| format!("{}:\\", (b'A' + i) as char))
        .filter(|root| {
            let wide: Vec<u16> = root.encode_utf16().chain(Some(0)).collect();
            LOCAL_TYPES.contains(&unsafe { GetDriveTypeW(wide.as_ptr()) })
        })
        .map(PathBuf::from)
        .collect()
}

#[cfg(not(windows))]
fn local_drives() -> Vec<PathBuf> {
    Vec::new()
}

const MY_DRIVE: [&str; 2] = ["My Drive", "내 드라이브"];

fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Children of `dir` whose names start with `prefix`.
fn children_starting(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    out
}

/// The Dropbox folders named in its `info.json` ("personal", "business").
fn dropbox_folders(info: &Path) -> Vec<PathBuf> {
    let Some(value) = read_text(info)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
    else {
        return Vec::new();
    };
    ["personal", "business"]
        .iter()
        .filter_map(|k| value.get(k)?.get("path")?.as_str().map(PathBuf::from))
        .collect()
}

/// Sync folders on this computer, then this computer only (`Local`).
pub fn detect(env: &Env) -> Vec<Place> {
    let mut found: Vec<(Service, PathBuf, String)> = Vec::new();
    let home = env.home.clone();

    // OneDrive
    let mut onedrive: Vec<PathBuf> = ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"]
        .iter()
        .filter_map(|k| env.var(k))
        .collect();
    if let Some(home) = &home {
        onedrive.extend(children_starting(
            &home.join("Library").join("CloudStorage"),
            "OneDrive",
        ));
    }
    for dir in onedrive {
        let name = folder_name(&dir);
        // macOS: "OneDrive-Personal", "OneDrive-한빛출판".
        let label = match name.strip_prefix("OneDrive-") {
            Some("Personal") => "OneDrive".to_string(),
            Some(org) => format!("OneDrive - {org}"),
            None if name.is_empty() => "OneDrive".to_string(),
            None => name,
        };
        found.push((Service::Onedrive, dir, label));
    }

    // Google Drive: its own drive letter, a folder in the home folder
    // (mirroring), or CloudStorage on macOS.
    let mut google: Vec<PathBuf> = Vec::new();
    for base in env.drives.iter().cloned().chain(home.clone()) {
        google.extend(MY_DRIVE.iter().map(|n| base.join(n)));
    }
    if let Some(home) = &home {
        for account in children_starting(&home.join("Library").join("CloudStorage"), "GoogleDrive")
        {
            google.extend(MY_DRIVE.iter().map(|n| account.join(n)));
        }
    }
    for dir in google {
        found.push((Service::Googledrive, dir, "Google Drive".into()));
    }

    // Dropbox
    let mut infos: Vec<PathBuf> = ["APPDATA", "LOCALAPPDATA"]
        .iter()
        .filter_map(|k| env.var(k).map(|d| d.join("Dropbox").join("info.json")))
        .collect();
    if let Some(home) = &home {
        infos.push(home.join(".dropbox").join("info.json"));
    }
    for dir in infos.iter().flat_map(|i| dropbox_folders(i)) {
        let label = match folder_name(&dir) {
            name if name.starts_with("Dropbox") => name,
            _ => "Dropbox".to_string(),
        };
        found.push((Service::Dropbox, dir, label));
    }

    // iCloud Drive
    if let Some(home) = &home {
        found.push((
            Service::Icloud,
            home.join("iCloudDrive"),
            "iCloud Drive".into(),
        ));
        found.push((
            Service::Icloud,
            home.join("Library")
                .join("Mobile Documents")
                .join("com~apple~CloudDocs"),
            "iCloud Drive".into(),
        ));
    }

    let mut places: Vec<Place> = Vec::new();
    for (service, dir, label) in found {
        if !dir.is_dir() || places.iter().any(|p| same_path(Path::new(&p.root), &dir)) {
            continue;
        }
        // Keep the usual Documents/WriterProgram when Documents is in here.
        let base = match &env.documents {
            Some(docs) if inside(docs, &dir) => docs.clone(),
            _ => dir.clone(),
        };
        places.push(Place {
            service,
            label,
            root: dir.to_string_lossy().into_owned(),
            suggested: base.join(APP_NAME).to_string_lossy().into_owned(),
        });
    }

    // This computer only: Documents, unless a sync program keeps it in step;
    // then the home folder.
    let docs_synced = env
        .documents
        .as_ref()
        .is_some_and(|d| places.iter().any(|p| inside(d, Path::new(&p.root))));
    let local = match (&env.documents, &home) {
        (Some(docs), _) if !docs_synced => Some(docs.clone()),
        (_, Some(home)) => Some(home.clone()),
        (Some(docs), None) => Some(docs.clone()),
        (None, None) => None,
    };
    if let Some(base) = local {
        places.push(Place {
            service: Service::Local,
            label: "이 PC".into(),
            root: base.to_string_lossy().into_owned(),
            suggested: base.join(APP_NAME).to_string_lossy().into_owned(),
        });
    }
    places
}

/// The sync folder a path is in, if any.
pub fn place_of<'a>(path: &Path, places: &'a [Place]) -> Option<&'a Place> {
    places
        .iter()
        .filter(|p| p.service != Service::Local && inside(path, Path::new(&p.root)))
        .max_by_key(|p| p.root.len())
}

/// Path parts, compared without regard to case on Windows and macOS.
fn parts(path: &Path) -> Vec<String> {
    path.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| {
            let s = c.as_os_str().to_string_lossy();
            let s = s.trim_end_matches(['\\', '/']);
            if cfg!(any(windows, target_os = "macos")) {
                s.to_lowercase()
            } else {
                s.to_string()
            }
        })
        .collect()
}

/// Whether `path` is `dir` or inside it.
pub fn inside(path: &Path, dir: &Path) -> bool {
    let (p, d) = (parts(path), parts(dir));
    p.len() >= d.len() && p[..d.len()] == d[..]
}

fn same_path(a: &Path, b: &Path) -> bool {
    parts(a) == parts(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_each_program_and_keeps_documents_default() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("kim");
        let onedrive = home.join("OneDrive");
        let docs = onedrive.join("문서");
        let google = tmp.path().join("G").join("내 드라이브");
        let dropbox = home.join("Dropbox (한빛)");
        let appdata = home.join("AppData").join("Roaming");
        for d in [
            &docs,
            &google,
            &dropbox,
            &appdata.join("Dropbox"),
            &home.join("iCloudDrive"),
        ] {
            fs::create_dir_all(d).unwrap();
        }
        fs::write(
            appdata.join("Dropbox").join("info.json"),
            format!(
                r#"{{"business": {{"path": {}, "host": 1}}}}"#,
                serde_json::to_string(&dropbox.to_string_lossy()).unwrap()
            ),
        )
        .unwrap();
        let env = Env {
            home: Some(home.clone()),
            documents: Some(docs.clone()),
            vars: [
                (
                    "OneDrive".to_string(),
                    onedrive.to_string_lossy().into_owned(),
                ),
                (
                    "OneDriveConsumer".to_string(),
                    onedrive.to_string_lossy().into_owned(),
                ),
                (
                    "APPDATA".to_string(),
                    appdata.to_string_lossy().into_owned(),
                ),
            ]
            .into(),
            drives: vec![tmp.path().join("G")],
        };
        let places = detect(&env);
        let kinds: Vec<_> = places
            .iter()
            .map(|p| (p.service, p.label.as_str()))
            .collect();
        assert_eq!(
            kinds,
            [
                (Service::Onedrive, "OneDrive"),
                (Service::Googledrive, "Google Drive"),
                (Service::Dropbox, "Dropbox (한빛)"),
                (Service::Icloud, "iCloud Drive"),
                (Service::Local, "이 PC"),
            ]
        );
        // Documents is inside OneDrive: new projects keep going there.
        assert_eq!(
            places[0].suggested,
            docs.join("WriterProgram").to_string_lossy()
        );
        assert_eq!(
            places[1].suggested,
            google.join("WriterProgram").to_string_lossy()
        );
        // This computer only: the home folder, since Documents is synced.
        assert_eq!(
            places[4].suggested,
            home.join("WriterProgram").to_string_lossy()
        );

        let project = docs.join("WriterProgram").join("달빛 서점");
        assert_eq!(
            place_of(&project, &places).unwrap().service,
            Service::Onedrive
        );
        assert!(place_of(&home.join("WriterProgram").join("x"), &places).is_none());
    }

    #[test]
    fn nothing_synced() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("Documents");
        fs::create_dir_all(&docs).unwrap();
        let env = Env {
            home: Some(tmp.path().to_path_buf()),
            documents: Some(docs.clone()),
            ..Default::default()
        };
        let places = detect(&env);
        assert_eq!(places.len(), 1);
        assert_eq!(places[0].service, Service::Local);
        assert_eq!(
            places[0].suggested,
            docs.join("WriterProgram").to_string_lossy()
        );
    }

    #[test]
    fn inside_ignores_case_where_the_system_does() {
        assert!(inside(Path::new("/a/b/c"), Path::new("/a/b")));
        assert!(inside(Path::new("/a/b"), Path::new("/a/b/")));
        assert!(!inside(Path::new("/a/bc"), Path::new("/a/b")));
        if cfg!(windows) {
            assert!(inside(
                Path::new(r"C:\Users\Kim\OneDrive\x"),
                Path::new(r"c:\users\kim\onedrive")
            ));
        }
    }
}
