# Prisma (Rust)

O jogo de combinar pedras **Prisma**, refeito em Rust por diversão e aprendizado.
O Prisma original, em JavaScript, está em <https://github.com/thiagodluz/Prisma>.

- `src/` e `tests/`: o motor do jogo (`prisma_engine`), só as regras, sem dependências.
  Joga como o `engine.js` do original nos cenários cobertos pelos testes (partidas inteiras
  geradas pelo JS e 20 partidas aleatórias de 2000 ações); as diferenças de propósito estão em
  `docs/notes/js-quirks.md`.
- `game/`: a tela (`prisma_game`), feita com [Macroquad](https://macroquad.rs). Roda no PC e
  vira app Android pelo GitHub Actions (ver `docs/notes/android-build.md`).
- `docs/superpowers/`: desenhos e planos de cada fase.

## Estado

Dá para jogar, no PC e no Android (APK de teste). A tela imita o visual "Novo" do original.

**Já tem:** modos Zen e Clássico, partida e recordes salvos por modo, fim de partida e novo jogo,
dica, embaralhar (Zen), música e ambiente do Zen, efeitos sonoros, vibração, guia de respiração,
folhas de Som, Recordes e Créditos, aviso na tela quando o progresso não está sendo salvo e o
menu secreto (7 toques na versão, em Créditos) com desfazer e cheats (`docs/notes/ferramentas.md`) e
teclado no PC: setas, Enter/Espaço, Tab e Esc (`docs/notes/teclado-e-gestos.md`).

**Ainda não tem:** versão para navegador e importação das partidas do app em JS.

### Experimentar

- **Android:** o APK de teste (versão de teste, assinada com chave fixa) fica em
  <https://github.com/thiagodluz/prisma-rust/releases/download/apk-mais-recente/prisma-rust.apk>
  e só é atualizado depois de passar nos celulares virtuais. Detalhes em
  `docs/notes/android-build.md`.
- **PC:** `cargo run -p prisma_game` (no Linux precisa dos arquivos de desenvolvimento do ALSA). As
  músicas e ambientes não ficam no git: `cargo run --manifest-path tools/Cargo.toml -- audio game/sons`
  baixa e prepara tudo; sem isso o jogo funciona com efeitos e avisa que falta a faixa. Para
  usar outra pasta de sons, defina `PRISMA_SONS`.
- **Testes:** `cargo test --workspace` e `cargo test --manifest-path tools/Cargo.toml`.

## Licenças

- **Código:** GPL-3.0-only (arquivo `LICENSE`), a mesma do Prisma em JS.
- **Imagens** em `game/assets/`: CC BY-SA 4.0, © Thiago Luz (ver `game/assets/LICENSE-ART.md`).
- O nome Prisma e os ícones ficam fora das licenças acima.
