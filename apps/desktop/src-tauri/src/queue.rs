//! Comandos da fila de execução dos fluxos.

use tauri::{AppHandle, Manager, State};

use crate::db::{Database, QueueJob};
use crate::live_job::LiveJobs;
use crate::queue_run::QueuePump;

fn notify(app: &AppHandle) {
    app.state::<QueuePump>().notify();
}

fn stop_live(app: &AppHandle) {
    app.state::<LiveJobs>().request_stop();
}

async fn on_db<T: Send + 'static>(
    database: State<'_, Database>,
    work: impl FnOnce(Database) -> Result<T, crate::db::DbError> + Send + 'static,
) -> Result<T, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || work(database).map_err(|error| error.to_string()))
        .await
        .map_err(|error| error.to_string())?
}

/// Enfileira o fluxo e acorda o worker. A fila retoma sozinha se o app fechar.
///
/// # Errors
///
/// Falha se o fluxo não existir ou se o banco recusar o job.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn enqueue_flow(
    app: AppHandle,
    database: State<'_, Database>,
    id: String,
) -> Result<QueueJob, String> {
    let job = on_db(database, move |database| database.enqueue_flow(&id)).await?;
    notify(&app);
    Ok(job)
}

/// Job ativo, se a fila ainda tiver trabalho (pausada não conta).
///
/// # Errors
///
/// Falha se o banco não puder ser lido.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn active_queue_job(database: State<'_, Database>) -> Result<Option<QueueJob>, String> {
    on_db(database, |database| database.active_queue_job()).await
}

/// Job aberto, inclusive pausado.
///
/// # Errors
///
/// Falha se o banco não puder ser lido.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn unfinished_queue_job(
    database: State<'_, Database>,
) -> Result<Option<QueueJob>, String> {
    on_db(database, |database| database.unfinished_queue_job()).await
}

/// Pausa a fila e interrompe o passo em andamento, sem marcar falha.
///
/// # Errors
///
/// Falha se não houver job aberto.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn pause_queue(
    app: AppHandle,
    database: State<'_, Database>,
) -> Result<QueueJob, String> {
    let job = on_db(database, |database| database.pause_queue()).await?;
    stop_live(&app);
    notify(&app);
    Ok(job)
}

/// Retoma a fila pausada.
///
/// # Errors
///
/// Falha se a fila não estiver pausada.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn resume_queue(
    app: AppHandle,
    database: State<'_, Database>,
) -> Result<QueueJob, String> {
    let job = on_db(database, |database| database.resume_queue()).await?;
    notify(&app);
    Ok(job)
}

/// Cancela os jobs abertos e os itens que ainda não terminaram.
///
/// # Errors
///
/// Falha se o banco recusar a escrita.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn cancel_queue(app: AppHandle, database: State<'_, Database>) -> Result<(), String> {
    on_db(database, |database| database.cancel_queue()).await?;
    stop_live(&app);
    notify(&app);
    Ok(())
}

/// Reordena só os itens pendentes do job.
///
/// # Errors
///
/// Falha se a ordem for inválida ou se o job tiver encerrado.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn reorder_queue_items(
    app: AppHandle,
    database: State<'_, Database>,
    job_id: String,
    pending_ids: Vec<String>,
) -> Result<QueueJob, String> {
    let job = on_db(database, move |database| {
        database.reorder_queue_items(&job_id, &pending_ids)
    })
    .await?;
    notify(&app);
    Ok(job)
}
