# Gestos interrompidos e teclado — Plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Item 4 da auditoria: um gesto interrompido pelo sistema nunca vira jogada, e o jogo inteiro funciona pelo teclado no PC.

**Architecture:** O laço (`main.rs`) assina os eventos crus do Macroquad para saber de toque cancelado e de janela minimizada; uma função pura em `input.rs` decide os `Evento` do quadro. O teclado vira `Tecla` em `input.rs` e `App::tecla` decide o que fazer (foco em casa, botão ou barra de volume); o render desenha o foco.

**Tech Stack:** Rust, Macroquad 0.4.16 (`utils::register_input_subscriber`, `repeat_all_miniquad_input`, `is_key_pressed`).

**Spec:** sem documento de design: o Thiago pediu "pode fazer com a sua sugestão" (gestos + teclado completo). Decisões abaixo são minhas.

## Global Constraints

- Toque cancelado (`TouchPhase::Cancelled`) ou janela minimizada no quadro → só `Evento::Cancelar` nesse quadro (nunca `Cima`).
- Teclado como o original: setas andam pelas casas sem dar a volta na linha; Enter/Espaço = tocar a casa. Mais: Tab / Shift+Tab percorrem os botões visíveis e o tabuleiro (uma parada); Enter/Espaço apertam o botão em foco; Esc fecha diálogo, folha, escolha de casa ou desmarca a pedra; ←/→ na barra de volume em foco mudam de 5 em 5.
- O contorno de foco só aparece depois de usar o teclado e some ao tocar/clicar (como `:focus-visible`).
- Com animação, setas e Enter no tabuleiro não fazem nada (o original bloqueia); Tab e Esc continuam.
- Clippy do `game` em 9 linhas; testes TDD com mutação; commits em português.

## Review Focus

1. Arrasto pela metade e o app minimizado: ao voltar, nenhuma troca aconteceu e o próximo toque funciona.
2. Foco num botão que some (ex.: Embaralhar ao trocar para o Clássico, Dica no fim de partida): Enter não aperta nada invisível e Tab continua.
3. Esc com a escolha de casa do menu secreto pendente.
4. Foco numa casa e Enter durante a animação.
5. Teclado com a folha de Som aberta no Clássico (botões do Zen não existem).

---

### Task 1: Gesto interrompido vira cancelamento
**Files:** `game/src/input.rs`, `game/src/main.rs`, `game/src/app.rs` (testes)
- [ ] Teste: `eventos_do_quadro(EstadoDoMouse { apertou, segurando, soltou, x, y, ultima }, interrompido) -> Vec<Evento>`: interrompido → `[Cancelar]` mesmo com `soltou`; sem interrupção, a ordem de hoje (Baixo / Mover / Cima).
- [ ] Teste no App: arrasto começado + `Cancelar` + `Cima` em outra casa → nenhuma jogada; depois um arrasto novo joga. Com a barra de volume em arrasto, `Cancelar` encerra o arrasto e grava.
- [ ] Implementar; `main.rs` registra um assinante e um `Interrupcoes` (implementa `miniquad::EventHandler`: `touch_event` com `Cancelled` e `window_minimized_event` marcam).
- [ ] Commit `fix: gesto interrompido pelo sistema não vira jogada`.

### Task 2: Teclado no App
**Files:** `game/src/input.rs` (`pub enum Tecla { Esquerda, Direita, Acima, Abaixo, Confirmar, Proximo, Anterior, Voltar }`), `game/src/app.rs`
- Produces: `pub enum Foco { Casa(usize), Botao(Botao), Volume(Canal) }`, `App::tecla(&mut self, tecla: Tecla, tela: &TelaJogo)`, `App::foco_visivel(&self) -> Option<Foco>`.
- [ ] Testes: setas andam e não dão a volta; Enter duas vezes em casas vizinhas troca; Tab percorre botões visíveis na ordem de `botoes_visiveis` + o tabuleiro e Shift+Tab volta; Enter no botão em foco aperta; foco em botão que sumiu → Tab recomeça e Enter não faz nada; Esc fecha diálogo/folha/escolha/seleção nessa ordem; ←/→ na barra de volume; setas/Enter no tabuleiro durante animação não fazem nada; toque esconde o contorno; Enter numa casa durante a escolha de especial cria o especial.
- [ ] Implementar. Commit `feat: jogar e navegar pelo teclado`.

### Task 3: Desenho do foco e leitura das teclas
**Files:** `game/src/render.rs`, `game/src/main.rs`, `game/src/cenas.rs`
- [ ] Cena `teclado` (foco numa casa) e `teclado-botao` (foco no Novo jogo) no teste de cenas.
- [ ] Render: casa em foco com o anel tingido de verde-água; botão ou barra com o anel em volta (`formas.anel`, 3 px para fora). `main.rs` lê `is_key_pressed` das teclas e Shift.
- [ ] Fotos 360×640 e 1080×2400. Commit `feat: contorno de foco do teclado`.

### Task 4: Notas
- [ ] `docs/notes/teclado-e-gestos.md`; README tira "navegação por teclado" de "Ainda não tem". Verificação geral. Commit `docs: notas de teclado e gestos`.
