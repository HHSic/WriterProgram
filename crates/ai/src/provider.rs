//! The AI companies the writer can connect with their own key, the models
//! offered first, and where each one is reached.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Anthropic (Claude): Messages API.
    #[default]
    Anthropic,
    /// OpenAI: Chat Completions.
    Openai,
    /// Google Gemini API: generateContent.
    Gemini,
}

pub const ALL: [Provider; 3] = [Provider::Anthropic, Provider::Openai, Provider::Gemini];

impl Provider {
    /// Short key for files, the credential store and environment variables.
    pub fn key(self) -> &'static str {
        match self {
            Provider::Anthropic => "anthropic",
            Provider::Openai => "openai",
            Provider::Gemini => "gemini",
        }
    }

    /// Name for the screen.
    pub fn label(self) -> &'static str {
        match self {
            Provider::Anthropic => "Anthropic (Claude)",
            Provider::Openai => "OpenAI",
            Provider::Gemini => "Google Gemini",
        }
    }

    /// Model for summaries: the cheap tier. Names change; the writer can type
    /// another one in settings.
    pub fn summary_model(self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-haiku-4-5",
            Provider::Openai => "gpt-5-nano",
            Provider::Gemini => "gemini-2.5-flash-lite",
        }
    }

    /// Model for consistency checks: the middle tier.
    pub fn check_model(self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-sonnet-5",
            Provider::Openai => "gpt-5-mini",
            Provider::Gemini => "gemini-2.5-flash",
        }
    }

    fn default_base(self) -> &'static str {
        match self {
            Provider::Anthropic => "https://api.anthropic.com",
            Provider::Openai => "https://api.openai.com",
            Provider::Gemini => "https://generativelanguage.googleapis.com",
        }
    }

    /// Where the company's API is reached. The environment variable
    /// `WRITER_AI_<COMPANY>_ENDPOINT` (e.g. `WRITER_AI_ANTHROPIC_ENDPOINT`)
    /// points it somewhere else, for trying the app against a stand-in.
    pub fn base(self) -> String {
        self.base_with(|name| std::env::var(name).ok())
    }

    /// [`Provider::base`] with the environment given (tests).
    pub fn base_with(self, env: impl Fn(&str) -> Option<String>) -> String {
        let var = format!("WRITER_AI_{}_ENDPOINT", self.key().to_uppercase());
        env(&var)
            .map(|v| v.trim().trim_end_matches('/').to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| self.default_base().to_string())
    }
}

/// Checks a model name typed by the writer: letters, digits and `-._:`
/// only, since it goes into the address. A leading `models/` (as Google
/// writes them) is dropped.
pub fn clean_model(model: &str) -> Result<String> {
    let model = model.trim();
    let model = model.strip_prefix("models/").unwrap_or(model);
    if model.is_empty() {
        return Err(Error::Invalid("모델 이름을 적어 주세요".into()));
    }
    if model.len() > 100
        || !model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | ':'))
    {
        return Err(Error::Invalid(
            "모델 이름에는 영문, 숫자, '-', '.', '_'만 쓸 수 있음".into(),
        ));
    }
    Ok(model.to_string())
}

/// Checks a key typed or pasted by the writer (no spaces inside).
pub fn clean_key(key: &str) -> Result<String> {
    let key = key.trim();
    if key.is_empty() {
        return Err(Error::Invalid("API 키를 붙여 넣어 주세요".into()));
    }
    if key.len() > 400 || !key.chars().all(|c| c.is_ascii_graphic()) {
        return Err(Error::Invalid(
            "API 키 모양이 아님. AI 회사 누리집에서 키를 다시 복사해 주세요.".into(),
        ));
    }
    Ok(key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_can_be_moved_for_tests() {
        let none = |_: &str| None;
        assert_eq!(
            Provider::Anthropic.base_with(none),
            "https://api.anthropic.com"
        );
        let env = |name: &str| {
            (name == "WRITER_AI_GEMINI_ENDPOINT").then(|| "http://127.0.0.1:9/ ".to_string())
        };
        assert_eq!(Provider::Gemini.base_with(env), "http://127.0.0.1:9");
        assert_eq!(Provider::Openai.base_with(env), "https://api.openai.com");
    }

    #[test]
    fn model_names_are_checked() {
        assert_eq!(
            clean_model(" models/gemini-2.5-flash ").unwrap(),
            "gemini-2.5-flash"
        );
        assert!(clean_model("").is_err());
        assert!(clean_model("gpt 5").is_err());
        assert!(clean_model("../v1/x").is_err());
    }

    #[test]
    fn keys_are_checked() {
        assert_eq!(clean_key("  sk-abc\n").unwrap(), "sk-abc");
        assert!(clean_key("sk abc").is_err());
        assert!(clean_key("키").is_err());
    }
}
