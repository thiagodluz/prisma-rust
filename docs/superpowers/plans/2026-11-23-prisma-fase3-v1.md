# Prisma Rust · Fase 3 · V1 (tabuleiro e cores) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a tela de jogo do Rust ganha o fundo em foto, a moldura e as casas do tabuleiro, o cartão de pontos, a barra em degradê e os três botões arredondados com ícone, como no "Novo" do JS.

**Architecture:** formas (cantos arredondados, degradê, borda, sombra) são pintadas por código em Rust, uma vez, na abertura do app, e desenhadas esticadas em "nove fatias". Toda a matemática (pintura, fatias, corte da foto, ícones, posições) é função pura com teste; só o desenho em si (`render.rs`) é conferido por foto.

**Tech Stack:** Rust, Macroquad 0.4.16 (`Texture2D::from_rgba8`, `draw_texture_ex`, `draw_mesh`), crate `image` 0.24 só com `jpeg` (para a foto).

**Spec:** `docs/superpowers/specs/2026-11-23-prisma-fase3-visual-design.md` (seções 1 a 8). Referências visuais: `docs/notes/ref/js-novo.jpg` (o JS) e `docs/notes/ref/rust-m4.png` (o Rust antes da V1); o CSS do JS está em `/home/claude/thiagodluz/prisma/style.css` (somente leitura).

## Global Constraints

- Esta etapa **não** mexe em: topo (Zen, Clássico, ♪, ≡), cartões de modo, fonte (V2), folhas de Som e Menu (só ganham o visual por tabela), regras, som, vibração, salvamento.
- Toda área tocável vem de `layout.rs`; `render.rs` nunca decide toque.
- Nada é gerado por quadro: texturas só em `Sprites::load()`. Nenhum `Texture2D::from_*` dentro de função de desenho.
- Tela impossível (0, negativa, NaN, infinita) continua devolvendo tudo vazio; nenhuma conta nova pode dar `NaN`.
- `game/` e `tools/` não são rustfmt-limpos: não reformatar. Clippy do jogo igual à linha de base (`cargo clippy -p prisma_game --all-targets 2>&1 | grep -E "^(warning|error)" | sed 's/`[^`]*`/X/g' | sort | uniq -c`).
- Cada função pura nova passa por **uma mutação** (trocar um sinal ou uma constante): o teste dela tem de falhar; depois restaura.
- Ambiente por chamada: `export RUSTFLAGS="-L /tmp/asound" LIBRARY_PATH=/tmp/asound LIBGL_ALWAYS_SOFTWARE=1`. Fotos: `xvfb-run -a -s "-screen 0 1280x2600x24" cargo run -q -p prisma_game -- --cena NOME --screenshot arquivo.png --tamanho LxA` e depois Read no PNG.
- Commits em português (`feat:`/`fix:`/`docs:`/`test:`/`refactor:`) com os dois trailers (`Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` e `Claude-Session: https://claude.ai/code/session_01Y38ZhQcV92bomwv9cFgeqB`), mensagem em arquivo e `git commit -F`. Enviar, rodar o CI e juntar na `main` só com autorização nova do Thiago.
- `prisma-bg.jpg` é CC BY-SA 4.0: entra em `game/assets/LICENSE-ART.md`.

## Review Focus

1. **360×640** (a menor tela): tudo cabe, as casas têm vão visível (≥ 2 px) e as pedras não passam da casa.
2. **Toque na moldura**: tocar na borda da moldura (fora da grade) não seleciona casa; tocar no centro de cada casa seleciona aquela casa.
3. **Clássico** (sem Embaralhar): os dois botões continuam centrados e iguais, com ícone.
4. **Janela mudando de tamanho / tela impossível**: sem `NaN` nas fatias, no corte da foto nem no layout; nada é regenerado por quadro.
5. **Realces e avisos** (casa selecionada, dica pulsando, "Supernova", faixa "Nível N", fim de partida, diálogo, folhas) continuam legíveis sobre a foto e dentro das casas.

## Mapa de arquivos

- Criar `game/src/formas.rs`: `Cor`, `Receita`, `degrade_em`, `pintar`, `Fatia`, `nove_fatias`, `cobrir`, `expandir`.
- Criar `game/src/icones.rs`: polilinhas dos três ícones.
- Modificar `game/src/layout.rs` (`grade`, `cartao_pontos`, `fundo_da_grade`), `game/src/app.rs` / `cenas.rs` / `main.rs` (cada uso de `tela.tabuleiro` que precisa da grade), `game/src/render.rs`, `game/src/main.rs` (`mod formas; mod icones;`), `game/Cargo.toml` (`image` com `jpeg`), `game/assets/` (foto), `docs/notes/visual.md` (novo).

---

### Task 1: Pintar uma forma (`formas::pintar`)

**Files:** Create `game/src/formas.rs`; Modify `game/src/main.rs` (`mod formas;`).

**Interfaces:**
- Produces: `pub type Cor = [u8; 4]` (RGBA, sem pré-multiplicar); `pub const fn rgba(hex: u32) -> Cor` (hex `0xRRGGBBAA`); `pub struct Receita { pub lado: u32, pub raio: u32, pub borda: u32, pub cor_da_borda: Cor, pub degrade: [Cor; 3], pub angulo: f32, pub sombra: u32, pub cor_da_sombra: Cor }`; `pub fn degrade_em(degrade: &[Cor; 3], angulo: f32, x: f32, y: f32) -> Cor`; `pub fn pintar(receita: &Receita) -> Vec<u8>`.
- Convenção: `lado` é o lado da imagem inteira; a forma ocupa o quadrado de `lado - 2*sombra` no meio; `angulo` em graus como no CSS (0 = para cima, 90 = para a direita, 145 = para baixo e à direita); `x`,`y` em 0..1 dentro da forma; as três cores ficam em 0, 0,5 e 1 do degradê, com mistura linear por canal.

- [ ] **Step 1: Escrever os testes (em `formas.rs`, `#[cfg(test)]`).** `degrade_em_segue_o_angulo`: com cores de canal R 10, 100, 200 (A 255): a 90°, `(0,0.3)`→10, `(0.5,0.9)`→100, `(1,0)`→200, `(0.25,0.5)`→55; a 145°, `(0,0)`→10, `(1,1)`→200, `(0.5,0.5)`→100. `pintar_devolve_lado_por_lado_em_rgba`: lado 40 → 6400 bytes. `o_canto_de_fora_fica_transparente`: lado 40, raio 10, sombra 0: alfa de `(0,0)` é 0, de `(20,20)` é 255, de `(20,0)` é 255. `a_borda_tem_a_cor_da_borda`: borda 2, cor `[255,0,0,255]`: `(20,0)` e `(20,1)` são `[255,0,0,255]`, `(20,2)` não; o centro `(20,20)` difere da cor da borda e, com cores opacas, fica a ±1 do 2º degradê. `o_arco_tem_pixels_meio_transparentes`: existe pixel com 0 < alfa < 255 no quadrado 10×10 do canto. `a_forma_e_simetrica`: alfa(x,y) == alfa(lado-1-x, y) em todos os pixels. `a_sombra_some_para_fora`: lado 64, raio 6, sombra 8, cor da sombra `[0,0,0,200]`: em `x=32`, alfa(y) não decresce de y=0 até y=8; alfa `(32,0)` < alfa `(32,7)`; `(32,8)` e `(32,32)` têm alfa 255.
- [ ] **Step 2: Rodar `cargo test -p prisma_game formas`.** Expected: não compila (funções não existem) = RED.
- [ ] **Step 3: Implementar** as quatro funções em `game/src/formas.rs`. Abordagem: distância com sinal até o retângulo arredondado (alfa do contorno = `clamp(0,5 - d, 0, 1)`, 1 px de suavização); a borda é a faixa `d > -borda`, composta por cima do degradê; a sombra é `cor_da_sombra` com alfa × `(1 - smoothstep(0, sombra, distância para fora))`, composta por baixo da forma.
- [ ] **Step 4: Rodar `cargo test -p prisma_game formas`.** Expected: todos passam. **Mutação:** trocar `0.5 - d` por `0.5 + d` e ver `o_canto_de_fora_fica_transparente` falhar; restaurar.
- [ ] **Step 5: Commit** `feat: formas pintadas por código (cantos, degradê, borda e sombra)`.

### Task 2: Nove fatias, corte da foto e margem (`formas`)

**Files:** Modify `game/src/formas.rs`.

**Interfaces:**
- Consumes: `layout::Rect` (`x, y, w, h`).
- Produces: `pub struct Fatia { pub origem: Rect, pub destino: Rect }`; `pub fn nove_fatias(lado_da_imagem: f32, canto_da_imagem: f32, destino: Rect, canto_do_destino: f32) -> [Fatia; 9]` (ordem: linhas de cima para baixo, colunas da esquerda para a direita); `pub fn cobrir(imagem_w: f32, imagem_h: f32, tela_w: f32, tela_h: f32) -> Rect` (o pedaço da imagem, em pixels dela, que cobre a tela, centrado); `pub fn expandir(area: Rect, margem: f32) -> Rect`.
- Regras: o canto do destino é limitado a `min(canto, w/2, h/2)` (nunca negativo); entrada inválida (0, NaN) devolve retângulos de tamanho 0, nunca `NaN`; `cobrir` com tela inválida devolve `Rect::VAZIO`.

- [ ] **Step 1: Testes.** `as_nove_fatias_cobrem_o_destino`: destino `{10,20,100,60}`, canto 12: a soma das áreas dos destinos é 6000 (±1e-3), todos dentro do destino, nenhum par se sobrepõe; os quatro cantos medem 12×12. `as_fatias_da_origem_cobrem_a_imagem`: imagem 64, canto 12: soma das áreas das origens é 4096. `canto_grande_demais_e_reduzido`: destino 10×10, canto 20 → cantos 5×5 e o miolo com largura 0. `destino_sem_tamanho_nao_da_nan`: destino 0×0 e destino com `NaN` não geram `NaN`. `cobrir_preserva_a_proporcao_da_tela`: imagem 940×1672: tela 1080×1080 → `w=940`, `h=940`, `y=366`; tela 2000×500 → `w=940`, `h=235`; tela 360×640 → proporção `w/h` igual à da tela (±1e-4); o retângulo fica sempre dentro da imagem; tela 0×0 → `Rect::VAZIO`. `expandir_aumenta_os_quatro_lados`: `expandir({10,10,20,20}, 5)` é `{5,5,30,30}`.
- [ ] **Step 2: Rodar `cargo test -p prisma_game formas`.** Expected: falha ao compilar (RED).
- [ ] **Step 3: Implementar** as três funções.
- [ ] **Step 4: Rodar de novo.** Expected: passam. **Mutação:** em `cobrir`, trocar `max` por `min` na escala e ver `cobrir_preserva_a_proporcao_da_tela` falhar; restaurar.
- [ ] **Step 5: Commit** `feat: nove fatias, corte da foto de fundo e margem`.

### Task 3: Ícones dos botões (`icones.rs`)

**Files:** Create `game/src/icones.rs`; Modify `game/src/main.rs` (`mod icones;`).

**Interfaces:**
- Produces: `pub type Linha = Vec<(f32, f32)>` (polilinha, pontos absolutos em pixels); `pub fn embaralhar(area: Rect) -> Vec<Linha>` (duas diagonais que se cruzam, cada uma com ponta de seta no fim); `pub fn faisca(area: Rect) -> Vec<Linha>` (uma linha só: contorno fechado de estrela de 4 pontas, 8 pontos mais o fecho = 9); `pub fn novo_jogo(area: Rect) -> Vec<Linha>` (arco de 300° em volta do centro, mais a ponta de seta no fim). Tudo cabe em `area` (o quadrado do ícone).

- [ ] **Step 1: Testes.** Para os três, com `area = {100,200,40,40}`: `todos_os_pontos_ficam_dentro_da_area` (±0,5 px); `nao_ha_nan_com_area_vazia` (`Rect::VAZIO` não gera `NaN`). `a_faisca_e_fechada_e_simetrica`: 9 pontos, primeiro == último, e refletindo cada ponto no centro da área sai outro ponto da lista (±1e-3). `o_arco_do_novo_jogo_e_redondo_e_aberto`: os pontos da primeira linha ficam todos a uma mesma distância do centro (±0,5) e o primeiro está a mais de 0,1×40 do último. `as_duas_diagonais_do_embaralhar_se_cruzam`: os segmentos principais das duas primeiras linhas se cruzam (conta de interseção no próprio teste).
- [ ] **Step 2: Rodar `cargo test -p prisma_game icones`.** Expected: RED (não compila).
- [ ] **Step 3: Implementar** as três funções (a ponta de seta é uma polilinha de 3 pontos de cada lado da direção final).
- [ ] **Step 4: Rodar.** Expected: passam. **Mutação:** em `faisca`, trocar o raio interno por um valor maior que o externo e ver `a_faisca_e_fechada_e_simetrica` ou o teste de limites falhar; restaurar.
- [ ] **Step 5: Commit** `feat: ícones de embaralhar, dica e novo jogo por polilinhas`.

### Task 4: Posições novas no layout (`grade`, `cartao_pontos`)

**Files:** Modify `game/src/layout.rs`, `game/src/app.rs` (linhas ~522 e ~527, e os testes em ~1303 e ~1970), `game/src/cenas.rs` (~23), `game/src/main.rs` (~213 e o teste em ~590).

**Interfaces:**
- Produces em `TelaJogo` (e em `VAZIA`): `pub grade: Rect` (o quadrado onde ficam as 64 casas: o `tabuleiro` encolhido `0,035 × tabuleiro.w` de cada lado) e `pub cartao_pontos: Rect` (a faixa inteira do painel: `x = esquerda`, `y = painel_y`, `w = lado`, `h = painel_h`); `pub fn fundo_da_grade(tabuleiro: Rect) -> Rect` (o `tabuleiro` encolhido `0,021 × tabuleiro.w`: o painel escuro dentro da moldura).
- `tabuleiro` **continua sendo a moldura inteira** (os 94% da largura): todos os testes de alinhamento e as posições do M3 seguem valendo. As colunas `pontos`, `nivel`, `melhor` passam a ficar dentro do cartão com folga lateral de `0,045 × lado`: `pontos` encostado à esquerda (largura `0,36` da área útil), `melhor` encostado à direita (`0,36`), `nivel` no meio (`0,28`).
- Quem precisa da **grade** (cálculo de casa e de toque) em vez do `tabuleiro`: `app.rs` (começo do gesto e `gestos.tratar`), `cenas.rs` (`centro` da casa), `main.rs` (a linha de geometria `PRISMA_M1 tabuleiro x= y= lado=` mantém o nome, mas passa a informar a grade: é com ela que o teste do celular virtual calcula o centro de cada casa).

- [ ] **Step 1: Testes de layout (novos, `layout.rs`) e de toque.** `a_grade_fica_dentro_da_moldura`: para todos os `CELULARES` e `TELAS`, nos dois modos: `grade` é quadrada, `dentro(grade, tabuleiro)`, a folga é `0,035 × tabuleiro.w` (±1e-2), `dentro(grade, fundo_da_grade(tabuleiro))`, `dentro(fundo_da_grade(tabuleiro), tabuleiro)`. `o_cartao_de_pontos_cobre_as_tres_colunas`: `pontos`, `nivel`, `melhor` dentro de `cartao_pontos`, em ordem da esquerda para a direita e sem se cruzar; `cartao_pontos` não cruza as abas nem a barra. `a_borda_da_moldura_nao_e_casa`: o ponto `(tabuleiro.x + 1, tabuleiro.y + 1)` está em `tabuleiro` e não em `grade`. `tela_impossivel_tem_grade_e_cartao_vazios`: estender o teste existente das telas impossíveis. Em `app.rs`, trocar o auxiliar de teste que acha o centro de uma casa para usar `tela.grade` **antes** de mudar o código.
- [ ] **Step 2: Rodar `cargo test -p prisma_game`.** Expected: os testes de layout não compilam (campos novos) e, com o auxiliar de `app.rs` já usando `grade` mas o código ainda usando `tabuleiro`, os testes de toque do `app.rs` falham = RED.
- [ ] **Step 3: Implementar** os campos, a função e as posições em `tela_jogo_com`; trocar `tela.tabuleiro` por `tela.grade` nos quatro pontos acima. Ajustar os testes antigos que dependiam da largura das colunas do painel (se houver; o compilador e a suíte apontam).
- [ ] **Step 4: Rodar `cargo test --workspace`.** Expected: tudo passa (inclusive o teste de `linhas_de_geometria`, atualizado para a grade). **Mutação:** em `fundo_da_grade`, trocar `0,021` por `0,2` e ver `a_grade_fica_dentro_da_moldura` falhar; restaurar.
- [ ] **Step 5: Commit** `feat: grade, cartão de pontos e fundo da grade no layout`.

### Task 5: Fundo em foto, moldura e casas (desenho)

**Files:** Modify `game/Cargo.toml` (`image = { version = "0.24", default-features = false, features = ["jpeg"] }`; o Macroquad só liga `png` e `tga`, e a mesma crate com `jpeg` ligado passa a abrir a foto), `game/assets/prisma-bg.jpg` (copiar de `/home/claude/thiagodluz/prisma/prisma-bg.jpg`), `game/assets/LICENSE-ART.md`, `game/src/assets.rs` (`pub const FUNDO_JPG: &[u8] = include_bytes!("../assets/prisma-bg.jpg");`), `game/src/render.rs`.

**Interfaces:**
- Consumes: Tasks 1, 2 e 4.
- Produces em `render.rs`: `struct Forma { textura: Texture2D, lado: f32, canto: f32 }`; `struct Formas { fundo: Texture2D, moldura: Forma, grade: Forma, casa_par: Forma, casa_impar: Forma, cartao: Forma, botao: Forma, solida: Forma }` guardado em `Sprites` (campo `formas`), construído dentro de `Sprites::load()` (assim `draw_jogo(app, sprites, tela)` não muda de assinatura); `fn desenhar_forma(forma: &Forma, area: Rect, raio: f32, sombra: f32)` (usa `nove_fatias` sobre `expandir(area, sombra)` com canto de destino `sombra + raio`); `fn desenhar_degrade(area: Rect, cores: [Color; 4])` (cantos: cima-esq, cima-dir, baixo-dir, baixo-esq, por `draw_mesh`).
- Receitas (hex `RRGGBBAA`, do CSS "illustrated" do JS), todas com `angulo` 145 salvo indicação:

| Forma | lado / raio / borda / sombra | degradê (3 cores) | borda | raio no destino |
| --- | --- | --- | --- | --- |
| moldura | 168 / 22 / 2 / 20 (cor da sombra `030616bd`) | `7790bd65`, `202a4eff`, `785b9b72` | `cee6ff5c` | `0,059 × tabuleiro.w`; sombra `0,05 × tabuleiro.w` |
| grade (painel escuro) | 64 / 14 / 0 / 0 | `0d142cff` nas três | – | `0,04 × tabuleiro.w` |
| casa_par | 64 / 12 / 0 / 0, ângulo 155 | `bfd5ff19`, `869ec912`, `4d67940b` | – | `0,20 × lado da casa` |
| casa_impar | igual | `c8c1ff1b`, `8a94c913`, `4d67940b` | – | igual |
| solida | 64 / 12 / 0 / 0 | `ffffffff` nas três | – | usada tingida (Task 8) |

  A casa de índice (linha×8+coluna) **par** usa `casa_impar` (o JS conta as casas a partir de 1, então a "ímpar" do CSS é a de índice par; como a linha tem 8 casas, isso dá listras verticais, como no JS).
- Fundo: `cobrir` + `draw_texture_ex` com `source`, e por cima o véu `desenhar_degrade` de `0b102242` (em cima) a `0b10226b` (embaixo). Casas: tamanho `0,90 ×` o passo da grade, centradas na casa (`cell_rect`). As pedras seguem desenhadas como hoje, só que sobre a `grade` (`desenhar_pedras`, realces, partículas), e a moldura e o painel escuro por baixo.

- [ ] **Step 1: Dependência e foto.** Adicionar `image` com `jpeg` ao `game/Cargo.toml`, copiar a foto, citar `prisma-bg.jpg` no `LICENSE-ART.md`; `cargo build -p prisma_game`. Expected: compila. Se a crate de JPEG não puder ser baixada aqui, converter a foto para PNG 540×960 e usar `include_bytes!` do PNG (ruling no ledger).
- [ ] **Step 2: Implementar** `Forma`, `Formas`, as duas funções de desenho e o uso em `draw_jogo` (ordem: fundo e véu → sombra e moldura → painel escuro → casas → o que já existia). Só `draw_jogo`; as telas de diagnóstico e as folhas não mudam.
- [ ] **Step 3: Conferir que nada é gerado por quadro:** `grep -n "from_rgba8\|from_file_with_format" game/src/render.rs` só pode mostrar linhas dentro de `Sprites::load`/`Formas::load`.
- [ ] **Step 4: Fotos.** `--cena inicio` em `360x640` e `1080x2400`; Read nos dois PNG. Conferir com `docs/notes/ref/js-novo.jpg`: foto de cristais atrás, moldura brilhante, casas escuras arredondadas com vão, pedras no meio das casas e sem sair delas. Se algo estiver errado, ajustar as receitas (só esta tabela), nunca o layout.
- [ ] **Step 5: Rodar `cargo test --workspace` e o clippy da linha de base. Commit** `feat: fundo em foto, moldura e casas do tabuleiro`.

### Task 6: Cartão de pontos e barra em degradê

**Files:** Modify `game/src/render.rs` (`desenhar_cabecalho`).

**Interfaces:** Consumes Tasks 4 e 5 (`cartao`, `solida`, `desenhar_degrade`).

- [ ] **Step 1: Implementar.** Cartão: `cartao` em `tela.cartao_pontos` (receita: degradê `ffffff0d` nas três, borda 1 `ffffff17`, raio no destino `0,045 × cartao.w`). Textos dentro dele: PONTOS alinhado à esquerda, NÍVEL ao centro, MELHOR à direita (`Alinhamento::Esquerda/Centro/Direita`); rótulos em `b3b0d4`, valores em branco, o valor do NÍVEL em `a2eddd`. Barra: trilho em `solida` tingida `ffffff17` com raio `barra.h/2`; preenchimento por `desenhar_degrade` de `6be2d6` (esquerda) a `d09cff` (direita), largura `barra.w × progresso`, extremidade direita reta. Rótulo da barra: alinhado à direita, cor `9996b9`. O ganho ("+N") continua à esquerda, em `DESTAQUE`. Sem espaçamento entre letras e sem negrito nesta etapa (negrito vem com a fonte, na V2).
- [ ] **Step 2: Fotos** `--cena inicio` e `--cena nivel` em `360x640` e `1080x2400`; comparar com o cartão e a barra de `js-novo.jpg`; os números grandes (`1_234_567`, cena `recordes`/`nivel`) não saem do cartão.
- [ ] **Step 3: Suíte e clippy. Commit** `feat: cartão de pontos e barra de nível em degradê`.

### Task 7: Botões arredondados com ícone

**Files:** Modify `game/src/render.rs` (`draw_jogo`, nova `desenhar_botao_de_acao`).

**Interfaces:** Consumes Tasks 3 e 5. Produces `enum Icone { Embaralhar, Dica, NovoJogo }` e `fn desenhar_botao_de_acao(formas: &Formas, area: Rect, rotulo: &str, icone: Icone, tamanho: f32, ativo: bool)`.

- [ ] **Step 1: Implementar.** Fundo `botao` (receita: ângulo 155, degradê `60538763`, `443e6988`, `292a4cad`; borda 1 `b8a7ea70`; raio no destino `0,21 × area.h`); ícone no alto (área quadrada de `0,38 × area.h`, centrada na horizontal) desenhado com `draw_line` (espessura `0,07 × area.h`, cor `eeeaff`) sobre as polilinhas de `icones.rs`; rótulo embaixo em `eeeaff`. Inativo: tudo a 40% de opacidade. Usar nos três botões de baixo (Embaralhar só no Zen, como hoje); abas, ♪, ≡ e o diálogo ficam como estão.
- [ ] **Step 2: Fotos** `--cena inicio` (3 botões), `--cena classico` (2 botões centrados e iguais) e `--cena fim` (Dica apagada) em `360x640` e `1080x2400`. Conferir que o rótulo "Embaralhar" cabe no botão em 360×640; se não couber, diminuir só o `tamanho` do rótulo desses botões.
- [ ] **Step 3: Suíte e clippy. Commit** `feat: botões de embaralhar, dica e novo jogo arredondados com ícone`.

### Task 8: Realces, avisos e folhas sobre o visual novo

**Files:** Modify `game/src/render.rs` (`desenhar_fundos_de_realce`, `desenhar_contornos_de_realce`, ajustes pontuais), `docs/notes/visual.md` (novo), `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md` (uma linha na lista de marcos).

**Interfaces:** Consumes Task 5 (`solida`).

- [ ] **Step 1: Realces dentro da casa.** `desenhar_fundos_de_realce` desenha cada realce (selecionada `f7edab5c`, dica `ffe9933d`, apertada `e5d8ff3d`, alvo do arrasto `c4ffe451`, as mesmas cores de hoje) com `solida` tingida, no retângulo da casa encolhido para o tamanho da casa (`0,90`), e os contornos (`desenhar_contornos_de_realce`) no mesmo retângulo arredondado ou, se ficar feio, retos mas dentro dele.
- [ ] **Step 2: Fotos de tudo que fica por cima**, em `360x640` e `1080x2400`: `selecionada`, `dica`, `troca`, `limpando`, `nivel`, `fim`, `dialogo`, `som`, `recordes`, `creditos`, `particulas-pulso`, `respiracao`. Corrigir só o que estiver cortado, ilegível ou fora da casa (cada correção, uma linha no ledger); a foto de fundo não pode tornar nenhum texto difícil de ler.
- [ ] **Step 3: Notas.** `docs/notes/visual.md`: receitas de formas (a tabela da Task 5), como gerar fotos, o que só o celular confirma (suavidade, foto de fundo decodificando no Android, tempo de abertura); uma linha na lista de marcos do desenho da fase 2 apontando para o desenho da fase 3.
- [ ] **Step 4: Suíte completa, clippy, mutação final** (trocar uma cor de `Receita` em `moldura` e ver que a foto muda; restaurar). **Commit** `feat: realces, avisos e folhas sobre o visual novo` e `docs: notas do visual da V1`.

## Fim da V1

Depois da Task 8: revisão da branch inteira (guia `executing-plans`), e então, **com autorização nova do Thiago**, enviar a branch e rodar o fluxo do Android (as fotos que o celular virtual já tira da tela principal, `1-zen-inicio`, `2-zen-dica`, `4-zen-fim` e `5-classico`, passam a mostrar a tela nova nos Androids 34 e 30, e é lá que se prova a decodificação da foto de fundo em JPEG; nenhuma mudança em `tools/` é necessária). O plano da V2 (topo, cartões de modo e a fonte Roboto) é escrito depois de ele testar o APK da V1, com os números reais.
