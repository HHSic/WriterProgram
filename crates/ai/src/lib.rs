//! AI with the writer's own API key (자기 API 키 연결, BYOK): reading and
//! checking help from Anthropic, OpenAI or Google, called straight from this
//! device. There is no server of ours in between.
//!
//! The AI never writes the manuscript. It summarizes a chapter (회차 요약) and
//! checks a chapter against the setting cards (설정 모순 점검); what comes
//! back is shown to the writer and changes nothing by itself.
//!
//! - `provider`: the three companies, their default models and addresses;
//! - `client`: one plain HTTPS JSON call per company (ureq), with errors in
//!   screen words;
//! - `mask`: card names swapped for stand-ins (`인물A`) before sending and
//!   back again in the answer;
//! - `tasks`: what is sent for each kind of help, and reading the answers;
//! - `settings`: this device's choices (`ai.json`). The key itself is kept in
//!   the system's credential store by the app, never in a file.
//!
//! Nothing here writes the text it sends or receives to disk or to a log.

pub mod client;
pub mod error;
pub mod mask;
pub mod provider;
pub mod settings;
pub mod tasks;

pub use error::{Error, Result};
pub use provider::Provider;
