use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] writer_core::Error),
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// The sign-in expired or was withdrawn on the drive's side.
    #[error("드라이브 연결이 끊김")]
    SignedOut,
    /// No connection to the drive (offline, or the drive is down).
    #[error("드라이브에 닿지 않음 ({0})")]
    Offline(String),
    /// The drive answered with an error. The message is for the screen.
    #[error("{0}")]
    Drive(String),
    /// Something the app cannot do; the message is for the screen.
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

    /// Short reason for the screen, like `writer_core::Error::user_message`.
    pub fn user_message(&self) -> String {
        match self {
            Error::Core(e) => e.user_message(),
            Error::Io { source, .. } => writer_core::Error::Io {
                path: PathBuf::new(),
                source: io::Error::new(source.kind(), source.to_string()),
            }
            .user_message(),
            Error::SignedOut => "드라이브 연결이 끊김. 다시 연결해 주세요.".into(),
            Error::Offline(_) => "드라이브에 닿지 않음. 인터넷 연결을 확인해 주세요.".into(),
            Error::Drive(m) | Error::Invalid(m) => m.clone(),
        }
    }
}
