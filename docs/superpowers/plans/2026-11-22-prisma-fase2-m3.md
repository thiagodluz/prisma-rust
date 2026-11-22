# Prisma Fase 2 · M3 (Som) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O jogo tem som: efeitos sintetizados (como o `sound.js` do Prisma em JS), e no Zen música, ambientes, respiração guiada e três volumes ajustáveis (efeitos, música, ambiente), tudo lembrado ao fechar e abrir. O APK continua num tamanho razoável (meta: até 60 MB; hoje 26,9 MB).

**Architecture:** Peças puras e testáveis (`efeitos`: notas → amostras → WAV; `preferencias` + respiração; `fundo`: "o que deveria estar tocando" → lista de comandos) e uma camada fina com o Macroquad (`audio.rs`) que só executa. O `App` decide *quando* tocar (fila de efeitos, `som_desejado()`), a tela ganha uma folha "Som" e uma linha de respiração. Músicas e ambientes não entram no git: o comando novo `prisma_tools audio` baixa, confere o SHA-256 e prepara os arquivos em `game/sons/`; o APK leva essa pasta como "assets" do Android (uma cópia só).

**Tech Stack:** Rust 2021, Macroquad 0.4.16 (feature `audio` já ligada), `prisma_engine` (não muda), `serde_json` (já no jogo), `sha2` (só em `tools/`, feature `audio`), `ffmpeg`/`ffprobe` (programa externo, só na preparação dos sons).

**Spec:** `docs/superpowers/specs/2026-11-20-prisma-fase2-design.md` (seção 5.5 "Som", marco M3 da seção 6, risco 5). Referência de comportamento no Prisma em JS (`/home/claude/thiagodluz/prisma`, só leitura): `sound.js` (notas), `zen.js` (música e ambientes), `app.js` linhas 110–180 e 590–640 (respiração, volumes, quando cada efeito toca), `scripts/download-audio.mjs` (os 9 áudios e seus SHA-256), `LICENSE-ART.md` (créditos).

## Fatos que mandam no desenho (conferidos no código das bibliotecas)

- O Macroquad **carrega o arquivo inteiro descompactado** na memória: estéreo, 44,1 kHz, `f32` = **21 MB por minuto** (mono vira estéreo; 48 kHz é reamostrado por um método grosseiro). Por isso o "Ruído suave" original (12 min) viraria ~258 MB e o "Floresta após chuva" (4 min) ~85 MB. A decodificação roda na própria thread do jogo (trava o quadro enquanto carrega).
- `Sound` tem `Drop` que o apaga do mixer: soltar a faixa libera a memória.
- O Macroquad pausa e retoma o áudio sozinho ao minimizar/restaurar o app.
- O gerador de APK (`cargo-quad-apk` 0.1.4) empacota uma pasta de assets (`assets = "..."` em `[package.metadata.android]`), lida por `macroquad::file::load_file`; o CI gera 3 processadores (armeabi-v7a, arm64-v8a, x86): embutir o áudio no programa (`include_bytes!`) triplicaria o tamanho.
- O APK do CI é compilado em modo de desenvolvimento (sem otimização): decodificar OGG assim, no celular, seria lento. Bibliotecas de terceiros passam a ser otimizadas (`[profile.dev.package."*"]`).
- Medido aqui: os 9 originais somam 29 MB; as músicas (11,6 MB, ~2 min cada, 41–44 MB na memória) entram como estão; os 6 ambientes, depois de laço de 60 s e OGG a 44,1 kHz (`-q:a 3`), somam ~3,8 MB (0,46–0,92 MB cada). Total ~15,4 MB de áudio no APK.

## Global Constraints

- `edition = "2021"`; o motor `prisma_engine` não muda (`cargo test --workspace` hoje: 224 passam; `tools/`: 72 passam + 1 ignorado, 58 com `--no-default-features`; todos continuam passando). Dependência nova só em `tools/` (`sha2`, opcional na feature `audio`). Nenhuma regra de jogo nova.
- Volumes: três canais (efeitos, música, ambiente), inteiros de 0 a 100, de 5 em 5 na tela; padrões **80 / 70 / 65**. Volume de saída: efeitos = `efeitos/100 × 1,25` (o "ganho mestre" do JS); música = `musica/100`; ambiente = `ambiente/100`.
- Efeitos: sem arquivo; notas do `sound.js` valor por valor (tabela na Task 2), geradas em memória, WAV mono 16 bits 44 100 Hz.
- Música e ambiente só no Zen (como o JS: `active: game.mode === 'zen'`); efeitos nos dois modos. Música padrão `cozyPuzzle`, ambiente padrão `rain`, respiração `off`, música e ambiente desligados.
- Áudio **fora do git** (spec 5.5): `game/sons/` no `.gitignore`. Os 9 originais são conferidos pelo SHA-256 de `scripts/download-audio.mjs` (nunca baixar sem conferir). Música: bytes originais. Ambiente: sempre reencodado a 44 100 Hz; se passar de 63 s vira laço de exatamente 60 s (3 s de transição suave).
- Nomes dos arquivos em `game/sons/`: `musica/{magic-puzzle,cozy-puzzle,space-city}.ogg` e `ambiente/{stream,rain-soft,forest-cicadas,rainforest,rain-thunder,soft-noise}.ogg`.
- Registro do Android (logcat): linhas `PRISMA_M1` e `PRISMA_M2` continuam como estão; as novas começam com `PRISMA_M3` (formatos exatos nas Tasks 4, 6 e 9). Os botões novos usam o formato que já existe (`PRISMA_M1 botao=NOME x=.. y=..`).
- Se um áudio faltar ou estiver estragado: o jogo continua jogável, sem pânico; a folha mostra o aviso e o registro ganha `PRISMA_M3 erro ...`.
- Neste ambiente os testes e a "foto" precisam de `libasound`: `mkdir -p /tmp/asound && ln -sf /usr/lib/x86_64-linux-gnu/libasound.so.2 /tmp/asound/libasound.so` e `export RUSTFLAGS="-L /tmp/asound"`. Foto: `LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a -s "-screen 0 1280x1024x24" cargo run -q -p prisma_game -- --cena NOME --screenshot arquivo.png`. Não há placa de som aqui: quem julga o som é o ouvido do Thiago no celular.
- **Segredos:** nada novo; nenhuma chave entra no git. Merge na `main`, envio de branch e execução do CI só com autorização do Thiago.
- Textos e comentários em português simples (Thiago tem conhecimento básico de programação): explicar cada conceito novo de Rust numa linha, como nos módulos existentes. Commits em português (`feat:`, `test:`, `ci:`, `docs:`) com os trailers de atribuição.

## Review Focus

1. **Áudio ausente ou estragado** (sem `game/sons` no PC, APK sem assets, OGG cortado, arquivo vazio): sem pânico, jogo jogável, aviso na folha, uma linha `PRISMA_M3 erro` por tentativa, e nova tentativa só quando o jogador desliga e liga de novo (não a cada quadro). Tasks 6 e 7.
2. **Memória e travadas:** no máximo uma música e um ambiente carregados; trocar de faixa solta a anterior **antes** de carregar a nova; ligar e desligar várias vezes não acumula nada; trocar para Clássico libera as duas. Task 6 (planejador) e Task 9 (emulador mede).
3. **Valores esquisitos nas preferências** (arquivo à mão, de versão futura ou estragado): número fracionário, negativo, enorme, texto, `null`, faixa desconhecida, JSON cortado ou com BOM: cada campo cai no padrão como no JS; JSON ilegível vira `som.json.corrupt` e começa com os padrões. Volume de efeitos 0 ou "Sons do jogo" desligado = nenhum efeito (e a animação não muda). Task 3.
4. **Folha de Som aberta no meio de tudo** (animação em andamento, fim de partida, pergunta "Novo jogo?"): abrir e fechar não quebra a animação, os efeitos nem o fim de partida; dedo que sai da barra ou toque cancelado deixa o último valor válido e grava uma vez; os botões e o tabuleiro de baixo não reagem enquanto a folha está aberta. Task 5.
5. **Volta do segundo plano e salto de tempo:** um quadro de mais de 2 s (app voltando, ou carregamento de música) não dispara uma rajada de efeitos atrasados; cascata muito longa não passa da elevação 5; a respiração recomeça do "Inspire". Tasks 4 e 5.
6. **Tela pequena e tela alta:** a folha e a linha da respiração cabem em 360×640 e 1080×2400 sem sobrepor nada, e o tabuleiro não encolhe por causa da linha nova. Task 5.

## Escolhas minhas (a confirmar na revisão do plano)

- **Ambientes viram laços de 60 s** (e 44,1 kHz); músicas ficam inteiras. Custo: os ambientes longos perdem variedade (o trovão do "Chuva com trovoada" pode cair em outro ponto); ganho: nenhum ambiente passa de 21 MB na memória.
- **Sons como "assets" do Android**, não embutidos no programa (uma cópia no APK, não três).
- **Folha "Som"** aberta por um botão ♪ ao lado das abas, com: Sons do jogo (liga/desliga), três barras (de 5 em 5, arrastar ou tocar), Testar som, e no Zen: Música e Ambiente (liga/desliga + um toque no nome passa para a próxima opção) e Respiração. A linha da respiração (bolinha que cresce e encolhe, "Inspire/Expire" e os segundos) fica sempre reservada abaixo dos botões no Zen, para o tabuleiro não pular quando ela liga.
- **"Intensidade dos efeitos" do Zen fica no M4** (a spec põe lá).
- **Créditos do áudio:** arquivo `docs/CREDITOS-AUDIO.md` agora (inclui a atribuição CC BY 4.0 do "Ruído suave" e o aviso de que foi cortado em laço e reconvertido); a tela de créditos dentro do app fica para o M4.
- Preferências no arquivo `som.json` (mesmos nomes dos campos do JS), no mesmo diretório dos saves do M2.
- As músicas repetem de volta ao começo sem transição (como o `loop` do navegador no JS).
- Se os efeitos soarem baixos no celular (o JS usa volumes bem baixos por nota), ajusto o ganho depois da conferência, sem mexer no resto.

## Mapa de arquivos

- Create: `tools/src/audio.rs` (T1), `game/src/efeitos.rs` (T2), `game/src/preferencias.rs` (T3), `game/src/fundo.rs` e `game/src/audio.rs` (T6), `docs/CREDITOS-AUDIO.md` (T8), `docs/notes/audio.md` (T9).
- Modify: `tools/src/main.rs`, `tools/src/base64.rs`, `tools/Cargo.toml` (T1); `game/src/tom.rs` (T2); `game/src/salvar.rs` (T3); `game/src/linha.rs`, `game/src/app.rs` (T4, T5, T6); `game/src/layout.rs` (T5); `game/src/render.rs`, `game/src/cenas.rs` (T7); `game/src/main.rs` (T6, T9); `game/Cargo.toml`, `Cargo.toml` (raiz), `.gitignore`, `.github/workflows/android.yml` (T8); `tools/src/registro.rs`, `tools/src/emulador.rs`, `tools/tests/amostra-logcat.txt`, `docs/notes/android-build.md` (T9).

---

### Task 1: `prisma_tools audio` — preparar os sons

**Files:**
- Create: `tools/src/audio.rs`
- Modify: `tools/src/main.rs` (novo subcomando `audio <pasta-destino>`), `tools/src/base64.rs` (decodificar), `tools/Cargo.toml` (feature `audio`, padrão ligada: `audio = ["dep:sha2"]`; sem ela o subcomando diz "compilado sem a feature audio", como o `imagens`), `.gitignore` (`game/sons/`, para os sons preparados nunca entrarem no git)

**Interfaces:**
- Produces: `pub fn preparar(destino: &Path) -> Result<Vec<String>, String>` (lista dos arquivos gravados) e as peças puras abaixo; `base64::decodificar(texto: &str) -> Result<Vec<u8>, String>`.
- Consumes: programas `curl`, `ffmpeg`, `ffprobe` no PATH (a ausência de qualquer um é erro claro).

- [ ] **Step 1: Testes puros primeiro** (`audio.rs`, `base64.rs`). `decodificar_e_o_inverso_de_codificar` (vários tamanhos, incluindo vazio e com `=`), `decodificar_recusa_caractere_invalido`; `extrair_audios_acha_as_duas_formas`: um HTML pequeno com `data:audio/ogg;base64,...` e `data:audio/mpeg;base64,...` e lixo no meio devolve os bytes das duas; `conferir_sha256_recusa_arquivo_diferente`: erro nomeia o arquivo esperado; `tabela_de_origens_tem_9_itens_e_nomes_unicos`; `tabela_igual_ao_script_do_js`: se `/home/claude/thiagodluz/prisma/scripts/download-audio.mjs` existir, cada par caminho→SHA-256 da tabela aparece no texto do script (pula em silêncio se não existir); `plano_do_ambiente_curto_so_reencoda` (20 s → sem corte); `plano_do_ambiente_longo_vira_laco_de_60s` (732 s → início `(732 − 63) / 2`, 3 entradas do ffmpeg: início 3 s, "rabo" 3 s em +60 s, corpo 57 s em +3 s; filtro `afade` de 3 s com curva `qsin` nos dois pedaços curtos, `amix=inputs=2:normalize=0`, `concat`; saída `-ar 44100 -c:a libvorbis -q:a 3`).
- [ ] **Step 2: Rodar e ver falhar:** `cd /home/claude/prisma-rust && cargo test --manifest-path tools/Cargo.toml audio` → Expected: não compila / FAIL (funções ainda não existem).
- [ ] **Step 3: Implementar** `ORIGENS: [(&str /*caminho no JS*/, &str /*sha256*/, Destino)]` (os 9 pares do `download-audio.mjs`, mais o nome de saída da Constraint), `extrair_audios(html: &str) -> Vec<Vec<u8>>`, `conferir_sha256`, `plano_do_ambiente(duracao_s: f64) -> Plano`, `baixar_html()` (`curl -fsSL --retry 3` do endereço do `download-audio.mjs`, para um arquivo temporário) e `preparar` (música: grava os bytes originais; ambiente: ffmpeg conforme o plano; depois confere com `ffprobe` que a saída é Vorbis, 44 100 Hz e ≤ 60,5 s; arquivo que já existe e passa na conferência é mantido; grava por arquivo temporário + troca de nome). Mensagens de erro em português dizendo o que fazer.
- [ ] **Step 4: Rodar:** mesmo comando → Expected: testes novos passam; depois `cargo test --manifest-path tools/Cargo.toml` e `cargo test --manifest-path tools/Cargo.toml --no-default-features` → Expected: tudo verde (contagens da Global Constraints + os novos); `cargo clippy --manifest-path tools/Cargo.toml --all-targets -- -D warnings` (com e sem `--no-default-features`) sem avisos.
- [ ] **Step 5: Rodar de verdade** (com rede): `cargo run --release --manifest-path tools/Cargo.toml -- audio game/sons` → Expected: 9 arquivos em `game/sons/{musica,ambiente}/`, SHA-256 conferidos; `ffprobe` de cada ambiente mostra 44 100 Hz e duração ≤ 60,5 s; `du -sh game/sons` ≈ 15 MB; segunda execução não refaz nada. Registrar os tamanhos no livro-razão.
- [ ] **Step 6: Commit** `feat(tools): comando audio prepara musicas e ambientes (SHA-256 e ffmpeg)`; antes, `git status` não pode mostrar nada de `game/sons/`.

### Task 2: Efeitos sintetizados (puro)

**Files:**
- Create: `game/src/efeitos.rs`
- Modify: `game/src/tom.rs` (separar `pub fn wav_de_amostras(amostras: &[f32]) -> Vec<u8>` — mono, 16 bits, `TAXA`, cabeçalho de 44 bytes, valores presos em −1..1 — e fazer `tom_wav` usá-la, sem mudar o que `tom_wav` devolve), `game/src/main.rs` (`mod efeitos;`)

**Interfaces:**
- Produces: `pub enum Sinal { Invalida, Combinacao, Cascata, Supernova, Pulso, Raio, Espectro, Criacao, Nivel, Resgate }` com `nome(&self) -> &'static str` (`"invalida"`, `"combinacao"`, ...); `pub struct Efeito { pub sinal: Sinal, pub elevacao: u8 }` (`Copy`, `Eq`, `Hash`); `pub struct Nota { pub freq: f32, pub freq_final: f32, pub inicio_s: f32, pub duracao_s: f32, pub volume: f32, pub onda: Onda }`, `pub enum Onda { Seno, Triangular }`; `pub fn notas(efeito: Efeito) -> Vec<Nota>`; `pub fn renderizar(notas: &[Nota]) -> Vec<f32>`; `pub fn wav(efeito: Efeito) -> Vec<u8>`; `pub fn elevacao_da_cadeia(cadeia: u32) -> u8` (`min(max(cadeia−1, 0), 5)`); `pub fn sinal_da_limpeza(supernova: bool, ativadas: &[TileKind], cadeia: u32) -> Sinal` (prioridade do `cueForFrame`: Supernova > Espectro > Raio > Pulso > Cascata se `cadeia > 1` > Combinacao); `pub const GANHO_MESTRE: f32 = 1.25`; `pub fn todos() -> Vec<Efeito>` (os 20 que existem: 8 sinais sem elevação + Combinacao e Cascata com elevação 0 a 5).
- Consumes: `prisma_engine::TileKind`.

Tabela de notas (do `sound.js`; `f` = frequência Hz, `→g` = frequência final, `t` = início s, `d` = duração s, `v` = volume; onda S = seno, T = triangular; `lift = 2^(elevacao/12)` só em Combinacao e Cascata):

| Sinal | Notas |
|---|---|
| Invalida | 235→205, t 0, d .09, v .028, T |
| Combinacao / Cascata | 523.25, 659.25, 783.99 (× lift), t = i×.057, d .21, v .047 / .055, T |
| Supernova | 130.81→523.25, t 0, d .25, v .06, S · 220→55, t .24, d .44, v .09, S · 523.25, 783.99, 1046.5, 1567.98 em t = .24 + i×.045, d .32, v .045, T |
| Pulso | 190→63, t 0, d .31, v .085, S · 740, 988, 1318 em t = .045 + i×.035, d .17, v .042, T |
| Raio | 392, 587.33, 783.99, 1174.66 em t = i×.052, d .32, v .056, S |
| Espectro | 392, 440, 523.25, 587.33, 659.25, 783.99, 1046.5 em t = i×.046, d .3, v .042, S · 130.81→261.63, t 0, d .53, v .07, T |
| Criacao | 659.25, 987.77 em t = i×.065, d .29, v .04, S |
| Nivel | 392, 523.25, 659.25, 1046.5 em t = i×.11, d .39, v .055, T |
| Resgate | 587.33, 783.99, 1174.66 em t = i×.08, d .35, v .038, S |

Cada nota: o volume sobe de 0,0001 até `v` em `min(.018, d/4)` s e desce de `v` a 0,0001 até `d` (rampas **exponenciais**, como o `exponentialRampToValueAtTime`); a frequência vai de `f` a `→g` em rampa exponencial ao longo de `d`; a nota dura `d + 0,01` s. A soma de todas as notas é o som; sem normalizar.

- [ ] **Step 1: Testes primeiro** (`efeitos.rs` e `tom.rs`): `tabela_de_notas_bate_com_a_do_js` (para cada sinal: quantidade de notas e, da primeira e da última nota, `freq`, `freq_final`, `inicio_s`, `duracao_s`, `volume`, `onda`, com os números da tabela); `elevacao_5_multiplica_a_frequencia_por_2_elevado_a_5_sobre_12` (`notas(Efeito{Combinacao,5})[0].freq ≈ 523.25 × 1.3348`, tolerância 0,01); `elevacao_da_cadeia_limita_em_5` (0→0, 1→0, 2→1, 6→5, 7→5, `u32::MAX`→5); `sinal_da_limpeza_segue_a_prioridade_do_js` (cada ordem de empate); `duracao_do_som_e_o_fim_da_ultima_nota_mais_10_ms` (todos os 20: `renderizar(..).len()` = `round((max(t+d) + 0,01) × 44100)` ±1); `pico_nao_passa_da_soma_dos_volumes_e_nao_e_mudo` (todos os 20: 0,3 × maior `v` ≤ pico ≤ soma dos `v` + 0,001); `o_som_comeca_e_termina_quase_em_silencio` (|primeira| e |última| ≤ 0,001, sem estalo); `nota_unica_de_440hz_tem_o_numero_certo_de_cruzamentos_por_zero` (0,5 s → 440 ±3 cruzamentos para cima); `varredura_de_190_para_63_desce` (cruzamentos nos primeiros 50 ms ≥ 2× os dos últimos 50 ms); `onda_triangular_e_senoidal_diferem` (mesma nota, formas diferentes); `wav_tem_cabecalho_valido` (RIFF/WAVE, formato 1, 1 canal, 44 100, 16 bits, tamanhos batem com os dados) e `wav_de_amostras_prende_valores_fora_de_1` (`tom.rs`); `tom_wav_continua_igual` (um `tom_wav(440.0, 300, 0.4)` byte a byte igual ao de antes da mudança: guardar os primeiros 64 bytes e o tamanho como constantes tiradas do código atual **antes** de mexer).
- [ ] **Step 2: Rodar e ver falhar:** `cd /home/claude/prisma-rust && LIBRARY_PATH=/tmp/asound cargo test -p prisma_game efeitos tom` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar** as assinaturas acima. `renderizar`: mono, `TAXA` 44 100; fase acumulada em `f64` a cada amostra com a frequência do instante; seno por `sin`, triangular por `2/π·asin(sin(fase))`; envelope pela fórmula exponencial acima. `wav(efeito) = wav_de_amostras(&renderizar(&notas(efeito)))`.
- [ ] **Step 4: Rodar:** `LIBRARY_PATH=/tmp/asound cargo test --workspace` → Expected: tudo verde (224 + novos); `cargo clippy -p prisma_game --all-targets -- -D warnings` não ganha avisos novos (os antigos listados no M2 continuam).
- [ ] **Step 5: Commit** `feat(game): efeitos sonoros sintetizados (porte do sound.js)`.

### Task 3: Preferências de som, arquivo `som.json` e respiração (puro)

**Files:**
- Create: `game/src/preferencias.rs`
- Modify: `game/src/salvar.rs` (`carregar_preferencias`, `gravar_preferencias`), `game/src/main.rs` (`mod preferencias;`)

**Interfaces:**
- Produces: `pub struct Volumes { pub efeitos: u8, pub musica: u8, pub ambiente: u8 }`; `pub enum Canal { Efeitos, Musica, Ambiente }` (as três barras; no áudio de fundo da Task 6 só `Musica` e `Ambiente` aparecem); `pub enum Musica { MagicPuzzle, CozyPuzzle, SpaceCity }`; `pub enum Ambiente { Riacho, Chuva, Floresta, FlorestaAposChuva, Trovoada, RuidoSuave }` (nomes no JS: `stream, rain, forest, rainforest, thunder, softNoise`); `pub enum Respiracao { Desligada, Equilibrada, Lenta }` (`off, balanced, slow`); cada um com `proximo(self) -> Self` (volta ao primeiro depois do último), `texto(self) -> &'static str` (rótulos do JS: "Magic Puzzle", "Cozy Puzzle", "Space City"; "Riacho", "Chuva", "Floresta com cigarras", "Floresta após chuva", "Chuva com trovoada", "Ruído suave"; "Desligada", "4 s inspirar · 4 s expirar", "4 s inspirar · 6 s expirar") e `id_js()`; `pub struct Preferencias { pub sons_do_jogo: bool, pub volumes: Volumes, pub musica_ligada: bool, pub ambiente_ligado: bool, pub musica: Musica, pub ambiente: Ambiente, pub respiracao: Respiracao }` com `Default` (true, 80/70/65, false, false, CozyPuzzle, Chuva, Desligada); `pub fn preferencias_para_json(&Preferencias) -> String` e `pub fn json_para_preferencias(&str) -> Result<Preferencias, String>`; `Salvador::carregar_preferencias(&self) -> (Preferencias, Origem)` e `Salvador::gravar_preferencias(&self, &Preferencias) -> Result<(), String>`; `pub struct EstadoRespiracao { pub fase: Fase, pub restante_s: u32, pub progresso: f32 }`, `pub enum Fase { Inspire, Expire }`, `pub fn estado_da_respiracao(r: Respiracao, decorrido_ms: f32) -> Option<EstadoRespiracao>`, `pub fn orbe(estado: &EstadoRespiracao) -> (f32 /*escala*/, f32 /*opacidade*/)`.
- Consumes: `Origem`, `Salvador` e a limpeza de BOM de `formato.rs`/`salvar.rs` (M2).

Formato de `som.json` (nomes do JS): `{"soundOn":true,"audio":{"effects":80,"music":70,"ambience":65},"zen":{"music":false,"ambience":false,"musicTrack":"cozyPuzzle","ambienceSound":"rain","breath":"off"}}`. Leitura por campo, como `normalizeAudioSettings`/`normalizeZenSettings` do JS: número finito → arredonda e prende em 0..100, senão o padrão; booleano só vale se for `true`/`false` de verdade (`soundOn` ausente = `true`); texto fora da lista → padrão; campo ou objeto que falta → padrão. Raiz que não é objeto ou JSON inválido → erro.

Respiração (porte de `updateBreathStep`): tempos (inspirar, expirar) = 4/4 (Equilibrada) ou 4/6 (Lenta); `posicao = decorrido mod (a+b)`; Inspire se `posicao < a`; `restante = max(1, ceil(duração da fase − posição dentro da fase))`; `progresso` de 0 a 1 dentro da fase. Orbe (do CSS): inspirando vai de escala 0,72/opacidade 0,64 a 1,15/1,0 com suavização nas duas pontas; expirando o contrário. Tempo NaN/negativo vale 0; tempo enorme não dá pânico e continua dentro das faixas.

- [ ] **Step 1: Testes primeiro:** `padroes_sao_80_70_65_e_tudo_desligado`; `numeros_estranhos_entram_na_faixa_ou_viram_padrao` (150→100, −5→0, 72.6→73, `"70"` texto→padrão, `null`→padrão, `true`→padrão); `texto_fora_da_lista_vira_padrao` (`"musicTrack":"x"`→CozyPuzzle, `"breath":"foo"`→Desligada, `"music":"true"`→false, `"soundOn":0`→true); `objeto_ou_campo_que_falta_vira_padrao`; `ida_e_volta_do_json_mantem_tudo` (todas as combinações de enums); `json_ilegivel_e_recusado` (vazio, `{`, `[]`, `null`, `42`, só BOM); `bom_no_comeco_e_aceito`; `proximo_da_a_volta` (por enum); `salvador_sem_arquivo_devolve_padrao_e_origem_nova`; `salvador_grava_e_le`; `salvador_com_arquivo_estragado_guarda_copia_corrupt_e_devolve_padrao`; `salvador_sem_pasta_nao_da_panico` (grava devolve o mesmo resultado que os outros gravadores sem pasta); `gravacao_em_duas_etapas_nao_deixa_tmp`; `respiracao_equilibrada_ciclo_de_8s` (t=0 → Inspire, restante 4; t=3900 → restante 1; t=4000 → Expire, restante 4; t=7999 → Expire, restante 1; t=8000 → Inspire de novo); `respiracao_lenta_ciclo_de_10s`; `respiracao_desligada_nao_tem_estado`; `respiracao_tempo_invalido` (NaN, −1, 1e30, ∞ → sem pânico, `progresso` em 0..1, `restante_s` ≥ 1); `orbe_cresce_ao_inspirar_e_encolhe_ao_expirar` (pontas 0,72/0,64 e 1,15/1,0; monotônico).
- [ ] **Step 2: Rodar e ver falhar:** `LIBRARY_PATH=/tmp/asound cargo test -p prisma_game preferencias salvar` → Expected: não compila / FAIL.
- [ ] **Step 3: Implementar.** Arquivo `som.json` na pasta de dados, gravado como os outros do `Salvador` (temporário + troca de nome; ilegível vira `som.json.corrupt`).
- [ ] **Step 4: Rodar:** `LIBRARY_PATH=/tmp/asound cargo test --workspace` → Expected: tudo verde; clippy sem avisos novos.
- [ ] **Step 5: Commit** `feat(game): preferencias de som (som.json) e guia de respiracao`.

### Task 4: Quando cada efeito toca (gatilhos no `App`)

**Files:**
- Modify: `game/src/linha.rs` (`Passo::Limpeza` ganha `cadeia: u32` e `criou: bool`, preenchidos em `montar` a partir de `Event::Clear { chain, creations }`; ajustar os lugares que montam ou casam esse passo), `game/src/app.rs`

**Interfaces:**
- Consumes: `efeitos::{Efeito, Sinal, sinal_da_limpeza, elevacao_da_cadeia}` (Task 2), `preferencias::Preferencias` (Task 3: o `App` guarda `preferencias` e `App::com_preferencias(self, Preferencias) -> App` para os testes; a leitura do arquivo entra na Task 5).
- Produces: `pub fn sons(&mut self) -> Vec<Efeito>` (entrega e esvazia a fila, como `registro()`); linha de registro por efeito emitido: `PRISMA_M3 efeito sinal=<nome> elevacao=<n>`.

Quando toca (porte de `animateResult`): jogada inválida → `Invalida` no começo da troca; cada `Limpeza` → `sinal_da_limpeza(...)` com `elevacao_da_cadeia(cadeia)` no **começo** do passo (os tipos ativados vêm do `tabuleiro` do passo: `tabuleiro[casa].kind`); `Criacao` no **fim** da limpeza que criou pedra especial; `Resgate` no começo do passo de resgate; `Nivel` no começo do passo de nível; Embaralhar não toca. Só entra na fila se `sons_do_jogo` e `volumes.efeitos > 0`. Se um único `avancar` pular mais de 2000 ms, os passos pulados não tocam nada.

- [ ] **Step 1: Testes primeiro** (`app.rs`, com jogos de semente fixa como os testes do M1): `jogada_invalida_toca_so_invalida`; `limpeza_simples_toca_combinacao_elevacao_0` e `cascata_toca_cascata_com_elevacao_subindo` (a lista exata para uma semente com cascata, conferida a mão); `limpeza_que_cria_especial_toca_criacao_depois_da_combinacao`; `especial_ativado_toca_pulso_raio_espectro_ou_supernova` (um teste por tipo, montando o tabuleiro pelo `jogo_mut`); `subir_de_nivel_toca_nivel_depois_das_limpezas`; `resgate_toca_resgate`; `embaralhar_nao_toca`; `sons_do_jogo_desligado_ou_efeitos_zero_nao_enfileira_nada_e_a_animacao_e_a_mesma` (mesmos passos e mesmo placar); `salto_de_tempo_maior_que_2s_nao_enfileira_rajada` (`avancar(5000.0)` no meio da animação); `cada_efeito_sai_uma_vez_so` (andar quadro a quadro com `avancar(16.0)` até o fim e contar); `sons_esvazia_a_fila`; `o_registro_ganha_uma_linha_por_efeito` (formato exato acima).
- [ ] **Step 2: Rodar e ver falhar:** `LIBRARY_PATH=/tmp/asound cargo test -p prisma_game app::` → Expected: FAIL (e os testes antigos de `linha`/`app` ainda compilam depois da mudança do `Passo`).
- [ ] **Step 3: Implementar** em `observar()` e `jogar()`; a fila é um `Vec<Efeito>` no `App`.
- [ ] **Step 4: Rodar:** `LIBRARY_PATH=/tmp/asound cargo test --workspace` → Expected: tudo verde.
- [ ] **Step 5: Commit** `feat(game): gatilhos dos efeitos sonoros na animacao`.

### Task 5: Folha de Som no `App` e no layout (lógica e posições, sem desenhar)

**Files:**
- Modify: `game/src/layout.rs` (`TelaJogo.botao_som`, abas mais estreitas para caber o botão quadrado ♪ à direita; `TelaJogo.guia_respiracao: Rect` reservado abaixo dos botões **só no Zen**; `pub struct TelaSom` e `pub fn tela_som(largura, altura, zen: bool) -> TelaSom`), `game/src/app.rs`, `game/src/salvar.rs` não muda

**Interfaces:**
- Produces: `TelaSom { folha: Rect, titulo: Rect, sons_do_jogo: Rect, barra_efeitos: Rect, barra_musica: Rect, barra_ambiente: Rect, testar: Rect, liga_musica: Option<Rect>, escolhe_musica: Option<Rect>, liga_ambiente: Option<Rect>, escolhe_ambiente: Option<Rect>, respiracao: Option<Rect>, fechar: Rect, texto: f32 }` (as `Option` só existem no Zen; tudo vazio para tela impossível, como `TelaJogo::VAZIA`); `Botao::Som` e `Botao::FecharSom` (+ os controles da folha, com o `Botao` ou um `enum Controle` à escolha de quem implementa, desde que `botao_em`/`tocar` os tratem igual aos botões de hoje); `App::folha_aberta() -> bool`, `App::preferencias() -> Preferencias`, `App::guia_respiracao() -> Option<EstadoRespiracao>`, `App::aviso_de_audio() -> Option<&str>`, `App::erro_de_audio(canal: Canal, texto: Option<String>)` (a Task 6 chama), `App::abrir(salvador)` passa a ler `som.json` (`PRISMA_M3` não precisa de linha própria aqui).
- Consumes: Tasks 2–4 (`Canal` vem da Task 3).

Comportamento (porte do JS): a folha é **modal** (cobre da barra de progresso aos botões; tabuleiro, botões e abas de baixo não reagem; o botão ♪ ou Fechar fecha; a animação e os sons continuam por baixo). Barra: toque ou arrasto escolhe o valor de 5 em 5 pelo x (fora da barra, prende em 0 ou 100); `Cima`/`Cancelar` termina o arrasto e **grava uma vez**. "Sons do jogo": se estava ligado e `efeitos == 0`, põe `efeitos = 80` (não desliga) — senão alterna. "Testar som": se `efeitos == 0` mostra o aviso "Aumente o volume dos efeitos", senão enfileira `Combinacao` elevação 0 (mesmo com "Sons do jogo" desligado). Música/Ambiente: o botão liga alterna; o nome (`escolhe_*`) passa para a próxima opção; mudar qualquer um grava. Respiração: um toque passa para a próxima; recomeça do "Inspire" (`respiracao_ms = 0`). A respiração só anda no Zen; recomeça ao entrar no Zen, ao mudar a opção e no Novo jogo confirmado.

- [ ] **Step 1: Testes primeiro** (`layout.rs`): `tela_som_cabe_e_nao_se_sobrepoe` (para 360×640, 720×1280, 1080×1920, 1080×2400, e no Zen e no Clássico: todos os retângulos dentro de `folha`, sem sobreposição entre controles, cada linha tocável com altura ≥ 0,07 do menor lado); `no_classico_os_controles_do_zen_nao_existem`; `tela_impossivel_devolve_tudo_vazio`; `botao_som_fica_ao_lado_das_abas_e_nao_as_cobre`; `a_linha_da_respiracao_nao_encolhe_o_tabuleiro` (o `tabuleiro.w` no Zen é igual ao do Clássico nos 4 tamanhos); `guia_respiracao_so_existe_no_zen`; os testes de layout antigos continuam passando.
  (`app.rs`): `tocar_no_botao_som_abre_e_fecha`; `folha_aberta_bloqueia_tabuleiro_abas_e_botoes` (um toque num gesto do tabuleiro não troca pedras); `arrastar_a_barra_muda_o_valor_de_5_em_5_e_grava_uma_vez` (contar gravações com um `Salvador` numa pasta temporária); `arrasto_que_sai_da_barra_prende_em_0_e_100`; `cancelar_no_meio_do_arrasto_deixa_o_ultimo_valor_e_grava`; `sons_do_jogo_com_efeitos_zero_restaura_80_em_vez_de_desligar`; `testar_com_efeitos_zero_avisa`; `testar_toca_mesmo_com_sons_do_jogo_desligado`; `liga_e_escolhe_da_musica_e_do_ambiente_gravam`; `reabrir_mantem_as_preferencias` (App → muda → novo `App::abrir` na mesma pasta); `arquivo_de_preferencias_estragado_abre_com_padroes_e_o_jogo_funciona`; `abrir_a_folha_no_meio_da_animacao_nao_muda_o_resultado` (mesma pontuação/tabuleiro final com e sem a folha); `abrir_a_folha_com_fim_de_partida_ou_dialogo_aberto_nao_quebra`; `respiracao_anda_so_no_zen_e_recomeca_ao_voltar_ao_zen`; `respiracao_recomeca_ao_mudar_a_opcao_e_no_novo_jogo`; `salto_de_tempo_na_respiracao_nao_da_panico`; `erro_de_audio_aparece_e_some` (`erro_de_audio(Musica, Some(..))` → `aviso_de_audio()` com o texto do JS "O áudio não começou. Toque novamente em Música ou Ambiente para tentar de novo."; `None` limpa).
- [ ] **Step 2: Rodar e ver falhar:** `LIBRARY_PATH=/tmp/asound cargo test -p prisma_game layout app::` → Expected: FAIL.
- [ ] **Step 3: Implementar.** Livre para quem implementa: proporções dentro das restrições acima; o desenho é a Task 7.
- [ ] **Step 4: Rodar:** `LIBRARY_PATH=/tmp/asound cargo test --workspace` → Expected: tudo verde; clippy sem avisos novos.
- [ ] **Step 5: Commit** `feat(game): folha de Som, respiracao e preferencias no App`.

### Task 6: Música e ambiente: planejador puro e camada do Macroquad

**Files:**
- Create: `game/src/fundo.rs`, `game/src/audio.rs`
- Modify: `game/src/app.rs` (`som_desejado`), `game/src/main.rs` (`mod fundo; mod audio;`, laço do jogo)

**Interfaces:**
- Produces (`fundo.rs`, puro): `pub enum Faixa { Musica(Musica), Ambiente(Ambiente) }` com `caminho(&self) -> &'static str` (`"musica/cozy-puzzle.ogg"`, `"ambiente/rain-soft.ogg"`, ...: os nomes da Global Constraints; mapa: Riacho→`stream`, Chuva→`rain-soft`, Floresta→`forest-cicadas`, FlorestaAposChuva→`rainforest`, Trovoada→`rain-thunder`, RuidoSuave→`soft-noise`) e `nome(&self) -> &'static str` (sem pasta nem extensão); `pub struct Desejado { pub musica: Option<(Musica, f32)>, pub ambiente: Option<(Ambiente, f32)> }` (volume 0,0 a 1,0); `pub struct Atual` (mesma forma: o que está carregado, mesmo se falhou); `pub enum Comando { Parar(Canal), Carregar(Faixa, f32), Volume(Canal, f32) }`; `pub fn planejar(atual: &Atual, desejado: &Desejado) -> Vec<Comando>`; `App::som_desejado(&self) -> Desejado` (vazio fora do Zen; música/ambiente só se ligados; volume = canal/100).
- Produces (`audio.rs`, Macroquad): `pub struct Audio`; `pub async fn Audio::novo() -> Audio` (gera os 20 efeitos de `efeitos::todos()`, carrega cada WAV com `load_sound_from_bytes`, loga `PRISMA_M3 pronto efeitos=<n> ms=<tempo>`); `pub fn tocar(&self, efeito: Efeito, volumes: &Volumes)` (volume `efeitos/100 × GANHO_MESTRE`; qualquer pânico ou erro é engolido com `catch_unwind` como o `tocar_som` do M0b); `pub fn atual(&self) -> Atual`; `pub async fn aplicar(&mut self, comandos: Vec<Comando>, app: &mut App)`. Linhas do registro: `PRISMA_M3 carregado tipo=<musica|ambiente> nome=<nome> ms=<tempo>` e `PRISMA_M3 erro tipo=<musica|ambiente> nome=<nome> texto=<mensagem>`.
- Pasta dos sons: no Android, relativa aos assets do APK; no PC, `game/sons` (caminho de compilação, `env!("CARGO_MANIFEST_DIR")`) ou a pasta da variável de ambiente `PRISMA_SONS`, via `macroquad::file::set_pc_assets_folder`.

Regras do planejador: por canal, `desejado` vazio e `atual` cheio → `Parar`; mesma faixa com volume diferente (> 0,001) → `Volume`; faixa diferente → `Parar` da antiga **antes** do `Carregar` da nova; mesma faixa e mesmo volume → nada. `Atual` guarda a faixa tentada mesmo se o carregamento falhou (não tenta de novo até o jogador desligar e ligar). Música e ambiente tocam em laço.

- [ ] **Step 1: Testes primeiro** (`fundo.rs` e `app.rs`): `nada_desejado_e_nada_atual_nao_faz_nada`; `ligar_musica_carrega_uma_vez`; `mesmo_estado_nao_repete_comando`; `mudar_so_o_volume_so_manda_volume` (sem recarregar); `trocar_de_faixa_para_antes_de_carregar` (ordem `Parar` depois `Carregar`); `desligar_para`; `musica_e_ambiente_sao_independentes`; `ligar_e_desligar_varias_vezes_nunca_deixa_duas_faixas_no_mesmo_canal` (simulação de 200 passos aleatórios com semente fixa aplicando os comandos a um `Atual` de mentira e conferindo a invariante); `faixa_que_falhou_nao_e_tentada_de_novo_ate_desligar_e_ligar`; `caminhos_sao_relativos_ogg_e_sem_repeticao` (os 9); `som_desejado_e_vazio_no_classico_e_volta_ao_trocar_para_o_zen`; `som_desejado_usa_o_volume_do_canal_dividido_por_100`; `som_desejado_so_inclui_o_que_esta_ligado`.
- [ ] **Step 2: Rodar e ver falhar:** `LIBRARY_PATH=/tmp/asound cargo test -p prisma_game fundo som_desejado` → Expected: FAIL.
- [ ] **Step 3: Implementar** `fundo.rs` e `som_desejado`; depois `audio.rs` e o laço em `rodar_jogo`: criar o `Audio` depois do `Sprites::load()` só quando **não** for foto nem cena; a cada quadro, tocar `app.sons()`, calcular `planejar(&audio.atual(), &app.som_desejado())` e, se não vazio, `aplicar(...).await` (carregamento dentro do quadro; a trava é esperada e medida). Falha no carregamento → `app.erro_de_audio(canal, Some(texto))`; sucesso → `None`.
- [ ] **Step 4: Rodar:** `LIBRARY_PATH=/tmp/asound cargo test --workspace` e `cargo clippy -p prisma_game --all-targets -- -D warnings` → Expected: verdes. Um teste automático `faixa_ausente_vira_erro_e_nao_panico` (caminho que não existe → `Err` com texto, sem pânico, sem placa de som) cobre o Review Focus 1. Conferência no PC: `LIBRARY_PATH=/tmp/asound xvfb-run -a cargo run -q -p prisma_game` abre e fecha sem pânico com e sem a pasta `game/sons`; o som de verdade só se confere no celular (Task 9).
- [ ] **Step 5: Commit** `feat(game): musica e ambiente do Zen (planejador e camada do Macroquad)`.

### Task 7: Desenho da folha, do botão ♪ e da respiração; fotos

**Files:**
- Modify: `game/src/render.rs`, `game/src/cenas.rs` (cenas novas `som`, `som-classico`, `respiracao`), `game/src/main.rs` (cena nova no `match`/lista de cenas, se houver)

**Interfaces:**
- Consumes: `App::{folha_aberta, preferencias, guia_respiracao, aviso_de_audio}`, `layout::{TelaJogo, TelaSom}` (Task 5).
- Produces: `render::draw_jogo` desenha o botão ♪ (✕ quando aberta), a folha por cima do tabuleiro e dos botões (fundo escuro translúcido, título "Som", linhas com rótulo à esquerda e valor/opção à direita, barras com o preenchimento e o valor em %, "Testar som", "Fechar"), o aviso de áudio e, no Zen, a linha da respiração (bolinha com `orbe()`, "Inspire"/"Expire", "N segundo(s)"; escondida quando desligada, mas o espaço continua reservado). Textos em português; "1 segundo", "2 segundos".

- [ ] **Step 1: Cenas e fotos antes do desenho** (a "foto" é o teste visual): `cenas::cena("som")` (Zen, folha aberta, padrões, música ligada, respiração Equilibrada), `"som-classico"` (Clássico, só efeitos) e `"respiracao"` (Zen, guia visível com 2 s de respiração). Teste de unidade `as_cenas_de_som_existem_e_montam` (cada nome devolve um `App` com a folha aberta/guia ativo conforme o caso).
- [ ] **Step 2: Rodar e ver falhar** `LIBRARY_PATH=/tmp/asound cargo test -p prisma_game cenas` → Expected: FAIL (cenas desconhecidas).
- [ ] **Step 3: Implementar** as cenas e o desenho. Cuidar: texto que não cabe numa linha é cortado ou encolhido (nunca sai da folha); contraste legível; nada de texto sobre texto.
- [ ] **Step 4: Fotos e conferência visual** em 540×960 (janela padrão) e 360×640: gerar as três cenas, abrir cada PNG com o Read e conferir: nada sobreposto ou cortado, rótulos legíveis, barras proporcionais ao valor, a linha da respiração fora do tabuleiro, abas e botão ♪ alinhados. Corrigir e repetir até ficar certo; ledger com o que mudou.
- [ ] **Step 5: Rodar:** `LIBRARY_PATH=/tmp/asound cargo test --workspace` → Expected: verde. **Step 6: Commit** `feat(game): desenho da folha de Som e do guia de respiracao`.

### Task 8: Sons no APK (CI, assets, créditos)

**Files:**
- Modify: `game/Cargo.toml (`assets = "sons"` em `[package.metadata.android]`, com comentário), `Cargo.toml` da raiz (`[profile.dev.package."*"] opt-level = 2`, com comentário do porquê), `.github/workflows/android.yml`
- Create: `docs/CREDITOS-AUDIO.md`; Test: `game/tests/workspace.rs` (confere as duas linhas novas dos `Cargo.toml` e o `.gitignore`, como os outros testes de configuração)

- [ ] **Step 1: Teste primeiro** em `game/tests/workspace.rs`: `os_sons_vao_como_assets_do_android` (`assets = "sons"` presente), `dependencias_otimizadas_no_modo_de_desenvolvimento` (a seção `profile.dev.package."*"` com `opt-level = 2`), `a_pasta_de_sons_nao_entra_no_git` (`.gitignore` tem `game/sons/`, posto na Task 1), `os_creditos_de_audio_citam_os_9_trabalhos_e_a_licenca_cc_by` (o arquivo cita os 9 nomes de obra e "CC BY 4.0"). Rodar → Expected: FAIL.
- [ ] **Step 2: Implementar** as configurações e `docs/CREDITOS-AUDIO.md` (tabela do `LICENSE-ART.md` do Prisma em JS: obra, autor, licença, fonte; atribuição do "Ruído suave" com a nota "cortado em laço de 60 s e reconvertido a 44,1 kHz pelo `prisma_tools audio`"; a mesma nota para os outros cinco ambientes; músicas sem alteração; aviso de que a tela de créditos do app vem no M4).
- [ ] **Step 3: Workflow** (job `apk`): antes de gerar o APK, passo "Preparar os sons" — instalar o `ffmpeg` se faltar (`sudo apt-get install -y ffmpeg`), `cargo run --release --manifest-path tools/Cargo.toml -- audio game/sons`; depois de gerar e assinar, passo que reprova se o APK não tiver os 9 arquivos `assets/musica/*.ogg` e `assets/ambiente/*.ogg` (`unzip -l`) e que imprime o tamanho do APK e reprova acima de 60 MB (`::error::` em português). O job `emulador` não muda de estrutura.
- [ ] **Step 4: Rodar:** `LIBRARY_PATH=/tmp/asound cargo test --workspace` → Expected: verde (incluindo os testes novos). Conferir o YAML com `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/android.yml'))"` → Expected: sem erro.
- [ ] **Step 5: Commit** `ci: sons como assets do APK, otimizacao das dependencias e creditos do audio`. (O CI só roda com autorização do Thiago, na Task 9.)

### Task 9: Teste do celular virtual, medidas e notas

**Files:**
- Modify: `game/src/main.rs` (`linhas_de_geometria`: botão `som` sempre; com a folha aberta também `fechar_som`, `liga_musica`, `liga_ambiente`; a geometria é escrita de novo quando a folha abre ou fecha), `tools/src/registro.rs`, `tools/src/emulador.rs`, `tools/tests/amostra-logcat.txt`, `docs/notes/android-build.md`
- Create: `docs/notes/audio.md`

**Interfaces:**
- Produces (`registro.rs`): `Linha::{SomPronto{efeitos: u32, ms: u64}, Efeito{sinal: String, elevacao: u8}, FaixaCarregada{tipo: String, nome: String, ms: u64}, ErroDeSom{texto: String}}`; `Registro::{som_pronto, efeitos_tocados, faixa_carregada(tipo), primeiro_erro_de_som}` (lê as linhas `PRISMA_M3` da Constraint e da Task 4/6).
- Produces (`emulador.rs`): **Fase 10 — som**, depois das fases do M2: (a) o registro tem `PRISMA_M3 pronto`; (b) uma jogada válida no Zen gera pelo menos um `PRISMA_M3 efeito`; (c) toca em `som`, espera a geometria da folha, toca em `liga_musica` e espera `carregado tipo=musica` (até 20 s), toca em `liga_ambiente` e espera `carregado tipo=ambiente` (até 20 s); (d) nenhum `PRISMA_M3 erro`; (e) o app continua vivo e uma jogada seguinte ainda termina (`animacao_terminou`); foto `10-som`; mede `dumpsys meminfo` do app (linha TOTAL) e escreve no resultado, sem reprovar por ele.

- [ ] **Step 1: Testes primeiro** (`registro.rs`, `emulador.rs`): leitura das 4 linhas novas (válidas e malformadas: faltando campo, número que não é número, tipo desconhecido); `fase_som_passa_no_celular_de_mentira`; falhas, cada uma com mensagem que diz o que faltou: `fase_som_reprova_sem_pronto`, `..._sem_efeito_apos_jogada`, `..._musica_nao_carrega_no_prazo`, `..._ambiente_nao_carrega_no_prazo`, `..._erro_de_som_no_registro`, `..._app_morre_depois_de_ligar_a_musica`, `..._jogada_seguinte_nao_termina`; `o_registro_de_sons_nao_confunde_com_as_linhas_do_m2` (as amostras do M2 continuam lidas igual). Estender o celular de mentira. Rodar → Expected: FAIL.
- [ ] **Step 2: Implementar** o app (geometria) e a fase; amostra de logcat com as linhas novas.
- [ ] **Step 3: Rodar:** `cargo test --manifest-path tools/Cargo.toml` (+ `--no-default-features`), clippy dos dois conjuntos, `LIBRARY_PATH=/tmp/asound cargo test --workspace` → Expected: tudo verde. Passada de mutação nas checagens novas (como no M2: apagar/negar cada checagem e ver o teste correspondente falhar; sem `-q`).
- [ ] **Step 4: Notas:** `docs/notes/audio.md` (como os sons são preparados, por que laços de 60 s, 21 MB por minuto, onde ficam no APK, o que conferir se um áudio não toca, como rodar no PC); `docs/notes/android-build.md` (fase 10 e o passo "Preparar os sons"); atualizar na spec o item "tamanho do APK com as músicas" com o número medido (depois do CI).
- [ ] **Step 5: Commit** `test(tools): fase 10 do celular virtual confere o som`.
- [ ] **Step 6 (precisa de autorização do Thiago): enviar a branch e rodar o `android.yml`** nela; ler o resultado (as duas versões do Android: `pronto`, `carregado` das duas faixas, tamanho do APK, `meminfo`). Se algo falhar, corrigir com teste que falha antes.

---

## Conferência no celular (lista para o Thiago, depois do APK pronto)

1. Jogar no Zen e no Clássico: ouvir os efeitos (combinação, cascata mais aguda a cada nível da cadeia, especiais, criação, subir de nível, jogada inválida, resgate).
2. Botão ♪: abrir a folha; mexer nas três barras (sentir se respondem ao dedo); "Testar som"; "Sons do jogo" liga/desliga.
3. Zen: ligar a música (pode travar um instante ao carregar), trocar a faixa, desligar; o mesmo com os 6 ambientes (ouvir se o laço de 1 min emenda sem estalo).
4. Respiração: ligar as duas opções e ver a bolinha e os segundos.
5. Tirar o app da tela (segundo plano): o som para; voltar: continua.
6. Fechar e abrir o app: volumes, escolhas e respiração continuam; instalar o APK novo por cima sem desinstalar mantém tudo.
7. Clássico: só efeitos, sem música nem ambiente.
8. Tamanho do APK baixado (esperado ~42 MB) e se o celular aguenta música + ambiente ao mesmo tempo sem engasgar.
