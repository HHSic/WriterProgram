//! One plain HTTPS JSON call per company (ureq, blocking: the app runs these
//! away from the screen's thread). No SDKs: each company's request and answer
//! are a few fields, built and read here so they can be tested against a
//! local stand-in.
//!
//! - Anthropic: `POST /v1/messages` with `x-api-key` and `anthropic-version`;
//! - OpenAI: `POST /v1/chat/completions` with `Authorization: Bearer`;
//! - Gemini: `POST /v1beta/models/{model}:generateContent` with `x-goog-api-key`.
//!
//! "연결 확인" reads the two chosen models (`GET …/models/{model}`), which
//! costs nothing and tells a wrong key from a wrong model name.

use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};

use crate::{Error, Provider, Result};

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// One question: instructions, the text, and how long the answer may be.
#[derive(Debug, Clone)]
pub struct Ask {
    pub system: String,
    pub user: String,
    pub max_tokens: u32,
    /// Ask for a JSON object back.
    pub json: bool,
}

/// What the company says it used, when it says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub text: String,
    pub usage: Option<Usage>,
    /// The answer stopped at the length limit.
    pub cut: bool,
}

/// A connection to one company with the writer's key.
pub struct Client {
    agent: ureq::Agent,
    provider: Provider,
    base: String,
    key: String,
}

impl Client {
    /// `base`: the company's address (see [`Provider::base`]).
    pub fn new(provider: Provider, key: &str, base: String) -> Client {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(15)))
            // A long chapter on a mid-tier model can take a while.
            .timeout_global(Some(Duration::from_secs(180)))
            .http_status_as_error(false)
            .user_agent("WriterProgram")
            .build()
            .into();
        Client {
            agent,
            provider,
            base: base.trim_end_matches('/').to_string(),
            key: key.to_string(),
        }
    }

    fn headers(&self) -> Vec<(&'static str, String)> {
        match self.provider {
            Provider::Anthropic => vec![
                ("x-api-key", self.key.clone()),
                ("anthropic-version", ANTHROPIC_VERSION.to_string()),
            ],
            Provider::Openai => vec![("authorization", format!("Bearer {}", self.key))],
            Provider::Gemini => vec![("x-goog-api-key", self.key.clone())],
        }
    }

    /// Asks `model` one question.
    pub fn ask(&self, model: &str, ask: &Ask) -> Result<Answer> {
        let url = ask_url(self.provider, &self.base, model);
        let body = request_body(self.provider, model, ask);
        let mut req = self.agent.post(&url);
        for (k, v) in self.headers() {
            req = req.header(k, v.as_str());
        }
        let bytes = serde_json::to_vec(&body).expect("json serializes");
        let response = req.content_type("application/json").send(bytes.as_slice());
        let (status, body) = read(response)?;
        if !(200..300).contains(&status) {
            return Err(classify(self.provider, status, &body, model));
        }
        parse_answer(self.provider, &body)
    }

    /// Checks that the key works and that `model` exists (free of charge).
    pub fn check_model(&self, model: &str) -> Result<()> {
        let url = model_url(self.provider, &self.base, model);
        let mut req = self.agent.get(&url);
        for (k, v) in self.headers() {
            req = req.header(k, v.as_str());
        }
        let (status, body) = read(req.call())?;
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(classify(self.provider, status, &body, model))
        }
    }
}

fn read(
    response: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<(u16, Vec<u8>)> {
    let mut response = response.map_err(transport)?;
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .with_config()
        .limit(16 * 1024 * 1024)
        .read_to_vec()
        .map_err(transport)?;
    Ok((status, body))
}

fn transport(e: ureq::Error) -> Error {
    match e {
        ureq::Error::Timeout(_) => Error::Timeout,
        other => Error::Offline(other.to_string()),
    }
}

/// Where a question goes.
pub fn ask_url(provider: Provider, base: &str, model: &str) -> String {
    match provider {
        Provider::Anthropic => format!("{base}/v1/messages"),
        Provider::Openai => format!("{base}/v1/chat/completions"),
        Provider::Gemini => format!("{base}/v1beta/models/{model}:generateContent"),
    }
}

/// Where a model's description is read ("연결 확인").
pub fn model_url(provider: Provider, base: &str, model: &str) -> String {
    match provider {
        Provider::Anthropic | Provider::Openai => format!("{base}/v1/models/{model}"),
        Provider::Gemini => format!("{base}/v1beta/models/{model}"),
    }
}

/// The JSON body of a question. No sampling settings: newer models refuse
/// them, and the defaults suit reading and checking.
pub fn request_body(provider: Provider, model: &str, ask: &Ask) -> Value {
    match provider {
        Provider::Anthropic => json!({
            "model": model,
            "max_tokens": ask.max_tokens,
            "system": ask.system,
            "messages": [{ "role": "user", "content": ask.user }],
        }),
        Provider::Openai => {
            let mut body = json!({
                "model": model,
                "max_completion_tokens": ask.max_tokens,
                "messages": [
                    { "role": "system", "content": ask.system },
                    { "role": "user", "content": ask.user },
                ],
            });
            if ask.json {
                body["response_format"] = json!({ "type": "json_object" });
            }
            body
        }
        Provider::Gemini => {
            let mut config = json!({ "maxOutputTokens": ask.max_tokens });
            if ask.json {
                config["responseMimeType"] = json!("application/json");
            }
            json!({
                "systemInstruction": { "parts": [{ "text": ask.system }] },
                "contents": [{ "role": "user", "parts": [{ "text": ask.user }] }],
                "generationConfig": config,
            })
        }
    }
}

fn unreadable(e: impl std::fmt::Display) -> Error {
    Error::Unreadable(e.to_string())
}

fn count(v: &Value) -> Option<u64> {
    v.as_u64()
}

/// Reads a successful answer.
pub fn parse_answer(provider: Provider, body: impl AsRef<[u8]>) -> Result<Answer> {
    let v: Value = serde_json::from_slice(body.as_ref()).map_err(unreadable)?;
    match provider {
        Provider::Anthropic => {
            let stop = v["stop_reason"].as_str().unwrap_or_default();
            if stop == "refusal" {
                return Err(Error::Refused);
            }
            let blocks = v["content"]
                .as_array()
                .ok_or_else(|| unreadable("no content"))?;
            // Thinking blocks (newer models think first) are skipped.
            let text: String = blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect();
            let usage = v.get("usage").map(|u| Usage {
                input_tokens: count(&u["input_tokens"]),
                output_tokens: count(&u["output_tokens"]),
            });
            Ok(Answer {
                text,
                usage,
                cut: stop == "max_tokens",
            })
        }
        Provider::Openai => {
            let choice = &v["choices"][0];
            if choice.is_null() {
                return Err(unreadable("no choices"));
            }
            let message = &choice["message"];
            let finish = choice["finish_reason"].as_str().unwrap_or_default();
            if message["refusal"].as_str().is_some_and(|r| !r.is_empty())
                || finish == "content_filter"
            {
                return Err(Error::Refused);
            }
            let text = message["content"].as_str().unwrap_or_default().to_string();
            let usage = v.get("usage").map(|u| Usage {
                input_tokens: count(&u["prompt_tokens"]),
                output_tokens: count(&u["completion_tokens"]),
            });
            Ok(Answer {
                text,
                usage,
                cut: finish == "length",
            })
        }
        Provider::Gemini => {
            let candidate = &v["candidates"][0];
            if candidate.is_null() {
                if v["promptFeedback"]["blockReason"].is_string() {
                    return Err(Error::Refused);
                }
                return Err(unreadable("no candidates"));
            }
            let finish = candidate["finishReason"].as_str().unwrap_or_default();
            // Thought summaries, if any, are skipped.
            let text: String = candidate["content"]["parts"]
                .as_array()
                .map(|parts| {
                    parts
                        .iter()
                        .filter(|p| p["thought"] != true)
                        .filter_map(|p| p["text"].as_str())
                        .collect()
                })
                .unwrap_or_default();
            if text.is_empty()
                && matches!(
                    finish,
                    "SAFETY" | "PROHIBITED_CONTENT" | "BLOCKLIST" | "SPII" | "RECITATION"
                )
            {
                return Err(Error::Refused);
            }
            let usage = v.get("usageMetadata").map(|u| Usage {
                input_tokens: count(&u["promptTokenCount"]),
                // What is billed as output includes the model's thinking.
                output_tokens: match (
                    count(&u["candidatesTokenCount"]),
                    count(&u["thoughtsTokenCount"]),
                ) {
                    (None, None) => None,
                    (a, b) => Some(a.unwrap_or(0) + b.unwrap_or(0)),
                },
            });
            Ok(Answer {
                text,
                usage,
                cut: finish == "MAX_TOKENS",
            })
        }
    }
}

/// The company's own error words, shortened (they never carry the text sent).
fn provider_message(v: &Value) -> String {
    let message = v["error"]["message"]
        .as_str()
        .or_else(|| v["message"].as_str())
        .unwrap_or_default();
    message.chars().take(200).collect()
}

/// Turns an error answer into a reason the writer can act on.
pub fn classify(provider: Provider, status: u16, body: &[u8], model: &str) -> Error {
    let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let message = provider_message(&v);
    let lower = message.to_lowercase();
    match provider {
        Provider::Anthropic => {
            let kind = v["error"]["type"].as_str().unwrap_or_default();
            match (status, kind) {
                (401, _) | (_, "authentication_error") => Error::KeyRejected,
                (402, _) | (_, "billing_error") => Error::Quota,
                (400, _) if lower.contains("credit balance") => Error::Quota,
                (404, _) | (_, "not_found_error") => Error::ModelNotFound(model.into()),
                (429, _) | (_, "rate_limit_error") => Error::RateLimited,
                (529, _) | (_, "overloaded_error") => Error::Busy,
                (s, _) if s >= 500 => Error::Busy,
                _ => Error::Provider { status, message },
            }
        }
        Provider::Openai => {
            let code = v["error"]["code"].as_str().unwrap_or_default();
            match status {
                401 => Error::KeyRejected,
                429 if code == "insufficient_quota" => Error::Quota,
                429 => Error::RateLimited,
                404 if code == "model_not_found" || lower.contains("model") => {
                    Error::ModelNotFound(model.into())
                }
                s if s >= 500 => Error::Busy,
                _ => Error::Provider { status, message },
            }
        }
        Provider::Gemini => {
            let state = v["error"]["status"].as_str().unwrap_or_default();
            let bad_key = lower.contains("api key not valid")
                || lower.contains("api_key_invalid")
                || v["error"]["details"]
                    .as_array()
                    .is_some_and(|d| d.iter().any(|x| x["reason"] == "API_KEY_INVALID"));
            match status {
                _ if bad_key => Error::KeyRejected,
                401 | 403 => Error::KeyRejected,
                404 => Error::ModelNotFound(model.into()),
                429 if state == "RESOURCE_EXHAUSTED" && lower.contains("billing") => Error::Quota,
                429 => Error::RateLimited,
                s if s >= 500 => Error::Busy,
                _ => Error::Provider { status, message },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(json: bool) -> Ask {
        Ask {
            system: "지시".into(),
            user: "인물A가 웃었다.".into(),
            max_tokens: 500,
            json,
        }
    }

    #[test]
    fn anthropic_body() {
        let body = request_body(Provider::Anthropic, "claude-haiku-4-5", &ask(false));
        assert_eq!(
            body,
            json!({
                "model": "claude-haiku-4-5",
                "max_tokens": 500,
                "system": "지시",
                "messages": [{ "role": "user", "content": "인물A가 웃었다." }],
            })
        );
    }

    #[test]
    fn openai_body_asks_for_json_when_needed() {
        let plain = request_body(Provider::Openai, "gpt-5-mini", &ask(false));
        assert!(plain.get("response_format").is_none());
        assert_eq!(plain["messages"][0]["role"], "system");
        assert_eq!(plain["max_completion_tokens"], 500);
        let json = request_body(Provider::Openai, "gpt-5-mini", &ask(true));
        assert_eq!(json["response_format"]["type"], "json_object");
    }

    #[test]
    fn gemini_body() {
        let body = request_body(Provider::Gemini, "gemini-2.5-flash", &ask(true));
        assert_eq!(body["systemInstruction"]["parts"][0]["text"], "지시");
        assert_eq!(body["contents"][0]["parts"][0]["text"], "인물A가 웃었다.");
        assert_eq!(
            body["generationConfig"]["responseMimeType"],
            "application/json"
        );
        assert_eq!(
            ask_url(Provider::Gemini, "https://g", "gemini-2.5-flash"),
            "https://g/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn reads_answers_and_usage() {
        let a = parse_answer(
            Provider::Anthropic,
            r#"{"content":[{"type":"thinking","thinking":""},{"type":"text","text":"요약."}],
                "stop_reason":"end_turn","usage":{"input_tokens":10,"output_tokens":3}}"#,
        )
        .unwrap();
        assert_eq!(a.text, "요약.");
        assert_eq!(a.usage.unwrap().output_tokens, Some(3));
        assert!(!a.cut);

        let o = parse_answer(
            Provider::Openai,
            r#"{"choices":[{"message":{"content":"요약.","refusal":null},"finish_reason":"length"}],
                "usage":{"prompt_tokens":7,"completion_tokens":2}}"#,
        )
        .unwrap();
        assert_eq!(o.text, "요약.");
        assert_eq!(o.usage.unwrap().input_tokens, Some(7));
        assert!(o.cut);

        let g = parse_answer(
            Provider::Gemini,
            r#"{"candidates":[{"content":{"parts":[{"text":"생각","thought":true},{"text":"요약."}]},"finishReason":"STOP"}],
                "usageMetadata":{"promptTokenCount":5,"candidatesTokenCount":2,"thoughtsTokenCount":4}}"#,
        )
        .unwrap();
        assert_eq!(g.text, "요약.");
        assert_eq!(g.usage.unwrap().output_tokens, Some(6));
    }

    #[test]
    fn refusals_are_told_apart() {
        assert!(matches!(
            parse_answer(
                Provider::Anthropic,
                r#"{"content":[],"stop_reason":"refusal"}"#
            ),
            Err(Error::Refused)
        ));
        assert!(matches!(
            parse_answer(
                Provider::Openai,
                r#"{"choices":[{"message":{"content":null,"refusal":"no"},"finish_reason":"stop"}]}"#
            ),
            Err(Error::Refused)
        ));
        assert!(matches!(
            parse_answer(
                Provider::Gemini,
                r#"{"promptFeedback":{"blockReason":"SAFETY"}}"#
            ),
            Err(Error::Refused)
        ));
    }

    #[test]
    fn errors_become_reasons() {
        let c = |p, s, b: &str| classify(p, s, b.as_bytes(), "m");
        assert!(matches!(
            c(
                Provider::Anthropic,
                401,
                r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#
            ),
            Error::KeyRejected
        ));
        assert!(matches!(
            c(
                Provider::Anthropic,
                400,
                r#"{"type":"error","error":{"type":"invalid_request_error","message":"Your credit balance is too low"}}"#
            ),
            Error::Quota
        ));
        assert!(matches!(
            c(
                Provider::Anthropic,
                529,
                r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#
            ),
            Error::Busy
        ));
        assert!(matches!(
            c(
                Provider::Openai,
                429,
                r#"{"error":{"message":"quota","code":"insufficient_quota"}}"#
            ),
            Error::Quota
        ));
        assert!(matches!(
            c(
                Provider::Openai,
                429,
                r#"{"error":{"message":"slow down","code":"rate_limit_exceeded"}}"#
            ),
            Error::RateLimited
        ));
        assert!(matches!(
            c(
                Provider::Openai,
                404,
                r#"{"error":{"message":"The model `x` does not exist","code":"model_not_found"}}"#
            ),
            Error::ModelNotFound(_)
        ));
        assert!(matches!(
            c(
                Provider::Gemini,
                400,
                r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT"}}"#
            ),
            Error::KeyRejected
        ));
        assert!(matches!(
            c(
                Provider::Gemini,
                429,
                r#"{"error":{"code":429,"message":"Resource exhausted","status":"RESOURCE_EXHAUSTED"}}"#
            ),
            Error::RateLimited
        ));
        let other = c(
            Provider::Openai,
            400,
            r#"{"error":{"message":"bad thing"}}"#,
        );
        assert_eq!(
            other.user_message(),
            "AI 회사가 요청을 받지 않음 (400: bad thing)"
        );
    }
}
