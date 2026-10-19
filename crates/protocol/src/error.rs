#[derive(Debug)]
pub enum SchemaError {
    Json(serde_json::Error),
    Missing(&'static str),
    Unsupported(String),
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "json: {error}"),
            Self::Missing(part) => write!(formatter, "o esquema não tem {part}"),
            Self::Unsupported(detail) => write!(formatter, "esquema não suportado: {detail}"),
        }
    }
}

impl std::error::Error for SchemaError {}

impl From<serde_json::Error> for SchemaError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Debug)]
pub enum WorkerError {
    Io(std::io::Error),
    Http(Box<reqwest::Error>),
    Socket(Box<tungstenite::Error>),
    Protocol(String),
    Unauthorized,
    Timeout,
    Startup(String),
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "io: {error}"),
            Self::Http(error) => write!(formatter, "http: {error}"),
            Self::Socket(error) => write!(formatter, "websocket: {error}"),
            Self::Protocol(detail) => write!(formatter, "protocolo: {detail}"),
            Self::Unauthorized => write!(formatter, "token recusado"),
            Self::Timeout => write!(formatter, "o worker não ficou pronto a tempo"),
            Self::Startup(detail) => write!(formatter, "falha ao iniciar o worker: {detail}"),
        }
    }
}

impl std::error::Error for WorkerError {}

impl From<std::io::Error> for WorkerError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<reqwest::Error> for WorkerError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(Box::new(error))
    }
}

impl From<tungstenite::Error> for WorkerError {
    fn from(error: tungstenite::Error) -> Self {
        Self::Socket(Box::new(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mensagens_cobrem_cada_falha() {
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(200))
            .build()
            .expect("cliente")
            .get("http://127.0.0.1:1/")
            .send()
            .expect_err("porta fechada");
        let errors = [
            WorkerError::from(std::io::Error::other("disco")),
            WorkerError::from(http),
            WorkerError::from(tungstenite::Error::ConnectionClosed),
            WorkerError::Protocol("campo ausente".to_owned()),
            WorkerError::Unauthorized,
            WorkerError::Timeout,
            WorkerError::Startup("sem porta".to_owned()),
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
        }
        let schemas = [
            SchemaError::Missing("paths"),
            SchemaError::Unsupported("array".to_owned()),
            SchemaError::from(serde_json::from_str::<serde_json::Value>("{").expect_err("json")),
        ];
        for error in schemas {
            assert!(!error.to_string().is_empty());
        }
    }
}
