use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::Json;
use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use tokio::sync::Notify;
use voxel_protocol::{
    Ack, CancelRequest, CompletedEvent, CompletedName, DecodeLatentsRequest, ErrorBody,
    FailedEvent, FailedName, Health, JobOutput, LoadModelRequest, ProgressEvent, ProgressName,
    Txt2ImgRequest, paths,
};

use crate::run::{self, Running};

#[derive(Clone)]
struct AppState {
    authorization: Arc<String>,
    cli: PathBuf,
    backend: String,
    model: Arc<Mutex<Option<PathBuf>>>,
    jobs: Arc<Mutex<HashMap<String, Arc<Job>>>>,
}

struct Job {
    events: Mutex<Vec<serde_json::Value>>,
    done: Mutex<bool>,
    running: Arc<Running>,
    notify: Notify,
}

#[derive(Deserialize)]
struct EventQuery {
    #[serde(rename = "jobId")]
    job_id: String,
}

/// Sobe o HTTP do worker.
///
/// # Errors
///
/// Falha se o servidor não conseguir aceitar conexões.
pub async fn serve(
    listener: tokio::net::TcpListener,
    token: String,
    cli: PathBuf,
    backend: String,
) -> Result<(), std::io::Error> {
    let state = AppState {
        authorization: Arc::new(format!("Bearer {token}")),
        cli,
        backend,
        model: Arc::new(Mutex::new(None)),
        jobs: Arc::new(Mutex::new(HashMap::new())),
    };
    axum::serve(listener, router(state))
        .await
        .map_err(std::io::Error::other)
}

fn router(state: AppState) -> Router {
    Router::new()
        .route(paths::HEALTH, get(health))
        .route(paths::MODELS_LOAD, post(load_model))
        .route(paths::MODELS_UNLOAD, post(unload_model))
        .route(paths::JOBS_CANCEL, post(cancel_job))
        .route(paths::IMAGES_TXT2IMG, post(txt2img))
        .route(paths::IMAGES_IMG2IMG, post(unavailable))
        .route(paths::IMAGES_INPAINT, post(unavailable))
        .route(paths::IMAGES_DECODE_LATENTS, post(decode_latents))
        .route(paths::EVENTS, get(events))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_token))
        .with_state(state)
}

async fn require_token(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if presented == Some(state.authorization.as_str()) {
        next.run(request).await
    } else {
        reject(StatusCode::UNAUTHORIZED, "token recusado")
    }
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".to_owned(),
    })
}

async fn load_model(State(state): State<AppState>, Json(body): Json<LoadModelRequest>) -> Response {
    let path = PathBuf::from(body.model_path);
    if !path.is_file() {
        return reject(StatusCode::BAD_REQUEST, "modelo ausente");
    }
    *state.model.lock().unwrap_or_else(PoisonError::into_inner) = Some(path);
    Json(Ack { accepted: true }).into_response()
}

async fn unload_model(State(state): State<AppState>) -> Json<Ack> {
    *state.model.lock().unwrap_or_else(PoisonError::into_inner) = None;
    Json(Ack { accepted: true })
}

async fn cancel_job(State(state): State<AppState>, Json(body): Json<CancelRequest>) -> Json<Ack> {
    if let Some(job) = state
        .jobs
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&body.job_id)
    {
        job.running.cancel();
    }
    Json(Ack { accepted: true })
}

async fn unavailable() -> Response {
    reject(StatusCode::NOT_IMPLEMENTED, "ainda não neste worker")
}

async fn txt2img(State(state): State<AppState>, Json(body): Json<Txt2ImgRequest>) -> Response {
    let Some(model) = state
        .model
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
    else {
        return reject(StatusCode::CONFLICT, "modelo não carregado");
    };
    let job = Arc::new(Job {
        events: Mutex::new(Vec::new()),
        done: Mutex::new(false),
        running: Arc::new(Running::default()),
        notify: Notify::new(),
    });
    let job_id = body.job_id.clone();
    state
        .jobs
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(job_id.clone(), Arc::clone(&job));
    let cli = state.cli.clone();
    let backend = state.backend.clone();
    let watched = Arc::clone(&job);
    let report_id = job_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        run::txt2img(
            &cli,
            &model,
            &backend,
            &body,
            &watched.running,
            |step, total, preview| {
                watched.push(progress(&report_id, step, total, preview.as_ref()));
            },
        )
    })
    .await;
    match result {
        Ok(Ok(path)) => {
            job.push(completed(&job_id, &path));
            job.finish();
            Json(JobOutput {
                job_id,
                path: path.display().to_string(),
            })
            .into_response()
        }
        Ok(Err(message)) => {
            job.push(failed_event(&job_id, &message));
            job.finish();
            reject(StatusCode::INTERNAL_SERVER_ERROR, &message)
        }
        Err(join_error) => reject(StatusCode::INTERNAL_SERVER_ERROR, &join_error.to_string()),
    }
}

async fn decode_latents(
    State(state): State<AppState>,
    Json(body): Json<DecodeLatentsRequest>,
) -> Response {
    let Some(model) = state
        .model
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
    else {
        return reject(StatusCode::CONFLICT, "modelo não carregado");
    };
    let latent = PathBuf::from(&body.latents_path);
    if !latent.is_file() {
        return reject(StatusCode::BAD_REQUEST, "latente ausente");
    }
    let output = PathBuf::from(&body.output_path);
    let cli = state.cli.clone();
    let backend = state.backend.clone();
    let result = tokio::task::spawn_blocking(move || {
        let device = run::listed_device(&cli, &backend)?;
        run::decode_saved(&cli, &model, &output, &device, &latent, &Running::default())
    })
    .await;
    match result {
        Ok(Ok(path)) => Json(JobOutput {
            job_id: body.job_id,
            path: path.display().to_string(),
        })
        .into_response(),
        Ok(Err(message)) => reject(StatusCode::INTERNAL_SERVER_ERROR, &message),
        Err(join_error) => reject(StatusCode::INTERNAL_SERVER_ERROR, &join_error.to_string()),
    }
}

async fn events(
    State(state): State<AppState>,
    Query(query): Query<EventQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| replay(socket, state, query.job_id))
}

async fn replay(mut socket: WebSocket, state: AppState, job_id: String) {
    let mut sent = 0;
    let appear_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let (events, done, exists) = job_snapshot(&state, &job_id);
        while sent < events.len() {
            let Ok(text) = serde_json::to_string(&events[sent]) else {
                return;
            };
            if socket.send(Message::Text(text.into())).await.is_err() {
                return;
            }
            sent += 1;
        }
        if done || (!exists && Instant::now() >= appear_deadline) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let _ = socket.send(Message::Close(None)).await;
}

fn job_snapshot(state: &AppState, job_id: &str) -> (Vec<serde_json::Value>, bool, bool) {
    let jobs = state.jobs.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(job) = jobs.get(job_id) else {
        return (Vec::new(), false, false);
    };
    (
        job.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone(),
        *job.done.lock().unwrap_or_else(PoisonError::into_inner),
        true,
    )
}

impl Job {
    fn push(&self, event: serde_json::Value) {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
        self.notify.notify_waiters();
    }

    fn finish(&self) {
        *self.done.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.notify.notify_waiters();
    }
}

fn progress(job_id: &str, step: i64, total: i64, preview: Option<&PathBuf>) -> serde_json::Value {
    serde_json::to_value(ProgressEvent {
        event: ProgressName::Progress,
        job_id: job_id.to_owned(),
        step,
        total,
        preview_path: preview.map(|path| path.display().to_string()),
    })
    .unwrap_or(serde_json::Value::Null)
}

fn completed(job_id: &str, path: &std::path::Path) -> serde_json::Value {
    serde_json::to_value(CompletedEvent {
        event: CompletedName::Completed,
        job_id: job_id.to_owned(),
        path: path.display().to_string(),
    })
    .unwrap_or(serde_json::Value::Null)
}

fn failed_event(job_id: &str, message: &str) -> serde_json::Value {
    serde_json::to_value(FailedEvent {
        event: FailedName::Failed,
        job_id: job_id.to_owned(),
        message: message.to_owned(),
    })
    .unwrap_or(serde_json::Value::Null)
}

fn reject(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(ErrorBody {
            message: message.to_owned(),
        }),
    )
        .into_response()
}
