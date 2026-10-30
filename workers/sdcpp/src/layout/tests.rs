use super::header::{bare_diffusion, is_lora, is_qwen_image};
use super::{
    extend_model, extend_runtime_flags, missing_parts, parts_for, runtime_backend,
    tensor_type_rules,
};

fn touch(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, []).unwrap();
}

fn flux_tree(root: &std::path::Path) {
    let index = r#"{
        "_class_name": "FluxPipeline",
        "text_encoder": ["transformers", "CLIPTextModel"],
        "text_encoder_2": ["transformers", "T5EncoderModel"],
        "transformer": ["diffusers", "FluxTransformer2DModel"],
        "vae": ["diffusers", "AutoencoderKL"]
    }"#;
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(root.join("model_index.json"), index).unwrap();
    touch(&root.join("transformer/diffusion_pytorch_model.safetensors.index.json"));
    touch(&root.join("text_encoder/model.safetensors"));
    touch(&root.join("text_encoder_2/model.safetensors.index.json"));
    touch(&root.join("vae/diffusion_pytorch_model.safetensors"));
}

fn write_flux_gguf(path: &std::path::Path) {
    let mut data = Vec::new();
    data.extend(b"GGUF");
    data.extend(&3u32.to_le_bytes());
    data.extend(&1u64.to_le_bytes());
    data.extend(&3u64.to_le_bytes());
    push_str(&mut data, "general.architecture");
    data.extend(&8u32.to_le_bytes());
    push_str(&mut data, "flux");
    push_str(&mut data, "general.quantization_version");
    data.extend(&4u32.to_le_bytes());
    data.extend(&2u32.to_le_bytes());
    push_str(&mut data, "general.file_type");
    data.extend(&4u32.to_le_bytes());
    data.extend(&15u32.to_le_bytes());
    push_str(&mut data, "double_blocks.0.img_attn.qkv.weight");
    data.extend(&1u32.to_le_bytes());
    data.extend(&1u64.to_le_bytes());
    data.extend(&0u32.to_le_bytes());
    data.extend(&0u64.to_le_bytes());
    std::fs::write(path, data).unwrap();
}

fn push_str(data: &mut Vec<u8>, text: &str) {
    let len = u64::try_from(text.len()).unwrap();
    data.extend(&len.to_le_bytes());
    data.extend(text.as_bytes());
}

#[test]
fn a_pasta_flux_abre_cada_parte() {
    let root = std::env::temp_dir().join(format!("voxel-flux-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    flux_tree(&root);
    let parts = parts_for(&root);
    assert!(parts.checkpoint.is_none());
    assert_eq!(
        parts.diffusion.unwrap().file_name().unwrap(),
        "diffusion_pytorch_model.safetensors.index.json"
    );
    assert!(
        parts
            .clip_l
            .unwrap()
            .ends_with("text_encoder/model.safetensors")
    );
    assert!(
        parts
            .t5xxl
            .unwrap()
            .ends_with("text_encoder_2/model.safetensors.index.json")
    );
    assert!(
        parts
            .vae
            .unwrap()
            .ends_with("vae/diffusion_pytorch_model.safetensors")
    );
    assert!(parts.llm.is_none());
    let mut args = Vec::new();
    extend_model(&mut args, &root);
    assert!(!args.iter().any(|arg| arg == "-m"));
    assert!(args.iter().any(|arg| arg == "--diffusion-model"));
    assert!(args.iter().any(|arg| arg == "--clip_l"));
    assert!(args.iter().any(|arg| arg == "--t5xxl"));
    assert!(args.iter().any(|arg| arg == "--vae"));
    assert!(args.iter().any(|arg| arg == "--offload-to-cpu"));
    assert!(!args.iter().any(|arg| arg == "--tokenizer"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn o_gguf_flux_usa_o_arquivo_como_diffusion_e_os_vizinhos() {
    let root = std::env::temp_dir().join(format!("voxel-gguf-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    flux_tree(&root);
    let gguf = root.join("peso-Q4_K_M.gguf");
    write_flux_gguf(&gguf);
    let parts = parts_for(&gguf);
    assert_eq!(parts.diffusion.unwrap(), gguf);
    assert!(parts.clip_l.is_some());
    assert!(parts.t5xxl.is_some());
    assert!(parts.vae.is_some());
    assert!(parts.checkpoint.is_none());
    let mut args = Vec::new();
    extend_model(&mut args, &gguf);
    assert!(args.iter().any(|arg| arg == "--diffusion-model"));
    assert!(!args.iter().any(|arg| arg == "--offload-to-cpu"));
    assert!(missing_parts(&gguf).is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn o_gguf_flux_sem_vizinhos_lista_o_que_falta() {
    let root = std::env::temp_dir().join(format!("voxel-gguf-solo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("vae")).unwrap();
    let gguf = root.join("peso-Q4_K_M.gguf");
    write_flux_gguf(&gguf);
    assert_eq!(missing_parts(&gguf), ["VAE", "CLIP-L", "T5-XXL"]);
    let _ = std::fs::remove_dir_all(&root);
}

fn write_qwen_gguf(path: &std::path::Path) {
    let mut data = Vec::new();
    data.extend(b"GGUF");
    data.extend(&3u32.to_le_bytes());
    data.extend(&1u64.to_le_bytes());
    data.extend(&1u64.to_le_bytes());
    push_str(&mut data, "general.architecture");
    data.extend(&8u32.to_le_bytes());
    push_str(&mut data, "qwen_image21");
    push_str(&mut data, "img_in.weight");
    data.extend(&1u32.to_le_bytes());
    data.extend(&1u64.to_le_bytes());
    data.extend(&0u32.to_le_bytes());
    data.extend(&0u64.to_le_bytes());
    std::fs::write(path, data).unwrap();
}

#[test]
fn o_gguf_qwen_usa_llm_e_vae_ao_lado() {
    let root = std::env::temp_dir().join(format!("voxel-qwen-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    touch(&root.join("vae/qwen_vae.safetensors"));
    touch(&root.join("text_encoders/qwen3vl.safetensors"));
    let gguf = root.join("qwen-image-2.1-UC-BF16.gguf");
    write_qwen_gguf(&gguf);
    assert!(is_qwen_image(&gguf));
    assert!(bare_diffusion(&gguf));
    let parts = parts_for(&gguf);
    assert_eq!(parts.diffusion.as_deref(), Some(gguf.as_path()));
    assert!(parts.vae.is_some());
    assert!(parts.llm.is_some());
    assert!(parts.checkpoint.is_none());
    assert!(missing_parts(&gguf).is_empty());
    let mut args = Vec::new();
    extend_model(&mut args, &gguf);
    assert!(args.iter().any(|arg| arg == "--diffusion-model"));
    assert!(args.iter().any(|arg| arg == "--llm"));
    assert!(args.iter().any(|arg| arg == "--vae"));
    assert_eq!(runtime_backend(&gguf, "Vulkan0"), "llm=cpu,Vulkan0");
    assert_eq!(tensor_type_rules(&gguf, "Vulkan0"), r"^vae\.=f16");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn o_gguf_qwen_sem_vizinhos_pede_vae_e_llm() {
    let root = std::env::temp_dir().join(format!("voxel-qwen-solo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let gguf = root.join("qwen.gguf");
    write_qwen_gguf(&gguf);
    assert_eq!(missing_parts(&gguf), ["VAE", "LLM"]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn o_lora_nao_e_um_modelo() {
    let root = std::env::temp_dir().join(format!("voxel-lora-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let header = br#"{"transformer.transformer_blocks.0.attn.to_q.lora_A.weight":{"dtype":"BF16","shape":[1],"data_offsets":[0,2]}}"#;
    let mut data = u64::try_from(header.len()).unwrap().to_le_bytes().to_vec();
    data.extend(header);
    data.extend([0, 0]);
    let file = root.join("omni-lora-v0.1-bf16.safetensors");
    std::fs::write(&file, data).unwrap();
    assert!(is_lora(&file));
    assert_eq!(missing_parts(&file), ["modelo-base"]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn o_lora_usa_o_qwen_ja_instalado() {
    let root = std::env::temp_dir().join(format!("voxel-lora-base-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let hub = root.join("models/hub");
    let base = hub.join("Qwen/Qwen-Image-2.1");
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(
        base.join("model_index.json"),
        r#"{"_class_name":"QwenImage21Pipeline","text_encoder":["transformers","Qwen3VLForConditionalGeneration"]}"#,
    )
    .unwrap();
    touch(&base.join("vae/qwen_vae.safetensors"));
    touch(&base.join("text_encoder/model.safetensors.index.json"));
    touch(&base.join("transformer/diffusion_pytorch_model.safetensors.index.json"));
    let turbo = hub.join("Qwen/Qwen-Image-2.1-Turbo");
    std::fs::create_dir_all(&turbo).unwrap();
    std::fs::write(
        turbo.join("model_index.json"),
        r#"{"_class_name":"QwenImage21Pipeline"}"#,
    )
    .unwrap();
    touch(&turbo.join("vae/qwen_vae.safetensors"));
    touch(&turbo.join("transformer/diffusion_pytorch_model.safetensors.index.json"));
    let lora_dir = hub.join("labs/omni");
    std::fs::create_dir_all(&lora_dir).unwrap();
    let header = br#"{"transformer.transformer_blocks.0.attn.to_q.lora_A.weight":{"dtype":"BF16","shape":[1],"data_offsets":[0,2]}}"#;
    let mut data = u64::try_from(header.len()).unwrap().to_le_bytes().to_vec();
    data.extend(header);
    data.extend([0, 0]);
    let lora = lora_dir.join("omni-lora-v0.1-bf16.safetensors");
    std::fs::write(&lora, data).unwrap();
    assert!(missing_parts(&lora).is_empty());
    let found = super::shared::find_qwen_base(&lora).unwrap();
    assert!(found.ends_with("Qwen-Image-2.1"));
    let parts = parts_for(&lora);
    assert!(parts.diffusion.is_some());
    assert!(parts.llm.is_some());
    assert!(parts.vae.is_some());
    let mut args = Vec::new();
    extend_model(&mut args, &lora);
    super::shared::extend_lora(&mut args, &lora);
    assert!(args.iter().any(|arg| arg == "--diffusion-model"));
    assert!(args.iter().any(|arg| arg == "--lora-model-dir"));
    assert!(super::shared::prompt_with_lora(&lora, "um gato").contains("<lora:"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn o_gguf_qwen_empresta_vae_e_llm_de_outro_pacote() {
    let root = std::env::temp_dir().join(format!("voxel-qwen-emprestimo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let hub = root.join("models/hub");
    let base = hub.join("acme/qwen-completo");
    std::fs::create_dir_all(&base).unwrap();
    touch(&base.join("vae/qwen_vae.safetensors"));
    touch(&base.join("text_encoders/qwen3vl.safetensors"));
    let alone = hub.join("outro/qwen-gguf");
    std::fs::create_dir_all(&alone).unwrap();
    let gguf = alone.join("qwen.gguf");
    write_qwen_gguf(&gguf);
    assert!(missing_parts(&gguf).is_empty());
    let mut args = Vec::new();
    extend_model(&mut args, &gguf);
    assert!(args.iter().any(|arg| arg == "--llm"));
    assert!(args.iter().any(|arg| arg == "--vae"));
    let _ = std::fs::remove_dir_all(&root);
}

fn write_flux_safetensors(path: &std::path::Path) {
    let header = br#"{"double_blocks.0.img_attn.qkv.weight":{"dtype":"BF16","shape":[1],"data_offsets":[0,2]}}"#;
    let mut data = u64::try_from(header.len()).unwrap().to_le_bytes().to_vec();
    data.extend(header);
    data.extend([0, 0]);
    std::fs::write(path, data).unwrap();
}

#[test]
fn o_transformer_solto_empresta_as_pecas_de_outro_flux_do_hub() {
    let root = std::env::temp_dir().join(format!("voxel-emprestimo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let hub = root.join("models/hub");
    flux_tree(&hub.join("acme/flux-completo"));
    let alone = hub.join("outro/fhdr");
    std::fs::create_dir_all(alone.join("vae")).unwrap();
    let file = alone.join("fhdr.safetensors");
    write_flux_safetensors(&file);
    let gguf = alone.join("fhdr-Q8_0.gguf");
    write_flux_gguf(&gguf);
    for model in [&file, &gguf] {
        assert!(bare_diffusion(model));
        let parts = parts_for(model);
        assert_eq!(parts.diffusion.as_deref(), Some(model.as_path()));
        assert!(parts.checkpoint.is_none());
        assert!(
            parts
                .vae
                .unwrap()
                .starts_with(hub.join("acme/flux-completo"))
        );
        assert!(parts.clip_l.is_some());
        assert!(parts.t5xxl.is_some());
        assert!(missing_parts(model).is_empty());
    }
    let mut args = Vec::new();
    extend_model(&mut args, &file);
    assert!(args.iter().any(|arg| arg == "--offload-to-cpu"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn o_qwen_vai_no_llm() {
    let root = std::env::temp_dir().join(format!("voxel-moss-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("model_index.json"),
        r#"{"text_encoder":["transformers","Qwen3_5Model"]}"#,
    )
    .unwrap();
    touch(&root.join("text_encoder/model.safetensors.index.json"));
    touch(&root.join("transformer/diffusion_pytorch_model.safetensors.index.json"));
    touch(&root.join("vae/diffusion_pytorch_model.safetensors"));
    touch(&root.join("tokenizer/tokenizer.json"));
    let parts = parts_for(&root);
    assert!(
        parts
            .llm
            .unwrap()
            .ends_with("text_encoder/model.safetensors.index.json")
    );
    assert!(
        parts
            .tokenizer
            .unwrap()
            .ends_with("tokenizer/tokenizer.json")
    );
    assert!(parts.clip_l.is_none());
    assert!(parts.t5xxl.is_none());
    let mut args = Vec::new();
    extend_model(&mut args, &root);
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--tokenizer" && pair[1].ends_with("tokenizer/tokenizer.json"))
    );
    assert_eq!(runtime_backend(&root, "Vulkan0"), "llm=cpu,Vulkan0");
    assert_eq!(
        tensor_type_rules(&root, "Vulkan0"),
        r"^model\.=f16,^vae\.=f16,^text_encoders\.=f16"
    );
    assert_eq!(tensor_type_rules(&root, "ROCm0"), r"^vae\.=f16");
    let mut flags = Vec::new();
    extend_runtime_flags(&mut flags, &root);
    assert!(flags.iter().any(|arg| arg == "--disable-prefetch"));
    assert!(
        flags
            .windows(2)
            .any(|pair| pair[0] == "--max-vram" && pair[1] == "-2")
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_pasta_com_unet_continua_no_checkpoint() {
    let root = std::env::temp_dir().join(format!("voxel-unet-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    touch(&root.join("unet/diffusion_pytorch_model.safetensors"));
    touch(&root.join("model_index.json"));
    let parts = parts_for(&root);
    assert_eq!(parts.checkpoint.unwrap(), root);
    assert!(parts.diffusion.is_none());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn o_gguf_da_variavel_e_diffusion_solto() {
    let Ok(path) = std::env::var("VOXEL_FLUX_GGUF") else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    if !path.is_file() {
        return;
    }
    assert!(bare_diffusion(&path));
    let parts = parts_for(&path);
    assert_eq!(parts.diffusion.as_deref(), Some(path.as_path()));
    assert!(parts.clip_l.is_some());
    assert!(parts.t5xxl.is_some());
    assert!(parts.vae.is_some());
}

#[test]
fn um_arquivo_comum_continua_no_checkpoint() {
    let root = std::env::temp_dir().join(format!("voxel-sd15-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let file = root.join("sd15.safetensors");
    touch(&file);
    let parts = parts_for(&file);
    assert_eq!(parts.checkpoint.unwrap(), file);
    assert_eq!(runtime_backend(&file, "Vulkan0"), "Vulkan0");
    let _ = std::fs::remove_dir_all(&root);
}
