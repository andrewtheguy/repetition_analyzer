#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Failed to open file '{path}': {source}")]
    FileOpen {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to read line {line}: {source}")]
    LineRead { line: usize, source: std::io::Error },

    #[error("{0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Failed to write output: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("line {line}: missing field '{key}'")]
    MissingTextField { line: usize, key: String },

    #[error("line {line}: {message}")]
    LineError { line: usize, message: String },

    #[error("line {line}: duplicate id '{id}'")]
    DuplicateId { line: usize, id: String },

    #[error("{0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
