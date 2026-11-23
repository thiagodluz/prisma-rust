# Acabamento do Prisma em Rust (M4)

O M4 deixou o jogo com cara de acabado: vibração, brilho (partículas e a intensidade dos efeitos),
os Recordes e Créditos dentro do app, o aviso "Carregando…", o ícone certo no celular e as versões
do Android. O desenho completo está em `docs/superpowers/specs/2026-11-22-prisma-fase2-m4-design.md`;
aqui está o que é bom ter à mão para mexer nisso depois.

## Vibração

- Código: `game/src/vibracao.rs` (qual toque cada jogada pede) e `game/src/plataforma/android.rs`
  (`tem_vibrador` e `vibrar_padrao`, pela ponte JNI).
- Está **desligada de início**. O jogador liga no interruptor "Vibração", dentro da folha do ♪
  (nos dois modos). Fica guardada no mesmo arquivo das outras preferências (`som.json`).
- **Padrões** (em milissegundos; no Android o primeiro número é a espera antes de começar):

  | Quando | Padrão | O que o jogador sente |
  |---|---|---|
  | jogada válida comum | `[0, 28]` | um toque curto |
  | jogada que ativou uma pedra especial | `[0, 45]` | um toque um pouco mais longo |
  | jogada que subiu de nível | `[0, 28, 55, 35]` | vibra, pausa, vibra |
  | acabou de ligar o interruptor | `[0, 55]` | um toque de confirmação |

  Subir de nível vence "ativou especial", que vence a jogada comum (a mesma ordem do JS). Troca
  inválida **não vibra**. O volume dos efeitos não interfere.
- **Por que o interruptor some:** no começo o app pergunta ao Android se o aparelho tem vibrador
  (`tem_vibrador`). Sem vibrador, a linha "Vibração" nem aparece na folha. No PC a pergunta
  responde "sim" de mentira, para a foto e os testes mostrarem o interruptor.
- Se o aparelho **recusar** o toque de "ligou", o interruptor volta a desligado e a preferência é
  gravada assim, sem mensagem de erro na tela (registro: `PRISMA_M4 vibracao recusada`).
- A chamada usa `Vibrator.vibrate(long[], int)` com repetição `-1` (toca uma vez só). É uma forma
  antiga, mas existe em todas as versões. Um erro da ponte vira `Err` e nunca derruba o app.

## Partículas (efeitos da limpeza)

Código: `game/src/particulas.rs` (cada efeito é uma função do tempo, para ser testável) e
`desenhar_particulas` em `game/src/render.rs`. Os valores vêm do `app.js` e do `style.css` do
Prisma em JS; as curvas e as cores finas foram ajustadas olhando fotos (cenas `particulas-*`).

| Efeito | Quando | Como é | Duração |
|---|---|---|---|
| Faíscas | toda limpeza e em volta de cada especial | quadradinhos brilhantes que saem do centro e somem. Comum: 5. Especial: 8. Espectro: 12 | 380 ms |
| Onda | Pulso e Raio | anel que cresce e some | 360 ms |
| Feixes | Raio | uma linha na horizontal e outra na vertical cruzando o tabuleiro | 330 ms |
| Véu | Espectro | véu claro sobre o tabuleiro | 370 ms |
| Supernova | Supernova | anel grande de 5 casas, na cor da Supernova | 480 ms |
| Fusão | Supernova | linhas das pedras de origem até a casa da Supernova, **antes** da explosão | 240 ms |
| Pulso das pedras | pedras que somem | crescem um pouco e brilham antes de sumir; as mais longe da especial atrasam até 100 ms | 270 a 340 ms |

- Alcance das faíscas (em casas): 1,25 no comum e no Pulso, 1,65 no Raio, 2,2 no Espectro.
- Regras que vêm do JS: no máximo 3 especiais com efeito por limpeza (mais as Supernovas); uma
  limpeza sem especial faz uma só nuvem de faíscas, no meio das pedras que somem.
- **Teto:** no máximo 96 partículas por quadro (`MAXIMO_DE_PARTICULAS`). Se passar, perdem-se
  primeiro as faíscas; o anel, a linha e o véu ficam, porque são eles que mostram o efeito.
- O brilho das pedras é um halo de 3 círculos brancos translúcidos (não há shader no Macroquad).
  Fica um pouco menos "estourado" que o filtro de brilho do JS.
- Para ver: `cargo run -p prisma_game -- --cena particulas-pulso --screenshot pulso.png` (também
  `-raio`, `-espectro`, `-supernova`, `-espectro-suave` e `-espectro-vibrante`; no PC use
  `xvfb-run` se não houver tela).

## Intensidade dos efeitos (só no Zen)

- Três valores, passando ao próximo a cada toque na linha "Intensidade" da folha de Som (só no
  Zen): **Suave**, **Padrão** (o de início) e **Vibrante**. No Clássico vale sempre Padrão, mesmo
  que o Zen esteja em Vibrante. Muda só o visual (nem o som, nem a vibração, nem os pontos).
- Valores (iguais aos do JS):

  | | Suave | Padrão | Vibrante |
  |---|---|---|---|
  | duração dos efeitos | × 0,82 | × 1 | × 1,1 |
  | faíscas | 5 por especial e 3 por combinação comum | os da tabela acima | os da tabela acima |
  | pulso das pedras (tamanho / brilho) | 1,1 / 1,35 | 1,1 / 1,8 | 1,18 / 2,1 |
  | camada de efeitos | 48% de opacidade, cores menos vivas | normal | mais viva e mais clara |
  | faixa "Nível N" (sombra / brilho roxo) | 0,5 / 0 | 1 / 0,52 | 1 / 0,73 |

- O rótulo é "Intensidade" (o do JS, "Intensidade dos efeitos", não cabe na largura de 360 px).
- Guardada no `som.json`; arquivos antigos sem o campo voltam para Padrão.

## Menu ≡: Recordes e Créditos

- O botão ≡ fica ao lado do ♪, na linha das abas (o tabuleiro não muda de lugar). Abre uma folha
  do mesmo tamanho e lugar da de Som, com as abas **Recordes** (abre nela) e **Créditos** e o ✕.
  Só uma folha aberta por vez: abrir uma fecha a outra. Código: `game/src/menu.rs`.
- **Recordes:** os dois blocos aparecem juntos (Zen em cima, Clássico embaixo), cada um lido dos
  recordes do seu modo, em tempo real: Zen (maior pontuação, maior
  nível, melhor jogada) e Clássico (os mesmos mais partidas concluídas). Números com separador de
  milhar à brasileira (`12.345`).
- **Créditos:** 6 itens curtos, sem link (não há internet nem navegador a abrir), que cabem em
  360×640 sem rolagem. O texto mora num lugar só (`menu.rs`) e um teste confere que cita GPLv3,
  CC BY-SA 4.0, CC BY 4.0 e Geoff Bremner.

## Aviso "Carregando…"

Carregar uma música ou um ambiente trava o quadro por uns instantes (de 0,3 a 0,8 s nos celulares
virtuais). Por isso, antes de carregar uma faixa que **ainda não está carregada**, o jogo desenha
um quadro com "Carregando música…" ou "Carregando ambiente…" no lugar do título da folha de Som, e
só no quadro seguinte carrega. O aviso some quando acaba (com sucesso ou erro). Faixa já carregada
não mostra aviso. Se música e ambiente forem pedidos juntos, o aviso mostra só a música e as duas
cargas acontecem no mesmo quadro seguinte.

## Ícone

- O ícone do app é o **adaptativo** do Android 8 em diante: a figura (o losango do `icon-512.png`)
  sobre um fundo liso (`#222547`, a média dos cantos do ícone), com a figura dentro da zona segura
  (61% da altura, centrada) para não ser cortada em círculo, quadrado arredondado etc. Os ícones
  quadrados comuns continuam como reserva.
- **Como gerar:** o `prisma_tools imagens` produz tudo, sem arquivo à mão:
  `cargo run --release --manifest-path tools/Cargo.toml -- imagens <pasta-do-prisma-js> game`.
  Saem `mipmap-*/ic_launcher_foreground.png` (5 tamanhos: 108, 162, 216, 324 e 432 px),
  `mipmap-anydpi-v26/ic_launcher.xml` e `values/ic_launcher_background.xml`, tudo em
  `game/android_res/`. Atenção: rodar a ferramenta com o JS real também regrava os PNG antigos com
  bytes diferentes dos do M0; só os arquivos novos do ícone devem entrar no commit (use
  `git status` e `git checkout` nos antigos).
- **Como olhar a prévia:** monta-se fora do repositório (um script em `/tmp`) o primeiro plano
  sobre o fundo, com uma máscara redonda e outra de quadrado arredondado, e abre-se a imagem: o
  losango tem de aparecer inteiro e legível. (No ícone real a figura ocupa as colunas 139 a 292 e
  as linhas 84 a 347 de 432 px.)
- O CI confere no APK que o ícone aponta para `mipmap-anydpi-v26/ic_launcher.xml`.

## Versões do Android

- `min_sdk_version = 26` (Android 8: o ícone adaptativo e o vibrador novo valem a partir daí) e
  `target_sdk_version = 34`, em `[package.metadata.android]` de `game/Cargo.toml`.
- **Por que 34 e não 35:** com alvo 35 o Android 15 desenha o app até as bordas da tela, e o CI só
  tem celulares virtuais com Android 30 e 34, então ninguém veria o problema antes de ele chegar ao
  celular do Thiago. Subir para 35 pede cuidar das bordas primeiro.
- `tests/workspace.rs` (`as_versoes_do_android_estao_fixadas`) fixa as duas linhas, e o passo
  **"Conferir versões e ícone do APK"** do fluxo lê o `aapt2 dump badging` e reprova se o mínimo
  não for 26, se o alvo não for 34 ou se o ícone não for o adaptativo.
- Se o gerador de APK não aceitar as chaves ou a imagem Docker não tiver a plataforma 34, volta-se
  às versões de antes (o pedaço foi feito em commits só dele) e anota-se aqui o motivo.

## Linhas novas no registro (`PRISMA_M4`)

| Linha | Quando |
|---|---|
| `PRISMA_M4 vibracao toque=curto\|especial\|nivel\|ligou` | o app pediu uma vibração |
| `PRISMA_M4 vibracao recusada` | o aparelho recusou o toque de "ligou"; a vibração voltou a desligada |
| `PRISMA_M4 carregando tipo=musica\|ambiente` | o app vai carregar uma faixa (sai **antes** do `PRISMA_M3 carregado`) |
| `PRISMA_M4 intensidade valor=suave\|padrao\|vibrante` | o jogador trocou a intensidade |
| `PRISMA_M4 vibracao erro texto=..` | a ponte de vibração falhou (o teste do celular virtual não usa) |

E os botões novos na lista de posições: `menu` (sempre), `fechar_menu`, `aba_recordes` e
`aba_creditos` (só com o menu aberto), `liga_vibracao` (só com a folha de Som aberta e aparelho com
vibrador) e `escolhe_efeitos` (só no Zen, com a folha de Som aberta).

## O que só o celular de verdade confirma

O celular virtual do CI confere que o pedido de vibração **saiu**, mas não sente nada. Faltam,
para o Thiago olhar no aparelho dele:

1. sentir os três padrões (jogada comum, com especial, subida de nível) e o toque ao ligar;
2. ver as partículas nas três intensidades no Zen (Suave bem mais discreto que Vibrante);
3. ver o ícone adaptativo na tela inicial (sem corte, com a figura inteira);
4. abrir o ≡ e ler Recordes e Créditos (letras legíveis, nada cortado);
5. ver o "Carregando…" aparecer ao ligar a música no celular real (no virtual dura menos de um
   segundo).

## Medido no celular virtual (CI)

Fluxo do Android na branch do M4. A primeira execução ainda mostrava só um modo na aba Recordes;
a segunda, já com a correção dos dois blocos (Zen e Clássico), passou nos 3 jobs e a foto do
Android 34 mostra os dois blocos, cada um com os seus números e nada cortado:

- **APK:** 48.744.723 bytes (cerca de 46 MB; o limite é 60 MB), assinado com a chave fixa.
- **`aapt2 dump badging`:** `minSdkVersion` 26, `targetSdkVersion` 34 e o ícone de todas as
  densidades apontando para `res/mipmap-anydpi-v26/ic_launcher.xml` (ícone adaptativo).
- **Fase 11 no Android 34 (x86_64):** o menu abriu, mostrou Recordes e Créditos e fechou; ligar a
  vibração pediu o toque `ligou`; a intensidade passou a `vibrante`; a jogada pediu o toque `curto`;
  `carregando tipo=musica` e `carregando tipo=ambiente` apareceram antes de cada `carregado`.
- **Fase 11 no Android 30 (x86):** o mesmo, com a jogada pedindo o toque `nivel`.
- **Fotos:** Recordes e Créditos legíveis, nada cortado, nas duas versões.
- O que o celular virtual **não** confirma continua igual: sentir os toques, ver as partículas nas
  três intensidades e ver o ícone na tela inicial só se faz em um celular de verdade.
