//! Título curto do prompt recente. A imagem não espera este passo.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::db::Database;
use crate::paths::AppPaths;

const TITLE_LIMIT: usize = 60;
const TITLE_WORDS: usize = 6;
const TITLE_EVENT: &str = "prompt://title";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptTitlePayload {
    prompt: String,
    title: String,
}

/// Gera o título em segundo plano. Falha silenciosa: o prompt continua como está.
pub fn schedule(app: &AppHandle, database: &Database, paths: &AppPaths, prompt: &str) {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return;
    }
    if database
        .prompt_title(prompt)
        .ok()
        .flatten()
        .is_some_and(|title| kept_title(&title, prompt))
    {
        return;
    }
    let app = app.clone();
    let database = database.clone();
    let prompt = prompt.to_owned();
    let models = crate::models::folder(paths, &database);
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(title) = caption_of(&prompt, &models)
            && database.set_prompt_title(&prompt, &title).is_ok()
        {
            let _ = app.emit(TITLE_EVENT, PromptTitlePayload { prompt, title });
        }
    });
}

fn caption_of(prompt: &str, models: &Path) -> Option<String> {
    if let Ok(command) = std::env::var("VOXEL_CAPTION_CMD") {
        return run_command(&command, &[], prompt, prompt);
    }
    caption_gguf(prompt, models)
        .or_else(|| caption_ollama(prompt))
        .or_else(|| heuristic_title(prompt))
}

fn caption_gguf(prompt: &str, models: &Path) -> Option<String> {
    let binary = llama_binary()?;
    let model = small_text_gguf(models)?;
    run_command(
        &binary,
        &[
            "-m",
            model.to_str()?,
            "-n",
            "24",
            "--no-display-prompt",
            "-p",
            &instruction(prompt),
        ],
        "",
        prompt,
    )
}

fn caption_ollama(prompt: &str) -> Option<String> {
    let listed = Command::new("ollama")
        .arg("list")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !listed.status.success() {
        return None;
    }
    let model = ollama_model(&String::from_utf8_lossy(&listed.stdout))?;
    let prompt_text = instruction(prompt);
    let quiet = [
        "run",
        "--nowordwrap",
        "--think=false",
        "--hidethinking",
        model.as_str(),
        prompt_text.as_str(),
    ];
    if let Some(title) = run_command("ollama", &quiet, "", prompt) {
        return Some(title);
    }
    run_command(
        "ollama",
        &[
            "run",
            "--nowordwrap",
            "--think=false",
            model.as_str(),
            prompt_text.as_str(),
        ],
        "",
        prompt,
    )
}

fn llama_binary() -> Option<String> {
    for name in ["llama-cli", "llama-completion", "llama-run"] {
        let Ok(status) = Command::new(name)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        else {
            continue;
        };
        if status.success() {
            return Some(name.to_owned());
        }
    }
    None
}

fn small_text_gguf(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<(u64, PathBuf)> = Vec::new();
    collect_gguf(dir, 0, &mut found);
    found.sort_by_key(|(bytes, _)| *bytes);
    found
        .into_iter()
        .map(|(_, path)| path)
        .find(|path| !image_weight(path))
}

fn collect_gguf(dir: &Path, depth: u8, found: &mut Vec<(u64, PathBuf)>) {
    if depth > 4 || found.len() >= 24 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            collect_gguf(&path, depth + 1, found);
            continue;
        }
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
        {
            let bytes = std::fs::metadata(&path).map_or(u64::MAX, |meta| meta.len());
            if bytes > 0 && bytes < 2_000_000_000 {
                found.push((bytes, path));
            }
        }
    }
}

fn image_weight(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    [
        "flux",
        "sdxl",
        "sd3",
        "unet",
        "vae",
        "qwen-image",
        "qwen_image",
        "stable-diffusion",
    ]
    .iter()
    .any(|needle| name.contains(needle))
}

fn ollama_model(list: &str) -> Option<String> {
    list.lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .map(ToOwned::to_owned)
        .next()
}

fn instruction(prompt: &str) -> String {
    format!(
        "Write a 3-6 word image title. Do not copy the prompt. Reply with the title only.\n{prompt}"
    )
}

fn run_command(bin: &str, args: &[&str], stdin_text: &str, prompt: &str) -> Option<String> {
    let mut child = Command::new(bin);
    child
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if !stdin_text.is_empty() {
        child.stdin(Stdio::piped());
    }
    let mut spawned = child.spawn().ok()?;
    if !stdin_text.is_empty()
        && let Some(mut stdin) = spawned.stdin.take()
    {
        let _ = stdin.write_all(stdin_text.as_bytes());
    }
    let mut stdout = spawned.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut raw = String::new();
        let _ = stdout.read_to_string(&mut raw);
        raw
    });
    let Some(status) = wait_limited(&mut spawned, Duration::from_secs(25)) else {
        let _ = spawned.kill();
        let _ = spawned.wait();
        let _ = reader.join();
        return None;
    };
    if !status.success() {
        let _ = reader.join();
        return None;
    }
    let raw = reader.join().ok()?;
    clean_title(&raw, prompt)
}

fn wait_limited(child: &mut Child, limit: Duration) -> Option<std::process::ExitStatus> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if started.elapsed() >= limit => return None,
            Ok(None) => std::thread::sleep(Duration::from_millis(40)),
            Err(_) => return None,
        }
    }
}

fn strip_think(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "<think>".len()..];
        if let Some(end) = after.find("</think>") {
            rest = &after[end + "</think>".len()..];
        } else {
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn is_noise_line(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    lower.is_empty()
        || lower.starts_with("título")
        || lower.starts_with("thinking")
        || lower.starts_with("pensando")
        || matches!(lower.as_str(), "think" | "reasoning" | "wait" | "hmm")
}

fn kept_title(title: &str, prompt: &str) -> bool {
    usable_title(title) && !echoes_prompt(title, prompt)
}

fn usable_title(title: &str) -> bool {
    let trimmed = title.trim();
    !trimmed.is_empty() && !is_noise_line(trimmed)
}

fn echoes_prompt(title: &str, prompt: &str) -> bool {
    let title = title.trim().to_ascii_lowercase();
    let prompt = prompt.trim().to_ascii_lowercase();
    if title.chars().count() < 16 {
        return false;
    }
    prompt.starts_with(&title) || title.starts_with(&prompt)
}

fn clip_words(title: &str, max: usize) -> String {
    let words: Vec<&str> = title.split_whitespace().collect();
    if words.len() <= max {
        return title.trim().to_owned();
    }
    words.into_iter().take(max).collect::<Vec<_>>().join(" ")
}

fn heuristic_title(prompt: &str) -> Option<String> {
    let first = prompt.split(',').next()?.trim();
    let words = first.split_whitespace().count();
    if !(2..=TITLE_WORDS).contains(&words) {
        return None;
    }
    let chars = first.chars().count();
    if !(8..=40).contains(&chars) {
        return None;
    }
    kept_title(first, prompt).then(|| first.to_owned())
}

fn clean_title(raw: &str, prompt: &str) -> Option<String> {
    let stripped = strip_think(raw);
    let line = stripped
        .lines()
        .map(str::trim)
        .find(|line| !is_noise_line(line))?;
    let trimmed = line.trim_matches(|mark| mark == '"' || mark == '\'' || mark == '.');
    let mut title = clip_words(&trimmed.replace('\n', " "), TITLE_WORDS);
    if title.chars().count() > TITLE_LIMIT {
        title = title.chars().take(TITLE_LIMIT).collect();
    }
    let title = title.trim().to_owned();
    kept_title(&title, prompt).then_some(title)
}

#[cfg(test)]
mod tests {
    use super::{clean_title, heuristic_title, image_weight, ollama_model};
    use std::path::Path;

    #[test]
    fn o_titulo_fica_curto_e_sem_aspas() {
        assert_eq!(
            clean_title("  \"gato no telhado.\"  \n", "um gato"),
            Some("gato no telhado".to_owned())
        );
        assert_eq!(clean_title("\n\n", "um gato"), None);
    }

    #[test]
    fn ignora_o_raciocinio_e_pega_o_titulo() {
        assert_eq!(
            clean_title("Thinking\ngato no telhado\n", "um gato"),
            Some("gato no telhado".to_owned())
        );
        assert_eq!(
            clean_title(
                "<think>\nlongo raciocínio\n</think>\nfelino no telhado\n",
                "um gato"
            ),
            Some("felino no telhado".to_owned())
        );
        assert_eq!(clean_title("Thinking\n", "um gato"), None);
        assert_eq!(clean_title("Thinking...\n", "um gato"), None);
    }

    #[test]
    fn recusa_o_prompt_copiado() {
        let prompt = "Female mage, age 21, cheerful expression, Bright yellow eyes";
        assert_eq!(clean_title(prompt, prompt), None);
        assert_eq!(heuristic_title(prompt).as_deref(), Some("Female mage"));
        assert_eq!(heuristic_title("um gato"), None);
    }

    #[test]
    fn o_gguf_de_imagem_nao_serve_de_titulo() {
        assert!(image_weight(Path::new("/tmp/qwen-image-2.1-Q4.gguf")));
        assert!(!image_weight(Path::new("/tmp/qwen2-0.5b-instruct.gguf")));
    }

    #[test]
    fn o_ollama_usa_o_primeiro_modelo_da_lista() {
        let list = "NAME\tID\nllama3.2:1b\tabc\nqwen2.5:0.5b\tdef\n";
        assert_eq!(ollama_model(list).as_deref(), Some("llama3.2:1b"));
        assert_eq!(ollama_model("NAME\n"), None);
    }
}
