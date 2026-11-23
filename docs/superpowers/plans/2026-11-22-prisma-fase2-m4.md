# Prisma Fase 2 · M4 (Acabamento) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O jogo ganha vibração opcional, aviso "Carregando…", botão ≡ com Recordes e Créditos, as partículas do Prisma em JS (com a intensidade Suave/Padrão/Vibrante no Zen), ícone adaptativo e versão do Android correta, tudo num APK só.

**Architecture:** Peças puras e testáveis (`vibracao`, `menu`, `particulas`, e os dados/tempos da limpeza em `linha`) e camadas finas que só executam (`plataforma` para a vibração, `render` para o desenho, o laço de `main.rs`). O `App` guarda o estado novo (folha aberta, vibrador, "carregando", intensidade) e entrega pedidos (`vibracoes()`) como já faz com `sons()`. O motor `prisma_engine` não muda.

**Tech Stack:** Rust 2021, Macroquad 0.4.16, `prisma_engine` (intocado), `serde_json` (já no jogo), `image` (só `tools/`).

**Spec:** `docs/superpowers/specs/2026-11-22-prisma-fase2-m4-design.md` (design aprovado pelo Thiago); a spec principal é `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md`. Referência de comportamento: o Prisma em JS (`/home/claude/thiagodluz/prisma`, só leitura): `app.js` linhas 370–375 e 595 (vibração), 181–182 e 414–545 (efeitos), `style.css` 75–78 e 135–150, `index.html` 100–135.

## Global Constraints

- `edition = "2021"`; o motor não muda. Hoje: `cargo test --workspace` = 341 passam (jogo 262); `tools/` = 106 passam + 3 ignorados. Tudo continua passando. Nenhuma dependência nova. Nenhuma permissão nova (a `VIBRATE` já existe).
- Vibração (JS): padrões em ms, no formato do Android (começa por espera): comum `[0,28]`; com especial `[0,45]`; subiu de nível `[0,28,55,35]`; ao ligar `[0,55]`. Só em jogada **válida**; nível vence especial, que vence comum. Padrão **desligada**.
- Partículas e tempos (JS, em escala 1): faíscas 380 ms; onda 360; feixes 330; véu do Espectro 370; Supernova 480; fusão 240 (antes da explosão). Faíscas: 5 numa limpeza sem especial; 8 por especial; 12 no Espectro; alcance em casas 1,25 / Pulso 1,25 / Raio 1,65 / Espectro 2,2; ângulo do i-ésimo `2π·i/n + 0,18`. No máximo 3 especiais (fora Supernovas) com efeito por limpeza.
- Intensidade (só Zen; Clássico = Padrão): escala de duração Suave 0,82 / Padrão 1 / Vibrante 1,1; faíscas no Suave 5 por especial e 3 por limpeza comum; pico de tamanho da pedra 1,1 / 1,1 / 1,18; brilho da pedra 1,35 / 1,8 / 2,1. Só visual: não muda som, vibração nem pontos.
- Arquivo `som.json` (nomes do JS): `"vibration": true|false` na raiz; `"zen": {"effects": "soft"|"normal"|"vivid"}`. Arquivos do M3 (sem os campos novos) continuam carregando; valor estranho vira o padrão (vibração só liga com `true` de verdade).
- Folhas: só uma aberta por vez (Som ou Menu). Toda tela nova cabe em 360×640 e 1080×2400, controles com altura ≥ 0,07 do menor lado, sem se cruzar; o tabuleiro não muda de tamanho nem de lugar.
- Créditos (texto fixo, sem links) citam: GPLv3, CC BY-SA 4.0, CC BY 4.0, "Geoff Bremner", Thiago Luz (2026), CC0 com `docs/CREDITOS-AUDIO.md`.
- Android: `minSdk 26`; `targetSdk` o mais alto que o gerador aceitar até **34** (não 35: o CI só tem Android 30 e 34); tamanho do APK ≤ 60 MB.
- Registro (logcat): `PRISMA_M1/M2/M3` continuam; as novas começam com `PRISMA_M4` (formatos nas Tasks 1, 3, 10). Botões novos usam o formato que já existe (`PRISMA_M1 botao=NOME x=.. y=..`).
- Neste ambiente: `export RUSTFLAGS="-L /tmp/asound" LIBRARY_PATH=/tmp/asound LIBGL_ALWAYS_SOFTWARE=1` (e `mkdir -p /tmp/asound && ln -sf /usr/lib/x86_64-linux-gnu/libasound.so.2 /tmp/asound/libasound.so` se faltar). `cargo test` aceita **um** filtro por vez. Foto: `xvfb-run -a -s "-screen 0 1280x1024x24" cargo run -q -p prisma_game -- --cena NOME --screenshot arquivo.png [--tamanho LxA]`; conferir com a ferramenta Read. `game/` e `tools/` não passam no `rustfmt`: não reformatar. Clippy do jogo não pode ganhar aviso (os que existem: Cancelar nunca construído, `pular_para_o_fim`, 2× "large size difference", 4× loop por índice, `partial_cmp`, `div_ceil`, `Game Default`); conferir com `cargo clippy -p prisma_game --all-targets 2>&1 | grep -E "^(warning|error)" | sed 's/`[^`]*`/X/g' | sort | uniq -c`.
- **Segredos:** nada novo. Merge na `main`, envio de branch e execução do CI só com autorização nova do Thiago (autorização anterior não vale).
- Textos e comentários em português simples (Thiago tem conhecimento básico de programação): explicar cada conceito novo de Rust numa linha. Commits em português (`feat:`, `test:`, `ci:`, `docs:`) com os trailers de atribuição.

## Review Focus

1. **Aparelho sem vibrador ou que recusa:** o interruptor some (sem vibrador); se a vibração ao ligar falha, o interruptor volta a desligado; erro de JNI nunca derruba o app; jogada inválida não vibra; vibração desligada não pede nada. Tasks 1 e 2.
2. **Partículas em cascata, muitas especiais, tempo estranho** (quadro de vários segundos ao voltar do segundo plano, `NaN`, negativo, infinito): nada de pânico nem `NaN` no desenho, número de objetos limitado, nada "preso" na tela depois do fim do passo, e a limpeza comum não vira lenta demais. Tasks 7, 8, 9.
3. **Folhas no meio de tudo** (animação, fim de partida, pergunta "Novo jogo?", dedo arrastando uma barra): abrir o Menu com o Som aberto (e o contrário) fecha a outra sem perder o último volume; o tabuleiro e os botões de baixo não reagem; telas pequena e alta. Task 5 (e 2, 10 para as linhas novas do Som).
4. **"Carregando…":** faixa já carregada não mostra o aviso; o jogador desliga antes de a carga começar; erro de carga tira o aviso; música e ambiente pedidos juntos. Task 3.
5. **Ícone e versão do APK:** a figura não é cortada por máscara redonda; o CI reprova APK com `minSdk`/`targetSdk` ou ícone errados; se o gerador de APK não aceitar as versões pedidas, o pedaço se desfaz sozinho. Tasks 11 e 12.

## Escolhas minhas (a confirmar na revisão do plano)

- **Um só CI, no fim** (Task 14, com autorização): erro que só o Android vê (JNI da vibração, ícone, versões) aparece tarde. Custo: uma rodada de conserto no fim. Alternativa: pedir uma rodada de CI depois da Task 2.
- A vibração ao ligar é pedida pelo `App` (`Toque::Ligou`) e o laço de `main.rs` a executa; se falhar, o laço chama `app.vibracao_recusada()`.
- O passo de limpeza passa a durar a mais longa das animações (como o `waitAnimations` do JS): uma limpeza comum fica em 380 ms (hoje 270 ms). Isso muda o ritmo das cascatas e dos sons; é o comportamento do JS.
- A pedra "brilha" com um véu branco translúcido por cima dela (o Macroquad não tem filtro de brilho): `alfa = 0,3 × (brilho − 1)`; ajustado a olho nas fotos.
- No Clássico a intensidade vale Padrão mesmo se o arquivo guardar outra.
- Ícone: a figura é isolada do `icon-512.png` pela diferença para o fundo (o `image` não lê SVG); o fundo é a cor média dos cantos.

## Mapa de arquivos

- Create: `game/src/vibracao.rs` (T1), `game/src/menu.rs` (T4), `game/src/particulas.rs` (T8), `docs/notes/acabamento.md` (T14).
- Modify: `game/src/preferencias.rs` (T1, T6), `game/src/app.rs` (T1, T2, T3, T5, T10), `game/src/plataforma/{mod,pc,android}.rs` (T2), `game/src/layout.rs` (T2, T5, T10), `game/src/render.rs` (T2, T3, T5, T9, T10), `game/src/fundo.rs` (T3), `game/src/linha.rs` (T7), `game/src/cenas.rs` (T2, T5, T9), `game/src/main.rs` (T1, T2, T3, T5), `tools/src/imagens.rs` (T11), `tools/src/registro.rs`, `tools/src/emulador.rs`, `tools/tests/amostra-logcat.txt` (T13), `game/Cargo.toml`, `tests/workspace.rs`, `.github/workflows/android.yml` (T12), `docs/notes/android-build.md`, `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md` (T14).

---

### Task 1: Vibração: padrões, preferência e lógica no App (puro)

**Files:**
- Create: `game/src/vibracao.rs`
- Modify: `game/src/preferencias.rs`, `game/src/app.rs`, `game/src/main.rs` (`mod vibracao;`)

**Interfaces:**
- Produces: `pub enum Toque { Curto, ComEspecial, NivelAcima, Ligou }` com `pub fn padrao_ms(self) -> &'static [u32]` (tabela das Global Constraints) e `pub fn nome(self) -> &'static str` (`"curto"`, `"especial"`, `"nivel"`, `"ligou"`); `pub fn toque_da_jogada(valida: bool, ativou_especial: bool, niveis_ganhos: u32) -> Option<Toque>`; `Preferencias.vibracao: bool` (padrão `false`); em `App`: `Botao::Vibracao` (da folha de Som, nos dois modos), `pub fn com_vibrador(self, bool) -> App`, `pub fn tem_vibrador(&self) -> bool` (padrão `false`), `pub fn vibracoes(&mut self) -> Vec<Toque>` (esvazia a fila, como `sons()`), `pub fn vibracao_recusada(&mut self)`.
- Consumes: `MoveResult.valid`, `.levels_gained`, `Event::Clear { activated, .. }` do motor; `Preferencias` e `gravar_preferencias` do M3.

Comportamento: em `jogar`, depois de `play`, se `preferencias.vibracao` e `toque_da_jogada(..)` der `Some(t)`: põe `t` na fila e registra `PRISMA_M4 vibracao toque=<nome>`. `apertar(Botao::Vibracao)` só vale com a folha de Som aberta e `tem_vibrador()`: inverte a preferência, grava; ao ligar põe `Toque::Ligou` na fila (e registra). `vibracao_recusada()` desliga, grava e registra `PRISMA_M4 vibracao recusada`. JSON: `preferencias_para_json` escreve `"vibration"`; `json_para_preferencias` só aceita `true`/`false` de verdade.

- [ ] **Step 1: Testes primeiro.** `vibracao.rs`: `padroes_sao_os_do_js` (as quatro listas), `nomes_do_registro`, `jogada_invalida_nao_vibra` (`toque_da_jogada(false, true, 1) == None`), `nivel_vence_especial_que_vence_comum` (`(true,true,2)→NivelAcima`, `(true,true,0)→ComEspecial`, `(true,false,0)→Curto`). `preferencias.rs`: `vibracao_comeca_desligada`, `vibracao_vai_e_volta_no_json`, `json_do_m3_sem_vibration_carrega_desligada`, `vibration_so_vale_com_booleano_de_verdade` (`"on"`, `1`, `null` → `false`). `app.rs` (reusando as sementes e a ajuda `sons_da_jogada_numero`/`injetar` dos testes `especial_ativado_toca_*` e `subir_de_nivel_aparece_no_estagio_no_passo_certo`): `jogada_comum_com_vibracao_ligada_pede_toque_curto`, `jogada_com_especial_pede_toque_de_45`, `subir_de_nivel_pede_o_padrao_longo`, `jogada_invalida_nao_pede_toque`, `vibracao_desligada_nao_pede_nada_e_o_jogo_segue`, `apertar_vibracao_liga_grava_e_pede_toque_ligou`, `apertar_vibracao_sem_vibrador_ou_com_folha_fechada_nao_faz_nada`, `vibracao_recusada_desliga_e_grava`, `registro_tem_as_linhas_de_vibracao` (formato exato `PRISMA_M4 vibracao toque=curto`).
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game vibracao` → Expected: não compila / FAIL (repetir com o filtro `preferencias` e `app` depois de criar o módulo).
- [ ] **Step 3: Implementar** as assinaturas acima. `toque_da_jogada` é um `match` simples; a fila é um `Vec<Toque>` no `App`, esvaziado por `std::mem::take`.
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: tudo verde (341 + novos); clippy sem aviso novo.
- [ ] **Step 5: Commit** `feat(game): padrões de vibração, preferência e pedido de toque no App`.

### Task 2: Vibração: plataforma, interruptor na folha de Som e ligação no laço

**Files:**
- Modify: `game/src/plataforma/{mod,pc,android}.rs`, `game/src/layout.rs`, `game/src/render.rs`, `game/src/app.rs` (`botoes_visiveis`), `game/src/main.rs`, `game/src/cenas.rs`

**Interfaces:**
- Consumes: Task 1 (`Toque`, `Botao::Vibracao`, `com_vibrador`, `tem_vibrador`, `vibracoes`, `vibracao_recusada`).
- Produces: `plataforma::tem_vibrador() -> bool` e `plataforma::vibrar_padrao(padrao_ms: &[u32]) -> Result<(), String>` (a antiga `vibrar(u32)` do M0b continua); `layout::tela_jogo_com(largura: f32, altura: f32, zen: bool, vibrador: bool) -> TelaJogo` (`tela_jogo(l, a, zen)` vira atalho com `vibrador = true`); `TelaSom.vibracao: Option<Rect>` (`None` sem vibrador) — uma linha de interruptor logo depois do volume dos efeitos, nos dois modos; geometria `liga_vibracao` no registro com a folha aberta.

PC: `tem_vibrador()` é `true` e `vibrar_padrao` dá `Ok(())` (de mentira, como `vibrar`). Android (JNI, como `vibrar`): `tem_vibrador` pergunta `hasVibrator` e dá `false` em qualquer erro; `vibrar_padrao` cria um `long[]` (`NewLongArray` + `SetLongArrayRegion`) e chama `vibrate([JI)V` com repetição `-1`, limpando exceções; erro vira `Err`. Esse código só compila no CI (não há alvo Android aqui): ledger. Laço de `main.rs`: `App::abrir(..).com_vibrador(plataforma::tem_vibrador())`; `calcular_tela` usa `tela_jogo_com(.., app.tem_vibrador())`; a cada quadro, para cada `toque` de `app.vibracoes()`: `vibrar_padrao(toque.padrao_ms())`; se falhar, registra `PRISMA_M4 vibracao erro texto=...` e, se era `Ligou`, chama `app.vibracao_recusada()`.

- [ ] **Step 1: Testes primeiro.** `layout.rs`: `folha_com_vibracao_cabe_e_nao_se_sobrepoe` (estende `controles()` com `vibracao`, em `CELULARES` e `TELAS`, dois modos), `sem_vibrador_a_linha_some_e_o_resto_continua_cabendo`, `o_tabuleiro_nao_muda_com_a_linha_nova`. `app.rs`: `botao_vibracao_so_existe_com_folha_aberta_e_vibrador` (via `botao_em`). `main.rs`: `o_registro_de_geometria_leva_liga_vibracao_com_a_folha_aberta`. `pc.rs`: `tem_vibrador_e_vibrar_padrao_no_pc`. Cena `som` passa a usar `.com_vibrador(true)`.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game layout` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar** (layout: a folha do Zen passa de 8 para 9 linhas e a do Clássico de 5 para 6, só com vibrador; `desenhar_linha_com_interruptor(area, "Vibração", ligado, t)` no `render.rs`). Foto: `--cena som` em 540×960 e `--tamanho 360x640`; conferir com Read que a linha nova não corta nada.
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: verde; clippy sem aviso novo; `cargo build -p prisma_game` compila.
- [ ] **Step 5: Commit** `feat(game): interruptor de vibração, vibrar com padrão e ligação no laço`.

### Task 3: Aviso "Carregando…"

**Files:**
- Modify: `game/src/fundo.rs`, `game/src/app.rs`, `game/src/render.rs`, `game/src/main.rs`

**Interfaces:**
- Produces: em `fundo.rs`: `pub enum Decisao { Nada, Aplicar, Avisar(Canal) }` e `pub fn decidir(comandos: &[Comando], ja_avisado: Option<Canal>) -> Decisao` (`Nada` se não há comandos; `Avisar(c)` se há `Comando::Carregar` do canal `c` e `ja_avisado != Some(c)` — vale o primeiro `Carregar` da lista; senão `Aplicar`). Em `App`: `pub fn carregando(&self) -> Option<Canal>`, `pub fn marcar_carregando(&mut self, canal: Canal)` (registra `PRISMA_M4 carregando tipo=musica|ambiente`), `pub fn fim_do_carregamento(&mut self)`. `aviso_da_folha()` devolve primeiro `"Carregando música…"` / `"Carregando ambiente…"` quando `carregando()` é `Some`, antes dos outros avisos.
- Consumes: `fundo::planejar`, `Comando`, `Canal` (M3).

Laço (`main.rs`): `let comandos = fundo::planejar(..)`; `match fundo::decidir(&comandos, app.carregando())`: `Avisar(c)` → `app.marcar_carregando(c)` e **não aplica neste quadro** (o quadro é desenhado com o aviso); `Aplicar` → `audio.aplicar(..).await` e depois `app.fim_do_carregamento()`; `Nada` → `app.fim_do_carregamento()`.

- [ ] **Step 1: Testes primeiro.** `fundo.rs`: `decidir_sem_comandos_e_nada`, `decidir_com_carregar_novo_manda_avisar`, `decidir_depois_do_aviso_manda_aplicar`, `decidir_so_com_volume_ou_parar_aplica_sem_avisar`, `decidir_com_dois_carregar_avisa_o_primeiro_canal`. `app.rs`: `marcar_carregando_mostra_o_aviso_na_folha_e_registra_a_linha`, `fim_do_carregamento_tira_o_aviso`, `o_aviso_de_carregando_vem_antes_do_aviso_de_audio`, `faixa_ja_carregada_nao_marca_nada` (o planejador, com `atual` igual ao `desejado`, dá `Nada`/`Aplicar` sem `Avisar`).
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game fundo` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar.** `render.rs` já mostra `aviso_da_folha()` no lugar do título: nada novo a desenhar.
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: verde; no PC, `PRISMA_SONS` apontando para uma pasta com um OGG: abrir o jogo, ligar a música e ver no registro a linha `carregando` antes de `carregado`.
- [ ] **Step 5: Commit** `feat(game): aviso "Carregando…" antes de carregar música e ambiente`.

### Task 4: Conteúdo de Recordes e Créditos (puro)

**Files:**
- Create: `game/src/menu.rs`
- Modify: `game/src/main.rs` (`mod menu;`)

**Interfaces:**
- Produces: `pub struct LinhaDeRecorde { pub rotulo: &'static str, pub valor: String }`; `pub fn linhas_de_recordes(modo: Mode, recordes: Recordes) -> Vec<LinhaDeRecorde>` (Zen: `Maior pontuação`, `Maior nível`, `Melhor jogada`; Clássico: esses mais `Partidas concluídas`; valores com `formatar_numero`); `pub fn creditos(versao: &str) -> Vec<String>` (linhas curtas, texto do design 6.3, com `Prisma (Rust) · versão {versao}` na primeira).
- Consumes: `app::formatar_numero(u64)`, `formato::Recordes { pontuacao, nivel, jogada, terminadas }`.

- [ ] **Step 1: Testes primeiro.** `zen_tem_tres_linhas_e_classico_quatro`; `valores_usam_ponto_de_milhar` (`pontuacao: 1234567` → `"1.234.567"`); `recordes_zerados_mostram_0_e_nivel_1`; `creditos_citam_o_que_a_licenca_exige` (contém `GPLv3`, `CC BY-SA 4.0`, `CC BY 4.0`, `Geoff Bremner`, `Thiago Luz`, `CC0`, `CREDITOS-AUDIO.md`, e a versão passada); `creditos_nao_tem_link` (nenhuma linha contém `http`); `creditos_sao_poucas_linhas` (≤ 10 linhas, cada uma ≤ 120 caracteres).
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game menu` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar** (texto num lugar só, em constantes).
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: verde.
- [ ] **Step 5: Commit** `feat(game): textos de Recordes e Créditos`.

### Task 5: Botão ≡ e a folha do Menu (Recordes e Créditos)

**Files:**
- Modify: `game/src/layout.rs`, `game/src/app.rs`, `game/src/render.rs`, `game/src/main.rs`, `game/src/cenas.rs`

**Interfaces:**
- Consumes: Task 4 (`linhas_de_recordes`, `creditos`), `App::recordes(modo)` (M2), `env!("CARGO_PKG_VERSION")`.
- Produces: `TelaJogo.botao_menu: Rect` (quadrado, à direita do ♪, mesmo tamanho; as abas ficam mais estreitas: `aba_w = (lado − 2·abas_h − 3·espaco)/2`) e `TelaJogo.menu: TelaMenu { folha, aba_recordes, aba_creditos, fechar, conteudo: Rect, texto: f32 }` (a mesma `folha` da de Som; `TelaMenu::VAZIA`); em `app.rs`: `pub enum Folha { Som, Recordes, Creditos }`, o campo `folha: bool` vira `folha: Option<Folha>`, `pub fn folha(&self) -> Option<Folha>` (`folha_aberta()` continua, vale `folha().is_some()`), `Botao::Menu` (o ≡), `Botao::AbaRecordes`, `Botao::AbaCreditos`, `Botao::FecharMenu`.

Comportamento: ≡ abre o Menu em Recordes (fechado), ou o fecha (aberto); com a folha de Som aberta o ≡ está visível e abre o Menu fechando o Som (o ♪ faz o contrário com o Menu aberto); trocar de folha termina um arrasto de volume (grava o último valor). Com qualquer folha aberta só reagem ♪, ≡ e os controles dela; abas Zen/Clássico, tabuleiro e botões de baixo ficam "tampados" (como no M3). `AbaRecordes`/`AbaCreditos` só valem com o Menu aberto. `Botao::da_folha()` passa a dizer **a qual folha** o botão pertence (`Option<Folha>`; `Menu` e `Som` não pertencem a nenhuma), e `apertar` só aceita o botão cuja folha é a aberta. Geometria no registro: `menu` sempre; com o Menu aberto `aba_recordes`, `aba_creditos`, `fechar_menu`; `linhas_de_geometria` e a `situacao` do laço passam a usar `app.folha()` no lugar do `bool`. Render: ícone ≡ com três linhas (formas, como o ♪); Recordes = dois blocos (Zen, Clássico) com rótulo e valor; Créditos = as linhas de `creditos(..)` com `escrever_em_linhas`. Cenas `recordes` (com números não zerados) e `creditos`.

- [ ] **Step 1: Testes primeiro.** `layout.rs`: `botao_menu_fica_ao_lado_do_som_sem_cobrir_as_abas` (em `TELAS`, dois modos: quadrado, na linha das abas, à direita do ♪, sem cruzar nada, canto direito no do tabuleiro), `tela_menu_cabe_e_nao_se_sobrepoe` (`CELULARES`, altura dos botões ≥ 0,07 do menor lado), `o_tabuleiro_nao_muda_com_o_botao_menu`. `app.rs`: `menu_abre_em_recordes_e_o_x_fecha`, `abrir_o_menu_com_o_som_aberto_troca_de_folha_e_grava_o_volume_do_arrasto`, `abrir_o_som_com_o_menu_aberto_troca_de_folha`, `com_o_menu_aberto_o_tabuleiro_e_os_botoes_de_baixo_nao_reagem`, `menu_no_meio_da_animacao_nao_quebra_a_animacao_nem_o_fim_de_partida`, `menu_com_a_pergunta_novo_jogo_aberta_nao_abre` (a pergunta manda), `abas_do_menu_so_valem_com_o_menu_aberto`, e o antigo `botao_em_acha_so_o_botao_visivel` atualizado. `main.rs`: `o_registro_de_geometria_leva_menu_e_os_botoes_do_menu_so_com_ele_aberto`. Fotos: `recordes`, `creditos` em 360×640 e 540×960; conferir com Read que o texto cabe e não corta.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game layout` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar.** Trocar `folha: bool` por `Option<Folha>` mantendo o comportamento do M3 do Som (todos os testes do M3 passam).
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: verde; clippy sem aviso novo.
- [ ] **Step 5: Commit** `feat(game): botão ≡ com a folha de Recordes e Créditos`.

### Task 6: Intensidade dos efeitos: tipo e arquivo (puro)

**Files:**
- Modify: `game/src/preferencias.rs`

**Interfaces:**
- Produces: `pub enum Intensidade { Suave, Padrao, Vibrante }` com `Default = Padrao`, `pub const TODAS: [Intensidade; 3]`, `pub fn proximo(self) -> Intensidade`, `pub fn texto(self) -> &'static str` (`"Suave"`, `"Padrão"`, `"Vibrante"`), `pub fn id_js(self) -> &'static str` (`"soft"`, `"normal"`, `"vivid"`), `pub fn nome_do_registro(self) -> &'static str` (`"suave"`, `"padrao"`, `"vibrante"`); `Preferencias.intensidade: Intensidade`; JSON `"zen": {"effects": ..}`.

- [ ] **Step 1: Testes primeiro.** `intensidade_padrao_e_padrao`; `proximo_da_a_volta`; `nomes_e_rotulos`; `ida_e_volta_do_json_com_effects`; `effects_desconhecido_vira_padrao` (`"neon"`, `1`, `null`, ausente); `json_do_m3_sem_effects_carrega_com_padrao`; o teste `o_json_gravado_usa_os_nomes_do_js` do M3 passa a conferir `effects` também.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game preferencias` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar** (mesmo molde de `Respiracao`: `pelo_id_js` privado).
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: verde.
- [ ] **Step 5: Commit** `feat(game): tipo Intensidade e o campo effects do som.json`.

### Task 7: Dados e tempos da limpeza (puro)

**Files:**
- Modify: `game/src/linha.rs`, `game/src/app.rs` (chamada de `montar` e testes que montam `Passo::Limpeza`), `game/src/render.rs` (só para compilar com os campos novos)

**Interfaces:**
- Consumes: `Intensidade` (Task 6); `Activation::{Special{index,kind,..}, Supernova(Supernova{index,color,sources})}` do motor.
- Produces: `pub enum TipoVisual { Pulso, Raio, Espectro, Supernova }`; `pub struct Visual { pub casa: usize, pub tipo: TipoVisual, pub cor: u8, pub origens: Vec<usize> }` (`cor` e `origens` só valem na Supernova; no resto `0` e vazio); `Passo::Limpeza` ganha `visuais: Vec<Visual>` (todas as ativações, na ordem do motor) e `intensidade: Intensidade`; `pub fn montar(antes, a, b, resultado, nivel_final, intensidade: Intensidade) -> Vec<Passo>`; `pub fn fusao_ms(passo: &Passo) -> f32` (`FUSAO_MS × escala` se há Supernova com origens, senão `0`); `pub fn fusao_da_casa(t_ms: f32, intensidade: Intensidade) -> (f32, f32)` (escala 1 → 1,16 → 0,9 e brilho, nos `FUSAO_MS × escala`); `pub fn limpeza_da_casa(ativadas: &[usize], casa: usize, t_ms: f32, intensidade: Intensidade) -> (f32, f32, f32)` = (escala, opacidade, brilho 0..1), com `t_ms` contado **depois** da fusão. Constantes `FAISCA_MS=380`, `ONDA_MS=360`, `FEIXE_MS=330`, `VEU_MS=370`, `SUPERNOVA_MS=480`, `FUSAO_MS=240`. `duracao_ms(Limpeza)` = `fusao_ms` + `max(pedras, efeitos)` onde `pedras = (270|340) × escala + atraso` (o atraso por casa não escala) e `efeitos` = a mais longa das animações presentes (faíscas sempre; Supernova 480) × escala.

Escala de duração e picos (Global Constraints): `Intensidade::escala_de_duracao()` (0,82/1/1,1), `pico_da_pedra()` (1,1/1,1/1,18) e `brilho_da_pedra()` (1,35/1,8/2,1) como funções privadas de `linha.rs`.

- [ ] **Step 1: Testes primeiro.** `montar_guarda_os_visuais_e_a_intensidade` (jogada com Pulso → `TipoVisual::Pulso`; com Supernova → `cor` e `origens` do motor); `limpeza_sem_especial_tem_visuais_vazio`; `limpeza_comum_dura_380_ms_no_padrao` (substitui o antigo teste de 270 ms); `limpeza_com_especial_dura_o_maior_entre_pedras_e_efeitos`; `supernova_ganha_fusao_antes_da_explosao` (Padrão: `fusao_ms == 240`, duração total `240 + 480`); `escala_suave_e_vibrante_multiplicam_a_duracao` (`380×0,82` e `380×1,1`, tolerância 0,01); `limpeza_da_casa_pico_1_1_no_padrao_e_1_18_no_vibrante`; `limpeza_da_casa_brilho_sobe_e_volta`; `fusao_da_casa_chega_a_1_16`; `tempo_estranho_nao_gera_nan` (NaN, negativo, infinito → números finitos).
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game linha` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar.** Atualizar os testes antigos que montam `Passo::Limpeza` à mão (campos novos: `visuais: vec![]`, `intensidade: Intensidade::Padrao`), os que chamam `montar` (argumento novo) e os que fixavam durações; o `App` passa `Intensidade::Padrao` por enquanto (a Task 10 liga a preferência). `render.rs` só chama `limpeza_da_casa` com os argumentos novos, ignorando o brilho.
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: verde; os testes de som do M3 (`sons_*`) continuam verdes (a ordem dos sons não muda, só o tempo).
- [ ] **Step 5: Commit** `feat(game): dados dos efeitos e tempos da limpeza do JS no Passo::Limpeza`.

### Task 8: Partículas (puro)

**Files:**
- Create: `game/src/particulas.rs`
- Modify: `game/src/main.rs` (`mod particulas;`)

**Interfaces:**
- Consumes: `Visual`, `TipoVisual`, constantes de tempo e `Intensidade` (Tasks 6–7).
- Produces: `pub enum Forma { Faisca, Anel, Linha, Veu }`; `pub struct Particula { pub forma: Forma, pub x: f32, pub y: f32, pub tamanho: f32, pub espessura: f32, pub angulo: f32, pub alfa: f32, pub cor: [f32; 3] }` — tudo em **casas** a partir do canto de cima à esquerda do tabuleiro (o tabuleiro vai de 0 a 8; o desenho multiplica pelo lado da casa); `pub const MAXIMO_DE_PARTICULAS: usize = 96`; `pub fn particulas(visuais: &[Visual], celulas: &[usize], t_ms: f32, intensidade: Intensidade) -> Vec<Particula>` com `t_ms` contado do começo do passo (já com a fusão); `pub fn ajustar(cor: [f32; 3], alfa: f32, intensidade: Intensidade) -> ([f32; 3], f32)` (Suave: `alfa × 0,48` e cor menos viva; Padrão: igual; Vibrante: mais viva e mais clara).

Efeitos (valores do JS, tempos × escala, começam depois da fusão): **faíscas** — `n` por especial e alcance como nas Global Constraints; sem especial, uma nuvem de 5 (3 no Suave) no centro da célula do meio de `celulas`; trajetória: posição `origem + alcance·(cos, sin)·curva(t)` com `curva` de saída suave, escala 0,35 → 1,15 (a 25%) → 0,35 e opacidade 0 → 1 → 0; cor dourada `#fff6d1`, azul-gelo `#b5faff` (Raio), Espectro `hsl(i·360/n, 100%, 77%)`. **Onda** (Pulso e Raio): anel de 1,15 casa, escala 0,3 → 1,2 (35%) → 3 (Pulso) ou 2 (Raio), opacidade 0 → 0,9 → 0. **Feixes** (Raio): uma linha horizontal e uma vertical por todo o tabuleiro, espessura 8 px ≈ 0,2 casa, crescem de 0 a 1 (45%) e somem. **Véu** (Espectro): o tabuleiro todo, opacidade 0 → 0,9 (35%) → 0. **Supernova:** anel de 5 casas na cor `NOVA_CORES[cor]` (`#ff5774 #ffad48 #ffe36b #69ef98 #6deeff #6d9fff #d795ff`), escala 0,08 → 0,35 (18%) → 1 (65%), opacidade 0 → 0,9 → 0,7 → 0. **Fusão** (só na primeira fase): uma linha de cada origem até o centro, crescendo de 0 a 1 em 60% da fase, espessura 4 px. No máximo 3 especiais (fora Supernovas) por limpeza, na ordem do motor.

- [ ] **Step 1: Testes primeiro.** `limpeza_comum_solta_5_faiscas_no_padrao_e_3_no_suave`; `pulso_e_raio_8_faiscas_e_espectro_12_no_padrao`; `suave_usa_5_faiscas_nas_especiais`; `raio_tem_uma_onda_e_dois_feixes`; `espectro_tem_veu_e_faiscas_de_cores_diferentes`; `supernova_tem_anel_de_5_casas_e_linhas_de_fusao_so_na_primeira_fase`; `no_maximo_3_especiais_alem_das_supernovas`; `alfa_zero_no_comeco_e_particulas_somem_no_fim` (em `t = 0` todo `alfa == 0`; depois do fim do passo a lista é vazia); `tempo_estranho_nao_gera_nan_nem_passa_do_teto` (NaN, −5, 1e30, infinito: todo número finito; vazio ou com alfa 0); `pior_caso_nao_passa_do_teto` (3 especiais Espectro + 2 Supernovas com 8 origens ≤ `MAXIMO_DE_PARTICULAS`); `ajustar_suave_apaga_e_vibrante_realca`; `faiscas_saem_em_angulos_de_2pi_i_sobre_n_mais_0_18`.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game particulas` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar** como funções puras do progresso; ler as cores e medidas finas em `style.css` (linhas 135–150) e `app.js` (linhas 470–540).
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: verde; clippy sem aviso novo.
- [ ] **Step 5: Commit** `feat(game): partículas da limpeza (porte do JS), puras e testadas`.

### Task 9: Desenho das partículas e cenas de foto

**Files:**
- Modify: `game/src/render.rs`, `game/src/cenas.rs`, `game/src/main.rs` (se a lista de cenas morar lá)

**Interfaces:**
- Consumes: `particulas::particulas`, `Particula`, `ajustar`; `linha::{fusao_ms, fusao_da_casa, limpeza_da_casa}`; `App::passo_atual() -> Option<(&Passo, f32)>`.
- Produces: `render::desenhar_particulas(passo: &Passo, t_ms: f32, area: Rect)` (chamada em `draw_jogo` por cima das pedras e por baixo dos contornos); cenas de foto `particulas-pulso`, `particulas-raio`, `particulas-espectro`, `particulas-supernova` (instante fixo no meio do efeito).

Desenho: `Faisca` = quadrado girado com um quadrado maior e translúcido por baixo (o "brilho"); `Anel` = `draw_circle_lines`; `Linha` = retângulo girado; `Veu` = retângulo claro sobre o tabuleiro. Pedras: na fase de fusão as pedras de `origens` pulsam com `fusao_da_casa`; na fase principal as que somem usam `limpeza_da_casa` e, se `brilho > 0`, levam um quadrado branco translúcido por cima (`alfa = 0,3 × brilho`, já embutido na função). As sementes que produzem cada especial são achadas por exploração (como na Task 4 do M3: jogada N da Dica; Supernova só por seed ou por `injetar`-equivalente em `cenas.rs`).

- [ ] **Step 1: Testes primeiro.** `cenas.rs`: `cenas_de_particulas_existem_e_estao_no_passo_de_limpeza_certo` (cada cena termina num `Passo::Limpeza` cujo `visuais` tem o tipo esperado e `app.passo_atual()` está no meio do passo).
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game cenas` → Expected: FAIL (cenas não existem).
- [ ] **Step 3: Implementar** o desenho e as cenas. **Fotos obrigatórias** (540×960): as quatro cenas; conferir com Read: as faíscas aparecem em volta da casa da especial, o anel e os feixes cruzam o tabuleiro, a Supernova tem anel grande e linhas de fusão, nada fica fora da moldura de um jeito feio. Ajustar tamanho e brilho a olho e anotar no ledger.
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: verde; `cargo run -p prisma_game` por uns segundos no PC virtual sem pânico em jogada com cascata.
- [ ] **Step 5: Commit** `feat(game): desenho das partículas e cenas de foto`.

### Task 10: Intensidade dos efeitos na tela (controle no Zen e efeitos visuais)

**Files:**
- Modify: `game/src/layout.rs`, `game/src/app.rs`, `game/src/render.rs`, `game/src/main.rs`, `game/src/cenas.rs`

**Interfaces:**
- Consumes: `Intensidade` (Task 6), `montar(.., intensidade)` (Task 7), partículas (Tasks 8–9).
- Produces: `TelaSom.efeitos: Option<Rect>` (só Zen; última linha da folha, depois da Respiração: a folha do Zen passa a 10 linhas com vibrador, 9 sem); `Botao::Efeitos` (só Zen; toca para ir ao próximo valor, grava); `pub fn intensidade(&self) -> Intensidade` no `App` (Zen: a preferência; Clássico: `Padrao`), usada em `jogar` ao chamar `montar`; registro `PRISMA_M4 intensidade valor=suave|padrao|vibrante` ao mudar; geometria `escolhe_efeitos`; a camada de partículas usa `ajustar(..)`; a faixa "Nível N" ganha sombra menor no Suave e brilho roxo extra no Vibrante.

- [ ] **Step 1: Testes primeiro.** `layout.rs`: `a_folha_do_zen_com_10_linhas_cabe_em_360x640_e_1080x2400` (altura de cada linha ≥ 0,07 do menor lado), `no_classico_nao_ha_linha_de_efeitos`. `app.rs`: `apertar_efeitos_passa_ao_proximo_grava_e_registra`, `efeitos_so_existe_no_zen`, `jogada_usa_a_intensidade_escolhida_no_zen` (o `Passo::Limpeza.intensidade` e a duração mudam), `no_classico_a_intensidade_vale_padrao_mesmo_com_vibrante_gravado`, `trocar_a_intensidade_nao_muda_os_sons_nem_os_pontos` (mesma semente e jogada em Suave e Vibrante: mesma lista de efeitos sonoros, mesmos pontos), `fechar_e_abrir_lembra_a_intensidade`. `main.rs`: a geometria leva `escolhe_efeitos` só no Zen com a folha aberta. Cenas `particulas-espectro-suave` e `particulas-espectro-vibrante`.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game app` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar.** Fotos: `som` em 360×640 (Zen, 10 linhas, nada cortado) e as duas cenas novas ao lado de `particulas-espectro`; conferir com Read a diferença de intensidade.
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: verde; clippy sem aviso novo.
- [ ] **Step 5: Commit** `feat(game): intensidade dos efeitos no Zen (Suave, Padrão, Vibrante)`.

### Task 11: Ícone adaptativo (ferramenta de imagens)

**Files:**
- Modify: `tools/src/imagens.rs`

**Interfaces:**
- Produces: o comando `prisma_tools imagens` passa a gravar também, em `game/android_res/`: `mipmap-anydpi-v26/ic_launcher.xml` (`<adaptive-icon>` com `<background android:drawable="@color/ic_launcher_background"/>` e `<foreground android:drawable="@mipmap/ic_launcher_foreground"/>`), `values/ic_launcher_background.xml` (`<color name="ic_launcher_background">#RRGGBB</color>`) e `mipmap-{mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}/ic_launcher_foreground.png` com 108, 162, 216, 324 e 432 px (108 dp). Os `ic_launcher.png` comuns continuam. `pub fn primeiro_plano_do_icone(icone: &RgbaImage, lado: u32) -> RgbaImage` e `pub fn cor_de_fundo_do_icone(icone: &RgbaImage) -> [u8; 3]`.

Primeiro plano: a figura (o losango) isolada do `icon-512.png` pela diferença para o fundo (`alfa` cresce com a distância da cor de fundo local, com suavização nas bordas), centrada e com a altura em **61%** do lado (a ponta do losango cai a 33/108 do centro, dentro da zona segura de 66 dp). Fundo: média dos pixels opacos nos quatro cantos internos (a 12% das bordas).

- [ ] **Step 1: Testes primeiro** (com o `icon-512.png` sintético que os testes já montam, ou o real quando a pasta do JS existe): `primeiro_plano_tem_o_lado_pedido`, `a_figura_cabe_na_zona_segura_de_66_dp` (todo pixel com alfa > 10% está a no máximo `0,305 × lado` do centro, tolerância de 1 px), `a_figura_fica_centrada`, `fundo_vem_dos_cantos_do_icone`, `xml_adaptativo_cita_o_fundo_e_o_primeiro_plano`, `preparar_grava_tambem_o_icone_adaptativo` (as 5 densidades, o XML e a cor), e o teste `preparar_gera_22_imagens_e_5_icones...` passa a contar os arquivos novos (5 de primeiro plano + 2 de texto).
- [ ] **Step 2: Rodar e ver falhar:** `cd tools && cargo test imagens` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar.** Rodar `cargo run --manifest-path tools/Cargo.toml -- imagens /home/claude/thiagodluz/prisma game` e conferir `git status`: só os arquivos novos de `game/android_res/`. **Prévia:** montar fora do repositório (em `/tmp`, com um script Python) o primeiro plano sobre o fundo com máscara redonda e com máscara de quadrado arredondado, e abrir com Read: a figura inteira, sem corte, legível.
- [ ] **Step 4: Rodar:** `cd tools && cargo test` e `cargo test --workspace` → Expected: verdes (ferramentas 106 + novos).
- [ ] **Step 5: Commit** `feat(tools): ícone adaptativo do Android gerado a partir do icon-512.png`.

### Task 12: Versão do Android e conferências do APK no CI

**Files:**
- Modify: `game/Cargo.toml`, `tests/workspace.rs`, `.github/workflows/android.yml`

**Interfaces:**
- Produces: em `[package.metadata.android]`: `min_sdk_version = 26` e `target_sdk_version = 34`; teste que fixa os dois valores; passo "Conferir versões e ícone do APK" no fluxo.

Fluxo: depois do passo que lista o APK (ele mostra só 5 linhas do `aapt2 dump badging`), um passo novo roda o `badging` inteiro, lê `minSdkVersion`, `targetSdkVersion` e `application-icon-*` e reprova se o mínimo não for `26`, se o alvo não for `34` ou se o ícone não apontar para `mipmap-anydpi-v26/ic_launcher.xml`. Fica em **commits só desta tarefa**. Só o CI diz se o gerador de APK aceita as chaves e se a imagem Docker tem a plataforma 34; se não aceitar: registrar um `Ruling:` no ledger, voltar às versões antigas neste mesmo pedaço, anotar em `docs/notes/android-build.md` e avisar o Thiago (não mexer na imagem Docker).

- [ ] **Step 1: Teste primeiro** (`tests/workspace.rs`): `as_versoes_do_android_estao_fixadas` (`min_sdk_version = 26` e `target_sdk_version = 34` dentro de `[package.metadata.android]`, usando a ajuda `secao` do M3) e `o_fluxo_do_android_confere_versoes_e_icone` (o texto do fluxo cita `minSdkVersion`, `targetSdkVersion` e `anydpi`).
- [ ] **Step 2: Rodar e ver falhar:** `cargo test --test workspace` → Expected: FAIL.
- [ ] **Step 3: Implementar** as chaves e o passo do fluxo.
- [ ] **Step 4: Rodar:** `cargo test --workspace` → Expected: verde (a conferência real do APK só acontece no CI da Task 14).
- [ ] **Step 5: Commit** `ci: minSdk 26, targetSdk 34 e conferência das versões e do ícone no APK`.

### Task 13: Celular virtual: fase 11 (acabamento)

**Files:**
- Modify: `tools/src/registro.rs`, `tools/src/emulador.rs`, `tools/tests/amostra-logcat.txt`

**Interfaces:**
- Produces (`registro.rs`): `MARCADOR_M4`; `Linha::{Vibracao { toque: String }, VibracaoRecusada, Carregando { tipo: String }, Intensidade { valor: String }}` (lê `PRISMA_M4 vibracao toque=..`, `PRISMA_M4 vibracao recusada`, `PRISMA_M4 carregando tipo=..`, `PRISMA_M4 intensidade valor=..`); `Registro::toques_de_vibracao() -> usize`, `Registro::carregando_veio_antes(tipo: &str) -> Option<bool>` (a última `Carregando` do tipo vem antes da última `FaixaCarregada` do mesmo tipo; `None` se faltar uma delas), `Registro::ultimo_valor_de_intensidade() -> Option<String>`. `resumo` ganha `"prisma_m4"`. `Teste::acabamento()` = fase 11, chamada em `executar` depois de `som()`.

Fase 11 (dentro do Zen, depois da fase 10): (1) tocar `menu`; esperar a geometria `aba_recordes` (prazo `PRAZO_DA_FOLHA_MS`); foto `11-recordes`; tocar `aba_creditos`; foto `11-creditos`; tocar `fechar_menu`. (2) abrir o Som (`abrir_a_folha_de_som`); se a geometria tem `liga_vibracao`: tocá-lo e exigir `PRISMA_M4 vibracao toque=ligou`; senão imprimir `AVISO: o celular virtual não tem vibrador` (não reprova). (3) tocar `escolhe_efeitos` e exigir uma linha `PRISMA_M4 intensidade valor=..` nova. (4) tocar `liga_ambiente` duas vezes (desliga e liga) e exigir `carregando_veio_antes("ambiente") == Some(true)` e um `PRISMA_M3 carregado tipo=ambiente` **novo**: a fase 10 já deixou um `carregado` antigo no registro, então `esperar_faixa` ganha um parâmetro (quantos `carregado` do tipo já existiam) e só aceita quando houver mais que isso. (5) `fechar_som`; se a vibração ficou ligada, uma jogada (`jogada_arrastando`) tem de somar um `toque=` ao `toques_de_vibracao()`. (6) `continua_aberto` e `sem_erro_de_som` no fim.

- [ ] **Step 1: Testes primeiro.** `registro.rs`: leitura de cada linha nova (e de linha malformada → ignorada), `toques_de_vibracao_conta`, `carregando_veio_antes_*` (antes, depois, faltando), amostra de logcat com as linhas novas (a contagem do teste da amostra sobe). `emulador.rs` (celular falso com flags novas, como as da Task 9 do M3): `acabamento_completo_passa` (o teste do fluxo inteiro atualizado: novas fotos `11-recordes`, `11-creditos` e a contagem de toques), `reprova_se_o_menu_nao_abre`, `reprova_se_ligar_a_vibracao_nao_pede_o_toque`, `sem_vibrador_so_avisa`, `reprova_se_a_intensidade_nao_registra`, `reprova_se_o_carregando_vem_depois_do_carregado`, `reprova_se_o_app_morre_na_fase_11`.
- [ ] **Step 2: Rodar e ver falhar:** `cd tools && cargo test registro` → Expected: não compila / FAIL (depois `emulador`).
- [ ] **Step 3: Implementar.** Rodada de mutação como na Task 9 do M3 (trocar prazos, inverter a ordem do carregando, tirar a conferência do toque): cada mutação tem de derrubar o teste esperado; registrar no ledger.
- [ ] **Step 4: Rodar:** `cd tools && cargo test` e `cargo test --no-default-features` → Expected: verdes.
- [ ] **Step 5: Commit** `test(tools): fase 11 do celular virtual confere menu, vibração, intensidade e carregando`.

### Task 14: Documentação, conferência final e CI

**Files:**
- Create: `docs/notes/acabamento.md`
- Modify: `docs/notes/android-build.md`, `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md`

**Interfaces:**
- Consumes: tudo acima.

`acabamento.md` (português simples): como funcionam a vibração (padrões, quando vibra, por que o interruptor some), as partículas (tabela de efeitos e tempos), a intensidade, o menu ≡, o "Carregando…", o ícone (como gerar e como olhar a prévia) e as versões do Android (por que 34 e não 35); linhas de registro novas; o que só o celular de verdade confirma. `android-build.md`: fase 11, linhas `PRISMA_M4`, `minSdk`/`targetSdk`, ícone adaptativo. Spec principal: seção 5.7 e M4 com as versões reais e uma linha apontando para o design da M4.

- [ ] **Step 1: Escrever os documentos** com os números reais (nada de "a medir" quando já dá para medir).
- [ ] **Step 2: Conferência local completa:** `cargo test --workspace`, `cd tools && cargo test`, clippy do jogo (lista de avisos igual à das Global Constraints), `git status` limpo, nenhum áudio/chave no git (`git ls-files | grep -E "\.(ogg|wav|mp3|keystore|jks)$"` vazio).
- [ ] **Step 3: Commit** `docs: notas do acabamento (M4) e versões do Android`.
- [ ] **Step 4: CI (só com autorização nova do Thiago):** enviar a branch e rodar o fluxo do Android nela; esperar os três trabalhos. Registrar no ledger: tamanho do APK (≤ 60 MB), linhas do `badging` (`minSdkVersion`, `targetSdkVersion`, ícone), a saída da fase 11 nos dois Androids e as fotos `11-*` decodificadas do log. Se algo falhar: um único passe de conserto (cada conserto com teste RED→GREEN quando for do jogo) e outra autorização para rodar de novo.
- [ ] **Step 5: Atualizar os números** nas notas com o que o CI mediu e fazer o commit `docs: números medidos da M4 no celular virtual`.
