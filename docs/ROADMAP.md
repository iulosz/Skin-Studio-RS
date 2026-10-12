# Roadmap — migração Skin Studio 2.4.76 para Rust

## M0 — Fundação desktop e GPU
- [x] Aplicativo desktop sem dependência do Blender em runtime.
- [x] UI com as sete áreas principais do addon.
- [x] Renderer WGSL procedural no viewport.
- [x] Controles vivos para cor, poros, micro-relevo, roughness e especular.
- [x] GitHub Actions para testes e builds Windows/Linux/macOS.
- [x] Executar builds no runner do GitHub e corrigir incompatibilidades de toolchain/compilação conhecidas.

## M1 — Dados, presets e estabilidade
- [x] Estruturas Rust tipadas para parâmetros de material.
- [x] Serialização de presets JSON com versionamento.
- [x] Presets versões 1 a 7 migrados automaticamente para o esquema atual.
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
- [x] Gerador de mapa de altura de poros tileável portado para Rust com resolução, densidade, relevo e seed configuráveis.
- [x] Três populações de poros Voronoi (grandes, primários e finos), domain warp e variação de tamanho portados para WGSL.
- [x] Contribuição de poros para normal, albedo e oclusão especular; controles originais relevantes expostos.
- [x] Heurísticas aproximadas de distribuição regional baseadas no bounding box.
- [x] Adicionar escala física M/CM/MM/custom equivalente ao Coord Scale.
- [ ] Substituir heurísticas por máscaras anatômicas importadas/pintáveis.
- [x] Portar o estilo SKIN do mapa Reference Height: duas redes Worley, sulcos, poros nas junções, ruído periódico filtrado por FFT e upload linear R16Float.
- [ ] Portar o estilo LEATHER da Reference Height com seus parâmetros e calibração independentes.
- [x] Portar Skin Line Network com platôs, sulcos celulares em duas escalas, vincos e ajuste físico de relevo.
- [x] Portar build_color_layer(): variação macro, melanina, eritema, ouro, microcor, vermelhidão e Mottled em três escalas.
- [x] Portar broad/medium/micro Albedo Realism e canais hemoglobina/periorbital/melanina; preservar UVs OBJ/glTF.
- [x] Importar dois atlas anatômicos RGBA por UV (nariz/bochechas/testa/queixo + orelhas/olhos/pescoço/cobertura), com mistura controlável e fallback heurístico; pintura direta no app permanece pendente.
- [x] Portar Crosshatch direcional triplanar com orientação, escala, domain warp, breakup, relevo e contribuição à roughness.
- [x] Portar micro-rugas anisotrópicas com orientação/escala, continuidade variável, relevo e contribuição à roughness; ainda requer comparação visual com o addon Blender.
- [ ] Lábios, olheiras, vascularização, sardas e microvariação.
- [ ] Roughness, specular/coat e subsurface com limites calibrados.
- [ ] Comparação lado a lado com Blender usando cabeça, escala, iluminação e câmera controladas, principalmente para Reference Height e tamanho físico das células.
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
