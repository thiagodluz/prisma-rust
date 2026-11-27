# Menu secreto (aba Ferramentas)

Item 3 da auditoria. Design: `docs/superpowers/specs/2026-11-27-prisma-menu-secreto-design.md`.

## Como abrir

Menu (≡) → Créditos → 7 toques seguidos na primeira linha ("Prisma (Rust) · versão …"), com no
máximo 2,2 s entre um toque e o próximo. Aparece o aviso "Ferramentas do desenvolvedor ativadas"
e a terceira aba, **Ferramentas**. Ela some ao fechar o app (não é gravada).

## O que tem

| Linha | Botões | O que faz |
|---|---|---|
| Estado | — | "Partida normal" ou "Com cheats, sem recordes", e quantas ações dá para desfazer |
| Desfazer | Desfazer última ação | volta a partida e os recordes do modo ao estado de antes da última ação |
| Pontos | – / valor / + / Somar pontos | escolhe de 1.000 a 1.000.000 (passo 1.000, começa em 5.000) e soma |
| Nível | – / valor / + / Ir ao nível | escolhe de 1 a 10.000 (começa em 10) e vai; os pontos acompanham |
| Atalhos | +100, +1.000, Próx. nível | — |
| Especiais | Pulso, Raio, Espectro, Supernova | a folha fecha, aparece "Toque numa casa"; o próximo toque no tabuleiro escolhe a casa (fora dele, desiste). A Supernova funde três especiais na linha da casa |
| Poderes | Embaralhar, Jogada auto, Explodir | embaralhar vale também no Clássico; a jogada automática é a da Dica e conta como cheat (a partida não sobe recorde) |
| Impedir derrota | interruptor | o Clássico não acaba (se já acabou, volta com uma jogada) |

–/+, Desfazer e Impedir derrota deixam a folha aberta; os outros a fecham para mostrar o
tabuleiro. Explodir e Supernova animam como uma jogada. Com animação, nada responde.

## Recordes e histórico

- Partida com cheat (inclusive desfazer) não sobe recorde nenhum: nem jogando, nem ao trocar de
  modo, nem no novo jogo, nem ao reabrir. A marca fica no save (`cheatsUsed`) até o novo jogo.
- O App guarda, por modo, os recordes de antes de cada ação no histórico do motor
  (`pilha_de_recordes`), sempre do mesmo tamanho dele; desfazer devolve esses recordes e grava.
- Histórico: até 50 ações. É limpo ao reabrir o app, ao trocar de modo (as duas partidas), no
  novo jogo e no Embaralhar normal do Zen.

## Só o celular confirma

Os 7 toques com o dedo (a linha da versão tem 1,6 vez a altura da letra), o tamanho dos botões
no celular pequeno (em 320×480 alguns rótulos encolhem bastante) e o "Toque numa casa" no
dedo.
