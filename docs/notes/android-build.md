# Como o APK é gerado (e por quê)

## Resumo

O GitHub Actions gera o APK a cada envio para a `main` (fluxo **Android**, arquivo
`.github/workflows/android.yml`). O resultado fica como "artefato" da execução, com o nome
`prisma-debug-apk`.

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

1. Na página da execução do fluxo **Android** no GitHub, baixar o artefato `prisma-debug-apk`
   (um `.zip`).
2. Extrair o `.zip` e abrir o `.apk`. O Android pede para permitir "instalar apps
   desconhecidos" para o navegador ou o gerenciador de arquivos usado.
3. O app aparece como **Prisma (Rust)** (identificador `com.thiagodluz.prisma.rust`, diferente
   do `com.thiagodluz.prisma` do JS, então os dois convivem).

## Assinatura (importante)

No modo debug, **cada execução do GitHub Actions usa uma chave de teste nova**. O Android não
deixa instalar um APK por cima de outro assinado com chave diferente: é preciso **desinstalar o
app antes de instalar o seguinte** (e os dados dele se perdem). Isto é aceitável até o M1.
No M2 (salvar partida) entra uma chave fixa guardada como segredo do GitHub.
