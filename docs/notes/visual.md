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

## V2 (pronta, falta o celular)

- **Letra (trocada na V3, veja abaixo):** Roboto Regular e Bold embutidas, tudo via
  `texto.rs` (`desenhar`, `medir`, `tamanho_que_cabe`). A fonte não tem ♫ nem ≡: esses símbolos
  são desenhados por código (`icones.rs`). O teste `todo_texto_do_jogo_tem_glifo_nas_fontes_embutidas` confere
  os caracteres não-ASCII entre aspas nos arquivos que escrevem texto (render, menu, app, cenas,
  preferencias, fundo, layout) contra as fontes; ao criar um arquivo novo com texto, some-o à lista.
- **Topo:** logo (`marca.png`) + "PRISMA" em negrito com espaço entre letras + botões ♫ e ≡.
  Em tela deitada os itens do topo encolhem para a marca ter largura.
- **Cartões de modo** (Zen e Clássico) ficam embaixo, depois da guia de respiração: caixa com ícone
  (lua e faísca desenhadas por código), nome em negrito e frase. Receitas novas em `Formas`:
  `modo` e `modo_escolhido`. Altura `MODOS_ALTURA` = 0,13 (com 0,11 a frase ficava com 8 px).

Só o celular confirma: a Roboto no Android (o CI tira as fotos), o tempo de abertura com duas
fontes e a frase do Clássico (a mais longa) no menor celular.

## V3 (pronta, falta o celular)

Objetivo do Thiago: "profissional e polido", mantendo o layout e as pedras.

- **Fontes (`texto.rs`, `game/assets/`):** três estilos em `enum Estilo`. `Texto` = Inter Regular
  (frases, rótulos), `TextoForte` = Inter SemiBold (botões, nomes), `Titulo` = Sora Bold ("PRISMA",
  números do placar, títulos de cartões e folhas). São instâncias estáticas reduzidas ao Latim
  (ASCII, Latin-1 e pontuação comum), sem hinting e sem recursos OpenType, ~17 KB cada; licenças
  OFL em `OFL-Inter.txt` e `OFL-Sora.txt`, créditos em `LICENSE-ART.md`. O Macroquad não tem
  números tabulares, então o placar usa Sora (números de largura parecida). O teste de glifos
  confere as três fontes; se uma fonte falhar ao carregar, vale a padrão do Macroquad.
- **Vidro fosco (`render.rs`):** `receita_de_vidro(raio, preenchimento, borda)` = borda de 1 px,
  degradê de três paradas (brilho no topo, base mais escura) e sombra suave de 6 px; alimenta
  `cartao`, `botao_icone(_ativo)`, `modo(_escolhido)`. `desenhar_vidro` passa a sombra na
  proporção da receita (se passasse 0 a sombra seria espremida para dentro). `painel` (raio 20) é
  o vidro grande do cartão de fim, do diálogo, das folhas e do aviso; `botao_principal` é o
  degradê azul-lilás de "Jogar novamente", "Novo jogo" e da aba aberta (`0x4d5df0`→`0x7c4fe6`,
  escurecido em relação ao plano para o texto branco passar de 4,5 para 1).
- **Contraste:** `contraste::razao` (WCAG) e `PARES_DE_TEXTO` (nome, texto, fundo) em
  `render.rs`; o teste exige 4,5 para 1 em todos. Cores de texto: `TEXTO_PRINCIPAL`,
  `TEXTO_SECUNDARIO`, `TEXTO_ROTULO`. Ao criar uma cor de texto nova, some-a aos pares.
- **Placar e barra:** rótulos em `TEXTO_ROTULO` com letras espaçadas, números em Sora que encolhem
  (`tamanho_do_valor`) para caber na coluna, duas divisórias; barra de progresso mais alta
  (`BARRA_ALTURA` 0,030) com trilho de vidro.
- **Botões e cartões de modo:** rótulos a 0,9 do texto com margem de 10%; frases curtas
  ("Sem derrota", "Pode acabar") porque as antigas não cabiam no celular de 360 px. Os testes
  conferem as telas em pé (320×480 a 1080×2400); em tela deitada (800×600, só no computador) a
  coluna do jogo fica estreita e o texto encolhe, como já era antes.
- **Anel (`Formas.anel`, `anel_da_casa`):** borda branca e brilho lilás em volta da casa
  selecionada; o alvo do arrasto usa o anel verde-água a 80% e a Dica o dourado pulsando.
- **Película do fim:** arredondada como a moldura do tabuleiro (`raio_da_moldura`).
- **Limite do APK:** 100 MB (decisão do Thiago).

Só o celular confirma: as fontes no Android (o CI tira as fotos), o tempo de abertura (agora ~14
formas pintadas ao abrir), o vidro agradar aos olhos e as frases curtas dos cartões de modo.
