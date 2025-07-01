use thiserror::Error;

#[derive(Error, Debug)]
pub enum VmCliError {
    #[error("Ошибка HTTP запроса: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Ошибка парсинга JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Ошибка конфигурации: {0}")]
    ConfigError(#[from] config::ConfigError),

    #[error("Ошибка ввода-вывода: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Ошибка CSV: {0}")]
    CsvError(#[from] csv::Error),

    #[error("VictoriaMetrics API ошибка: {message}")]
    ApiError { message: String, status: Option<u16> },

    #[error("Неверный формат времени: {0}")]
    TimeParseError(String),

    #[error("Неверный запрос: {0}")]
    InvalidQuery(String),

    #[error("Файл не найден: {0}")]
    FileNotFound(String),

    #[error("Недостаточно прав для выполнения операции")]
    PermissionDenied,

    #[error("Таймаут операции")]
    Timeout,

    #[error("Неизвестная ошибка: {0}")]
    Unknown(String),
}

impl From<anyhow::Error> for VmCliError {
    fn from(err: anyhow::Error) -> Self {
        VmCliError::Unknown(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, VmCliError>; 