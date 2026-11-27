# Teclado e gestos interrompidos

Item 4 da auditoria (gestos interrompidos e navegação por teclado). Plano:
`docs/superpowers/plans/2026-11-27-prisma-gestos-teclado.md`.

## Gestos interrompidos

O Macroquad transforma o dedo em mouse, mas:

- um toque **cancelado** pelo Android (a barra de notificações puxada no meio do arrasto, por
  exemplo) não vira "soltar": o gesto ficava pendente;
- ao **minimizar** a janela, ele anuncia o botão como solto no lugar onde o dedo estava, o que
  podia completar uma troca que o jogador não fez.

O laço (`main.rs`) assina os eventos crus do Macroquad (`utils::register_input_subscriber`) e,
no quadro em que vê um toque cancelado ou a janela minimizada (`Interrupcoes`), manda só
`Evento::Cancelar` (`input::eventos_do_quadro`). O `Gestos` esquece o gesto e a barra de volume
em arrasto grava o valor de agora.

## Teclado (PC)

| Tecla | O que faz |
|---|---|
| Setas | andam pelas casas (sem dar a volta); a primeira põe o foco no meio do tabuleiro (ou na pedra marcada). Com a barra de volume em foco, ←/→ mudam de 5 em 5 |
| Enter / Espaço | tocam a casa em foco (duas vizinhas trocam) ou apertam o botão em foco; com um especial do menu secreto esperando, criam-no na casa |
| Tab / Shift+Tab | passam pelos botões visíveis, pelas barras de volume (folha de Som) e pelo tabuleiro, em ordem de leitura |
| Esc | fecha a pergunta, cancela a escolha de casa, fecha a folha ou desmarca a pedra, nessa ordem |

O contorno (anel verde-água) só aparece depois de usar o teclado; um toque ou clique o esconde.
Durante a animação, setas e Enter no tabuleiro não fazem nada; Tab e Esc continuam. Segurar a
seta não repete.

## Só o aparelho confirma

Puxar a barra de notificações no meio de um arrasto (Android) e minimizar no meio de um arrasto
(PC): ao voltar, nada pode ter sido trocado e o próximo toque tem de funcionar.
