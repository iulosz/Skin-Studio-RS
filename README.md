# Skin Studio RS

Aplicativo desktop independente em Rust, criado para reconstruir gradualmente os recursos do Skin Studio 2.4.76 sem depender do Blender no runtime.

## Estado atual — protótipo 0.1.0

- Interface desktop em Rust com navegação Setup, Color, Detail, Reference, Surface, Hair e Bake.
- Pré-visualização de material usando shader WGSL próprio executado por `wgpu`.
- Viewport rasterizado de malhas triangulares com teste de profundidade e grade real.
- Importação de malhas OBJ, glTF e GLB, com centralização e escala automáticas.
- Câmera orbital com zoom, pan e restauração de enquadramento.
- Esfera de demonstração como malha padrão para ajustar o material antes da importação de um modelo.
- Controles de poros portados do Skin Studio 2.4.76: intensidade, escala, irregularidade, variação de tamanho, mapa de altura tileável, seed e relevo.
- Shader WGSL com três populações Voronoi, projeção triplanar do height map de poros, fine detail, micro-grain e oclusão especular.
- Gerador independente de Reference Height em Rust, com rede Voronoi em duas escalas, sulcos, poros nas junções e ruído periódico por FFT; a textura é enviada à GPU como R16Float não-color.
- Skin Line Network no WGSL: células em platôs, sulcos primários/secundários, vincos mascarados por ruído, controle de escala física e sombreado dos sulcos.
- GitHub Actions com verificação de formato, testes e builds de release para Windows, Linux e macOS.

**Ainda não é a conversão completa do addon.** O primeiro subsistema de poros está portado, mas as máscaras anatômicas ainda são heurísticas baseadas no bounding box e assumem uma cabeça voltada para -Y. Coord Scale agora inclui M/CM/MM/custom; Skin Line Network, crosshatch, estilo LEATHER do mapa de referência, pigmentação anatômica, vascularização, lábios, SSS, cabelo e bake continuam no roadmap. A compilação não substitui comparação visual com o Blender.

## Editor de pele — primeira etapa

O painel de propriedades expõe controles para Base Color, tonalidade aproximada de hemoglobina, variação de pigmentação, poros, micro-relevo, sardas, rugosidade, especular, oleosidade e subsurface. Os ajustes são enviados em tempo real ao shader WGSL da pré-visualização.

Os parâmetros são salvos em presets JSON versão 6. Presets versões 1 a 5 são migrados automaticamente, preservando os valores antigos e preenchendo os controles novos com padrões derivados do `__init__.py` original. Hemoglobina e sardas são efeitos procedurais de look development; ainda não representam máscaras anatômicas ou texturas calibradas.

## Tecnologias

- Rust stable (a versão atual do `eframe` requer Rust 1.95 ou superior).
- `egui` / `eframe` para a interface.
- `wgpu` e WGSL para o renderizador GPU próprio do aplicativo.
- `bytemuck` para empacotar uniformes enviados ao shader.

## Executar localmente

Instale Rust stable com `rustup` e depois, na raiz do repositório:

```bash
cargo fmt --all
cargo test --all-targets
cargo run --release
```

O primeiro build baixa as dependências do Cargo. No Linux, a janela precisa das bibliotecas de sistema usadas por winit/wgpu; o workflow instala as dependências no runner do GitHub Actions.

## Build no GitHub

1. Crie um repositório vazio no GitHub.
2. Extraia este pacote e envie todos os arquivos para a branch `main`.
3. Abra **Actions** e acompanhe `Build Skin Studio RS`.
4. Os executáveis de cada sistema aparecerão como artefatos da execução.
5. Para gerar uma Release com os binários anexados, crie e envie uma tag, por exemplo:

```bash
git tag v0.1.0
git push origin v0.1.0
```

O workflow compila para os sistemas operacionais usando runners do GitHub. Isso não garante compatibilidade com toda placa de vídeo; os drivers locais precisam implementar um backend suportado pelo `wgpu`.

## Direção de desenvolvimento

Veja [`docs/ROADMAP.md`](docs/ROADMAP.md) e [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
