# Prisma Rust · Fase 3 V2 (topo, modos e letra Roboto) — plano de implementação

> **Para quem executa:** REQUIRED SUB-SKILL: superpowers:executing-plans (execução na própria sessão, uma tarefa por vez). Os passos usam caixas `- [ ]`.

**Objetivo:** completar a tela igual à do "Novo" do JS: topo com logo + PRISMA + ♫ + ≡, dois cartões de modo (ícone + nome + frase) embaixo, e a fonte Roboto em todo o jogo.

**Arquitetura:** a fonte vira um módulo (`texto.rs`) por onde passa todo texto; `layout.rs` troca as abas do topo por uma linha de topo (marca + dois botões) e por uma linha de cartões de modo no fim (os campos `aba_zen`/`aba_classico` mantêm o nome e passam a ser os dois cartões); `render.rs` desenha as formas novas com as ferramentas da V1 (`Formas`, `desenhar_forma`).

**Tecnologia:** Rust, Macroquad 0.4.16 (`load_ttf_font_from_bytes`, `draw_text_ex`), `formas.rs` e `icones.rs` da V1.

**Spec:** `docs/superpowers/specs/2026-11-23-prisma-fase3-visual-design.md` (seções 4, 5 "V2" e 8). Notas da V1: `docs/notes/visual.md`.

## Global Constraints

- Tudo cabe numa tela só, sem rolar, em 360×640 e em 1080×2400; toda área tocável vem de `layout.rs`; vale "cabe na tela e não se cruza" para o layout novo (`CELULARES`, `TELAS`, os dois modos de jogo).
- Roboto Regular e Bold **estáticas**, embutidas com `include_bytes!` em `game/assets/`, licença OFL junto (`OFL.txt`) e citada em `LICENSE-ART.md`. As estáticas se geram da variável instalada: `fonttools varLib.instancer "/usr/share/fonts/truetype/google-fonts/ofl/roboto/Roboto[wdth,wght].ttf" wght=400 wdth=100 -o Roboto-Regular.ttf` (e `wght=700` para a Bold); a licença está em `.../roboto/OFL.txt`. Cada uma tem ~158 KB.
- A Roboto não tem o glifo ♫: ♫, ≡, lua e faísca são desenhados com formas (como a nota ♪ de hoje).
- Valores do JS (`style.css`): `h1` letra espaçada 0,22 em, 19 px, peso 850 (usar Bold); marca = `icon-192.png` 42×42 raio 10; botão de ícone 38×38 raio 12, fundo `ffffff10`, borda `ffffff27`, cor do ícone `d7d5f8`; cartão de modo raio 16, fundo `ffffff0b`, borda `ffffff23`, selecionado fundo `756aad32` borda `ad9bff99`; caixa do ícone raio 11, fundo `9785e438`, ícone `cfbdff`; nome do modo Bold, frase `b9b5d4` (cerca de 70% do tamanho do nome); frases: "Sem pressa, sem derrota" e "Sem cronômetro · pode acabar".
- Não mudar regras, animações, som, vibração, salvamento. Texto nunca cortado: `escrever` continua encolhendo a letra para caber.
- Commits em português (`feat:`, `test:` ...) com os dois trailers do projeto; `game/` e `tools/` não são rustfmt-clean: não reformatar.

## Review Focus

- Texto com Roboto mais largo ou mais estreito que a fonte de hoje: rótulos de botão, cartões de modo e as três folhas (Som, Recordes, Créditos) em 360×640 sem corte nem sobreposição (testar por foto).
- Caractere sem glifo na Roboto (·, —, Í, ç, ã): nenhum quadradinho vazio em lugar nenhum (varrer os textos do jogo).
- Fonte não carregada ou medida zero: nada de NaN nem pânico (`escrever` ignora tamanho < 1).
- Toque na linha do topo e nos cartões de modo: o toque no logo/PRISMA não faz nada; os cartões trocam de modo; ♫ e ≡ abrem as folhas; toque fora de tudo não faz nada.
- Em 360×640 o tabuleiro encolhe (se preciso) mas as casas continuam com alvo de toque confortável; nada se cruza.

---

### Task 1: Módulo de texto com a Roboto

**Files:**
- Create: `game/src/texto.rs`, `game/assets/Roboto-Regular.ttf`, `game/assets/Roboto-Bold.ttf`, `game/assets/OFL.txt`
- Modify: `game/src/main.rs` (`mod texto;`, carregar as fontes na abertura), `game/src/render.rs` (trocar todo `draw_text`/`measure_text` com fonte `None` do jogo de verdade pelas funções do módulo; manter só o texto do "teste de vida" antigo), `game/assets/LICENSE-ART.md`

**Interfaces:**
- Produces: `pub enum Peso { Normal, Negrito }`; `pub fn carregar()` (lê as duas fontes embutidas; chamada uma vez na abertura, guarda em `thread_local!`); `pub fn medir(texto: &str, tamanho: f32, peso: Peso) -> (f32, f32)` (largura e altura de referência de "Ag"); `pub fn desenhar(texto: &str, x: f32, y: f32, tamanho: f32, cor: Color, peso: Peso)`; `pub fn tamanho_que_cabe(largura_do_texto: f32, largura_da_area: f32, tamanho: f32) -> f32` (puro).
- Consumes: nada de tarefas anteriores. `escrever(texto, area, tamanho, cor, alinhamento)` de `render.rs` ganha um parâmetro `peso: Peso`.

- [ ] **Step 1: Testes primeiro** em `texto.rs`: `as_fontes_embutidas_sao_estaticas` (os bytes de Regular e Bold começam com `0x00010000`, têm mais de 100 000 bytes e **não** contêm a tabela `fvar`); `tamanho_que_cabe_encolhe_so_quando_precisa` (cabe → devolve igual; 2× maior → metade; largura da área 0 ou negativa → 0; NaN → 0). Rodar: FALHAM (módulo não existe).
- [ ] **Step 2: Gerar as fontes** com os comandos de Global Constraints, copiar `OFL.txt`, acrescentar as linhas no `LICENSE-ART.md` (a Roboto é Apache/OFL do Google, não CC BY-SA: escrever a seção separada).
- [ ] **Step 3: Implementar** `texto.rs` e trocar as chamadas em `render.rs` (`escrever`, `botao_da_tela`, os outros `measure_text`/`draw_text` do jogo, a quebra de linhas em ~linha 876). Rótulos de botão e valores de pontos em `Negrito`; frases e textos corridos em `Normal`.
- [ ] **Step 4:** `cargo test --workspace` passa e `cargo clippy` sem avisos novos. Foto `--cena inicio` em 360×640: o texto é Roboto (letras com cara de Roboto, "Ag" sem serifa monoespaçada).
- [ ] **Step 5: Mutação** (cp/sed/restaurar): `tamanho_que_cabe` sem o ramo "encolhe" deve fazer o teste falhar. Commit `feat: texto do jogo em Roboto (Regular e Bold embutidas)`.

### Task 2: Layout da linha de topo e dos cartões de modo

**Files:** Modify: `game/src/layout.rs`, `game/src/app.rs` (só os testes que citam `aba_*`), testes em `layout.rs`.

**Interfaces:**
- Produces: em `TelaJogo` novo campo `pub marca: Rect` (a área do logo + PRISMA, à esquerda da linha de topo); `botao_som` e `botao_menu` passam a ser quadrados 0,105 × largura útil (a 360 px: 38 px) alinhados à direita da MESMA linha de topo (menu na ponta, som à esquerda dele, vão de `ESPACO/2` entre eles); `aba_zen` e `aba_classico` passam a ser os dois cartões de modo, lado a lado, na parte de BAIXO da tela, depois dos botões de ação; constantes `TOPO_ALTURA = 0.085` (substitui `ABAS_ALTURA` em cima) e `MODOS_ALTURA = 0.11`. Ordem vertical: topo, cartão de pontos, barra, rótulo, moldura, botões, cartões de modo.
- Consumes: V1 (`grade`, `cartao_pontos`, `tela_jogo_com`).

- [ ] **Step 1: Testes primeiro** (todos nos `CELULARES`/`TELAS`, os dois modos): `a_linha_de_topo_tem_marca_e_dois_botoes` (marca à esquerda de `botao_som`, som à esquerda do menu, os três na mesma faixa vertical, nenhum cruza outro, menu termina na margem direita do tabuleiro); `os_cartoes_de_modo_ficam_embaixo` (`aba_zen.y` e `aba_classico.y` maiores que o fim dos botões de ação e menores que a altura da tela, mesma altura e largura, lado a lado sem cruzar, dentro da tela); `a_ordem_vertical_da_tela_nova` (topo < cartão de pontos < barra < moldura < botões < cartões de modo); `o_toque_confortavel_continua` (cada casa da grade tem lado ≥ 20 px em 360×640 — se não tiver, o orçamento vertical precisa de ajuste); ampliar o teste de tela impossível (`marca` também vazia). Atualizar os testes antigos que assumiam as abas no topo (`layout.rs` ~915–940, ~1166–1173; `app.rs` ~1986). Rodar: FALHAM.
- [ ] **Step 2: Implementar** em `tela_jogo_com`. Se a soma das alturas não couber, `tabuleiro` encolhe antes de qualquer outro bloco (a lógica de encolher já existe); documentar as proporções em comentário.
- [ ] **Step 3:** suíte do jogo verde. Mutação: trocar a ordem vertical (cartões de modo acima dos botões) → `a_ordem_vertical_da_tela_nova` falha. Commit `feat: layout com linha de topo e cartões de modo embaixo`.

### Task 3: Topo desenhado (logo, PRISMA, ♫ e ≡)

**Files:** Create: `game/assets/marca.png` (cópia de `icon-192.png` do JS, 192×192; entra no `LICENSE-ART.md`). Modify: `game/src/render.rs` (`Formas` ganha `botao_icone: Forma`, receita 64/12/1 do Global Constraints; `Sprites` guarda a textura da marca), `desenhar_cabecalho`.

**Interfaces:** Consumes: `tela.marca`, `tela.botao_som`, `tela.botao_menu` (Task 2); `texto::desenhar`/`medir` (Task 1); `desenhar_forma_tingida` e a nota ♪ e o ≡ desenhados que já existem. Produces: `fn desenhar_topo(sprites: &Sprites, app: &App, tela: &TelaJogo)`.

- [ ] **Step 1:** implementar `desenhar_topo`: logo 42/390 da largura, quadrado com raio 10/42 do lado (desenhar a textura com `desenhar_forma`-style não serve para PNG: recortar cantos com a forma `solida` não é preciso; aceitar canto reto se o PNG já tem cantos arredondados — conferir o arquivo); "PRISMA" Bold com 0,22 em entre as letras, desenhado letra a letra com `medir`, centrado na vertical; ♫ (nota) e ≡ dentro de `botao_icone`, com o estado "som ligado/desligado" que o ♪ de hoje já tem; remover o desenho das abas Zen/Clássico do topo.
- [ ] **Step 2:** fotos `inicio` e `classico` em 360×640 e 1080×2400, lado a lado com `docs/notes/ref/js-novo.jpg`: topo igual em ordem, tamanhos e cores. Suíte e clippy sem avisos novos.
- [ ] **Step 3: Mutação visual:** trocar a cor da receita `botao_icone` por vermelho, a foto muda; restaurar. Commit `feat: topo com logo, PRISMA e botões de som e menu`.

### Task 4: Cartões de modo com ícone e frase

**Files:** Modify: `game/src/icones.rs` (`pub fn lua(area: Rect) -> Vec<Linha>`), `game/src/render.rs` (`Formas` ganha `modo` e `modo_escolhido`, `desenhar_cartao_de_modo`), testes.

**Interfaces:** Consumes: `tela.aba_zen`/`aba_classico` (Task 2), `icones::faisca` (V1), `texto` (Task 1). Produces: `icones::lua`; `fn desenhar_cartao_de_modo(formas: &Formas, area: Rect, icone: Icone, nome: &str, frase: &str, escolhido: bool, tamanho: f32)` (estende o `enum Icone` da V1 com `Lua` e `Faisca`).

- [ ] **Step 1: Testes primeiro** (`icones.rs`): `a_lua_cabe_na_area_e_e_um_crescente` (todos os pontos dentro da área, uma polilinha aberta com pelo menos 20 pontos, e o ponto mais à direita fica a menos de 60% da largura — é um crescente, não um disco); `a_lua_resiste_a_area_impossivel` (largura 0 e NaN não geram NaN). Rodar: FALHAM.
- [ ] **Step 2: Implementar** `lua` (dois arcos: um externo de ~300° e a volta interna menor, como a ☾) e `desenhar_cartao_de_modo`: caixa do ícone à esquerda (quadrado de 0,60 da altura do cartão, `solida` tingida `9785e438`), nome Bold e frase Normal a ~70% do tamanho, empilhados à direita da caixa; frases e cores do Global Constraints; `escolhido` usa `modo_escolhido`. Desenhar os dois cartões em `desenhar_jogo` com `tela.aba_zen`/`aba_classico` e o modo atual do `App`.
- [ ] **Step 3:** fotos `inicio` (Zen escolhido) e `classico` em 360×640 e 1080×2400: as frases cabem inteiras (se não, `escrever` encolhe; se ficar < 70% do nome, aumentar `MODOS_ALTURA` e ajustar Task 2). Mutação: trocar `escolhido` por `!escolhido` → foto mostra o cartão errado destacado (conferir e restaurar). Commit `feat: cartões de modo com ícone e frase`.

### Task 5: Folhas, fim de partida, varredura de glifos e notas

**Files:** Modify: `game/src/render.rs` (folhas Som/Recordes/Créditos e fim usam o cartão novo e `texto`), `docs/notes/visual.md`; Test: varredura de glifos.

**Interfaces:** Consumes: tudo acima.

- [ ] **Step 1: Teste primeiro** `todo_texto_do_jogo_tem_glifo_na_roboto` em `texto.rs`: junta todos os textos fixos do jogo (rótulos, folhas, créditos, frases, avisos de nível; pegar de `cenas.rs`/`render.rs`/`app.rs` por uma função `pub fn textos_fixos() -> Vec<&'static str>` que o builder monta onde os textos já vivem) e confere cada caractere contra a tabela `cmap` das duas fontes (ler com `ttf-parser` como dependência de DEV só do teste, ou `fonttools` fora do teste e um array de pontos de código ausentes comitado — escolher o mais simples e ledgerar). Rodar: FALHA se algum texto usa ♫, ≡ ou outro glifo ausente (corrigir o texto ou desenhar o símbolo).
- [ ] **Step 2:** fotos das 12 cenas em 360×640 e 1080×2400 (lista em `docs/notes/visual.md`): nada cortado, sobreposto ou com quadradinho; folhas e diálogo legíveis. Ajustar o que falhar com mudanças mínimas e ledgerar cada uma.
- [ ] **Step 3:** atualizar `docs/notes/visual.md` (V2 pronta, receitas novas, "só o celular confirma": Roboto no Android e o tempo de abertura com as duas fontes) e a linha da fase 3 no desenho da fase 2. Suíte (`cargo test --workspace` e `tools/`), clippy sem avisos novos. Commit `docs: notas da V2`.

**Fim da V2:** revisão final da branch (self-review se não houver subagente), depois pedir autorização nova ao Thiago para enviar a branch, abrir o PR e rodar o CI Android (as fotos `1-zen-inicio` … `5-classico` passam a mostrar a tela completa); só juntar na `main` depois que ele testar o APK.
