# Como o APK é gerado (e por quê)

## Resumo

O GitHub Actions gera o APK a cada envio para a `main` (fluxo **Android**, arquivo
`.github/workflows/android.yml`). O resultado vai para a release `apk-mais-recente` (endereço
fixo, sem zip) e também fica como "artefato" da execução, com o nome `prisma-debug-apk`.
O APK sai **assinado com uma chave fixa** (a mesma em toda execução), então cada APK novo instala
por cima do anterior e o jogo continua com a partida e os recordes salvos (ver "Assinatura").

## Gerar na mão (se um dia for preciso)

Precisa de Docker. Na pasta do projeto:

```
docker run --rm -v "$PWD":/root/src -w /root/src/game notfl3/cargo-apk sh -c '
  rustup update stable && rustup default stable &&
  rustup target add armv7-linux-androideabi aarch64-linux-android i686-linux-android &&
  cargo quad-apk build'
```

O APK sai em `target/android-artifacts/debug/apk/`. Para a versão "release", acrescentar
`--release` no fim. Feito na mão assim, o APK sai com uma chave de teste do próprio gerador (não a
chave fixa do app; ver "Assinatura").

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
- **Imagens dentro do executável** (`include_bytes!`); os efeitos sonoros são gerados pelo código.
  **Música e ambiente são a exceção:** os 9 arquivos OGG (uns 15 MB) vão na pasta `assets` do
  APK (`assets = "sons"` em `game/Cargo.toml`, que vira `assets/musica/` e `assets/ambiente/`).
  Eles não ficam no git: o passo **"Preparar os sons"** do fluxo os recria com
  `prisma_tools audio game/sons` antes de gerar o APK (veja `docs/notes/audio.md`).
  Depois de assinar, o passo **"Conferir os sons e o tamanho do APK"** reprova se faltar algum
  dos 9 arquivos dentro do APK ou se o APK passar de **60 MB**, e imprime o tamanho.

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

O Android só deixa instalar um APK por cima de outro se os dois foram assinados com a **mesma
chave**; só assim o app novo herda os dados do antigo (partida e recordes). Por isso o fluxo
assina todo APK com **uma chave fixa**.

- **Onde fica a chave:** nos segredos do repositório no GitHub (Settings → Secrets and variables →
  Actions): `PRISMA_RUST_KEYSTORE_BASE64` (o arquivo da chave, em base64) e
  `PRISMA_RUST_KEYSTORE_PASSWORD` (a senha). O arquivo da chave e a senha **nunca** entram no
  git; só a impressão digital pública do certificado fica no repositório, em
  `tools/assinatura-sha256.txt`. O GitHub não deixa ler um segredo de volta: **guarde uma cópia
  do arquivo da chave e da senha** (por exemplo num gerenciador de senhas), fora do GitHub.
- **Como o fluxo assina** (trabalho `apk` de `.github/workflows/android.yml`): gera o APK sem
  assinar (`cargo quad-apk build --nosign`), alinha (`zipalign`), assina (`apksigner`, alias
  `prisma`) e **confere** se a impressão digital do certificado é a de
  `tools/assinatura-sha256.txt`; se não for, o fluxo reprova em vez de publicar. Sem os segredos,
  reprova com uma mensagem apontando esta seção. O arquivo da chave só existe numa pasta
  temporária do executor e é apagado num passo que roda sempre. O artefato, a release e o
  celular virtual usam o mesmo APK assinado.
- **Número da versão:** o `versionCode` do app é o número da execução do fluxo (o fluxo reescreve
  a linha `version_code` de `game/Cargo.toml` só no executor, sem commitar); o Android não aceita
  instalar por cima uma versão com número menor.
- **Como conferir:** no registro do passo "Assinar o APK com a chave fixa" aparecem `Verified
  using v2 scheme ...: true` e `Signer #1 certificate SHA-256 digest: ...`, que tem de ser igual
  ao conteúdo de `tools/assinatura-sha256.txt`. Num APK baixado:
  `apksigner verify --print-certs prisma-rust.apk` (vem nas `build-tools` do Android SDK).
- **A primeira vez:** o APK de antes desta mudança foi assinado com uma chave de teste que mudava
  a cada execução; desinstale-o uma vez antes de instalar o primeiro APK com a chave fixa (os
  dados dele se perdem). Depois disso, é só instalar por cima.
- **Conferido no celular (22/11/2026):** o Thiago instalou o APK do M2, jogou Zen e Clássico,
  fechou pelos apps recentes e abriu de novo: voltou no modo, com a partida de cada modo e os
  recordes (inclusive depois de Novo jogo e com o app fechado no meio da animação).
- **Se a chave se perder:** gere outra (`keytool -genkeypair -v -keystore prisma-rust.keystore
  -alias prisma -keyalg RSA -keysize 2048 -validity 10000`), converta em base64 (`base64 -w0
  prisma-rust.keystore`), troque os dois segredos, troque o conteúdo de
  `tools/assinatura-sha256.txt` (a linha `SHA256:` de `keytool -list -v -keystore
  prisma-rust.keystore`, em minúsculas e sem os dois-pontos) e **todo mundo desinstala e instala
  de novo uma vez**, perdendo os dados salvos do app.

## Teste no celular virtual (emulador)

Depois de gerar o APK, o fluxo **Android** tem um segundo trabalho (`emulador`) que **joga o
Prisma** num emulador do Android (versões 30 e 34), como um dedo de verdade:

1. abre o app, espera 15 segundos e confere que continua aberto e abriu direto no jogo
   (`PRISMA_M1 pronto modo=zen`) e **calibra o toque**: o app mede os toques a partir do canto
   da própria janela, mas o `adb input` usa coordenadas da tela, e no emulador a janela começa
   abaixo da barra de status (uns 24 a 28 px). O programa toca num ponto da margem de cima (que não
   faz nada no jogo), lê no registro onde o app viu esse toque (`PRISMA_M1 toque x=.. y=..`) e
   soma a diferença a todos os toques seguintes;
2. **Zen:** 5 vezes, toca em "Dica", lê no registro quais são as duas pedras da dica, **arrasta
   o dedo** de uma até a outra (`adb shell input draganddrop`), espera a animação terminar e confere
   que a jogada valeu (`jogada valida=true`) e que os pontos subiram;
3. **Clássico:** toca na aba, confere que começou com 0 pontos e faz 5 jogadas tocando numa
   pedra e depois na vizinha (`adb shell input tap`);
4. toca em "Novo jogo" e em "Confirmar" e confere que os pontos voltaram a 0;
5. volta para a aba Zen e confere que os pontos do Zen continuam lá;
6. **fecha o app de verdade** (`adb shell am force-stop`, como tirá-lo dos apps recentes), apaga o
   registro e abre de novo: o app tem de voltar no Zen (`PRISMA_M2 aberto modo=zen ...`), com a
   partida do Zen vinda **do arquivo** com os mesmos pontos, a do Clássico (zerada pelo Novo
   jogo) e recordes (`melhor_zen`, `melhor_classico`) que cobrem tudo o que foi jogado;
7. **Clássico em separado:** toca na aba Clássico, faz uma jogada, **espera até 5 s o app avisar
   que gravou** (`PRISMA_M2 gravado modo=classico pontos=..`: só depois é seguro fechar), fecha e
   abre de novo: agora o app tem de voltar **no Clássico** (o modo ativo foi guardado), com a
   jogada, e o Zen intacto;
8. **atualiza por cima:** instala o mesmo APK de novo (`adb install -r`, como o jogador fará
   quando sair uma versão nova, já que a chave é fixa) e abre: modo, partidas e recordes iguais;
9. confere que os recordes do fim não são menores que os da primeira reabertura;
10. **som (M3):** confere que o app avisou que os efeitos ficaram prontos
    (`PRISMA_M3 pronto efeitos=..`, nenhum `PRISMA_M3 erro`), vai para o Zen e faz uma jogada que
    tem de **pedir pelo menos um efeito** (`PRISMA_M3 efeito ..`); toca no **♪** e espera a folha
    de Som abrir (até 10 s: o app só escreve a posição dos botões dela, `liga_musica`,
    `liga_ambiente` e `fechar_som`, quando ela abre); liga a **música** e o **ambiente**, cada um
    com **até 20 s para carregar** (`PRISMA_M3 carregado tipo=musica|ambiente ..`; um
    `PRISMA_M3 erro` ou o app fechando nesse meio-tempo reprova na hora); tira a foto
    `10-som` (com a folha aberta); **mede a memória** do app (`adb shell dumpsys meminfo`: imprime
    as linhas `TOTAL` e guarda `meminfo.txt`; só informa, nunca reprova); fecha a folha e faz mais
    uma jogada, que tem de terminar com a música e o ambiente tocando;
11. **acabamento (M4):** toca no **≡** e espera os Recordes e Créditos abrirem (até 10 s: só
    então o app escreve a posição de `aba_recordes`, `aba_creditos` e `fechar_menu`; conta a
    geometria **nova**, porque a de uma abertura anterior continua no registro); tira as fotos
    `11-recordes` e `11-creditos` e fecha. Abre a folha de Som de novo e **liga a vibração**
    (`liga_vibracao`, que só existe se o aparelho tem vibrador): tem de aparecer
    `PRISMA_M4 vibracao toque=ligou`. Um celular virtual sem vibrador só imprime um `AVISO` e
    segue; se o aparelho recusar o toque (`PRISMA_M4 vibracao recusada`) também só avisa. Toca em
    `escolhe_efeitos` (só no Zen) e exige uma linha **nova** `PRISMA_M4 intensidade valor=..`.
    **Desliga e liga o ambiente** e exige o `PRISMA_M3 carregado tipo=ambiente` **novo** (o da fase
    10 continua no registro e não vale; por isso a espera conta quantos `carregado` já existiam) e
    que o `PRISMA_M4 carregando tipo=ambiente` tenha vindo **antes** dele. Fecha a folha e, se a
    vibração ficou ligada, faz uma jogada que tem de pedir um toque (`PRISMA_M4 vibracao toque=..`).
    No fim o app tem de estar aberto e sem nenhum `PRISMA_M3 erro`.

Se o app avisar no registro que não conseguiu gravar (`PRISMA_M2 erro_ao_gravar <motivo>`), o
teste para na hora com esse motivo. Como o `logcat -c` de cada reabertura apaga o registro, o de
cada sessão fica guardado à parte (`logcat-sessao-0.txt`, `-1`, `-2`; o da última sessão é o
`logcat.txt`), e duas fotos novas saem: `8-reaberto-zen` e `9-atualizado`. O app do teste não é
"depurável", então o teste não consegue estragar um arquivo de dentro do celular para ver o app
se recuperar; essa parte (arquivo estragado vira cópia `.corrupt` e só aquele item recomeça) é
coberta pelos testes do próprio jogo (`game/src/salvar.rs` e `game/src/app.rs`).

O app escreve no registro do Android linhas que começam com `PRISMA_M1` (função `App::registro`
em `game/src/app.rs` e `rodar_jogo` em `game/src/main.rs`): a posição do tabuleiro e dos botões,
**em pixels reais da tela** (para o teste saber onde tocar), e o que acontece no jogo
(`toque x=.. y=..`, `dica a=.. b=..`, `jogada valida=.. pontos=.. nivel=..`,
`animacao_terminou`, `modo=..`, `novo_jogo ..`, `fim ..`) e, desde o M2 (salvar), linhas que
começam com `PRISMA_M2`: `aberto modo=.. zen=<novo|arquivo|estragado>:<pontos> classico=..
melhor_zen=.. melhor_classico=..` (o que ele leu ao abrir), `gravado modo=.. pontos=..` e
`erro_ao_gravar <motivo>`. Desde o M3 há também as linhas `PRISMA_M3` do som (`pronto`, `efeito`,
`carregado`, `erro`: veja `docs/notes/audio.md`) e os botões `som`, `fechar_som`, `liga_musica` e
`liga_ambiente` na lista de posições (os três últimos só enquanto a folha de Som está aberta).
Desde o M4 há as linhas `PRISMA_M4` (`vibracao toque=..`, `vibracao recusada`, `carregando tipo=..`,
`intensidade valor=..`: veja `docs/notes/acabamento.md`) e os botões `menu`, `fechar_menu`,
`aba_recordes`, `aba_creditos`, `liga_vibracao` e `escolhe_efeitos`. O teste lê essas linhas. Ele guarda o registro de erros
(`logcat.txt`) e uma foto da tela de cada etapa como artefato `emulador-android-<versão>`, e
imprime um resumo no próprio registro da execução (as fotos saem em texto, entre
`TELA_BASE64_INICIO` e `TELA_BASE64_FIM`).

**Quem faz o teste:** o programa `prisma_tools` (Rust, pasta `tools/`, fora do workspace do jogo
para o gerador de APK nunca ver as dependências dele). No fluxo ele é compilado só com a parte do
emulador (`--no-default-features`, sem nenhuma dependência) e roda assim:
`tools/target/release/prisma_tools emulador apk resultado`. Termina com código 1 se qualquer
conferência falhar, e nesse caso ainda guarda o registro, imprime o resumo e a foto da tela.
Os testes dele (com um "celular falso" que imita o `adb` e o jogo) rodam em qualquer
computador, sem emulador: `cargo test --manifest-path tools/Cargo.toml`. O programa de
`adb` pode ser trocado com a variável `ADB`, e cada chamada do `adb` tem limite de 60 s.

**Por que `draganddrop` e não `swipe`:** o app lê a posição do dedo **uma vez por quadro**
(`mouse_position()` em `rodar_jogo`), e os quadros do emulador do GitHub são lentos. Com `swipe`
o dedo já andou quando o app "vê" o toque, e o arrasto pode começar na casa do lado (derrubou o
Android 14 numa execução, `toque x=198` para um arrasto que saiu de `x=179`). O `draganddrop`
segura o dedo parado uns 0,5 s no começo e só depois anda; assim o ponto de partida registrado é
exatamente o pedido.

**Imagens do jogo:** o mesmo programa gera as imagens e os ícones a partir da pasta do Prisma em JS:
`cargo run --release --manifest-path tools/Cargo.toml -- imagens <pasta-do-prisma-js> game`.
Os PNG já estão no repositório; só precisa rodar se a origem mudar. Só grava depois de ter tudo
pronto, então uma imagem de origem errada não deixa o jogo pela metade. Um teste à parte compara
com os PNG do repositório (rodado só sob pedido):
`PRISMA_JS=<pasta-do-prisma-js> cargo test --manifest-path tools/Cargo.toml -- --ignored`.

O "teste de vida" do M0b (contador de aberturas, som, vibração) saiu do emulador; a tela dele
continua no app e abre com `--tela diagnostico` no PC.

O APK só vira a release `apk-mais-recente` quando o fluxo roda na branch `main`; numa branch
de trabalho ele só gera o APK e roda o emulador (dá para disparar à mão:
`gh workflow run android.yml --ref <branch>`).

Limites: o emulador usa a biblioteca de PC (x86), não a do celular (arm64); um erro que só
existe no arm64 não aparece nele. Serve para pegar erros de programa e de montagem do APK.

## Versões do Android e ícone adaptativo (M4)

- `game/Cargo.toml` fixa `min_sdk_version = 26` e `target_sdk_version = 34` (por que 34 e não 35:
  `docs/notes/acabamento.md`). O teste `as_versoes_do_android_estao_fixadas` guarda as duas linhas.
- Depois de gerar o APK, o passo **"Conferir versões e ícone do APK"** lê o `aapt2 dump badging`
  inteiro e reprova se `minSdkVersion` não for 26, se `targetSdkVersion` não for 34 ou se os
  `application-icon-*` não apontarem para `mipmap-anydpi-v26/ic_launcher.xml`. Os três valores
  aparecem no registro do passo.
- O ícone é o **adaptativo**: `game/android_res/mipmap-anydpi-v26/ic_launcher.xml`, a figura em 5
  tamanhos (`ic_launcher_foreground.png`) e a cor de fundo (`values/ic_launcher_background.xml`),
  tudo gerado por `prisma_tools imagens` (veja "Imagens do jogo" acima). Os ícones quadrados comuns
  ficam como reserva.
- Se o gerador de APK não aceitar as chaves ou a imagem Docker não tiver a plataforma 34, a saída
  é voltar às versões de antes (o pedaço está em commits só dele) e anotar aqui o motivo, sem
  mexer na imagem Docker.

## Por que o `package_name` termina em `prisma_game`

O cargo-quad-apk escreve no app "carregue a biblioteca chamada <último pedaço do package_name>",
mas grava o arquivo com o nome do programa (`libprisma_game.so`). Com o nome antigo
(`...prisma.rust`) o app procurava `librust.so`, não achava e fechava ao abrir
(`UnsatisfiedLinkError: library "librust.so" not found`, visto no emulador). Por isso os dois
nomes precisam bater; o teste `ultimo_pedaco_do_package_name_e_o_nome_do_programa` guarda isso.
Como o `package_name` é o identificador do app, o app novo é outro para o Android: o
"Prisma (Rust)" antigo, que nunca abriu, pode ser desinstalado.
