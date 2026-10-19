use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Quantas linhas recentes de cada fluxo ficam guardadas.
pub const DEFAULT_LINE_LIMIT: usize = 200;

/// Como o processo filho terminou.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessOutcome {
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
}

/// Saída capturada até o processo terminar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedProcess {
    pub outcome: ProcessOutcome,
    pub stdout: String,
    pub stderr: String,
}

struct LineBuffer {
    limit: usize,
    lines: VecDeque<String>,
}

impl LineBuffer {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            lines: VecDeque::new(),
        }
    }

    fn push(&mut self, line: String) {
        if self.limit == 0 {
            return;
        }
        if self.lines.len() == self.limit {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    fn joined(&self) -> String {
        let mut text = String::new();
        for (index, line) in self.lines.iter().enumerate() {
            if index > 0 {
                text.push('\n');
            }
            text.push_str(line);
        }
        text
    }
}

/// Processo filho cuja saída é lida enquanto ele ainda roda.
///
/// As últimas `line_limit` linhas de `stdout` e de `stderr` ficam disponíveis
/// antes do término. Um `SIGABRT` fecha o pipe, e o que já tinha sido escrito permanece.
pub struct RunningProcess {
    child: Option<Child>,
    stdout: Arc<Mutex<LineBuffer>>,
    stderr: Arc<Mutex<LineBuffer>>,
    stdout_thread: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
}

impl RunningProcess {
    /// Inicia `command` com os dois fluxos em pipe.
    ///
    /// # Errors
    ///
    /// Falha se o processo não puder ser criado ou se o sistema não entregar os pipes.
    pub fn spawn(mut command: Command, line_limit: usize) -> std::io::Result<Self> {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("stdout sem pipe"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| std::io::Error::other("stderr sem pipe"))?;
        let stdout_buffer = Arc::new(Mutex::new(LineBuffer::new(line_limit)));
        let stderr_buffer = Arc::new(Mutex::new(LineBuffer::new(line_limit)));
        Ok(Self {
            child: Some(child),
            stdout_thread: Some(spawn_reader(stdout, Arc::clone(&stdout_buffer))),
            stderr_thread: Some(spawn_reader(stderr, Arc::clone(&stderr_buffer))),
            stdout: stdout_buffer,
            stderr: stderr_buffer,
        })
    }

    /// Linhas de `stdout` já lidas. Não espera o processo terminar.
    #[must_use]
    pub fn stdout_text(&self) -> String {
        joined(&self.stdout)
    }

    /// Linhas de `stderr` já lidas. Não espera o processo terminar.
    #[must_use]
    pub fn stderr_text(&self) -> String {
        joined(&self.stderr)
    }

    /// Espera o término e devolve o código, o sinal e as últimas linhas.
    ///
    /// # Errors
    ///
    /// Falha se a espera pelo processo falhar.
    pub fn finish(mut self) -> std::io::Result<FinishedProcess> {
        let Some(mut child) = self.child.take() else {
            return Err(std::io::Error::other("processo já encerrado"));
        };
        let status = child.wait()?;
        self.join_readers();
        Ok(FinishedProcess {
            outcome: outcome_from(status),
            stdout: joined(&self.stdout),
            stderr: joined(&self.stderr),
        })
    }

    fn join_readers(&mut self) {
        if let Some(handle) = self.stdout_thread.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.stderr_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for RunningProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.join_readers();
    }
}

fn spawn_reader(
    reader: impl Read + Send + 'static,
    buffer: Arc<Mutex<LineBuffer>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else {
                continue;
            };
            if let Ok(mut guard) = buffer.lock() {
                guard.push(line);
            }
        }
    })
}

fn joined(buffer: &Mutex<LineBuffer>) -> String {
    buffer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .joined()
}

fn outcome_from(status: ExitStatus) -> ProcessOutcome {
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(&status);
    #[cfg(not(unix))]
    let signal = None;
    ProcessOutcome {
        exit_code: status.code(),
        signal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.arg("-c").arg(script);
        command
    }

    #[test]
    fn guarda_so_as_ultimas_linhas_de_cada_fluxo() {
        let script = (0..10)
            .map(|index| format!("echo out-{index}\necho err-{index} >&2"))
            .collect::<Vec<_>>()
            .join("\n");
        let finished = RunningProcess::spawn(shell(&script), 3)
            .unwrap()
            .finish()
            .unwrap();

        assert_eq!(finished.stdout, "out-7\nout-8\nout-9");
        assert_eq!(finished.stderr, "err-7\nerr-8\nerr-9");
        assert_eq!(finished.outcome.exit_code, Some(0));
        assert_eq!(finished.outcome.signal, None);
    }

    #[test]
    fn limite_zero_descarta_a_saida() {
        let finished = RunningProcess::spawn(shell("echo some; echo err >&2"), 0)
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!(finished.stdout, "");
        assert_eq!(finished.stderr, "");
    }

    #[test]
    fn le_a_saida_antes_do_processo_terminar() {
        let running = RunningProcess::spawn(
            shell("printf 'ready\\n'; exec sleep 30"),
            DEFAULT_LINE_LIMIT,
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !running.stdout_text().contains("ready") {
            assert!(
                Instant::now() < deadline,
                "a saída não chegou com o processo vivo"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(running.stderr_text(), "");
    }

    #[cfg(unix)]
    #[test]
    fn um_sigabrt_preserva_o_que_ja_foi_escrito() {
        let finished = RunningProcess::spawn(shell("echo antes; kill -ABRT $$"), 20)
            .unwrap()
            .finish()
            .unwrap();
        assert!(finished.stdout.contains("antes"), "{}", finished.stdout);
        assert_eq!(finished.outcome.signal, Some(6));
    }
}
