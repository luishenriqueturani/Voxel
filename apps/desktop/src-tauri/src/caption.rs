//! Título curto do prompt recente. A imagem não espera este passo.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::db::Database;
use crate::paths::AppPaths;

const TITLE_LIMIT: usize = 60;

/// Gera o título em segundo plano. Falha silenciosa: o prompt continua como está.
pub fn schedule(database: &Database, paths: &AppPaths, prompt: &str) {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return;
    }
    if database
        .prompt_title(prompt)
        .ok()
        .flatten()
        .is_some_and(|title| !title.is_empty())
    {
        return;
    }
    let database = database.clone();
    let prompt = prompt.to_owned();
    let models = crate::models::folder(paths, &database);
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(title) = caption_of(&prompt, &models) {
            let _ = database.set_prompt_title(&prompt, &title);
        }
    });
}

fn caption_of(prompt: &str, models: &Path) -> Option<String> {
    if let Ok(command) = std::env::var("VOXEL_CAPTION_CMD") {
        return run_command(&command, &[], prompt);
    }
    if let Some(title) = caption_gguf(prompt, models) {
        return Some(title);
    }
    caption_ollama(prompt)
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
    run_command(
        "ollama",
        &["run", "--nowordwrap", &model, &instruction(prompt)],
        "",
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
    format!("Resuma em até 6 palavras, sem aspas e sem ponto final:\n{prompt}\nTítulo:")
}

fn run_command(bin: &str, args: &[&str], stdin_text: &str) -> Option<String> {
    let mut child = Command::new(bin);
    child
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if stdin_text.is_empty() {
        let output = child.output().ok()?;
        if !output.status.success() {
            return None;
        }
        return clean_title(&String::from_utf8_lossy(&output.stdout));
    }
    child.stdin(Stdio::piped());
    let mut spawned = child.spawn().ok()?;
    if let Some(mut stdin) = spawned.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(stdin_text.as_bytes());
    }
    let output = spawned.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    clean_title(&String::from_utf8_lossy(&output.stdout))
}

fn clean_title(raw: &str) -> Option<String> {
    let line = raw
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("Título"))?;
    let trimmed = line.trim_matches(|mark| mark == '"' || mark == '\'' || mark == '.');
    let mut title = trimmed.replace('\n', " ");
    if title.chars().count() > TITLE_LIMIT {
        title = title.chars().take(TITLE_LIMIT).collect();
    }
    let title = title.trim().to_owned();
    if title.is_empty() { None } else { Some(title) }
}

#[cfg(test)]
mod tests {
    use super::{clean_title, image_weight, ollama_model};
    use std::path::Path;

    #[test]
    fn o_titulo_fica_curto_e_sem_aspas() {
        assert_eq!(
            clean_title("  \"gato no telhado.\"  \n"),
            Some("gato no telhado".to_owned())
        );
        assert_eq!(clean_title("\n\n"), None);
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
