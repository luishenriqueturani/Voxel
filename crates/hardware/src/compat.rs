/// Combinação que o setup pode lembrar: modelo, backend e configuração.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Combination {
    pub model: String,
    pub backend: String,
    pub configuration: String,
}

/// Uma combinação que já quebrou nesta máquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub combination: Combination,
    pub detail: String,
}

/// Memória de compatibilidade. Vale até o próximo "Refazer setup".
#[derive(Debug, Default, Clone)]
pub struct CompatibilityMemory {
    failures: Vec<Failure>,
}

impl CompatibilityMemory {
    #[must_use]
    pub fn allows(&self, combination: &Combination) -> bool {
        !self
            .failures
            .iter()
            .any(|failure| &failure.combination == combination)
    }

    pub fn record_failure(&mut self, combination: Combination, detail: impl Into<String>) {
        if self.allows(&combination) {
            self.failures.push(Failure {
                combination,
                detail: detail.into(),
            });
        }
    }

    pub fn clear(&mut self) {
        self.failures.clear();
    }

    #[must_use]
    pub fn failures(&self) -> &[Failure] {
        &self.failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(backend: &str) -> Combination {
        Combination {
            model: "sd15".to_owned(),
            backend: backend.to_owned(),
            configuration: "vae-f16".to_owned(),
        }
    }

    #[test]
    fn falha_registrada_nao_volta_a_ser_tentada_ate_limpar() {
        let mut memory = CompatibilityMemory::default();
        assert!(memory.allows(&combo("vulkan")));
        memory.record_failure(combo("vulkan"), "SIGABRT");
        memory.record_failure(combo("vulkan"), "de novo");
        assert!(!memory.allows(&combo("vulkan")));
        assert!(memory.allows(&combo("cpu")));
        assert_eq!(memory.failures().len(), 1);
        assert_eq!(memory.failures()[0].detail, "SIGABRT");

        memory.clear();
        assert!(memory.allows(&combo("vulkan")));
        assert!(memory.failures().is_empty());
    }
}
