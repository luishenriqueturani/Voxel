//! Memória principal a partir de `/proc/meminfo`. Os valores do arquivo estão em KiB.

use crate::report::Memory;

#[must_use]
pub fn parse_meminfo(text: &str) -> Option<Memory> {
    let total = kib(text, "MemTotal")?;
    let available = kib(text, "MemAvailable")?;
    Some(Memory {
        total_bytes: total.saturating_mul(1024),
        available_bytes: available.saturating_mul(1024),
    })
}

fn kib(text: &str, key: &str) -> Option<u64> {
    for line in text.lines() {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        if name == key {
            let number = rest.split_whitespace().next()?;
            return number.parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_meminfo;

    #[test]
    fn converte_kib_para_bytes() {
        let memory = parse_meminfo("MemTotal:       1024 kB\nMemAvailable:   512 kB\n").unwrap();
        assert_eq!(memory.total_bytes, 1024 * 1024);
        assert_eq!(memory.available_bytes, 512 * 1024);
    }

    #[test]
    fn sem_memavailable_nao_chuta() {
        assert!(parse_meminfo("MemTotal: 1024 kB\n").is_none());
    }
}
