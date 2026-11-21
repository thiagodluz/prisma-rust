# Prisma Fase 2 · M1 (Jogar) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O app abre direto no jogo: o Thiago joga uma partida completa de cada modo (Zen e Clássico) no celular, com toque e arrasto, animações na ordem dos eventos do motor, painel (pontos, nível, melhor, progresso), Dica, Embaralhar (só Zen), Novo jogo, subida de nível e fim de partida.

**Architecture:** Toda a lógica nova é pura (sem janela) e testada com `cargo test`: `input` (toque/arrasto → pedido de troca), `linha` (eventos do motor → linha do tempo de animação, com durações e curvas), `layout` (tamanho da tela → retângulos) e `app` (a máquina de estados: uma partida por modo, linha do tempo em andamento, painel, botões, diálogo, dica). Só `render` e o laço do `main` tocam a janela; o visual é conferido por "fotos" (`--screenshot`) de cenas fixas e o Android pelo celular virtual do CI e pelo celular do Thiago. A tela só mostra o que o motor decidiu: nenhuma regra de jogo é recalculada no `game/`.

**Tech Stack:** Rust 2021, Macroquad 0.4.16, `prisma_engine` (motor, não muda). Nenhuma dependência nova.

**Spec:** `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md` (seções 5.2, 5.3, 5.3b e marco M1 da seção 6). Referência de comportamento: `app.js` do Prisma em JS 1.4.2, em `/home/claude/thiagodluz/prisma` (somente leitura): gestos nas linhas ~649–735, animações nas ~378–546, resultado da jogada nas ~585–647, painel (`hud`) nas ~293–328.

## Global Constraints

- `edition = "2021"` em todos os `Cargo.toml`; nenhuma dependência nova; o motor `prisma_engine` não muda (os testes dele continuam passando). Hoje `cargo test --workspace` soma 115.
- **Anti-objetivo da spec:** regra de jogo duplicada na tela. O `game/` usa `Game::play`, `hint`, `shuffle(false)`, `restart`, `new_game` e `prisma_engine::level_goal`; não recalcula combinações, pontos nem metas por conta própria.
- Código específico do Android só compila no CI. Neste M1 não há código novo específico do Android: tudo precisa compilar e passar no PC.
- Neste ambiente os testes e a "foto" precisam de `libasound`: `mkdir -p /tmp/asound && ln -sf /usr/lib/x86_64-linux-gnu/libasound.so.2 /tmp/asound/libasound.so` e `export RUSTFLAGS="-L /tmp/asound"`. Foto: `LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a -s "-screen 0 1280x1024x24" cargo run -q -p prisma_game -- ...` (a mensagem do ALSA sobre "card 0" é normal).
- Coordenadas do Macroquad (`mouse_position`, `screen_width`) são em pixels lógicos (`high_dpi: true`); para o registro do Android (Task 7) converta para pixels reais com `screen_dpi_scale()`, como o M0b fez.
- Textos exatos do JS (acentos funcionam na fonte padrão; **não** usar símbolos como ✧ ↻ ⤨): `PONTOS`, `NÍVEL`, `MELHOR`, `Zen`, `Clássico`, `{ganho} / {meta} para o próximo nível`, `Embaralhar`, `Dica`, `Novo jogo`, `Fim de partida`, `Não há mais jogadas possíveis nesta partida.`, `Jogar novamente`, `Começar novo jogo?`, `O progresso da partida atual será substituído.`, `Cancelar`, `Tabuleiro reorganizado`, `Supernova`, `Nível {n}`. Números no formato pt-BR (`1.800`, `12.450`).
- Durações do JS (efeitos em escala 1): troca válida 225 ms, inválida 300 ms; limpeza 270 ms (340 ms se ativou especial, com atraso por distância `min(100, 22 × distância)`); queda por pedra `min(470, 250 + 32 × linhas)` ms; embaralhar 190 ms saindo + 250 ms entrando; resgate 300 ms; faixa de nível 580 ms; dica visível 2600 ms. As curvas de suavização ficam a critério de quem constrói (spec, seção 9).
- Textos da tela e comentários de código em português simples (Thiago tem conhecimento básico de programação): explicar cada conceito novo de Rust numa linha, como nos módulos existentes.
- Commits em português (`feat:`, `test:`, `ci:`, `docs:`) com os trailers de atribuição de sempre.

## Review Focus

1. **Toque e arrasto ilegais:** arrastar a pedra da última coluna para a direita (índice 7 → 8 seria a primeira casa da linha de baixo), da primeira coluna para a esquerda, para cima na primeira linha ou para baixo na última nunca troca com a casa errada nem com nada; diagonal vale o eixo maior; movimento curto demais não faz nada; segundo dedo e toque fora do tabuleiro são ignorados. Testes na Task 1.
2. **Entrada durante animação, diálogo ou fim de partida:** nenhuma jogada dupla; trocar de modo, Novo jogo, Dica e Embaralhar são ignorados enquanto há animação; com o diálogo aberto só os botões dele respondem; com a partida acabada o tabuleiro não responde. Testes nas Tasks 4 e 5.
3. **Jogada longa (reação em cadeia, Supernova, embaralhar automático, subir vários níveis):** a linha do tempo termina; os pontos e o nível mostrados no fim são exatamente os do motor; pedra nova vinda de cima (origem de queda negativa) usa divisão que arredonda para baixo (`div_euclid`), senão a queda sai com a distância errada. Testes nas Tasks 2 e 4.
4. **Tempo e tela estranhos:** `dt` de 0, negativo, NaN, infinito ou enorme (app voltando do segundo plano) pula/termina a animação sem pânico e sem prender o jogo; tela de tamanho 0, minúscula, muito larga ou muito estreita não dá pânico e mantém tudo dentro da tela quando ela existe. Testes nas Tasks 2 e 3.
5. **Fim de partida e recuperação:** no Clássico sem jogadas aparece o cartão de fim, a entrada trava e "Jogar novamente" abre o diálogo (cancelar mantém o cartão); Embaralhar não existe no Clássico; Dica sem jogada ou com a partida acabada não faz nada; no Zen sem jogadas o resgate (Espectro) aparece na animação. Testes na Task 5.
6. *(só no aparelho)* **Toque real e desempenho:** arrasto e toque simulados no emulador (Task 7) e no celular do Thiago (Task 8) precisam funcionar; fluidez em celular fraco só dá para ver no celular.

## Escolhas minhas (a confirmar na revisão do plano)

- Uma partida na memória **por modo**: trocar de Zen para Clássico não perde a partida do outro modo (como no JS). O M2 passa a gravar em arquivo.
- "MELHOR" no M1 é a melhor pontuação do modo enquanto o app está aberto; o M2 passa a guardar de verdade (e os demais recordes).
- Layout empilhado e centralizado na vertical (abas de modo, painel, barra, tabuleiro, botões), que serve para qualquer proporção de tela.
- Botões agem ao **apertar** (como no M0b); o tabuleiro age ao **soltar**, como no JS.
- Limiares de gesto em frações da casa (0,25 para "foi um toque", 0,28 para "foi um arrasto"), porque os 12/24 pixels do JS valem para casas de ~46 px e os celulares variam muito.
- Limpeza anima encolhendo e apagando (sem faíscas, raios nem linhas de fusão da Supernova: partículas e efeitos ficam para o M4); resgate e faixa de nível entram; embaralhar faz fade.
- Ficam para os marcos seguintes: salvar partida e recordes (M2), todo o som (M3), vibração, partículas e intensidade do Zen (M4), estilo "Anterior", menu de desenvolvedor.
- A tela de teste do M0b continua no programa, mas só abre com `--tela diagnostico` (no celular não há argumentos, então abre o jogo). O teste do celular virtual deixa de conferir contador, som e vibração (já provados no aparelho) e passa a **jogar**.

## Mapa de arquivos

- Create `game/src/input.rs` (T1), `game/src/linha.rs` (T2), `game/src/app.rs` (T4, T5), `game/src/cenas.rs` (T6).
- Modify `game/src/layout.rs` (T3), `game/src/render.rs` (T6), `game/src/main.rs` (T1–T7: `mod`s, `Tela`, laço do jogo), `tools/teste-emulador.sh` e `docs/notes/android-build.md` (T7).

---

### Task 1: Toque e arrasto (`input`, puro)

**Files:**
- Create: `game/src/input.rs`
- Modify: `game/src/main.rs` (só `mod input;`)

**Interfaces:**
- Consumes: `layout::Rect` (`x, y, w, h`, `contem`), `prisma_engine::{adjacent, SIZE}`.
- Produces (usado pelas Tasks 4 e 5):
```rust
pub enum Evento { Baixo { x: f32, y: f32 }, Mover { x: f32, y: f32 }, Cima { x: f32, y: f32 }, Cancelar }
pub enum Acao { Nada, Trocar(usize, usize) }
pub fn casa_em(tabuleiro: Rect, x: f32, y: f32) -> Option<usize>; // 0..=63, ou None se fora/tabuleiro vazio
#[derive(Default)] pub struct Gestos { /* privado */ }
impl Gestos {
    pub fn selecionada(&self) -> Option<usize>;      // casa marcada por um toque
    pub fn pressionada(&self) -> Option<usize>;      // casa sob o dedo agora
    pub fn alvo_do_arrasto(&self) -> Option<usize>;  // vizinha que o arrasto atual escolheria
    pub fn limpar(&mut self);                        // zera seleção e gesto
    pub fn tratar(&mut self, evento: Evento, tabuleiro: Rect, bloqueado: bool) -> Acao;
}
```

Regras (portadas de `app.js` ~649–735; `lado` = largura da casa = `tabuleiro.w / 8`):
- `Baixo`: ignorado se `bloqueado`, se já há gesto (segundo dedo) ou se cai fora de uma casa; senão guarda (casa, x, y) e `pressionada`.
- `Mover`: só com gesto ativo; recalcula `alvo_do_arrasto`.
- Alvo do arrasto: se `max(|dx|, |dy|) < 0,28 × lado` → `None`; senão um passo no eixo dominante (`|dx| > |dy|` → horizontal; empate → vertical, como no JS), e só vale se `adjacent(início, fim)` (isso barra a volta de linha e sair do tabuleiro).
- `Cima`: termina o gesto. Se `bloqueado` → `Nada`. Com alvo → `selecionada = None` e `Trocar(início, alvo)`. Sem alvo e `max(|dx|, |dy|) < 0,25 × lado` → é um **toque** na casa de início: sem seleção → seleciona; mesma casa → desmarca; vizinha da selecionada → `selecionada = None` e `Trocar(selecionada, casa)`; outra casa → passa a selecionada. Movimento entre 0,25 e 0,28 da casa → `Nada`.
- `Cancelar`: solta o gesto, mantém a seleção.

- [ ] **Step 1: Escrever os testes que falham** (em `input.rs`, módulo `tests`, com um tabuleiro de teste `Rect { x: 100.0, y: 200.0, w: 400.0, h: 400.0 }`, casa de 50 px, e um ajudante `centro(casa) -> (f32, f32)`). Nomes e o que cada um afirma:
  - `toque_seleciona_depois_troca_com_a_vizinha`: toque (Baixo+Cima no mesmo ponto) na casa 27 → `selecionada() == Some(27)`, `Nada`; toque na 28 → `Trocar(27, 28)` e `selecionada() == None`.
  - `toque_na_mesma_casa_desmarca` e `toque_em_casa_longe_passa_a_selecao`.
  - `arrasto_para_a_direita_troca_com_a_casa_ao_lado`: Baixo no centro da 26, Mover/Cima 30 px à direita → `alvo_do_arrasto() == Some(27)` e `Trocar(26, 27)`; idem esquerda, cima e baixo.
  - `arrasto_nas_bordas_nao_da_a_volta`: casa 7 para a direita, casa 8 para a esquerda, casa 3 para cima, casa 59 para baixo → sempre `Nada` e `alvo_do_arrasto() == None`.
  - `arrasto_diagonal_vale_o_eixo_maior_e_empate_vale_vertical`.
  - `limiares_de_toque_e_arrasto`: deslocamento de 0,1 casa (5 px) → toque; 0,26 casa (13 px) → `Nada`; 0,3 casa (15 px) → arrasto.
  - `toque_fora_do_tabuleiro_e_ignorado` e `casa_em_nas_bordas` (`x == tabuleiro.x` → coluna 0; `x == tabuleiro.x + tabuleiro.w` → `None`; `y` idem).
  - `bloqueado_ignora_tudo`: `Baixo` bloqueado não cria gesto; `Baixo` livre e `Cima` bloqueado → `Nada` e gesto solto.
  - `segundo_dedo_e_ignorado`, `cancelar_solta_o_gesto_e_guarda_a_selecao`, `limpar_zera_selecao_e_gesto`.
  - `tabuleiro_vazio_ou_invalido_nao_da_pane`: `Rect` de tamanho 0 e com NaN → `casa_em` devolve `None`, `tratar` devolve `Nada`.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game input` → Expected: erro de compilação (`cannot find ... Gestos`/`casa_em`), que conta como RED.
- [ ] **Step 3: Implementar** `Evento`, `Acao`, `casa_em`, `Gestos` em `game/src/input.rs` com as regras acima (constantes `LIMITE_TOQUE = 0.25` e `LIMITE_ARRASTO = 0.28`, em frações da casa). Usar `prisma_engine::adjacent` para a vizinhança; não escrever a conta de linha/coluna à mão.
- [ ] **Step 4: Rodar e ver passar:** `cargo test -p prisma_game input` → Expected: todos passam. **Checagem por mutação:** trocar `0.28` por `0.0` e ver `limiares_de_toque_e_arrasto` falhar; trocar `adjacent(...)` por `true` e ver `arrasto_nas_bordas_nao_da_a_volta` falhar; desfazer.
- [ ] **Step 5: Commit** `feat: toque e arrasto do tabuleiro (input)`.

---

### Task 2: Linha do tempo das animações (`linha`, puro)

**Files:**
- Create: `game/src/linha.rs`
- Modify: `game/src/main.rs` (só `mod linha;`)

**Interfaces:**
- Consumes: `prisma_engine::{Board, Event, MoveResult, Activation, SIZE}`.
- Produces (usado pelas Tasks 4, 5 e 6):
```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Passo {
    Troca { antes: Board, a: usize, b: usize, valida: bool },        // `antes` = tabuleiro antes da troca
    Limpeza { tabuleiro: Board, celulas: Vec<usize>, ativadas: Vec<usize>, supernova: bool, ganho: u64 }, // tabuleiro ANTES de limpar
    Queda { tabuleiro: Board, origens: Vec<i32> },                    // tabuleiro DEPOIS; origens[destino] = casa de origem (negativa = nasceu acima)
    Embaralhar { antes: Board, depois: Board },
    Resgate { tabuleiro: Board, indice: usize },
    Nivel { nivel: u32 },
}
pub fn montar(antes: Board, a: usize, b: usize, resultado: &MoveResult, nivel_final: u32) -> Vec<Passo>;
pub fn duracao_ms(passo: &Passo) -> f32;
// Amostragem (tudo em milissegundos desde o começo do passo; resultado sempre finito):
pub fn deslocamento_troca(t_ms: f32, valida: bool) -> f32;          // 0..=1: fração do caminho até a casa vizinha (inválida: vai e volta, pico em 48%)
pub fn linhas_de_queda(origem: i32, linha_destino: usize) -> i32;   // quantas linhas a pedra desce; origem negativa usa div_euclid
pub fn queda_ms(linhas: i32) -> f32;                                // min(470, 250 + 32 × linhas)
pub fn queda_deslocamento(origem: i32, linha_destino: usize, t_ms: f32) -> f32; // em casas, acima do destino (>= 0), 0 no fim
pub fn limpeza_da_casa(ativadas: &[usize], casa: usize, t_ms: f32) -> (f32, f32); // (escala, alfa): 1→1,1→0,72 e 1→1→0, com o atraso do JS
pub fn embaralhar_alfa(t_ms: f32) -> (bool, f32);                   // (mostra_o_novo, alfa): 1→0,12 em 190 ms; depois 0,12→1 em 250 ms
pub fn resgate_da_casa(t_ms: f32) -> (f32, f32);                    // (escala, alfa): 0,25→1,16 (em 65%)→1 ; 0→1
pub fn nivel_faixa(t_ms: f32) -> (f32, f32);                        // (escala, alfa) da faixa "Nível N"
#[derive(Clone, Debug)] pub struct Reprodutor { /* privado */ }
impl Reprodutor {
    pub fn novo(passos: Vec<Passo>) -> Reprodutor;
    pub fn avancar(&mut self, dt_ms: f32);            // dt NaN/negativo conta como 0; infinito/enorme pula tudo; nunca dá pânico
    pub fn atual(&self) -> Option<(&Passo, f32)>;     // passo em andamento e ms já decorridos nele
    pub fn concluidos(&self) -> &[Passo];             // passos já terminados, em ordem
    pub fn terminou(&self) -> bool;
    pub fn pular_para_o_fim(&mut self);
}
```
Mapeamento de eventos do motor: `Clear` → `Limpeza` (`ativadas` = `index` de cada `Activation`; `supernova` = alguma é `Supernova`; `ganho` = `earned`; `tabuleiro` = o `board` do evento); `Fall` → `Queda`; `Shuffle` → `Embaralhar`; `Rescue` → `Resgate`; `GameOver` **não** vira passo (o cartão de fim aparece depois da animação, como no JS). Jogada inválida → só `Troca { valida: false }`. Jogada válida → `Troca { valida: true }`, os passos dos eventos e, no fim, `Nivel { nivel: nivel_final }` se `levels_gained > 0`.

- [ ] **Step 1: Escrever os testes que falham.** Ajudante de teste: `jogada_valida(seed) -> (Game, (usize, usize))` usando `Game::with_random(lcg_random(seed))` e `hint()`; e `jogada_invalida(seed)` que acha por força bruta um par vizinho para o qual `play` devolve `valid == false`. Testes:
  - `troca_invalida_vira_um_passo_so` (e `duracao_ms == 300.0`); `troca_valida_dura_225`.
  - `jogada_valida_segue_a_ordem_dos_eventos`: os nomes dos passos (clear→Limpeza, fall→Queda…) batem, na ordem, com `resultado.events` e começam com `Troca { valida: true }`; `nivel_so_aparece_se_subiu` (forçar: `game.score = 1790` antes da jogada → último passo `Nivel`; sem forçar → nenhum `Nivel`); `fim_de_jogo_nao_vira_passo`.
  - `duracoes_do_js`: `duracao_ms` de Limpeza sem ativadas 270 e com ativadas ≥ 340; `queda_ms(1) == 282.0`, `queda_ms(7) == 470.0` (teto), `queda_ms(0) == 250.0`; Queda dura o máximo entre as pedras; Embaralhar 440; Resgate 300; Nivel 580.
  - `queda_de_pedra_nova_usa_divisao_para_baixo`: `linhas_de_queda(-5, 0) == 1` (origem −5 é a linha −1 da coluna 3: desce 1), `linhas_de_queda(-1, 0) == 1`, `linhas_de_queda(-8, 0) == 1`, `linhas_de_queda(-9, 0) == 2`, `linhas_de_queda(27, 3) == 0` (casa 27 já está na linha 3) e `linhas_de_queda(3, 3) == 3` (da linha 0 à linha 3); `queda_deslocamento` é `linhas` em t=0, 0 em t ≥ `queda_ms`, nunca cresce e nunca é negativo.
  - `amostras_nos_extremos`: `deslocamento_troca(0, _) == 0`, `deslocamento_troca(225, true) == 1`, inválida: 0 no começo e no fim e 1 perto de 48% de 300 ms; `limpeza_da_casa` dá `(1.0, 1.0)` em t=0 e `(0.72, 0.0)` no fim; casa a 5 casas de uma ativada espera `min(100, 22 × 5) = 100` ms antes de começar; `embaralhar_alfa(0).1 == 1.0`, mínimo 0,12 em 190, 1.0 em 440; `resgate_da_casa(0) == (0.25, 0.0)` e `(1.0, 1.0)` no fim; todas devolvem valores finitos para `t_ms` NaN, negativo e infinito.
  - `reprodutor_passa_passo_a_passo_e_conta_concluidos`: avançar a duração do 1º passo → `concluidos().len() == 1`; `avancar(f32::NAN)`, `avancar(-5.0)` não mudam nada; `avancar(f32::INFINITY)` e `avancar(1e30)` terminam; `pular_para_o_fim` termina; `Reprodutor::novo(vec![])` já `terminou()`.
  - `jogadas_longas_terminam`: 300 jogadas válidas seguidas (`hint()`, seeds 1..=20, 15 jogadas cada, nos dois modos) → soma das durações finita e < 60 000 ms; `Reprodutor` avançado em passos de 16 ms termina em tempo finito.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game linha` → Expected: erro de compilação.
- [ ] **Step 3: Implementar** o módulo (constantes de duração nomeadas, uma por efeito, com comentário citando o JS; `limpeza_da_casa` usa as 3 paradas do JS: escala 1 → 1,1 em 38% → 0,72 e alfa 1 → 1 → 0, com `ease-out`; `ativadas` vazio ⇒ sem atraso). Funções de amostragem **totais**: `t_ms` inválido vira 0 e o resultado é limitado ao intervalo do efeito. Comentar o `div_euclid`: "`-5 / 8` dá 0 em Rust (arredonda para zero), mas queremos −1".
- [ ] **Step 4: Rodar e ver passar:** `cargo test -p prisma_game linha` → Expected: todos passam. **Mutação:** trocar `div_euclid` por `/` e ver `queda_de_pedra_nova_usa_divisao_para_baixo` falhar; trocar 470 por 900 e ver `duracoes_do_js` falhar; desfazer.
- [ ] **Step 5: Commit** `feat: linha do tempo das animações (linha)`.

---

### Task 3: Layout da tela de jogo (`layout`, puro)

**Files:**
- Modify: `game/src/layout.rs` (acrescentar; `board_rect`, `cell_rect` e `tela_diagnostico` continuam como estão)

**Interfaces:**
- Consumes: `layout::Rect`.
- Produces (Tasks 4–6):
```rust
pub struct TelaJogo {
    pub aba_zen: Rect, pub aba_classico: Rect,
    pub pontos: Rect, pub nivel: Rect, pub melhor: Rect,      // três colunas do painel
    pub barra: Rect, pub rotulo: Rect,                        // barra de progresso e a linha de texto embaixo dela
    pub tabuleiro: Rect,                                      // quadrado
    pub botao_embaralhar: Option<Rect>, pub botao_dica: Rect, pub botao_novo_jogo: Rect, // Embaralhar só existe se pedido
    pub cartao_fim: Rect, pub botao_jogar_novamente: Rect,    // sobre o tabuleiro
    pub dialogo: Rect, pub dialogo_cancelar: Rect, pub dialogo_confirmar: Rect, // centrado na tela
    pub texto: f32,                                           // tamanho base de fonte (0,045 do menor lado)
}
pub fn tela_jogo(largura: f32, altura: f32, com_embaralhar: bool) -> TelaJogo;
```
Ordem de cima para baixo: abas, painel (três colunas), barra, rótulo, tabuleiro, botões; o conjunto é **centrado na vertical**; o tabuleiro é o maior quadrado que cabe (lado ≤ 94% da largura e ≤ altura livre depois de reservar abas, painel e botões). Com `com_embaralhar == false` os dois botões restantes ficam centrados e do mesmo tamanho que os três. Tela inválida (zero, negativa, NaN, infinita) → todos os retângulos vazios (como `tela_diagnostico`).

- [ ] **Step 1: Escrever os testes que falham** (módulo `tests` de `layout.rs`): para cada tela em `[(540,960), (1080,2340), (1080,2400), (360,640), (800,600), (1000,1000), (200,2000), (2000,200), (50,50)]` e para `com_embaralhar` verdadeiro e falso:
  - `tudo_cabe_na_tela`: todo retângulo não vazio fica dentro de `[0,largura]×[0,altura]`.
  - `elementos_nao_se_sobrepoem`: abas, as três colunas do painel, barra, rótulo, tabuleiro e botões não se intersectam dois a dois (as bordas encostadas valem como fora, como em `Rect::contem`); cartão de fim dentro do tabuleiro; botão "Jogar novamente" dentro do cartão; botões do diálogo dentro do diálogo e sem se sobrepor.
  - `ordem_vertical`: `abas.y < pontos.y < barra.y < rotulo.y < tabuleiro.y < botao_dica.y`.
  - `tabuleiro_e_quadrado` e `tabuleiro_grande_em_retrato`: em 540×960 o lado é ≥ 0,9 da largura.
  - `embaralhar_some_e_os_outros_continuam_centrados`.
  - `tela_invalida_da_tudo_vazio`: `(0,0)`, `(-1,10)`, `(NaN,10)`, `(f32::INFINITY,10)`.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game layout` → Expected: erro de compilação (`TelaJogo`).
- [ ] **Step 3: Implementar** `TelaJogo` e `tela_jogo` (unidade `u` = menor lado da tela; alturas em frações de `u`: abas 0,085, painel 0,11, barra 0,022, rótulo 0,05, botões 0,10, espaços 0,02; se a altura total passar da tela, reduzir só o tabuleiro).
- [ ] **Step 4: Rodar e ver passar:** `cargo test -p prisma_game layout` → Expected: todos passam (os 12 de antes também). **Mutação:** tirar o limite "≤ altura livre" do tabuleiro e ver `tudo_cabe_na_tela` falhar em (800,600); desfazer.
- [ ] **Step 5: Commit** `feat: layout da tela de jogo`.

---

### Task 4: Estado do jogo — jogar uma jogada (`app`, parte 1)

**Files:**
- Create: `game/src/app.rs`
- Modify: `game/src/main.rs` (só `mod app;`)

**Interfaces:**
- Consumes: Task 1 (`Evento`, `Acao`, `Gestos`), Task 2 (`Passo`, `Reprodutor`, `montar`), Task 3 (`TelaJogo`, `tela_jogo`), `prisma_engine::{Game, Mode, lcg_random, level_goal, system_random}`.
- Produces (Tasks 5–7):
```rust
pub struct Estagio { pub pontos: u64, pub nivel: u32, pub inicio_do_nivel: u64 } // o que o painel mostra AGORA
pub struct App { /* privado */ }
impl App {
    pub fn novo() -> App;                                   // sorte do sistema; começa no Zen
    pub fn com_sementes(zen: u32, classico: u32) -> App;    // para testes e cenas (lcg_random)
    pub fn modo(&self) -> Mode;
    pub fn jogo(&self) -> &Game;                            // a partida do modo atual
    pub fn gestos(&self) -> &Gestos;
    pub fn ocupado(&self) -> bool;                          // há linha do tempo em andamento
    pub fn tocar(&mut self, evento: Evento, tela: &TelaJogo); // tabuleiro (botões entram na Task 5)
    pub fn avancar(&mut self, dt_ms: f32);
    pub fn passo_atual(&self) -> Option<(&Passo, f32)>;
    pub fn estagio(&self) -> Estagio;
    pub fn registro(&mut self) -> Vec<String>;              // drena as linhas "PRISMA_M1 ..." acumuladas
    pub(crate) fn jogo_mut(&mut self) -> &mut Game;         // SÓ para testes e cenas: mexer direto no motor (score, ended, force_no_moves...)
}
```
Comportamento: guarda **uma `Game` por modo** (Zen e Clássico). Uma `Acao::Trocar(a, b)` guarda `antes = jogo.board`, guarda o `Estagio` de antes (a "base"), chama `Game::play(a, b)`, monta os passos com `linha::montar(antes, a, b, &resultado, jogo.level)` e cria o `Reprodutor`; a entrada fica bloqueada (`bloqueado = ocupado() || jogo.ended`) até ele terminar. `estagio()` sem linha do tempo = números do motor; com linha do tempo = a base mais o `ganho` dos `Limpeza` já `concluidos()`, subindo de nível com o laço do JS (`while pontos - inicio >= level_goal(nivel) { inicio += meta; nivel += 1 }`, usando `prisma_engine::level_goal`). Ao terminar o `Reprodutor`: `reprodutor = None`, `gestos.limpar()`. Linha de registro por jogada: `PRISMA_M1 jogada valida=<true|false> pontos=<n> nivel=<n>` (valores do motor após a jogada); ao começar: `PRISMA_M1 pronto modo=zen`; quando a linha do tempo de uma jogada (ou de um embaralhar) termina: `PRISMA_M1 animacao_terminou` (o teste do celular virtual usa essa linha para saber que já pode jogar de novo).

- [ ] **Step 1: Escrever os testes que falham** (com `App::com_sementes(7, 9)`, uma tela `tela_jogo(540.0, 960.0, true)` e ajudantes `arrastar(app, a, b)` e `tocar_casa(app, i)` que montam eventos a partir de `tela.tabuleiro`, e `boa_jogada(app) -> (usize, usize)` que chama `app.jogo_mut().hint().unwrap()`; o motor só entra em campos públicos como `score` e `ended` por `jogo_mut()`):
  - `arrasto_valido_inicia_linha_e_trava_a_entrada`: depois do arrasto, `ocupado()`; `jogo().score` já é o final; novo `Baixo` não cria gesto (`gestos().pressionada() == None`).
  - `jogada_invalida_so_balanca`: arrastar um par que não combina → `ocupado()` por 300 ms, depois `jogo().board` igual ao de antes e `score` igual; registro `valida=false`.
  - `toque_toque_tambem_joga`: tocar `a` e depois `b` da dica → mesma jogada.
  - `no_fim_o_estagio_e_igual_ao_motor`: para as seeds 1..=15, 12 jogadas por seed em cada modo (sempre pela `hint`, parando se ela devolver `None`), avançando `avancar(16.0)` até `!ocupado()`: `estagio() == Estagio { pontos: jogo.score, nivel: jogo.level, inicio_do_nivel: jogo.level_start_score }`; **e** durante a animação `estagio().pontos` nunca diminui e fica entre a base e o final do motor.
  - `subir_de_nivel_aparece_no_estagio_no_passo_certo`: com `jogo_mut().score = 1790` (só no teste) uma jogada boa → `estagio().nivel == 1` no começo e `2` ao fim; o último passo é `Nivel`.
  - `avancar_com_tempo_estranho_nao_trava`: `avancar(f32::NAN)`, `-3.0`, `0.0` mantêm `ocupado()`; `avancar(1e30)` termina e destrava.
  - `toque_com_animacao_nao_faz_nada`: durante a linha, tocar duas casas não muda `selecionada()` nem o motor.
  - `partida_acabada_trava_o_tabuleiro`: `jogo_mut().ended = true` → arrasto não joga.
  - `registro_tem_o_formato_esperado`: depois de uma jogada boa e de `avancar` até o fim, `registro()` devolve `["PRISMA_M1 pronto modo=zen", "PRISMA_M1 jogada valida=true pontos=<n> nivel=<n>", "PRISMA_M1 animacao_terminou"]` com os números do motor, e esvazia a fila.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game app` → Expected: erro de compilação (`App`).
- [ ] **Step 3: Implementar** `Estagio`, `App` com os métodos acima em `game/src/app.rs`. `tocar` com `Evento::Baixo/Mover/Cima` chama `gestos.tratar(evento, tela.tabuleiro, bloqueado)`; o botão de Dica e demais ficam para a Task 5.
- [ ] **Step 4: Rodar e ver passar:** `cargo test -p prisma_game app` → Expected: todos passam. **Mutação:** fazer `estagio()` ignorar o laço de nível e ver `subir_de_nivel_aparece_no_estagio_no_passo_certo` falhar; remover o `bloqueado` de `tocar` e ver `toque_com_animacao_nao_faz_nada` falhar; desfazer.
- [ ] **Step 5: Commit** `feat: estado do jogo e fluxo de uma jogada (app)`.

---

### Task 5: Botões, painel, dica, diálogo e fim de partida (`app`, parte 2)

**Files:**
- Modify: `game/src/app.rs`

**Interfaces:**
- Consumes: Task 4 (`App`, `Estagio`), Task 3 (`TelaJogo`).
- Produces (Tasks 6–7):
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Botao { Zen, Classico, Embaralhar, Dica, NovoJogo, JogarNovamente, Cancelar, Confirmar }
pub fn formatar_numero(n: u64) -> String;                    // 1234567 -> "1.234.567"
pub struct Painel {
    pub pontos: String, pub nivel: String, pub melhor: String,
    pub progresso: f32,                 // 0..=1 (limitado)
    pub rotulo: String,                 // "{ganho} / {meta} para o próximo nível"
    pub ganho: Option<String>,          // "+250" por ~1 s depois de cada limpeza
    pub aviso: Option<String>,          // "Tabuleiro reorganizado" / "Supernova", ~1,6 s
    pub fim: Option<(String, String)>,  // (nível, pontos) quando o cartão de fim aparece
    pub dialogo: bool,
    pub embaralhar_visivel: bool,       // só no Zen
    pub dica_ativa: bool,               // falso com a partida acabada
}
impl App {
    pub fn tocar(&mut self, evento: Evento, tela: &TelaJogo);   // agora também trata Baixo em botões
    pub fn botao_em(&self, tela: &TelaJogo, x: f32, y: f32) -> Option<Botao>; // só botões visíveis agora
    pub fn apertar(&mut self, botao: Botao);
    pub fn painel(&self) -> Painel;
    pub fn dica(&self) -> Option<(usize, usize)>;               // par destacado agora
}
```
Regras (JS ~765–800 e `hud()` ~293–328):
- **Botões respondem ao `Baixo`** que cai dentro deles; com o diálogo aberto só `Cancelar` e `Confirmar` existem; com a partida acabada (cartão) existem `JogarNovamente`, `NovoJogo`, `Zen`, `Classico`.
- `Dica`: `Game::hint()`; `Some((a, b))` → destaca 2600 ms e registra `PRISMA_M1 dica a=<a> b=<b>`; `None` → nada. Qualquer toque no tabuleiro, jogada, troca de modo ou Novo jogo apaga a dica.
- `Embaralhar` (só Zen, só sem animação): `Game::shuffle(false)`; se `true` → linha do tempo com um `Passo::Embaralhar { antes, depois }` e, ao terminar, aviso "Tabuleiro reorganizado".
- `Zen`/`Classico`: troca de modo só sem animação e se for outro modo; apaga gestos, dica, ganho e aviso; cada modo mantém a sua partida; registra `PRISMA_M1 modo=<zen|classico> pontos=<n>`.
- `NovoJogo` e `JogarNovamente` (sem animação) abrem o diálogo; `Cancelar` fecha; `Confirmar` → `Game::restart()` no modo atual, limpa tudo e registra `PRISMA_M1 novo_jogo modo=<m> pontos=0`.
- "MELHOR" = maior `score` já visto no modo (atualizado quando a linha do tempo termina e ao trocar de modo/novo jogo); `ganho` = `earned` do último `Limpeza` concluído; aviso "Supernova" quando um `Limpeza` com `supernova` começa; ao terminar uma jogada com `ended`, registra `PRISMA_M1 fim pontos=<n> nivel=<n>`.
- `painel()` com linha do tempo usa `estagio()`; `rotulo` e `progresso` usam `level_goal(nivel)` e `inicio_do_nivel` (`progresso = min(1, (pontos - inicio) / meta)`).

- [ ] **Step 1: Escrever os testes que falham:**
  - `formatar_numero_usa_pontos_de_milhar`: `0→"0"`, `999→"999"`, `1800→"1.800"`, `12450→"12.450"`, `1234567→"1.234.567"`, `u64::MAX` sem pânico e terminando em `".615"`.
  - `painel_inicial_do_zen`: `pontos "0"`, `nivel "1"`, `melhor "0"`, `rotulo "0 / 1.800 para o próximo nível"`, `progresso 0.0`, `embaralhar_visivel`, `dica_ativa`, sem `fim`, sem `dialogo`.
  - `progresso_e_limitado_e_acompanha_o_estagio`: após uma jogada, durante a animação o `rotulo` usa o `estagio`; no fim `progresso` está em `[0, 1]`.
  - `dica_destaca_por_2600ms_e_some`: `apertar(Dica)` → `dica() == Some(par da hint)`; `avancar(2599.0)` ainda; `avancar(2.0)` some; um toque no tabuleiro também apaga; registro `dica a=.. b=..`.
  - `dica_sem_jogada_ou_partida_acabada_nao_faz_nada`: `jogo_mut().ended = true` → `dica() == None`, `dica_ativa == false`.
  - `embaralhar_so_no_zen_e_so_sem_animacao`: no Zen vira `ocupado()` e depois o aviso aparece e some (~1,6 s); no Clássico `apertar(Embaralhar)` não muda nada e `embaralhar_visivel == false`; durante uma animação é ignorado; o tabuleiro depois tem as mesmas pedras (mesmos ids) em outra ordem.
  - `cada_modo_guarda_a_sua_partida`: jogar no Zen, ir ao Clássico (pontos 0), voltar ao Zen (pontos de antes); troca ignorada durante animação e para o mesmo modo.
  - `novo_jogo_pede_confirmacao`: `apertar(NovoJogo)` → `painel().dialogo`; com o diálogo aberto o tabuleiro e `Dica` não respondem; `Cancelar` fecha e nada muda; `Confirmar` zera `score` e `level`, fecha, registra `novo_jogo`.
  - `fim_de_partida_no_classico`: forçar `jogo_mut().ended = true`, `score = 4321` e `level = 3` (só no teste) → `painel().fim == Some(("3", "4.321"))`, o arrasto não joga, `JogarNovamente` abre o diálogo e `Cancelar` mantém o cartão; `Confirmar` começa outra partida (sem `fim`).
  - `classico_sem_jogadas_termina_pelo_motor`: no Clássico, ligar `jogo_mut().force_no_moves = true` (campo do motor feito para testes; **não** chamar `restart` depois, no mesmo teste) e fazer uma jogada boa → ao fim da animação `jogo().ended`, `painel().fim.is_some()`, o arrasto não joga e o registro tem `PRISMA_M1 fim pontos=<n> nivel=<n>`.
  - `zen_sem_jogadas_mostra_o_resgate`: mesmo truque no Zen → entre os passos há um `Passo::Resgate`, `ended` continua falso e a entrada volta ao terminar.
  - `botao_em_acha_so_o_botao_visivel`: `Embaralhar` não é achado no Clássico; no diálogo só `Cancelar`/`Confirmar`; toque na borda exata de um botão acha um só.
  - `ganho_e_supernova`: `painel().ganho == Some("+<earned do primeiro Limpeza>")` logo depois do primeiro `Limpeza` concluído e some em ~1 s; um `Limpeza` com `supernova: true` mostra o aviso "Supernova" (no teste, dentro do módulo, pôr direto no `app` um `Reprodutor` com `Passo::Limpeza { supernova: true, ganho: 250, .. }` e avançar).
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game app` → Expected: erro de compilação (`Botao`, `Painel`, …).
- [ ] **Step 3: Implementar** (`formatar_numero` puro e separado; `Painel` montado a cada chamada, sem guardar texto).
- [ ] **Step 4: Rodar e ver passar:** `cargo test --workspace` → Expected: tudo passa. **Mutação:** deixar `Embaralhar` aparecer no Clássico e ver `embaralhar_so_no_zen_e_so_sem_animacao` falhar; deixar o diálogo aberto aceitar `Dica` e ver `novo_jogo_pede_confirmacao` falhar; desfazer.
- [ ] **Step 5: Commit** `feat: botões, painel, dica, diálogo e fim de partida (app)`.

---

### Task 6: Desenho da tela de jogo e cenas para foto (`render`, `cenas`)

**Files:**
- Modify: `game/src/render.rs`
- Create: `game/src/cenas.rs`
- Modify: `game/src/main.rs` (`mod cenas;`; ainda sem o laço do jogo)

**Interfaces:**
- Consumes: `App` (`modo`, `jogo`, `gestos`, `passo_atual`, `estagio`, `painel`, `dica`), `Painel`, `TelaJogo`, funções de amostragem de `linha`, `Sprites`, `assets::SPRITE_RATIO`.
- Produces:
```rust
// render.rs
pub fn draw_jogo(app: &App, sprites: &Sprites, tela: &TelaJogo);
// cenas.rs
pub fn cena(nome: &str) -> Option<App>;   // "inicio", "selecionada", "dica", "troca", "limpando", "queda", "nivel", "fim", "dialogo", "classico"
```
`cena` monta um `App::com_sementes(7, 9)` fixo e o leva a um instante conhecido (usando `tocar`/`apertar`/`avancar` e, só aqui, os campos públicos do motor para `fim`): `inicio` = tabuleiro parado; `selecionada` = uma casa tocada; `dica` = Dica apertada; `troca` = a meio caminho da troca; `limpando` = a 50% do primeiro `Limpeza`; `queda` = a 40% da primeira `Queda`; `nivel` = com `score = 1790` e a jogada boa, no meio da faixa "Nível 2"; `fim` = Clássico com `ended = true`, `score = 4321`, `level = 3`; `dialogo` = Novo jogo apertado; `classico` = aba do Clássico.

Desenho (cores a critério de quem constrói, partindo do M0a/M0b: `FUNDO 0x101229`, `PAINEL 0x0d142c`, `BOTAO 0x2a3f8f`): fundo; abas com a atual realçada; três colunas `PONTOS`/`NÍVEL`/`MELHOR` (rótulo pequeno em cima, valor grande embaixo, `estagio` durante a animação); barra de progresso e rótulo, com o `ganho` ("+250") à esquerda do rótulo; painel do tabuleiro; pedras (sem linha do tempo: `jogo().board` parado; com linha do tempo: pelo passo atual — `Troca` desenha `antes` com as duas pedras deslocadas por `deslocamento_troca`; `Limpeza` desenha o tabuleiro do passo com escala/alfa de `limpeza_da_casa` nas `celulas`; `Queda` desenha as pedras deslocadas para cima por `queda_deslocamento` (pedra nova com alfa 0,45 → 1); `Embaralhar` desenha `antes` ou `depois` com `embaralhar_alfa`; `Resgate` desenha o tabuleiro com a pedra do `indice` por `resgate_da_casa`; `Nivel` desenha o tabuleiro final e a faixa); realces: casa `selecionada` (fundo claro translúcido + borda), casa `pressionada`, `alvo_do_arrasto` (contorno), par da `dica` (pulsando com `get_time()`); botões visíveis (Embaralhar só no Zen); aviso (`toast`) por cima do tabuleiro; faixa "Nível N" no centro do tabuleiro; cartão de fim e diálogo (com um véu escuro atrás). Pedras que descem de cima podem passar por cima do painel (como no JS, não há corte).

- [ ] **Step 1: Escrever os testes que falham** (só em `cenas.rs`, sem janela): `cada_cena_existe_e_nome_estranho_nao`: as 10 cenas devolvem `Some`, `cena("xyz") == None`; `cenas_tem_o_estado_esperado`: `inicio` não ocupado e sem seleção; `selecionada` com `gestos().selecionada().is_some()`; `dica` com `dica().is_some()`; `troca`/`limpando`/`queda`/`nivel` ocupadas com `passo_atual()` do tipo certo (`Troca`, `Limpeza`, `Queda`, `Nivel`); `fim` com `painel().fim == Some(("3", "4.321"))`; `dialogo` com `painel().dialogo`; `classico` com `modo() == Mode::Classic`.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game cenas` → Expected: erro de compilação.
- [ ] **Step 3: Implementar** `cenas.rs` e `render::draw_jogo`. Ajudantes pequenos em `render.rs`: texto centralizado numa área (`measure_text`, como em `desenhar_botao`), barra, botão. Não duplicar regras: tudo vem de `Painel`, `Passo` e das funções de `linha`.
- [ ] **Step 4: Rodar os testes:** `cargo test -p prisma_game cenas` → Expected: passam. Depois, **sem o laço do jogo ainda**, ligar provisoriamente um `rodar_cena(nome, foto)` no `main.rs` (será o mesmo código que a Task 7 aproveita): `--tela jogo --cena <nome> --screenshot /tmp/m1-<nome>.png`.
- [ ] **Step 5: Foto das 10 cenas e conferência a olho** (ler cada PNG com a ferramenta de imagem): texto legível e sem cortes; acentos corretos; nada sobreposto; pedras no tabuleiro; realces visíveis; `limpando` mostra pedras encolhendo; `queda` mostra pedras no ar; `fim` e `dialogo` com véu e botões dentro do cartão. Corrigir o que estiver feio e repetir. Anotar no ledger o que foi ajustado.
- [ ] **Step 6: Commit** `feat: desenho da tela de jogo e cenas para foto`.

---

### Task 7: Laço do jogo, registro e teste do celular virtual jogando

**Files:**
- Modify: `game/src/main.rs`, `tools/teste-emulador.sh`, `docs/notes/android-build.md`

**Interfaces:**
- Consumes: Tasks 1–6.
- Produces: o programa abre no jogo; `--tela diagnostico` abre a tela do M0b; `--tela vitrine` o M0a; `--tela jogo --cena <nome>` a cena.

Mudanças em `main.rs`: `enum Tela { Jogo, Diagnostico, Vitrine }`, o padrão (sem argumento ou argumento desconhecido) passa a ser `Jogo` (atualizar o teste `tela_pedida_le_o_argumento_tela`: `--tela diagnostico` → `Diagnostico`, `--tela vitrine` → `Vitrine`, nada/`--tela xyz` → `Jogo`; e acrescentar `cena_pedida(&args) -> Option<String>` com teste para `--cena limpando`). `rodar_jogo(foto, cena)`: cria `App::novo()` (ou a cena); a cada quadro: `tela = tela_jogo(screen_width(), screen_height(), app.modo() == Mode::Zen)`; eventos do ponteiro (`is_mouse_button_pressed` → `Baixo`, botão segurado e posição mudou → `Mover`, `is_mouse_button_released` → `Cima`; o toque no celular já vira mouse, como no M0b) → `app.tocar(evento, &tela)`; `app.avancar(get_frame_time() * 1000.0)` (**sem limitar o `dt`**: voltar do segundo plano termina a animação, como a spec manda); `render::draw_jogo`; `foto_do_quadro`; `next_frame().await`. Depois de cada quadro, `app.registro()` vai para `info!` (as linhas `PRISMA_M1 ...`). A cada mudança de tamanho da tela **ou de modo** (o layout muda: sem o Embaralhar, Dica e Novo jogo se deslocam), registrar em **pixels reais** (lógico × `screen_dpi_scale()`): `PRISMA_M1 tabuleiro x=<x> y=<y> lado=<l>` e `PRISMA_M1 botao=<zen|classico|embaralhar|dica|novo_jogo|jogar_novamente|cancelar|confirmar> x=<cx> y=<cy>` (centros dos retângulos do layout do modo atual; `embaralhar` só aparece no Zen; `jogar_novamente`, `cancelar` e `confirmar` são sempre registrados, mesmo escondidos).

`tools/teste-emulador.sh` (substitui as conferências do M0b): abre o app; espera 15 s e confere que continua aberto e que o registro tem `PRISMA_M1 pronto modo=zen`; lê a última linha de `tabuleiro` e de cada `botao`; **Zen:** 5 vezes — toca em `dica`, lê a última linha `PRISMA_M1 dica a=<a> b=<b>`, faz `adb shell input swipe` do centro da casa `a` ao centro da casa `b` (200 ms), espera (até 30 s, de meio em meio segundo) uma nova linha `PRISMA_M1 animacao_terminou` e confere que surgiu uma linha `jogada valida=true` com `pontos` maior que o da jogada anterior e que o app continua aberto; **Clássico:** toca em `classico`, espera a linha `modo=classico pontos=0`, repete 5 vezes com **dois toques** (casa `a`, depois casa `b`); toca em `novo_jogo` e `confirmar` e confere `novo_jogo modo=classico pontos=0`; volta ao Zen e confere `modo=zen` com os pontos de antes (a partida do Zen foi guardada); tira uma foto da tela em cada fase e imprime reduzida em base64 (como hoje). O ajudante `centro_da_casa <i>` calcula `x = tx + (col + 0,5) × lado/8`, `y = ty + (linha + 0,5) × lado/8`.

- [ ] **Step 1: Escrever os testes que falham** no `main.rs` (`tela_pedida` e `cena_pedida`, descritos acima) e rodar: `cargo test -p prisma_game -- tela_pedida cena_pedida` → Expected: FAIL (o padrão ainda é `Diagnostico`; `cena_pedida` não existe).
- [ ] **Step 2: Implementar** `Tela::Jogo`, `cena_pedida`, `rodar_jogo` e o registro de geometria. Rodar `cargo test --workspace` → Expected: tudo passa.
- [ ] **Step 3: Conferir no PC:** foto sem cena (`--screenshot /tmp/m1-jogo.png`) mostra o tabuleiro inicial do Zen; `--tela diagnostico` e `--tela vitrine` continuam funcionando (foto de cada); conferir também `--cena fim`. Ler as imagens.
- [ ] **Step 4: Escrever o `tools/teste-emulador.sh` novo** e rodar `bash -n tools/teste-emulador.sh`. Atualizar `docs/notes/android-build.md` (a seção do celular virtual agora descreve que o teste **joga**, o que o `PRISMA_M1` registra e que o teste de vida do M0b foi aposentado do CI).
- [ ] **Step 5: Commit** `feat: jogo jogável e teste do celular virtual jogando`.
- [ ] **Step 6 (com autorização do Thiago): enviar a branch `fase2-m1` e disparar o fluxo Android nela** (`git push -u origin fase2-m1`; `gh workflow run android.yml --ref fase2-m1`; acompanhar com `gh api` e ler o log com `gh run view --log`). *Pronto quando:* APK + os dois celulares virtuais (Android 11 x86 e Android 14 x86_64) verdes, com as linhas `jogada valida=true` no log, e as fotos do log (lidas a olho) mostrando o jogo. Se o arrasto simulado (`input swipe`) não for reconhecido mas o toque-toque for, registrar uma decisão no ledger (o arrasto passa a ser conferido só no celular do Thiago) e ajustar o script; se o app fechar ou travar, ler o log, corrigir e reenviar.

---

### Task 8: Conferência no celular do Thiago e fechamento do marco

**Files:**
- Modify: `docs/notes/android-build.md` (só se algo novo for aprendido)

- [ ] **Step 1: Integrar na `main` e esperar a release** (com autorização do Thiago): `git checkout main`, `git merge --no-ff fase2-m1`, `cargo test --workspace`, `git push origin main`; esperar o fluxo Android da `main` ficar verde e a release `apk-mais-recente` ser republicada.
- [ ] **Step 2: Thiago instala e joga** (desinstalando o app antigo antes, porque a chave de debug muda a cada geração). Lista de conferência:
  1. O app abre direto no jogo (Zen), com tabuleiro, painel e botões.
  2. Arrastar uma pedra para a vizinha troca; tocar numa pedra e depois na vizinha também troca; uma troca que não combina balança e volta.
  3. As combinações somem, as pedras caem, os pontos e a barra sobem; uma reação em cadeia anima em ordem.
  4. **Dica** destaca duas pedras por uns segundos; **Embaralhar** reorganiza (só no Zen); **Novo jogo** pede confirmação.
  5. Ao passar da meta aparece "Nível 2" e o tabuleiro é reorganizado.
  6. Trocar para **Clássico** começa uma partida zerada; voltar ao Zen mantém a partida de antes.
  7. Jogar o Clássico até o **Fim de partida** (aparece o cartão; "Jogar novamente" abre a confirmação).
  8. Sem travadas fortes nem pedras no lugar errado depois de uma animação.
  *Pronto quando:* uma partida completa de cada modo roda sem erro visual evidente. Qualquer falha: anotar no ledger (com print), corrigir antes do M2.
- [ ] **Step 3: Fechar o marco:** ledger, apagar a pasta de trabalho deste plano, relatar "Rulings I made" e "Deferred minors". M2 (salvar) começa depois, com o plano dele.
