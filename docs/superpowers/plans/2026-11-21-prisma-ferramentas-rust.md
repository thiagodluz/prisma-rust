# Prisma · Ferramentas em Rust Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Os dois scripts que sobraram fora do Rust — o teste do celular virtual (`tools/teste-emulador.sh`, shell, ~300 linhas) e o gerador de imagens (`tools/prepare_assets.py`, Python, ~115 linhas) — viram um único programa Rust, `prisma_tools`, com testes. O comportamento é o mesmo; o que muda é a linguagem (e o teste do celular virtual passa a ter testes de verdade).

**Architecture:** Um pacote novo em `tools/`, **fora** do workspace do jogo (tem o próprio `[workspace]`, `Cargo.lock` e `target/`), para que as dependências dele nunca cheguem ao Cargo antigo do gerador de APK. Um programa, dois subcomandos: `prisma_tools emulador <pasta-do-apk> <pasta-de-saida>` e `prisma_tools imagens <pasta-do-prisma-js> <pasta-do-jogo>`. O subcomando do emulador não tem dependências (só `std::process::Command` para chamar o `adb`); o das imagens usa a biblioteca `image`, ligada por uma "feature" do Cargo (`imagens`), de modo que o CI do emulador compila só a parte leve.

**Tech Stack:** Rust 2021, biblioteca padrão (`std`) e `image` 0.25 (só `png` e `webp`, só na feature `imagens`). Continuam externos: `adb` e `convert` (ImageMagick, já instalado no CI), chamados por `Command`.

**Spec:** `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md` (seção 5.4, imagens, e 5.9, como verificar). Referências do comportamento a preservar, somente leitura: `tools/teste-emulador.sh` e `tools/prepare_assets.py` (ficam no repositório até a Task 4 e a Task 5 provarem a equivalência).

## Global Constraints

- `edition = "2021"` em `tools/Cargo.toml`. O motor e o jogo não mudam; `cargo test --workspace` continua somando **189** (não inclui `tools/`). Testes das ferramentas: `cargo test --manifest-path tools/Cargo.toml`.
- Nenhuma dependência nova no jogo nem no motor. Em `tools/`, a única dependência é `image` (opcional, feature `imagens`, `default-features = false`, `features = ["png", "webp"]`). O subcomando `emulador` precisa compilar com `--no-default-features`.
- O pacote `tools/` não entra no APK: a raiz ganha `exclude = ["tools"]` e `tools/Cargo.toml` tem `[workspace]` próprio. O fluxo `android.yml` só muda no trabalho `emulador` (Task 4).
- Este ambiente não tem `adb` nem emulador: a lógica do emulador é provada com um **celular falso** nos testes (Task 3) e a prova final é o fluxo do GitHub nos dois celulares virtuais (Android 11 x86 e Android 14 x86_64), como nos marcos anteriores. Neste ambiente, `export RUSTFLAGS="-L /tmp/asound"` só é preciso para o jogo, não para `tools/`. Sempre `cd /home/claude/prisma-rust &&` antes de cada comando.
- Mensagens para a pessoa (erros, `OK: ...`, uso) em português simples e **iguais às do script de shell** (o registro do GitHub continua legível do mesmo jeito); comentários em português simples explicando cada conceito novo de Rust numa linha, como nos módulos existentes (Thiago tem conhecimento básico de programação).
- Commits em português (`feat:`, `test:`, `ci:`, `docs:`, `refactor:`) com os trailers de atribuição de sempre. Envio de branch e merge na `main` só com autorização do Thiago.

## Review Focus

1. **Equivalência do teste do emulador:** mesmas fases, mesmas conferências e mesmos limites de espera do shell; termina com código de erro (1) em **qualquer** conferência que falhar; uma linha do registro que não dá para ler **reprova** (nunca "passa em silêncio"). Tasks 2 e 3.
2. **Esperas e comandos presos:** nenhuma espera sem limite (30 s por animação, 15 s para abrir, 60 s por chamada do `adb`); `adb` que não responde vira erro com mensagem, não trava o fluxo. Task 3.
3. **Linhas do registro estranhas:** linhas de outros programas, linhas cortadas, espaços a mais, números enormes ou negativos, a mesma linha repetida, geometria escrita várias vezes (vale a última): nunca dá pânico e a leitura fica igual à do `grep`/`sed` do shell. Task 2.
4. **Foto e resumo na falha:** quando algo falha, o resumo do registro e a foto da tela ainda saem; sem ImageMagick a foto é pulada sem reprovar o teste (como hoje). Task 3.
5. **Imagens:** cada atlas tem exatamente 7 pedras (senão erro claro, com os índices achados); poeira (< 2000 pixels) é ignorada; a ponta de uma pedra que invade a casa vizinha não vira pedaço da outra; a redução é feita em alfa pré-multiplicado (sem borda escura); arquivo de origem ausente ou inválido dá erro claro, sem pânico; as 22 imagens e os 5 ícones saem com os mesmos nomes e tamanhos. Task 5.
6. **Isolamento do APK:** com `tools/` na árvore, o APK continua sendo gerado (o trabalho `apk` do GitHub fica verde) e o `cargo test --workspace` continua em 189. Tasks 1 e 4.

## Escolhas minhas (a confirmar na revisão do plano)

- **Pacote isolado em vez de membro do workspace.** O gerador de APK carrega um Cargo antigo que lê as dependências de **todo** o workspace; uma biblioteca nova com manifesto moderno poderia quebrar o APK. Isolado, o risco some. Custo: `tools/` tem o próprio `Cargo.lock`, e o CI de testes ganha um passo.
- **Um programa com dois subcomandos**, não dois programas: um só lugar para ajudas e testes. A feature `imagens` mantém o compilado do emulador pequeno e rápido.
- **Base64 escrito à mão** (~20 linhas, com os vetores de teste da RFC 4648) em vez de uma biblioteca; a foto continua sendo reduzida pelo `convert` do ImageMagick (como hoje), porque o subcomando do emulador não tem dependências.
- **Os PNG que já estão em `game/assets/` e `game/android_res/` não são regenerados.** O Lanczos do Rust e o do Pillow arredondam diferente, então os pixels não saem idênticos; o programa novo é validado por **tolerância** contra os arquivos atuais (Task 5) e vira a fonte para as próximas vezes. Custo se errado: numa regeneração futura as imagens podem diferir levemente das de hoje.
- **O celular falso entra nos testes** (um simulador pequeno de `adb` + app, com o deslocamento de 24 px da janela que o M1 descobriu), porque a parte difícil do script é o controle de fluxo, e é ela que quebra em CI sem ninguém ver.
- **Isto vem antes do M2.** O M2 acrescenta fases ao teste do emulador; é melhor escrevê-las já em Rust. A Task 6 ajusta o plano do M2 (só a Task 7 dele e as menções ao script).
- Continuam fora do Rust: os dois fluxos do GitHub (YAML) e, no M2, os comandos `keytool`/`apksigner` do passo de assinatura (cola de CI dentro do YAML).

## Mapa de arquivos

- Create: `tools/Cargo.toml`, `tools/src/main.rs`, `tools/src/base64.rs` (T1); `tools/src/registro.rs` e `tools/tests/amostra-logcat.txt` (T2); `tools/src/emulador.rs` (T3); `tools/src/imagens.rs` (T5).
- Modify: `Cargo.toml` da raiz (`exclude`, T1), `.github/workflows/test.yml` (T1), `.github/workflows/android.yml` (T4), `docs/notes/android-build.md` (T4, T5), `game/Cargo.toml` e `game/assets/LICENSE-ART.md` (comentários que citam o script, T5), spec seção 5.4 (T5), plano do M2 (T6).
- Delete: `tools/teste-emulador.sh` (T4), `tools/prepare_assets.py` (T5).

---

### Task 1: Pacote `tools/` isolado, linha de comando e base64

**Files:**
- Create: `tools/Cargo.toml`, `tools/src/main.rs`, `tools/src/base64.rs`
- Modify: `Cargo.toml` (raiz: `exclude = ["tools"]` em `[workspace]`), `.github/workflows/test.yml`

**Interfaces:**
- Produces (usado pelas Tasks 3 a 5):
```rust
// tools/src/main.rs
#[derive(Debug, PartialEq, Eq)]
pub enum Comando { Emulador { apk: PathBuf, saida: PathBuf }, Imagens { origem: PathBuf, jogo: PathBuf } }
pub fn ler_comando(args: &[String]) -> Result<Comando, String>;   // Err = texto de uso, em português
// tools/src/base64.rs
pub fn codificar(bytes: &[u8]) -> String;                          // base64 padrão, com "=" no fim, sem quebras de linha
```
`tools/Cargo.toml`: `[package] name = "prisma_tools"`, `edition = "2021"`, `license = "GPL-3.0-only"`; `[features] default = ["imagens"]` e `imagens = ["dep:image"]`; `image` opcional como nas Global Constraints; tabela `[workspace]` vazia (isola o pacote). `main()` lê `std::env::args()`, chama `ler_comando`, e despacha: `Emulador` → `emulador::rodar_do_programa(apk, saida)` (Task 3; até lá, `main` devolve "ainda não implementado" para os dois, e a compilação passa), `Imagens` → `imagens::preparar` só `#[cfg(feature = "imagens")]` (sem a feature, mensagem clara).

- [ ] **Step 1: Escrever os testes que falham** (módulos `tests`):
  - `ler_comando_entende_os_dois_subcomandos`: `["emulador","apk","resultado"]` → `Comando::Emulador{..}`; `["imagens","/js","game"]` → `Comando::Imagens{..}`.
  - `ler_comando_recusa_o_resto_com_o_uso`: sem argumentos, subcomando desconhecido, falta ou sobra de caminho → `Err` com um texto que cita `emulador` e `imagens`.
  - `base64_bate_com_os_vetores_da_rfc_4648`: `""`→`""`, `"f"`→`"Zg=="`, `"fo"`→`"Zm8="`, `"foo"`→`"Zm9v"`, `"foob"`→`"Zm9vYg=="`, `"fooba"`→`"Zm9vYmE="`, `"foobar"`→`"Zm9vYmFy"`; e os 256 bytes de 0 a 255 dão o mesmo texto que `base64 -w0` do sistema (valor escrito no teste, conferido uma vez com o comando).
- [ ] **Step 2: Rodar e ver falhar:** `cargo test --manifest-path tools/Cargo.toml` → Expected: erro de compilação (`cannot find ... ler_comando`), que conta como RED.
- [ ] **Step 3: Implementar** o pacote, `ler_comando`, `codificar` (alfabeto padrão, grupos de 3 bytes) e o `main`. Criar o passo do CI em `.github/workflows/test.yml`: `cargo test --manifest-path tools/Cargo.toml` depois do `cargo test --workspace`.
- [ ] **Step 4: Rodar e ver passar:** `cargo test --manifest-path tools/Cargo.toml` e `cargo test --manifest-path tools/Cargo.toml --no-default-features` → Expected: todos passam nos dois. `cargo test --workspace` → Expected: continua 189 passando (o `tools/` não entra). **Checagem por mutação:** trocar `=` por `-` no preenchimento e ver a RFC falhar; desfazer.
- [ ] **Step 5: Commit** `feat: pacote tools/ isolado com linha de comando e base64`.

---

### Task 2: Leitura do registro do celular (`registro`, puro)

**Files:**
- Create: `tools/src/registro.rs`, `tools/tests/amostra-logcat.txt`
- Modify: `tools/src/main.rs` (só `mod registro;`)

**Interfaces:**
- Produces (usado pela Task 3):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Linha {
    Pronto { modo: String },                                  // PRISMA_M1 pronto modo=zen
    Tabuleiro { x: i32, y: i32, lado: i32 },                  // PRISMA_M1 tabuleiro x=32 y=612 lado=1016
    Botao { nome: String, x: i32, y: i32 },                   // PRISMA_M1 botao=dica x=540 y=1830
    Toque { x: i32, y: i32 },                                 // PRISMA_M1 toque x=160 y=36
    Dica { a: usize, b: usize },                              // PRISMA_M1 dica a=17 b=18
    Jogada { valida: bool, pontos: u64, nivel: u32 },         // PRISMA_M1 jogada valida=true pontos=60 nivel=1
    AnimacaoTerminou,                                         // PRISMA_M1 animacao_terminou
    Modo { modo: String, pontos: u64 },                       // PRISMA_M1 modo=classico pontos=0
    NovoJogo { modo: String },                                // PRISMA_M1 novo_jogo modo=classico pontos=0
    Fim { pontos: u64, nivel: u32 },                          // PRISMA_M1 fim pontos=4321 nivel=3
}
pub struct Registro { /* linhas em ordem */ }
impl Registro {
    pub fn de(logcat: &str) -> Registro;                      // só olha linhas com "PRISMA_M1 "; o resto é ignorado
    pub fn ultimo_tabuleiro(&self) -> Option<(i32, i32, i32)>;             // (x, y, lado)
    pub fn ultimo_botao(&self, nome: &str) -> Option<(i32, i32)>;          // centro
    pub fn ultimo_toque(&self) -> Option<(i32, i32)>;
    pub fn ultima_dica(&self) -> Option<(usize, usize)>;
    pub fn ultima_jogada(&self) -> Option<(bool, u64, u32)>;               // (valida, pontos, nivel)
    pub fn ultimo_modo(&self) -> Option<(String, u64)>;                    // linha "modo=.. pontos=.."
    pub fn contar_animacoes(&self) -> usize;  pub fn contar_dicas(&self) -> usize;  pub fn contar_toques(&self) -> usize;
    pub fn tem_pronto(&self, modo: &str) -> bool;
    pub fn tem_novo_jogo(&self, modo: &str) -> bool;
}
pub fn centro_da_casa(tabuleiro: (i32, i32, i32), deslocamento: (i32, i32), casa: usize) -> (i32, i32);
pub fn deslocamento(tela: (i32, i32), visto: (i32, i32)) -> (i32, i32);   // tela − visto, nos dois eixos
```
`centro_da_casa` = `(x + dx + (2·coluna + 1)·lado/16, y + dy + (2·linha + 1)·lado/16)` com divisão inteira, igual à conta do shell (`coluna = casa % 8`, `linha = casa / 8`). Uma linha `PRISMA_M1 ...` que não dá para ler (falta campo, número que não é inteiro, nome desconhecido) **não** vira `Linha`; é ignorada (quem espera uma linha, falha por não achá-la).

- [ ] **Step 1: Criar a amostra e escrever os testes que falham.** `tools/tests/amostra-logcat.txt`: ~40 linhas **reais** tiradas do registro do GitHub do M1 (execução `44946466597`: `gh run view --log --job <id>`; guardar só a parte do logcat, da data `11-21 ..:..:..` em diante, sem o prefixo que o `gh` põe na frente; linhas que contêm `PRISMA_M1`), misturadas com linhas de outros programas copiadas do mesmo registro (ex.: `AndroidRuntime`, `WVCdm`). Testes (nomes e o que afirmam):
  - `le_todas_as_linhas_da_amostra_real`: a quantidade de cada tipo bate com o que o `grep -c` dá no arquivo (conferido uma vez à mão e escrito no teste); `ultimo_tabuleiro()` e `ultimo_botao("dica")` devolvem os valores reais da amostra (por exemplo `(10, 194, 301)` e `(160, 517)`).
  - `um_teste_por_tipo_de_linha` (tabela texto → `Linha`) com as 10 linhas do quadro acima.
  - `vale_a_ultima_linha_de_geometria`: dois `tabuleiro`/`botao` seguidos → o segundo.
  - `linhas_estranhas_sao_ignoradas_sem_pane`: tabela com `PRISMA_M1 tabuleiro x=1 y=2` (falta `lado`), `x=1.5`, `x=abc`, `x=99999999999999999999`, `PRISMA_M1 dica a=-1 b=2`, `PRISMA_M1 jogada valida=talvez pontos=1 nivel=1`, `PRISMA_M1 inventada x=1`, linha cortada ao meio, linha vazia, `PRISMA_M1` sozinho, texto com `\r\n` → nenhuma vira `Linha` (ou vira só a parte legítima) e nada dá pânico.
  - `centro_da_casa_bate_com_a_conta_do_shell`: com `(100, 300, 800)` e deslocamento `(0, 28)`: casa 0 → `(150, 378)`, casa 9 → `(250, 478)`, casa 63 → `(850, 1078)`; `lado` que não divide por 16 usa divisão inteira.
  - `deslocamento_e_a_diferenca`: `deslocamento((160, 60), (160, 36)) == (0, 24)`.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test --manifest-path tools/Cargo.toml registro` → Expected: erro de compilação, RED.
- [ ] **Step 3: Implementar** `registro.rs` com `str::split_whitespace` e `strip_prefix`/`parse` (sem biblioteca de expressões regulares), cada linha por um `fn ler_linha(texto: &str) -> Option<Linha>`.
- [ ] **Step 4: Rodar e ver passar:** `cargo test --manifest-path tools/Cargo.toml registro` → Expected: todos passam. **Checagem por mutação:** trocar `ultimo_*` por `primeiro_*` (usar `.find` em vez de `.rfind`) e ver `vale_a_ultima_linha_de_geometria` falhar; trocar `/ 16` por `/ 8` em `centro_da_casa` e ver o teste de centro falhar; desfazer.
- [ ] **Step 5: Commit** `feat: leitura tipada do registro do celular virtual`.

---

### Task 3: O fluxo do teste em Rust, com celular falso (`emulador`)

**Files:**
- Create: `tools/src/emulador.rs`
- Modify: `tools/src/main.rs` (`mod emulador;` e o despacho do subcomando)

**Interfaces:**
- Consumes: `registro::{Registro, centro_da_casa, deslocamento}`, `base64::codificar`, `Comando::Emulador`.
- Produces (usado pela Task 4 e pelo M2):
```rust
pub trait Celular {
    /// Roda `adb <args>` e devolve a saída (bytes). Erro de execução ou passou de 60 s → Err(texto).
    fn adb(&mut self, args: &[&str]) -> Result<Vec<u8>, String>;
    /// Espera `ms` milissegundos (o celular falso só avança um relógio de mentira).
    fn dormir(&mut self, ms: u64);
    /// Reduz a foto para caber no registro: `convert <origem> -resize 270x -colors 48 PNG8:<destino>`.
    /// `Ok(true)` = reduziu; `Ok(false)` = o `convert` (ImageMagick) não existe, e a foto é pulada.
    fn reduzir_foto(&mut self, origem: &Path, destino: &Path) -> Result<bool, String>;
}
/// Implementa `Celular` com `std::process::Command`: programa `adb` (ou o de `$ADB`), limite de 60 s por chamada.
pub struct AdbReal { /* programa, limite */ }
impl AdbReal { pub fn novo() -> AdbReal; pub fn com_programa(programa: &str, limite: Duration) -> AdbReal; }
pub fn rodar(celular: &mut dyn Celular, apk: &Path, saida: &Path) -> Result<(), String>;   // Err(motivo) → o `main` imprime "ERRO: <motivo>", o resumo do registro e a foto da tela e sai com 1
pub fn rodar_do_programa(apk: &Path, saida: &Path) -> i32;  // usa AdbReal; devolve o código de saída (0 ou 1)
```
`rodar` reproduz o script `tools/teste-emulador.sh` **fase por fase, com as mesmas mensagens e os mesmos limites** (leia o script inteiro antes; o resumo abaixo não o substitui): descobrir o pacote em `game/Cargo.toml` (`package_name = "..."`); imprimir versão do Android e processadores; `adb install -r`; **abertura** (`logcat -c`, abrir pelo `monkey`, 15 s, app vivo (`pidof`), `pronto modo=zen`, tabuleiro e botões `zen classico embaralhar dica novo_jogo confirmar` presentes, foto `1-zen-inicio`, **calibração do toque** — tocar em `(tx + lado/2, 60)` na tela, ler `toque`, deslocamento = tela − visto, aceito só com `0 ≤ dx ≤ 100` e `0 ≤ dy ≤ 200`); **Zen** (5 rodadas: tocar `dica` exigindo uma linha `dica` **nova**, arrastar de uma casa à outra com `input swipe … 200`, esperar até 30 s (60 × 0,5 s) por uma `animacao_terminou` nova, app vivo, `jogada valida=true`, pontos maiores que os da rodada anterior, 1 s de pausa; fotos `2-zen-dica`, `3-zen-depois-da-jogada`, `4-zen-fim`); **Clássico** (tocar a aba, `modo=classico pontos=0`, reler o tabuleiro (ele muda de lugar sem o Embaralhar), 5 rodadas com `dica` e dois toques (0,5 s entre eles), foto `5-classico`); **Novo jogo** (`novo_jogo`, foto `6-classico-confirmando`, `confirmar`, linha `novo_jogo modo=classico pontos=0`, app vivo); **volta ao Zen** (`modo=zen` com os pontos do Zen de antes, app vivo, foto `7-zen-de-volta`); resumo do registro (`PRISMA_M1`, `FATAL`, `AndroidRuntime`, `panicked`, `SIGSEGV`, `SIGABRT`, `UnsatisfiedLink`, `dlopen`, `libprisma`, `miniquad`, no máximo 200 linhas) e a linha final `OK: o jogo rodou no celular virtual (Zen e Clássico, toque e arrasto).`. Foto: `exec-out screencap -p` → `saida/tela-<nome>.png`; reduzir com `convert <arquivo> -resize 270x -colors 48 PNG8:<saida>/pequena-<nome>.png`; imprimir `TELA_BASE64_INICIO <nome>`, o base64 numa linha, `TELA_BASE64_FIM`; se o `convert` não existir, pular sem erro. Na falha: guardar `logcat.txt`, tentar a foto `do-erro`, imprimir o resumo e devolver o erro.

- [ ] **Step 1: Escrever o celular falso e os testes que falham** (módulo `tests` de `emulador.rs`). O `CelularFalso` guarda: um relógio virtual (`dormir` soma), um texto de logcat, o deslocamento da janela `(0, 24)`, a tela `320×640`, o estado de um mini-jogo (dois modos, pontos, dica fixa) e uma lista de tudo que o teste mandou (`Vec<String>` com `tap x y`, `swipe ...`, `install`, etc.). Ele responde: `getprop`, `install` (Ok), `logcat -c` (limpa), `logcat -d` (devolve o texto), `shell monkey ...` (o app "abre": escreve `tabuleiro`, `botao` (todos), `pronto`), `shell pidof` (vazio se `app_fechado`), `shell input tap x y` (converte para coordenadas do app **subtraindo o deslocamento**, escreve `toque`; se cair no botão `dica` escreve `dica a=17 b=18`; no tabuleiro seleciona; dois toques vizinhos → `jogada valida=true ...` + `animacao_terminou`; abas e botões mudam modo/escrevem as linhas do M1), `shell input swipe ...` (de casa válida a casa vizinha → jogada), `exec-out screencap -p` (devolve alguns bytes), `shell am force-stop`. Testes:
  - `fluxo_completo_passa_com_o_celular_falso`: `rodar(...)` é `Ok`; a lista de comandos tem 5 `swipe` e os toques do Clássico; **todo** toque/arrasto cai em coordenadas de tela = app + `(0, 24)` (o falso só reage se o teste compensou o deslocamento).
  - `calibracao_acha_o_deslocamento_mesmo_diferente`: com deslocamento `(0, 40)` o fluxo também passa.
  - `falha_com_mensagem_quando_o_toque_na_dica_nao_gera_dica`: falso que ignora toques → `Err` contendo `o toque em 'Dica' não gerou uma dica nova`.
  - `falha_quando_o_app_fecha`: `app_fechado` depois da abertura → `Err` contendo `o app FECHOU sozinho`.
  - `falha_quando_a_animacao_nao_termina`: falso que nunca escreve `animacao_terminou` → `Err` contendo `a animação não terminou em 30 s`, e o relógio virtual mostra que esperou **entre 30 e 31 s** (não menos, não para sempre).
  - `falha_quando_os_pontos_nao_sobem` e `falha_quando_a_jogada_nao_e_valida` (mensagens do shell).
  - `falha_quando_falta_a_linha_pronto` e `falha_quando_falta_a_posicao_de_um_botao`.
  - `foto_e_pulada_sem_imagemagick_e_o_teste_continua`: com o falso devolvendo `Ok(false)` em `reduzir_foto` (sem ImageMagick), `rodar` continua `Ok` e nenhuma linha `TELA_BASE64_INICIO` é impressa.
  - `adb_que_passa_de_60_s_vira_erro`: testado com `AdbReal::com_programa("sleep", 200 ms)` chamado com `["5"]`: devolve `Err` em bem menos de 5 s, sem esperar 60 s de verdade.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test --manifest-path tools/Cargo.toml --no-default-features emulador` → Expected: erro de compilação, RED.
- [ ] **Step 3: Implementar** `Celular`, `AdbReal` (`std::process::Command`, limite por chamada com `try_wait` em laço curto e `kill`), `rodar` (funções pequenas por fase, cada uma devolvendo `Result<_, String>`, sem `unwrap`) e `rodar_do_programa` (imprime e devolve 0/1). O pacote lê `package_name` de `game/Cargo.toml` (caminho relativo ao diretório atual, como o shell).
- [ ] **Step 4: Rodar e ver passar:** `cargo test --manifest-path tools/Cargo.toml --no-default-features` → Expected: todos passam. **Checagem por mutação:** esquecer de somar o deslocamento em `centro_da_casa` dentro de `rodar` e ver `fluxo_completo...` falhar; trocar 60 rodadas de espera por 600 e ver o teste de tempo falhar; desfazer.
- [ ] **Step 5: Commit** `feat: fluxo do teste do celular virtual em Rust, com celular falso`.

---

### Task 4: Ligar no CI e aposentar o shell

**Files:**
- Modify: `.github/workflows/android.yml` (trabalho `emulador`), `docs/notes/android-build.md` (seção do emulador e do resumo)
- Delete: `tools/teste-emulador.sh`

**Mudança no `emulador`:** antes do passo do emulador, `dtolnay/rust-toolchain@stable` e `cargo build --release --manifest-path tools/Cargo.toml --no-default-features`; o passo do emulador roda `tools/target/release/prisma_tools emulador apk resultado` no lugar de `bash tools/teste-emulador.sh apk resultado`; atualizar os comentários que citam o script (incluindo o do ImageMagick).

- [ ] **Step 1: Implementar** as mudanças acima. Antes de enviar: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/android.yml'))"` e `cargo build --release --manifest-path tools/Cargo.toml --no-default-features` local.
- [ ] **Step 2 (com autorização do Thiago para enviar a branch `ferramentas-rust`):** `git push -u origin ferramentas-rust` e `gh workflow run android.yml --ref ferramentas-rust`; acompanhar com `gh api` e ler com `gh run view --log`. Expected: `apk` verde (**prova que `tools/` não atrapalha o APK**) e os dois celulares virtuais verdes, com as **mesmas** linhas `OK: ...` e o mesmo número de fotos do último fluxo bom (`44946466597`: 5 rodadas no Zen, 5 no Clássico, Novo jogo, volta ao Zen); conferir as fotos olhando. O `test.yml` também verde (`testes`).
- [ ] **Step 3: Se algo divergir** (mensagem, espera, foto), ler o registro, corrigir **com um teste do celular falso que reproduza**, reenviar e repetir. Só depois, apagar `tools/teste-emulador.sh` e atualizar o `docs/notes/android-build.md` (script = `prisma_tools emulador`; como rodar os testes dele).
- [ ] **Step 4: Commit** `ci: o teste do celular virtual roda pelo prisma_tools (Rust)`.

---

### Task 5: Imagens em Rust (`imagens`) e fim do Python

**Files:**
- Create: `tools/src/imagens.rs` (compilado só com a feature `imagens`)
- Modify: `tools/src/main.rs` (despacho), `game/Cargo.toml` e `game/assets/LICENSE-ART.md` (os comentários que citam o script passam a citar `prisma_tools imagens`), spec seção 5.4, `docs/notes/android-build.md`
- Delete: `tools/prepare_assets.py`

**Interfaces:**
- Consumes: `Comando::Imagens`, a biblioteca `image` (`RgbaImage`, `imageops::resize` com `FilterType::Lanczos3`).
- Produces:
```rust
pub fn pedras_do_atlas(atlas: &RgbaImage) -> Result<Vec<RgbaImage>, String>;        // 7 recortes, índice 0..=6 (posição na grade como no style.css do JS)
pub fn centralizar(pedra: &RgbaImage, fator: f32) -> RgbaImage;                     // tela transparente 160×160 com a pedra reduzida, centro da caixa visível (alfa ≥ 128) em (80, 80)
pub fn preparar(origem: &Path, jogo: &Path) -> Result<Vec<PathBuf>, String>;        // grava as 22 imagens em <jogo>/assets e os 5 ícones em <jogo>/android_res/mipmap-*/ic_launcher.png; devolve os arquivos
```
Algoritmo e constantes: **idênticos** a `tools/prepare_assets.py` (leia o script inteiro): casas de 418 px na grade 3×3 de 1254; `CELULA_PX = 128`, `TELA_PX = 160`, `MIN_PEDRA = 2000`; `POSICAO` e `ESCALA` por pedra, `ESCALA_ESPECTRO = 1.18`; componentes ligados de alfa > 0 com **vizinhança de 4** (como o `ndimage.label` padrão), recorte pela caixa do componente apagando o que for de outro componente, índice da pedra pelo centro da caixa; redução por `CELULA_PX × ESCALA / 418` em alfa **pré-multiplicado** (multiplicar RGB pelo alfa, reduzir, desfazer) com tamanho `max(1, round(...))`; deslocamento `round(80 − (x0 + x1) / 2)` e idem em y; ícones de 48/72/96/144/192 px (`mdpi`..`xxxhdpi`) reduzidos de `icon-512.png`. Mensagens de erro como as do Python (`<arquivo>: esperava 7 pedras, achei as de índice [..]`).

- [ ] **Step 1: Escrever os testes que falham** (módulo `tests`, atlas e pedras **sintéticos** gerados no teste, sem arquivos):
  - `acha_as_sete_pedras_na_grade`: atlas 1254×1254 com 7 manchas opacas de formas diferentes, uma por casa da grade (0,0)…(0,2) (as duas últimas casas vazias) → `pedras_do_atlas` devolve 7 recortes na ordem do `POSICAO`, cada um com o tamanho da caixa da sua mancha.
  - `ignora_poeira_e_nao_mistura_a_ponta_que_invade_a_casa_vizinha`: uma mancha de 50 pixels (poeira) é ignorada; a ponta da mancha da casa 6 que invade 3 pixels a casa 3 **não** aparece no recorte da pedra 3 (o recorte da 3 só tem pixels da própria mancha).
  - `erra_com_o_indice_das_pedras_achadas_quando_nao_sao_sete`: seis manchas → `Err` contendo `esperava 7 pedras` e os índices.
  - `centralizar_poe_o_centro_visivel_em_80_80`: pedra assimétrica (opaca só num canto do recorte) → a caixa de alfa ≥ 128 do resultado tem centro em `(80, 80)` com diferença de no máximo 1 px; resultado sempre 160×160.
  - `reduzir_em_alfa_pre_multiplicado_nao_escurece_a_borda`: quadrado branco opaco com margem transparente preta, reduzido pela metade → todo pixel com alfa > 0 do resultado tem R, G e B ≥ 250.
  - `preparar_gera_22_imagens_e_5_icones_com_os_nomes_certos` (atlas e ícone sintéticos gravados numa pasta temporária; 7×3 + 1 = 22 arquivos `gem-N.png`, `burst-N.png`, `cross-N.png`, `spectrum.png`, todos 160×160; 5 ícones nos tamanhos certos) e `preparar_com_origem_faltando_ou_invalida_da_erro_claro` (arquivo ausente, arquivo que não é imagem).
  - `confere_com_os_png_de_hoje` (`#[ignore]`; roda com `PRISMA_JS=/home/claude/thiagodluz/prisma cargo test --manifest-path tools/Cargo.toml -- --ignored`): `preparar` numa pasta temporária e compara cada uma das 22 imagens e dos 5 ícones com `game/assets/` e `game/android_res/`: mesmo tamanho; erro médio absoluto por canal ≤ 1,5; ≥ 99% dos pixels com diferença ≤ 12 em todos os canais; caixa visível (alfa ≥ 128) igual com tolerância de 1 px. Se algum arquivo passar do limite: olhar a imagem de diferença **antes** de aceitar ou mexer no limite, e registrar um Ruling com o motivo.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test --manifest-path tools/Cargo.toml imagens` → Expected: erro de compilação, RED. (Se a biblioteca `image` não decodificar algum dos WebP, o `confere_com_os_png_de_hoje` mostra isso; nesse caso parar e perguntar: a saída seria uma dependência a mais, `libwebp`.)
- [ ] **Step 3: Implementar** `imagens.rs` (rotulagem por fila/pilha sobre uma máscara de 1254×1254, sem biblioteca extra) e o despacho do subcomando em `main.rs`.
- [ ] **Step 4: Rodar e ver passar:** `cargo test --manifest-path tools/Cargo.toml` → Expected: todos passam; depois `PRISMA_JS=... cargo test --manifest-path tools/Cargo.toml -- --ignored` → Expected: passa dentro da tolerância; guardar no ledger os números reais (erro médio por arquivo) e **olhar** uma comparação lado a lado de duas pedras e de um ícone. **Checagem por mutação:** trocar a vizinhança de 4 por 8, ou o alfa pré-multiplicado por não pré-multiplicado, e ver o teste correspondente falhar; desfazer.
- [ ] **Step 5: Aposentar o Python:** apagar `tools/prepare_assets.py` e atualizar os quatro lugares que o citam (comentário em `game/Cargo.toml`, `game/assets/LICENSE-ART.md` linha 20, spec 5.4, notas) para `cargo run --manifest-path tools/Cargo.toml -- imagens <pasta-do-prisma-js> game`. Os PNG commitados **não** são regenerados. `cargo test --workspace` → Expected: 189 passando.
- [ ] **Step 6: Commit** `feat: gerador de imagens em Rust (prisma_tools imagens), sem Python`.

---

### Task 6: Fechamento e ajuste do plano do M2

**Files:**
- Modify: `docs/superpowers/plans/2026-11-21-prisma-fase2-m2.md`, `docs/notes/android-build.md` (só se algo novo for aprendido)

- [ ] **Step 1: Ajustar o plano do M2** (só texto): na Task 7 do M2, "estender `tools/teste-emulador.sh`" vira "estender `tools/src/emulador.rs` e `tools/src/registro.rs`": `Registro` ganha as linhas `PRISMA_M2` (`aberto`, `gravado`, `erro_ao_gravar`) com testes na amostra e o `CelularFalso` ganha `force-stop`, reabertura com os dados guardados e `install -r`; os passos de `bash -n` e de ensaio com `sed` somem (os testes do celular falso os substituem); nas Tasks 6 e 7 e no mapa de arquivos, trocar as menções ao script. Commitar `docs: plano do M2 usa o prisma_tools (Rust)`.
- [ ] **Step 2: Integrar na `main`** (com autorização do Thiago): merge de `ferramentas-rust` com `git merge --no-ff -F <arquivo-com-a-mensagem> ferramentas-rust` (o `git merge` não aceita `-F -`), `cargo test --workspace` (189) e `cargo test --manifest-path tools/Cargo.toml`, `git push origin main`; esperar o fluxo Android da `main` verde e a release republicada (o APK é funcionalmente igual ao anterior: **não** precisa de conferência no celular).
- [ ] **Step 3: Fechar:** ledger, apagar a pasta de trabalho deste plano, relatar "Rulings I made" e "Deferred minors". Em seguida, retomar o M2 pela Task 1 (chave de assinatura).
