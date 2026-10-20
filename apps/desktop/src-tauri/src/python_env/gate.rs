//! Uma instalação por vez. Quem pede 3D no meio do caminho espera, não falha.

use std::sync::{Condvar, Mutex, PoisonError};

use super::PythonProgress;

struct Inner {
    progress: PythonProgress,
    job: u64,
}

pub struct Gate {
    inner: Mutex<Inner>,
    changed: Condvar,
}

impl Default for Gate {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner {
                progress: PythonProgress::absent(),
                job: 0,
            }),
            changed: Condvar::new(),
        }
    }
}

fn locked(inner: &Mutex<Inner>) -> std::sync::MutexGuard<'_, Inner> {
    inner.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Gate {
    pub fn get(&self) -> PythonProgress {
        locked(&self.inner).progress.clone()
    }

    pub fn set(&self, progress: PythonProgress) {
        locked(&self.inner).progress = progress;
        self.changed.notify_all();
    }

    /// Começa o trabalho se ainda não há ambiente pronto nem instalação andando.
    pub fn start(&self) -> Option<u64> {
        let mut inner = locked(&self.inner);
        if inner.progress.is_working() || inner.progress.phase == "ready" {
            return None;
        }
        Some(arm(&mut inner, &self.changed))
    }

    /// Troca a instalação atual por outra. O job anterior deixa de valer.
    pub fn restart(&self) -> u64 {
        arm(&mut locked(&self.inner), &self.changed)
    }

    #[must_use]
    pub fn current_job(&self) -> u64 {
        locked(&self.inner).job
    }

    /// Publica o progresso só se esta ainda for a instalação corrente.
    pub fn publish(&self, job: u64, progress: PythonProgress) -> bool {
        let mut inner = locked(&self.inner);
        if inner.job != job {
            return false;
        }
        inner.progress = progress;
        drop(inner);
        self.changed.notify_all();
        true
    }

    pub fn set_if_absent(&self, progress: PythonProgress) {
        let mut inner = locked(&self.inner);
        if inner.progress.phase != "absent" {
            return;
        }
        inner.progress = progress;
        drop(inner);
        self.changed.notify_all();
    }

    pub fn wait(&self) -> PythonProgress {
        let mut inner = locked(&self.inner);
        while inner.progress.is_working() {
            inner = self
                .changed
                .wait(inner)
                .unwrap_or_else(PoisonError::into_inner);
        }
        inner.progress.clone()
    }
}

fn arm(inner: &mut Inner, changed: &Condvar) -> u64 {
    inner.progress = PythonProgress::preparing(None, None, None);
    inner.job = inner.job.saturating_add(1);
    changed.notify_all();
    inner.job
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_segunda_intencao_nao_abre_outra_instalacao_e_a_espera_sai_quando_termina() {
        let gate = Gate::default();
        let job = gate.start().unwrap();
        assert_eq!(gate.start(), None);
        assert!(gate.get().is_working());

        std::thread::scope(|scope| {
            let waiting = scope.spawn(|| gate.wait());
            std::thread::sleep(Duration::from_millis(20));
            assert!(gate.publish(job, PythonProgress::ready("rocm_native")));
            let done = waiting.join().unwrap();
            assert_eq!(done.phase, "ready");
        });
        assert_eq!(gate.start(), None);
    }

    #[test]
    fn reinstalar_invalida_o_resultado_da_instalacao_anterior() {
        let gate = Gate::default();
        let first = gate.start().unwrap();
        let second = gate.restart();
        assert_ne!(first, second);
        assert!(!gate.publish(first, PythonProgress::failed("tarde")));
        assert!(gate.get().is_working());
        assert!(gate.publish(second, PythonProgress::ready("cpu")));
        assert_eq!(gate.wait().backend.as_deref(), Some("cpu"));
    }
}
