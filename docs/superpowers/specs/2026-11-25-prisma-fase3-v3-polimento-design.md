# Prisma em Rust — Fase 3, V3: polimento ("profissional e polido")

Data: 2026-11-25. Pedido do Thiago: manter o layout e as pedras e deixar a tela com aparência
**profissional e polida**, com fontes melhores. Caminho escolhido por ele: **refino + vidro**
(opção 2). O visual "Anterior" do JS foi descartado.

## Por quê

A V2 deixou a tela correta, mas com cara de protótipo: letra neutra (Roboto), botões apertados
(ícone colado no texto, texto quase na borda), textos pequenos e apagados, barra de progresso
quase invisível quando vazia, caixa de fim de partida chapada e de cantos retos. A sensação
pedida é de app acabado, não de enfeite.

## Objetivos

1. **Letra com personalidade e boa leitura:** Inter (textos e botões) e Sora (título "PRISMA",
   números grandes e títulos de caixas).
2. **Hierarquia clara:** o olho vai do placar para o tabuleiro, depois para as ações, depois para
   os modos. Botões de ação mais discretos que o tabuleiro, modo ativo claramente marcado.
3. **Acabamento de vidro fosco** nos painéis, botões e cartões: borda fina clara, brilho suave no
   topo, sombra macia. Tudo com a mesma receita, para a tela parecer uma peça só.
4. **Legibilidade:** nenhum texto menor que hoje; textos secundários com contraste de pelo menos
   4,5:1 sobre o fundo do seu painel.
5. **Peça selecionada** com um anel arredondado luminoso no lugar do quadrado branco.
6. **Fim de partida e diálogos** no mesmo vidro arredondado, com botão principal de degradê.

## Fora do escopo (não-objetivos)

- Mexer no desenho, nas cores ou na animação das pedras.
- Mudar a ordem ou o tamanho geral dos blocos da tela (layout da V2).
- Animações novas de entrada e brilhos nas pedras (opção 3, fica para depois).
- Recursos novos, o visual "Anterior", teclado, web.

## O que seria um fracasso (anti-objetivos)

- Enfeite demais: brilho forte, muitas bordas, cores novas que brigam com o fundo roxo.
- Texto que cabia e agora corta, ou botão menor para o toque que o de hoje.
- Abertura do app mais lenta, ou APK acima de 100 MB (limite definido pelo Thiago; hoje 54,7 MB).
- Tela de celular pequeno (360×640) pior que a de hoje.

## Restrições

- Macroquad 0.4 e o desenho atual: formas pré-renderizadas em textura (`Formas` / `Receita` em
  `render.rs`) e texto por `texto.rs`. Sem shader novo.
- O Macroquad (via fontdue) **não aplica recursos OpenType**: não há números de largura fixa. Os
  números do placar ficam proporcionais; o painel mantém colunas de largura fixa para não
  "dançarem".
- Fontes livres (OFL), embutidas, com licença em `LICENSE-ART.md` e `game/assets/OFL.txt`. Só os
  pesos usados, reduzidos ao Latino para o APK crescer pouco (meta: no máximo +1 MB no total,
  contando que a Roboto sai). O limite de 100 MB do Thiago sobra, mas fonte inteira sem
  necessidade continua sendo desperdício.
- O teste que confere os glifos de todo texto (`todo_texto_do_jogo_tem_glifo_na_roboto`) passa
  a conferir as fontes novas; ♫ e ≡ continuam desenhados por código.
- Fotos de todas as cenas em 360×640 e 1080×2400 sem corte, sobreposição ou quadradinho.

## Caminhos considerados

1. **Refino:** só fontes, espaçamento, contraste e redesenho do placar e da caixa final.
   Menor e mais seguro, mas a tela continua "chapada".
2. **Refino + vidro (escolhido pelo Thiago):** o 1, mais a receita de vidro em tudo e o anel da
   peça selecionada. Dá o ar de app acabado sem tocar nas pedras.
3. **Ousado:** o 2, mais animações de entrada e brilho nas pedras. Mexe no que o Thiago pediu para
   manter e custa mais desempenho; fica para depois, se ele quiser.

## Decisões de design (minhas, para ele conferir)

- **Fontes:** Inter Regular e Inter SemiBold; Sora Bold (título, números do placar, títulos de
  caixas). A Roboto é removida. Chamada de texto passa a ter três estilos (texto, texto forte,
  título) no lugar de Normal/Negrito.
- **Placar:** rótulos (PONTOS, NÍVEL, MELHOR) com respiro de cima, em caixa-alta pequena com
  espaço entre letras; números em Sora; divisórias verticais finas entre as três colunas.
- **Progresso:** trilho sempre visível (fundo claro translúcido com borda), um pouco mais alto;
  preenchimento em degradê como hoje; o texto "x / y para o próximo nível" 10% maior e mais claro.
- **Botões de ação (Embaralhar, Dica, Novo jogo):** ícone em cima, rótulo embaixo em Inter
  SemiBold, margem interna fixa; desabilitado fica apagado, sem mudar o tamanho.
- **Cartões de modo:** mesma receita de vidro; modo ativo com borda e fundo mais claros; frase em
  Inter, que é mais estreita que a Roboto em tamanho pequeno. Se a do Clássico ainda não couber,
  encurta-se o texto ("Sem cronômetro, pode acabar") em vez de diminuir a letra.
- **Peça selecionada:** anel arredondado na cor `DESTAQUE` com brilho suave, no lugar do quadrado
  branco.
- **Fim de partida / diálogos / folhas (Som, Recordes, Créditos):** painel de vidro com raio 20,
  título em Sora, botão principal com degradê roxo-azul e cantos arredondados. A película escura
  sobre o tabuleiro passa a seguir os cantos arredondados da moldura (corrige um item adiado da
  V1).
- **Contraste:** um teste confere que as cores de texto principais e secundárias passam de 4,5:1
  sobre o fundo do respectivo painel.

## O que fica para quem constrói

Valores exatos (tamanhos, raios, opacidades) dentro destas decisões; a ordem das tarefas; como
reduzir as fontes (fonttools) e medir o APK; quais cenas fotografar além das já listadas.

## Riscos

- Só o celular confirma como as fontes novas aparecem no Android e se o brilho do vidro agrada.
- Sora e Inter têm métricas diferentes da Roboto: textos centralizados podem precisar de ajuste
  fino de linha de base (as fotos conferem).
- O tempo de abertura com fontes novas (mesmo número de fontes que hoje, três pesos em vez de
  dois) deve ser parecido; o celular confirma.
