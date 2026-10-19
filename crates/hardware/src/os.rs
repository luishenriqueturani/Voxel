//! Distribuição (`/etc/os-release`) e versão do módulo da NVIDIA.

use crate::report::OperatingSystem;

pub fn parse_os_release(text: &str) -> OperatingSystem {
    let name = field(text, "PRETTY_NAME").unwrap_or_else(|| "Linux".to_owned());
    let id = field(text, "ID").unwrap_or_default();
    let version = field(text, "VERSION_ID")
        .or_else(|| field(text, "BUILD_ID"))
        .unwrap_or_default();
    OperatingSystem {
        name,
        id,
        version,
        kernel: String::new(),
        drivers: Vec::new(),
    }
}

pub fn parse_nvidia_driver(text: &str) -> Option<String> {
    let line = text.lines().find(|line| line.contains("Kernel Module"))?;
    line.split_whitespace()
        .find(|token| is_version(token))
        .map(str::to_owned)
}

pub fn nvidia_driver_version() -> Option<String> {
    let text = std::fs::read_to_string("/proc/driver/nvidia/version").ok()?;
    parse_nvidia_driver(&text)
}

fn field(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name == key {
            return Some(unquote(value));
        }
    }
    None
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').trim().to_owned()
}

fn is_version(token: &str) -> bool {
    let mut parts = token.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    if first.is_empty() || !first.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }
    let mut extra = false;
    for part in parts {
        extra = true;
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
    }
    extra
}

#[cfg(test)]
mod tests {
    use super::{parse_nvidia_driver, parse_os_release};

    #[test]
    fn le_cachyos_sem_version_id() {
        let os = parse_os_release(
            "NAME=\"CachyOS Linux\"\nPRETTY_NAME=\"CachyOS\"\nID=cachyos\nBUILD_ID=rolling\n",
        );
        assert_eq!(os.name, "CachyOS");
        assert_eq!(os.id, "cachyos");
        assert_eq!(os.version, "rolling");
    }

    #[test]
    fn prefere_version_id() {
        let os = parse_os_release(
            "PRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nID=ubuntu\nVERSION_ID=\"24.04\"\nBUILD_ID=ignored\n",
        );
        assert_eq!(os.version, "24.04");
        assert_eq!(os.id, "ubuntu");
    }

    #[test]
    fn extrai_a_versao_do_modulo_nvidia() {
        let text = "NVRM version: NVIDIA UNIX x86_64 Kernel Module  570.86.16  Fri Jan 24 00:00:00 UTC 2025\n";
        assert_eq!(parse_nvidia_driver(text).as_deref(), Some("570.86.16"));
        assert_eq!(parse_nvidia_driver("sem modulo\n"), None);
    }
}
