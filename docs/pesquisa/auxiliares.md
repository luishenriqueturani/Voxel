# Remoção de fundo e decimação

As duas provas ficaram fora do git, em `~/.cache/voxel-pesquisa`. A imagem é a mesma da pesquisa de 3D: 512×512, fundo cinza e uma elipse clara.

## Remoção de fundo

O probe usa a crate `ort` 2.0.0-rc.15, com o binário pré-compilado do ONNX Runtime 1.31, só em CPU. A máquina tem AVX2, que esse binário exige. A sessão pediu 8 threads intra-op.

A crate declara `rust-version` 1.92. O workspace declara 1.90, e o `rustc` 1.97 desta máquina compilou o probe. Entrar com o `ort` no produto pede subir o MSRV para 1.92 ou fixar um release candidate anterior.

Os ONNX vêm dos assets do rembg `v0.0.0`, não do RMBG. O RMBG/BRIA continua fora: a licença é não comercial. As licenças de origem são Apache-2.0 para U²-Net e IS-Net (DIS) e MIT para o BiRefNet.

| Modelo | Arquivo | Entrada | Carga | Inferência | Máscara |
| --- | --- | --- | --- | --- | --- |
| U²-Net | `u2net.onnx`, 176 MB | 320, ImageNet | 249 ms | 239 ms | elipse limpa |
| IS-Net | `isnet-general-use.onnx`, 179 MB | 1024, média 0,5 | 172 ms | 646 ms | elipse limpa |
| BiRefNet lite | `birefnet-general-lite.onnx`, 224 MB | 1024, ImageNet, sigmoide | 2,3 s | 5,8 s | elipse limpa |
| BiRefNet | `birefnet-general.onnx`, 973 MB | 1024, ImageNet, sigmoide | 4,0 s | 10,1 s | elipse limpa |

A parcela de pixels claros ficou entre 16,4% e 16,5% nos quatro. Para o MVP, U²-Net ou IS-Net entregam a mesma máscara nesta cena com bem menos tempo e memória. O BiRefNet completo só se justifica se uma cena mais difícil mostrar diferença.

## Decimação

Alvo de 5 000 faces, nas malhas já geradas. Nenhuma era fechada, e nenhuma ficou fechada depois.

| Malha | Original | trimesh | pymeshlab | meshoptimizer |
| --- | --- | --- | --- | --- |
| TripoSR | 22908 faces, 11494 verts | 4999 / 2505 em 56 ms | 4999 / 2519 em 216 ms | 4997 / 2506 em 6 ms |
| Stable Fast 3D | 24228 faces, 14357 verts | 4999 / 3172 em 20 ms | 5000 / 8809 em 123 ms | 5000 / 3863 em 7 ms |

O erro relativo que o `meshopt` 0.7.0 reportou foi 0,0013 no TripoSR e 0,0015 no Stable Fast 3D, com `target_error` 1,0 para deixar o limite de faces mandar. Os OBJ reduzidos reabrem no trimesh.

O pymeshlab não gravou o GLB direto: falta o plugin da textura embutida. A prova usou um OBJ só com a geometria. Na malha do Stable Fast 3D ele manteve 8809 vértices para 5000 faces, bem mais do que as outras duas.

Licenças e encaixe:

- `meshopt` 0.7.0 é MIT ou Apache-2.0, pede Rust 1.87 e cabe no core, sem Python. Foi o mais rápido e o que melhor acompanha o alvo.
- `trimesh` 5.1.1 e `fast-simplification` 0.2.0 são MIT. Servem no worker Python, se ele já estiver de pé.
- `pymeshlab` 2025.7.post1 é GPL-3. Funciona como ferramenta de avaliação e não entra no aplicativo MIT.
