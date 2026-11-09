//! Impede suspensão e hibernação enquanto a fila está gerando.

use std::io;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, PoisonError};

/// Processo `systemd-inhibit` vivo. Soltar o processo libera o bloqueio.
#[derive(Default)]
pub struct SleepInhibit {
    child: Mutex<Option<Child>>,
}

/// Argumentos do `systemd-inhibit` usados na fila.
#[must_use]
pub fn inhibit_args() -> &'static [&'static str] {
    &[
        "--what=sleep:idle",
        "--who=Voxel",
        "--why=Fila de execução",
        "--mode=block",
        "sleep",
        "infinity",
    ]
}

impl SleepInhibit {
    /// Segura o bloqueio se ainda não houver um processo vivo.
    pub fn hold(&self) {
        self.hold_spawn(start_inhibit);
    }

    fn hold_spawn<F>(&self, spawn: F)
    where
        F: FnOnce() -> io::Result<Child>,
    {
        let mut slot = self.child.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(child) = slot.as_mut()
            && child.try_wait().ok().flatten().is_none()
        {
            return;
        }
        *slot = spawn().ok();
    }

    /// Encerra o processo e libera a suspensão.
    pub fn release(&self) {
        let mut slot = self.child.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(mut child) = slot.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for SleepInhibit {
    fn drop(&mut self) {
        self.release();
    }
}

fn start_inhibit() -> io::Result<Child> {
    Command::new("systemd-inhibit")
        .args(inhibit_args())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

#[cfg(test)]
mod tests {
    use super::{SleepInhibit, inhibit_args};
    use std::process::Command;

    #[test]
    fn os_argumentos_bloqueiam_sono_e_ociosidade() {
        let args = inhibit_args();
        assert!(args.contains(&"--what=sleep:idle"));
        assert!(args.contains(&"--mode=block"));
        assert!(args.contains(&"--who=Voxel"));
    }

    #[test]
    fn segura_e_solta_o_processo() {
        let inhibit = SleepInhibit::default();
        inhibit.hold_spawn(|| Command::new("sleep").arg("30").spawn());
        {
            let slot = inhibit.child.lock().expect("lock");
            assert!(slot.is_some());
        }
        inhibit.release();
        {
            let slot = inhibit.child.lock().expect("lock");
            assert!(slot.is_none());
        }
    }
}
