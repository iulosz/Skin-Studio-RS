# Arquitetura técnica

## Princípio

O projeto não chama `bpy`, não abre o Blender e não usa Cycles/Eevee. `wgpu` é a abstração de baixo nível da GPU; os shaders próprios do projeto compõem a visualização do material. O viewport atual rasteriza uma malha triangular no alvo offscreen com teste de profundidade e depois compõe esse alvo na interface. Uma esfera triangulada serve como modelo padrão; OBJ e glTF/GLB podem ser importados pelo seletor de arquivos.

## Camadas previstas

```text
skin-studio-rs/
├── src/app.rs               # estado e interface do estúdio
├── src/render/preview.rs     # viewport GPU, buffers, câmera e recursos WGPU
├── src/render/*.wgsl         # shader de pele, grade e composição do viewport
├── src/skin_detail.rs        # geração CPU de mapa de altura tileável de poros
├── src/material.rs           # parâmetros tipados e presets versionados
├── src/geometry.rs           # importação OBJ/glTF/GLB e atributos da malha
└── .github/workflows/        # testes, validação WGSL e build multiplataforma
```

## Renderizador: etapas

1. **Viewport rasterizado** (implementado): malhas triangulares, alvo de profundidade, câmera orbital, grade, shader de pele aproximado e normalização de modelos importados.
2. **Malhas reais** (base implementada): importação de glTF/GLB e OBJ, buffers de vértices/índices, normais, câmera orbital e depth buffer. Tangentes, seleção de múltiplos objetos e materiais por primitiva continuam pendentes.
3. **Material PBR**: fluxo de Base Color/Roughness/Normal, gestão explícita de espaço de cor, fontes de luz e tone mapping.
4. **Pele**: migração por subsistemas do `__init__.py`, preservando valores, frequência física, seeds e significado dos controles. A primeira parte do sistema de poros foi portada: mapa de altura tileável, três populações Voronoi, deformação de domínio, variação de tamanho, relevo, escurecimento de abertura, oclusão especular e controles regionais aproximados. Referência de altura, Skin Line Network, crosshatch, rugas, pigmentação/anatomia, vasos, sardas, lábios e SSS permanecem etapas separadas.
5. **Baker independente**: UVs, padding/margem, Base Color, Normal tangencial e Roughness. Alpha poderá ser incluído quando o formato de geometria/material e o caso de uso forem definidos.
6. **Render final**: separar a pré-visualização em tempo real do caminho de render de qualidade final. Caso seja necessário path tracing offline, ele deve ser um subsistema próprio, não uma alegação implícita de que o preview atual já é um path tracer.


## Porta de poros do Skin Studio 2.4.76

O código original de referência é `__init__.py`, especialmente `generate_pore_normal_image()`, `build_detail_layer()`, `REGIONAL_RELIEF_TABLE`, `_region_weight()` e `_patch_skin_coordinates()`.

- `src/skin_detail.rs` implementa o gerador CPU de altura tileável: células Voronoi com F1/F2, domos, sulcos, depressões e micro-ondas periódicas. A saída é um canal linear, convertido para R16Float no upload WGPU, não uma imagem sRGB de cor.
- `src/render/mesh_preview.wgsl` implementa três escalas de poros Voronoi (primária, fina e grande), warp de coordenadas, variação de tamanho, projeção triplanar do mapa de altura e contribuições separadas de fine bump e micro-grain. A máscara dos poros também altera discretamente o albedo e o especular.
- `MeshVertex.skin_position` preserva as coordenadas de origem para que a normalização feita só para enquadrar a viewport não mude o tamanho do detalhe. Neste estágio, OBJ/glTF é interpretado em metros; o controle equivalente ao `coord_unit` do addon ainda precisa ser adicionado.
- `MeshVertex.region_position` permite usar as heurísticas de região derivadas de `_region_weight()`. Essas máscaras são aproximadas pelo bounding box e orientação -Y; não são máscaras pintadas nem equivalem a grupos anatômicos reais. Por isso a interface identifica essa limitação.

## Critérios de fidelidade

A compilação não prova equivalência visual. Cada camada portada deve manter o conjunto de controles do addon, passar validações de parâmetros/presets, ter testes determinísticos para geradores CPU e ser comparada lado a lado com uma cena de referência do Blender. Nenhum subsistema será marcado como concluído apenas porque o shader compila.

## Por que o protótipo não é uma conversão 1:1

O addon original é um construtor de nós e chama operadores do Blender para meshes, atributos, UVs e bake. Não existe uma conversão sintática direta para Rust: cada efeito precisa de uma definição independente de parâmetros, uma implementação de shader/CPU e testes de comparação visual. Os valores de calibração e tabelas devem ser portados com testes de regressão, preservando nomes e significado. Como exemplo obrigatório, a correção da linha Scalp registrada em 2.4.76 usa Hue +0.0056, Saturation 0.8658 e Value 0.9129; ela deve ser mantida, e não substituída pelos valores antigos da 2.4.74.
