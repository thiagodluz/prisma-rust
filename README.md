# Prisma (Rust)

O jogo de combinar pedras **Prisma**, refeito em Rust por diversão e aprendizado.
O Prisma original, em JavaScript, está em <https://github.com/thiagodluz/Prisma>.

- `src/` e `tests/`: o motor do jogo (`prisma_engine`), só as regras, sem dependências.
  É idêntico ao `engine.js` do original e é conferido contra ele por testes.
- `game/`: a tela (`prisma_game`), feita com [Macroquad](https://macroquad.rs). Roda no PC e
  vira app Android pelo GitHub Actions (ver `docs/notes/android-build.md`).
- `docs/superpowers/`: desenhos e planos de cada fase.

Estado: Fase 2, marco M0a ("Olá, tabuleiro"). Ainda não dá para jogar.

## Licenças

- **Código:** GPL-3.0-only (arquivo `LICENSE`), a mesma do Prisma em JS.
- **Imagens** em `game/assets/`: CC BY-SA 4.0, © Thiago Luz (ver `game/assets/LICENSE-ART.md`).
- O nome Prisma e os ícones ficam fora das licenças acima.
