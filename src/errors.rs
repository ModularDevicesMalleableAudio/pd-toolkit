use pd_toolkit::model::ParseError;
use thiserror::Error;

/// Exit codes per §9 of the plan:
/// 0 = success
/// 1 = validation/lint errors
/// 2 = parse/CLI usage errors
/// 3 = IO errors
#[derive(Debug, Error)]
pub enum PdtkError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Usage(String),
}

impl PdtkError {
    pub fn exit_code(&self) -> i32 {
        match self {
            PdtkError::Io(_) => 3,
            PdtkError::Parse(_) | PdtkError::Json(_) | PdtkError::Usage(_) => 2,
        }
    }
}
