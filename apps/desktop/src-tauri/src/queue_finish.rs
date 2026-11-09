//! Ação depois que a fila termina sozinha: nada, notificar, suspender ou desligar.

use std::process::Command;

use crate::db::{Database, Language, QueueFinish};

/// A ação só roda quando o último job concluiu sozinho.
#[must_use]
pub fn should_apply(status: Option<&str>) -> bool {
    status == Some("done")
}

/// Notifica o relatório e, se configurado, suspende ou desliga.
pub fn apply_finish(database: &Database) {
    let Ok(status) = database.latest_queue_status() else {
        return;
    };
    if !should_apply(status.as_deref()) {
        return;
    }
    let Ok(Some(job)) = database.latest_queue_job() else {
        return;
    };
    let language = database
        .settings()
        .ok()
        .and_then(|settings| settings.language);
    let english = language == Some(Language::English);
    let body = Database::queue_notify_body(&job, english);
    let _ = Command::new("notify-send")
        .args(["--app-name=Voxel", "Voxel", &body])
        .spawn();
    let Ok(action) = database.queue_finish() else {
        return;
    };
    if action == QueueFinish::Notify {
        return;
    }
    if let Some((bin, args)) = finish_command(action, language) {
        let _ = Command::new(bin).args(args).spawn();
    }
}

/// Comando do sistema para a ação, se houver.
#[must_use]
pub fn finish_command(
    action: QueueFinish,
    language: Option<Language>,
) -> Option<(&'static str, Vec<String>)> {
    match action {
        QueueFinish::None => None,
        QueueFinish::Notify => {
            let body = match language {
                Some(Language::English) => "The execution queue has finished.",
                _ => "A fila de execução terminou.",
            };
            Some((
                "notify-send",
                vec![
                    "--app-name=Voxel".to_owned(),
                    "Voxel".to_owned(),
                    body.to_owned(),
                ],
            ))
        }
        QueueFinish::Suspend => Some(("systemctl", vec!["suspend".to_owned()])),
        QueueFinish::PowerOff => Some(("systemctl", vec!["poweroff".to_owned()])),
    }
}

#[cfg(test)]
mod tests {
    use super::{finish_command, should_apply};
    use crate::db::{Language, QueueFinish};

    #[test]
    fn so_concluido_dispara_a_acao() {
        assert!(should_apply(Some("done")));
        assert!(!should_apply(Some("cancelled")));
        assert!(!should_apply(Some("paused")));
        assert!(!should_apply(None));
    }

    #[test]
    fn nada_nao_dispara_comando() {
        assert!(finish_command(QueueFinish::None, None).is_none());
    }

    #[test]
    fn notificar_usa_o_idioma() {
        let (bin, args) = finish_command(QueueFinish::Notify, Some(Language::English)).unwrap();
        assert_eq!(bin, "notify-send");
        assert!(args.iter().any(|arg| arg.contains("finished")));
        let (_, pt) = finish_command(QueueFinish::Notify, None).unwrap();
        assert!(pt.iter().any(|arg| arg.contains("terminou")));
    }

    #[test]
    fn suspender_e_desligar_chamam_o_systemctl() {
        assert_eq!(
            finish_command(QueueFinish::Suspend, None).unwrap().1,
            ["suspend"]
        );
        assert_eq!(
            finish_command(QueueFinish::PowerOff, None).unwrap().1,
            ["poweroff"]
        );
    }
}
