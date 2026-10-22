//! `writer path add | remove | status`: puts the folder `writer` is in on the
//! user's PATH, so a terminal opened afterwards finds `writer` by name. On
//! Windows that is the `Path` value under HKCU\Environment, the per-user part
//! that needs no administrator; the installer calls this (app/src-tauri/
//! windows/hooks.nsh) rather than editing PATH itself, because NSIS strings
//! are cut at 1024 characters and a long PATH would be cut with them.

use std::path::PathBuf;

/// The folder this program is in.
pub fn folder() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("실행 파일 위치를 알 수 없음: {e}"))?;
    let exe = exe.canonicalize().unwrap_or(exe);
    exe.parent()
        .map(PathBuf::from)
        .ok_or_else(|| "실행 파일 위치를 알 수 없음".into())
}

/// Same folder, written either way: case, `/` or `\`, a trailing slash.
fn same(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        s.trim()
            .trim_matches('"')
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    !a.trim().is_empty() && norm(a) == norm(b)
}

/// Whether `list` (entries split by `;`) has `folder`.
pub fn has(list: &str, folder: &str) -> bool {
    list.split(';').any(|p| same(p, folder))
}

/// `list` with `folder` added at the end; none when it is there already.
pub fn with(list: &str, folder: &str) -> Option<String> {
    if has(list, folder) {
        return None;
    }
    let base = list.trim_end_matches(';');
    Some(if base.is_empty() {
        folder.to_string()
    } else {
        format!("{base};{folder}")
    })
}

/// `list` without `folder`, every other entry kept as it was; none when it
/// was not there.
pub fn without(list: &str, folder: &str) -> Option<String> {
    if !has(list, folder) {
        return None;
    }
    Some(
        list.split(';')
            .filter(|p| !same(p, folder))
            .collect::<Vec<_>>()
            .join(";"),
    )
}

/// `dir` as it goes on PATH, without the `\\?\` a canonical Windows path has.
pub fn shown(dir: &std::path::Path) -> String {
    dir.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string()
}

/// Whether `dir` is on the user's PATH.
pub fn is_on(dir: &str) -> Result<bool, String> {
    Ok(has(&user_path()?, dir))
}

/// Puts `dir` on or takes it off the user's PATH; whether anything changed.
pub fn set(dir: &str, on: bool) -> Result<bool, String> {
    let now = user_path()?;
    match if on {
        with(&now, dir)
    } else {
        without(&now, dir)
    } {
        None => Ok(false),
        Some(list) => set_user_path(&list).map(|()| true),
    }
}

/// What `writer path <what>` prints.
pub fn run(what: &str) -> Result<String, String> {
    let dir = shown(&folder()?);
    match what {
        "status" => Ok(if is_on(&dir)? {
            format!("등록됨: {dir}\n새로 연 터미널에서 writer로 부를 수 있습니다.")
        } else {
            format!("등록되지 않음: {dir}\nwriter path add로 등록합니다.")
        }),
        "add" => Ok(if set(&dir, true)? {
            format!("PATH에 등록함: {dir}\n지금 열려 있는 터미널은 다시 열어야 writer를 찾습니다.")
        } else {
            format!("이미 등록됨: {dir}")
        }),
        "remove" => Ok(if set(&dir, false)? {
            format!("PATH에서 뺌: {dir}")
        } else {
            format!("등록되어 있지 않음: {dir}")
        }),
        _ => Err("writer path add | remove | status".into()),
    }
}

#[cfg(windows)]
fn user_path() -> Result<String, String> {
    reg::read()
}

#[cfg(windows)]
fn set_user_path(list: &str) -> Result<(), String> {
    reg::write(list)
}

#[cfg(not(windows))]
fn user_path() -> Result<String, String> {
    Ok(std::env::var("PATH").unwrap_or_default().replace(':', ";"))
}

#[cfg(not(windows))]
fn set_user_path(_list: &str) -> Result<(), String> {
    let dir = folder()?;
    Err(format!(
        "PATH 등록은 Windows에서만 합니다. 이 셸에서는 셸 설정 파일(~/.bashrc, ~/.zshrc 등)에 다음 줄을 넣습니다.\nexport PATH=\"{}:$PATH\"",
        dir.display()
    ))
}

#[cfg(windows)]
mod reg {
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ, REG_SZ,
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            unsafe { RegCloseKey(self.0) };
        }
    }

    fn open(access: u32) -> Result<Key, String> {
        let mut key: HKEY = null_mut();
        let name = wide("Environment");
        let e = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, name.as_ptr(), 0, access, &mut key) };
        if e != ERROR_SUCCESS {
            return Err(format!("사용자 환경 변수를 열지 못함 (오류 {e})"));
        }
        Ok(Key(key))
    }

    /// The user's own Path, unexpanded (`%USERPROFILE%\…` stays as written).
    pub fn read() -> Result<String, String> {
        let key = open(KEY_QUERY_VALUE)?;
        let name = wide("Path");
        let mut kind = 0u32;
        let mut size = 0u32;
        let e = unsafe {
            RegQueryValueExW(
                key.0,
                name.as_ptr(),
                null(),
                &mut kind,
                null_mut(),
                &mut size,
            )
        };
        if e == ERROR_FILE_NOT_FOUND {
            return Ok(String::new());
        }
        if e != ERROR_SUCCESS {
            return Err(format!("사용자 PATH를 읽지 못함 (오류 {e})"));
        }
        if kind != REG_SZ && kind != REG_EXPAND_SZ {
            return Err("사용자 PATH가 글자 값이 아님. 손대지 않습니다".into());
        }
        let mut buf = vec![0u16; (size as usize).div_ceil(2) + 1];
        let mut bytes = (buf.len() * 2) as u32;
        let e = unsafe {
            RegQueryValueExW(
                key.0,
                name.as_ptr(),
                null(),
                &mut kind,
                buf.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        if e != ERROR_SUCCESS {
            return Err(format!("사용자 PATH를 읽지 못함 (오류 {e})"));
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16(&buf[..len]).map_err(|_| "사용자 PATH를 읽지 못함".into())
    }

    /// Sets the user's Path (kept expandable) and tells running programs.
    pub fn write(list: &str) -> Result<(), String> {
        let key = open(KEY_SET_VALUE)?;
        let name = wide("Path");
        let value = wide(list);
        let e = unsafe {
            RegSetValueExW(
                key.0,
                name.as_ptr(),
                0,
                REG_EXPAND_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        };
        if e != ERROR_SUCCESS {
            return Err(format!("사용자 PATH를 고치지 못함 (오류 {e})"));
        }
        let env = wide("Environment");
        let mut result = 0usize;
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                env.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                5000,
                &mut result,
            )
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_added_once_and_taken_out_alone() {
        let dir = r"C:\Users\작가\AppData\Local\WriterProgram";
        let list = r"%USERPROFILE%\bin;C:\Tools\";
        let added = with(list, dir).unwrap();
        assert_eq!(added, format!(r"%USERPROFILE%\bin;C:\Tools\;{dir}"));
        assert_eq!(with(&added, &dir.to_uppercase()), None);
        assert_eq!(with(&format!("{added};"), dir), None);
        assert_eq!(without(&added, &format!(r"{dir}\")).unwrap(), list);
        assert_eq!(without(list, dir), None);
        assert_eq!(with("", dir).unwrap(), dir);
        // Entries around it, empty ones included, stay as they were.
        assert_eq!(
            without(&format!(r"a;;{dir};b"), &dir.replace('\\', "/")).unwrap(),
            "a;;b"
        );
    }
}
