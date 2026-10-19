//! Logs da aplicação, arquivos de log por erro e classificação de erros.
//!
//! O log geral usa `tracing` e gira por dia. Cada falha de worker ganha um arquivo próprio.
//! A saída dos processos filhos é lida enquanto eles ainda estão rodando.

mod app_log;
mod capture;
mod classify;
mod report;

pub use app_log::{AppLog, DEFAULT_RETENTION_DAYS, LogError, prune_old_logs};
pub use capture::{DEFAULT_LINE_LIMIT, FinishedProcess, ProcessOutcome, RunningProcess};
pub use classify::{Classification, ErrorKind, SuggestedAction, classify};
pub use report::{ErrorNotice, FailureInput, FailureRecord, record_failure};

/// Evento Tauri que abre o modal de erro. O payload é [`ErrorNotice`].
pub const ERROR_EVENT: &str = "error://show";
