# Menu secreto, desfazer e cheats — Plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ligar à tela do jogo o desfazer e os cheats que o motor já tem, numa aba "Ferramentas" da folha do Menu liberada por 7 toques na linha da versão, com recordes protegidos e restaurados.

**Architecture:** Toda a regra fica no `App` (`game/src/app.rs`), testável sem janela: pilha de recordes paralela ao `undo_history` do motor, proteção de recordes em partida com cheat, botões novos e a "escolha de casa". `layout.rs` ganha os retângulos da aba; `render.rs` a desenha com o vidro da V3; `cenas.rs` ganha cenas de foto.

**Tech Stack:** Rust, Macroquad 0.4, motor `prisma_engine` (`src/cheats.rs`, `Game::undo_last_move`).

**Spec:** `docs/superpowers/specs/2026-11-27-prisma-menu-secreto-design.md`

## Global Constraints

- 7 toques na linha da versão (aba Créditos), no máximo 2.200 ms entre toques, liberam a aba; aviso "Ferramentas do desenvolvedor ativadas".
- A aba some ao fechar o app (não é gravada).
- Pontos: valor inicial 5.000, passo 1.000, de 1.000 a 1.000.000. Nível: inicial 10, passo 1, de 1 a 10.000.
- Histórico: até 50 ações; limpo ao reabrir, ao trocar de modo (as duas partidas), ao começar outra partida e no Embaralhar normal do Zen.
- Partida com `cheats_used` nunca sobe recorde (jogada, troca de modo, novo jogo, abertura).
- Desfazer devolve os recordes do modo de antes da ação desfeita e os grava.
- Com animação (`ocupado()`), nenhum botão da aba faz nada e a escolha de casa não aceita toque.
- Mensagens: "Confira os valores ou tente outra ação" (cheat recusado), "Embaralhe ou ative a proteção contra derrota" (jogada automática sem jogada), "Toque numa casa" (escolhendo), avisos curtos dos cheats aceitos.
- Visual da V3; toda cor de texto nova entra em `PARES_DE_TEXTO`. Clippy do `game` em 9 linhas. Commits em português.

## Review Focus

1. Desfazer depois de 50+ ações: a pilha de recordes e a do motor continuam do mesmo tamanho e casam (a 51ª ação descarta as duas mais antigas).
2. Cheat seguido de troca de modo e volta: o recorde do modo com cheat não sobe pelo `alcancar_recordes_de`.
3. Escolhendo casa, o jogador abre o ♪/≡ ou troca de modo: a escolha é cancelada, nada é aplicado depois.
4. Toques na versão espaçados (> 2,2 s) ou fora da aba Créditos não liberam nada.
5. Menor celular (320×480): os botões da aba cabem e não se sobrepõem.

---

### Task 1: Recordes protegidos e desfazer no App

**Files:**
- Modify: `game/src/app.rs`

**Interfaces:**
- Produces: `App::desfazer(&mut self)` (privada; chamada por `Botao::Desfazer` na Task 2), `App::lembrar_recordes(&mut self, modo: Mode, antes: Recordes)` (privada), campo `pilha_de_recordes: [Vec<Recordes>; 2]`, `App::pode_desfazer(&self) -> bool`, `App::acoes_para_desfazer(&self) -> usize`.

- [ ] **Step 1: Testes que falham** (no módulo de testes do `app.rs`, com `App::com_sementes` e as ajudas `boa_jogada`/`arrastar` que já existem):
  - `partida_com_cheat_nao_sobe_recorde`: `jogo_mut().cheat_add_points(100_000)`; uma jogada válida; `recordes(modo).pontuacao` continua o de antes.
  - `cheat_nao_sobe_recorde_ao_trocar_de_modo_nem_no_novo_jogo`: cheat de pontos no Zen, `apertar(Classico)`, `apertar(Zen)`, `apertar(NovoJogo)`+`Confirmar` → recorde do Zen inalterado.
  - `o_melhor_do_painel_nao_conta_placar_com_cheat`: depois do cheat, `painel().melhor` = recorde formatado.
  - `desfazer_volta_a_jogada_e_os_recordes`: jogada que bate recorde (recorde zerado), `desfazer()` → placar 0 e `recordes(Zen)` = zerados; `jogo().cheats_used` true.
  - `a_pilha_de_recordes_acompanha_as_50_do_motor`: 55 ações válidas (cheat de +100 repetido, cada uma com `lembrar_recordes`) → `pilha_de_recordes[0].len() == jogo().undo_history.len() == 50`.
  - `trocar_de_modo_e_embaralhar_limpam_o_historico`: depois de uma jogada, `apertar(Classico)` → os dois `undo_history` e as duas pilhas vazias; no Zen, jogada + `apertar(Embaralhar)` (sem animação pendente) → vazios.
  - `desfazer_sem_historico_nao_faz_nada` e `desfazer_durante_animacao_nao_faz_nada`.
- [ ] **Step 2:** `cargo test -p prisma_game` → falham (funções inexistentes / recordes sobem).
- [ ] **Step 3: Implementar.** `registrar` e `alcancar_recordes_de` não mexem nos recordes se `partida(modo).cheats_used`; `melhor_parado` usa só o recorde se `jogo().cheats_used`. Em `jogar`, quando `resultado.valid`, `lembrar_recordes(self.modo, recordes_antes)` com os recordes de antes de `registrar`. `lembrar_recordes` empurra e mantém só os últimos `partida(modo).undo_history.len()`; `gravar_tudo` também apara as duas pilhas ao tamanho do histórico do motor (cobre embaralhar, novo jogo). `trocar_de_modo` limpa `undo_history` das duas partidas e as duas pilhas. `desfazer`: se `ocupado()` ou nada a desfazer, sai; `undo_last_move()`; recordes do modo = topo da pilha (se houver); `limpar_tela()`; `gravar_tudo` com o modo marcado como mudado; aviso "Ação desfeita".
- [ ] **Step 4:** `cargo test -p prisma_game` → passa; mutações: tirar a checagem de `cheats_used` em `registrar`; não aparar a pilha.
- [ ] **Step 5: Commit** `feat: desfazer com recordes e recordes protegidos em partida com cheat`.

### Task 2: Cheats e escolha de casa no App

**Files:**
- Modify: `game/src/app.rs`

**Interfaces:**
- Consumes: `desfazer`, `lembrar_recordes` (Task 1).
- Produces: `Folha::Ferramentas` (`e_do_menu` inclui); variantes de `Botao` (todas com `da_folha() == Some(Folha::Ferramentas)`, exceto `Versao` → `Some(Folha::Creditos)` e `AbaFerramentas` → `Some(Folha::Ferramentas)`): `AbaFerramentas, Versao, Desfazer, PontosMenos, PontosMais, Somar, NivelMenos, NivelMais, IrParaNivel, Mais100, Mais1000, ProximoNivel, Pulso, Raio, Espectro, Supernova, EmbaralharSemLimite, JogadaAutomatica, Explodir, ImpedirDerrota`; `pub enum Especial { Pulso, Raio, Espectro, Supernova }`; campos `pontos_escolhidos: u64`, `nivel_escolhido: u32`, `escolhendo: Option<Especial>`, `ferramentas_liberadas: bool`, `toques_na_versao: u8`, `desde_o_toque_ms: f32`; `pub fn ferramentas(&self) -> EstadoFerramentas` com `{ pontos: String, nivel: String, acoes: usize, pode_desfazer: bool, com_cheats: bool, impedir_derrota: bool, ocupado: bool }`.

- [ ] **Step 1: Testes que falham:**
  - `sete_toques_na_versao_liberam_a_aba`: com `Folha::Creditos` aberta, 6× `apertar(Versao)` → não liberada; 7º → liberada e aviso "Ferramentas do desenvolvedor ativadas".
  - `toques_espacados_nao_liberam`: 4 toques, `avancar(2_300.0)`, 3 toques → não liberada.
  - `botoes_da_aba_so_valem_liberada_e_com_ela_aberta`.
  - `passos_de_pontos_e_nivel_respeitam_os_limites`: −/+ e limites 1.000..=1.000.000 e 1..=10.000.
  - `somar_e_ir_aplicam_e_avisam`: `Somar` → +5.000 no placar, `cheats_used`, pilha +1, aviso "Pontos adicionados"; `IrParaNivel` → nível 10, aviso "Nível alterado".
  - `especial_espera_o_toque_na_casa`: `apertar(Pulso)` fecha a folha e põe `escolhendo`; `painel().aviso` = "Toque numa casa"; um `Evento::Baixo` na casa 27 cria o Pulso lá (`board` da casa 27 tem `TileKind::Burst`), aviso "Especial criado".
  - `toque_fora_ou_abrir_folha_cancela_a_escolha`: fora da grade → `escolhendo` None e nada criado; o mesmo com `apertar(Som)` e com troca de modo.
  - `supernova_e_explodir_animam_como_jogada`: depois de `Explodir`, `ocupado()` true, e o placar só sobe no fim da animação (como na jogada).
  - `jogada_automatica_joga_a_dica` e, sem jogada (`force_no_moves` do motor, se exposto, senão Clássico sem jogadas da cena "fim"), aviso "Embaralhe ou ative a proteção contra derrota".
  - `cheat_recusado_avisa`: `IrParaNivel` com o nível atual igual ao escolhido ou `cheat_spawn_special` recusado → "Confira os valores ou tente outra ação", pilha inalterada.
  - `durante_a_animacao_a_aba_nao_faz_nada`.
- [ ] **Step 2:** testes falham.
- [ ] **Step 3: Implementar.** Regra de folha: −/+, `Desfazer` e `ImpedirDerrota` deixam a folha aberta; os demais cheats fecham a folha e mostram o aviso. Cheats de tabuleiro com cascata (`cheat_explode`, `cheat_supernova`) usam a mesma montagem de `animar_atalho` (deixa de ser só para cenas) e `registrar` dos recordes no fim como `jogar`. `EmbaralharSemLimite` anima com `Passo::Embaralhar`. Todo cheat aceito chama `lembrar_recordes` antes de aplicar e `gravar_tudo` depois. A escolha de casa entra em `tocar` antes dos botões: `Evento::Baixo` dentro da grade (`input::casa_em`) aplica; fora cancela e não aperta nada. `trocar_de_modo`, `alternar_folha`, `confirmar_novo_jogo` zeram `escolhendo`. Os 7 toques: `desde_o_toque_ms` soma em `avancar`; toque com `desde_o_toque_ms > 2_200` recomeça a contagem.
- [ ] **Step 4:** testes passam; mutações: limite 2.200 → infinito; `ImpedirDerrota` sem `lembrar_recordes`.
- [ ] **Step 5: Commit** `feat: cheats e escolha de casa no menu secreto`.

### Task 3: Lugar dos controles e toques

**Files:**
- Modify: `game/src/layout.rs`, `game/src/app.rs` (`botoes_visiveis`)

**Interfaces:**
- Consumes: as variantes de `Botao` e `Folha::Ferramentas` (Task 2).
- Produces: em `TelaMenu`: `versao: Rect` (faixa do alto de `conteudo`, altura `1,6 × texto`), `abas_com_ferramentas: [Rect; 3]`, `fechar` igual; `pub struct TelaFerramentas` com `estado, desfazer, pontos_menos, pontos_valor, pontos_mais, somar, nivel_menos, nivel_valor, nivel_mais, ir, mais_100, mais_1000, proximo_nivel: Rect, especiais: [Rect; 4], embaralhar, jogada_automatica, explodir, impedir_derrota: Rect` dentro de `TelaMenu.ferramentas`.

- [ ] **Step 1: Testes que falham** (`layout.rs`): `a_aba_ferramentas_cabe_sem_sobrepor` em 320×480, 360×640, 540×960, 1080×2400: todos os retângulos de `TelaFerramentas` dentro de `conteudo`, dois a dois sem sobrepor, e altura ≥ 32 px a partir de 360×640; `a_linha_da_versao_fica_no_alto_dos_creditos`; `as_tres_abas_dividem_a_largura`. Em `app.rs`: `os_botoes_da_aba_ficam_visiveis_so_liberada` (com `botao_em` no centro de `menu.ferramentas.somar`) e `a_versao_so_e_tocavel_em_creditos`.
- [ ] **Step 2:** falham.
- [ ] **Step 3: Implementar** as linhas na ordem do esboço do design (estado; desfazer; pontos; nível; +100/+1.000/próximo; 4 especiais; embaralhar/auto/explodir; impedir derrota), cada linha com a mesma altura. `botoes_visiveis` usa `abas_com_ferramentas` quando liberada (Recordes, Créditos, Ferramentas) e acrescenta `Versao` só em Créditos e os controles só em Ferramentas.
- [ ] **Step 4:** passam; mutação: duas linhas com o mesmo `y`.
- [ ] **Step 5: Commit** `feat: lugar dos controles da aba Ferramentas`.

### Task 4: Desenho da aba, aviso de escolha e cenas de foto

**Files:**
- Modify: `game/src/render.rs`, `game/src/cenas.rs`

**Interfaces:**
- Consumes: `App::ferramentas()`, `TelaFerramentas`, `TelaMenu.versao` (Tasks 2–3).

- [ ] **Step 1: Testes que falham:** em `cenas.rs`, a lista de cenas inclui `ferramentas` e `escolher-casa` (o teste que já confere a lista); a cena `ferramentas` abre com a aba liberada e aberta, `escolher-casa` com `escolhendo = Some(Pulso)`. Em `render.rs`, se surgir cor de texto nova, o par em `PARES_DE_TEXTO`.
- [ ] **Step 2:** falham.
- [ ] **Step 3: Implementar:** três abas quando liberada; em Créditos, a primeira linha dos créditos desenhada em `versao` e o resto abaixo; a aba Ferramentas com `botao_da_tela` (Desfazer, Somar, Ir e o especial em `botao_principal`; os outros em vidro), valores em Sora no meio dos −/+, a linha de estado em `TEXTO_SECUNDARIO`, "Impedir derrota" com `desenhar_linha_com_interruptor`; com `ocupado` todos apagados (`ativo = false`), `Desfazer` apagado sem histórico. O aviso "Toque numa casa" usa a pílula de sempre.
- [ ] **Step 4:** testes passam; fotos de `ferramentas`, `escolher-casa` e `creditos` em 320×480, 360×640 e 1080×2400, sem corte nem sobreposição.
- [ ] **Step 5: Commit** `feat: aba Ferramentas desenhada e cenas de foto`.

### Task 5: Verificação geral e notas

**Files:**
- Modify: `README.md` (linha das funções), `docs/notes/` (nota nova `ferramentas.md`)

- [ ] **Step 1:** `cargo test --workspace`, `cargo test --manifest-path tools/Cargo.toml`, clippy do `game` (9).
- [ ] **Step 2:** Fotos de todas as cenas em 360×640 (nada ficou pior que na V3).
- [ ] **Step 3:** `docs/notes/ferramentas.md`: como abrir, o que cada botão faz, limites e limpeza do histórico, recordes protegidos, o que só o celular confirma (7 toques no dedo, tamanho dos botões). README menciona o menu secreto em uma linha.
- [ ] **Step 4: Commit** `docs: notas do menu secreto`.
