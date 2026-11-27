# Prisma em Rust — item 3 da auditoria: menu secreto, desfazer e cheats

## Intenção e porquê

No Prisma original (JS), um menu de desenvolvedor fica escondido em Créditos: 7 toques seguidos
na linha da versão o abrem. Ele tem o "desfazer" (até 50 ações) e cheats para testar o jogo
(pontos, nível, especiais, Supernova, poderes). O motor Rust já tem toda essa lógica
(`src/cheats.rs`, `Game::undo_last_move`), mas o jogo não tem tela para ela. O Thiago quer a
mesma ferramenta de volta, para testar o jogo no celular como fazia no original.

## Metas

- 7 toques na linha da versão (aba Créditos), com no máximo 2,2 s entre um toque e o próximo,
  liberam a aba **Ferramentas** na folha do Menu, ao lado de Recordes e Créditos. Ao liberar, o
  aviso "Ferramentas do desenvolvedor ativadas" aparece (o mesmo aviso em pílula do jogo).
- A aba Ferramentas tem tudo o que o original tem:
  - **Histórico:** "Desfazer última ação" e a linha "N de 50 ações para desfazer".
  - **Pontos e nível:** +100, +1.000, "Próximo nível"; um valor de pontos com − e + e o botão
    "Somar"; um nível com − e + e o botão "Ir".
  - **Especiais:** Pulso, Raio, Espectro e Supernova. Tocar num deles fecha a folha e mostra o
    aviso "Toque numa casa"; o próximo toque no tabuleiro escolhe a casa e aplica o cheat. Um
    toque fora do tabuleiro (ou abrir uma folha, ou trocar de modo) cancela a escolha.
  - **Poderes:** "Impedir derrota" (interruptor), "Embaralhar" (sem limite), "Jogada automática"
    e "Explodir".
  - Uma linha de estado: "Partida normal: usar um cheat protege os recordes desta partida" ou
    "Partida com cheats: os recordes estão protegidos".
- **Recordes protegidos:** partida com `cheats_used` não sobe recorde nenhum, nem ao jogar, nem
  ao trocar de modo, nem ao reabrir. (Hoje o Rust não confere isso; o original confere.)
- **Desfazer restaura os recordes:** cada ação que pode ser desfeita guarda também os recordes
  do modo de antes dela; ao desfazer, esses recordes voltam e são gravados.
- **Limites e limpeza do histórico** (os do original): até 50 ações; limpo ao reabrir o app, ao
  trocar de modo, ao começar outra partida e no embaralhamento normal do Zen (o botão
  Embaralhar de baixo). O texto da aba diz isso numa linha curta.
- **Momentos inseguros:** com animação em andamento (`ocupado()`), todos os botões da aba ficam
  apagados e não fazem nada; a escolha de casa também não aceita toque durante animação.
- Explodir e Supernova animam como uma jogada (cascatas, partículas, sons), e a jogada
  automática é uma jogada de verdade (a da Dica) com a animação de troca.

## Não-metas

- Não há campo de texto: números só com − e + (passos de 100/1.000 para pontos, 1 para nível;
  segurar não acelera).
- A aba não fica liberada depois de fechar o app (como no original).
- Nada de cheat fora da aba (sem teclas de atalho no computador).

## Anti-metas

- Um cheat nunca pode subir um recorde normal.
- Um desfazer nunca pode deixar recordes de uma ação que deixou de existir.
- O jogador comum não pode abrir a aba sem querer (um toque solto na versão não faz nada).

## Restrições

- Macroquad 0.4, sem campo de texto; layout tem de caber em 360×640 em pé e funcionar de
  320×480 a 1080×2400 (os testes de encaixe da V3 cobrem telas em pé).
- Visual da V3: vidro (`Formas.painel`, `cartao`, `botao_principal`), fontes Inter/Sora,
  `PARES_DE_TEXTO` com contraste ≥ 4,5 para toda cor de texto nova.
- A partida com cheat continua marcada no save (`cheatsUsed`), como já é.
- TDD com mutação, clippy do `game` na linha de base (9), commits em português.

## Abordagens consideradas

1. **Terceira aba no Menu (escolhida pelo Thiago).** Reaproveita a folha do Menu, o vidro e a
   navegação; os especiais pedem a casa tocando no tabuleiro.
2. Folha só dela, como a de Som: mais separada, mas mais uma tela para manter.
3. Barra pequena sobre o tabuleiro: mostra as pedras, mas não cabe tudo.

E para os números (escolhido pelo Thiago): − e + para pontos e nível, e a casa escolhida
tocando no tabuleiro (em vez de linha e coluna com − e +, como no original).

## Decisões do como (minhas, para o Thiago conferir)

- Valores iniciais: pontos 5.000 (passo 1.000 no −/+, de 1.000 a 1.000.000); nível 10 (passo 1,
  de 1 a 10.000), os mesmos do original.
- A pilha de recordes do desfazer fica no `App`, por modo, e sempre com o mesmo tamanho do
  `undo_history` do jogo daquele modo (quando o motor descarta a mais antiga, o App também).
- Trocar de modo limpa o histórico das duas partidas (o original só tem uma partida, e limpa
  ao trocar); assim as duas pilhas nunca divergem.
- Os 7 toques contam só com a aba Créditos aberta; a área de toque é a linha da versão
  (a primeira linha dos créditos).
- Interruptor "Impedir derrota" conta como cheat (o motor já marca `cheats_used`).
- Mensagens (as do original): cheat recusado pelo motor → "Confira os valores ou tente outra
  ação"; jogada automática sem jogada possível → "Embaralhe ou ative a proteção contra derrota";
  cada cheat aceito mostra um aviso curto ("+100 pontos", "Nível alterado", "Especial criado"…).
- Desfazer depois do fim de partida do Clássico volta a partida (o motor restaura `ended`), e o
  cartão de fim some.

## O que fica para quem constrói

- Disposição exata dos botões na aba (a do esboço abaixo é a intenção, não a medida).
- Nomes dos novos `Botao` e campos de `TelaMenu`.
- Como dividir em tarefas.

```
[Recordes] [Créditos] [Ferramentas]  ✕
Partida normal · 3 de 50 para desfazer
[ Desfazer última ação ]
Pontos  [−] 5.000 [+]   [Somar]
Nível   [−]   10  [+]   [Ir]
[+100] [+1.000] [Próx. nível]
[Pulso] [Raio] [Espectro] [Supernova]
[Embaralhar] [Jogada auto] [Explodir]
Impedir derrota            (●  )
```

## Como validar (da auditoria)

Acesso ao menu; jogada seguida de desfazer; várias ações; recorde batido e desfeito; troca de
modo; novo jogo; embaralhamento; reabertura do app (histórico vazio, aba escondida, partida
ainda marcada como com cheat).
