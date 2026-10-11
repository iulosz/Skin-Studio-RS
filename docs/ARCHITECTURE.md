# Arquitetura técnica

## Princípio

O projeto não chama `bpy`, não abre o Blender e não usa Cycles/Eevee. `wgpu` é a abstração de baixo nível da GPU; a lógica de renderização do material de pele pertence ao projeto (`src/render/skin_preview.wgsl`). O primeiro preview usa um raio por pixel para intersectar uma esfera procedural, permitindo iterar rapidamente em pigmentação e resposta da superfície.

## Camadas previstas

```text
skin-studio-rs/
├── src/app.rs               # estado e interface do estúdio
├── src/render/               # renderização GPU e shaders WGSL
├── src/material.rs           # parâmetros tipados e presets versionados
├── src/geometry/             # malhas, normais, UVs e importação (próxima etapa)
├── src/texture/              # imagens e bake de mapas (próxima etapa)
└── .github/workflows/        # testes e build multiplataforma
```

## Renderizador: etapas

1. **Preview procedural** (implementado): ray/sphere por pixel, iluminação difusa, especular, variação de pigmento, hemoglobina aproximada, sardas procedurais e detalhe de poros.
2. **Malhas reais**: importação de glTF/OBJ, buffers de vértices/índices, câmera orbital, depth buffer e desenho de triângulos.
3. **Material PBR**: fluxo de Base Color/Roughness/Normal, gestão explícita de espaço de cor, fontes de luz e tone mapping.
4. **Pele**: pigmentos em múltiplas escalas, máscaras anatômicas, poros por região, rugas, vasos, sardas, lábios e o modelo SSS escolhido.
5. **Baker independente**: UVs, padding/margem, Base Color, Normal tangencial e Roughness. Alpha poderá ser incluído quando o formato de geometria/material e o caso de uso forem definidos.
6. **Render final**: separar a pré-visualização em tempo real do caminho de render de qualidade final. Caso seja necessário path tracing offline, ele deve ser um subsistema próprio, não uma alegação implícita de que o preview atual já é um path tracer.

## Por que o protótipo não é uma conversão 1:1

O addon original é um construtor de nós e chama operadores do Blender para meshes, atributos, UVs e bake. Não existe uma conversão sintática direta para Rust: cada efeito precisa de uma definição independente de parâmetros, uma implementação de shader/CPU e testes de comparação visual. Os valores de calibração e tabelas devem ser portados com testes de regressão, preservando nomes e significado. Como exemplo obrigatório, a correção da linha Scalp registrada em 2.4.76 usa Hue +0.0056, Saturation 0.8658 e Value 0.9129; ela deve ser mantida, e não substituída pelos valores antigos da 2.4.74.
