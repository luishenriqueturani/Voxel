//! Totais do relatório: concluídos, falhos, cancelados e tempo.

use super::types::{QueueItem, QueueJob, QueueSummary};

impl crate::db::Database {
    /// Texto da notificação do sistema ao terminar.
    #[must_use]
    pub fn queue_notify_body(job: &QueueJob, english: bool) -> String {
        notify_body(&summarize(job), english)
    }
}

/// Conta itens e o tempo entre criação e última atualização.
#[must_use]
pub fn summarize(job: &QueueJob) -> QueueSummary {
    let mut done = 0;
    let mut failed = 0;
    let mut cancelled = 0;
    let mut later = 0;
    for item in &job.items {
        match item.status.as_str() {
            "failed" => failed += 1,
            "cancelled" => cancelled += 1,
            "done" => {
                done += 1;
                if waiting_later(item) {
                    later += 1;
                }
            }
            _ => {}
        }
    }
    QueueSummary {
        name: job.name.clone(),
        status: job.status.clone(),
        elapsed_secs: elapsed_secs(&job.created_at, &job.updated_at),
        done,
        failed,
        cancelled,
        later,
    }
}

/// Item no modo escolher depois, ainda sem malha.
#[must_use]
pub fn waiting_later(item: &QueueItem) -> bool {
    item.mesh == "later" && item.status == "done" && !item.image_paths.is_empty()
}

/// Texto da notificação do relatório.
#[must_use]
pub fn notify_body(summary: &QueueSummary, english: bool) -> String {
    let time = format_elapsed(summary.elapsed_secs);
    if english {
        if summary.failed == 0 {
            format!("{}: {} done, {time}", summary.name, summary.done)
        } else {
            format!(
                "{}: {} done, {} failed, {time}",
                summary.name, summary.done, summary.failed
            )
        }
    } else if summary.failed == 0 {
        format!("{}: {} concluídos, {time}", summary.name, summary.done)
    } else {
        format!(
            "{}: {} concluídos, {} falhos, {time}",
            summary.name, summary.done, summary.failed
        )
    }
}

fn format_elapsed(secs: i64) -> String {
    let hours = secs / 3_600;
    let minutes = (secs % 3_600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}min")
    } else {
        format!("{minutes} min")
    }
}

fn elapsed_secs(from: &str, to: &str) -> i64 {
    match (unix_from_utc(from), unix_from_utc(to)) {
        (Some(start), Some(end)) => end.saturating_sub(start).max(0),
        _ => 0,
    }
}

fn unix_from_utc(value: &str) -> Option<i64> {
    if value.len() != 20 || !value.ends_with('Z') || value.as_bytes().get(10) != Some(&b'T') {
        return None;
    }
    let year: i64 = value.get(0..4)?.parse().ok()?;
    let month: i64 = value.get(5..7)?.parse().ok()?;
    let day: i64 = value.get(8..10)?.parse().ok()?;
    let hour: i64 = value.get(11..13)?.parse().ok()?;
    let minute: i64 = value.get(14..16)?.parse().ok()?;
    let second: i64 = value.get(17..19)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::{notify_body, summarize, unix_from_utc, waiting_later};
    use crate::db::queue::{QueueItem, QueueJob, QueueResolved};

    fn item(status: &str, mesh: &str, paths: &[&str]) -> QueueItem {
        QueueItem {
            id: status.to_owned(),
            status: status.to_owned(),
            prompt: status.to_owned(),
            count: 1,
            mesh: mesh.to_owned(),
            images: i64::from(!paths.is_empty()),
            meshes: 0,
            resolved: QueueResolved {
                prompt: status.to_owned(),
                negative: String::new(),
                model_2d: "sd15".to_owned(),
                model_3d: "triposr".to_owned(),
                resolution: 512,
                aspect: "1:1".to_owned(),
                steps: 20,
                cfg: 7.0,
                seed: String::new(),
                postprocess: false,
                count: 1,
                mesh: mesh.to_owned(),
            },
            error_log: None,
            error_summary: None,
            image_paths: paths.iter().map(|path| (*path).to_owned()).collect(),
        }
    }

    #[test]
    fn conta_falhos_e_os_que_esperam_escolha() {
        let job = QueueJob {
            id: "j".to_owned(),
            name: "NPCs".to_owned(),
            status: "done".to_owned(),
            end_retry: false,
            created_at: "2026-11-09T10:00:00Z".to_owned(),
            updated_at: "2026-11-09T12:10:00Z".to_owned(),
            items: vec![
                item("done", "later", &["/a.png"]),
                item("failed", "all", &[]),
                item("done", "none", &["/b.png"]),
                item("cancelled", "none", &[]),
            ],
        };
        let summary = summarize(&job);
        assert_eq!(summary.done, 2);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.cancelled, 1);
        assert_eq!(summary.later, 1);
        assert_eq!(summary.elapsed_secs, 7_800);
        assert!(waiting_later(&job.items[0]));
        assert!(!waiting_later(&job.items[2]));
        assert!(notify_body(&summary, false).contains("falhos"));
        assert!(notify_body(&summary, true).contains("failed"));
        assert!(notify_body(&summary, false).contains("2h 10min"));
    }

    #[test]
    fn instante_utc_vira_unix() {
        assert_eq!(unix_from_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(unix_from_utc("agora"), None);
    }
}
