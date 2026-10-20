use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde::de::DeserializeOwned;
use voxel_logging::{DEFAULT_LINE_LIMIT, ProcessOutcome, RunningProcess};

use crate::error::WorkerError;
use crate::generated::{
    Ack, CancelRequest, CompletedEvent, DecodeLatentsRequest, FailedEvent, Health,
    ImageToMeshRequest, Img2ImgRequest, InpaintRequest, JobOutput, LoadModelRequest,
    MeshPostProcessRequest, ProgressEvent, RemoveBackgroundRequest, Txt2ImgRequest, paths,
};
use crate::traits::{BackgroundRemover, ImageBackend, MeshBackend, MeshPostProcess};

const STARTUP_LIMIT: Duration = Duration::from_secs(2);
const POLL_PAUSE: Duration = Duration::from_millis(20);
const READY_PREFIX: &str = "VOXEL_READY ";
/// Uma imagem grande, com muitos passos, passa dos três minutos. O pedido espera até aqui.
const JOB_LIMIT: Duration = Duration::from_hours(1);
const WATCH_SLICE: Duration = Duration::from_millis(200);

/// Evento lido do WebSocket de progresso.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerEvent {
    Progress(ProgressEvent),
    Completed(CompletedEvent),
    Failed(FailedEvent),
}

/// Processo de worker supervisionado pelo core.
///
/// Sobe o programa, espera `VOXEL_READY`, autentica com um token novo e
/// recomeça o processo quando [`Self::restart`] é chamado. Ao sair de escopo, o filho é encerrado.
pub struct WorkerProcess {
    process: RunningProcess,
    port: u16,
    token: String,
    client: reqwest::blocking::Client,
    program: PathBuf,
    scenario: String,
}

impl WorkerProcess {
    /// Inicia `program` com o cenário pedido (`ok`, `abort`, `oom` ou `fail`).
    ///
    /// # Errors
    ///
    /// Falha se o processo não subir, não anunciar a porta ou não responder no `/health`.
    pub fn spawn(program: impl AsRef<Path>, scenario: &str) -> Result<Self, WorkerError> {
        Self::spawn_with(program, scenario, &[], Duration::from_secs(3))
    }

    /// Inicia o worker com variáveis extras e um limite próprio para o pedido HTTP.
    ///
    /// # Errors
    ///
    /// Falha se o processo não subir, não anunciar a porta ou não responder no `/health`.
    pub fn spawn_with(
        program: impl AsRef<Path>,
        scenario: &str,
        env: &[(&str, &str)],
        timeout: Duration,
    ) -> Result<Self, WorkerError> {
        let program = program.as_ref().to_path_buf();
        let scenario = scenario.to_owned();
        let (process, port, token, client) = launch(&program, &scenario, env, timeout)?;
        Ok(Self {
            process,
            port,
            token,
            client,
            program,
            scenario,
        })
    }

    /// Encerra o processo atual e sobe outro, com porta e token novos.
    ///
    /// # Errors
    ///
    /// Falha se o processo novo não ficar pronto. O anterior já foi encerrado.
    pub fn restart(&mut self) -> Result<(), WorkerError> {
        let program = self.program.clone();
        let scenario = self.scenario.clone();
        let (process, port, token, client) =
            launch(&program, &scenario, &[], Duration::from_secs(3))?;
        self.process = process;
        self.port = port;
        self.token = token;
        self.client = client;
        Ok(())
    }

    /// Porta local anunciada pelo worker.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Token desta execução. Não serve para a execução seguinte.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Consulta se o processo já terminou, sem bloquear.
    ///
    /// # Errors
    ///
    /// Falha se o sistema não conseguir consultar o processo.
    pub fn poll(&mut self) -> Result<Option<ProcessOutcome>, WorkerError> {
        self.process.poll().map_err(WorkerError::from)
    }

    /// Últimas linhas de stderr já capturadas.
    #[must_use]
    pub fn stderr(&self) -> String {
        self.process.stderr_text()
    }

    /// Confere se o worker está no ar e aceita o token.
    ///
    /// # Errors
    ///
    /// Falha se o pedido não completar ou o token for recusado.
    pub fn health(&self) -> Result<Health, WorkerError> {
        self.get_json(paths::HEALTH)
    }

    /// Lê o progresso do job até `completed` ou `failed`.
    ///
    /// # Errors
    ///
    /// Falha se o WebSocket não conectar ou se uma mensagem sair do contrato.
    pub fn events(&self, job_id: &str) -> Result<Vec<WorkerEvent>, WorkerError> {
        let mut events = Vec::new();
        Self::watch(self.port, &self.token, job_id, |event| events.push(event))?;
        Ok(events)
    }

    /// Lê o progresso e entrega cada evento assim que chega.
    ///
    /// # Errors
    ///
    /// Falha se o WebSocket não conectar ou se uma mensagem sair do contrato.
    pub fn watch(
        port: u16,
        token: &str,
        job_id: &str,
        on_event: impl FnMut(WorkerEvent),
    ) -> Result<(), WorkerError> {
        Self::watch_until(
            port,
            token,
            job_id,
            &std::sync::atomic::AtomicBool::new(false),
            on_event,
        )
    }

    /// Como [`Self::watch`], mas volta assim que `stop` fica verdadeiro.
    ///
    /// # Errors
    ///
    /// Falha se o WebSocket não conectar ou se uma mensagem sair do contrato.
    pub fn watch_until(
        port: u16,
        token: &str,
        job_id: &str,
        stop: &std::sync::atomic::AtomicBool,
        mut on_event: impl FnMut(WorkerEvent),
    ) -> Result<(), WorkerError> {
        let mut socket = open_events(port, job_id, token)?;
        if let tungstenite::stream::MaybeTlsStream::Plain(tcp) = socket.get_ref() {
            tcp.set_read_timeout(Some(WATCH_SLICE))?;
        }
        loop {
            if stop.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            match socket.read() {
                Ok(tungstenite::Message::Text(text)) => {
                    let event = parse_event(&text)?;
                    let finished =
                        matches!(event, WorkerEvent::Completed(_) | WorkerEvent::Failed(_));
                    on_event(event);
                    if finished {
                        break;
                    }
                }
                Ok(tungstenite::Message::Ping(payload)) => {
                    socket.send(tungstenite::Message::Pong(payload))?;
                }
                Ok(tungstenite::Message::Close(_)) => break,
                Err(tungstenite::Error::Io(error))
                    if error.kind() == std::io::ErrorKind::TimedOut
                        || error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error.into()),
                _ => {}
            }
        }
        Ok(())
    }

    fn get_json<R: DeserializeOwned>(&self, path: &str) -> Result<R, WorkerError> {
        let response = self
            .client
            .get(self.url(path))
            .bearer_auth(&self.token)
            .send()?;
        read_json(response)
    }

    fn post_json<B: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R, WorkerError> {
        self.send_json(path, body, None)
    }

    fn post_job<B: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R, WorkerError> {
        self.send_json(path, body, Some(JOB_LIMIT))
    }

    fn send_json<B: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        timeout: Option<Duration>,
    ) -> Result<R, WorkerError> {
        let mut request = self.client.post(self.url(path)).bearer_auth(&self.token);
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        read_json(request.json(body).send()?)
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl ImageBackend for WorkerProcess {
    fn load_model(&self, request: &LoadModelRequest) -> Result<Ack, WorkerError> {
        self.post_json(paths::MODELS_LOAD, request)
    }

    fn txt2img(&self, request: &Txt2ImgRequest) -> Result<JobOutput, WorkerError> {
        self.post_job(paths::IMAGES_TXT2IMG, request)
    }

    fn img2img(&self, request: &Img2ImgRequest) -> Result<JobOutput, WorkerError> {
        self.post_job(paths::IMAGES_IMG2IMG, request)
    }

    fn inpaint(&self, request: &InpaintRequest) -> Result<JobOutput, WorkerError> {
        self.post_job(paths::IMAGES_INPAINT, request)
    }

    fn decode_latents(&self, request: &DecodeLatentsRequest) -> Result<JobOutput, WorkerError> {
        self.post_job(paths::IMAGES_DECODE_LATENTS, request)
    }

    fn cancel(&self, request: &CancelRequest) -> Result<Ack, WorkerError> {
        self.post_json(paths::JOBS_CANCEL, request)
    }

    fn unload(&self) -> Result<Ack, WorkerError> {
        self.post_json(paths::MODELS_UNLOAD, &serde_json::json!({}))
    }
}

impl MeshBackend for WorkerProcess {
    fn load_model(&self, request: &LoadModelRequest) -> Result<Ack, WorkerError> {
        self.post_json(paths::MODELS_LOAD, request)
    }

    fn image_to_mesh(&self, request: &ImageToMeshRequest) -> Result<JobOutput, WorkerError> {
        self.post_job(paths::MESHES_FROM_IMAGE, request)
    }

    fn cancel(&self, request: &CancelRequest) -> Result<Ack, WorkerError> {
        self.post_json(paths::JOBS_CANCEL, request)
    }

    fn unload(&self) -> Result<Ack, WorkerError> {
        self.post_json(paths::MODELS_UNLOAD, &serde_json::json!({}))
    }
}

impl BackgroundRemover for WorkerProcess {
    fn remove_background(
        &self,
        request: &RemoveBackgroundRequest,
    ) -> Result<JobOutput, WorkerError> {
        self.post_job(paths::IMAGES_REMOVE_BACKGROUND, request)
    }
}

impl MeshPostProcess for WorkerProcess {
    fn post_process(&self, request: &MeshPostProcessRequest) -> Result<JobOutput, WorkerError> {
        self.post_job(paths::MESHES_POST_PROCESS, request)
    }
}

fn launch(
    program: &Path,
    scenario: &str,
    env: &[(&str, &str)],
    timeout: Duration,
) -> Result<(RunningProcess, u16, String, reqwest::blocking::Client), WorkerError> {
    let token = fresh_token()?;
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    let mut command = Command::new(program);
    command
        .env("VOXEL_TOKEN", &token)
        .env("VOXEL_SCENARIO", scenario);
    for (key, value) in env {
        command.env(key, value);
    }
    let mut process = RunningProcess::spawn(command, DEFAULT_LINE_LIMIT)?;
    let port = wait_ready(&mut process)?;
    wait_health(&client, port, &token)?;
    Ok((process, port, token, client))
}

fn fresh_token() -> Result<String, WorkerError> {
    let mut bytes = [0_u8; 16];
    let mut file = std::fs::File::open("/dev/urandom")?;
    std::io::Read::read_exact(&mut file, &mut bytes)?;
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        token.push(hex(byte >> 4));
        token.push(hex(byte & 0x0f));
    }
    Ok(token)
}

fn hex(nibble: u8) -> char {
    char::from(if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    })
}

fn wait_ready(process: &mut RunningProcess) -> Result<u16, WorkerError> {
    let deadline = Instant::now() + STARTUP_LIMIT;
    loop {
        if let Some(outcome) = process.poll()? {
            return Err(WorkerError::Startup(format!(
                "encerrou antes de ficar pronto (código {:?}, sinal {:?}): {}",
                outcome.exit_code,
                outcome.signal,
                process.stderr_text()
            )));
        }
        if let Some(port) = ready_port(&process.stdout_text())? {
            return Ok(port);
        }
        if Instant::now() >= deadline {
            return Err(WorkerError::Timeout);
        }
        std::thread::sleep(POLL_PAUSE);
    }
}

fn ready_port(stdout: &str) -> Result<Option<u16>, WorkerError> {
    for line in stdout.lines() {
        if let Some(raw) = line.strip_prefix(READY_PREFIX) {
            let port = raw
                .trim()
                .parse()
                .map_err(|_| WorkerError::Startup(line.to_owned()))?;
            return Ok(Some(port));
        }
    }
    Ok(None)
}

fn wait_health(
    client: &reqwest::blocking::Client,
    port: u16,
    token: &str,
) -> Result<(), WorkerError> {
    let deadline = Instant::now() + STARTUP_LIMIT;
    let url = format!("http://127.0.0.1:{port}{}", paths::HEALTH);
    let mut last = WorkerError::Timeout;
    while Instant::now() < deadline {
        match client.get(&url).bearer_auth(token).send() {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(response) => last = WorkerError::Startup(response.status().to_string()),
            Err(error) => last = WorkerError::Http(Box::new(error)),
        }
        std::thread::sleep(POLL_PAUSE);
    }
    Err(last)
}

fn read_json<R: DeserializeOwned>(response: reqwest::blocking::Response) -> Result<R, WorkerError> {
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(WorkerError::Unauthorized);
    }
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(WorkerError::Protocol(format!("{status}: {body}")));
    }
    response.json().map_err(WorkerError::from)
}

fn open_events(
    port: u16,
    job_id: &str,
    token: &str,
) -> Result<
    tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    WorkerError,
> {
    let mut url = reqwest::Url::parse(&format!("http://127.0.0.1:{port}{}", paths::EVENTS))
        .map_err(|error| WorkerError::Protocol(error.to_string()))?;
    url.query_pairs_mut().append_pair("jobId", job_id);
    let ws = url.as_str().replacen("http://", "ws://", 1);
    let uri: tungstenite::http::Uri = ws
        .parse()
        .map_err(|error| WorkerError::Protocol(format!("{error}")))?;
    let request = tungstenite::ClientRequestBuilder::new(uri)
        .with_header("Authorization", format!("Bearer {token}"));
    let (socket, _) = tungstenite::connect(request)?;
    Ok(socket)
}

fn parse_event(text: &str) -> Result<WorkerEvent, WorkerError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| WorkerError::Protocol(error.to_string()))?;
    let name = value.get("event").and_then(serde_json::Value::as_str);
    match name {
        Some("progress") => serde_json::from_value(value)
            .map(WorkerEvent::Progress)
            .map_err(|error| WorkerError::Protocol(error.to_string())),
        Some("completed") => serde_json::from_value(value)
            .map(WorkerEvent::Completed)
            .map_err(|error| WorkerError::Protocol(error.to_string())),
        Some("failed") => serde_json::from_value(value)
            .map(WorkerEvent::Failed)
            .map_err(|error| WorkerError::Protocol(error.to_string())),
        Some(other) => Err(WorkerError::Protocol(format!(
            "evento desconhecido: {other}"
        ))),
        None => Err(WorkerError::Protocol("evento sem nome".to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_tem_32_digitos_hexadecimais() {
        let token = fresh_token().expect("urandom");
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn porta_sai_da_linha_pronta() {
        assert_eq!(ready_port("VOXEL_READY 4312").expect("porta"), Some(4312));
        assert_eq!(ready_port("ainda não").expect("vazio"), None);
        assert!(ready_port("VOXEL_READY porta").is_err());
    }

    #[test]
    fn evento_falho_entra_no_contrato() {
        let event =
            parse_event(r#"{"event":"failed","jobId":"j","message":"caiu"}"#).expect("evento");
        assert!(matches!(event, WorkerEvent::Failed(failed) if failed.message == "caiu"));
        assert!(parse_event(r#"{"event":"outro"}"#).is_err());
        assert!(parse_event("não é json").is_err());
    }
}
