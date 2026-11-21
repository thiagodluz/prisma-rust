# Como o APK é gerado (e por quê)

## Resumo

O GitHub Actions gera o APK a cada envio para a `main` (fluxo **Android**, arquivo
`.github/workflows/android.yml`). O resultado vai para a release `apk-mais-recente` (endereço
fixo, sem zip) e também fica como "artefato" da execução, com o nome `prisma-debug-apk`.

## Gerar na mão (se um dia for preciso)

Precisa de Docker. Na pasta do projeto:

```
docker run --rm -v "$PWD":/root/src -w /root/src/game notfl3/cargo-apk sh -c '
  rustup update stable && rustup default stable &&
  rustup target add armv7-linux-androideabi aarch64-linux-android i686-linux-android &&
  cargo quad-apk build'
```

O APK sai em `target/android-artifacts/debug/apk/`. Para a versão "release", acrescentar
`--release` no fim (ela ainda não é assinada com chave própria; ver "Assinatura").

## Por que o projeto é assim

- **Edition 2021, não 2024.** A ferramenta `cargo-quad-apk` (0.1.4) carrega por dentro um
  Cargo de 2022, que recusa `edition = "2024"` ("this version of Cargo is older than the
  `2024` edition"). O teste `tests/workspace.rs` avisa se alguém subir a edition.
  Existe um fork da ferramenta para a edition 2024 (`hoklims/cargo-quad-apk`), que é a
  reserva se o original deixar de funcionar.
- **`Cargo.lock` na versão 3, só no CI.** O mesmo Cargo antigo não lê a versão 4 do arquivo
  ("lock file version `4` was found, but this version of Cargo does not understand this
  lock file"). O fluxo troca a primeira linha de versão com `sed` antes de gerar o APK; o
  arquivo do repositório não muda.
- **O Rust do contêiner é atualizado a cada execução.** A imagem `notfl3/cargo-apk` traz o
  Rust 1.79 (de 2024) e o motor usa `Option::is_none_or`, que só existe a partir do 1.82.
  Por isso o comando começa com `rustup update stable` e instala de novo os alvos do Android.
  O Cargo antigo da ferramenta só lê os arquivos do projeto; quem compila é o `rustc` novo.
- **O comando roda de dentro de `game/`.** A ferramenta ignora `-p prisma_game` e usa o
  pacote da pasta onde está. Da raiz ela pegaria o motor (que não depende do `miniquad`) e
  pararia com "no miniquad is found in the dependencies tree".
- **Imagens dentro do executável** (`include_bytes!`). O APK não leva a pasta `assets`.

## Instalar no celular

1. No celular, abrir <https://github.com/thiagodluz/prisma-rust/releases/download/apk-mais-recente/prisma-rust.apk>
   (a página da release é <https://github.com/thiagodluz/prisma-rust/releases/tag/apk-mais-recente>).
   Não precisa de login nem de zip; o Android baixa o `.apk` direto.
2. Abrir o `.apk` baixado. O Android pede para permitir "instalar apps desconhecidos" para o
   navegador usado.
3. O app aparece como **Prisma (Rust)** (identificador `com.thiagodluz.prisma_game`, diferente
   do `com.thiagodluz.prisma` do JS, então os dois convivem).

A release `apk-mais-recente` é apagada e recriada a cada envio para a `main`, sempre com o
APK novo. Por segurança o fluxo também guarda o APK como artefato da execução
(`prisma-debug-apk`, em zip).

## Assinatura (importante)

No modo debug, **cada execução do GitHub Actions usa uma chave de teste nova**. O Android não
deixa instalar um APK por cima de outro assinado com chave diferente: é preciso **desinstalar o
app antes de instalar o seguinte** (e os dados dele se perdem). Isto é aceitável até o M1.
No M2 (salvar partida) entra uma chave fixa guardada como segredo do GitHub.

## Teste no celular virtual (emulador)

Depois de gerar o APK, o fluxo **Android** tem um segundo trabalho (`emulador`) que **joga o
Prisma** num emulador do Android (versões 30 e 34), como um dedo de verdade:

1. abre o app, espera 15 segundos e confere que continua aberto e abriu direto no jogo
   (`PRISMA_M1 pronto modo=zen`);
2. **Zen:** 5 vezes, toca em "Dica", lê no registro quais são as duas pedras da dica, **arrasta
   o dedo** de uma até a outra (`adb shell input swipe`), espera a animação terminar e confere
   que a jogada valeu (`jogada valida=true`) e que os pontos subiram;
3. **Clássico:** toca na aba, confere que começou com 0 pontos e faz 5 jogadas tocando numa
   pedra e depois na vizinha (`adb shell input tap`);
4. toca em "Novo jogo" e em "Confirmar" e confere que os pontos voltaram a 0;
5. volta para a aba Zen e confere que os pontos do Zen continuam lá.

O app escreve no registro do Android linhas que começam com `PRISMA_M1` (função `App::registro`
em `game/src/app.rs` e `rodar_jogo` em `game/src/main.rs`): a posição do tabuleiro e dos botões,
**em pixels reais da tela** (para o script saber onde tocar), e o que acontece no jogo
(`dica a=.. b=..`, `jogada valida=.. pontos=.. nivel=..`, `animacao_terminou`, `modo=..`,
`novo_jogo ..`, `fim ..`). O script lê essas linhas. Ele guarda o registro de erros
(`logcat.txt`) e uma foto da tela de cada etapa como artefato `emulador-android-<versão>`, e
imprime um resumo no próprio registro da execução (as fotos saem em texto, entre
`TELA_BASE64_INICIO` e `TELA_BASE64_FIM`). Script: `tools/teste-emulador.sh`.

O "teste de vida" do M0b (contador de aberturas, som, vibração) saiu do emulador; a tela dele
continua no app e abre com `--tela diagnostico` no PC.

O APK só vira a release `apk-mais-recente` quando o fluxo roda na branch `main`; numa branch
de trabalho ele só gera o APK e roda o emulador (dá para disparar à mão:
`gh workflow run android.yml --ref <branch>`).

Limites: o emulador usa a biblioteca de PC (x86), não a do celular (arm64); um erro que só
existe no arm64 não aparece nele. Serve para pegar erros de programa e de montagem do APK.

## Por que o `package_name` termina em `prisma_game`

O cargo-quad-apk escreve no app "carregue a biblioteca chamada <último pedaço do package_name>",
mas grava o arquivo com o nome do programa (`libprisma_game.so`). Com o nome antigo
(`...prisma.rust`) o app procurava `librust.so`, não achava e fechava ao abrir
(`UnsatisfiedLinkError: library "librust.so" not found`, visto no emulador). Por isso os dois
nomes precisam bater; o teste `ultimo_pedaco_do_package_name_e_o_nome_do_programa` guarda isso.
Como o `package_name` é o identificador do app, o app novo é outro para o Android: o
"Prisma (Rust)" antigo, que nunca abriu, pode ser desinstalado.
