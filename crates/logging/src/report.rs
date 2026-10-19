use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;

use crate::capture::ProcessOutcome;
use crate::classify::{Classification, classify};

const STAMP: &[FormatItem<'_>] =
    format_description!("[year]-[month]-[day]_[hour]-[minute]-[second]");

/// Dados de uma falha de worker, prontos para o arquivo de log.
pub struct FailureInput {
    pub message: String,
    pub outcome: ProcessOutcome,
    pub stdout: String,
    pub stderr: String,
    pub job_id: String,
    pub parameters: BTreeMap<String, String>,
    pub backend: String,
    pub model: String,
    pub hardware: String,
    pub when: OffsetDateTime,
}

/// Arquivo gravado e o texto que o modal mostra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureRecord {
    pub classification: Classification,
    pub path: PathBuf,
    pub log: String,
}

/// Payload do evento que abre o modal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorNotice {
    pub summary_key: String,
    pub log: String,
    pub path: String,
}

impl FailureRecord {
    #[must_use]
    pub fn notice(&self) -> ErrorNotice {
        ErrorNotice {
            summary_key: self.classification.kind.summary_key().to_owned(),
            log: self.log.clone(),
            path: self.path.display().to_string(),
        }
    }
}

/// Grava `erro_<data>_<job>.log` com a mensagem, o processo, a saída, o job e o hardware.
///
/// # Errors
///
/// Falha se o diretório ou o arquivo não puder ser criado.
pub fn record_failure(directory: &Path, input: &FailureInput) -> std::io::Result<FailureRecord> {
    std::fs::create_dir_all(directory)?;
    let output = format!("{}\n{}", input.stdout, input.stderr);
    let classification = classify(&output, input.outcome.signal);
    let log = render(input, classification);
    let stamp = input
        .when
        .format(STAMP)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let (path, mut file) = create_log_file(directory, &stamp, &sanitize(&input.job_id))?;
    file.write_all(log.as_bytes())?;
    Ok(FailureRecord {
        classification,
        path,
        log,
    })
}

fn render(input: &FailureInput, classification: Classification) -> String {
    let mut text = format!(
        "\
Voxel — log de erro
Quando: {when}
Mensagem: {message}
Código de saída: {exit_code}
Sinal: {signal}
Classificação: {kind}
Ação sugerida: {action}
Backend: {backend}
Modelo: {model}
Hardware: {hardware}
Job: {job}
",
        when = input.when,
        message = input.message,
        exit_code = display_optional(input.outcome.exit_code),
        signal = display_optional(input.outcome.signal),
        kind = classification.kind.id(),
        action = classification.action.as_str(),
        backend = input.backend,
        model = input.model,
        hardware = input.hardware,
        job = input.job_id,
    );
    push_parameters(&mut text, &input.parameters);
    text.push_str("\n--- stdout (últimas linhas) ---\n");
    text.push_str(&input.stdout);
    if !input.stdout.is_empty() && !input.stdout.ends_with('\n') {
        text.push('\n');
    }
    text.push_str("\n--- stderr (últimas linhas) ---\n");
    text.push_str(&input.stderr);
    if !input.stderr.is_empty() && !input.stderr.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn push_parameters(text: &mut String, parameters: &BTreeMap<String, String>) {
    if parameters.is_empty() {
        text.push_str("Parâmetros: (nenhum)\n");
        return;
    }
    text.push_str("Parâmetros:\n");
    for (key, value) in parameters {
        text.push_str("  ");
        text.push_str(key);
        text.push_str(": ");
        text.push_str(value);
        text.push('\n');
    }
}

fn display_optional(value: Option<i32>) -> String {
    value.map_or_else(|| "—".to_owned(), |number| number.to_string())
}

fn sanitize(job_id: &str) -> String {
    let mut name: String = job_id
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() {
        name.push_str("job");
    }
    name.chars().take(40).collect()
}

fn create_log_file(
    directory: &Path,
    stamp: &str,
    job: &str,
) -> std::io::Result<(PathBuf, std::fs::File)> {
    for sequence in 1..=100 {
        let name = if sequence == 1 {
            format!("erro_{stamp}_{job}.log")
        } else {
            format!("erro_{stamp}_{job}_{sequence}.log")
        };
        let path = directory.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::other(
        "muitos logs de erro no mesmo segundo",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn input(job_id: &str) -> FailureInput {
        let mut parameters = BTreeMap::new();
        parameters.insert("steps".to_owned(), "30".to_owned());
        FailureInput {
            message: "o worker morreu no VAE".to_owned(),
            outcome: ProcessOutcome {
                exit_code: Some(0),
                signal: Some(6),
            },
            stdout: "denoising ok".to_owned(),
            stderr: include_str!("../fixtures/invokeai_rx7600.log").to_owned(),
            job_id: job_id.to_owned(),
            parameters,
            backend: "pytorch-rocm".to_owned(),
            model: "Anima".to_owned(),
            hardware: "GPU: AMD Radeon RX 7600 (gfx1102), 8 GB".to_owned(),
            when: datetime!(2026-10-18 14:32:05 UTC),
        }
    }

    fn directory(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("voxel-error-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn grava_o_caso_do_invokeai_com_o_nome_do_planejamento() {
        let directory = directory("invokeai");
        let record = record_failure(&directory, &input("npc-1")).unwrap();

        assert_eq!(
            record.path.file_name().unwrap(),
            "erro_2026-10-18_14-32-05_npc-1.log"
        );
        assert_eq!(std::fs::read_to_string(&record.path).unwrap(), record.log);
        assert!(record.log.contains("Sinal: 6"));
        assert!(record.log.contains("Código de saída: 0"));
        assert!(record.log.contains("Classificação: incompatible-kernel"));
        assert!(record.log.contains("pytorch-rocm"));
        assert!(record.log.contains("Anima"));
        assert!(record.log.contains("gfx1102"));
        assert!(record.log.contains("steps: 30"));
        assert!(record.log.contains("HSA_STATUS_ERROR_INVALID_ISA"));
        assert!(record.log.contains("--- stdout (últimas linhas) ---"));
        assert!(record.log.contains("denoising ok"));

        let notice = record.notice();
        let json = serde_json::to_value(&notice).unwrap();
        assert_eq!(json["summaryKey"], "error.summary.incompatibleKernel");
        assert_eq!(json["path"], record.path.display().to_string());
        assert!(json.get("summary_key").is_none());

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn o_segundo_arquivo_no_mesmo_segundo_nao_sobrescreve() {
        let directory = directory("colisao");
        let first = record_failure(&directory, &input("npc-1")).unwrap();
        let second = record_failure(&directory, &input("npc-1")).unwrap();
        assert_ne!(first.path, second.path);
        assert!(
            second
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with("_2.log")
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn job_vazio_ou_com_barra_vira_nome_de_arquivo() {
        let directory = directory("nome");
        let mut blank = input("");
        blank.parameters.clear();
        blank.outcome.exit_code = None;
        blank.outcome.signal = None;
        blank.stdout.clear();
        blank.stderr = "algo inesperado".to_owned();
        let record = record_failure(&directory, &blank).unwrap();
        assert!(
            record
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("_job.log")
        );
        assert!(record.log.contains("Código de saída: —"));
        assert!(record.log.contains("Sinal: —"));
        assert!(record.log.contains("Parâmetros: (nenhum)"));
        assert!(record.log.contains("Classificação: unknown"));

        let mut messy = input("vila/ferreiro 01");
        messy.when = datetime!(2026-10-18 14:32:06 UTC);
        let named = record_failure(&directory, &messy).unwrap();
        assert!(
            named
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("vila_ferreiro_01")
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn nome_longo_e_cortado() {
        assert_eq!(sanitize(&"a".repeat(50)).chars().count(), 40);
    }

    #[test]
    fn diretorio_invalido_falha() {
        let directory = directory("io");
        std::fs::create_dir_all(&directory).unwrap();
        let blocker = directory.join("arquivo");
        std::fs::write(&blocker, "x").unwrap();
        assert!(record_failure(&blocker.join("logs"), &input("npc-1")).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
