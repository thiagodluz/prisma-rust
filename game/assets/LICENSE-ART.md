# Imagens do Prisma em Rust — licença

As imagens desta pasta são **adaptações** das imagens do Prisma em JS
(<https://github.com/thiagodluz/Prisma>), © 2026 Thiago Luz, e continuam sob a
[Creative Commons Atribuição-CompartilhaIgual 4.0 Internacional (CC BY-SA 4.0)](https://creativecommons.org/licenses/by-sa/4.0/legalcode).

| Arquivos aqui | Vêm de (no projeto original) |
| --- | --- |
| `gem-0.png` a `gem-6.png` | `gem-atlas.webp` (pedras comuns) |
| `burst-0.png` a `burst-6.png` | `burst-atlas.webp` (Pulso) |
| `cross-0.png` a `cross-6.png` | `cross-atlas.webp` (Raio) |
| `spectrum.png` | `spectrum-gem.webp` (Espectro) |
| `prisma-bg.jpg` | `prisma-bg.jpg` (foto de fundo do jogo; copiada sem alteração) |
| `marca.png` | `icon-192.png` (logo do topo; copiada sem alteração) |

## O que foi alterado

- cada pedra foi recortada do atlas e guardada em um arquivo próprio;
- redimensionada e centralizada numa tela transparente de 160×160 pixels;
- convertida de WebP para PNG.

Nada de cor, forma ou desenho foi alterado. O programa que faz isso é o `prisma_tools imagens` (pasta `tools/`, escrito em Rust).

## O que a licença não cobre

O nome do jogo e os ícones (`icon-*.png`, usados em `game/android_res/`) ficam fora da
licença, como no projeto original.

Quem redistribuir estas imagens ou adaptações delas deve atribuir a autoria a Thiago Luz,
indicar a licença e manter a CC BY-SA 4.0 (ou uma licença compatível).

## Fontes Inter e Sora

`Inter-Regular.ttf` e `Inter-SemiBold.ttf` são instâncias estáticas (peso 400 e 600, `opsz` 14) da
fonte variável Inter, de The Inter Project Authors (<https://github.com/rsms/inter>); `Sora-Bold.ttf`
é a instância de peso 700 da fonte variável Sora, de The Sora Project Authors
(<https://github.com/sora-xor/sora-font>). Todas foram geradas com `fonttools varLib.instancer` e
reduzidas ao Latim (ASCII, Latin-1 e pontuação comum) com `pyftsubset`, sem hinting e sem recursos
OpenType. Estão sob a [SIL Open Font License 1.1](OFL-Inter.txt) (Inter) e a
[SIL Open Font License 1.1](OFL-Sora.txt) (Sora), e não sob a CC BY-SA das imagens acima.
