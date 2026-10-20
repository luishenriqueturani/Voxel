use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

use voxel_protocol::{
    ImageBackend, LoadModelRequest, ModelKind, Txt2ImgRequest, WorkerEvent, WorkerProcess,
};

#[test]
fn o_contrato_entrega_png_progresso_e_cancelamento() {
    let dir = std::env::temp_dir().join(format!("voxel-sdcpp-http-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let binary = dir.join("sd-cli");
    std::fs::write(
        &binary,
        r#"#!/bin/sh
if [ "$1" = "--list-devices" ]; then printf 'Vulkan0\tGPU\n'; exit 0; fi
if [ "$1" = "--sleep" ]; then exec sleep 30; fi
prev=""
for arg in "$@"; do
  if [ "$prev" = "--preview-path" ] || [ "$prev" = "-o" ]; then
    printf '\x89PNG\r\n\x1a\n' > "$arg"
    dd if=/dev/zero bs=1100 count=1 >> "$arg" 2>/dev/null
  fi
  prev="$arg"
done
printf '\r1/2 - 1.00it/s'
exit 0
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&binary, permissions).unwrap();
    let model = dir.join("modelo");
    std::fs::write(&model, "pesos").unwrap();

    let worker = WorkerProcess::spawn_with(
        env!("CARGO_BIN_EXE_voxel-sdcpp"),
        "ok",
        &[
            ("VOXEL_SD_CLI", binary.to_str().unwrap()),
            ("VOXEL_IMAGE_BACKEND", "vulkan"),
        ],
        Duration::from_secs(10),
    )
    .expect("worker");
    worker
        .load_model(&LoadModelRequest {
            model_id: "sd15".to_owned(),
            model_path: model.display().to_string(),
            kind: ModelKind::Image,
        })
        .expect("carregar");
    let output = dir.join("out.png");
    let request = Txt2ImgRequest {
        job_id: "job-http".to_owned(),
        prompt: "um quadrado".to_owned(),
        negative_prompt: None,
        model_id: "sd15".to_owned(),
        output_path: output.display().to_string(),
        steps: 2,
        width: 64,
        height: 64,
        seed: 1,
        cfg_scale: None,
    };
    let generated = worker.txt2img(&request).expect("txt2img");
    assert_eq!(generated.path, output.display().to_string());
    assert!(std::fs::read(&output).unwrap().starts_with(b"\x89PNG"));
    let events = worker.events("job-http").expect("eventos");
    assert!(events.iter().any(|event| matches!(
        event,
        WorkerEvent::Progress(progress) if progress.step == 1 && progress.preview_path.is_some()
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, WorkerEvent::Completed(_)))
    );

    worker
        .cancel(&voxel_protocol::CancelRequest {
            job_id: "ausente".to_owned(),
        })
        .expect("cancelar");
    let _ = PathBuf::from(&generated.path);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "gera uma imagem real com o sd-cli Vulkan"]
fn vulkan_entrega_png_e_progresso() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = root.join("workers/sdcpp/build/vulkan/bin/sd-cli");
    let model = root.join("models/sd15/v1-5-pruned-emaonly.safetensors");
    assert!(binary.is_file(), "sd-cli vulkan");
    assert!(model.is_file(), "modelo sd15");
    let dir = std::env::temp_dir().join(format!("voxel-sdcpp-real-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let worker = WorkerProcess::spawn_with(
        env!("CARGO_BIN_EXE_voxel-sdcpp"),
        "ok",
        &[
            ("VOXEL_SD_CLI", binary.to_str().unwrap()),
            ("VOXEL_IMAGE_BACKEND", "vulkan"),
        ],
        Duration::from_mins(3),
    )
    .expect("worker");
    worker
        .load_model(&LoadModelRequest {
            model_id: "sd15".to_owned(),
            model_path: model.display().to_string(),
            kind: ModelKind::Image,
        })
        .expect("carregar");
    let output = dir.join("out.png");
    let request = Txt2ImgRequest {
        job_id: "job-real".to_owned(),
        prompt: "a red square on a white background".to_owned(),
        negative_prompt: None,
        model_id: "sd15".to_owned(),
        output_path: output.display().to_string(),
        steps: 4,
        width: 512,
        height: 512,
        seed: 1,
        cfg_scale: None,
    };
    let generated = worker.txt2img(&request).expect("txt2img");
    assert!(
        std::fs::read(&generated.path)
            .unwrap()
            .starts_with(b"\x89PNG")
    );
    let events = worker.events("job-real").expect("eventos");
    assert!(events.iter().any(|event| matches!(
        event,
        WorkerEvent::Progress(progress) if progress.step >= 1 && progress.preview_path.is_some()
    )));
    let _ = std::fs::remove_dir_all(&dir);
}
