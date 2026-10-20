//! Teste real do ambiente: a convolução fp16 que aborta com o override errado.
//!
//! Multiplicar matriz não basta. O caso da fase 1 quebra num `conv2d` fp16 com filtro
//! 2×3 e 16 canais, dentro da `MIOpen`. CPU usa fp32 porque o risco medido é o da GPU.

pub const PROBE_SCRIPT: &str = r#"import sys
import torch

device = sys.argv[1]
if device == "cuda" and not torch.cuda.is_available():
    sys.exit("cuda indisponivel")
if device == "xpu" and not (hasattr(torch, "xpu") and torch.xpu.is_available()):
    sys.exit("xpu indisponivel")
dtype = torch.float32 if device == "cpu" else torch.float16
weight = torch.randn(16, 16, 2, 3, device=device, dtype=dtype)
image = torch.randn(1, 16, 8, 8, device=device, dtype=dtype)
result = torch.nn.functional.conv2d(image, weight)
if device == "cuda":
    torch.cuda.synchronize()
print("voxel-python-ok", tuple(result.shape))
"#;

#[must_use]
pub fn passed(status: bool, output: &str) -> bool {
    status && output.contains("voxel-python-ok")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_teste_exercita_a_convolucao_fp16_e_nao_so_uma_matriz() {
        assert!(PROBE_SCRIPT.contains("torch.float16"));
        assert!(PROBE_SCRIPT.contains("conv2d"));
        assert!(PROBE_SCRIPT.contains("2, 3"));
        assert!(passed(true, "voxel-python-ok (1, 16, 7, 6)\n"));
        assert!(!passed(false, "voxel-python-ok"));
        assert!(!passed(true, "cuda indisponivel"));
    }
}
