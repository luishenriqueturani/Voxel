//! Roda o `uv` e, no final, a convolução que decide se o backend serve.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use voxel_hardware::{PythonBackend, facts_from, python_fallback};

use super::PythonProgress;
use super::plan::{self, Layout, ProcessEnv};
use super::probe::{self, PROBE_SCRIPT};
use super::progress::{self, Watch};
use crate::paths::AppPaths;

pub(super) const INTERRUPTED: &str = "interrompido";

pub fn install(
    paths: &AppPaths,
    wipe: bool,
    child: &AtomicU32,
    job: u64,
    current_job: &dyn Fn() -> u64,
    mut emit: impl FnMut(&PythonProgress),
) -> Result<String, String> {
    let alive = || current_job() == job;
    let layout = Layout::new(&paths.data, &paths.cache);
    if wipe && layout.root.exists() {
        std::fs::remove_dir_all(&layout.root).map_err(|error| error.to_string())?;
    }
    std::fs::create_dir_all(&layout.interpreters).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&layout.cache).map_err(|error| error.to_string())?;
    let machine = voxel_hardware::detect();
    let isa = machine
        .gpus
        .iter()
        .max_by_key(|gpu| gpu.vram_bytes)
        .and_then(|gpu| gpu.isa.clone());
    let order = python_fallback(&facts_from(&machine, false));
    let mut watch = Watch::default();
    emit(&preparing(&watch, 0));
    run_uv(
        &plan::venv_args(&layout.venv),
        &layout,
        &mut watch,
        child,
        &alive,
        &mut emit,
    )?;
    let mut last = "nenhum backend Python passou no teste".to_owned();
    for backend in order {
        if !alive() {
            return Err(INTERRUPTED.to_owned());
        }
        let Some(index) = plan::index_url(backend, machine.rocm_version.as_deref()) else {
            continue;
        };
        run_uv(
            &plan::pip_args(&layout.python(), &index),
            &layout,
            &mut watch,
            child,
            &alive,
            &mut emit,
        )?;
        emit(&PythonProgress::testing());
        match probe(&layout, backend, isa.as_deref(), child, &alive) {
            Ok(()) => {
                let marker = Marker {
                    backend: backend.id().to_owned(),
                    torch: plan::torch_requirement().to_owned(),
                    index,
                };
                write_marker(&layout.marker, &marker)?;
                return Ok(marker.backend);
            }
            Err(error) => last = error,
        }
    }
    Err(last)
}

#[derive(serde::Serialize)]
struct Marker {
    backend: String,
    torch: String,
    index: String,
}

fn preparing(watch: &Watch, received: u64) -> PythonProgress {
    let total = watch.total();
    PythonProgress::preparing(
        progress::percent(received, total),
        (received > 0).then_some(received),
        (total > 0).then_some(total),
    )
}

fn run_uv(
    args: &[String],
    layout: &Layout,
    watch: &mut Watch,
    child: &AtomicU32,
    alive: &dyn Fn() -> bool,
    emit: &mut dyn FnMut(&PythonProgress),
) -> Result<(), String> {
    if !alive() {
        return Err(INTERRUPTED.to_owned());
    }
    let baseline = progress::dir_size(&layout.cache);
    let mut command = Command::new("uv");
    command
        .args(args)
        .env("UV_PYTHON_INSTALL_DIR", &layout.interpreters)
        .env("UV_CACHE_DIR", &layout.cache)
        .env("UV_PYTHON_DOWNLOADS", "automatic")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut process = command.spawn().map_err(|_| "uv ausente".to_owned())?;
    let pid = process.id();
    child.store(pid, Ordering::SeqCst);
    let _slot = ChildSlot { child, pid };
    let (sender, receiver) = mpsc::channel();
    pump(process.stdout.take(), sender.clone());
    pump(process.stderr.take(), sender);
    let mut tail = Vec::new();
    loop {
        match receiver.recv_timeout(Duration::from_millis(400)) {
            Ok(line) => {
                remember(&mut tail, &line);
                watch.note(&line);
                let arrived = received(watch, layout, baseline);
                emit(&preparing(watch, arrived));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                emit(&preparing(watch, received(watch, layout, baseline)));
                if process
                    .try_wait()
                    .map_err(|error| error.to_string())?
                    .is_some()
                {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if !alive() {
            let _ = process.kill();
            let _ = process.wait();
            return Err(INTERRUPTED.to_owned());
        }
    }
    let status = process.wait().map_err(|error| error.to_string())?;
    if status.success() {
        emit(&preparing(
            watch,
            watch.total().max(received(watch, layout, baseline)),
        ));
        return Ok(());
    }
    if tail.is_empty() {
        Err("uv falhou".to_owned())
    } else {
        Err(tail.join("\n"))
    }
}

fn received(watch: &Watch, layout: &Layout, baseline: u64) -> u64 {
    let growth = progress::dir_size(&layout.cache).saturating_sub(baseline);
    growth.max(watch.completed())
}

fn pump(stream: Option<impl std::io::Read + Send + 'static>, sender: Sender<String>) {
    let Some(stream) = stream else {
        return;
    };
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if sender.send(line).is_err() {
                break;
            }
        }
    });
}

fn remember(tail: &mut Vec<String>, line: &str) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }
    tail.push(trimmed.to_owned());
    if tail.len() > 8 {
        tail.remove(0);
    }
}

fn probe(
    layout: &Layout,
    backend: PythonBackend,
    isa: Option<&str>,
    child: &AtomicU32,
    alive: &dyn Fn() -> bool,
) -> Result<(), String> {
    if !alive() {
        return Err(INTERRUPTED.to_owned());
    }
    let script = layout.cache.join("probe.py");
    std::fs::write(&script, PROBE_SCRIPT).map_err(|error| error.to_string())?;
    let env = plan::process_env(backend, isa);
    let mut command = Command::new(layout.python());
    command
        .arg(&script)
        .arg(plan::probe_device(backend))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_env(&mut command, &env);
    let process = command.spawn().map_err(|error| error.to_string())?;
    let pid = process.id();
    child.store(pid, Ordering::SeqCst);
    let _slot = ChildSlot { child, pid };
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = std::sync::Arc::clone(&done);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_mins(2));
        if !flag.load(Ordering::SeqCst) {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
    });
    let output = process
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    done.store(true, Ordering::SeqCst);
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if probe::passed(output.status.success(), &text) {
        Ok(())
    } else if text.is_empty() {
        Err("teste do ambiente falhou".to_owned())
    } else {
        Err(text)
    }
}

fn apply_env(command: &mut Command, env: &ProcessEnv) {
    for name in &env.remove {
        command.env_remove(name);
    }
    for (name, value) in &env.set {
        command.env(name, value);
    }
}

fn write_marker(path: &Path, marker: &Marker) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string(marker).map_err(|error| error.to_string())?;
    std::fs::write(path, text).map_err(|error| error.to_string())
}

struct ChildSlot<'a> {
    child: &'a AtomicU32,
    pid: u32,
}

impl Drop for ChildSlot<'_> {
    fn drop(&mut self) {
        let _ = self
            .child
            .compare_exchange(self.pid, 0, Ordering::SeqCst, Ordering::SeqCst);
    }
}

pub fn stop_child(child: &AtomicU32) {
    let pid = child.swap(0, Ordering::SeqCst);
    if pid > 0 {
        let _ = Command::new("kill").arg(pid.to_string()).status();
    }
}
