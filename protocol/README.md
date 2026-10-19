# protocol

Esquema compartilhado (OpenAPI/JSON Schema) do protocolo entre o core e os workers. Os tipos em Rust, TypeScript e Python são gerados a partir daqui. Ver `docs/PLANEJAMENTO.md` §7.4.

```
cargo run -p voxel-protocol --bin gen-protocol
```

O comando regrava `crates/protocol/src/generated.rs`, `apps/desktop/src/protocol/types.ts` e `workers/python/src/voxel_worker/protocol.py`. Um teste compara esses arquivos com a saída do gerador.
