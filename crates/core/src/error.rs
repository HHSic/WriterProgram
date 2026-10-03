use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{}: {message}", path.display())]
    Format { path: PathBuf, message: String },
    /// Something the user asked for does not exist. The message is already
    /// written for the screen, e.g. "회차를 찾을 수 없음".
    #[error("{0}")]
    NotFound(String),
    /// A request that cannot be carried out. The message is written for the screen.
    #[error("{0}")]
    Invalid(String),
    /// `project.json` is there but is not a project any more (cut short,
    /// empty, other JSON). The details are for logs; the screen offers
    /// recovery (project/recover.rs, docs/safety-design.md S5).
    #[error("{}: damaged project file: {detail}", path.display())]
    ProjectDamaged { path: PathBuf, detail: String },
}

/// What the screen says when `project.json` is damaged.
pub const PROJECT_DAMAGED: &str =
    "작품 구조 파일(project.json)이 손상되어 열 수 없습니다. 원고 파일은 그대로 있습니다.";

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub(crate) fn io(path: &Path, source: io::Error) -> Self {
        Error::Io {
            path: path.to_path_buf(),
            source,
        }
    }

    pub(crate) fn format(path: &Path, message: impl Into<String>) -> Self {
        Error::Format {
            path: path.to_path_buf(),
            message: message.into(),
        }
    }

    /// Short reason for the screen, in the words of docs/terminology.md.
    /// The UI puts it after its own lead-in, e.g. "저장하지 못함 · 디스크 공간 부족".
    pub fn user_message(&self) -> String {
        match self {
            Error::Io { source, .. } if is_held(source) => {
                "파일이 다른 프로그램에 잡혀 있음".into()
            }
            Error::Io { source, .. } => match source.kind() {
                io::ErrorKind::StorageFull => "디스크 공간 부족".into(),
                io::ErrorKind::PermissionDenied => "이 위치에 쓸 권한이 없음".into(),
                io::ErrorKind::ReadOnlyFilesystem => "읽기 전용 위치".into(),
                io::ErrorKind::NotFound => "파일을 찾을 수 없음".into(),
                _ => format!("파일 오류 ({source})"),
            },
            Error::Format { path, message } => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                format!("파일을 읽을 수 없음 · {name} ({message})")
            }
            Error::NotFound(message) | Error::Invalid(message) => message.clone(),
            Error::ProjectDamaged { .. } => PROJECT_DAMAGED.into(),
        }
    }
}

/// Another program (a sync client, a virus scanner, a word processor) has the
/// file open and will not share it: ERROR_SHARING_VIOLATION (32) and
/// ERROR_LOCK_VIOLATION (33) on Windows.
fn is_held(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::ResourceBusy
        || (cfg!(windows) && matches!(e.raw_os_error(), Some(32 | 33)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(e: io::Error) -> String {
        Error::io(Path::new("a.md"), e).user_message()
    }

    #[test]
    fn says_why_a_file_could_not_be_written() {
        assert_eq!(
            message(io::ErrorKind::StorageFull.into()),
            "디스크 공간 부족"
        );
        assert_eq!(
            message(io::ErrorKind::PermissionDenied.into()),
            "이 위치에 쓸 권한이 없음"
        );
        assert_eq!(
            message(io::ErrorKind::ResourceBusy.into()),
            "파일이 다른 프로그램에 잡혀 있음"
        );
        #[cfg(windows)]
        assert_eq!(
            message(io::Error::from_raw_os_error(32)),
            "파일이 다른 프로그램에 잡혀 있음"
        );
    }
}
