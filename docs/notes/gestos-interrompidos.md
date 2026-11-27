# Gestos interrompidos

Item 4 da auditoria (gestos interrompidos; a parte de teclado foi retirada, veja abaixo). Plano:
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

## Teclado: feito e retirado

A navegação por teclado no PC (setas, Enter/Espaço, Tab, Esc e um contorno de foco) chegou a
entrar na main (PR #9) e foi retirada em seguida: o projeto passou a ser só para celular. O código
está no histórico do git (commits `1c48a72` e `682f8b4`).

## Só o aparelho confirma

Puxar a barra de notificações no meio de um arrasto (Android) e minimizar no meio de um arrasto
(PC): ao voltar, nada pode ter sido trocado e o próximo toque tem de funcionar.
