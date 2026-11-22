# Som do Prisma em Rust

O jogo tem três tipos de som, cada um com o seu volume (0 a 100) na folha do ♪:

- **Efeitos** (nos dois modos): 20 sons curtos, **gerados pelo próprio código** na hora em que o
  jogo abre (`game/src/efeitos.rs`: notas, formas de onda e volumes portados do `sound.js` do
  Prisma em JS). Viram WAV na memória e são carregados pelo Macroquad. Não existe arquivo de efeito.
- **Música** (só no Zen): 3 faixas OGG, uma tocando por vez.
- **Ambiente** (só no Zen): 6 faixas OGG, uma tocando por vez, em laço.

O Clássico só tem os efeitos. Música e ambiente começam **desligados** e o jogador liga na folha.

## Como as músicas e os ambientes são preparados

Os 9 arquivos **não ficam no git** (`/game/sons/` está no `.gitignore`). O comando

```
cargo run --release --manifest-path tools/Cargo.toml -- audio game/sons
```

(código em `tools/src/audio.rs`; precisa do `ffmpeg` instalado) recria a pasta do zero:

1. baixa o HTML da release v1.4.2 do Prisma em JS, que traz os 9 áudios embutidos;
2. tira cada áudio de dentro dele e confere o SHA-256 (o mesmo do `download-audio.mjs` do JS);
3. grava as **músicas como vieram** e prepara os **ambientes** com o `ffmpeg`: 44 100 Hz e, se
   passarem de 63 s, viram um **laço de exatamente 60 s** (um trecho do meio, com 3 s de
   transição suave entre o fim e o começo, para repetir sem estalo). Os dois ambientes que eram
   MP3 (`stream`, `rainforest`) viram OGG nessa passagem.

No fluxo do Android isso roda no passo **"Preparar os sons"**, antes de gerar o APK. Os créditos
e as licenças de cada obra estão em `docs/CREDITOS-AUDIO.md` (uma delas, o "Ruído suave", é
CC BY 4.0 e **exige atribuição**).

## Por que laços de 60 s? (a conta da memória)

O Macroquad (e o `quad-snd` por baixo) **decodifica o arquivo inteiro para a memória** antes de
tocar: são 44 100 amostras por segundo × 2 canais × 4 bytes = uns **21 MB por minuto**. Os
ambientes originais chegam a 12 minutos (mais de 250 MB só de um som), o que derrubaria o app.
Com o laço de 60 s cada ambiente pesa uns 21 MB na memória (10 MB o `soft-noise`, que é mono).

As músicas ficam inteiras (de 117 s a 124 s, uns 41 a 44 MB cada, decodificadas). Pela conta,
**música + ambiente ligados** passam de 60 MB de memória (estimativa). A fase 10 do celular
virtual mede o real (`dumpsys meminfo`, linhas `TOTAL`, arquivo `meminfo.txt` no artefato) com os
dois ligados. Na primeira execução (emulador x86, música Cozy Puzzle e ambiente Chuva), o **PSS
total do app inteiro** ficou em **131,8 MB no Android 30** e **179,2 MB no Android 34**. Os dois
carregaram sem erro: a música em 0,5 s (Android 30) e 1,0 s (Android 34), o ambiente em 0,06 s e
0,14 s; os 20 efeitos ficaram prontos em 0,17 s e 0,63 s.

O `Audio` guarda **no máximo uma música e um ambiente** por vez; ao trocar de faixa, a anterior é
parada antes de a nova carregar (`parar` em `game/src/audio.rs`).

## Onde ficam os arquivos

| Onde | Caminho |
| --- | --- |
| No git | nenhum (só os créditos, em `docs/CREDITOS-AUDIO.md`) |
| No PC (para jogar e testar) | `game/sons/musica/*.ogg` e `game/sons/ambiente/*.ogg`; a variável `PRISMA_SONS` troca a pasta |
| Dentro do APK | `assets/musica/*.ogg` e `assets/ambiente/*.ogg` (`assets = "sons"` em `game/Cargo.toml`) |

Os sons somam uns 15 MB. O APK sem eles tinha 26,9 MB; com os 9 sons ele ficou com
**48 084 723 bytes (uns 46 MiB; o fluxo escreve "cerca de 45 MB")**, medido no CI. O fluxo reprova
acima de **60 MB**.

## Linhas que o app escreve no registro

Para o celular virtual (e para quem lê o `logcat`) o app escreve linhas `PRISMA_M3 ...`
(`game/src/audio.rs` e `App::registro` em `game/src/app.rs`):

| Linha | Quando |
| --- | --- |
| `pronto efeitos=<n> ms=<t>` | os efeitos terminaram de carregar (`n` deve ser 20) |
| `efeito sinal=<nome> elevacao=<n>` | um efeito foi pedido ao áudio (uma jogada pode pedir vários) |
| `carregado tipo=<musica\|ambiente> nome=<nome> ms=<t>` | uma faixa de fundo terminou de carregar |
| `erro tipo=<efeito\|musica\|ambiente> nome=<nome> texto=<motivo>` | um som falhou; o jogo segue e a folha de Som mostra um aviso |

## Se um som não toca

1. **Efeito mudo, com a folha avisando algo:** olhe o `texto=` da linha `PRISMA_M3 erro`.
2. **Música ou ambiente não começa:** procure `PRISMA_M3 carregado tipo=...`. Se só aparece `erro`
   com "não consegui ler ...", o arquivo não está no lugar: no PC rode o comando `audio` de novo
   (ou confira `PRISMA_SONS`); no celular, confira no passo "Preparar os sons" do CI e na lista
   `unzip -l` do APK se os 9 arquivos entraram.
3. **Carrega, mas demora:** o `ms=` da linha `carregado` diz quanto. No PC, o perfil de
   desenvolvimento compila as dependências com `opt-level = 2` (`[profile.dev.package."*"]` no
   `Cargo.toml` da raiz); sem isso a decodificação em modo `debug` levava uns 5 s por faixa e
   passou a levar uns 0,5 s.
4. **O app fecha ao ligar música e ambiente:** suspeite de memória (veja o `meminfo.txt` e o
   resumo do `logcat` no artefato do celular virtual).
5. **Segundo plano:** o jogo não faz nada de especial com o áudio quando o app sai da tela; o plano
   supõe que o Macroquad pausa e retoma o som sozinho. Isso **ainda não foi visto num celular**:
   está na lista de conferências do M3 (tirar o app da tela: o som para; voltar: continua).

## Como jogar com som no PC

```
cargo run --release -p prisma_game
```

(depois de rodar o comando `audio` acima uma vez). Sem a pasta `game/sons` o jogo abre normal,
só que sem música e ambiente, com o aviso na folha.
