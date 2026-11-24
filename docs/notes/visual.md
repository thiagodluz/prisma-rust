# Visual da fase 3 (V1): o que existe e como conferir

A tela de jogo imita o estilo "Novo" do Prisma em JS. As formas (moldura, painel escuro, casas,
cartão, botões) **não são imagens**: `game/src/formas.rs` pinta cada uma uma vez, ao abrir o jogo
(`Formas::load` em `render.rs`), e `desenhar_forma` a estica em nove fatias (cantos sem entortar).
Só a foto de fundo é arquivo: `game/assets/prisma-bg.jpg` (CC BY-SA, citada em `LICENSE-ART.md`).

## Receitas das formas (hex `RRGGBBAA`, do CSS do JS)

| Forma | lado / raio / borda / sombra (px da imagem) | degradê (3 cores) | borda |
| --- | --- | --- | --- |
| moldura | 168 / 22 / 2 / 20 (sombra `030616bd`), ângulo 145 | `7790bd65` `202a4eff` `785b9b72` | `cee6ff5c` |
| grade (painel escuro) | 64 / 14 / 0 / 0, ângulo 145 | `0d142cff` ×3 | – |
| casa (índice ímpar) | 64 / 12 / 0 / 0, ângulo 155 | `bfd5ff19` `869ec912` `4d67940b` | – |
| casa (índice par) | igual | `c8c1ff1b` `8a94c913` `4d67940b` | – |
| cartão de pontos | 64 / 12 / 1 / 0, ângulo 145 | `ffffff0d` ×3 | `ffffff17` |
| botão de ação | 64 / 12 / 1 / 0, ângulo 155 | `60538763` `443e6988` `292a4cad` | `b8a7ea70` |
| sólida (tingida) | 64 / 12 / 0 / 0 | `ffffffff` ×3 | – |

Para mudar o visual de uma forma, mude só a receita em `Formas::load`; o layout (`layout.rs`) não
muda. A moldura (`tela.tabuleiro`) é só desenho; **toque e posição das pedras contam a partir da
grade** (`tela.grade`, a moldura encolhida em 3,5%).

## Como ver (sem celular)

```
export RUSTFLAGS="-L /tmp/asound" LIBRARY_PATH=/tmp/asound LIBGL_ALWAYS_SOFTWARE=1
xvfb-run -a -s "-screen 0 1280x2600x24" cargo run -q -p prisma_game -- \
  --cena inicio --screenshot foto.png --tamanho 360x640
```

Cenas: `inicio`, `selecionada`, `dica`, `troca`, `limpando`, `queda`, `nivel`, `fim`, `dialogo`,
`classico`, `som`, `recordes`, `creditos`, `respiracao`, `particulas-pulso`, `particulas-raio`.
Referências: `docs/notes/ref/js-novo.jpg` (JS) e `docs/notes/ref/rust-m4.png` (Rust antes).
O aviso de áudio no terminal ("Can't open PCM device") é esperado aqui e não impede a foto.

## O que só o celular confirma

- A **foto de fundo em JPEG** decodificando no Android (a crate `image` ganhou o leitor de JPEG);
  o CI com os celulares virtuais Android 34 e 30 tira as fotos da tela nova.
- Suavidade (as formas são texturas, nada é gerado por quadro) e tempo de abertura (agora o jogo
  pinta 7 formas e decodifica a foto ao abrir).
- Legibilidade das letras sobre a foto num celular de verdade.

## O que fica para a V2

Topo com logo e PRISMA, cartões de modo com ícone e frase, e a fonte Roboto (hoje a letra é a
embutida do Macroquad, sem negrito nem espaçamento entre letras).
