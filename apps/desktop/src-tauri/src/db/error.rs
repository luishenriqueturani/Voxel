#[derive(Debug)]
pub enum DbError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    LockPoisoned,
    Gap { expected: i64, found: i64 },
    NewerThanApp { version: i64 },
}

impl std::fmt::Display for DbError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "sqlite: {error}"),
            Self::Io(error) => write!(formatter, "io: {error}"),
            Self::LockPoisoned => write!(formatter, "o banco está bloqueado"),
            Self::Gap { expected, found } => {
                write!(
                    formatter,
                    "migração {found} fora de ordem; esperada a {expected}"
                )
            }
            Self::NewerThanApp { version } => {
                write!(
                    formatter,
                    "o banco está na versão {version}, mais nova que este app"
                )
            }
        }
    }
}

impl std::error::Error for DbError {}

impl From<rusqlite::Error> for DbError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<std::io::Error> for DbError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
