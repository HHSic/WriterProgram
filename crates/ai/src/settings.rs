//! This device's AI choices (`ai.json` in the app's settings folder): on or
//! off, which company, which models, and when the writer agreed to send
//! text. The API key is not here; the app keeps it in the system's
//! credential store.

use std::path::Path;

use serde::{Deserialize, Serialize};
use writer_core::store::{atomic_write, now_iso};

use crate::provider::{ALL, clean_model};
use crate::{Error, Provider, Result};

/// The two models used with one company.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Models {
    /// 회차 요약 (cheap tier).
    pub summary: String,
    /// 설정 모순 점검 (middle tier).
    pub check: String,
}

impl Models {
    pub fn defaults(provider: Provider) -> Models {
        Models {
            summary: provider.summary_model().into(),
            check: provider.check_model().into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Off until the writer turns it on and agrees.
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub provider: Provider,
    /// Models per company; a company missing here uses its defaults.
    #[serde(default)]
    pub models: Vec<(Provider, Models)>,
    /// When the writer agreed that text is sent (ISO time).
    #[serde(default)]
    pub agreed: Option<String>,
}

/// A change from the settings screen. `None` leaves a value as it is.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Patch {
    pub enabled: Option<bool>,
    pub provider: Option<Provider>,
    /// Models for one company.
    pub models: Option<(Provider, Models)>,
}

impl Settings {
    pub fn models(&self, provider: Provider) -> Models {
        self.models
            .iter()
            .find(|(p, _)| *p == provider)
            .map(|(_, m)| m.clone())
            .unwrap_or_else(|| Models::defaults(provider))
    }

    /// Every company's models, defaults filled in.
    pub fn all_models(&self) -> Vec<(Provider, Models)> {
        ALL.iter().map(|p| (*p, self.models(*p))).collect()
    }

    /// Applies a change. Turning AI on notes the time of agreement.
    pub fn apply(&mut self, patch: Patch) -> Result<()> {
        if let Some(provider) = patch.provider {
            self.provider = provider;
        }
        if let Some((provider, models)) = patch.models {
            let clean = |m: &str, fallback: &str| {
                if m.trim().is_empty() {
                    Ok(fallback.to_string())
                } else {
                    clean_model(m)
                }
            };
            let models = Models {
                summary: clean(&models.summary, provider.summary_model())?,
                check: clean(&models.check, provider.check_model())?,
            };
            self.models.retain(|(p, _)| *p != provider);
            if models != Models::defaults(provider) {
                self.models.push((provider, models));
            }
        }
        if let Some(enabled) = patch.enabled {
            if enabled && !self.enabled {
                self.agreed = Some(now_iso());
            }
            self.enabled = enabled;
        }
        Ok(())
    }
}

/// Reads the settings at `path`; none yet (or unreadable) means off.
pub fn load(path: &Path) -> Settings {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, settings: &Settings) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| {
            Error::Core(writer_core::Error::Io {
                path: dir.to_path_buf(),
                source: e,
            })
        })?;
    }
    let bytes = serde_json::to_vec_pretty(settings).expect("settings serialize");
    atomic_write(path, &bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_until_turned_on() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai.json");
        let mut s = load(&path);
        assert!(!s.enabled);
        assert_eq!(
            s.models(Provider::Gemini),
            Models::defaults(Provider::Gemini)
        );

        s.apply(Patch {
            enabled: Some(true),
            provider: Some(Provider::Openai),
            models: Some((
                Provider::Openai,
                Models {
                    summary: "".into(),
                    check: " gpt-5 ".into(),
                },
            )),
        })
        .unwrap();
        assert!(s.agreed.is_some());
        save(&path, &s).unwrap();

        let back = load(&path);
        assert!(back.enabled);
        assert_eq!(back.provider, Provider::Openai);
        assert_eq!(back.models(Provider::Openai).summary, "gpt-5-nano");
        assert_eq!(back.models(Provider::Openai).check, "gpt-5");
        // The file holds choices only.
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.to_lowercase().contains("key"));

        let mut bad = back.clone();
        let err = bad.apply(Patch {
            models: Some((
                Provider::Openai,
                Models {
                    summary: "a b".into(),
                    check: "x".into(),
                },
            )),
            ..Patch::default()
        });
        assert!(err.is_err());
    }
}
