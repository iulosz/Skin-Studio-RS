# Roadmap — migração Skin Studio 2.4.76 para Rust

## M0 — Fundação desktop e GPU
- [x] Aplicativo desktop sem dependência do Blender em runtime.
- [x] UI com as sete áreas principais do addon.
- [x] Renderer WGSL procedural no viewport.
- [x] Controles vivos para cor, poros, micro-relevo, roughness e especular.
- [x] GitHub Actions para testes e builds Windows/Linux/macOS.
- [ ] Executar o primeiro build no runner do GitHub e corrigir qualquer incompatibilidade de toolchain/driver que surgir.

## M1 — Dados, presets e estabilidade
- [x] Estruturas Rust tipadas para parâmetros de material.
- [x] Serialização de presets JSON com versionamento.
- [x] Presets versão 1 migrados automaticamente para o esquema atual.
- [x] Testes unitários para faixas válidas, defaults e migração de preset.

## M1.5 — Editor inicial de parâmetros de pele
- [x] Controles para Base Color, hemoglobina aproximada e variação de pigmentação.
- [x] Controles de poros, micro-relevo e sardas procedurais.
- [x] Controles de roughness, especular, oleosidade e subsurface aproximado.
- [x] Parâmetros ligados em tempo real ao shader de pré-visualização WGSL.
- [ ] Validar visualmente no build Windows e refinar a resposta dos controles.

## M2 — Malhas e viewport 3D
- [x] Importar malhas triangulares OBJ e glTF/GLB.
- [x] Criar buffers de vértices/índices e normalizar escala/centro para a pré-visualização.
- [x] Câmera orbital, zoom, pan por Shift+arraste e restauração de enquadramento.
- [x] Grade 3D desenhada na GPU e depth buffer para superfícies.
- [x] Carregamento de normais existentes e geração de normais ausentes.
- [ ] Tangentes, seleção de múltiplos objetos e materiais por primitiva.
- [ ] Interface preparada para receber novas referências visuais do usuário.

## M3 — Material de pele
- [ ] Base Color por pigmentação em escalas múltiplas.
- [ ] Portar a calibração Scalp do Skin Studio 2.4.76 sem regressão: Hue +0.0056, Saturation 0.8658, Value 0.9129; validar os alvos de cor documentados na cabeça de teste.
- [ ] Máscaras anatômicas e mistura suave entre regiões.
- [ ] Poros grandes, primários e finos com relevo controlável.
- [ ] Skin Line Network e mapa de altura de referência tileável.
- [ ] Lábios, olheiras, vascularização, sardas e microvariação.
- [ ] Roughness, specular/coat e subsurface com limites calibrados.
- [ ] Comparação visual automatizada por imagens de referência controladas.
- [ ] Testes de regressão para os parâmetros/tabelas calibrados existentes no código e no changelog 2.4.76.

## M4 — Texturas
- [ ] Bake Base Color.
- [ ] Bake Normal em tangent space.
- [ ] Bake Roughness.
- [ ] Margem/padding, prevenção de seams/halos, resolução configurável e exportação PNG.
- [ ] Alpha quando necessário e com semântica definida.

## M5 — Render final e distribuição
- [ ] Render de qualidade superior separado da pré-visualização interativa.
- [ ] Iluminação configurável, sombras e gerenciamento de cor.
- [ ] Profiling de CPU/GPU e tratamento de dispositivos sem suporte.
- [ ] Instalador/empacotamento por plataforma e releases versionadas.
