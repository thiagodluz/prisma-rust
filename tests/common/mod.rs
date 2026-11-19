//! Auxiliares compartilhados pelos testes de integração.
//!
//! Cada arquivo em `tests/` é compilado como um programa separado e usa só uma
//! parte destes auxiliares; por isso o aviso de "código não usado" fica desligado.
#![allow(dead_code)]

use prisma_engine::{Game, SIZE, TileKind, adjacent, find_matches, lcg_random, swap_on};

/// Jogo de teste: sorte repetível (semente 7654321) e um tabuleiro fixo,
/// cor = (linha * 3 + coluna * 2) % 7, que não tem nenhuma combinação nem jogada.
pub fn fixture() -> Game {
    let mut game = Game::with_random(lcg_random(7654321));
    for r in 0..SIZE {
        for c in 0..SIZE {
            let tile = game.gem(Some(((r * 3 + c * 2) % 7) as u8), TileKind::Normal);
            game.board[r][c] = tile;
        }
    }
    game
}

/// Índice (0 a 63) da casa na linha `r` e coluna `c`.
pub fn at(r: usize, c: usize) -> usize {
    r * SIZE + c
}

/// Coloca uma pedra nova (com id novo) na casa (r, c).
pub fn place(game: &mut Game, r: usize, c: usize, color: Option<u8>, kind: TileKind) {
    let tile = game.gem(color, kind);
    game.set(at(r, c), tile);
}

/// Primeira jogada válida na ordem do tabuleiro (porta de `matchMove` do JS).
pub fn match_move(game: &Game) -> (usize, usize) {
    for a in 0..SIZE * SIZE {
        for b in [a + 1, a + SIZE] {
            if !adjacent(a, b) {
                continue;
            }
            // `Board` é `Copy`: esta cópia é só nossa e não mexe no jogo.
            let mut board = game.board;
            swap_on(&mut board, a, b);
            let has_spectrum = [a, b]
                .iter()
                .any(|&i| game.tile(i).map(|t| t.kind) == Some(TileKind::Spectrum));
            if !find_matches(&board).cells.is_empty() || has_spectrum {
                return (a, b);
            }
        }
    }
    panic!("Sem jogadas");
}
