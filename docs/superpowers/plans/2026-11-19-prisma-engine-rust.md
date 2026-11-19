# Prisma Engine em Rust — Plano de Implementação (Fase 1)

> **Para quem executa:** use superpowers:executing-plans (recomendado, porque o Thiago aprova cada tarefa) ou superpowers:subagent-driven-development. Os passos usam caixas `- [ ]`.

**Objetivo:** uma biblioteca Rust, `prisma_engine`, que se comporta como o `engine.js` do Prisma, provada pelos 36 testes do JS portados para Rust.

**Arquitetura:** biblioteca sem dependências e sem tela, dividida por responsabilidade: tipos, regras puras do tabuleiro, estado do jogo, especiais, resolução em cascata, recuperação/desfazer, saves e cheats. Testes de integração em `tests/`, um arquivo por tarefa. A última tarefa compara a saída do Rust com a do `engine.js` original, com a mesma semente.

**Stack:** Rust 1.97 (só `std`). Node 22 apenas para gerar o arquivo de comparação (Tarefa 8).

**Desenho:** `docs/superpowers/specs/2026-11-19-prisma-engine-rust-design.md`

**Referência de comportamento (somente leitura):** `/home/claude/thiagodluz/prisma`, commit `94ae2437452dd906f337ab8e559f5e4e2a5b4f08`: `engine.js`, `tests/engine.test.js`, `tests/developer-cheats.test.js`. Em caso de dúvida, vale o que o JS faz.

## Mudança em relação ao desenho

O desenho tinha 6 etapas. Ao detalhar, apareceram dependências: `play` chama embaralhar, resgate e desfazer, e `hint` usa `creations` e `supernovas`. Por isso a ordem ficou assim (mesmo conteúdo, reordenado, e a etapa 2 dividida em duas):

| Desenho | Plano |
|---|---|
| Etapa 1 (núcleo) | Tarefa 1 (sem `hint`) |
| Etapa 3 (parte) | Tarefa 2 (embaralhar e resgate) |
| Etapa 2 | Tarefa 3 (especiais e dica) e Tarefa 4 (jogada em cascata) |
| Etapa 3 (resto) | Tarefa 5 (fim da jogada e desfazer) |
| Etapa 4 | Tarefa 6 |
| Etapa 5 | Tarefa 7 |
| Etapa 6 | Tarefa 8 |

## Restrições Globais

- Rust 1.97, **zero dependências** (só `std`). Pacote `prisma_engine`, pasta `/home/claude/prisma-rust`, fora do repositório Prisma.
- Comportamento igual ao `engine.js`. Bug encontrado no JS: **anotar** em `docs/notes/js-quirks.md` (criar na primeira ocorrência), não corrigir.
- Nomes de código em inglês (como no original); comentários em **português**, simples, explicando cada conceito de Rust na primeira vez que ele aparece.
- **Cada tarefa termina esperando o "pode seguir" do Thiago.** Ele aprova cada ação.
- Teste Rust = título do teste JS em `snake_case`, portado 1:1 do arquivo JS indicado, mantendo todos os valores numéricos.
- Commits em git local; a mensagem termina com as linhas de autoria exigidas pela sessão (`Co-Authored-By` e `Claude-Session`).
- `cargo test` sem nenhum aviso do compilador ao fim de cada tarefa.
- Valores fixos do `engine.js`: tabuleiro 8×8, 7 cores, meta do nível = `min(6000, 1800 + 150×(nível−1))`; pontos de criação Pulso 120 / Raio 180 / Espectro 240; 25 pontos por pedra limpa; multiplicador `1 + min(cadeia−1, 5)×0,4`; pontos arredondados ao múltiplo de 5; Supernova 5×5; desfazer guarda no máximo 50; 1000 tentativas para tabuleiro inicial; 128 tentativas de embaralhar.

## Foco da Revisão

Entradas que o desenho implica mas nenhum teste do JS exercita, da mais provável para a menos:

1. `restore` com tabuleiro de formato errado (linhas ou colunas a mais ou a menos): devolve `false`, o jogo atual fica intacto. → teste na Tarefa 6.
2. `restore` com números gigantes (nível, pontos): devolve `false`, sem estouro de conta nem pânico. → Tarefa 6.
3. `play` com índice fora do tabuleiro (≥ 64) ou `a == b`: inválido, sem pânico, nada muda. → Tarefa 4.
4. Jogada inválida não consome números da fonte de sorte, não altera `next_id` nem o histórico de desfazer. → Tarefas 4 e 5.
5. Fonte de sorte que devolve exatamente `1.0`: nenhum índice fora dos limites. → Tarefas 1 e 2.

## Estrutura de arquivos

```
prisma-rust/
  Cargo.toml, .gitignore
  src/lib.rs          módulos e re-exports públicos
  src/tile.rs         SIZE, COLORS, TileKind, Tile, Mode, Board, position
  src/rng.rs          RandomSource, system_random, lcg_random, pick
  src/matching.rs     Run, Matches, find_matches, adjacent, swap_on (regras puras)
  src/game.rs         Game, Snapshot, construção, consultas do tabuleiro
  src/specials.rs     creations, supernovas
  src/hint.rs         hint
  src/event.rs        Creation, Supernova, Created, Activation, Event, MoveResult
  src/progression.rs  level_goal, goal_for, level_start_at
  src/resolve.rs      play, resolve_board, nearby_color
  src/recovery.rs     shuffle, rescue, snapshot e desfazer
  src/saves.rs        SavedMode, SavedTile, SavedGame, restore, to_saved
  src/cheats.rs       cheat_*
  tests/common/mod.rs fixture, at, place, match_move
  tests/core.rs · recovery.rs · specials.rs · play.rs · endgame.rs · saves.rs · cheats.rs · cross_check.rs
  tests/golden/       generate.mjs e arquivos de referência gerados pelo JS
  docs/notes/js-quirks.md
```

---

### Tarefa 1: Projeto e núcleo do tabuleiro

**Conceitos de Rust para explicar nos comentários:** `struct`, `enum`, `Option`, `Copy`, arrays de tamanho fixo, `Box<dyn FnMut>`.

**Files:** Criar tudo de `Cargo.toml` até `src/game.rs` exceto `specials`, `hint`, `event`, `progression`, `resolve`, `recovery`, `saves`, `cheats`; `tests/common/mod.rs`, `tests/core.rs`.

**Interfaces — Produces:**
- `tile.rs`: `pub const SIZE: usize = 8; pub const COLORS: u8 = 7;` `pub enum TileKind { Normal, Burst, Cross, Spectrum }` (Burst = Pulso, Cross = Raio, Spectrum = Espectro); `pub struct Tile { pub id: u32, pub color: Option<u8>, pub kind: TileKind }` (Espectro tem `color: None`); `pub enum Mode { Zen, Classic }` (`Default` = Zen); `pub type Board = [[Tile; SIZE]; SIZE]`; `pub fn position(index: usize) -> (usize, usize)`. Tudo `Clone, Copy, Debug, PartialEq, Eq` (Mode também `Default`). Índice = linha×8 + coluna.
- `rng.rs`: `pub type RandomSource = Box<dyn FnMut() -> f64>`; `pub fn system_random() -> RandomSource` (xorshift64 semeado por `SystemTime`); `pub fn lcg_random(seed: u32) -> RandomSource` (`seed = seed.wrapping_mul(1664525).wrapping_add(1013904223)`, devolve `seed as f64 / 4294967296.0`); `pub fn pick(random: &mut dyn FnMut() -> f64, len: usize) -> usize` (`floor(r × len)`, **limitado a `len − 1`**).
- `matching.rs`: `pub struct Run { pub cells: Vec<usize>, pub color: u8, pub length: usize }`; `pub struct Matches { pub cells: Vec<usize>, pub runs: Vec<Run> }`; `pub fn find_matches(board: &Board) -> Matches` (varre linhas e depois colunas; sequências de 3 ou mais da mesma cor, Espectro não combina; `cells` = união sem repetição, na ordem em que aparecem); `pub fn adjacent(a: usize, b: usize) -> bool` (vizinhos na horizontal/vertical, ambos < 64); `pub fn swap_on(board: &mut Board, a: usize, b: usize)`.
- `game.rs`: `pub struct Snapshot { pub next_id: u32, pub board: Board, pub score: u64, pub level: u32, pub level_start_score: u64, pub progression_offset: u64, pub mode: Mode, pub ended: bool, pub no_defeat: bool }` (`Clone, Debug, PartialEq`; os métodos de desfazer só entram na Tarefa 5). `pub struct Game { pub random: RandomSource, pub next_id: u32, pub board: Board, pub score: u64, pub level: u32, pub level_start_score: u64, pub progression_offset: u64, pub mode: Mode, pub ended: bool, pub undo_history: Vec<Snapshot>, pub cheats_used: bool, pub no_defeat: bool, pub force_no_moves: bool }`. Métodos: `Game::new()` (usa `system_random`), `Game::with_random(random: RandomSource)`, `new_game(&mut self, mode: Mode)`, `restart(&mut self)` (= `new_game(self.mode)`), `gem(&mut self, color: Option<u8>, kind: TileKind) -> Tile` (Espectro força `None`; incrementa `next_id`), `random_gem(&mut self) -> Tile` (cor sorteada, Normal), `tile(&self, index: usize) -> Option<Tile>` (`None` se ≥ 64), `set(&mut self, index: usize, tile: Tile)`, `matches(&self) -> Matches`, `swap(&mut self, a: usize, b: usize)`, `has_move(&self) -> bool`, `has_move_on(&self, board: &Board) -> bool`.
- **Gancho de teste `force_no_moves`:** quando `true`, `has_move` **e** `has_move_on` devolvem `false` para qualquer tabuleiro, inclusive o candidato do embaralhar. Dois testes do JS substituem `hasMove` assim, e o teste "Zen recovers a move" só passa se o embaralhar de subida de nível também falhar.
- `new_game` zera pontos, nível, histórico, `cheats_used`, `no_defeat`, `ended`; monta o tabuleiro com `fresh_board` (`pub(crate)`): até 1000 tentativas; cada célula sorteia (via `pick`) entre as cores que não formam trinca com as duas células à esquerda/acima; aceita se `has_move_on`; esgotadas as tentativas, `panic!("Não foi possível criar um tabuleiro jogável")` (equivale ao `throw` do JS).
- `has_move_on`: para cada par vizinho (a, a+1) e (a, a+8), com Espectro em qualquer lado = jogada; senão troca, vê se há combinação, desfaz.

**Consumes:** nada (primeira tarefa).

- [ ] **Passo 1: Criar o projeto.** `cd /home/claude/prisma-rust && cargo init --lib --name prisma_engine --vcs none && git init -q && git config user.name Claude && git config user.email noreply@anthropic.com && printf '/target\n' > .gitignore && cargo build`. Esperado: `Finished`. Commit inicial incluindo `docs/`.
- [ ] **Passo 2: Escrever os testes** em `tests/core.rs`, com os auxiliares em `tests/common/mod.rs` (`fixture() -> Game`: `lcg_random(7654321)` e `board[r][c] = gem(Some((r*3 + c*2) % 7), Normal)`; `at(r, c) -> usize`; `place(&mut Game, r, c, Option<u8>, TileKind)`; `match_move(&Game) -> (usize, usize)`, portado do JS). Testes:
  - `lcg_random_matches_the_js_sequence`:
    ```rust
    let mut r = lcg_random(7654321);
    assert_eq!(r(), 0.6867495393380523);
    assert_eq!(r(), 0.013034644303843379);
    assert_eq!(r(), 0.727377827744931);
    ```
  - `new_game_with_lcg_seed_matches_the_js_board`: `Game::with_random(lcg_random(7654321))`; as cores das linhas, em ordem, são `[4,0,5,5,0,4,1,3] [3,5,3,6,0,3,4,6] [1,2,5,6,1,0,3,1] [5,1,1,2,3,6,1,6] [6,6,5,0,6,5,0,0] [0,5,5,4,6,5,1,5] [4,3,0,1,5,0,1,1] [3,5,6,5,3,3,4,6]` e `next_id == 65` (valores gerados pelo `engine.js` original).
  - `starting_boards_have_unique_stable_ids_no_matches_and_a_move` (de `engine.test.js`, 100 jogos com `Game::new()`).
  - `pick_never_returns_an_index_outside_the_range`: `pick(&mut || 1.0, 5) == 4`, `pick(&mut || 0.0, 5) == 0`, `pick(&mut || 0.999999, 5) == 4`.
  - `matches_finds_runs_of_three_or_more_and_ignores_spectrum`: trinca horizontal, quadra vertical, forma em L (células sem repetição em `cells`), Espectro no meio de uma linha não combina.
  - `adjacent_accepts_only_orthogonal_neighbours_inside_the_board`: `adjacent(0,1)` e `adjacent(0,8)` verdadeiros; `adjacent(7,8)`, `adjacent(0,9)`, `adjacent(5,5)`, `adjacent(63,64)` falsos.
  - `fixture_board_has_no_moves_but_force_no_moves_hides_real_ones`: `fixture().has_move() == false`; num `Game::new()`, `force_no_moves = true` faz `has_move()` ser `false`.
- [ ] **Passo 3: Rodar e ver falhar.** `cargo test --test core`. Esperado: erro de compilação (`unresolved import prisma_engine::...`).
- [ ] **Passo 4: Implementar** os arquivos e assinaturas de "Produces". `lib.rs` declara os módulos e reexporta os tipos públicos.
- [ ] **Passo 5: Rodar.** `cargo test`. Esperado: 7 testes passando, sem avisos.
- [ ] **Passo 6: Commit** (`feat: núcleo do tabuleiro`).
- [ ] **Passo 7: Parar para aprovação.** Mostrar ao Thiago, em poucas frases, o que ficou pronto e o resultado do `cargo test`. Lembrar do repositório permanente (desenho, "Em aberto"): só ele pode criar um repositório novo no GitHub.

---

### Tarefa 2: Embaralhar e resgate

**Conceitos:** `&mut self`, laços, `Vec`, tentativas com limite.

**Files:** Criar `src/recovery.rs`; modificar `src/lib.rs`; criar `tests/recovery.rs`.

**Interfaces — Produces:** em `impl Game`: `pub fn shuffle(&mut self, automatic: bool) -> bool`; `pub fn rescue(&mut self) -> usize`.
**Consumes:** `Game`, `find_matches`, `has_move_on`, `pick`, `Board`, `Tile`.

Regras do `shuffle`: devolve `false` se `ended`, ou se Classic e `!automatic`. Até 128 tentativas de Fisher–Yates (`i` de 63 a 1, `j = pick(random, i+1)`) sobre as 64 pedras atuais; uma permutação é aceita se mudou algum id, não tem combinação pronta e `has_move_on` é verdadeiro. Se as 128 falharem, tentar trocas de pares `(a, b)` com `a < b` em ordem. Ao aceitar: trocar `board`; se `!automatic`, limpar `undo_history`. `rescue`: escolher o primeiro índice de `[27, 28, 35, 36, 0..64]` cuja pedra seja Normal (se nenhum, 27); colocar nele `gem(None, Spectrum)`; devolver o índice.

- [ ] **Passo 1: Testes** em `tests/recovery.rs`:
  - `manual_zen_shuffle_keeps_every_tile_special_type_and_color_without_free_matches` (`engine.test.js`: Burst em (0,0), Cross em (1,1), Espectro em (2,2), score 175; 100 embaralhadas; no fim, em Classic, `shuffle(false) == false`).
  - `low_entropy_random_source_still_finds_a_safe_permutation_without_new_tiles` (`game.random = Box::new(|| 0.0)`).
  - `shuffle_survives_a_random_source_that_returns_one` (`|| 1.0`: sem pânico; os ids continuam os mesmos).
  - `rescue_places_a_spectrum_at_the_first_normal_tile_of_the_centre`: em `fixture()`, `rescue() == 27`, `tile(27)` é Espectro sem cor, as outras 63 pedras mantêm o id, e `has_move()` passa a ser verdadeiro.
- [ ] **Passo 2: Rodar e ver falhar.** `cargo test --test recovery` (erro: `shuffle` não existe).
- [ ] **Passo 3: Implementar** `shuffle` e `rescue` em `src/recovery.rs`.
- [ ] **Passo 4: Rodar.** `cargo test`. Esperado: tudo passando.
- [ ] **Passo 5: Commit** (`feat: embaralhar e resgate`) e **parar para aprovação**.

---

### Tarefa 3: Especiais e dica

**Conceitos:** fatias (`&[usize]`), iteradores, `Vec` de structs.

**Files:** Criar `src/specials.rs`, `src/hint.rs`; modificar `src/event.rs` (criar com `Creation` e `Supernova`), `src/lib.rs`; criar `tests/specials.rs`.

**Interfaces — Produces:**
- `event.rs`: `pub struct Creation { pub index: usize, pub kind: TileKind, pub color: u8 }`; `pub struct Supernova { pub index: usize, pub color: u8, pub sources: Vec<usize> }`.
- `impl Game`: `pub fn creations(&self, matches: &Matches, preferred: &[usize], fused: &[usize]) -> Vec<Creation>`; `pub fn supernovas(&self, matches: &Matches) -> Vec<Supernova>`; `pub fn hint(&mut self) -> Option<(usize, usize)>` (`&mut` porque troca e desfaz no próprio tabuleiro, como o JS).

**Consumes:** `Game`, `Matches`, `Run`, `Tile`, `TileKind`, `adjacent`, `swap`.

Regras (de `engine.js`): `creations` junta sequências da mesma cor que se cruzam em um componente; tipo = Espectro se alguma sequência tem 5+, senão Raio se há mais de uma sequência e o componente tem menos células que a soma dos comprimentos, senão Pulso se alguma tem 4+; ignora componente que contém célula de `fused`; candidatos = células do componente com pedra Normal; índice = primeiro de `preferred` que seja candidato, senão o maior candidato. `supernovas`: para cada sequência, percorrer as células em ordem e agrupar pedras coloridas consecutivas Pulso/Raio ainda não consumidas; grupo de 3+ vira `Supernova { index: grupo[(len−1)/2], color: cor da sequência, sources: grupo }` e consome as pedras. `hint`: valor de cada troca (a, a+1) e (a, a+8) adjacente: 2 Espectros = 100; 1 Espectro = `8 + quantidade de pedras da outra cor`; senão, se a troca combina, `células + 20 × supernovas + soma(Espectro 10, Raio 7, Pulso 5)` das criações (com `preferred = [b, a]` e `fused` = fontes das supernovas); vence o maior valor (empate: o primeiro). `None` se `ended`.

- [ ] **Passo 1: Testes** em `tests/specials.rs`:
  - `creations_pick_burst_for_four_spectrum_for_five_and_cross_for_an_intersection`: em `fixture()`, quatro pedras de cor 0 na linha 3 → um `Creation` Burst com índice `at(3,3)` (maior candidato) e, com `preferred = [at(3,1)]`, índice `at(3,1)`; cinco → Spectrum; um T/L → Cross.
  - `long_and_crossing_runs_fuse_each_special_at_most_once` (`engine.test.js`: 4 Raios em linha → 1 supernova com `index == at(3,2)` e 4 fontes, `creations` vazio; cruz de 5 Pulsos → 3 fontes distintas).
  - `specials_separated_by_a_normal_gem_never_form_a_supernova` (parte direta do teste "two specials or specials separated…": `supernovas` vazio).
  - `hint_returns_a_legal_move_and_prioritizes_a_pair_of_spectra` (Espectros em (1,1) e (1,2) → `Some((at(1,1), at(1,2)))`; com `ended = true` → `None`).
  - `hint_on_fresh_boards_is_always_a_legal_move`: 20 sementes `lcg_random`, `hint()` é `Some((a,b))` com `adjacent(a,b)` e, após trocar, `matches()` não vazio ou há Espectro.
- [ ] **Passo 2: Rodar e ver falhar.** `cargo test --test specials`.
- [ ] **Passo 3: Implementar** `creations`, `supernovas` e `hint`.
- [ ] **Passo 4: Rodar.** `cargo test`. Esperado: tudo passando.
- [ ] **Passo 5: Commit** (`feat: especiais e dica`) e **parar para aprovação**.

---

### Tarefa 4: Jogada em cascata e pontuação

**Conceitos:** `match` em `enum`, cópia/`clone` do tabuleiro para os eventos, tipos inteiros e decimais.

**Files:** Criar `src/progression.rs`, `src/resolve.rs`; modificar `src/event.rs`, `src/lib.rs`; criar `tests/play.rs`.

**Interfaces — Produces:**
- `progression.rs`: `pub fn level_goal(level: u32) -> u64`; `pub(crate) fn goal_for(level: u64, cap: u64) -> u64`; `pub(crate) fn level_start_at(level: u64, cap: u64) -> Option<u64>` (**contas com `checked_*`**: estouro = `None`).
- `event.rs`: `pub struct Created { pub index: usize, pub kind: TileKind, pub color: u8, pub tile: Tile }`; `pub enum Activation { Special { index: usize, kind: TileKind, id: u32 }, Supernova(Supernova) }` com `pub fn name(&self) -> &'static str` (`"burst"`, `"cross"`, `"spectrum"`, `"supernova"`); `pub enum Event { Clear { board: Board, cells: Vec<usize>, activated: Vec<Activation>, creations: Vec<Created>, chain: u32, base: u64, creation_bonus: u64, combo_multiplier: f64, earned: u64 }, Fall { board: Board, falls: Vec<i32>, chain: u32 }, Shuffle { before: Board, board: Board, chain: u32 }, GameOver { board: Board, chain: u32 }, Rescue { board: Board, index: usize, chain: u32 } }` com `pub fn name(&self) -> &'static str` (`"clear"`, `"fall"`, `"shuffle"`, `"gameover"`, `"rescue"`); `pub struct MoveResult { pub valid: bool, pub events: Vec<Event>, pub earned: u64, pub chain: u32, pub rescued: bool, pub shuffled: bool, pub ended: bool, pub levels_gained: u32 }` com `MoveResult::invalid()`. `falls[destino]` = índice de origem; **negativo** = pedra nova vinda de cima do tabuleiro. `Clear.board` = tabuleiro antes da limpeza.
- `resolve.rs`: `pub fn play(&mut self, a: usize, b: usize) -> MoveResult` (é o `move` do JS; `move` é palavra reservada); `pub(crate) struct Direct { pub spectrum: Vec<usize>, pub target: Option<usize> }`; `pub(crate) fn resolve_board(&mut self, matches: Matches, direct: Option<Direct>, preferred: &[usize], initial_cells: &[usize]) -> MoveResult`; `fn nearby_color(&self, index: usize, cleared: &[usize]) -> Option<u8>`.

**Consumes:** `Game`, `Matches`, `creations`, `supernovas`, `find_matches`, `gem`, `random_gem`, `position`, `Board`.

Escopo desta tarefa: `play` e `resolve_board` completos até a contagem de nível. **Ainda sem** embaralhar na subida de nível, resgate, fim do Clássico e desfazer (Tarefa 5): `shuffled` e `rescued` ficam `false`, `ended` reflete o campo. `play` devolve `MoveResult::invalid()` se `ended` ou `!adjacent(a, b)`, ou se a troca não combina e nenhuma das duas pedras é Espectro (desfazendo a troca). Pontos de cada limpeza: `round((limpas × 25 + bônus de criação) × multiplicador / 5) × 5`. Ordem dos elementos: onde o JS usa `Set`, usar `Vec` com verificação de repetição (ordem de inserção importa em `cells`, `activated`). Índices vizinhos: contas com inteiro com sinal e verificação de limites. Cada limpeza gera `Clear` seguido de `Fall`; novas pedras caem com `random_gem`.

- [ ] **Passo 1: Testes** em `tests/play.rs` (portados de `engine.test.js`, com os valores):
  - `invalid_swaps_preserve_board_and_score` — e, estendido: `next_id` igual antes e depois, e a fonte de sorte não foi chamada (contador).
  - `play_never_panics_or_changes_anything_for_any_index_pair`: para todo `a` e `b` em `0..70`, `play(a, b)` não entra em pânico; se `!valid`, tabuleiro, pontos e `next_id` iguais ao início.
  - `four_five_and_l_t_shapes_create_one_special_at_the_swapped_destination`
  - `burst_and_cross_chain_reactions_clear_each_cell_once`
  - `three_colored_specials_fuse_into_one_centered_5x5_blast_in_either_direction` (`earned == 625`; centro `at(3,3)` na horizontal e `at(4,2)` na vertical)
  - `supernova_clips_at_the_edge_without_wrapping_to_the_opposite_side`
  - `supernova_activates_other_specials_once_including_a_spectrum_beyond_the_fusion` (`activated` = `["supernova","cross","spectrum"]`)
  - `a_later_cascade_can_align_three_specials_and_trigger_supernova`
  - `two_adjacent_specials_keep_their_own_effects` (parte de jogada do teste "two specials or specials separated…": `activated.len() == 2`, sem supernova)
  - `an_indirectly_triggered_spectrum_clears_a_surviving_color_after_surrounding_bursts` (`activated` = `["burst","burst","burst","spectrum"]`, `cells` contém `at(7,7)`)
  - `creating_a_special_never_replaces_an_existing_special_in_the_same_match`
  - `spectrum_swaps_clear_target_color_and_two_spectra_clear_all` (dois Espectros: 64 células)
  - `score_rewards_special_creation_and_cascades_with_a_capped_multiplier` (`base == 75`, `creation_bonus == 120`, `combo_multiplier == 1.0`, `earned == 195`; multiplicadores entre 1 e 3)
  - `level_goals_grow_gradually_to_6000_and_progress_can_cross_multiple_levels` (`level_goal` de `[1,2,3,4,5,10,19,20,21,22,23,28,29,100]` = `[1800,1950,2100,2250,2400,3150,4500,4650,4800,4950,5100,5850,6000,6000]`)
- [ ] **Passo 2: Rodar e ver falhar.** `cargo test --test play`.
- [ ] **Passo 3: Implementar** `progression.rs`, os tipos de `event.rs`, `play` e `resolve_board`. Seguir `engine.js` linhas 190–317 (até a contagem de nível), trazendo para Rust a mesma ordem de passos.
- [ ] **Passo 4: Rodar.** `cargo test`. Esperado: tudo passando.
- [ ] **Passo 5: Commit** (`feat: jogada em cascata e pontuação`) e **parar para aprovação**.

---

### Tarefa 5: Fim da jogada e desfazer

**Conceitos:** `Vec::push`/`remove`, histórico com limite, enum de resultado.

**Files:** Modificar `src/resolve.rs`, `src/recovery.rs`, `src/game.rs`; criar `tests/endgame.rs`.

**Interfaces — Produces:** em `impl Game`: `pub fn snapshot(&self) -> Snapshot`; `pub fn remember_state(&mut self, previous: Snapshot)` (histórico de no máximo 50, descartando o mais antigo); `pub fn can_undo_last_move(&self) -> bool`; `pub fn undo_last_move(&mut self) -> bool` (restaura o snapshot e marca `cheats_used = true`).
**Consumes:** `Snapshot`, `shuffle`, `rescue`, `has_move`, `resolve_board`, `play`, `Event`, `MoveResult`.

Mudanças: `play` guarda `snapshot()` antes da troca e chama `remember_state` só se a jogada for válida. Fim de `resolve_board`: se o nível subiu, `before = board`, `shuffle(true)`, e se funcionou, evento `Shuffle` e `shuffled = true`; depois, se `!has_move()`: Classic e `!no_defeat` → `ended = true` + evento `GameOver`; senão `rescue()` + evento `Rescue`, `rescued = true`. `new_game` já zera o histórico (Tarefa 1); conferir.

- [ ] **Passo 1: Testes** em `tests/endgame.rs`:
  - `both_modes_reshuffle_once_when_leveling_up_and_keep_surviving_specials` (semente 9921, os dois modos, `score = level_goal(1) - 75`; último evento é `shuffle`)
  - `classic_ends_without_moves_preserves_the_final_board_and_forbids_shuffling` (com `force_no_moves = true`; `ended`; último evento `gameover`; `shuffle(false) == false`; nova `play` inválida; `restart()` zera `ended`)
  - `zen_recovers_a_move_without_resetting_progress_or_other_specials` (`score = 1990`, `force_no_moves = true`; `rescued`; último evento `rescue`; todas as outras pedras mantêm o id)
  - `a_long_sequence_keeps_falling_ids_events_and_board_consistent` (100 jogadas com `match_move`, semente 7654321; cada `Fall` vem depois de um `Clear`, `falls.len() == 64`, nenhuma origem repetida nem limpa, ids coerentes, 64 ids únicos, sem combinações, com jogada)
  - `developer_undo_restores_the_complete_state_before_the_last_valid_move` (jogada inválida `play(0, 63)` não entra no histórico)
  - `invalid_play_leaves_the_undo_history_untouched`
  - `new_game_clears_developer_undo` (a parte de `restore` fica na Tarefa 6)
- [ ] **Passo 2: Rodar e ver falhar.** `cargo test --test endgame`.
- [ ] **Passo 3: Implementar** snapshot/desfazer e o final de `resolve_board` (`engine.js` linhas 292–339).
- [ ] **Passo 4: Rodar.** `cargo test`. Esperado: tudo passando.
- [ ] **Passo 5: Commit** (`feat: fim da jogada, resgate e desfazer`) e **parar para aprovação**.

---

### Tarefa 6: Restaurar save

**Conceitos:** `Result`/`Option`, validação, contas com `checked_*`, `std::mem::replace`.

**Files:** Criar `src/saves.rs`; modificar `src/lib.rs`; criar `tests/saves.rs`.

**Interfaces — Produces:** `pub enum SavedMode { Zen, Classic, Endless }` (`Default` = Zen); `pub enum SavedTile { Legacy(i64), Full { id: i64, kind: TileKind, color: Option<i64> } }`; `pub struct SavedGame { pub mode: SavedMode, pub score: i64, pub board: Vec<Vec<SavedTile>>, pub progression_version: Option<i64>, pub level: Option<i64>, pub level_start_score: Option<i64>, pub progression_offset: Option<i64>, pub cheats_used: bool, pub no_defeat: bool, pub ended: bool }` (`Clone, Debug, Default`); em `impl Game`: `pub fn restore(&mut self, saved: &SavedGame) -> bool`; `pub fn to_saved(&self) -> SavedGame` (versão 4, pedras completas, `cheats_used` e `no_defeat` do jogo).
**Consumes:** `Game`, `level_start_at`, `goal_for`, `shuffle`, `rescue`, `fresh_board`, `find_matches`, `has_move`.

Regras: as do `restore` de `engine.js` (linhas 489–549), na mesma ordem: formato 8×8, validade de cada pedra, ids sem repetição, versões 2/3/4 com tetos 4650/5000/6000, saves sem versão recalculam o nível por busca binária, `Endless` vira Classic, `Legacy(n)` vira pedra Normal de cor `n`. **Tudo ou nada:** monta um candidato; só se tudo for válido copia os campos para `self`. O candidato precisa da mesma fonte de sorte: pegue-a de `self` com `std::mem::replace` e **devolva-a antes de sair, inclusive no caminho de erro**. Toda conta em `i64` com `checked_*`; estouro = `false`. Termina limpando `undo_history`.

- [ ] **Passo 1: Testes** em `tests/saves.rs`:
  - `old_numeric_save_migrates_specials_persist_ids_remain_unique` (`engine.test.js`)
  - `restoring_an_unsettled_save_rearranges_its_specials_instead_of_discarding_them`
  - `a_no_moves_save_restores_as_finished_classic_or_playable_zen`
  - `v2_and_v3_progression_migrate_to_6000_without_losing_level_points_or_progress`
  - `corrupt_saves_leave_the_current_game_untouched` (id repetido; nível 999 com versão 2)
  - `restore_clears_developer_undo` (parte de restore do teste "new games and restores clear developer undo")
  - `restore_rejects_boards_with_the_wrong_shape`: 7 linhas; uma linha com 9 pedras; tabuleiro vazio → `false`, jogo intacto.
  - `restore_rejects_absurdly_large_numbers_without_overflow`: `score`, `level` e `level_start_score` iguais a `i64::MAX`, com `progression_version` `Some(2)`, `Some(3)`, `Some(4)` e `None`; `score` negativo → sempre `false`, sem pânico, jogo intacto.
- [ ] **Passo 2: Rodar e ver falhar.** `cargo test --test saves`.
- [ ] **Passo 3: Implementar** `SavedGame`, `restore` e `to_saved`.
- [ ] **Passo 4: Rodar.** `cargo test`. Esperado: tudo passando.
- [ ] **Passo 5: Commit** (`feat: restaurar e migrar saves`) e **parar para aprovação**.

---

### Tarefa 7: Cheats de desenvolvedor

**Conceitos:** conversão entre tipos inteiros (`i64` → `usize`), validação de entrada.

**Files:** Criar `src/cheats.rs`; modificar `src/lib.rs`; criar `tests/cheats.rs`.

**Interfaces — Produces:** em `impl Game`: `pub fn cheat_add_points(&mut self, amount: i64) -> bool` (válido: `1..=1_000_000` e `score + amount <= 1_000_000_000`); `pub fn cheat_set_level(&mut self, level: i64) -> bool` (`1..=10000`; usa `level_start_at(level, 6000)`); `pub fn cheat_spawn_special(&mut self, index: i64, kind: TileKind) -> bool` (índice 0–63; `Normal` é inválido; Pulso/Raio sem cor tentam cores até não formar combinação); `pub fn cheat_shuffle(&mut self) -> bool`; `pub fn cheat_no_defeat(&mut self, enabled: bool) -> bool`; `pub fn cheat_explode(&mut self) -> MoveResult`; `pub fn cheat_supernova(&mut self, index: i64) -> MoveResult` (índice inválido = `MoveResult::invalid()`).
**Consumes:** `Game`, `snapshot`, `remember_state`, `resolve_board`, `level_start_at`, `level_goal`, `rescue`, `shuffle`, `to_saved`, `restore`.

Regras: `engine.js` linhas 341–425. Todo cheat válido chama `remember_state` e marca `cheats_used = true`; os inválidos não mudam nada.

- [ ] **Passo 1: Testes** em `tests/cheats.rs`, portando os 7 de `developer-cheats.test.js` (auxiliar `valid_board(&Game)`; `saved(game)` = `game.to_saved()`; igualdade de `snapshot()` via `PartialEq`). As entradas `NaN` e `1.5` do JS **deixam de existir** (o tipo `i64` já as impede); manter `0`, `-1`, `1000001`, `10001`, índices `-1` e `64`. Onde o JS troca `hasMove`, usar `force_no_moves`. Onde o JS escreve `tile.color = ...`, escrever em `game.board[r][c].color`. Nomes:
  - `points_and_level_cheats_preserve_valid_progress_and_reject_invalid_input_without_side_effects`
  - `special_cheats_preserve_other_tiles_and_resolve_spectrum_color_safely`
  - `secret_shuffle_resumes_classic_preserves_tiles_and_remains_reversible`
  - `no_defeat_recovers_a_deadlocked_classic_and_survives_saving_normal_classic_still_ends`
  - `explode_activates_every_special_once_refills_and_undoes_the_complete_cascade` (`earned >= 1600`)
  - `forced_supernova_uses_real_fusion_at_every_corner_and_is_reversible` (índices 0, 7, 56, 63, 27)
  - `history_mixes_normal_moves_and_cheats_retains_50_actions_and_keeps_cheat_status_after_undo`
- [ ] **Passo 2: Rodar e ver falhar.** `cargo test --test cheats`.
- [ ] **Passo 3: Implementar** os `cheat_*`.
- [ ] **Passo 4: Rodar.** `cargo test`. Esperado: tudo passando.
- [ ] **Passo 5: Commit** (`feat: cheats de desenvolvedor`) e **parar para aprovação**.

---

### Tarefa 8: Conferência com o `engine.js` original

**Conceitos:** ler texto linha a linha, `split`, `parse`.

**Files:** Criar `tests/golden/generate.mjs`, `tests/golden/zen-7654321.txt`, `tests/golden/classic-42.txt`, `tests/cross_check.rs`.

**Interfaces — Produces:** nenhuma (verificação). **Consumes:** tudo de `Game`, `lcg_random`, `Mode`, `hint`, `play`, `Event::name`.

O gerador usa o `engine.js` **original**: `node tests/golden/generate.mjs <caminho do engine.js> tests/golden`. Dois cenários com a fonte LCG igual à do Rust: (A) Zen, semente 7654321, até 100 jogadas; (B) Classic, semente 42 (`new Game(lcg)` e depois `newGame('classic')`, como no Rust: `Game::with_random(...)` seguido de `new_game(Mode::Classic)`), até 300 jogadas ou `ended`. Em cada rodada a jogada vem de `game.hint()`, o que também confere o `hint`. Formato de texto, sem JSON: uma linha `turn N a b earned E chain C score S level L next_id I events nome1,nome2,...` seguida de 8 linhas do tabuleiro, cada uma com 8 tokens `id:cor:tipo` (cor `-` para Espectro; tipo `n`, `b`, `c` ou `s`). O teste Rust repete a mesma sequência e compara rodada a rodada, apontando a primeira rodada que diverge.

- [ ] **Passo 1: Escrever o gerador** e rodá-lo. Esperado: dois arquivos `.txt` criados; conferir que o primeiro tem 100 blocos `turn`.
- [ ] **Passo 2: Escrever `tests/cross_check.rs`** com `rust_matches_the_original_js_engine_zen_seed_7654321` e `rust_matches_the_original_js_engine_classic_seed_42`.
- [ ] **Passo 3: Rodar.** `cargo test --test cross_check`. Esperado: passa. Se divergir, a primeira rodada diferente mostra o que investigar: o mais provável é a **ordem ou a quantidade de sorteios** (tabuleiro inicial, pedras novas, embaralhar) ou a ordem de `activated`. Corrigir o Rust; se a causa for estranheza do JS, anotar em `docs/notes/js-quirks.md` e perguntar ao Thiago.
- [ ] **Passo 4: Rodar tudo.** `cargo test`. Esperado: todos os testes passando (36 portados + os extras), sem avisos.
- [ ] **Passo 5: Commit** (`test: conferência com o engine.js original`).
- [ ] **Passo 6: Parar para aprovação** e fechar a fase 1: resumir ao Thiago o que existe, o que aprendeu de Rust e propor o planejamento da fase 2.
