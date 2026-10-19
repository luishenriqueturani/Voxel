#![allow(clippy::unused_async)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use voxel_protocol::{
    Ack, CancelRequest, CompletedEvent, CompletedName, DecodeLatentsRequest, ErrorBody,
    FailedEvent, FailedName, Health, ImageToMeshRequest, Img2ImgRequest, InpaintRequest, JobOutput,
    LoadModelRequest, MeshPostProcessRequest, ProgressEvent, ProgressName, RemoveBackgroundRequest,
    Txt2ImgRequest, paths,
};

use crate::sample::{PNG, minimal_glb};

#[derive(Clone)]
struct AppState {
    authorization: Arc<String>,
    scenario: Arc<String>,
    jobs: Arc<Mutex<HashMap<String, Vec<serde_json::Value>>>>,
}

#[derive(Deserialize)]
struct EventQuery {
    #[serde(rename = "jobId")]
    job_id: String,
}

pub async fn serve(
    listener: tokio::net::TcpListener,
    token: String,
    scenario: String,
) -> Result<(), std::io::Error> {
    let state = AppState {
        authorization: Arc::new(format!("Bearer {token}")),
        scenario: Arc::new(scenario),
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
        .route(paths::IMAGES_IMG2IMG, post(img2img))
        .route(paths::IMAGES_INPAINT, post(inpaint))
        .route(paths::IMAGES_DECODE_LATENTS, post(decode_latents))
        .route(paths::IMAGES_REMOVE_BACKGROUND, post(remove_background))
        .route(paths::MESHES_FROM_IMAGE, post(image_to_mesh))
        .route(paths::MESHES_POST_PROCESS, post(post_process))
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
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorBody {
                message: "token recusado".to_owned(),
            }),
        )
            .into_response()
    }
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".to_owned(),
    })
}

async fn load_model(Json(_body): Json<LoadModelRequest>) -> Json<Ack> {
    accepted()
}

async fn unload_model() -> Json<Ack> {
    accepted()
}

async fn cancel_job(Json(_body): Json<CancelRequest>) -> Json<Ack> {
    accepted()
}

fn accepted() -> Json<Ack> {
    Json(Ack { accepted: true })
}

async fn txt2img(State(state): State<AppState>, Json(body): Json<Txt2ImgRequest>) -> Response {
    write_output(&state, &body.job_id, &body.output_path, PNG)
}

async fn img2img(State(state): State<AppState>, Json(body): Json<Img2ImgRequest>) -> Response {
    write_output(&state, &body.job_id, &body.output_path, PNG)
}

async fn inpaint(State(state): State<AppState>, Json(body): Json<InpaintRequest>) -> Response {
    write_output(&state, &body.job_id, &body.output_path, PNG)
}

async fn decode_latents(
    State(state): State<AppState>,
    Json(body): Json<DecodeLatentsRequest>,
) -> Response {
    write_output(&state, &body.job_id, &body.output_path, PNG)
}

async fn remove_background(
    State(state): State<AppState>,
    Json(body): Json<RemoveBackgroundRequest>,
) -> Response {
    write_output(&state, &body.job_id, &body.output_path, PNG)
}

async fn image_to_mesh(
    State(state): State<AppState>,
    Json(body): Json<ImageToMeshRequest>,
) -> Response {
    let mesh = minimal_glb();
    write_output(&state, &body.job_id, &body.output_path, &mesh)
}

async fn post_process(
    State(state): State<AppState>,
    Json(body): Json<MeshPostProcessRequest>,
) -> Response {
    let mesh = minimal_glb();
    write_output(&state, &body.job_id, &body.output_path, &mesh)
}

async fn events(
    State(state): State<AppState>,
    Query(query): Query<EventQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let stored = {
        let jobs = state.jobs.lock().unwrap_or_else(PoisonError::into_inner);
        jobs.get(&query.job_id).cloned().unwrap_or_default()
    };
    ws.on_upgrade(move |socket| replay(socket, stored))
}

async fn replay(mut socket: WebSocket, events: Vec<serde_json::Value>) {
    for event in events {
        let Ok(text) = serde_json::to_string(&event) else {
            return;
        };
        if socket.send(Message::Text(text.into())).await.is_err() {
            return;
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

fn write_output(state: &AppState, job_id: &str, output_path: &str, bytes: &[u8]) -> Response {
    scripted_failure(&state.scenario);
    if let Err(error) = std::fs::write(output_path, bytes) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                message: error.to_string(),
            }),
        )
            .into_response();
    }
    remember(state, job_id, output_path);
    Json(JobOutput {
        job_id: job_id.to_owned(),
        path: output_path.to_owned(),
    })
    .into_response()
}

fn scripted_failure(scenario: &str) {
    if scenario == "abort" {
        eprintln!("HSA_STATUS_ERROR_INVALID_ISA");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        std::process::abort();
    }
    if scenario == "oom" {
        eprintln!("CUDA out of memory");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        std::process::exit(1);
    }
}

fn remember(state: &AppState, job_id: &str, output_path: &str) {
    let events = if state.scenario.as_str() == "fail" {
        vec![to_json(FailedEvent {
            event: FailedName::Failed,
            job_id: job_id.to_owned(),
            message: "falha simulada".to_owned(),
        })]
    } else {
        vec![
            to_json(ProgressEvent {
                event: ProgressName::Progress,
                job_id: job_id.to_owned(),
                step: 1,
                total: 2,
                preview_path: None,
            }),
            to_json(ProgressEvent {
                event: ProgressName::Progress,
                job_id: job_id.to_owned(),
                step: 2,
                total: 2,
                preview_path: Some(output_path.to_owned()),
            }),
            to_json(CompletedEvent {
                event: CompletedName::Completed,
                job_id: job_id.to_owned(),
                path: output_path.to_owned(),
            }),
        ]
    };
    let mut jobs = state.jobs.lock().unwrap_or_else(PoisonError::into_inner);
    jobs.insert(job_id.to_owned(), events);
}

fn to_json(value: impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).expect("evento do contrato")
}
