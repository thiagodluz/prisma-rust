//! Tarefa 2: embaralhar o tabuleiro e resgatar um jogo sem jogadas.
mod common;

use common::fixture;
use prisma_engine::{Game, Mode, TileKind, lcg_random};

/// (id, tipo, cor) de todas as pedras, ordenado pelo id.
fn identities(game: &Game) -> Vec<(u32, TileKind, Option<u8>)> {
    let mut all: Vec<_> = game
        .board
        .iter()
        .flatten()
        .map(|tile| (tile.id, tile.kind, tile.color))
        .collect();
    all.sort_by_key(|identity| identity.0);
    all
}

/// Ids das pedras na ordem em que aparecem no tabuleiro (linha por linha).
fn ids_in_board_order(game: &Game) -> Vec<u32> {
    game.board.iter().flatten().map(|tile| tile.id).collect()
}

#[test]
fn manual_zen_shuffle_keeps_every_tile_special_type_and_color_without_free_matches() {
    let mut game = Game::new();
    game.board[0][0].kind = TileKind::Burst;
    game.board[1][1].kind = TileKind::Cross;
    let spectrum = game.gem(None, TileKind::Spectrum);
    game.board[2][2] = spectrum;
    let initial = identities(&game);
    let next_id = game.next_id;
    game.score = 175;
    for _ in 0..100 {
        let positions = ids_in_board_order(&game);
        assert!(game.shuffle(false));
        assert_ne!(ids_in_board_order(&game), positions);
        assert_eq!(identities(&game), initial);
        assert_eq!(game.next_id, next_id);
        assert_eq!(game.score, 175);
        assert!(game.matches().cells.is_empty());
        assert!(game.has_move());
    }
    game.mode = Mode::Classic;
    assert!(!game.shuffle(false), "o Clássico não tem embaralhar manual");
}

#[test]
fn low_entropy_random_source_still_finds_a_safe_permutation_without_new_tiles() {
    let mut game = Game::new();
    game.random = Box::new(|| 0.0);
    let mut ids = ids_in_board_order(&game);
    ids.sort();
    assert!(game.shuffle(false));
    let mut after = ids_in_board_order(&game);
    after.sort();
    assert_eq!(after, ids);
    assert!(game.matches().cells.is_empty());
    assert!(game.has_move());
}

#[test]
fn shuffle_survives_a_random_source_that_returns_one() {
    // Um sorteio que devolve exatamente 1.0 não pode apontar para fora da lista.
    let mut game = Game::with_random(lcg_random(7654321));
    game.random = Box::new(|| 1.0);
    let before = identities(&game);
    game.shuffle(true); // o que importa: não entrar em pânico
    assert_eq!(identities(&game), before, "as pedras continuam as mesmas");
    assert!(game.matches().cells.is_empty());
    assert!(game.has_move());
}

#[test]
fn rescue_places_a_spectrum_at_the_first_normal_tile_of_the_centre() {
    let mut game = fixture();
    assert!(!game.has_move());
    let before = ids_in_board_order(&game);
    assert_eq!(game.rescue(), 27);
    let placed = game.tile(27).expect("casa 27 existe");
    assert_eq!(placed.kind, TileKind::Spectrum);
    assert_eq!(placed.color, None);
    let after = ids_in_board_order(&game);
    for index in (0..64).filter(|&i| i != 27) {
        assert_eq!(after[index], before[index], "casa {index} não pode mudar");
    }
    assert!(game.has_move());
}
