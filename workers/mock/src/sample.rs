/// PNG 1×1 vermelho, válido, para os testes de contrato.
pub const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0,
    0, 0, 144, 119, 83, 222, 0, 0, 0, 12, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 0, 0, 3, 1,
    1, 0, 201, 254, 146, 239, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

/// GLB mínimo com o cabeçalho e o JSON do asset.
#[must_use]
pub fn minimal_glb() -> Vec<u8> {
    let mut json = br#"{"asset":{"version":"2.0"}}"#.to_vec();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let json_len = u32::try_from(json.len()).expect("json cabe em u32");
    let total = 20 + json_len;
    let mut out = Vec::with_capacity(usize::try_from(total).expect("tamanho"));
    push_u32(&mut out, 0x4654_6C67);
    push_u32(&mut out, 2);
    push_u32(&mut out, total);
    push_u32(&mut out, json_len);
    push_u32(&mut out, 0x4E4F_534A);
    out.extend(json);
    out
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exemplos_tem_assinatura() {
        assert!(PNG.starts_with(b"\x89PNG"));
        let glb = minimal_glb();
        assert!(glb.starts_with(b"glTF"));
        let json = &glb[20..];
        let text = std::str::from_utf8(json).expect("json");
        assert!(text.contains(r#""version":"2.0""#));
    }
}
