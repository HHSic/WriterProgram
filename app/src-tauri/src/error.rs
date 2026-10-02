//! What a command answers: its value, or a short reason in screen words.

pub type Res<T> = Result<T, String>;

/// Errors that come with a short reason for the screen.
pub trait Reason {
    fn reason(&self) -> String;
}

impl Reason for writer_core::Error {
    fn reason(&self) -> String {
        self.user_message()
    }
}

impl Reason for writer_sync::Error {
    fn reason(&self) -> String {
        self.user_message()
    }
}

/// The error's reason for the screen (`.map_err(fail)`).
pub fn fail(e: impl Reason) -> String {
    e.reason()
}
