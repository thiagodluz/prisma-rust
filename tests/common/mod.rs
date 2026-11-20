//! Auxiliares compartilhados pelos testes de integração.
//!
//! Cada arquivo em `tests/` é compilado como um programa separado e usa só uma
//! parte destes auxiliares; por isso o aviso de "código não usado" fica desligado.
#![allow(dead_code)]

use prisma_engine::{
    Activation, Board, Created, Event, Game, RandomSource, SIZE, TileKind, adjacent, find_matches,
    lcg_random, swap_on,
};
use std::{cell::Cell, rc::Rc};

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

/// Fonte de sorte repetível (igual à do JS) que também conta quantas vezes foi usada.
pub fn counting_random(seed: u32) -> (RandomSource, Rc<Cell<u32>>) {
    let calls = Rc::new(Cell::new(0));
    let counter = Rc::clone(&calls);
    let mut inner = lcg_random(seed);
    let source: RandomSource = Box::new(move || {
        counter.set(counter.get() + 1);
        inner()
    });
    (source, calls)
}

/// Os campos de um evento de limpeza (`Event::Clear`), para os testes lerem sem repetir `match`.
pub struct Clear<'a> {
    pub board: &'a Board,
    pub cells: &'a [usize],
    pub activated: &'a [Activation],
    pub creations: &'a [Created],
    pub chain: u32,
    pub base: u64,
    pub creation_bonus: u64,
    pub combo_multiplier: f64,
    pub earned: u64,
}

pub fn clear(event: &Event) -> Clear<'_> {
    match event {
        Event::Clear {
            board,
            cells,
            activated,
            creations,
            chain,
            base,
            creation_bonus,
            combo_multiplier,
            earned,
        } => Clear {
            board,
            cells,
            activated,
            creations,
            chain: *chain,
            base: *base,
            creation_bonus: *creation_bonus,
            combo_multiplier: *combo_multiplier,
            earned: *earned,
        },
        other => panic!("esperava uma limpeza, veio {}", other.name()),
    }
}

/// Os nomes das ativações, na ordem (`"burst"`, `"cross"`, `"spectrum"`, `"supernova"`).
pub fn names(activated: &[Activation]) -> Vec<&'static str> {
    activated.iter().map(|activation| activation.name()).collect()
}
