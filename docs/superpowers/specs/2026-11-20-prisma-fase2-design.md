# Prisma em Rust — Fase 2: tela, toque, som, saves e Android

**Estado:** rascunho para aprovação do Thiago · 2026-11-20
**Antes dele:** Fase 1 pronta (`prisma_engine`, motor sem dependências, 72 testes, idêntico ao `engine.js`).
**Referência do jogo original:** `github.com/thiagodluz/Prisma`, versão 1.4.2 (só leitura).

## 1. Intenção

Terminar o Prisma em Rust, jogando no celular Android, por diversão e aprendizado. É um projeto à parte: o Prisma em JS continua do jeito que está. O Android é o destino; o PC é só a bancada onde o desenvolvimento e os testes acontecem.

## 2. Objetivos, não-objetivos e anti-objetivos

**Objetivos**
- Um APK instalável que joga o Prisma de verdade (Zen e Clássico) usando o motor Rust.
- Tudo em Rust: tela, toque, som e saves. Só entra Java se não houver outro jeito.
- Aprender Rust no caminho: código simples, comentários em português explicando os conceitos novos, como na Fase 1.
- Cada marco termina num APK que o Thiago instala e joga.

**Não-objetivos (por enquanto)**
- Play Store, release assinada para publicação, AAB, iOS, versão web (WebAssembly).
- Cópia 1:1 de todos os detalhes do JS: o estilo visual "Anterior", o menu de desenvolvedor e a preferência do sistema por menos movimento só entram se ele pedir.
- Mudar regras. O motor é a verdade; qualquer ajuste de regra acontece no motor, com teste contra o JS.

**Anti-objetivos (é fracasso mesmo que "funcione")**
- Regra de jogo duplicada na tela. A tela só mostra o que o motor decidiu.
- Depender de o Thiago instalar Android Studio ou o SDK. O APK é gerado no GitHub Actions.
- Código que o Thiago não consegue ler.
- O motor ganhar dependências.

## 3. Fatos verificados e restrições

**Neste ambiente de trabalho (nuvem):**
- Não compila para Android nem para WebAssembly: não há SDK/NDK do Android, e o download dos alvos do Rust é bloqueado.
- Abre janela numa tela virtual (Xvfb) com OpenGL por software e tira foto dela. Testado com um programa Macroquad descartável: o desenho saiu correto. Então o visual pode ser conferido aqui.
- O crates.io é acessível.

**Macroquad 0.4.16** (conferido no código da biblioteca):
- Imagens: só PNG e TGA. Os WebP e o JPG do Prisma precisam virar PNG.
- Áudio: a feature `audio` é opcional e precisa ser ligada. Lê WAV e OGG; **não lê MP3**. No Android usa OpenSL ES.
- Compila em cerca de 11 s no modo de desenvolvimento.

**Android com Macroquad:**
- O APK sai de `cargo quad-apk build`, de preferência dentro do Docker `notfl3/cargo-apk` (tutorial oficial do Macroquad).
- Gravar arquivo: a biblioteca `quad-storage` grava um arquivo `local.data` em plataformas nativas, sem escolher a pasta, e o último release é de 2021. Não confio nela para o Android. O caminho que desenho é obter o diretório privado do app (`getFilesDir()`) por JNI, usando `miniquad::native::android::attach_jni_env()` e `ACTIVITY`, que pelo menos uma outra biblioteca (`ply-engine`) já usa. Isso ainda não foi testado: o M0b existe para prová-lo.
- Vibração: também por JNI, mais a permissão `VIBRATE` no manifesto.
- **Assinatura:** cada execução do GitHub Actions cria uma chave de debug nova. O Android recusa instalar por cima de um APK assinado com outra chave, então cada atualização exigiria desinstalar o app e perderia os saves. Por isso o desenho usa uma chave fixa (seção 5.7).

**Recursos do JS (para reaproveitar):**
- Imagens: três atlas WebP de 1254×1254 (grade 3×3 com sete pedras e duas casas vazias: comuns, Pulso e Raio), `spectrum-gem.webp` e fundo `prisma-bg.jpg` 940×1672.
- Áudio: três músicas OGG e seis ambientes (quatro OGG e **dois MP3**), todos CC0, fora do repositório. O `scripts/download-audio.mjs` os restaura da release v1.4.2 e confere cada SHA-256. Os efeitos de jogo são sintetizados em `sound.js`, sem arquivo.
- Zen: respiração 4/4 ou 4/6 s, três intensidades de efeito, música e ambiente com volumes separados (efeitos 80, música 70, ambiente 65 por padrão).
- Licenças: as cinco imagens são CC BY-SA 4.0, © Thiago Luz (o nome Prisma e os ícones ficam fora da licença). O código do JS é GPL-3.0-only. Os créditos do áudio estão em `LICENSE-ART.md` do JS.

## 4. Abordagens consideradas

| Caminho | Prós | Contras | Resultado |
| --- | --- | --- | --- |
| **Macroquad** | Biblioteca 2D simples (desenhar, tocar, ler toque); tutorial oficial de Android; compila rápido. | A ferramenta de APK (`cargo-quad-apk`) é o elo mais frágil e existe um fork novo para Rust 2024, sinal de que o original pode estar defasado. | **Escolhida** |
| Slint | Suporte oficial a Android; ótimo para menus. | Pensado para interface, não para tabuleiro animado; som viria de outra biblioteca. | Não escolhida |
| Bevy | Motor completo; Android pelo caminho padrão (`cargo-ndk` + Gradle). | Mais pesado: ECS é um segundo assunto grande; builds lentos. | Não escolhida |
| Motor Rust dentro da tela web atual (WebView/Tauri) | Android fácil. | A tela continua em JS; contraria "tudo em Rust". | Descartada |

**Plano B** se o `cargo-quad-apk` travar no M0: gerar o APK por Gradle + `cargo-ndk`, que é o caminho que o projeto Android do JS já usa. Não é desenhado agora.

## 5. Desenho

### 5.1 Repositório

A raiz continua sendo o motor (`prisma_engine`, sem dependências). A raiz ganha `[workspace] members = ["game"]`, e nasce a pasta `game/` com o pacote `prisma_game` (binário) que depende do motor por caminho e do `macroquad`. `cargo test` na raiz continua testando só o motor; `cargo test --workspace` testa tudo.

### 5.2 Módulos de `game/` (cada um com um trabalho)

- `layout`: tamanho da tela → retângulos do tabuleiro, do painel e dos botões. Puro e testável.
- `input`: toque/mouse → "tocou numa casa" ou "arrastou de uma casa para a vizinha" → pedido de troca. Máquina de estados pura e testável. As regras de toque e arrasto seguem `app.js` (linhas ~649–735).
- `playback`: eventos do motor (`MoveResult.events`) → linha do tempo de animação (troca, limpeza, queda, embaralhar, resgate, subida de nível, fim). Puro e testável (ordem e durações).
- `render`: desenha o estado atual e o quadro da animação com as texturas.
- `assets`: carrega os PNG embutidos no executável (`include_bytes!`).
- `audio`: efeitos sintetizados, música, ambientes, volumes.
- `storage`: save, recordes e preferências em JSON no armazenamento privado do app.
- `app`: o laço principal e a máquina de estados da tela (jogando, animando, menu, fim de jogo, diálogo de novo jogo).

### 5.3 Fluxo de uma jogada

Ponteiro → `input` → `Game::play(a, b)` → `MoveResult` → **salvar logo** (como o JS, que salva antes das animações) → `playback` monta a linha do tempo → `render` e `audio` a executam → a entrada fica travada até o fim → volta a esperar. O estado do motor já é o final; a animação só mostra o caminho. Se o app for para o segundo plano, a animação pula direto para o fim e o áudio pausa.

### 5.3b Tela (retrato), como no JS

```
┌─────────────────────────┐
│ ZEN                 🔊  │
│ Pontos 12.450   Nível 7 │
│ ▓▓▓▓▓▓░░░░  3.200/4.050 │
│ ┌─────────────────────┐ │
│ │    tabuleiro 8×8    │ │
│ └─────────────────────┘ │
│  [Dica] [Embaralhar]    │
│        [Novo jogo]      │
└─────────────────────────┘
```

Só o estilo "Novo" (pedras ilustradas, o padrão do JS) vem primeiro.

### 5.4 Imagens

O comando `prisma_tools imagens` (Rust, pasta `tools/`) lê os WebP da pasta do Prisma em JS, recorta as sete pedras de cada atlas (grade 3×3), centraliza cada pedra pelo pixel visível (alfa acima de 50%, como o JS faz), reduz ao tamanho do jogo e grava PNG em `game/assets/` (e os ícones do Android em `game/android_res/`). Os PNG entram no repositório, junto com o aviso de licença CC BY-SA 4.0 e a atribuição.

### 5.5 Som

- **Efeitos:** portar as notas de `sound.js` (frequência, duração, volume, forma de onda) para Rust, gerando as amostras em memória, montando bytes WAV e carregando com o Macroquad. Os testes conferem duração e volume máximo; quem julga o som é o ouvido do Thiago.
- **Música e ambientes:** arquivos OGG. Os dois MP3 (`stream`, `rainforest`) precisam virar OGG (ffmpeg na preparação) ou ficam de fora até lá. Os arquivos não entram no git: um script os baixa e confere o SHA-256, como o `download-audio.mjs`. O tamanho do APK é conferido no M3.
- **Volumes:** três canais (efeitos, música, ambiente), de 0 a 100.

### 5.6 Saves e preferências

JSON em arquivo no diretório privado do app (Android: `getFilesDir()` via JNI; PC: pasta de dados do usuário). Formato próprio, com os mesmos campos do `SavedGame`. O leitor repete as checagens listadas em `docs/notes/js-quirks.md` (modo conhecido, números inteiros, booleanos de verdade, tipos de pedra conhecidos) e recusa o save inteiro se alguma falhar; o motor faz o resto em `restore`. Uma partida por modo e recordes por modo, como no JS. Save corrompido vira cópia `.corrupt` e começa jogo novo. Biblioteca de JSON: `serde_json` só no pacote do jogo.

### 5.7 Android

- Pacote `com.thiagodluz.prisma.rust`, **diferente** do `com.thiagodluz.prisma` do JS, para os dois conviverem no celular.
- Retrato fixo; minSdk 26 e targetSdk 35, como o JS, se a ferramenta permitir; sem permissão de internet; `VIBRATE` só entra no M4.
- Ícone a partir de `icon-512.png`.
- **Assinatura:** a mesma chave do Prisma em JS (decisão do Thiago em 22/11/2026, no lugar de uma chave nova só para este app), guardada como segredo do GitHub do `prisma-rust` com os mesmos quatro nomes que o JS usa (`PRISMA_KEYSTORE_BASE64` e os outros). Os pacotes são diferentes (`com.thiagodluz.prisma` e `com.thiagodluz.prisma_game`), então os dois apps convivem no celular. Do M2 em diante isso é obrigatório (os saves dependem de atualizar por cima); no M0 e no M1 reinstalar é aceitável.
- Instalação: baixar o APK do resultado (artefato) do GitHub Actions no próprio celular.

### 5.8 Integração contínua (GitHub Actions)

Dois fluxos pequenos: (1) testes (`cargo test --workspace`); (2) Android (Docker `notfl3/cargo-apk`, `cargo quad-apk build`), publicando o APK como artefato. Rodam a cada envio para a `main` e sob demanda.

### 5.9 Como verificar

- Lógica pura (`layout`, `input`, `playback`, JSON de save, síntese de som): `cargo test`.
- Visual: o jogo ganha um modo de "foto" (`--screenshot arquivo.png`, com estado e quadro fixos) que roda na tela virtual; as imagens são conferidas a olho, não por comparação automática.
- Android: o CI garante que o APK compila; o resto é uma lista de conferência por marco, feita no celular do Thiago.
- Motor: nada muda; os 72 testes continuam passando. Mudança no motor exige teste contra o JS.

## 6. Marcos (cada um termina num APK instalável)

- **M0a · Olá, tabuleiro.** Workspace pronto; `game/` abre uma janela e desenha um tabuleiro do motor (semente fixa) com as imagens reais; o CI gera o APK. *Pronto quando:* o APK instala no celular do Thiago e mostra o tabuleiro de pé.
- **M0b · Teste de vida do Android.** Tela de diagnóstico com um contador de aberturas gravado em arquivo, um botão que toca um som e um botão que vibra. *Pronto quando:* os três funcionam no celular. Prova cedo as três coisas específicas do Android (arquivo, som, vibração).
- **M1 · Jogar.** Toque e arrasto, animações na ordem dos eventos, Zen e Clássico, Dica, Embaralhar, Novo jogo, painel (pontos, nível, progresso, melhor), subida de nível e fim de jogo no Clássico. *Pronto quando:* uma partida completa de cada modo roda no celular sem erro visual evidente.
- **M2 · Salvar.** Retomar a partida de cada modo e guardar recordes, com a chave fixa de assinatura. *Pronto quando:* fechar e abrir o app (e atualizar o APK por cima) mantém partida e recordes.
- **M3 · Som.** Efeitos sintetizados, depois música, ambientes do Zen, respiração e volumes. *Pronto quando:* o Zen tem os três canais ajustáveis e o APK continua num tamanho razoável.
- **M4 · Acabamento.** Vibração, partículas, intensidade de efeitos do Zen, ícone final.
- **Depois, se ele quiser:** estilo "Anterior", menu de desenvolvedor (os cheats já existem no motor), release assinada, Play Store.

Cada marco recebe o seu plano de implementação só quando o anterior termina, para aproveitar o que foi aprendido.

## 7. Riscos

1. O `cargo-quad-apk` pode não funcionar com o Rust atual → o M0a vem primeiro; Plano B na seção 4.
2. O Android não é testado no ambiente de trabalho → o M0 prova cedo; cada marco tem lista de conferência no celular.
3. Arquivo e vibração por JNI nunca foram testados aqui → M0b.
4. Chave de assinatura mudando a cada CI → chave fixa (5.7).
5. Tamanho do APK com as músicas → conferido no M3.
6. Desempenho em celular fraco → medir no celular do Thiago a partir do M1.

## 8. Escolhas minhas (a confirmar)

- Raiz como motor e `game/` como membro do workspace, em vez de mover o motor de pasta.
- `serde_json` no jogo (o motor continua sem dependências).
- Efeitos sonoros sintetizados em Rust, em vez de gravar WAV com outra ferramenta.
- PNG recortados e versionados em `game/assets/`; áudio baixado e verificado por script, fora do git.
- Mesma chave de assinatura do Prisma em JS (decisão do Thiago em 22/11/2026; a primeira ideia era uma chave própria, separada).
- Marcos pequenos, cada um com seu plano.

## 9. Fica para quem constrói

Tamanho em pixels das pedras, cores e fontes do painel, curvas e durações das animações, nomes de funções e tipos, detalhes de layout, estrutura exata do `playback`.

## 10. Decisões do Thiago, sem pressa

- Licença do código do `prisma-rust` (o JS usa GPL-3.0-only).
- Repositório público ou privado no GitHub.
- Guardar a chave de assinatura: resolvido, é a do Prisma em JS, que ele já guarda (segredos no GitHub e cópia de segurança fora dele); falta só cadastrar os segredos no `prisma-rust`.
