# Prisma Fase 2 · M0b (Teste de vida do Android) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ao abrir o APK no celular, uma tela de diagnóstico mostra um contador de aberturas gravado em arquivo, um botão que toca um som e um botão que vibra, provando as três coisas específicas do Android antes de construir o jogo em cima delas.

**Architecture:** A lógica que não depende do Android (contador, pasta de dados, tom em WAV, posição dos botões) fica em módulos puros de `game/`, testados com `cargo test`. Uma camada fina `plataforma` tem duas versões escolhidas por `#[cfg(target_os = "android")]`: no Android fala com o sistema por JNI (usando `ndk_utils` do miniquad) para achar a pasta privada do app e vibrar; no PC usa pasta do usuário e finge a vibração. O som usa a feature `audio` do Macroquad com um WAV gerado em memória. O CI (celular virtual) abre o app duas vezes, toca nos botões e lê o resultado no registro do Android.

**Tech Stack:** Rust 2021, Macroquad 0.4.16 (feature `audio`), miniquad 0.4.11 (`native::android::{attach_jni_env, ACTIVITY}` e `ndk_utils`), cargo-quad-apk 0.1.4, GitHub Actions com emulador.

**Spec:** `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md` (seções 3, 5.5, 5.6, 5.7 e marco M0b da seção 6). Nota: a seção 5.7 diz que a permissão `VIBRATE` só entra no M4, mas o M0b exige um botão que vibra; por isso a permissão entra agora (decisão deste plano).

## Global Constraints

- `edition = "2021"` em todos os `Cargo.toml` (o cargo-quad-apk não lê a 2024); o teste `tests/workspace.rs` guarda isso.
- O último pedaço do `package_name` (`com.thiagodluz.prisma_game`) é o nome do programa (`prisma_game`); não mudar.
- Retrato fixo; sem permissão de internet; a única permissão nova é `android.permission.VIBRATE`.
- O motor (`prisma_engine`) não muda; os 72 testes dele continuam passando. Hoje `cargo test --workspace` soma 88.
- Nenhuma dependência nova além de ligar a feature `audio` do `macroquad` (o `serde_json` só entra no M2).
- O alvo Android do Rust não instala neste ambiente (o servidor de download é bloqueado): todo código específico do Android fica em `game/src/plataforma/android.rs` e só compila no CI. O resto precisa compilar e passar no PC.
- Ligar a feature `audio` faz o jogo, no Linux, precisar da biblioteca `libasound` na hora de juntar o programa (`-lasound`, do pacote `libasound2-dev`). O CI de testes instala esse pacote (Task 4). Neste ambiente não há o pacote de desenvolvimento: depois do Step 4 da Task 4, todo comando `cargo` do pacote `prisma_game` leva `RUSTFLAGS="-L /tmp/asound"`, com `mkdir -p /tmp/asound && ln -sf /usr/lib/x86_64-linux-gnu/libasound.so.2 /tmp/asound/libasound.so`. Isso foi conferido numa cópia descartável: compila, e sem placa de som o PC só imprime um pânico da thread de áudio, sem fechar o programa.
- Conferido na mesma cópia: as 11 dependências novas do áudio (`audir-sles`, `audrey`, `dasp_*`, `hound`, `lewton`, `maybe-uninit`, `ogg`, `quad-alsa-sys`, `quad-snd`, `smallvec`) usam edition 2015 ou 2018, que o Cargo antigo do cargo-quad-apk lê.
- Textos da tela e comentários em português, simples (Thiago tem conhecimento básico de programação).
- Commits em português (`feat:`, `test:`, `ci:`, `docs:`) com os trailers de atribuição de sempre.

## Review Focus

1. **Arquivo do contador vazio, com lixo, negativo ou gigante:** recomeça de 1 sem travar e nunca deixa o arquivo pela metade (grava num temporário e troca de nome). Testes na Task 1.
2. **Pasta de dados que não existe ou não pode ser criada:** a tela mostra o erro em texto e o app continua aberto. Teste na Task 1 (`Err`) e ligação na Task 4.
3. **Tom com valores absurdos (volume maior que 1 ou negativo, duração 0 ou gigante, frequência 0, negativa ou NaN):** WAV válido, sem estourar o volume nem dar pânico. Testes na Task 2.
4. **Toque na borda exata, fora dos botões, ou tela muito larga/estreita:** só o botão certo reage e os botões nunca se sobrepõem nem saem da tela. Testes na Task 3.
5. **Falha de áudio ou de JNI (sem aparelho de som, exceção Java, sem vibrador) e uso repetido:** a linha mostra "erro: …" e o app continua; exceção Java pendente é limpa e referências JNI são soltas a cada chamada (senão cai depois de umas centenas de toques). Só dá para provar no emulador e no celular (Task 5 e Task 6).

---

### Task 1: Contador de aberturas e pasta de dados (puro)

**Files:**
- Create: `game/src/contador.rs`
- Modify: `game/src/main.rs` (só `mod contador;`)

**Interfaces:**
- Produces:
  - `pub const NOME_ARQUIVO: &str = "contador.txt";`
  - `pub fn interpretar(texto: &str) -> u32`
  - `pub fn registrar_abertura(pasta: &Path) -> Result<u32, String>`
  - `pub fn pasta_de_dados_no_pc(appdata: Option<&str>, xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf>`

- [ ] **Step 1: Escrever os testes que falham** em `game/src/contador.rs` (módulo `tests`). Um ajudante `pasta_temporaria(nome) -> PathBuf` usa `std::env::temp_dir().join(format!("prisma-teste-{}-{nome}", std::process::id()))`, apaga a pasta antes e cada teste apaga depois. Testes e o que afirmam:
  - `interpretar_aceita_numero_com_espacos_e_quebra_de_linha`: `"7\n"` → 7, `" 12 "` → 12.
  - `interpretar_devolve_zero_para_vazio_lixo_negativo_e_estouro`: `""`, `"abc"`, `"-3"`, `"1.5"`, `"99999999999999999999"`, `"٣"` → 0.
  - `registrar_abertura_comeca_em_1_e_conta_a_cada_chamada`: três chamadas na mesma pasta → 1, 2, 3; o arquivo contém `3`.
  - `registrar_abertura_recomeca_de_1_com_arquivo_corrompido`: arquivo com `lixo` → `Ok(1)`.
  - `registrar_abertura_satura_em_vez_de_dar_a_volta`: arquivo com `4294967295` → `Ok(4294967295)`.
  - `registrar_abertura_cria_a_pasta_se_nao_existir`: pasta com dois níveis que não existem → `Ok(1)`.
  - `registrar_abertura_devolve_erro_quando_a_pasta_e_um_arquivo`: caminho de um arquivo comum → `Err` (texto não vazio).
  - `registrar_abertura_nao_deixa_arquivo_temporario`: depois da chamada a pasta só tem `contador.txt`.
  - `pasta_de_dados_no_pc_prefere_appdata_depois_xdg_depois_home`: `Some("C:/A")` → `C:/A/prisma-rust`; sem appdata e `Some("/x/share")` → `/x/share/prisma-rust`; só `Some("/home/t")` → `/home/t/.local/share/prisma-rust`; tudo `None` → `None`; string vazia conta como ausente.

- [ ] **Step 2: Rodar e ver falhar.** Run: `cargo test -p prisma_game contador`. Expected: erro de compilação (funções não existem).

- [ ] **Step 3: Implementar as quatro assinaturas acima em `game/src/contador.rs`.** `registrar_abertura` lê `contador.txt` (arquivo ausente conta como 0), soma 1 com `saturating_add`, grava em `contador.txt.tmp` e troca o nome para `contador.txt` com `fs::rename`; cria a pasta com `create_dir_all`; erros viram `String` com o caminho dentro. Comentários em português simples.

- [ ] **Step 4: Rodar e ver passar.** Run: `cargo test -p prisma_game contador`. Expected: 9 passed. Avisos de `dead_code` são esperados até a Task 4.

- [ ] **Step 5: Commit** (`feat: contador de aberturas gravado em arquivo e pasta de dados no PC`).

### Task 2: Tom em memória (WAV)

**Files:**
- Create: `game/src/tom.rs`
- Modify: `game/src/main.rs` (só `mod tom;`)

**Interfaces:**
- Produces: `pub const TAXA: u32 = 44_100;` e `pub fn tom_wav(frequencia_hz: f32, duracao_ms: u32, volume: f32) -> Vec<u8>` (WAV PCM 16 bits, mono, cabeçalho padrão de 44 bytes; onda senoidal com subida e descida lineares de 5 ms para não estalar; `volume` limitado a 0..=1, NaN vira 0; frequência limitada a 20..=20000 Hz, NaN vira 440; duração limitada a 5000 ms).

- [ ] **Step 1: Escrever os testes que falham** em `game/src/tom.rs`:
  - `cabecalho_wav_esta_correto`: `RIFF` no início, `WAVE` em 8..12, formato 1 (PCM), 1 canal, taxa 44100, 16 bits, tamanho do RIFF = `len - 8`, tamanho dos dados = `len - 44`.
  - `duracao_define_o_numero_de_amostras`: 300 ms → `len == 44 + 2 * 13_230`.
  - `frequencia_certa_pelo_numero_de_cruzamentos_por_zero`: 440 Hz por 1000 ms → entre 876 e 884 cruzamentos.
  - `volume_define_o_pico`: volume 0.5 → pico entre 0,48 e 0,52 de 32767; volume 1.0 → pico entre 0,95 e 1,0 de 32767.
  - `volume_fora_da_faixa_e_limitado`: volume 5.0 → mesmo pico do volume 1.0 (sem dar a volta); volume -1.0 e NaN → todas as amostras 0.
  - `duracao_zero_gera_wav_valido_sem_amostras`: `len == 44` e tamanho dos dados 0.
  - `duracao_gigante_e_limitada`: `u32::MAX` ms → `len == 44 + 2 * 5 * 44_100`.
  - `comeca_e_termina_em_silencio`: primeira e última amostra valem 0.
  - `frequencia_invalida_nao_da_panico`: 0.0, -5.0, NaN e infinito devolvem WAV com o tamanho esperado.

- [ ] **Step 2: Rodar e ver falhar.** Run: `cargo test -p prisma_game tom`. Expected: erro de compilação.

- [ ] **Step 3: Implementar `tom_wav` e `TAXA`.** Amostra = `seno * envelope * volume * 32767`, arredondada e gravada em `i16` little-endian.

- [ ] **Step 4: Rodar e ver passar.** Run: `cargo test -p prisma_game tom`. Expected: 9 passed.

- [ ] **Step 5: Commit** (`feat: gerador de tom em WAV na memória`).

### Task 3: Layout da tela de diagnóstico (puro)

**Files:**
- Modify: `game/src/layout.rs`

**Interfaces:**
- Consumes: `Rect { x, y, w, h: f32 }` de `layout.rs` (já existe).
- Produces:
  - `impl Rect { pub fn contem(&self, x: f32, y: f32) -> bool }` (borda esquerda e de cima incluídas, direita e de baixo excluídas; NaN → `false`).
  - `pub struct TelaDiagnostico { pub x_texto: f32, pub y_titulo: f32, pub y_linhas: [f32; 3], pub tamanho_titulo: f32, pub tamanho_texto: f32, pub botao_som: Rect, pub botao_vibrar: Rect }`
  - `pub fn tela_diagnostico(largura: f32, altura: f32) -> TelaDiagnostico` com estas medidas, com `m = min(largura, altura)`: `x_texto = 0.06·largura`; `y_titulo = 0.10·altura`; `y_linhas = [0.20, 0.28, 0.36]·altura` (contador, som, vibração); `tamanho_titulo = 0.075·m`; `tamanho_texto = 0.045·m`; botões com `x = 0.06·largura`, `w = 0.88·largura`, `h = 0.09·altura`, `botao_som.y = 0.50·altura`, `botao_vibrar.y = 0.62·altura`. Tamanho inválido (≤ 0, NaN ou infinito) → botões com `w = h = 0` e `contem` sempre `false`.

- [ ] **Step 1: Escrever os testes que falham** em `layout.rs`:
  - `botoes_ficam_dentro_da_tela`: para 540×960, 360×640, 1080×2400, 960×540 e 300×300, os dois botões cabem em `[0, largura] × [0, altura]`.
  - `botoes_nao_se_sobrepoem`: mesmas telas; o fundo de `botao_som` é menor que o topo de `botao_vibrar`.
  - `texto_fica_acima_dos_botoes_e_as_linhas_nao_se_sobrepoem`: mesmas telas; `y_linhas[2] < botao_som.y` e cada linha está pelo menos `1.2 · tamanho_texto` abaixo da anterior.
  - `botoes_em_pe_tem_altura_boa_para_o_dedo`: em 540×960 e 360×640, `h >= 0.08 · altura`.
  - `contem_inclui_esquerda_e_topo_e_exclui_direita_e_baixo`: ponto `(x, y)` dentro; `(x + w, y)` e `(x, y + h)` fora.
  - `contem_rejeita_nan`.
  - `tela_invalida_da_botoes_vazios`: `(0, 0)`, `(-1, 5)`, `(NaN, 10)`, `(f32::INFINITY, 10)` → `w == 0`, `h == 0` e `contem(0.0, 0.0) == false`.

- [ ] **Step 2: Rodar e ver falhar.** Run: `cargo test -p prisma_game layout`. Expected: erro de compilação.

- [ ] **Step 3: Implementar `Rect::contem`, `TelaDiagnostico` e `tela_diagnostico`.**

- [ ] **Step 4: Rodar e ver passar.** Run: `cargo test -p prisma_game layout`. Expected: os 5 testes antigos mais os 7 novos passam.

- [ ] **Step 5: Commit** (`feat: layout da tela de diagnóstico`).

### Task 4: Camada da plataforma e tela montada

**Files:**
- Create: `game/src/plataforma/mod.rs`, `game/src/plataforma/pc.rs`, `game/src/plataforma/android.rs`
- Modify: `game/src/main.rs`, `game/Cargo.toml`, `.github/workflows/test.yml`

**Interfaces:**
- Consumes: `contador::{registrar_abertura, pasta_de_dados_no_pc}`, `tom::tom_wav`, `layout::{tela_diagnostico, TelaDiagnostico, Rect::contem}`.
- Produces: `plataforma::pasta_de_dados() -> Result<PathBuf, String>` e `plataforma::vibrar(milissegundos: u32) -> Result<(), String>`; `main.rs`: `enum Tela { Diagnostico, Vitrine }` e `fn tela_pedida(args: &[String]) -> Tela`.

- [ ] **Step 1: Escrever os testes que falham** (rodam no PC):
  - em `main.rs`, `tela_pedida_le_o_argumento_tela`: `["--tela", "vitrine"]` → `Vitrine`; `[]`, `["--tela"]` e `["--tela", "xyz"]` → `Diagnostico`;
  - em `plataforma/pc.rs`, `vibrar_no_pc_nao_da_erro`: `vibrar(200)` → `Ok(())`.

- [ ] **Step 2: Rodar e ver falhar.** Run: `cargo test -p prisma_game`. Expected: erro de compilação.

- [ ] **Step 3: Implementar `plataforma`.** `mod.rs` escolhe `android` ou `pc` com `#[cfg(target_os = "android")]` e reexporta as duas funções. `pc.rs`: `pasta_de_dados` lê `APPDATA`, `XDG_DATA_HOME` e `HOME` e chama `pasta_de_dados_no_pc` (erro em português se vier `None`); `vibrar` devolve `Ok(())`. `android.rs`, com `unsafe` isolado e comentado:
  - obter o `env` com `macroquad::miniquad::native::android::attach_jni_env()` e usar `ACTIVITY` e as macros `call_object_method!`, `call_void_method!`, `call_bool_method!`, `get_utf_str!` (reexportadas por `macroquad::miniquad`; se o caminho exato for outro, usar o que o compilador do CI indicar e anotar no ledger);
  - cada função abre um quadro de referências (`PushLocalFrame(env, 16)`) e fecha com `PopLocalFrame` no fim, em qualquer caminho de saída, para não acumular referências;
  - depois de cada chamada Java, `ExceptionCheck`: se houver exceção, `ExceptionDescribe`, `ExceptionClear` e devolver `Err("exceção do Android …")`; resultado nulo também vira `Err`;
  - `pasta_de_dados`: `getFilesDir()` (`"()Ljava/io/File;"`) → `getAbsolutePath()` (`"()Ljava/lang/String;"`);
  - `vibrar`: `getSystemService("vibrator")` (`"(Ljava/lang/String;)Ljava/lang/Object;"`, texto criado com `NewStringUTF`), `hasVibrator()` (`"()Z"`; falso → `Err("este aparelho não tem vibrador")`), `vibrate(J)V` com o tempo como `jlong` (método marcado como antigo desde o Android 8, mas funciona em todas as versões).

- [ ] **Step 4: Ligar a tela em `main.rs` e as configurações em `Cargo.toml`.** `Cargo.toml`: `macroquad = { version = "0.4", features = ["audio"] }` e, em `[package.metadata.android]`, `[[package.metadata.android.permission]] name = "android.permission.VIBRATE"`. `main.rs`, tela `Diagnostico` (padrão; `--tela vitrine` mostra o tabuleiro do M0a e mantém `--screenshot` funcionando):
  - no início: `registrar_abertura(&pasta_de_dados()?)`; carregar o som com `macroquad::audio::load_sound_from_bytes(&tom_wav(440.0, 300, 0.4))`;
  - desenhar com `tela_diagnostico(screen_width(), screen_height())`: título "Prisma · teste de vida", linhas "Aberturas: N" (ou "Aberturas: erro: …"), "Som: …" e "Vibração: …" (começam em "ainda não testado"), botões "Tocar som" e "Vibrar" (retângulos cheios com texto);
  - ao `is_mouse_button_pressed(MouseButton::Left)` dentro de um botão (`mouse_position()`; no celular o toque vira mouse): tocar com `play_sound_once` dentro de `std::panic::catch_unwind` (se o áudio estiver morto, a linha mostra "Som: erro: …" em vez de fechar o app; linha "Som: tocado" quando der certo) ou chamar `plataforma::vibrar(300)` (linha "Vibração: ok" ou "Vibração: erro: …");
  - registrar no log do Android (`macroquad::logging::info!`, aparece com a etiqueta `SAPP`), uma linha por fato, para o CI ler: `PRISMA_M0B aberturas=N` (ou `aberturas=erro: …`), `PRISMA_M0B som_carregado=ok` (ou `erro: …`), `PRISMA_M0B botao=som x=X y=Y` e `PRISMA_M0B botao=vibrar x=X y=Y` (centros em pixels físicos: coordenada lógica × `screen_dpi_scale()`), e depois de cada toque `PRISMA_M0B som=ok` e `PRISMA_M0B vibracao=ok` (ou `erro: …`).

  - `.github/workflows/test.yml`: antes do `cargo test`, um passo `sudo apt-get install -y libasound2-dev` (sem ele o teste do pacote `game` não junta o programa).

- [ ] **Step 5: Rodar tudo no PC.** Run: `mkdir -p /tmp/asound && ln -sf /usr/lib/x86_64-linux-gnu/libasound.so.2 /tmp/asound/libasound.so && RUSTFLAGS="-L /tmp/asound" cargo test --workspace`. Expected: todos passam (88 + os novos: 9 + 9 + 7 + 2 = 115).

- [ ] **Step 6: Foto da tela no PC.** Run: `RUSTFLAGS="-L /tmp/asound" LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a -s "-screen 0 1280x1024x24" cargo run -p prisma_game -- --screenshot /tmp/m0b.png` e abrir a foto. Expected: título, três linhas e dois botões legíveis; a linha do contador mostra um número. Uma mensagem de pânico de `quad_snd`/`alsa_snd` no terminal é normal aqui (não há placa de som) e não impede a foto.

- [ ] **Step 7: Commit** (`feat: tela de diagnóstico com arquivo, som e vibração`).

### Task 5: CI: conferir tudo no celular virtual

**Files:**
- Modify: `.github/workflows/android.yml`, `tools/teste-emulador.sh`, `docs/notes/android-build.md`

**Interfaces:**
- Consumes: as linhas `PRISMA_M0B …` da Task 4 e o pacote lido de `game/Cargo.toml`.

- [ ] **Step 1: Só publicar a release na `main`.** No passo "Publicar na release", `if: github.ref == 'refs/heads/main'`, para poder rodar o fluxo numa branch (`gh workflow run android.yml --ref <branch>`) sem trocar o APK da release.

- [ ] **Step 2: Estender `tools/teste-emulador.sh`.** Depois da abertura e da foto atuais: (a) o registro precisa ter `PRISMA_M0B aberturas=1`; (b) `adb shell am force-stop`, `adb logcat -c`, abrir de novo (monkey), esperar 10 s: precisa ter `PRISMA_M0B aberturas=2`; (c) ler `botao=som` e `botao=vibrar` do registro e tocar com `adb shell input tap X Y`, esperando 3 s depois de cada um: precisa ter `PRISMA_M0B som=ok` e uma linha `PRISMA_M0B vibracao=` que seja `ok` ou `erro: este aparelho não tem vibrador` (qualquer outro erro reprova); (d) o app continua aberto no fim. Cada falha imprime a linha que faltou, o resumo do registro e `exit 1`.

- [ ] **Step 3: Conferir a sintaxe.** Run: `bash -n tools/teste-emulador.sh`. Expected: sem saída.

- [ ] **Step 4: Rodar o CI numa branch.** Subir a branch do M0b (com autorização do Thiago) e rodar `gh workflow run android.yml --ref fase2-m0b`, depois acompanhar com `gh run list`/`gh run view --log`. Expected: `apk` e os dois `emulador` verdes; o registro mostra `aberturas=1`, `aberturas=2`, `som=ok` e `vibracao=`. Se o APK não compilar por causa do código Android (que não dá para compilar aqui) ou do áudio (as dependências novas têm edition 2018, então o Cargo antigo deve ler; o risco que sobra é o OpenSL do Android não iniciar), corrigir pelo erro do registro; se o problema for o áudio, registrar a decisão no ledger e avisar o Thiago antes de seguir (o M3 depende dele).

- [ ] **Step 5: Atualizar `docs/notes/android-build.md`** com a lista de conferência do M0b (Task 6) e o que o emulador cobre e não cobre (som real e vibração de verdade só no celular).

- [ ] **Step 6: Commit** (`ci: o emulador confere contador, som e vibração do M0b`).

### Task 6: Conferência no celular do Thiago

**Files:**
- Modify: `docs/notes/android-build.md` (só se algo novo for aprendido)

- [ ] **Step 1: Integrar na `main` e esperar a release** (com autorização do Thiago): o fluxo Android volta a publicar `apk-mais-recente`. Run: `gh run list` até o fluxo Android da `main` ficar verde.

- [ ] **Step 2: Thiago instala e confere (desinstalando o app antigo antes, porque a chave de debug muda):**
  1. Abrir: aparece "Aberturas: 1". Fechar o app de vez e abrir de novo: "Aberturas: 2".
  2. Tocar em "Tocar som": ouve um bipe e a linha diz "Som: tocado".
  3. Tocar em "Vibrar": o celular vibra e a linha diz "Vibração: ok".

  *Pronto quando:* os três funcionam. Se algum falhar, a linha da tela mostra o erro; anotar no ledger e corrigir antes do M1.

- [ ] **Step 3: Fechar o marco.** Registrar no ledger, apagar a pasta de trabalho deste plano e relatar as decisões ("Rulings I made") e os adiados ("Deferred minors").
