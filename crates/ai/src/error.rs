//! What can go wrong when asking an AI service, in the writer's words.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] writer_core::Error),
    /// AI is turned off on this device.
    #[error("AI 연결이 꺼져 있음")]
    Off,
    /// No key was put in for the chosen company.
    #[error("API 키가 없음")]
    NoKey,
    /// The company refused the key.
    #[error("API 키가 맞지 않음")]
    KeyRejected,
    /// The account has no credit left or reached its spending limit.
    #[error("사용 한도 또는 잔액 부족")]
    Quota,
    /// Too many requests in a short time.
    #[error("요청이 너무 잦음")]
    RateLimited,
    /// The company's servers are busy or down.
    #[error("AI 회사 서버가 바쁨")]
    Busy,
    /// The model name is not known to the company.
    #[error("모델을 찾지 못함: {0}")]
    ModelNotFound(String),
    /// The company's safety rules stopped the answer.
    #[error("AI가 답하지 않음")]
    Refused,
    /// No answer in time.
    #[error("시간 초과")]
    Timeout,
    /// No connection (offline, blocked, or the address is wrong).
    #[error("AI 회사에 닿지 않음 ({0})")]
    Offline(String),
    /// The company answered with another error; `message` is its own words.
    #[error("{status}: {message}")]
    Provider { status: u16, message: String },
    /// The answer was not in the expected shape.
    #[error("대답을 읽지 못함 ({0})")]
    Unreadable(String),
    /// Something the writer can fix; the message is for the screen.
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Short reason for the screen, like `writer_core::Error::user_message`.
    pub fn user_message(&self) -> String {
        match self {
            Error::Core(e) => e.user_message(),
            Error::Off => "AI 연결이 꺼져 있음. 'AI 연결 (내 API 키)'에서 켜 주세요.".into(),
            Error::NoKey => "API 키가 없음. 'AI 연결 (내 API 키)'에서 키를 넣어 주세요.".into(),
            Error::KeyRejected => {
                "API 키가 맞지 않음. 키를 다시 복사해 넣었는지, 고른 AI 회사의 키인지 확인해 주세요."
                    .into()
            }
            Error::Quota => {
                "AI 회사 계정의 잔액이나 사용 한도가 부족함. AI 회사 누리집에서 결제·한도를 확인해 주세요."
                    .into()
            }
            Error::RateLimited => "요청이 너무 잦아 AI 회사가 잠시 막음. 1분쯤 뒤에 다시 해 주세요.".into(),
            Error::Busy => "AI 회사 서버가 바쁘거나 멈춤. 잠시 뒤에 다시 해 주세요.".into(),
            Error::ModelNotFound(model) => format!(
                "모델 이름 '{model}'을(를) AI 회사가 모름. 'AI 연결 (내 API 키)'에서 모델 이름을 확인해 주세요."
            ),
            Error::Refused => "AI 회사의 안전 기준 때문에 AI가 이 내용에 답하지 않음.".into(),
            Error::Timeout => "AI 회사가 제때 답하지 않음. 잠시 뒤에 다시 해 주세요.".into(),
            Error::Offline(_) => "AI 회사에 닿지 않음. 인터넷 연결을 확인해 주세요.".into(),
            Error::Provider { status, message } => {
                if message.is_empty() {
                    format!("AI 회사가 요청을 받지 않음 ({status})")
                } else {
                    format!("AI 회사가 요청을 받지 않음 ({status}: {message})")
                }
            }
            Error::Unreadable(_) => {
                "AI의 대답을 읽지 못함. 다시 해 보거나 점검용 모델을 바꿔 보세요.".into()
            }
            Error::Invalid(m) => m.clone(),
        }
    }
}
