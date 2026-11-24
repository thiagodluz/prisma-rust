# Prisma Rust · Fase 3: a tela igual à do "Novo" do JS (V1 e V2)

Data: 2026-11-23 · Branch: `fase3-visual` · Depois de V1 e V2 vem o estilo "Anterior" (A), com desenho próprio.

## 1. Por que e o que é

Depois do M4 o Thiago testou o app e notou que o jogo em Rust ainda não é igual ao Prisma em JS no visual "Novo": só as pedras são as mesmas. Ele quer o Rust parecido com o JS, e **só depois** o visual "Anterior" por cima (o "Anterior" do JS é feito de formas com degradê, sem imagens; ele nasce melhor se a estrutura da tela já for a do JS).

O que muda na tela (comparando os dois prints dele, JS contra Rust):

| Parte | JS "Novo" | Rust hoje |
| --- | --- | --- |
| Fundo | foto de cristais (`prisma-bg.jpg`) com véu escuro | azul liso `#101229` |
| Topo | logo + "PRISMA" + dois botões arredondados | quatro retângulos: Zen, Clássico, ♪, ≡ |
| Pontos | cartão arredondado, NÍVEL em verde-água | três números soltos |
| Barra de nível | degradê verde-água → roxo, arredondada | azul reto e fino |
| Tabuleiro | moldura brilhante e 64 casas escuras arredondadas | pedras soltas no fundo |
| Botões | arredondados, ícone + letra em negrito | retângulos com letra monoespaçada |
| Modos | dois cartões com ícone e frase | os dois botões do topo |
| Letra | Roboto (a fonte do sistema no Android) | fonte monoespaçada embutida |

## 2. Objetivos, não-objetivos, anti-objetivos

**Objetivos**
- Em 360×640 e em 1080×2400, a tela do Rust se parece com a do JS: mesma ordem, mesmas proporções, mesmas cores.
- Tudo continua cabendo numa tela só, sem rolar (como hoje).
- As folhas do M4 (Som, Recordes, Créditos) e o fim de partida continuam funcionando e ganham o mesmo visual.

**Não-objetivos**
- Cópia ao pixel: o Macroquad não faz desfoque nem vidro translúcido; sombra suave e brilho são aproximados.
- Página que rola, como a do JS (Visual e Configurações de som ficam em folhas, como hoje).
- Mudar regras, animações, som, vibração ou o salvamento.
- A chave "Visual" (Anterior/Novo): é da etapa A.

**Anti-objetivos (seria falha mesmo que "funcione")**
- Jogo mais lento: desenhar 64 casas, moldura e cartões a 60 quadros por segundo no celular fraco tem de custar quase nada (por isso as formas são geradas uma vez, na abertura).
- APK bem maior: a foto (103 KB) e a fonte (cerca de 350 KB) são o único peso novo.
- Texto cortado ou ilegível em tela pequena (360×640 é o limite).
- Toque no lugar errado: toda área tocável continua vindo de `layout.rs`, com os testes de "não cruzam e cabem" valendo para o layout novo.

## 3. Como desenhar (duas opções, escolhida a 1)

1. **Formas por código (escolhida).** Em Rust, ao abrir o app, geram-se pequenas texturas (canto arredondado, degradê, borda, sombra suave) que o desenho estica em "nove fatias" sem deformar os cantos. Só entram arquivos para a foto de fundo e a fonte. Aumenta a parte em Rust, e as mesmas ferramentas desenharão as pedras do "Anterior" (hexágono, losango, gota…). Risco: parecido, não idêntico, em sombra e brilho.
2. **Imagens prontas.** Mais fiel nos brilhos, mas mais arquivos, ajuste de cor pede gerar tudo de novo, e o "Anterior" precisaria de outro jogo de imagens. Descartada.

## 4. Decisões

Do Thiago:
- Tela toda igual ao JS, **em duas etapas**, cada uma com seu APK para ele testar.
- Formas por código (seção 3).

Minhas (ele pode corrigir ao ler):
- **Fonte:** Roboto Regular e Bold, embutidas no binário (`include_bytes!`, sem depender de pasta no Android). O JS não carrega fonte nenhuma e usa a do sistema, que no Android é Roboto. Licença OFL; entra em `game/assets/` com o texto da licença e no `LICENSE`/créditos.
- **Ícones** dos botões e dos modos desenhados com formas (como a nota ♪ e o ≡ de hoje): embaralhar, dica (faísca de 4 pontas), novo jogo (seta circular), lua (Zen), faísca (Clássico).
- **Foto de fundo:** a `prisma-bg.jpg` do JS (CC BY-SA 4.0, já é arte do projeto; entra no `LICENSE-ART.md`), cobrindo a tela inteira, com o véu `#0b102242 → #0b10226b` que o JS usa no "Novo".
- **Tela fixa, sem rolar.** De cima para baixo, como no JS: topo, cartão de pontos, barra e texto, tabuleiro, três botões, cartões de modo. O botão ♫ continua abrindo a folha de Som e o ≡ o menu (o JS não tem ≡; ele fica no topo, ao lado do ♫).
- As folhas (Som, menu, fim de partida) reaproveitam o cartão novo; sem refazer o conteúdo delas.
- Valores exatos (cores, raios, espaços, tamanhos de letra) vêm do `style.css` do JS (`.header`, `.hud`, `.stat`, `.progress`, `.board-frame`, `.cell`, `.quiet-button`, `.mode`, e as regras `data-visual-style="illustrated"`), convertidos em proporções da largura da tela.

## 5. Etapas

**V1 · Tabuleiro e cores.** Módulo novo de formas (cantos arredondados, degradê, borda, sombra suave); fundo em foto com véu; moldura e casas do tabuleiro; cartão de pontos; barra em degradê; botões Embaralhar/Dica/Novo jogo arredondados com ícone. O topo (Zen, Clássico, ♪, ≡) continua como está. *Pronto quando:* as fotos 360×640 e 1080×2400 mostram essas partes como as do JS, e o APK roda sem queda de desempenho perceptível no celular dele.

**V2 · Topo, modos e letra.** Topo com logo + PRISMA + ♫ + ≡; os dois cartões de modo com ícone e frase; Roboto em todo o jogo (inclusive nas folhas). *Pronto quando:* a tela inteira, nas fotos, confere com os prints do JS, e todas as folhas continuam legíveis em 360×640.

## 6. Como provar

- Contas puras com teste: a geometria nova (`layout.rs`) cabe em todos os `CELULARES` e `TELAS`, sem retângulos que se cruzem; as texturas de formas têm canto transparente e centro cheio (testes sem janela).
- Fotos com `--cena` nos dois tamanhos, lado a lado com os prints do JS.
- O CI do Android já existente (fotos da fase 11 e demais) tem de continuar verde; a fase do emulador ganha a foto da tela principal nova.
- Mutação nos testes novos, como nos marcos anteriores.

## 7. Fica para quem constrói

- Receita de cada textura (raio do canto como proporção do lado, passos do degradê).
- Como obter a Roboto estática (Regular e Bold): o computador de trabalho só tem a versão variável (`Roboto[wdth,wght].ttf`), que a biblioteca de fonte do Macroquad pode não aceitar; pegar as estáticas no GitHub do projeto Roboto ou gerá-las com `fonttools`, e conferir a licença.
- Quanto cada parte encolhe em 360×640 para tudo caber, mantendo toque confortável (mínimo hoje do projeto).

## 8. Riscos

- A fonte nova muda a largura de todo texto: rótulos e folhas do M4 podem não caber; os testes de layout e as fotos pegam isso.
- Texto desenhado com a fonte do Macroquad no Android nunca foi testado com fonte embutida por nós: V1 não mexe na fonte justamente para isolar esse risco na V2.
- "Parecido, não idêntico": se o resultado ficar longe do JS em sombras e brilhos, as texturas pedem retoque (aceitável; é só receita).

## 9. Depois (A · "Anterior")

Terá desenho próprio. Ideia já combinada: as 7 pedras e as caixas dos especiais do "Anterior" (formas com degradê, símbolos ✺ ✦ ✶) feitas com as mesmas ferramentas de formas, e uma preferência "Visual" guardada como o volume e a vibração. Onde a chave fica (folha de Som ou menu) se decide lá.
