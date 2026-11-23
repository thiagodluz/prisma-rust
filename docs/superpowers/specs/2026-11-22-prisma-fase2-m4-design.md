# Prisma em Rust · Fase 2 · M4 (Acabamento) — design

Este documento detalha o marco M4 da spec principal (`2026-11-20-prisma-fase2-design.md`, seção 6). Fica a cargo do plano de implementação dividir em tarefas. Referência de comportamento: o Prisma em JS (`/home/claude/thiagodluz/prisma`, só leitura): `app.js` (linhas 370–375 e 595 para a vibração; 181–182 e 414–545 para os efeitos visuais), `style.css` (linhas 75–78 e 135–150), `index.html` (linhas 100–135 para recordes e créditos).

## 1. Intenção

Deixar o jogo com cara de acabado: tato (vibração), brilho (partículas e a intensidade dos efeitos no Zen), identidade (ícone certo no celular) e as peças que faltam para o jogo ser distribuído com honestidade (recordes visíveis, créditos e licenças dentro do app, aviso de carregamento da música, versão do Android correta). Ao fim, o jogo em Rust é equivalente ao Prisma em JS no que o jogador vê, sente e ouve.

O Thiago escolheu, nesta conversa: (a) o escopo é os quatro itens da spec **mais** as pendências anotadas; (b) **uma branch só e um APK no fim**, sem dividir em duas partes.

## 2. O que entra, em ordem

1. Vibração.
2. Aviso "Carregando…" na folha de Som.
3. Botão ≡ com a folha de Recordes e Créditos.
4. Partículas (efeitos visuais da limpeza).
5. Intensidade dos efeitos no Zen (Suave, Padrão, Vibrante).
6. Ícone adaptativo.
7. Versão mínima/alvo do Android (por último, em commits só dela, para desfazer sozinha se der problema).

A ordem põe o que é pequeno e certo primeiro e o que é arriscado por último. As partículas são a parte maior; se o tempo apertar, a ordem de valor dentro delas é: faíscas e brilho das pedras → ondas e feixes → supernova e fusão.

## 3. Fora do escopo e anti-objetivos

- Não mexer no motor (raiz `src/`) nem nas regras, na pontuação ou nos recordes.
- Sem permissão de internet. Nenhuma permissão nova (a `VIBRATE` já existe desde o M0b).
- Sem estilo "Anterior", menu de desenvolvedor, release assinada, Play Store (ficam em "Depois, se ele quiser").
- Sem "reduzir movimento" do sistema: o JS lê essa preferência, o M4 não.
- Os efeitos novos não podem encolher nem mover o tabuleiro, nem fazer o M3 regredir (folha de Som, respiração, volumes, música e ambiente continuam como estão).
- Nenhum áudio, chave ou senha entra no git.
- **Anti-objetivo:** um APK que "funciona" mas fica pesado. Partículas têm número máximo e não alocam à toa a cada quadro (risco 6 da spec: celular fraco).

## 4. Restrições

- Texto do jogo em português do Brasil. Números com separador de milhar à brasileira (`12.345`), como o `toLocaleString('pt-BR')` do JS.
- Toda tela nova cabe em 360×640 e em 1080×2400, com botões de pelo menos 7% do menor lado de altura (a mesma regra da folha de Som), e é testada nesses tamanhos e nos de `layout.rs`.
- Arquivos de preferências de versões anteriores (o `som.json` do M3) continuam carregando; campos novos ausentes viram o padrão.
- Os testes seguem o rito: lógica pura em teste unitário, visual em foto (`--screenshot`), Android no celular virtual do CI (fase 11) e na lista de conferência do celular do Thiago.
- O limite de 60 MB do APK continua valendo.

## 5. Abordagens consideradas

**Entrega.** (1) Duas partes, M4a e M4b, cada uma com APK. (2) Uma branch e um APK no fim. **Escolhida pelo Thiago: a 2.** Custo aceito: se algo quebrar, a causa é mais difícil de achar; por isso a ordem acima põe o arriscado por último e cada item vira commits separados.

**Partículas.** (a) Fiel ao JS: faíscas, ondas, feixes, brilho, supernova e fusão. (b) Só faíscas. **Escolhida a (a)**, porque o projeto é refazer o JS; o item 4 tem uma ordem de corte (seção 2) se precisar.

**Versão do Android.** (a) minSdk 26 e targetSdk 35, como a spec principal pedia "se a ferramenta permitir". (b) minSdk 26 e targetSdk 34. **Escolhida a (b)**: com alvo 35 o Android 15 desenha o app até as bordas da tela, e o CI só tem celulares virtuais com Android 30 e 34, então ninguém veria o problema antes de chegar ao celular do Thiago. Se o gerador de APK não aceitar nem o 34, fica o que o CI permitir, anotado em `docs/notes/android-build.md`, sem mexer na imagem Docker.

## 6. Decisões por item

### 6.1 Vibração

- Preferência `vibracao` (verdadeiro/falso), **desligada de início**, guardada junto das outras preferências (no `som.json`; o nome do arquivo não muda).
- Interruptor "Vibração" dentro da folha de Som, no estilo dos interruptores de Música e Ambiente, nos dois modos. Só aparece se o aparelho vibra (no Android, pergunta ao `Vibrator`; no PC, que já responde "ok" de mentira, aparece, para a foto e os testes).
- Ao ligar, vibra 55 ms; se a vibração falhar, o interruptor volta a desligado, sem erro na tela.
- Padrões, só numa jogada **válida** (troca inválida não vibra), como o JS: 28 ms; 45 ms se ativou uma pedra especial; padrão curto 28·55·35 ms (vibra, pausa, vibra) se subiu de nível. O volume dos efeitos não interfere.
- A escolha do padrão é uma função pura, testada com tabela de casos. A vibração real só se confere no celular; o celular virtual confere que o pedido saiu (linha de registro `PRISMA_M4 vibracao ...`, nome exato a cargo do plano).
- Atenção do implementador: no Android um padrão de vibração começa por um tempo de **espera**, então o 28·55·35 do JS vira `[0, 28, 55, 35]`.

### 6.2 "Carregando…"

- Carregar uma música ou um ambiente trava o quadro (de ~0,5 a ~1 s nos celulares virtuais; no celular real, desconhecido). Antes de carregar uma faixa que **ainda não está carregada**, o jogo desenha pelo menos um quadro com o aviso "Carregando música…" ou "Carregando ambiente…" no lugar do título da folha (onde já aparecem os avisos do M3) e só no quadro seguinte faz o carregamento. O aviso some quando acaba, com sucesso ou erro.
- Faixa já carregada não mostra aviso.
- Verificação: teste unitário da ordem (aviso antes do pedido de carga) e, no celular virtual, a linha do aviso aparecer **antes** da `PRISMA_M3 carregado ...` correspondente.

### 6.3 Botão ≡: Recordes e Créditos

- Um botão ≡ ao lado do ♪ (o ♪ e o ≡ quadrados, na linha das abas, que ficam um pouco mais estreitas; o tabuleiro não muda).
- Abre uma folha do mesmo tamanho e lugar da de Som, com dois botões no alto, **Recordes** e **Créditos**, e o ✕. Abre em Recordes. Só uma folha aberta por vez: abrir uma fecha a outra. As abas Zen/Clássico e o ♪/≡ continuam tocáveis.
- **Recordes:** dois blocos, Zen (Maior pontuação, Maior nível, Melhor jogada) e Clássico (os mesmos mais Partidas concluídas), lidos dos recordes do M2, em tempo real.
- **Créditos:** poucas linhas curtas, sem links (não há internet nem navegador a abrir), que cabem em 360×640 sem rolagem. Conteúdo fixo:
  - "Prisma (Rust)" e a versão do pacote (`CARGO_PKG_VERSION`).
  - Jogo original criado por Thiago Luz (2026); versões derivadas devem preservar os créditos e identificar suas alterações.
  - Código: GPLv3. Arte das pedras e fundo: CC BY-SA 4.0. Nome e ícone não integram a licença da arte.
  - Ruído suave: "Gentle White Noise Air Tone", Geoff Bremner, CC BY 4.0.
  - Demais músicas e ambientes: CC0, autores em `docs/CREDITOS-AUDIO.md` (repositório `thiagodluz/prisma-rust`).
- O texto mora num lugar só e um teste confere que cita GPLv3, CC BY-SA 4.0, CC BY 4.0 e Geoff Bremner (como o teste que já existe para `CREDITOS-AUDIO.md`).

### 6.4 Partículas

Porte dos efeitos visuais da limpeza do JS (valores de `app.js` e `style.css`; o implementador lê as cores e medidas exatas de lá). Cada efeito é uma função pura do progresso `t` de 0 a 1, para ser testável; o desenho usa formas simples do Macroquad.

| Efeito | Quando | Como é |
|---|---|---|
| Faíscas | toda limpeza; em torno de cada especial ativada | quadradinhos brilhantes que saem do centro e somem (380 ms). Combinação comum: 5. Especial: 8; Espectro: 12. Alcance (em casas): 1,25 comum e Pulso, 1,65 Cruz, 2,2 Espectro. Cor: dourada; azul-gelo na Cruz; arco-íris no Espectro |
| Onda | Pulso e Cruz | anel que cresce e some (360 ms) |
| Feixes | Cruz | uma linha horizontal e uma vertical cruzando o tabuleiro (330 ms) |
| Brilho | Espectro | véu claro sobre o tabuleiro (370 ms) |
| Supernova | Supernova | anel de 5 casas, na cor da Supernova (480 ms) |
| Fusão | Supernova | linhas das pedras de origem até a casa da Supernova (240 ms), **antes** da explosão |
| Pulso das pedras | pedras que somem | crescem um pouco e brilham antes de sumir; as mais longe da especial atrasam até 100 ms |

Regras do JS que continuam: no máximo 3 especiais com efeito por limpeza (mais as Supernovas); uma limpeza sem especial faz uma só nuvem de faíscas, no meio das pedras que somem.

Consequência para o código: o `Passo::Limpeza` hoje só leva as casas ativadas e se houve Supernova. Para desenhar cor, origem das fusões e tipo de cada especial, ele precisa carregar esses dados (o motor já os informa); isso muda só o jogo, não o motor.

Verificação: cenas de foto novas (uma por efeito, em instantes fixos, nas três intensidades) para conferir a olho, mais testes das funções (começa e termina em zero; contagem de faíscas por caso; teto de objetos desenhados).

### 6.5 Intensidade dos efeitos (só no Zen)

- Preferência com três valores: Suave, Padrão, Vibrante; padrão **Padrão**. No Clássico vale sempre Padrão. Muda só o visual (nem o som, nem a vibração, nem a pontuação).
- Controle na parte do Zen da folha de Som, como as outras escolhas do Zen (toca para passar ao próximo valor).
- Valores do JS:
  - escala de duração dos efeitos: Suave 0,82; Padrão 1; Vibrante 1,1;
  - faíscas: no Suave, 5 por especial (qualquer tipo) e 3 por combinação comum; nos outros, os números da tabela;
  - pulso das pedras: tamanho 1,1 (Suave e Padrão) e 1,18 (Vibrante); brilho 1,35 / 1,8 / 2,1;
  - camada de efeitos: Suave mais apagada (48% de opacidade, cores menos vivas); Vibrante mais viva e mais clara; e a faixa "Nível N" com sombra menor no Suave e brilho extra no Vibrante.
- A geometria da folha de Som continua cabendo em 360×640 com a linha nova (o Zen já é a folha mais cheia).

### 6.6 Ícone

- Hoje só há os ícones quadrados comuns, gerados pelo `prisma_tools imagens` a partir do `icon-512.png`. O ícone "final" será o **adaptativo** do Android 8+: figura (o ícone do JS) sobre um fundo liso na cor do próprio ícone, com a figura dentro da zona segura (cerca de 66% central), para não ser cortada em círculo, quadrado arredondado etc. Os ícones comuns continuam como reserva.
- Gerado pela ferramenta, sem arquivo à mão: o `prisma_tools imagens` passa a produzir também o `mipmap-anydpi-v26/ic_launcher.xml`, a figura em cinco densidades e a cor de fundo. (Esta é a minha leitura de "ícone final"; o Thiago corrige se queria outra coisa.)
- Verificação: teste da ferramenta (tamanhos, figura dentro da zona segura); foto do ícone sob máscaras redonda e arredondada; o CI confere o ícone no APK; o resto é olhar a tela inicial do celular.

### 6.7 Versão do Android

- `minSdk 26`; `targetSdk` o mais alto que o gerador aceitar até 34 (seção 5). O CI confere no APK (`aapt2 dump badging`, que já aparece no log) e o `tests/workspace.rs` fixa os valores no `game/Cargo.toml`.
- Feito por último, em commits próprios. Se o CI não gerar o APK ou o layout mudar nos dois celulares virtuais, desfaz-se esse pedaço e se registra o motivo.

## 7. Como verificar (resumo)

- Lógica pura: `cargo test --workspace` (padrões de vibração, ordem do "Carregando…", funções dos efeitos, textos dos créditos, layout da nova folha nos tamanhos de celular, preferências antigas ainda carregando).
- Fotos: cenas da folha ≡ (Recordes e Créditos), da folha de Som com a vibração e a intensidade, de cada efeito e do ícone.
- Celular virtual (fase 11, nova): abrir o ≡ e fechar; ligar a vibração e ver o pedido; o aviso de carregamento antes do `carregado`; `badging` com as versões e o ícone; continua sem erro e sem morrer.
- No celular do Thiago (lista de conferência, ao fim): sentir os três padrões; ver as partículas nas três intensidades no Zen; ícone na tela inicial; Recordes e Créditos legíveis; "Carregando…" aparece ao ligar a música.

## 8. Deixado ao implementador

Nomes de tipos e funções, como os efeitos são modelados no `Passo::Limpeza`, a chamada JNI do padrão de vibração (as formas antigas `vibrate(long[], int)` e a nova com `VibrationEffect` valem; o `minSdk` 26 permite a nova), os nomes exatos das linhas `PRISMA_M4`, a divisão em tarefas, e as cores e medidas finas dos efeitos (lidas do `style.css`, ajustadas a olho nas fotos).

## 9. Minhas escolhas, para o Thiago conferir

- Alvo 34 em vez de 35 (seção 5).
- O ≡ novo para Recordes e Créditos, com a vibração dentro da folha de Som.
- Créditos sem links e com o resumo dos áudios CC0 apontando para o repositório.
- Ordem das tarefas (seção 2) e a ordem de corte das partículas.
- "Ícone final" lido como ícone adaptativo.
- Sem "reduzir movimento" do sistema.
