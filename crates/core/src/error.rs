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
}

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
        }
    }
}
