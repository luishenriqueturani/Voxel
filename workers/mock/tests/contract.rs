use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use voxel_protocol::{
    BackgroundRemover, CancelRequest, DecodeLatentsRequest, ErrorBody, ImageBackend,
    ImageToMeshRequest, Img2ImgRequest, InpaintRequest, LoadModelRequest, MeshBackend,
    MeshPostProcess, MeshPostProcessRequest, ModelKind, ProcessOutcome, RemoveBackgroundRequest,
    Txt2ImgRequest, WorkerEvent, WorkerProcess, paths,
};

fn worker(scenario: &str) -> WorkerProcess {
    WorkerProcess::spawn(env!("CARGO_BIN_EXE_voxel-mock"), scenario).expect("worker falso")
}

fn temp_file(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!("voxel-mock-{name}-{}-{id}", std::process::id()));
    path
}

fn txt(path: &Path, job: &str) -> Txt2ImgRequest {
    Txt2ImgRequest {
        job_id: job.to_owned(),
        prompt: "um gato".to_owned(),
        negative_prompt: Some("texto".to_owned()),
        model_id: "sd".to_owned(),
        output_path: path.display().to_string(),
        steps: 2,
        width: 8,
        height: 8,
        seed: 1,
        cfg_scale: None,
    }
}

fn wait_dead(worker: &mut WorkerProcess) -> ProcessOutcome {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(outcome) = worker.poll().expect("consultar o processo") {
            return outcome;
        }
        assert!(Instant::now() < deadline, "o worker não encerrou");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn health_aceita_o_token() {
    let worker = worker("ok");
    let health = worker.health().expect("health");
    assert_eq!(health.status, "ok");
}

#[test]
fn token_errado_recebe_401() {
    let worker = worker("ok");
    let response = reqwest::blocking::Client::new()
        .get(format!(
            "http://127.0.0.1:{}{}",
            worker.port(),
            paths::HEALTH
        ))
        .header("Authorization", "Bearer errado")
        .send()
        .expect("resposta");
    assert_eq!(response.status(), 401);
    let body: ErrorBody = response.json().expect("corpo");
    assert!(!body.message.is_empty());
}

#[test]
fn txt2img_grava_png_e_publica_progresso() {
    let worker = worker("ok");
    let path = temp_file("txt.png");
    let output = worker.txt2img(&txt(&path, "job-txt")).expect("txt2img");
    assert_eq!(output.path, path.display().to_string());
    let bytes = std::fs::read(&path).expect("png");
    assert!(bytes.starts_with(b"\x89PNG"));
    let events = worker.events("job-txt").expect("eventos");
    assert!(matches!(
        events.first(),
        Some(WorkerEvent::Progress(event)) if event.step == 1 && event.total == 2
    ));
    assert!(matches!(
        events.last(),
        Some(WorkerEvent::Completed(event)) if event.path == output.path
    ));
    let _ = std::fs::remove_file(path);
}

#[test]
fn falha_simulada_chega_pelo_websocket() {
    let worker = worker("fail");
    let path = temp_file("fail.png");
    worker.txt2img(&txt(&path, "job-fail")).expect("txt2img");
    let events = worker.events("job-fail").expect("eventos");
    assert!(matches!(
        events.last(),
        Some(WorkerEvent::Failed(event)) if event.message == "falha simulada"
    ));
    let _ = std::fs::remove_file(path);
}

#[test]
fn comandos_de_imagem_gravam_png() {
    let worker = worker("ok");
    let image = temp_file("img.png");
    let painted = temp_file("paint.png");
    let decoded = temp_file("latents.png");
    let cut = temp_file("cut.png");

    assert!(
        ImageBackend::load_model(
            &worker,
            &LoadModelRequest {
                model_id: "sd".to_owned(),
                model_path: "/modelos/sd".to_owned(),
                kind: ModelKind::Image,
            },
        )
        .expect("load")
        .accepted
    );
    let img = ImageBackend::img2img(
        &worker,
        &Img2ImgRequest {
            job_id: "job-img".to_owned(),
            prompt: "um gato".to_owned(),
            model_id: "sd".to_owned(),
            image_path: "/entrada.png".to_owned(),
            output_path: image.display().to_string(),
            steps: 2,
            strength: 0.5,
        },
    )
    .expect("img2img");
    assert!(
        std::fs::read(img.path)
            .expect("png")
            .starts_with(b"\x89PNG")
    );
    ImageBackend::inpaint(
        &worker,
        &InpaintRequest {
            job_id: "job-paint".to_owned(),
            prompt: "um gato".to_owned(),
            model_id: "sd".to_owned(),
            image_path: "/entrada.png".to_owned(),
            mask_path: "/mascara.png".to_owned(),
            output_path: painted.display().to_string(),
        },
    )
    .expect("inpaint");
    ImageBackend::decode_latents(
        &worker,
        &DecodeLatentsRequest {
            job_id: "job-latents".to_owned(),
            model_id: "sd".to_owned(),
            latents_path: "/latentes.bin".to_owned(),
            output_path: decoded.display().to_string(),
        },
    )
    .expect("decode");
    BackgroundRemover::remove_background(
        &worker,
        &RemoveBackgroundRequest {
            job_id: "job-cut".to_owned(),
            image_path: "/entrada.png".to_owned(),
            output_path: cut.display().to_string(),
        },
    )
    .expect("fundo");
    assert!(
        ImageBackend::cancel(
            &worker,
            &CancelRequest {
                job_id: "job-img".to_owned(),
            },
        )
        .expect("cancel")
        .accepted
    );
    assert!(ImageBackend::unload(&worker).expect("unload").accepted);
    for path in [image, painted, decoded, cut] {
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn comandos_de_malha_gravam_glb() {
    let worker = worker("ok");
    let mesh = temp_file("mesh.glb");
    let processed = temp_file("mesh-out.glb");
    assert!(
        MeshBackend::load_model(
            &worker,
            &LoadModelRequest {
                model_id: "tripo".to_owned(),
                model_path: "/modelos/tripo".to_owned(),
                kind: ModelKind::Mesh,
            },
        )
        .expect("load")
        .accepted
    );
    let mesh_out = MeshBackend::image_to_mesh(
        &worker,
        &ImageToMeshRequest {
            job_id: "job-mesh".to_owned(),
            image_path: "/entrada.png".to_owned(),
            model_id: "tripo".to_owned(),
            output_path: mesh.display().to_string(),
        },
    )
    .expect("malha");
    assert!(
        std::fs::read(mesh_out.path)
            .expect("glb")
            .starts_with(b"glTF")
    );
    MeshPostProcess::post_process(
        &worker,
        &MeshPostProcessRequest {
            job_id: "job-post".to_owned(),
            mesh_path: mesh.display().to_string(),
            output_path: processed.display().to_string(),
            decimate: true,
            normalize: true,
            max_textures: Some(2),
        },
    )
    .expect("pós");
    assert!(processed.is_file());
    assert!(
        MeshBackend::cancel(
            &worker,
            &CancelRequest {
                job_id: "job-mesh".to_owned(),
            },
        )
        .expect("cancel")
        .accepted
    );
    assert!(MeshBackend::unload(&worker).expect("unload").accepted);
    for path in [mesh, processed] {
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn abort_imprime_isa_e_morre_com_sinal_6() {
    let mut worker = worker("abort");
    let path = temp_file("abort.png");
    let error = worker.txt2img(&txt(&path, "job-abort"));
    assert!(error.is_err());
    let outcome = wait_dead(&mut worker);
    assert_eq!(outcome.signal, Some(6));
    assert!(
        worker.stderr().contains("HSA_STATUS_ERROR_INVALID_ISA"),
        "{}",
        worker.stderr()
    );
}

#[test]
fn falta_de_memoria_encerra_com_codigo_1() {
    let mut worker = worker("oom");
    let path = temp_file("oom.png");
    assert!(worker.txt2img(&txt(&path, "job-oom")).is_err());
    let outcome = wait_dead(&mut worker);
    assert_eq!(outcome.exit_code, Some(1));
    assert_eq!(outcome.signal, None);
    assert!(
        worker.stderr().contains("CUDA out of memory"),
        "{}",
        worker.stderr()
    );
}

#[test]
fn reinicio_troca_o_token() {
    let mut worker = worker("ok");
    let old_token = worker.token().to_owned();
    worker.restart().expect("reiniciar");
    assert_ne!(worker.token(), old_token);
    worker.health().expect("health novo");
    let response = reqwest::blocking::Client::new()
        .get(format!(
            "http://127.0.0.1:{}{}",
            worker.port(),
            paths::HEALTH
        ))
        .bearer_auth(old_token)
        .send()
        .expect("resposta");
    assert_eq!(response.status(), 401);
}
