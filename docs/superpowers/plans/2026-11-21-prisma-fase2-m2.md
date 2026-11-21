# Prisma Fase 2 · M2 (Salvar) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fechar e abrir o app mantém a partida de cada modo (Zen e Clássico), o modo em que o Thiago estava e os recordes; e atualizar o APK por cima, sem desinstalar, também mantém tudo (chave de assinatura fixa).

**Architecture:** Três peças novas e pequenas. `formato` (puro): partida e recordes ⇄ texto JSON, com as mesmas checagens do JS. `salvar` (arquivos): ler/gravar na pasta de dados do app (a do M0b), com gravação segura e quarentena de arquivo estragado. `App` (M1) passa a carregar tudo ao abrir e a gravar nos mesmos momentos do JS. O CI passa a assinar o APK com uma chave fixa (segredo do GitHub) e o teste do celular virtual fecha e reabre o app conferindo os dados.

**Tech Stack:** Rust 2021, Macroquad 0.4.16, `prisma_engine` (não muda), `serde_json` (única dependência nova, só em `game/`), `apksigner`/`zipalign` do Android SDK no CI.

**Spec:** `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md` (seções 5.6, 5.7 e marco M2 da seção 6) e `docs/notes/js-quirks.md` (seção "O que o leitor de saves da Fase 2 precisa repetir do JS"). Referência de comportamento: `app.js` do Prisma em JS 1.4.2, em `/home/claude/thiagodluz/prisma` (somente leitura): `restoreMode` nas linhas ~40–55, `safeRecord` ~187–203, `save` ~205–213, `commitRecords` ~276–290 (chamado em ~594 com `result.earned, result.ended`).

## Global Constraints

- `edition = "2021"`; o motor `prisma_engine` não muda (`cargo test --workspace` hoje soma 189 e todos continuam passando). Única dependência nova: `serde_json`, só em `game/Cargo.toml`. Anti-objetivo: nenhuma regra de jogo nova; a validação de save é do `Game::restore` (tudo ou nada), o leitor só confere formato e tipos.
- Formato dos arquivos igual ao do JS: `mode` `"zen"`/`"classic"` (lê também `"endless"`; grava sempre `"classic"`), campos `camelCase` (`score`, `board`, `ended`, `progressionVersion`, `level`, `levelStartScore`, `progressionOffset`, `cheatsUsed`, `noDefeat`), pedra `{"id":1,"type":"normal|burst|cross|spectrum","color":0..6|null}`; tabuleiro antigo de números soltos (`SavedTile::Legacy`) também é aceito. Assim um save do JS poderá ser importado no futuro.
- Arquivos na pasta de dados do app (`plataforma::pasta_de_dados()`, a do M0b; no Android é o `getFilesDir()`): `zen.json`, `classico.json`, `recordes-zen.json`, `recordes-classico.json`, `modo-ativo.txt` (`zen` ou `classic`). Arquivo ilegível vira `<nome>.corrupt` (cópia) e aquele item recomeça; nunca dá pânico.
- Gravação sempre em duas etapas (arquivo temporário e troca de nome), como o `contador.rs` do M0b.
- Registro do Android (logcat): linhas `PRISMA_M1 ...` do M1 continuam como estão; as novas começam com `PRISMA_M2` (formatos exatos nas Tasks 4 e 5).
- Código específico do Android só compila no CI. Neste M2 não há código Rust novo específico do Android; o que muda no Android é o fluxo do GitHub (assinatura) e o teste do emulador.
- Neste ambiente os testes e a "foto" precisam de `libasound`: `mkdir -p /tmp/asound && ln -sf /usr/lib/x86_64-linux-gnu/libasound.so.2 /tmp/asound/libasound.so` e `export RUSTFLAGS="-L /tmp/asound"`. Foto: `LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a -s "-screen 0 1280x1024x24" cargo run -q -p prisma_game -- ...`. Sempre `cd /home/claude/prisma-rust &&` antes de cada comando.
- **Segredos e chaves:** o acesso aos segredos do GitHub é bloqueado nesta sessão; quem cadastra é o Thiago, pelo site. Nenhuma chave, senha ou keystore entra no git (só a impressão digital pública do certificado). Cópias na sessão são apagadas depois que o Thiago confirmar.
- Textos e comentários em português simples (Thiago tem conhecimento básico de programação): explicar cada conceito novo de Rust numa linha, como nos módulos existentes. Commits em português (`feat:`, `test:`, `ci:`, `docs:`) com os trailers de atribuição de sempre. Merge na `main` e envio para o GitHub só com autorização do Thiago.

## Review Focus

1. **Arquivo estragado ou estranho:** vazio, lixo, JSON cortado no meio, BOM no começo, arquivo enorme; JSON válido com forma errada (modo desconhecido ou trocado — `zen.json` com `"classic"` —, número fracionário/negativo/enorme/com expoente, booleano que não é `true`/`false`, pedra de tipo desconhecido, tabuleiro 7×8, ids repetidos). Nunca dá pânico; o arquivo ruim é copiado para `.corrupt` (o original fica intacto), só aquele modo recomeça, o outro modo e os recordes não mudam. Testes nas Tasks 2, 3 e 4.
2. **Pasta indisponível ou escrita que falha** (pasta apagada, disco cheio, somente leitura): o jogo continua jogável sem salvar, sem pânico, e uma gravação que falha nunca destrói o save anterior. Tasks 3 e 4.
3. **Momento da gravação:** salvo logo após cada jogada válida (antes da animação, como o JS), após Novo jogo/Jogar novamente, após Embaralhar e ao trocar de modo; fechar o app no meio da animação reabre no tabuleiro final da jogada com os pontos e o nível do motor; partida acabada (Clássico) reabre com o cartão de fim e sem aceitar jogadas. Task 4.
4. **Recordes:** nunca diminuem (Novo jogo, trocar de modo e save estragado não os mexem); `jogada` guarda o maior ganho de uma jogada; `terminadas` só conta partidas do Clássico e só uma vez; número perto de 2^53 funciona; recorde de um modo não vaza para o outro; durante a animação o painel continua sem adiantar o MELHOR (regra do M1). Tasks 2 e 4.
5. **Modo ativo:** reabre no modo em que parou; `modo-ativo.txt` ausente ou inválido → Zen. Tasks 3 e 4.
6. *(CI e aparelho)* **Assinatura fixa:** dois APKs de execuções diferentes têm a mesma impressão digital; com o segredo ausente o CI falha com mensagem clara e não publica APK assinado com outra chave; atualizar por cima no celular mantém os dados. Tasks 6 e 8.

## Escolhas minhas (a confirmar na revisão do plano)

- **Quem faz a chave:** eu gero na sessão (`keytool`, RSA 2048, validade 10000 dias, alias `prisma`, senha aleatória igual para o arquivo e para a chave) e entrego ao Thiago como arquivos. Ele cadastra dois segredos no GitHub (`PRISMA_RUST_KEYSTORE_BASE64` e `PRISMA_RUST_KEYSTORE_PASSWORD`; nomes diferentes dos do Prisma em JS) e guarda a cópia de segurança fora do GitHub (a decisão "guardar a chave" da seção 10 da spec). Depois eu apago as minhas cópias. Custo se errado: quem tiver o arquivo poderia assinar atualizações do app dele; para um app de teste instalado à mão o risco é baixo, e trocar de chave exige reinstalar.
- **Como assinar:** `cargo quad-apk build --nosign` e depois `zipalign` + `apksigner sign` no próprio executor do GitHub (o Android SDK dele já traz as duas). O CI confere que a impressão digital do certificado é a de `tools/assinatura-sha256.txt` (pública, no git); se o segredo faltar, falha.
- **Versão do app:** `version_code` passa a ser o número da execução do GitHub (monotônico), para o Android nunca ver "versão menor". O `versionName` continua `0.1.0`.
- **Android mínimo 18 / alvo 31 ficam como estão** (é o que o APK declara hoje) até o M4: não afetam salvar, e subir o alvo para 35 faz a janela ocupar a tela inteira (inclusive a barra de status), o que muda o layout e o teste do emulador; é melhor tratar junto com o polimento.
- Um arquivo por modo e por recorde (como o JS, que usa uma chave por modo); `modo-ativo.txt` guarda só o nome do modo.
- Recordes guardam os quatro números do JS (maior pontuação, maior nível, maior ganho numa jogada, partidas terminadas), mas a tela só mostra MELHOR; a tela de recordes fica para o M4.
- Gravação nos mesmos momentos do JS (logo após a jogada), não só ao sair do app: o Android pode matar o app a qualquer hora.
- Fora do M2: som (M3), vibração e partículas (M4), tela de recordes, estilo "Anterior", transferência de saves do JS.

## Mapa de arquivos

- Create `game/src/formato.rs` (T2), `game/src/salvar.rs` (T3), `tools/assinatura-sha256.txt` (T1/T6).
- Modify `game/Cargo.toml` (T2, T6), `game/src/app.rs` (T4), `game/src/main.rs` (T2 `mod`, T5), `.github/workflows/android.yml` (T6), `tools/teste-emulador.sh` (T7), `docs/notes/android-build.md` (T6, T8).

---

### Task 1: Chave de assinatura (parte humana, sem código)

**Files:**
- Create: `tools/assinatura-sha256.txt`

**Interfaces:**
- Produces (usado pelas Tasks 6 e 8): segredos do GitHub `PRISMA_RUST_KEYSTORE_BASE64` (o `.keystore` em base64, uma linha) e `PRISMA_RUST_KEYSTORE_PASSWORD`; alias fixo `prisma` (no workflow); `tools/assinatura-sha256.txt` com **uma linha**: o SHA-256 do certificado em minúsculas, sem dois-pontos, no mesmo formato que o `apksigner verify --print-certs` imprime.

- [ ] **Step 0: Criar a branch** `git checkout -b fase2-m2` a partir da `main` (que já tem este plano commitado).
- [ ] **Step 1: Gerar a chave** na sessão, em `/tmp/chave/` (fora do repositório): `keytool -genkeypair -v -keystore prisma-rust.keystore -alias prisma -keyalg RSA -keysize 2048 -validity 10000 -storepass <senha> -keypass <senha> -dname "CN=Prisma Rust, O=Thiago Luz, C=BR"` com senha aleatória de 24 caracteres (`openssl rand -base64 18`).
- [ ] **Step 2: Conferir e registrar a impressão digital:** `keytool -list -v -keystore /tmp/chave/prisma-rust.keystore -storepass <senha>` → Expected: uma linha `SHA256: AA:BB:...`; gravar em `tools/assinatura-sha256.txt` o valor em minúsculas e sem `:` (`tr -d ':' | tr 'A-F' 'a-f'`), 64 caracteres.
- [ ] **Step 3: Entregar ao Thiago** com `SendUserFile` (três arquivos): `prisma-rust.keystore` (cópia de segurança, a guardar fora do GitHub), `senha.txt` (a senha) e `segredo-keystore-base64.txt` (`base64 -w0` do keystore). Explicar em poucas linhas, no celular: GitHub → repositório `prisma-rust` → Settings → Secrets and variables → Actions → New repository secret; criar os dois segredos com os nomes exatos acima (colar o conteúdo dos arquivos de texto) e guardar o `.keystore` e a senha num lugar dele (nuvem pessoal ou gerenciador de senhas). **Não esperar:** seguir para a Task 2 enquanto ele cadastra; a Task 6 é quem precisa dos segredos.
- [ ] **Step 4: Commit** só da impressão digital: `ci: impressão digital da chave de assinatura do APK` (arquivo `tools/assinatura-sha256.txt`). As cópias de `/tmp/chave/` são apagadas na Task 6, depois que o CI provar que os segredos funcionam.

---

### Task 2: Formato dos arquivos (`formato`, puro)

**Files:**
- Create: `game/src/formato.rs`
- Modify: `game/Cargo.toml` (`serde_json = "1"` em `[dependencies]`), `game/src/main.rs` (só `mod formato;`)

**Interfaces:**
- Consumes: `prisma_engine::{SavedGame, SavedMode, SavedTile, TileKind}` (campos públicos; `SavedTile::{Legacy(i64), Full{id: i64, kind: TileKind, color: Option<i64>}}`).
- Produces (usado pelas Tasks 3 e 4):
```rust
pub fn partida_para_json(salvo: &SavedGame) -> String;               // sempre com mode "zen"/"classic" (nunca "endless")
pub fn json_para_partida(texto: &str) -> Result<SavedGame, String>;  // Err = motivo curto em português
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recordes { pub pontuacao: u64, pub nivel: u32, pub jogada: u64, pub terminadas: u32 }
impl Recordes {
    pub fn zerados() -> Recordes;                                   // nivel = 1, o resto 0 (como o JS)
    /// Depois de uma jogada: sobe pontuação, nível e jogada se forem maiores; `terminou` soma 1 em
    /// `terminadas` (quem chama só passa `true` no Clássico). Devolve se algo mudou.
    pub fn registrar(&mut self, pontos: u64, nivel: u32, ganho_da_jogada: u64, terminou: bool) -> bool;
    /// Parado: só sobe `pontuacao` e `nivel` para o placar visto agora (o `hud()` do JS). Devolve se mudou.
    pub fn alcancar(&mut self, pontos: u64, nivel: u32) -> bool;
}
pub fn recordes_para_json(recordes: &Recordes) -> String;            // {"bestScore":..,"bestLevel":..,"bestMove":..,"finished":..}
pub fn json_para_recordes(texto: &str) -> Result<Recordes, String>;  // Err só se não for um objeto JSON; campo ruim vale 0 (nivel 1)
```
Regras de `json_para_partida` (de `js-quirks.md`): recusa o save **inteiro** se o JSON não for um objeto; `mode` fora de `zen`/`classic`/`endless`; número que não seja inteiro (`1.5`, `1e3`, texto) em `score`, `level`, `levelStartScore`, `progressionOffset`, `progressionVersion`, `id` ou `color`; `cheatsUsed`/`noDefeat`/`ended` presentes e que não sejam `true`/`false` (ausentes valem `false`); pedra com `type` desconhecido ou `color` que não seja inteiro (nem `null`, só aceito no `spectrum`); `board` que não seja lista de listas (a forma 8×8 é conferida pelo `Game::restore`). Campos extras são ignorados. `progressionVersion`, `level`, `levelStartScore` e `progressionOffset` ausentes viram `None`. `json_para_recordes`: cada campo que não seja inteiro seguro (0 a 2^53 − 1) vale 0 (`bestLevel` mínimo 1), como o `safe` do JS; `bestLevel` e `finished` acima de `u32::MAX` também valem 0.

- [ ] **Step 1: Escrever os testes que falham** (módulo `tests` de `formato.rs`; ajudante que monta um `SavedGame` de um `Game::with_random(lcg_random(7))` com `to_saved()`). Nomes e o que afirmam:
  - `ida_e_volta_da_partida_e_identica`: para 20 jogos (sementes 1–20, com algumas jogadas) `json_para_partida(&partida_para_json(&salvo)) == Ok(salvo)`, e um `Game` novo que faz `restore` do resultado fica com `board`, `score`, `level` e `level_start_score` iguais.
  - `modo_classico_e_gravado_como_classic_e_endless_e_lido`: o texto gravado tem `"mode":"classic"`; um JSON à mão com `"endless"` vira `SavedMode::Endless`.
  - `le_um_save_do_js_com_pecas_de_todos_os_tipos` (JSON escrito à mão no formato do JS, com `burst`, `cross`, `spectrum` com `"color":null`) e `le_tabuleiro_antigo_de_numeros` (`Legacy`).
  - `recusa_o_save_inteiro_quando` (uma tabela de textos ruins, cada um com uma mensagem não vazia): texto vazio; `"lixo"`; JSON cortado; `[]`; BOM (`\u{feff}`) antes do `{`; modo `"hard"`; `"score": 12.5`, `"score": 1e3`, `"score": "10"`, `"level": -1.0`; `"ended": 1`; `"cheatsUsed": "true"`; pedra `"type":"gelo"`; `"color": 1.5`; `"id": "a"`; `"board": 5`.
  - `numeros_grandes_nao_dao_pane`: `"score": 9007199254740991` lê; `"score": 99999999999999999999999` recusa (não cabe em `i64`) sem pânico; `"score": -5` lê como `-5` (quem decide é o `restore`).
  - `campos_ausentes_e_extras`: sem `progressionVersion`/`level`/... → `None`; sem os três booleanos → `false`; um campo desconhecido é ignorado.
  - `recordes_ida_e_volta_e_leitura_gentil`: ida e volta igual; `{"bestScore":-1,"bestLevel":0,"bestMove":2.5,"finished":"x"}` → `Recordes { pontuacao: 0, nivel: 1, jogada: 0, terminadas: 0 }`; `{"bestScore":9007199254740991}` lê; `"lixo"` → `Err`.
  - `registrar_sobe_e_nunca_desce`: `registrar(500, 2, 120, false)` em zerados → `true` e `{500,2,120,0}`; `registrar(100, 1, 30, false)` → `false` e nada muda; `registrar(900, 2, 50, true)` → `terminadas == 1`; `terminou == false` nunca soma.
  - `alcancar_so_mexe_em_pontuacao_e_nivel`: `alcancar(700, 3)` sobe os dois, não mexe em `jogada` nem `terminadas`; valores menores → `false`.
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game formato` → Expected: erro de compilação (`cannot find ... json_para_partida`), que conta como RED.
- [ ] **Step 3: Implementar** o módulo com as assinaturas e regras acima, usando `serde_json::Value` (ler) e `serde_json::json!`/`to_string` (gravar), sem `derive`. Um ajudante pequeno `inteiro(valor: &Value, nome: &str) -> Result<i64, String>` (usa `Value::as_i64`, que já recusa fracionário e expoente) para não repetir a conferência.
- [ ] **Step 4: Rodar e ver passar:** `cargo test -p prisma_game formato` → Expected: todos passam. **Checagem por mutação:** trocar `as_i64` por `as_f64().map(|n| n as i64)` e ver `recusa_o_save_inteiro_quando` falhar; trocar `>` por `>=` no `registrar` e ver o teste de recordes falhar; desfazer.
- [ ] **Step 5: Commit** `feat: formato JSON da partida e dos recordes`.
- [ ] **Step 6 (com a autorização do Thiago para enviar a branch `fase2-m2`):** `git push -u origin fase2-m2` e `gh workflow run android.yml --ref fase2-m2`; acompanhar com `gh api`. Expected: o trabalho `apk` termina verde. Isso prova cedo que a ferramenta antiga do APK consegue ler a dependência nova (`serde_json`). Se falhar por causa do formato do manifesto de alguma dependência (por exemplo "feature `edition2024` is required"): fixar a versão que funciona com `=x.y.z` em `game/Cargo.toml`, anotar um Ruling no registro e repetir; se nenhuma versão servir, parar e perguntar (a alternativa seria escrever o JSON à mão, sem biblioteca).

---

### Task 3: Arquivos (`salvar`)

**Files:**
- Create: `game/src/salvar.rs`
- Modify: `game/src/main.rs` (só `mod salvar;`)

**Interfaces:**
- Consumes: `formato::{partida_para_json, json_para_partida, recordes_para_json, json_para_recordes, Recordes}`; `prisma_engine::{Game, Mode, SavedMode}` (`Game::restore(&SavedGame) -> bool`, `Game::to_saved()`, `Game::new_game(Mode)`).
- Produces (usado pelas Tasks 4 e 5):
```rust
pub enum Origem { Novo, Arquivo, Estragado(String) }   // de onde veio a partida aberta
#[derive(Clone, Debug)]
pub struct Salvador { /* pasta: Option<PathBuf> */ }
impl Salvador {
    pub fn em(pasta: PathBuf) -> Salvador;
    pub fn nenhum() -> Salvador;                        // não lê nem grava nada (testes, fotos, pasta indisponível)
    pub fn carregar_partida(&self, modo: Mode, jogo: &mut Game) -> Origem;   // restaura, ou deixa o jogo em new_game(modo)
    pub fn gravar_partida(&self, jogo: &Game) -> Result<(), String>;         // no arquivo do modo do próprio jogo
    pub fn carregar_recordes(&self, modo: Mode) -> Recordes;                 // sem arquivo ou estragado: zerados
    pub fn gravar_recordes(&self, modo: Mode, recordes: &Recordes) -> Result<(), String>;
    pub fn carregar_modo_ativo(&self) -> Mode;                               // ausente ou inválido: Zen
    pub fn gravar_modo_ativo(&self, modo: Mode) -> Result<(), String>;
    pub fn tem_pasta(&self) -> bool;
}
```
Regras: nomes de arquivo fixos (Global Constraints). Arquivo maior que 1 MiB conta como estragado sem ser lido inteiro. Estragado (não lê como texto, JSON recusado, modo diferente do esperado — `endless` vale como Clássico —, ou `restore` devolve `false`) → copia para `<nome>.corrupt` (se já existir, sobrescreve) e devolve `Origem::Estragado(motivo)` com o jogo em `new_game(modo)`. Sem arquivo → `Origem::Novo` com `new_game(modo)`. `Salvador::nenhum()` sempre `Origem::Novo`, e as gravações devolvem `Ok(())` sem fazer nada. Gravar: cria a pasta se faltar, escreve `<nome>.tmp` e troca o nome (como `contador::registrar_abertura`); falha devolve `Err(texto)` e **não toca** no arquivo antigo. Testes usam pastas de verdade em `std::env::temp_dir()` com nome único (processo + contador) e apagam no fim.

- [ ] **Step 1: Escrever os testes que falham** (módulo `tests`; ajudante `pasta_de_teste()`):
  - `partida_gravada_volta_igual`: jogo com semente, 3 jogadas, `gravar_partida`; um `Game::new()` recebe `carregar_partida(Mode::Zen, ...)` → `Origem::Arquivo` e `board`, `score`, `level`, `ended` iguais; idem no Clássico (arquivo `classico.json`).
  - `sem_arquivo_comeca_jogo_novo` (`Origem::Novo`, `ended == false`, `score == 0`) e `nenhum_nao_cria_nada`: `Salvador::nenhum()` não cria pasta nem arquivo e `gravar_*` devolve `Ok`.
  - `arquivo_estragado_vira_copia_corrupt`: para cada texto da lista (vazio, `lixo`, JSON cortado, modo `"hard"`, `zen.json` contendo `"classic"`, tabuleiro 7×8, pedra com id repetido, 2 MiB de `x`) → `Origem::Estragado(_)`, o jogo está em partida nova do modo pedido, `zen.json.corrupt` existe com **exatamente** o texto original, e o outro modo não foi mexido.
  - `endless_antigo_vale_como_classico`: `classico.json` com `"mode":"endless"` e um tabuleiro válido restaura.
  - `gravar_nao_destroi_o_arquivo_antigo_se_falhar`: grava uma partida; troca `zen.json.tmp` por uma **pasta** de mesmo nome (a escrita do temporário falha); `gravar_partida` devolve `Err` e `zen.json` continua lendo a partida antiga.
  - `pasta_impossivel_devolve_erro_sem_pane`: pasta dentro de um **arquivo** → `gravar_*` devolvem `Err`, `carregar_*` devolvem `Novo`/zerados/Zen.
  - `recordes_ida_volta_e_estragado`: ida e volta; texto `lixo` → zerados e `recordes-zen.json.corrupt` criado; `recordes-classico.json` separado do Zen.
  - `modo_ativo`: grava `Classic` e lê `Classic`; arquivo ausente, vazio, `Zen\n` com espaços e `qualquer coisa` → `Zen` (`zen`/`classic`, com `trim`, e só esses dois nomes valem).
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game salvar` → Expected: erro de compilação (`cannot find ... Salvador`), que conta como RED.
- [ ] **Step 3: Implementar** `Origem` e `Salvador` em `game/src/salvar.rs`. Reaproveitar a ideia de gravação em duas etapas do `contador.rs` (não importar do `contador`, que é do teste do M0b; copiar a técnica num `fn gravar_com_seguranca(pasta, nome, texto)` privado). Ler com `fs::metadata` antes (limite de 1 MiB) e `fs::read` + `String::from_utf8` (bytes que não são texto = estragado).
- [ ] **Step 4: Rodar e ver passar:** `cargo test -p prisma_game salvar` → Expected: todos passam. **Checagem por mutação:** tirar a cópia para `.corrupt` e ver `arquivo_estragado_vira_copia_corrupt` falhar; gravar direto em `zen.json` (sem o temporário) e ver `gravar_nao_destroi_o_arquivo_antigo_se_falhar` falhar; desfazer.
- [ ] **Step 5: Commit** `feat: ler e gravar partida, recordes e modo ativo em arquivos`.

---

### Task 4: O `App` carrega e grava

**Files:**
- Modify: `game/src/app.rs`

**Interfaces:**
- Consumes: `Salvador`, `Origem`, `Recordes` (Tasks 2 e 3); do M1: `App`, `jogar`, `confirmar_novo_jogo`, `embaralhar`, `trocar_de_modo`, `painel`, `registro`.
- Produces (usado pela Task 5):
```rust
impl App {
    pub fn abrir(salvador: Salvador) -> App;   // lê modo ativo, as duas partidas e os dois recordes; começa no modo ativo
    pub fn novo() -> App;                      // como no M1, mas com Salvador::nenhum() (testes e cenas)
    pub fn com_sementes(zen: u32, classico: u32) -> App;   // idem, sem disco
    pub fn recordes(&self, modo: Mode) -> Recordes;        // para testes e para o painel
}
```
Linhas de registro novas (texto exato; `<m>` é `zen` ou `classico`, como `nome_do_modo` do M1):
`PRISMA_M2 aberto modo=<m> zen=<origem>:<pontos> classico=<origem>:<pontos> melhor_zen=<n> melhor_classico=<n>` (uma vez, na abertura; `<origem>` é `novo`, `arquivo` ou `estragado`), `PRISMA_M2 gravado modo=<m> pontos=<n>` (a cada gravação da partida que deu certo) e `PRISMA_M2 erro_ao_gravar <texto>` (cada falha, sem repetir a mesma linha em sequência).
Comportamento: o campo `melhor: [u64; 2]` do M1 vira `recordes: [Recordes; 2]` e a `Painel.melhor` mostra `recordes[modo].pontuacao` (parado, `max` com o placar atual, como no M1). Grava (partida e modo ativo; recordes quando mudarem) **logo depois** de: cada `play` válido (em `jogar`, com `Recordes::registrar(pontos, nivel, resultado.earned, resultado.ended && modo == Clássico)`), `confirmar_novo_jogo`, `embaralhar` manual que funcionou e `trocar_de_modo` (só o modo ativo). Jogada inválida não grava. `Recordes::alcancar(placar, nível)` roda na abertura (para os dois modos) e ao trocar de modo, como o `hud()` do JS, e grava se mudou; parado, o painel mostra `max(recordes.pontuacao, placar)`, igual ao M1. Durante a animação o painel mostra o MELHOR **de antes da jogada** (guardar o valor antes de registrar). Falha de gravação nunca interrompe o jogo.

- [ ] **Step 1: Escrever os testes que falham** (módulo `tests` de `app.rs`, com `Salvador::em(pasta_de_teste)`; ajudantes para jogar uma jogada boa até o fim da animação, que o M1 já tem):
  - `fechar_e_abrir_retoma_a_partida_de_cada_modo`: abre, joga 3 jogadas no Zen, troca para o Clássico (partida nova), joga 2; `App::abrir` de novo na mesma pasta → começa no Clássico, `jogo()` com o mesmo `board` e `score`; ao tocar na aba Zen o Zen tem o `board` e o `score` de antes; o registro de abertura tem `zen=arquivo:<pontos>` e `classico=arquivo:<pontos>`.
  - `grava_logo_apos_a_jogada_antes_da_animacao_acabar`: depois de `tocar` a troca válida, **sem** chamar `avancar`, reabrir a pasta já dá o tabuleiro final e os pontos do motor (`jogo().score`), e há uma linha `PRISMA_M2 gravado modo=zen pontos=<n>`.
  - `jogada_invalida_nao_grava`: arquivo (data de modificação) e registro não mudam.
  - `novo_jogo_embaralhar_e_troca_de_modo_gravam`: cada um deixa o arquivo igual ao estado do jogo (comparar `partida_para_json(&jogo().to_saved())` com o conteúdo do arquivo); `modo-ativo.txt` muda ao trocar de aba.
  - `partida_acabada_reabre_com_o_cartao_de_fim`: um `Game` do Clássico com `jogo.ended = true` (campo público; o `restore` do motor respeita `ended` no Clássico) é gravado com `Salvador::gravar_partida`; `App::abrir` na mesma pasta → `modo()` é o Clássico (grave também o modo ativo), `painel().fim.is_some()`, `tocar` no tabuleiro não faz jogada (o registro não ganha `jogada`) e `Dica` não faz nada.
  - `arquivo_estragado_de_um_modo_nao_afeta_o_outro_nem_os_recordes`: estraga só `classico.json`; reabrir → `classico=estragado:0`, partida nova no Clássico, Zen e `recordes()` intactos, e existe `classico.json.corrupt`.
  - `recordes_sobem_e_sobrevivem`: depois de várias jogadas `recordes(Zen)` tem `pontuacao == jogo().score` (parado), `nivel`, `jogada > 0`; Novo jogo e trocar de modo não os reduzem; reabrir mostra o mesmo `Painel.melhor`; os recordes do Zen não aparecem no Clássico.
  - `terminadas_so_conta_no_classico_e_uma_vez`: uma partida do Clássico que acaba soma 1 em `recordes(Classic).terminadas`; reabrir e `avancar` muitas vezes não soma de novo; no Zen nunca soma.
  - `melhor_nao_adianta_durante_a_animacao`: depois de uma jogada que bate o recorde, com a animação em andamento o `painel().melhor` ainda é o de antes; ao acabar, é o novo (o teste do M1 continua valendo).
  - `sem_pasta_o_jogo_funciona_sem_salvar`: `App::abrir(Salvador::em(pasta dentro de um arquivo))` joga 3 jogadas sem pânico; o registro tem `PRISMA_M2 erro_ao_gravar ...` (uma vez só, não uma por jogada) e nenhuma linha `gravado`.
  - Os testes do M1 continuam passando (`App::novo()` e `com_sementes` não tocam em disco).
- [ ] **Step 2: Rodar e ver falhar:** `cargo test -p prisma_game app::` → Expected: erro de compilação (`cannot find ... abrir`/`recordes`), que conta como RED.
- [ ] **Step 3: Implementar** em `app.rs`: campo `salvador: Salvador`, troca de `melhor` por `recordes`, `abrir`, e as chamadas de gravar nos pontos descritos (um ajudante privado `gravar_tudo(&mut self)` que grava partida do modo atual, modo ativo e recordes se mudaram, e registra `gravado`/`erro_ao_gravar`). `abrir` usa `Salvador::carregar_modo_ativo`, `carregar_partida` para os dois jogos (o do modo ativo por último não importa) e `carregar_recordes`; recordes são "alcançados" com o placar carregado (`alcancar`).
- [ ] **Step 4: Rodar e ver passar:** `cargo test -p prisma_game app::` e depois `cargo test --workspace` → Expected: tudo passa (189 + os novos). **Checagem por mutação:** mover a gravação para o fim da animação e ver `grava_logo_apos_a_jogada_antes_da_animacao_acabar` falhar; contar `terminadas` também no Zen e ver o teste de `terminadas` falhar; trocar `max` por atribuição direta em `registrar` e ver `recordes_sobem_e_sobrevivem` falhar; desfazer.
- [ ] **Step 5: Commit** `feat: o jogo grava e retoma partida, modo e recordes`.

---

### Task 5: Ligar no programa (`main.rs`) e foto no PC

**Files:**
- Modify: `game/src/main.rs`

**Interfaces:**
- Consumes: `App::abrir`, `Salvador`, `plataforma::pasta_de_dados() -> Result<PathBuf, String>`.
- Produces: argumento `--dados <pasta>` (PC: usar essa pasta em vez da padrão, para testar sem mexer nos dados reais) e a função `fn dados_pedidos(args: &[String]) -> Option<String>`. Em `rodar_jogo`: sem `--cena` e sem `--screenshot`, `App::abrir(Salvador::em(pasta))` com a pasta de `--dados` ou de `plataforma::pasta_de_dados()`; se a pasta não puder ser achada, `info!("PRISMA_M2 sem_pasta: {}", erro)` e `Salvador::nenhum()`. Regra para a foto: com `--cena`, ou com `--screenshot` **sem** `--dados`, nunca lê nem grava (`App::novo`/cena), para a foto não depender dos dados do Thiago nem mexer neles; com `--screenshot` **e** `--dados` abre essa pasta (para fotografar um save restaurado). O modo foto não toca nem avança o jogo, então só a abertura pode gravar (por exemplo, subir o recorde), e a pasta é de teste.

- [ ] **Step 1: Escrever o teste que falha** (em `main.rs`): `dados_pedidos_le_o_argumento_dados` (`--dados /tmp/x` → `Some("/tmp/x")`; `--dados` sem valor → `None`; sem argumento → `None`). Rodar `cargo test -p prisma_game dados_pedidos` → Expected: erro de compilação, RED.
- [ ] **Step 2: Implementar** `dados_pedidos` e a ligação descrita; o registro `PRISMA_M2 aberto ...` já sai pelo `app.registro()` do laço (Task 4). `cargo test --workspace` → Expected: tudo passa.
- [ ] **Step 3: Fotos no PC** (conferir **olhando** as imagens): (a) pasta vazia: `--dados /tmp/m2-vazia --screenshot /tmp/m2-novo.png` → tabuleiro inicial do Zen, MELHOR 0; (b) preparar a pasta `/tmp/m2-dados` com um teste auxiliar `#[ignore]` ou um pequeno código de teste que grave, com `Salvador`, um Zen com 3 jogadas e recordes `{pontuacao: 4321, nivel: 3, ...}` (documentar como foi gerado no ledger) e fotografar com `--dados /tmp/m2-dados --screenshot /tmp/m2-retomado.png` → tabuleiro e pontos do save, MELHOR 4.321; (c) com `zen.json` cheio de lixo: a foto mostra um Zen novo, `zen.json.corrupt` existe e o registro do PC mostra `estragado`; (d) `--cena fim` e `--tela diagnostico` continuam como antes.
- [ ] **Step 4: Commit** `feat: o programa abre e grava na pasta de dados (e --dados no PC)`.

---

### Task 6: APK assinado com a chave fixa no CI

**Files:**
- Modify: `.github/workflows/android.yml`, `game/Cargo.toml` (`version_code = 1` em `[package.metadata.android]`), `docs/notes/android-build.md` (seção "Assinatura")

**Interfaces:**
- Consumes: segredos `PRISMA_RUST_KEYSTORE_BASE64` e `PRISMA_RUST_KEYSTORE_PASSWORD` e `tools/assinatura-sha256.txt` (Task 1). O Thiago precisa tê-los cadastrado.
- Produces: o artefato `prisma-debug-apk` e a release `apk-mais-recente` passam a conter o APK **assinado com a chave fixa** (o emulador, o artefato e a release usam o mesmo arquivo).

Desenho do trabalho `apk` (no executor, depois de gerar): (1) trocar `cargo quad-apk build` por `cargo quad-apk build --nosign` e, antes dele, `sed -i "s/^version_code = .*/version_code = ${GITHUB_RUN_NUMBER}/" game/Cargo.toml` (só no executor, sem commitar); **não presumir o nome** do APK sem assinatura: listar `target/android-artifacts/debug/apk/` no primeiro registro e ajustar o passo seguinte ao nome real (os arquivos saem do Docker como `root`, então só leia deles e grave o resultado numa pasta nova); (2) passo "Assinar o APK": falha com mensagem clara (`::error::` apontando as instruções de `docs/notes/android-build.md`) se algum dos dois segredos estiver vazio; decodifica o keystore para um arquivo temporário fora do repositório, acha `zipalign` e `apksigner` em `$ANDROID_HOME/build-tools/*/` (o mais novo), roda `zipalign -p -f 4 <sem-assinatura> <alinhado>`, `apksigner sign --ks <keystore> --ks-key-alias prisma --ks-pass env:... --out <final>.apk <alinhado>`, `apksigner verify --print-certs -v <final>.apk`, compara o `SHA-256 digest` impresso com `tools/assinatura-sha256.txt` (falha se for diferente), apaga o keystore (`rm -f`, também com `if: always()` num passo de limpeza) e grava o APK assinado em `apk-assinado/prisma-rust.apk` (pasta nova do executor); os passos "Conferir o que tem dentro do APK", "Guardar o APK" (artefato `prisma-debug-apk`) e "Publicar na release" passam a usar `apk-assinado/*.apk`, e o emulador continua baixando o artefato e achando `*.apk` nele (o artefato só tem o assinado); (3) no passo informativo, acrescentar `aapt2 dump badging <apk> | head -5` (de `$ANDROID_HOME/build-tools/*/`) para registrar `versionCode`, `sdkVersion` e `targetSdkVersion`.

- [ ] **Step 1: Pré-condição:** o Thiago confirmou (na conversa) que cadastrou os dois segredos. Se ainda não, parar e lembrar com os passos curtos da Task 1, Step 3.
- [ ] **Step 2: Implementar** as mudanças acima. Não há teste de unidade; a prova é o fluxo. Antes de enviar: `bash -n` em qualquer script novo e `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/android.yml'))"`.
- [ ] **Step 3 (com autorização para enviar `fase2-m2`):** enviar e rodar `gh workflow run android.yml --ref fase2-m2`. Expected (ler o registro com `gh run view --log`): o passo de assinatura imprime `Verified using v2 scheme ... : true`, o `SHA-256 digest` igual ao do arquivo, `versionCode` = número da execução; os dois emuladores continuam verdes (o teste do M1 passa com o APK assinado); o passo de limpeza apaga o keystore. Teste negativo uma vez: rodar o fluxo numa branch de teste sem o segredo **não** é possível (segredos valem para todas as branches), então registrar no ledger que o ramo "segredo ausente" foi conferido só lendo o script (o `if [ -z ... ]` com `::error::`).
- [ ] **Step 4: Limpar e documentar:** apagar `/tmp/chave/` da sessão (`rm -rf`) e confirmar com `ls`; reescrever a seção "Assinatura" de `docs/notes/android-build.md` (chave fixa, onde ficam os segredos, como conferir a impressão digital, o que fazer se a chave se perder: gerar outra, atualizar segredos e o arquivo de impressão digital, e todo mundo reinstala uma vez) e o resumo do começo do arquivo.
- [ ] **Step 5: Commit** `ci: assinar o APK com a chave fixa do app`.

---

### Task 7: O celular virtual fecha, reabre e atualiza por cima

**Files:**
- Modify: `tools/teste-emulador.sh`, `docs/notes/android-build.md` (seção do emulador)

**Interfaces:**
- Consumes: linhas `PRISMA_M2 aberto ...` e `PRISMA_M2 gravado ...` (Task 4), as `PRISMA_M1` do M1 e as ajudas já existentes no script (`ler_registro`, `centro_da_casa`, `tocar_botao`, `calibrar_toque`, `esperar_animacao`, `falhou`, `foto`).
- Produces: novas fases ao **fim** do script (as fases do M1 não mudam), na ordem:
  1. **Reabrir (retomar):** ao fim da fase "volta ao Zen" (pontos do Zen `pontos_zen`, Clássico recém zerado), `adb shell am force-stop`, `adb logcat -c`, abrir, esperar 10 s; conferir `PRISMA_M2 aberto modo=zen zen=arquivo:<pontos_zen> classico=arquivo:0 ...` e `melhor_zen` maior ou igual a `pontos_zen`; foto.
  2. **Modo ativo e partidas separadas:** tocar na aba Clássico, jogar 1 jogada com Dica + toque-toque (como no M1), esperar a linha `PRISMA_M2 gravado modo=classico pontos=<pc>`, `force-stop`, reabrir → `aberto modo=classico ... classico=arquivo:<pc>` e `zen=arquivo:<pontos_zen>`.
  3. **Atualizar por cima:** `adb install -r "$APK"` (mesmo pacote, mesma assinatura), reabrir → a mesma linha `aberto` com os mesmos números (prova que a atualização preserva os dados); foto.
  4. **Recordes:** `melhor_zen` e `melhor_classico` da última linha `aberto` não são menores que os da primeira abertura desta fase.
  Depois de cada reabertura, ler de novo as linhas `tabuleiro` e `botao` (o app as escreve ao abrir) e refazer `calibrar_toque` antes de tocar. O teste não consegue estragar arquivos (o app de debug não é depurável, então `run-as` não funciona): a recuperação de arquivo estragado fica só nos testes de unidade (Tasks 2–4).

- [ ] **Step 1: Escrever as fases** no script, mantendo o estilo (mensagens em português, `falhou` com motivo, `foto <nome>` por fase). `bash -n tools/teste-emulador.sh`.
- [ ] **Step 2: Ensaiar o parse:** com um arquivo de amostra de logcat com as linhas `PRISMA_M2 ...` do formato da Task 4, rodar as funções de leitura do script (como foi feito no M1) e conferir que extraem `modo`, `zen`, `classico` e `melhor_*` certos.
- [ ] **Step 3: Atualizar** a seção do emulador em `docs/notes/android-build.md` (as fases novas e o que o teste não cobre).
- [ ] **Step 4 (com autorização para enviar):** enviar `fase2-m2`, rodar o fluxo. Expected: APK e os dois celulares virtuais (Android 11 x86 e Android 14 x86_64) verdes, com as linhas `aberto` das três reaberturas iguais aos pontos jogados; fotos conferidas olhando. Se o app fechar sozinho ou uma linha não vier, ler o registro, corrigir (com teste, se for erro de lógica) e reenviar.
- [ ] **Step 5: Commit** `ci: o celular virtual fecha, reabre e atualiza o app por cima`.

---

### Task 8: Conferência no celular do Thiago e fechamento do marco

**Files:**
- Modify: `docs/notes/android-build.md` (só se algo novo for aprendido)

- [ ] **Step 1: Integrar na `main` e esperar a release** (com autorização do Thiago): merge de `fase2-m2` com `--no-ff`, `cargo test --workspace`, envio, esperar o fluxo Android da `main` verde e a release `apk-mais-recente` republicada com o APK assinado pela chave fixa.
- [ ] **Step 2: Thiago confere no celular.** Lista (a **primeira** instalação exige desinstalar o "Prisma (Rust)" de debug de antes, pela última vez, porque a chave mudou):
  1. Instalar o APK novo, abrir: Zen novo, MELHOR 0 (ou o que já tiver).
  2. Jogar umas jogadas no Zen, trocar para o Clássico e jogar um pouco. Fechar o app **pelas apps recentes** (arrastar para fora) e abrir: continua no Clássico, com os pontos de antes; tocar em Zen: a partida do Zen também está lá.
  3. Fazer **Novo jogo** no Clássico: o MELHOR do Clássico continua com o maior placar de antes.
  4. Fechar no meio de uma animação (jogar e tirar o app logo depois): ao abrir, o tabuleiro mostra o resultado da jogada.
  5. Jogar o Clássico até o Fim de partida, fechar e abrir: o cartão de fim continua lá; "Jogar novamente" começa outra.
  6. **Atualizar por cima:** eu envio uma mudança mínima (só um texto de documentação) para a `main`; o Thiago baixa o APK novo e instala **sem desinstalar**: partida, modo e MELHOR continuam. (Só aqui se prova a chave fixa de verdade no aparelho.)
  *Pronto quando:* fechar, abrir e atualizar por cima mantém partida de cada modo, modo ativo e recordes. Qualquer falha: anotar no ledger (com print), corrigir antes do M3.
- [ ] **Step 3: Fechar o marco:** ledger, apagar a pasta de trabalho deste plano, relatar "Rulings I made" e "Deferred minors". Pendências conhecidas para depois: branches antigas no GitHub (`fase2-m0b`, `fase2-m1`), M3 (som), M4 (polimento, tela de recordes, versão mínima/alvo do Android).
