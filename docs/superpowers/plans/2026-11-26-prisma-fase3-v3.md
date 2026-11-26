# Prisma Fase 3 V3 (polimento) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline, o jeito que o Thiago escolheu na V2) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deixar a tela do jogo com aparência profissional e polida: fontes Inter e Sora, acabamento de vidro fosco, hierarquia e contraste melhores, sem mexer no layout geral nem nas pedras.

**Architecture:** Tudo no crate `game/`. `texto.rs` troca Roboto por três estilos (Inter Regular, Inter SemiBold, Sora Bold). `render.rs` ganha receitas de vidro (`Formas`/`Receita`, pintadas uma vez em textura) e passa a usá-las no placar, botões, cartões, anel da peça selecionada, fim de partida, diálogo e folhas. Contraste de texto vira constantes nomeadas + teste.

**Tech Stack:** Rust, Macroquad 0.4.16 (fontdue por baixo, sem recursos OpenType), fonttools (Python) para reduzir as fontes.

**Spec:** `docs/superpowers/specs/2026-11-25-prisma-fase3-v3-polimento-design.md`

## Global Constraints

- Layout geral e pedras ficam como estão (V2). Nenhum texto menor que hoje; alvo de toque dos botões não diminui.
- Limite do APK: **100 MB** (hoje 54,7 MB). O CI hoje barra em 60 MB (`android.yml` linhas ~190-212): passa a 100.
- Fontes livres (OFL) embutidas: Inter Regular e SemiBold, Sora Bold, reduzidas ao Latim; a Roboto sai (arquivos, licença, texto em `LICENSE-ART.md`).
- Texto secundário com contraste ≥ 4,5:1 sobre o fundo do seu painel (pior caso: fundo claro `0x2a2250`).
- Env por comando: `export RUSTFLAGS="-L /tmp/asound" LIBRARY_PATH=/tmp/asound LIBGL_ALWAYS_SOFTWARE=1`. Fotos: `xvfb-run -a -s "-screen 0 1280x2600x24" cargo run -q -p prisma_game -- --cena NOME --screenshot arq.png --tamanho LxA` (cenas: inicio, selecionada, dica, troca, limpando, queda, nivel, fim, dialogo, classico, som, recordes, creditos, respiracao). `game/` e `tools/` não passam no rustfmt: não reformatar. Clippy do `game` tem 9 linhas distintas de aviso de base: nenhum aviso novo.
- Commits em português (`feat:`/`fix:`/`docs:`/`refactor:`/`test:`), terminando com os dois trailers (`Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` e `Claude-Session: https://claude.ai/code/session_01Y38ZhQcV92bomwv9cFgeqB`). Nada vai ao GitHub sem autorização nova do Thiago.

## Review Focus

1. **Placar com número enorme** (ex. 12.345.678 pontos, nível 100): não pode passar da coluna nem cobrir o vizinho; encolhe a letra. Teste na Tarefa 2.
2. **Telas fora do retrato comum** (320×480 e 800×600 deitada): nada corta nem se sobrepõe; os textos só encolhem. Foto + teste de layout na Tarefa 3.
3. **Fonte que falha ao carregar:** o jogo continua desenhando com a fonte padrão do Macroquad, sem pânico. Teste na Tarefa 1.
4. **Botão desabilitado** (Dica com partida acabada): mesmo tamanho e posição, só apagado. Foto na Tarefa 3.
5. **Acento e símbolos** (Í, ç, ã, ·, —, …) existem nas três fontes. Teste de glifos na Tarefa 1.

---

### Task 1: Fontes Inter e Sora no lugar da Roboto

**Files:**
- Create: `game/assets/Inter-Regular.ttf`, `Inter-SemiBold.ttf`, `Sora-Bold.ttf`, `OFL-Inter.txt`, `OFL-Sora.txt`
- Delete: `game/assets/Roboto-Regular.ttf`, `Roboto-Bold.ttf`, `OFL.txt`
- Modify: `game/src/texto.rs`, `game/src/render.rs` e demais usos de `Peso`, `LICENSE-ART.md`, `.github/workflows/android.yml` (limite 60→100 MB), `docs/notes/android-build.md` (se citar 60 MB)
- Test: `game/src/texto.rs` (módulo de testes)

**Interfaces:**
- Consumes: `texto::{medir, desenhar, tamanho_que_cabe, carregar}` da V2.
- Produces: `pub enum Estilo { Texto, TextoForte, Titulo }` no lugar de `Peso` (`Normal`→`Texto`, `Negrito`→`TextoForte`); `medir(texto, tamanho, estilo: Estilo)` e `desenhar(texto, x, y, tamanho, cor, estilo: Estilo)`; `render.rs`: `escrever_com_peso` vira `escrever_com_estilo(texto, area, tamanho, cor, alinhamento, estilo: Estilo)`; `escrever(..)` continua = `Estilo::Texto`.

- [ ] **Step 1: Preparar as fontes** (fora do git, em `/tmp/fontes`; os baixados já estão lá: `Inter[opsz,wght].ttf`, `Sora[wght].ttf`; as licenças OFL vêm do zip do Inter 4.0 e do repositório `google/fonts/ofl/sora/OFL.txt`). Com `fontTools.varLib.instancer` fixar Inter em wght 400 e 600 (opsz 14) e Sora em wght 700; com `pyftsubset --unicodes=U+0020-007E,U+00A0-00FF,U+2013,U+2014,U+2018,U+2019,U+201C,U+201D,U+2022,U+2026,U+00B7 --layout-features= ` reduzir cada uma; salvar os três `.ttf` estáticos em `game/assets/`. Conferir `ls -la` (meta: as três somam < 400 KB).
- [ ] **Step 2: Escrever os testes (vermelhos)** em `texto.rs`: `as_fontes_embutidas_sao_estaticas` passa a olhar as três (sem tabela `fvar`); `todo_texto_do_jogo_tem_glifo_nas_fontes_embutidas` (renomeado) confere cada caractere não-ASCII achado por `caracteres_de_texto_do_jogo()` nas TRÊS fontes; `a_varredura_pega_um_glifo_que_nao_existe` troca Roboto por Inter e continua provando que `♫` e `≡` faltam; `sem_fonte_carregada_medir_e_desenhar_nao_entram_em_panico`: chama `medir("Olá", 20.0, Estilo::Titulo)` sem `carregar()` e confere `width > 0.0`. Rodar `cargo test -p prisma_game texto`: falha (não compila).
- [ ] **Step 3: Implementar** `Estilo` e as três constantes `include_bytes!` em `texto.rs`; `FONTES` guarda `(Font, Font, Font)`; `carregar()` só guarda se as três carregarem (senão fica o fallback). Trocar todos os `Peso::Normal/Negrito` por `Estilo::Texto/TextoForte` nos arquivos que usam (`grep -rn "Peso" game/src`), renomear `escrever_com_peso`. Neste passo `PRISMA` (topo) e os números do placar passam a `Estilo::Titulo`. Remover os arquivos da Roboto e a licença antiga; atualizar `LICENSE-ART.md` (seções Inter e Sora, fonte, licença OFL, o que é cada arquivo). Em `android.yml`: limite 60→100 MB na verificação e nos textos do comentário e da mensagem.
- [ ] **Step 4: Verificar:** `cargo test --workspace` verde; `cargo clippy -p prisma_game --all-targets` sem aviso novo; foto `inicio` em 360×640: título e placar em Sora, resto em Inter, sem quadradinho. Mutação: trocar Sora por Inter no `Titulo` ou apagar um acento do subset e ver o teste de glifos falhar.
- [ ] **Step 5: Commit** `feat: fontes Inter e Sora no lugar da Roboto`.

### Task 2: Receita de vidro, placar e barra de progresso

**Files:**
- Modify: `game/src/render.rs` (`Formas::load`, `desenhar_cabecalho`, constantes de cor), `game/src/layout.rs` (altura da barra), `game/src/formas.rs` só se faltar parâmetro
- Create: `game/src/contraste.rs`
- Test: `game/src/contraste.rs`, `game/src/layout.rs`, `game/src/render.rs` (módulo de testes)

**Interfaces:**
- Consumes: `Receita`/`pintar` de `formas.rs`; `Estilo` da Tarefa 1; `tamanho_que_cabe`.
- Produces: `contraste::razao(a: [u8; 3], b: [u8; 3]) -> f32` (razão WCAG, 1.0 a 21.0); `fn receita_de_vidro(raio: u32, preenchimento: Cor, borda: Cor) -> Receita` em `render.rs` (borda 1 px; degradê de 3 paradas com brilho no topo: topo = preenchimento mais claro, meio = preenchimento, base = preenchimento mais escuro; sombra suave de 6 px, `0x030616` com alfa baixo); constantes `TEXTO_PRINCIPAL = 0xf4f2ff`, `TEXTO_SECUNDARIO = 0xc9c6e6`, `TEXTO_ROTULO = 0xaaa6cf` e `PARES_DE_TEXTO: [(&str, u32, u32); N]` (nome, texto, fundo do painel já misturado com `0x2a2250`).

- [ ] **Step 1: Testes vermelhos.** `contraste`: `razao([255,255,255],[0,0,0])` ≈ 21,0 (±0,01); `razao(a,a)` = 1,0; simétrica. `render`: `todo_texto_passa_de_4_5_para_1_de_contraste`: percorre `PARES_DE_TEXTO` e afirma ≥ 4,5 mostrando o nome do par que falhar. `layout`: `a_barra_de_progresso_e_mais_alta_que_o_traco_de_antes` (`tela.barra.h >= 0.012 * lado_menor` em 360×640 e 1080×2400). `render`: `o_placar_encolhe_numero_enorme_para_caber_na_coluna`: função pura nova `tamanho_do_valor(valor: &str, coluna_w: f32, tamanho: f32) -> f32` (usa `texto::medir` Titulo + `tamanho_que_cabe`) devolve ≤ `tamanho` e, para "12.345.678" numa coluna de 100 px a 40 px de letra, uma largura medida ≤ 100 px.
- [ ] **Step 2: Rodar** (falham: não compila).
- [ ] **Step 3: Implementar** `contraste.rs` (luminância relativa sRGB WCAG). Em `render.rs`: `receita_de_vidro`; refazer as receitas `cartao`, `botao`, `botao_icone`, `botao_icone_ativo`, `modo`, `modo_escolhido` a partir dela (mesmas tintas de hoje, só com brilho no topo e sombra suave; o ativo/escolhido com borda lilás `0xc4b2ff` e preenchimento mais claro). Placar: rótulos com 0,06·altura de respiro em cima, em `TEXTO_ROTULO`, 0,62·t com espaço entre letras 0,12 em; valores em `Estilo::Titulo` usando `tamanho_do_valor`; duas divisórias verticais de 1 px (`0xffffff1f`) entre as colunas. Barra: trilho `0xffffff14` com borda 1 px `0xffffff26`, altura nova em `layout.rs` (ajustar `fixo` e as posições esperadas dos testes, como na V2), texto do progresso 0,86·t em `TEXTO_SECUNDARIO`. Trocar as cores de texto soltas pelas constantes.
- [ ] **Step 4: Verificar:** testes, clippy, fotos `inicio` e `fim` (placar com 4.321) em 360×640 e 1080×2400; nada cola na borda do cartão. Mutação: baixar `TEXTO_ROTULO` para `0x6a6890` e ver o teste de contraste falhar.
- [ ] **Step 5: Commit** `feat: vidro fosco, placar e barra de progresso mais legíveis`.

### Task 3: Botões de ação e cartões de modo

**Files:**
- Modify: `game/src/render.rs` (`desenhar_botao_de_acao`, `desenhar_cartao_de_modo`), `game/src/layout.rs` se a altura precisar
- Test: `game/src/render.rs`, `game/src/layout.rs`

**Interfaces:**
- Consumes: `receita_de_vidro`, `tamanho_do_valor`-style helper e constantes da Tarefa 2.
- Produces: `fn rotulo_cabe(rotulo: &str, largura: f32, tamanho: f32, estilo: Estilo) -> bool`; texto do Clássico definido em `const FRASE_CLASSICO: &str` (começa "Sem cronômetro · pode acabar").

- [ ] **Step 1: Testes vermelhos:** `os_rotulos_dos_botoes_cabem_em_todas_as_telas`: para 320×480, 360×640, 540×960, 800×600 (deitada) e 1080×2400, `rotulo_cabe` é verdadeiro para "Embaralhar", "Dica", "Novo jogo" no tamanho padrão `tela.texto` dentro da largura do botão menos 2·margem; `a_frase_do_classico_cabe_nos_cartoes_de_modo`: em cada tela acima, `FRASE_CLASSICO` medida em Inter Texto a ≥ 0,20·altura do cartão cabe na largura útil; se algum tamanho falhar, o teste ensina a encurtar a frase (ruling no ledger: vira "Sem cronômetro, pode acabar" e depois "Sem cronômetro").
- [ ] **Step 2: Rodar** e ver falhar (não compila / falha).
- [ ] **Step 3: Implementar:** botão de ação com ícone em 0,36·altura no alto, rótulo em `Estilo::TextoForte` SemiBold 0,9·`tela.texto`, margem interna lateral 0,1·largura; apagado = 40% de opacidade sem mudar medidas; cartões de modo: nome em `TextoForte`, frase em `Texto` a 0,24·altura em `TEXTO_SECUNDARIO`, ativo com borda lilás da Tarefa 2.
- [ ] **Step 4: Verificar:** testes e fotos `inicio`, `classico`, `fim` (Dica apagada) em 360×640 e 1080×2400, mais `inicio` em 320×480 e 800×600. Mutação: aumentar o tamanho do rótulo para 1,6·t e ver o teste falhar.
- [ ] **Step 5: Commit** `feat: botões de ação e cartões de modo mais limpos`.

### Task 4: Anel luminoso na peça selecionada

**Files:**
- Modify: `game/src/render.rs` (`Formas`, `desenhar_contornos_de_realce`)
- Test: `game/src/render.rs`

**Interfaces:**
- Consumes: `Forma`, `desenhar_forma_tingida`, `casa_visivel`.
- Produces: `Formas.anel: Forma` (receita com preenchimento transparente, borda 3 px branca, sombra/brilho de 8 px lilás `0xc4b2ff` com alfa ~0x55); `fn anel_da_casa(area: Rect, indice: usize) -> Rect` (a casa visível aumentada de 0,04·lado para o brilho caber).

- [ ] **Step 1: Teste vermelho:** `o_anel_da_casa_e_maior_que_a_casa_e_cabe_no_tabuleiro`: para os índices 0, 7, 56 e 63 em 360×640 e 1080×2400, `anel_da_casa` contém a casa visível e `x`/`y` do anel não passam da moldura (`tela.tabuleiro`) mais 2 px.
- [ ] **Step 2: Rodar** (falha).
- [ ] **Step 3: Implementar** `anel` e `anel_da_casa`; a casa selecionada passa a usar o anel branco; o alvo do arrasto usa o anel com tinta `0xc4ffe4` a 80%; a Dica usa o anel com `DESTAQUE` pulsando (mesmo `pulso_da_dica`). Os `draw_rectangle_lines` saem.
- [ ] **Step 4: Verificar:** foto `selecionada`, `dica` e `troca` em 360×640 e 1080×2400: anel arredondado sem cortar nas bordas do tabuleiro (peças dos cantos). Mutação: `anel_da_casa` devolver a própria casa e ver o teste falhar.
- [ ] **Step 5: Commit** `feat: anel luminoso na peça selecionada, no alvo e na dica`.

### Task 5: Fim de partida, diálogo e folhas em vidro

**Files:**
- Modify: `game/src/render.rs` (`desenhar_fim`, `desenhar_dialogo`, `desenhar_folha_de_som`, `desenhar_folha_do_menu`, `desenhar_cartao`, `botao_da_tela`, `desenhar_faixa_de_nivel`, `desenhar_aviso`), `Formas`
- Test: `game/src/render.rs`

**Interfaces:**
- Consumes: `receita_de_vidro`, `Estilo::Titulo`, constantes de texto.
- Produces: `Formas.painel: Forma` (vidro maior, raio 20, para fim/diálogo/folhas), `Formas.botao_principal: Forma` (degradê `0x5b6cff`→`0x8b5cf6`, raio 14, borda clara); `fn pelicula(area: Rect, raio: f32, alfa: f32)` (escurece com cantos arredondados usando `solida` tingida de preto).

- [ ] **Step 1: Teste vermelho:** `a_pelicula_do_fim_segue_o_raio_da_moldura`: função pura `fn raio_da_moldura(tabuleiro: Rect) -> f32` (o mesmo valor que `desenhar_tabuleiro_vazio` usa) devolve > 0 e é o raio passado à película do fim (conferido pela constante compartilhada).
- [ ] **Step 2: Rodar** (falha).
- [ ] **Step 3: Implementar:** fim e diálogo em `painel` com título `Estilo::Titulo`, texto de apoio em `TEXTO_SECUNDARIO`, números "Nível | pontos" em `DESTAQUE`; "Jogar novamente"/"Novo jogo" com `botao_principal`, "Cancelar" com `botao`; película do fim com `pelicula` no raio da moldura (fecha o item adiado da V1); folhas Som/Menu com fundo `painel` em tela cheia da folha (mantendo a opacidade alta atual) e as linhas (`desenhar_cartao`) em vidro; interruptor, trilho e botão de escolha com as cores novas; faixa de nível e aviso com o mesmo vidro e título em Sora.
- [ ] **Step 4: Verificar:** fotos de `fim`, `dialogo`, `som`, `recordes`, `creditos`, `nivel` em 360×640 e 1080×2400 (legíveis, nada cortado). Mutação: raio 0 na película e ver o teste falhar.
- [ ] **Step 5: Commit** `feat: fim de partida, diálogo e folhas em vidro`.

### Task 6: Verificação geral e notas

**Files:**
- Modify: `docs/notes/visual.md`, `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md` (linha da fase 3)

- [ ] **Step 1:** Fotos das 14 cenas em 360×640 e das 6 mais densas em 1080×2400, mais `inicio` em 320×480 e 800×600; conferir sem corte, sobreposição ou quadradinho, e comparar o conjunto com as fotos da V2 (nada ficou pior).
- [ ] **Step 2:** `cargo test --workspace`, `cargo test --manifest-path tools/Cargo.toml`, clippy do `game` (9 linhas de base) e `ls -la game/assets` (fontes novas somam o previsto).
- [ ] **Step 3:** Atualizar `docs/notes/visual.md` com a seção "V3": fontes e por que três estilos, receita de vidro, constantes de contraste, anel, e o que só o celular confirma (fontes no Android, tempo de abertura, o brilho do vidro agradar); atualizar a linha da fase 3 na spec.
- [ ] **Step 4: Commit** `docs: notas da V3`.
