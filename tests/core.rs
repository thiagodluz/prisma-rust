//! Tarefa 1: núcleo do tabuleiro (sorte repetível, combinações, jogadas possíveis).
mod common;

use common::{at, fixture, place};
use prisma_engine::{Game, TileKind, adjacent, find_matches, lcg_random, pick};
use std::collections::HashSet;

#[test]
fn lcg_random_matches_the_js_sequence() {
    // Valores calculados pelo próprio engine.js (Node) com a mesma fórmula.
    let mut r = lcg_random(7654321);
    assert_eq!(r(), 0.6867495393380523);
    assert_eq!(r(), 0.013034644303843379);
    assert_eq!(r(), 0.727377827744931);
}

#[test]
fn new_game_with_lcg_seed_matches_the_js_board() {
    // Tabuleiro inicial que o engine.js original gera com a semente 7654321.
    let game = Game::with_random(lcg_random(7654321));
    let colors: Vec<Vec<u8>> = game
        .board
        .iter()
        .map(|row| {
            row.iter()
                .map(|tile| tile.color.expect("pedra comum tem cor"))
                .collect()
        })
        .collect();
    let expected = vec![
        vec![4, 0, 5, 5, 0, 4, 1, 3],
        vec![3, 5, 3, 6, 0, 3, 4, 6],
        vec![1, 2, 5, 6, 1, 0, 3, 1],
        vec![5, 1, 1, 2, 3, 6, 1, 6],
        vec![6, 6, 5, 0, 6, 5, 0, 0],
        vec![0, 5, 5, 4, 6, 5, 1, 5],
        vec![4, 3, 0, 1, 5, 0, 1, 1],
        vec![3, 5, 6, 5, 3, 3, 4, 6],
    ];
    assert_eq!(colors, expected);
    assert_eq!(game.next_id, 65);
}

#[test]
fn starting_boards_have_unique_stable_ids_no_matches_and_a_move() {
    for _ in 0..100 {
        let game = Game::new();
        let ids: HashSet<u32> = game.board.iter().flatten().map(|tile| tile.id).collect();
        assert_eq!(ids.len(), 64);
        assert!(game.matches().cells.is_empty());
        assert!(game.has_move());
    }
}

#[test]
fn pick_never_returns_an_index_outside_the_range() {
    assert_eq!(pick(&mut || 1.0, 5), 4);
    assert_eq!(pick(&mut || 0.0, 5), 0);
    assert_eq!(pick(&mut || 0.999999, 5), 4);
}

#[test]
fn matches_finds_runs_of_three_or_more_and_ignores_spectrum() {
    // Trinca na horizontal: linha 0, colunas 0 a 2, cor 1.
    let mut game = fixture();
    for c in 0..3 {
        place(&mut game, 0, c, Some(1), TileKind::Normal);
    }
    let found = find_matches(&game.board);
    assert_eq!(found.runs.len(), 1);
    assert_eq!(found.runs[0].color, 1);
    assert_eq!(found.runs[0].length, 3);
    assert_eq!(found.cells, vec![at(0, 0), at(0, 1), at(0, 2)]);

    // Quadra na vertical: coluna 0, linhas 3 a 6, cor 3.
    let mut game = fixture();
    for r in 3..7 {
        place(&mut game, r, 0, Some(3), TileKind::Normal);
    }
    let found = find_matches(&game.board);
    assert_eq!(found.runs.len(), 1);
    assert_eq!(found.runs[0].length, 4);
    assert_eq!(found.cells, vec![at(3, 0), at(4, 0), at(5, 0), at(6, 0)]);

    // Forma em L: o canto (3,4) pertence a duas sequências, mas aparece uma vez só.
    let mut game = fixture();
    for (r, c) in [(3, 4), (3, 5), (3, 6), (4, 4), (5, 4)] {
        place(&mut game, r, c, Some(4), TileKind::Normal);
    }
    let found = find_matches(&game.board);
    assert_eq!(found.runs.len(), 2);
    assert_eq!(
        found.cells,
        vec![at(3, 4), at(3, 5), at(3, 6), at(4, 4), at(5, 4)]
    );

    // Cinco iguais na linha 0 (as colunas 0 a 3 de cor 1 + a coluna 4, que já é cor 1);
    // trocando a do meio por um Espectro, nada combina.
    let mut game = fixture();
    for c in 0..4 {
        place(&mut game, 0, c, Some(1), TileKind::Normal);
    }
    assert_eq!(find_matches(&game.board).runs[0].length, 5);
    place(&mut game, 0, 2, None, TileKind::Spectrum);
    assert!(find_matches(&game.board).cells.is_empty());
}

#[test]
fn adjacent_accepts_only_orthogonal_neighbours_inside_the_board() {
    assert!(adjacent(0, 1));
    assert!(adjacent(1, 0));
    assert!(adjacent(0, 8));
    assert!(!adjacent(7, 8), "fim de uma linha e começo da outra não são vizinhos");
    assert!(!adjacent(0, 9), "diagonal não vale");
    assert!(!adjacent(5, 5));
    assert!(!adjacent(63, 64), "64 está fora do tabuleiro");
}

#[test]
fn fixture_board_has_no_moves_but_force_no_moves_hides_real_ones() {
    assert!(!fixture().has_move(), "o tabuleiro fixo não tem jogadas");
    let mut game = Game::new();
    assert!(game.has_move());
    game.force_no_moves = true;
    assert!(!game.has_move());
    assert!(!game.has_move_on(&game.board));
}
