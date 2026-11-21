# Prisma Fase 2 · M0a "Olá, tabuleiro" — Plano de implementação

> **Para quem executa:** REQUIRED SUB-SKILL: use superpowers:executing-plans (execução nativa, tarefa por tarefa). Os passos usam caixas `- [ ]` para acompanhar.

**Objetivo:** o pacote `game/` abre uma janela retrato, desenha um tabuleiro do motor (semente fixa) com as imagens reais das pedras, tira "foto" de si mesmo para conferência, e o GitHub Actions gera o APK que o Thiago instala no celular.

**Arquitetura:** a raiz continua sendo o motor `prisma_engine`; ela ganha `[workspace] members = ["game"]`. O pacote `prisma_game` (só binário) tem três módulos pequenos: `layout` (contas puras de retângulos), `assets` (PNG embutidos com `include_bytes!` e a escolha da imagem de cada pedra) e `render` (desenho com Macroquad). Os PNG saem de um script Python que fatia os atlas do Prisma em JS. O APK sai do `cargo quad-apk` dentro do Docker `notfl3/cargo-apk`.

**Tecnologias:** Rust (edition 2021, ver decisão D1), Macroquad 0.4.16 (sem a feature `audio`), Python 3 + Pillow (só para preparar imagens), GitHub Actions + Docker `notfl3/cargo-apk`.

**Spec:** `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md` (seções 5.1, 5.2, 5.4, 5.7, 5.8, 5.9 e marco M0a da seção 6).

## Decisões deste plano (a confirmar pelo Thiago na revisão)

- **D1 · Edition 2021 em vez de 2024.** Testei aqui: o `cargo-quad-apk` 0.1.4 usa um Cargo de 2022 por dentro e recusa qualquer `Cargo.toml` com `edition = "2024"` ("this version of Cargo is older than the `2024` edition"). Com 2021 ele passa dessa etapa. O engine tem só 1 trecho que exige 2024 (`if let … && …` em `src/resolve.rs:67`, "let chains"); reescrevo com `let … else`. Numa cópia, os 72 testes passam em 2021. Custo se estiver errado: nenhum no comportamento (os testes provam); o fork "Rust 2024" do `cargo-quad-apk` fica como reserva (Tarefa 6).
- **D2 · SDK padrão da ferramenta no M0a.** A seção 5.7 diz "minSdk 26 e targetSdk 35 se a ferramenta permitir". O README do `cargo-quad-apk` 0.1.4 traz `android_version`/`target_sdk_version` 29 como padrão; não verifiquei se alvos mais novos assinam e instalam bem com essa ferramenta antiga, e aqui não dá para testar. No M0a não declaro versões de SDK: uso o padrão da ferramenta e revejo na hora de assinar com chave fixa (M2). Custo se estiver errado: o app aparece com aviso de "feito para versão antiga" em Androids novos; nada além.
- **D3 · Imagens embutidas, sem pasta `assets` no APK.** Como a seção 5.2 já prevê (`include_bytes!`), o APK não precisa da chave `assets` dos metadados: menos uma coisa que pode dar errado no Android.
- **D4 · Só o corpo das pedras.** Ficam para depois (M4 ou pedido dele): sombra/brilho dos filtros CSS, o ajuste de matiz das especiais da pedra 0, o aumento da pedra selecionada e o fundo `prisma-bg.jpg`. O fundo do M0a é cor lisa `#101229` e painel do tabuleiro `#0d142c` (valores do `style.css` do JS).
- **D5 · O JS centraliza à mão, eu centralizo por medida.** O JS corrige cada pedra com deslocamentos (`--gem-x`, `--gem-y`) e escala (`--gem-scale`) escritos à mão. O script centraliza pelo pixel visível (alfa ≥ 50%, como a seção 5.4 pede) e reaproveita só a escala do JS, tabela `[.98, 1, 1.08, .87, .96, 1.08, .9]`; o Espectro usa 1,18 (o `background-size:118%` do JS).

## Restrições globais (valem para todas as tarefas)

- Motor sem dependências; mudança no motor só a da D1 (`src/resolve.rs` e a edition).
- `cargo test` na raiz continua testando só o motor: **72 testes**; `cargo test --workspace` testa tudo.
- Comentários e mensagens de teste em português, explicando conceitos novos de Rust em linguagem simples (leitor com programação básica).
- Janela de desenvolvimento 540×960 (retrato). Pacote Android `com.thiagodluz.prisma.rust`, rótulo "Prisma (Rust)", retrato fixo, sem permissões.
- Imagens: CC BY-SA 4.0 © Thiago Luz, com aviso e lista de alterações junto dos PNG. Sem `rust-anything`.
- Commits em português, no estilo `feat:` / `test:` / `ci:` / `docs:`, terminando com as duas linhas de atribuição da sessão (`Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` e `Claude-Session: https://claude.ai/code/session_01Y38ZhQcV92bomwv9cFgeqB`).
- Ferramentas desta nuvem: Xvfb com `LIBGL_ALWAYS_SOFTWARE=1` para abrir janela; sem SDK/NDK do Android (o APK só sai no GitHub). Pasta do Prisma em JS (só leitura): `/home/claude/thiagodluz/prisma`.

## Foco da revisão (o que os testes das tarefas não esgotariam sozinhos)

1. Janela de tamanho degenerado (0×0 ao minimizar, 1×1, muito larga, muito alta): `board_rect` não pode devolver largura/altura negativa, NaN, nem sair da tela. → teste na Tarefa 2.
2. Pedra sem cor ou com cor ≥ 7 (save velho, cheat): nada de pânico, a casa fica sem imagem. → teste na Tarefa 3.
3. Atlas fatiado errado (a mesma pedra sete vezes, casa vazia, pedra fora do centro). → testes na Tarefa 3.
4. `--screenshot` sem caminho: abre a janela normal, não quebra. → teste na Tarefa 4.
5. Alguém volta a edition 2024 no futuro e o APK quebra só no primeiro envio. → teste na Tarefa 1.

## Estrutura de arquivos

| Arquivo | Responsabilidade |
| --- | --- |
| `Cargo.toml` (raiz) | edition 2021; `[workspace] members = ["game"]` |
| `src/resolve.rs` | só o `let … else` da D1 |
| `tests/workspace.rs` | guarda: nenhuma `Cargo.toml` do projeto usa edition 2024 |
| `game/Cargo.toml` | pacote `prisma_game`; metadados do Android |
| `game/src/main.rs` | janela, argumentos, tabuleiro de vitrine, laço principal |
| `game/src/layout.rs` | `Rect`, `board_rect`, `cell_rect` (puro) |
| `game/src/assets.rs` | `Sprite`, `sprite_for`, `all_sprites`, `png_bytes` (PNG embutidos) |
| `game/src/render.rs` | `Sprites::load`, `draw_frame` (Macroquad) |
| `tools/prepare_assets.py` | atlas do JS → `game/assets/*.png` e ícones do Android |
| `game/assets/*.png`, `game/assets/LICENSE-ART.md` | 22 imagens + aviso de licença |
| `game/android_res/mipmap-*/ic_launcher.png` | ícone do app |
| `.github/workflows/test.yml`, `android.yml` | CI |
| `docs/notes/android-build.md` | por que edition 2021, como gerar o APK, como instalar |

---

### Tarefa 1: Workspace, edition 2021 e janela vazia

**Arquivos:**
- Criar: `tests/workspace.rs`, `game/Cargo.toml`, `game/src/main.rs`
- Modificar: `Cargo.toml` (raiz), `src/resolve.rs:65-73`

**Interfaces:**
- Produz: pacote `prisma_game` (binário) dependendo de `prisma_engine = { path = ".." }` e `macroquad = "0.4"` (resolve para 0.4.16); `fn window_conf() -> Conf` em `game/src/main.rs`.

- [ ] **Passo 1: teste que falha.** Em `tests/workspace.rs`, `fn nenhum_cargo_toml_usa_edition_2024()`: lê `Cargo.toml` da raiz e `game/Cargo.toml` (via `env!("CARGO_MANIFEST_DIR")`) e exige que cada um contenha `edition = "2021"` e não contenha `edition = "2024"`; a mensagem de falha explica que o `cargo-quad-apk` não lê 2024.
- [ ] **Passo 2: rodar e ver falhar.** `cargo test --test workspace`. Esperado: FALHA (a raiz está em 2024 e `game/Cargo.toml` não existe).
- [ ] **Passo 3: edition e engine.** Mudar a raiz para `edition = "2021"` e acrescentar `[workspace] members = ["game"]`. Rodar `cargo build`: esperado erro "let chains are only allowed in Rust 2024 or later" em `src/resolve.rs:67`. Reescrever `Reaction::enqueue` com `let Some(tile) = self.game.tile(index) else { return; };` seguido do `if` com as outras três condições (mesma lógica, mesma ordem).
- [ ] **Passo 4: pacote `game`.** `game/Cargo.toml` com `edition = "2021"`, as duas dependências acima e, em `[package.metadata.android]`, apenas `package_name = "com.thiagodluz.prisma.rust"` e `label = "Prisma (Rust)"`; `[package.metadata.android.activity_attributes]` com `"android:exported" = "true"` e `"android:screenOrientation" = "portrait"`. `game/src/main.rs`: `fn window_conf() -> Conf` (título "Prisma", 540×960, `high_dpi: true`, `window_resizable: true`) e `#[macroquad::main(window_conf)] async fn main()` que limpa a tela com `Color::from_hex(0x101229)` a cada quadro.
- [ ] **Passo 5: rodar tudo.** `cargo test --workspace` e `cargo test` (raiz). Esperado: o teste novo passa; o total do motor continua **72** testes passando (somar as linhas `test result` da raiz) e nenhum aviso novo do compilador; `cargo build -p prisma_game` compila.
- [ ] **Passo 6: Commit.** `git add Cargo.toml Cargo.lock src/resolve.rs tests/workspace.rs game && git commit -m "feat: workspace com o pacote game e edition 2021 (exigência do cargo-quad-apk)"`.

---

### Tarefa 2: `layout` — onde fica o tabuleiro

**Arquivos:**
- Criar: `game/src/layout.rs`
- Modificar: `game/src/main.rs` (`mod layout;`)

**Interfaces:**
- Consome: `prisma_engine::SIZE` (8).
- Produz, em `layout.rs`:
  - `#[derive(Clone, Copy, Debug, PartialEq)] pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }`
  - `pub fn board_rect(screen_w: f32, screen_h: f32) -> Rect` — quadrado centrado na tela, lado `min(w, h)` menos margem de 3% de cada lado; tela com tamanho ≤ 0 ou não finito (NaN) devolve `Rect` de tamanho zero.
  - `pub fn cell_rect(board: Rect, row: usize, col: usize) -> Rect` — casa `(row, col)`, com `row`/`col` de 0 a `SIZE - 1`.

- [ ] **Passo 1: testes que falham** (em `#[cfg(test)] mod tests` do próprio arquivo), com tolerância de 0,01 nas comparações de `f32`:
  - `tabuleiro_e_quadrado_e_cabe_na_tela`: para 540×960, 960×540, 1080×2400, 400×400 → `w == h`, `x ≥ 0`, `y ≥ 0`, `x + w ≤ tela_w`, `y + h ≤ tela_h`.
  - `tabuleiro_fica_centralizado`: para 540×960 → `x + w/2 == 270` e `y + h/2 == 480`.
  - `retrato_e_paisagem_usam_o_menor_lado`: 540×960 e 960×540 dão o mesmo `w`, que vale `540 × 0,94 = 507,6`.
  - `casas_ladrilham_o_tabuleiro`: `cell_rect(b, 0, 0)` começa em `(b.x, b.y)`; `cell_rect(b, 7, 7)` termina em `(b.x + b.w, b.y + b.h)`; para todo `(r, c)` com `c < 7`, `cell.x + cell.w == vizinha_da_direita.x` (e igual para linhas); todas as casas têm `w == b.w / 8`.
  - `tela_degenerada_nao_quebra`: para `(0,0)`, `(1,1)`, `(-5,100)`, `(f32::NAN, 100)`, `(100000, 1)` → resultado com `w ≥ 0`, `h ≥ 0`, sem NaN, dentro da tela (ou tamanho zero), sem pânico.
- [ ] **Passo 2: ver falhar.** `cargo test -p prisma_game layout`. Esperado: erro de compilação (funções não definidas).
- [ ] **Passo 3: implementar** `Rect`, `board_rect`, `cell_rect` como acima. Comentário simples explicando `f32` e por que a tela é tratada como número solto (testável sem abrir janela).
- [ ] **Passo 4: ver passar.** `cargo test -p prisma_game layout`. Esperado: 5 testes passando.
- [ ] **Passo 5: Commit.** `git add game/src && git commit -m "feat: layout puro do tabuleiro (retângulo e casas)"`.

---

### Tarefa 3: Imagens — script, PNG e módulo `assets`

**Arquivos:**
- Criar: `tools/prepare_assets.py`, `game/assets/{gem,burst,cross}-{0..6}.png`, `game/assets/spectrum.png`, `game/assets/LICENSE-ART.md`, `game/android_res/mipmap-{mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}/ic_launcher.png`, `game/src/assets.rs`
- Modificar: `game/src/main.rs` (`mod assets;`)

**Interfaces:**
- Consome: `prisma_engine::{Tile, TileKind, COLORS}`; `macroquad::texture::Image` (só nos testes).
- Produz, em `assets.rs`:
  - `pub const CELL_PX: u32 = 128;` `pub const SPRITE_PX: u32 = 160;` `pub const SPRITE_RATIO: f32 = 1.25;` (= `SPRITE_PX / CELL_PX`: o PNG é a casa mais margem, para a pedra ampliada não ser cortada).
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum Sprite { Gem(u8), Burst(u8), Cross(u8), Spectrum }` — só nasce com índice 0..=6 (garantido por `sprite_for` e `all_sprites`).
  - `pub fn sprite_for(tile: &Tile) -> Option<Sprite>` — `Normal`+cor → `Gem`, `Burst` → `Burst`, `Cross` → `Cross`, `Spectrum` → `Spectrum` (cor ignorada); cor ausente ou ≥ `COLORS` (nas três primeiras) → `None`.
  - `pub fn all_sprites() -> Vec<Sprite>` — as 22, sem repetição.
  - `pub fn png_bytes(sprite: Sprite) -> &'static [u8]` — tabelas de `include_bytes!("../assets/…")`.
- Dados do script (copiados do `style.css` do JS, linhas 105–117): posição de cada pedra no atlas 3×3 de 418 px, `(coluna, linha)`: 0→(0,0) 1→(1,0) 2→(2,0) 3→(0,1) 4→(1,1) 5→(2,1) 6→(0,2); as casas (1,2) e (2,2) são vazias; escala por pedra `[.98, 1, 1.08, .87, .96, 1.08, .9]`; Pulso e Raio usam a mesma posição e a mesma escala da pedra de mesmo número; Espectro: `spectrum-gem.webp` inteira, escala 1,18.

- [ ] **Passo 1: testes que falham** (em `#[cfg(test)] mod tests` de `assets.rs`; descodificam com `Image::from_file_with_format(png_bytes(s), None)`, que não precisa de janela):
  - `cada_tipo_e_cor_escolhe_a_imagem_certa`: `Tile { id: 1, color: Some(3), kind: Normal }` → `Gem(3)`; `Burst` → `Burst(3)`; `Cross` → `Cross(3)`; `Spectrum` com `None` e com `Some(2)` → `Spectrum`.
  - `pedra_sem_cor_valida_nao_tem_imagem`: `Normal` com `None`, `Some(7)`, `Some(255)`; `Burst` com `None`; `Cross` com `Some(9)` → `None` em todos.
  - `existem_22_imagens_diferentes`: `all_sprites()` tem 22 itens e 22 valores distintos (conjunto).
  - `todo_png_decodifica_com_o_tamanho_combinado`: para cada sprite, `width == height == SPRITE_PX`; `SPRITE_PX as f32 / CELL_PX as f32 == SPRITE_RATIO`.
  - `todo_png_esta_centrado_pelo_pixel_visivel`: para cada sprite, caixa dos pixels com alfa ≥ 128; o centro da caixa `((min_x + max_x) / 2, (min_y + max_y) / 2)` fica a no máximo 2 px de `(79,5; 79,5)`.
  - `todo_png_tem_corpo_visivel`: pixels com alfa ≥ 128 somam pelo menos 15% de `CELL_PX²`.
  - `as_sete_pedras_de_cada_familia_sao_diferentes`: os bytes decodificados de `Gem(0..6)` são todos distintos entre si; idem `Burst` e `Cross`.
- [ ] **Passo 2: ver falhar.** `cargo test -p prisma_game assets`. Esperado: erro de compilação (módulo vazio / PNG inexistentes em `include_bytes!`).
- [ ] **Passo 3: `tools/prepare_assets.py`** — uso: `python3 tools/prepare_assets.py <pasta-do-prisma-js> game`. Para cada atlas (`gem-atlas.webp`, `burst-atlas.webp`, `cross-atlas.webp`): recortar a pedra `i` (418×418), converter para alfa pré-multiplicado (`RGBa`), reduzir com LANCZOS para `round(128 × escala[i])` px, voltar a `RGBA`, achar a caixa dos pixels com alfa ≥ 128 e colar numa tela transparente 160×160 de modo que o centro da caixa caia em `(80, 80)` (`paste` sem máscara, que copia o alfa como está). `spectrum-gem.webp` inteira → `round(128 × 1,18)` px, mesmo tratamento. Ícones: `icon-512.png` → `game/android_res/mipmap-{mdpi 48, hdpi 72, xhdpi 96, xxhdpi 144, xxxhdpi 192}/ic_launcher.png`. Imprimir uma linha por arquivo gerado. Rodar o script.
- [ ] **Passo 4: `game/assets/LICENSE-ART.md`** (português): as imagens são adaptações das do Prisma em JS (`gem-atlas`, `burst-atlas`, `cross-atlas`, `spectrum-gem`), © 2026 Thiago Luz, CC BY-SA 4.0; alterações: fatiadas por pedra, centralizadas, redimensionadas, convertidas para PNG; link para `LICENSE-ART.md` do repositório original; o nome Prisma e os ícones ficam fora da licença.
- [ ] **Passo 5: implementar `assets.rs`** com as assinaturas acima. `png_bytes` indexa as três tabelas de 7 e o Espectro; `sprite_for` devolve `None` em vez de indexar fora.
- [ ] **Passo 6: ver passar.** `cargo test -p prisma_game assets`. Esperado: 7 testes passando. Se `todo_png_tem_corpo_visivel` ou o centro falharem por causa de um limite mal escolhido (e não de um defeito do script), abrir um dos PNG com `Read` para decidir e registrar a decisão na linha do ledger.
- [ ] **Passo 7: Commit.** `git add tools game && git commit -m "feat: imagens das pedras (script, PNG e módulo assets) e ícone do Android"`.

---

### Tarefa 4: `render` e tabuleiro de vitrine, com "foto"

**Arquivos:**
- Criar: `game/src/render.rs`
- Modificar: `game/src/main.rs`

**Interfaces:**
- Consome: `layout::{board_rect, cell_rect, Rect}`; `assets::{all_sprites, png_bytes, sprite_for, Sprite, SPRITE_RATIO}`; engine `Game::with_random`, `lcg_random`, `Game.board`, `Tile`, `TileKind`, `SIZE`.
- Produz:
  - `render.rs`: `pub struct Sprites` com `pub fn load() -> Sprites` (cria uma `Texture2D` por sprite com `Texture2D::from_file_with_format(png_bytes(s), None)`; só roda com janela aberta) e `pub fn draw_frame(board: &Board, sprites: &Sprites)` (limpa com `#101229`, pinta o painel `#0d142c` em `board_rect(screen_width(), screen_height())`, e para cada casa com `sprite_for(..) == Some(s)` desenha a textura com lado `cell.w × SPRITE_RATIO` centrada na casa; casa sem imagem não desenha nada).
  - `main.rs`: `fn screenshot_path(args: &[String]) -> Option<String>` (devolve o texto depois de `--screenshot`; sem texto depois, `None`); `fn demo_game() -> Game` (semente `lcg_random(1)`; linha 0 = Pulso das cores 0..6 nas colunas 0..6; linha 1 = Raio das cores 0..6; casa `(2,0)` = Espectro; ids novos e distintos; resto como o motor gerou); o laço principal chama `draw_frame` e, com `--screenshot`, grava o PNG no segundo quadro e sai.

- [ ] **Passo 1: testes que falham** (em `#[cfg(test)]` de `main.rs`):
  - `foto_sem_caminho_e_ignorada`: `screenshot_path(&[])` → `None`; `["--screenshot"]` → `None`; `["--screenshot", "a.png"]` → `Some("a.png")`; `["x", "--screenshot", "b.png"]` → `Some("b.png")`.
  - `a_vitrine_mostra_as_22_imagens`: o conjunto de `sprite_for` sobre as 64 casas de `demo_game().board` contém todos os itens de `all_sprites()`. Se a semente 1 não trouxer as sete cores comuns, trocar a semente (a menor que funcione) e registrar a decisão.
- [ ] **Passo 2: ver falhar.** `cargo test -p prisma_game`. Esperado: erro de compilação (funções não definidas).
- [ ] **Passo 3: implementar** `screenshot_path`, `demo_game` e `render.rs`; ligar o laço: `Sprites::load()` uma vez; por quadro `draw_frame(&game.board, &sprites)`; com foto pedida, no quadro de número 1 (o segundo) chamar `get_screen_data().export_png(&caminho)` e `std::process::exit(0)`.
- [ ] **Passo 4: ver passar.** `cargo test --workspace`. Esperado: tudo passando (motor 72 + guarda + testes do `game`).
- [ ] **Passo 5: foto na tela virtual.** `LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a -s "-screen 0 1280x1024x24" cargo run -p prisma_game -- --screenshot /tmp/m0a.png`. Esperado: sai sozinho com código 0 e cria `/tmp/m0a.png` de 540×960.
- [ ] **Passo 6: conferir a foto.** Com `Read` em `/tmp/m0a.png`: tabuleiro quadrado centrado em fundo escuro, 8×8 casas, linha 0 com 7 Pulsos, linha 1 com 7 Raios, Espectro em `(2,0)`, pedras centradas nas casas e sem recorte. Depois uma conferência automática em Python (Pillow): pixel `(2,2)` é `(16,18,41)`; pixel no centro da casa `(0,0)` — `(48, 258)` — difere do fundo e do painel `(13,20,44)`. Se a foto mostrar defeito, corrigir e repetir.
- [ ] **Passo 7: Commit.** `git add game && git commit -m "feat: tabuleiro de vitrine na tela, com modo --screenshot"`.

---

### Tarefa 5: Arquivos do Android e do CI (sem GitHub ainda)

**Arquivos:**
- Criar: `.github/workflows/test.yml`, `.github/workflows/android.yml`, `docs/notes/android-build.md`
- Modificar: `game/Cargo.toml` (ícone)

**Interfaces:**
- Consome: ícones da Tarefa 3; pacote `prisma_game`.
- Produz: dois fluxos do GitHub Actions; o artefato `prisma-debug-apk`.

- [ ] **Passo 1: ícone nos metadados.** Em `[package.metadata.android]` de `game/Cargo.toml`: `res = "android_res"` e `icon = "@mipmap/ic_launcher"` (nomes e formato conferidos no README do `cargo-quad-apk` 0.1.4). Nada de `assets`, `android_version`, `min_sdk_version` ou `target_sdk_version` (D2, D3).
- [ ] **Passo 2: `test.yml`.** Nome "Testes"; dispara em `push` e `pull_request`; `ubuntu-latest`; `actions/checkout@v4`; `dtolnay/rust-toolchain@stable`; `cargo test --workspace`.
- [ ] **Passo 3: `android.yml`.** Nome "Android"; dispara em `push` na `main` e `workflow_dispatch`; `ubuntu-latest`; `actions/checkout@v4`; passo `sed -i 's/^version = 4$/version = 3/' Cargo.lock` (o Cargo antigo da ferramenta pode não ler o formato novo do arquivo; só dentro do CI, sem commitar); passo `docker run --rm -v "$PWD":/root/src -w /root/src notfl3/cargo-apk cargo quad-apk build -p prisma_game` (modo debug); passo `actions/upload-artifact@v4` com nome `prisma-debug-apk` e caminho `target/android-artifacts/debug/apk/*.apk`, falhando se não houver arquivo (`if-no-files-found: error`).
- [ ] **Passo 4: conferir o que dá para conferir aqui.** (a) Os dois YAML descodificam: `python3 -c "import yaml,sys; [yaml.safe_load(open(f)) for f in sys.argv[1:]]" .github/workflows/*.yml` (sem erro). (b) Com o `cargo-quad-apk` instalado em `/tmp/cqa-bin` e `ANDROID_HOME=/tmp/fake-sdk NDK_HOME=/tmp/fake-ndk`: `cargo quad-apk build -p prisma_game` deve passar da leitura dos manifestos e parar em "Android SDK has no build-tools directory" (prova que o Cargo antigo aceita o workspace; não gera APK). Se o `/tmp/cqa-bin` não existir mais, reinstalar com `cargo install cargo-quad-apk --root /tmp/cqa-bin`. (c) `cargo test --workspace` continua verde.
- [ ] **Passo 5: `docs/notes/android-build.md`** (português, curto): por que edition 2021 e `Cargo.lock` versão 3 no CI; comando do Docker para gerar o APK; como instalar (baixar o artefato no celular, extrair o `.zip`, permitir "instalar apps desconhecidos"); aviso de que a chave de debug muda a cada execução, então é preciso desinstalar antes de instalar outro APK até a chave fixa (M2).
- [ ] **Passo 6: Commit.** `git add .github docs game/Cargo.toml && git commit -m "ci: testes e APK no GitHub Actions, com notas de como gerar e instalar"`.

---

### Tarefa 6: Primeira execução no GitHub e APK no celular

**Pré-condição (só o Thiago pode cumprir):** criar no GitHub o repositório **vazio** `prisma-rust` (sem README, sem licença, sem .gitignore) e informar o endereço. As Tarefas 1–5 não dependem disso. Antes de qualquer `git push`, pedir o "pode" dele (ele aprova cada ação).

**Arquivos:** os que a escada de correções abaixo exigir (`.github/workflows/android.yml`, `game/Cargo.toml`, `docs/notes/android-build.md`).

**Interfaces:**
- Consome: fluxos da Tarefa 5.
- Produz: execuções verdes de "Testes" e "Android"; o APK `prisma-debug-apk`; a confirmação do Thiago de que o tabuleiro aparece de pé.

- [ ] **Passo 1: ligar o repositório.** `add_repo` com acesso de escrita para `thiagodluz/prisma-rust`; `git remote add origin <endereço>`; com o "pode" do Thiago, `git push -u origin main`.
- [ ] **Passo 2: ver as execuções.** `gh run list` e `gh run watch` (ou, se o `gh` não alcançar, o endereço da aba Actions). Esperado: "Testes" verde; "Android" verde com o artefato `prisma-debug-apk` contendo um `.apk`.
- [ ] **Passo 3: se "Android" ficar vermelho,** ler o log (`gh run view --log-failed`) e aplicar **uma** correção por tentativa, nesta ordem, cada uma em seu commit `ci: …` e com novo push: (a) Rust do contêiner antigo demais → rodar `rustup update stable` dentro do contêiner antes do `cargo quad-apk` (`sh -c "…"`); (b) formato do `Cargo.lock` ou resolução do registro → apagar o `Cargo.lock` no CI e deixar o Cargo antigo resolver; (c) a ferramenta não compila o macroquad atual → fixar `macroquad = "=0.4.<versão anterior que o log indicar>"`; (d) ferramenta antiga sem conserto → trocar por `cargo-quad-apk` do fork `hoklims/cargo-quad-apk` (edition 2024) instalado com `cargo install --git`, num fluxo com SDK/NDK 25b do runner (`ANDROID_HOME`/`NDK_HOME`), voltando a edition 2024 só se o fork exigir. Depois de **3 tentativas sem verde**, parar e contar ao Thiago o que o log mostra e as opções (inclui o Plano B da seção 4 da spec).
- [ ] **Passo 4: entregar o APK.** Tentar `gh run download <id> -n prisma-debug-apk -D /tmp/apk` e, se funcionar, enviar o `.apk` com `SendUserFile`; se não, explicar em poucas linhas como baixar o artefato pela página do GitHub no celular (instruções de `docs/notes/android-build.md`).
- [ ] **Passo 5: conferência no celular (Thiago).** Instalar e abrir. Esperado: o tabuleiro 8×8 aparece de pé, sem recorte, com sete Pulsos e sete Raios nas duas primeiras linhas e um Espectro na terceira. Qualquer defeito (deitado, cortado, fecha sozinho, preto) vira correção nova, com `adb logcat` se ele puder, ou descrição/foto da tela.
- [ ] **Passo 6: fechar o marco.** Registrar no ledger; contar ao Thiago o resultado; o plano do M0b só nasce depois disso.

---

## Autoavaliação

- **Cobertura da spec:** workspace e `game/` (5.1, T1); `layout`, `assets`, `render` do 5.2 (T2–T4; `input`, `playback`, `audio`, `storage`, `app` ficam para M1–M3); imagens e licença (5.4, T3); foto (5.9, T4); Android, ícone, pacote, retrato (5.7, T1/T3/T5); CI de dois fluxos (5.8, T5); critério "APK instala e mostra o tabuleiro de pé" (T6). Desvios da spec: D1, D2, D3 e a escolha de não exigir minSdk/targetSdk — todos listados acima.
- **Consistência de nomes:** `Rect`, `board_rect`, `cell_rect` (T2) usados em `render` (T4); `Sprite`, `sprite_for`, `all_sprites`, `png_bytes`, `SPRITE_RATIO` (T3) usados em T4; artefato `prisma-debug-apk` igual em T5 e T6; pacote `prisma_game` igual em todo lugar.
- **Proporção:** cobre um marco de seis da spec; as tarefas trazem assinaturas, nomes de teste com seus valores e os números que vêm do JS; nenhum corpo de função.
