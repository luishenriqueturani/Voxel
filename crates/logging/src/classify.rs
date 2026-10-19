/// Interpretação de uma falha a partir da saída do worker e do sinal do processo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Classification {
    pub kind: ErrorKind,
    pub action: SuggestedAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    IncompatibleKernel,
    OutOfMemory,
    NativeCrash,
    Network,
    Unknown,
}

impl ErrorKind {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::IncompatibleKernel => "incompatible-kernel",
            Self::OutOfMemory => "out-of-memory",
            Self::NativeCrash => "native-crash",
            Self::Network => "network",
            Self::Unknown => "unknown",
        }
    }

    /// Chave do i18next com o resumo em linguagem simples.
    #[must_use]
    pub const fn summary_key(self) -> &'static str {
        match self {
            Self::IncompatibleKernel => "error.summary.incompatibleKernel",
            Self::OutOfMemory => "error.summary.outOfMemory",
            Self::NativeCrash => "error.summary.nativeCrash",
            Self::Network => "error.summary.network",
            Self::Unknown => "error.summary.unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestedAction {
    RetryWithoutMiopen,
    RetryLighter,
    RestartWithFallback,
    ResumeDownload,
    None,
}

impl SuggestedAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RetryWithoutMiopen => "repetir sem os kernels assembly da MIOpen",
            Self::RetryLighter => "repetir com uma configuração mais leve",
            Self::RestartWithFallback => "reiniciar o worker e aplicar o fallback",
            Self::ResumeDownload => "retomar o download depois",
            Self::None => "nenhuma",
        }
    }
}

struct Rule {
    kind: ErrorKind,
    action: SuggestedAction,
    contains_all: &'static [&'static str],
    contains_any: &'static [&'static str],
    signals: &'static [i32],
}

impl Rule {
    fn matches(&self, haystack: &str, signal: Option<i32>) -> bool {
        let needs_text = !self.contains_all.is_empty() || !self.contains_any.is_empty();
        if needs_text {
            let all = self
                .contains_all
                .iter()
                .all(|needle| haystack.contains(needle));
            let any = self.contains_any.is_empty()
                || self
                    .contains_any
                    .iter()
                    .any(|needle| haystack.contains(needle));
            if !all || !any {
                return false;
            }
        }
        if !self.signals.is_empty() && !signal.is_some_and(|value| self.signals.contains(&value)) {
            return false;
        }
        needs_text || !self.signals.is_empty()
    }
}

/// Tabela de padrões conhecidos. A primeira regra que casar vence.
const RULES: &[Rule] = &[
    Rule {
        kind: ErrorKind::IncompatibleKernel,
        action: SuggestedAction::RetryWithoutMiopen,
        contains_all: &["hsa_status_error_invalid_isa", "miopen", "asm"],
        contains_any: &[],
        signals: &[],
    },
    Rule {
        kind: ErrorKind::OutOfMemory,
        action: SuggestedAction::RetryLighter,
        contains_all: &[],
        contains_any: &["out of memory", "hiperroroutofmemory", "cuda out of memory"],
        signals: &[],
    },
    Rule {
        kind: ErrorKind::Network,
        action: SuggestedAction::ResumeDownload,
        contains_all: &[],
        contains_any: &[
            "failed to download",
            "download failed",
            "could not resolve host",
            "connection reset",
            "network is unreachable",
            "temporary failure in name resolution",
            "failed to fetch",
        ],
        signals: &[],
    },
    Rule {
        kind: ErrorKind::NativeCrash,
        action: SuggestedAction::RestartWithFallback,
        contains_all: &[],
        contains_any: &[],
        signals: &[6, 11],
    },
];

/// Reconhece um padrão conhecido na saída do worker.
///
/// `signal` é o sinal do processo, quando ele morreu por sinal. Texto mais específico
/// ganha de um sinal genérico: um `SIGABRT` no meio do caso da `MIOpen` continua sendo
/// kernel incompatível.
#[must_use]
pub fn classify(output: &str, signal: Option<i32>) -> Classification {
    let haystack = output.to_lowercase();
    for rule in RULES {
        if rule.matches(&haystack, signal) {
            return Classification {
                kind: rule.kind,
                action: rule.action,
            };
        }
    }
    Classification {
        kind: ErrorKind::Unknown,
        action: SuggestedAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_do_invokeai_na_rx_7600_e_kernel_incompativel() {
        let log = include_str!("../fixtures/invokeai_rx7600.log");
        let classification = classify(log, Some(6));

        assert_eq!(classification.kind, ErrorKind::IncompatibleKernel);
        assert_eq!(classification.action, SuggestedAction::RetryWithoutMiopen);
        assert_eq!(
            classification.kind.summary_key(),
            "error.summary.incompatibleKernel"
        );
    }

    #[test]
    fn o_texto_do_caso_basta_sem_o_sinal_do_processo() {
        let log = include_str!("../fixtures/invokeai_rx7600.log");
        assert_eq!(classify(log, None).kind, ErrorKind::IncompatibleKernel);
    }

    #[test]
    fn isa_sem_kernel_miopen_cai_no_sinal() {
        let classification = classify("HSA_STATUS_ERROR_INVALID_ISA", Some(6));
        assert_eq!(classification.kind, ErrorKind::NativeCrash);
    }

    #[test]
    fn falta_de_memoria_ignora_maiusculas_e_vence_o_sinal() {
        assert_eq!(
            classify("CUDA out of memory", Some(6)).action,
            SuggestedAction::RetryLighter
        );
        assert_eq!(
            classify("hipErrorOutOfMemory", None).kind,
            ErrorKind::OutOfMemory
        );
        assert_eq!(classify("Out of Memory", None).kind, ErrorKind::OutOfMemory);
    }

    #[test]
    fn falha_de_rede() {
        assert_eq!(
            classify("Could not resolve host: huggingface.co", None).action,
            SuggestedAction::ResumeDownload
        );
        assert_eq!(
            classify("connection reset by peer", Some(6)).kind,
            ErrorKind::Network
        );
    }

    #[test]
    fn sinais_6_e_11_sao_crash_nativo() {
        assert_eq!(classify("morreu", Some(6)).kind, ErrorKind::NativeCrash);
        assert_eq!(classify("morreu", Some(11)).kind, ErrorKind::NativeCrash);
        assert_eq!(classify("morreu", Some(9)).kind, ErrorKind::Unknown);
        assert_eq!(ErrorKind::NativeCrash.id(), "native-crash");
        assert_eq!(
            SuggestedAction::RestartWithFallback.as_str(),
            "reiniciar o worker e aplicar o fallback"
        );
    }

    #[test]
    fn saida_desconhecida_nao_sugere_acao() {
        let classification = classify("algo inesperado", None);
        assert_eq!(classification.kind, ErrorKind::Unknown);
        assert_eq!(classification.action, SuggestedAction::None);
        assert_eq!(classification.kind.summary_key(), "error.summary.unknown");
    }

    #[test]
    fn a_tabela_tem_os_quatro_casos_do_planejamento() {
        assert_eq!(RULES.len(), 4);
    }
}
