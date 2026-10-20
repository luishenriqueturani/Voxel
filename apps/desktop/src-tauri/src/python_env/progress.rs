//! Lê o tamanho anunciado pelo `uv` e soma o que já chegou no cache.

use std::collections::HashSet;
use std::path::Path;

/// Conta os downloads anunciados e os que o `uv` já concluiu.
#[derive(Debug, Default)]
pub struct Watch {
    total: u64,
    completed: u64,
    seen: HashSet<String>,
    sizes: Vec<(String, u64)>,
}

impl Watch {
    #[must_use]
    pub fn total(&self) -> u64 {
        self.total
    }

    #[must_use]
    pub fn completed(&self) -> u64 {
        self.completed
    }

    pub fn note(&mut self, line: &str) {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Downloading ") {
            self.note_start(rest);
        } else if let Some(rest) = trimmed.strip_prefix("Downloaded ") {
            self.note_done(rest);
        }
    }

    fn note_start(&mut self, rest: &str) {
        let Some((name, size)) = rest.rsplit_once(" (") else {
            return;
        };
        let Some(size) = size.strip_suffix(')') else {
            return;
        };
        let Some(bytes) = parse_bytes(size) else {
            return;
        };
        if self.seen.insert(name.to_owned()) {
            self.sizes.push((name.to_owned(), bytes));
            self.total = self.total.saturating_add(bytes);
        }
    }

    fn note_done(&mut self, name: &str) {
        if let Some((_, bytes)) = self.sizes.iter().find(|(known, _)| known == name) {
            self.completed = self.completed.saturating_add(*bytes);
        }
    }
}

#[must_use]
pub fn percent(received: u64, total: u64) -> Option<u8> {
    if total == 0 {
        return None;
    }
    let value = received.saturating_mul(100) / total;
    u8::try_from(value.min(100)).ok()
}

#[must_use]
pub fn dir_size(path: &Path) -> u64 {
    let mut total: u64 = 0;
    let mut pending = vec![path.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                pending.push(entry.path());
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

fn parse_bytes(token: &str) -> Option<u64> {
    let split = token.find(|symbol: char| symbol.is_ascii_alphabetic())?;
    let (number, unit) = token.split_at(split);
    let factor = match unit {
        "B" => 1,
        "KiB" => 1024,
        "MiB" => 1024 * 1024,
        "GiB" => 1024 * 1024 * 1024,
        _ => return None,
    };
    scaled(number, factor)
}

fn scaled(number: &str, factor: u64) -> Option<u64> {
    let mut parts = number.split('.');
    let whole: u64 = parts.next()?.parse().ok()?;
    let fraction = parts.next().unwrap_or("0");
    if parts.next().is_some() || fraction.len() > 6 {
        return None;
    }
    let digits = u32::try_from(fraction.len()).ok()?;
    let fraction: u64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().ok()?
    };
    let denom = 10u64.pow(digits);
    Some(
        whole
            .saturating_mul(factor)
            .saturating_add(factor.saturating_mul(fraction) / denom.max(1)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soma_o_anuncio_do_uv_e_marca_o_que_terminou() {
        let mut watch = Watch::default();
        watch.note("Downloading cpython-3.12.15-linux-x86_64-gnu (download) (32.7MiB)");
        watch.note("Downloading torch (1.5GiB)");
        watch.note("Downloading torch (1.5GiB)");
        assert_eq!(
            watch.total(),
            scaled("32.7", 1024 * 1024).unwrap() + scaled("1.5", 1024 * 1024 * 1024).unwrap()
        );
        watch.note(" Downloaded cpython-3.12.15-linux-x86_64-gnu (download)");
        assert_eq!(watch.completed(), scaled("32.7", 1024 * 1024).unwrap());
        assert_eq!(percent(45, 100), Some(45));
        assert_eq!(percent(0, 0), None);
    }

    #[test]
    fn le_kib_mib_e_gib() {
        assert_eq!(parse_bytes("512KiB"), Some(512 * 1024));
        assert_eq!(parse_bytes("32.7MiB"), Some(34_288_435));
        assert_eq!(parse_bytes("2GiB"), Some(2 * 1024 * 1024 * 1024));
        assert_eq!(parse_bytes("quase"), None);
    }
}
