use std::path::Path;
use std::sync::Mutex;

use time::macros::format_description;
use time::{Date, OffsetDateTime};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;

/// Dias de log geral que permanecem em disco, contando o dia de hoje.
pub const DEFAULT_RETENTION_DAYS: u32 = 14;

const FILE_PREFIX: &str = "voxel.";
const FILE_SUFFIX: &str = ".log";
const DATE_FORMAT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");

#[derive(Debug)]
pub enum LogError {
    Io(std::io::Error),
    Appender(tracing_appender::rolling::InitError),
    AlreadyInstalled,
}

impl std::fmt::Display for LogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "io: {error}"),
            Self::Appender(error) => write!(formatter, "log: {error}"),
            Self::AlreadyInstalled => write!(formatter, "o log do aplicativo já foi iniciado"),
        }
    }
}

impl std::error::Error for LogError {}

impl From<std::io::Error> for LogError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<tracing_appender::rolling::InitError> for LogError {
    fn from(error: tracing_appender::rolling::InitError) -> Self {
        Self::Appender(error)
    }
}

/// Mantém o worker do `tracing` vivo. Ao ser descartado, descarrega o que ainda está na fila.
#[derive(Debug)]
pub struct AppLog {
    guard: Mutex<Option<WorkerGuard>>,
}

impl Drop for AppLog {
    fn drop(&mut self) {
        if let Ok(mut slot) = self.guard.lock() {
            drop(slot.take());
        }
    }
}

impl AppLog {
    /// Começa o log diário em `directory` e apaga arquivos mais velhos que `retention_days`.
    ///
    /// Os arquivos se chamam `voxel.AAAA-MM-DD.log`, em UTC. O nível gravado é `info`.
    ///
    /// # Errors
    ///
    /// Falha se o diretório não puder ser criado, se o arquivo não puder ser aberto,
    /// ou se o log global já tiver sido instalado neste processo.
    pub fn start(directory: &Path, retention_days: u32) -> Result<Self, LogError> {
        std::fs::create_dir_all(directory)?;
        prune_old_logs(directory, OffsetDateTime::now_utc().date(), retention_days)?;
        let appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("voxel")
            .filename_suffix("log")
            .build(directory)?;
        let (writer, guard) = tracing_appender::non_blocking(appender);
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_env_filter(EnvFilter::new("info"))
            .with_writer(writer)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .map_err(|_| LogError::AlreadyInstalled)?;
        tracing::info!("log do aplicativo iniciado");
        Ok(Self {
            guard: Mutex::new(Some(guard)),
        })
    }
}

/// Remove logs gerais com `retention_days` dias ou mais. Logs de erro e outros arquivos ficam.
///
/// # Errors
///
/// Falha se `directory` existir e não puder ser lido, ou se um arquivo velho não puder ser apagado.
pub fn prune_old_logs(directory: &Path, today: Date, retention_days: u32) -> std::io::Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    let retention = i64::from(retention_days);
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().into_string().ok() else {
            continue;
        };
        let Some(date) = app_log_date(&name) else {
            continue;
        };
        let age = (today - date).whole_days();
        if age >= retention {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn app_log_date(name: &str) -> Option<Date> {
    let rest = name.strip_prefix(FILE_PREFIX)?.strip_suffix(FILE_SUFFIX)?;
    Date::parse(rest, DATE_FORMAT).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;
    use time::macros::date;

    fn directory(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("voxel-applog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn apaga_so_o_log_geral_velho() {
        let directory = directory("prune");
        std::fs::create_dir_all(&directory).unwrap();
        let today = date!(2026 - 10 - 19);
        let expired = today.checked_sub(Duration::days(14)).unwrap();
        let fresh = today.checked_sub(Duration::days(13)).unwrap();
        std::fs::write(directory.join(format!("voxel.{expired}.log")), "velho").unwrap();
        std::fs::write(directory.join(format!("voxel.{fresh}.log")), "recente").unwrap();
        std::fs::write(directory.join("erro_2020-01-01_job.log"), "erro").unwrap();
        std::fs::write(directory.join("notas.txt"), "x").unwrap();
        std::fs::write(directory.join("voxel.not-a-date.log"), "x").unwrap();
        std::fs::create_dir(directory.join("voxel.2000-01-01.log")).unwrap();

        prune_old_logs(&directory, today, DEFAULT_RETENTION_DAYS).unwrap();

        assert!(!directory.join(format!("voxel.{expired}.log")).exists());
        assert!(directory.join(format!("voxel.{fresh}.log")).exists());
        assert!(directory.join("erro_2020-01-01_job.log").exists());
        assert!(directory.join("notas.txt").exists());
        assert!(directory.join("voxel.not-a-date.log").exists());
        assert!(directory.join("voxel.2000-01-01.log").is_dir());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn diretorio_ausente_nao_e_erro_e_arquivo_no_lugar_e() {
        let missing = directory("ausente");
        assert!(prune_old_logs(&missing, date!(2026 - 10 - 19), 14).is_ok());

        let parent = directory("arquivo");
        std::fs::create_dir_all(&parent).unwrap();
        let file = parent.join("nao-e-dir");
        std::fs::write(&file, "x").unwrap();
        assert!(prune_old_logs(&file, date!(2026 - 10 - 19), 14).is_err());
        assert!(LogError::AlreadyInstalled.to_string().contains("iniciado"));
        let io = LogError::Io(std::io::Error::other("disco cheio"));
        assert!(io.to_string().contains("disco cheio"));
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn grava_o_log_do_dia_e_recusa_a_segunda_instalacao() {
        let directory = directory("start");
        let log = AppLog::start(&directory, DEFAULT_RETENTION_DAYS).unwrap();
        tracing::info!("linha de teste");
        drop(log);

        let mut contents = String::new();
        for entry in std::fs::read_dir(&directory).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            let extension = std::path::Path::new(&name).extension();
            if name.starts_with("voxel.")
                && extension.is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
            {
                contents.push_str(&std::fs::read_to_string(entry.path()).unwrap());
            }
        }
        assert!(contents.contains("linha de teste"), "{contents}");
        assert!(
            contents.contains("log do aplicativo iniciado"),
            "{contents}"
        );

        let again = AppLog::start(&directory, DEFAULT_RETENTION_DAYS).unwrap_err();
        assert!(matches!(again, LogError::AlreadyInstalled));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
