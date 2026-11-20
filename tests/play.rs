//! Tarefa 4: a jogada em cascata, os efeitos das pedras especiais e a pontuação.
mod common;

use common::{at, clear, counting_random, fixture, match_move, names, place};
use prisma_engine::{
    Activation, Event, Game, MoveResult, SIZE, TileKind, adjacent, find_matches, lcg_random,
    level_goal, swap_on,
};
use std::collections::HashSet;

/// Monta três pedras especiais de cor 2 (duas já no lugar, a terceira chega por uma troca) e joga.
/// `vertical` escolhe se a fileira de três fica na coluna ou na linha.
fn nova_move(
    game: &mut Game,
    row: usize,
    column: usize,
    vertical: bool,
    types: [TileKind; 3],
) -> MoveResult {
    let indices: Vec<usize> = (0..3)
        .map(|i| if vertical { at(row + i, column) } else { at(row, column + i) })
        .collect();
    let source = if vertical {
        if column == 0 { indices[2] + 1 } else { indices[2] - 1 }
    } else {
        indices[2] - SIZE
    };
    for slot in 0..2 {
        let tile = game.gem(Some(2), types[slot]);
        game.set(indices[slot], tile);
    }
    let plain = game.gem(Some(1), TileKind::Normal);
    game.set(indices[2], plain);
    let from = game.gem(Some(2), types[2]);
    game.set(source, from);
    game.play(source, indices[2])
}

/// Soma dos pontos de todas as limpezas de uma jogada.
fn total_cleared(result: &MoveResult) -> u64 {
    result
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Clear { earned, .. } => Some(*earned),
            _ => None,
        })
        .sum()
}

#[test]
fn invalid_swaps_preserve_board_and_score() {
    let (random, calls) = counting_random(1);
    let mut game = Game::with_random(random);
    let before = (game.board, game.score, game.next_id);
    let draws = calls.get();
    assert!(!game.play(0, 9).valid);
    assert!(!game.play(7, 8).valid);
    // Uma troca de vizinhas que não forma combinação nenhuma.
    let (a, b) = (0..64)
        .find_map(|a| {
            let b = a + 1;
            if !adjacent(a, b) {
                return None;
            }
            let mut board = game.board;
            swap_on(&mut board, a, b);
            find_matches(&board).cells.is_empty().then_some((a, b))
        })
        .expect("deve existir uma troca inútil");
    let result = game.play(a, b);
    assert!(!result.valid);
    assert!(result.events.is_empty());
    assert_eq!((game.board, game.score, game.next_id), before);
    assert_eq!(calls.get(), draws, "jogada inválida não pode gastar sorte");
}

#[test]
fn play_never_panics_or_changes_anything_for_any_index_pair() {
    for a in 0..70 {
        for b in 0..70 {
            let mut game = Game::with_random(lcg_random(7654321));
            let before = (game.board, game.score, game.next_id);
            let result = game.play(a, b);
            if !result.valid {
                assert_eq!((game.board, game.score, game.next_id), before, "jogada ({a}, {b})");
            }
        }
    }
}

#[test]
fn four_five_and_l_t_shapes_create_one_special_at_the_swapped_destination() {
    for (length, expected) in [(4, TileKind::Burst), (5, TileKind::Spectrum)] {
        let mut game = fixture();
        for c in 0..length - 1 {
            place(&mut game, 3, c, Some(0), TileKind::Normal);
        }
        place(&mut game, 3, length - 1, Some(1), TileKind::Normal);
        place(&mut game, 2, length - 1, Some(0), TileKind::Normal);
        let result = game.play(at(2, length - 1), at(3, length - 1));
        assert!(result.valid);
        let first = clear(&result.events[0]);
        assert_eq!(first.creations.len(), 1);
        assert_eq!(first.creations[0].kind, expected);
        assert_eq!(first.creations[0].index, at(3, length - 1));
        assert_eq!(first.cells.len(), length - 1);
        let Event::Fall { board, .. } = &result.events[1] else {
            panic!("o segundo evento deveria ser a queda");
        };
        assert!(board.iter().flatten().any(|tile| tile.kind == expected));
    }
    let mut game = fixture();
    place(&mut game, 3, 2, Some(1), TileKind::Normal);
    place(&mut game, 2, 2, Some(0), TileKind::Normal);
    place(&mut game, 3, 3, Some(0), TileKind::Normal);
    place(&mut game, 3, 4, Some(0), TileKind::Normal);
    place(&mut game, 4, 2, Some(0), TileKind::Normal);
    place(&mut game, 5, 2, Some(0), TileKind::Normal);
    let result = game.play(at(2, 2), at(3, 2));
    let first = clear(&result.events[0]);
    assert_eq!(first.creations[0].kind, TileKind::Cross);
    assert_eq!(first.creations[0].index, at(3, 2));
}

#[test]
fn burst_and_cross_chain_reactions_clear_each_cell_once() {
    let mut game = fixture();
    place(&mut game, 3, 1, Some(0), TileKind::Normal);
    place(&mut game, 3, 2, Some(0), TileKind::Burst);
    place(&mut game, 3, 3, Some(1), TileKind::Normal);
    place(&mut game, 2, 3, Some(0), TileKind::Normal);
    place(&mut game, 2, 2, Some(3), TileKind::Cross);
    let result = game.play(at(2, 3), at(3, 3));
    let first = clear(&result.events[0]);
    assert_eq!(names(first.activated), vec!["burst", "cross"]);
    let unique: HashSet<usize> = first.cells.iter().copied().collect();
    assert_eq!(unique.len(), first.cells.len());
    for c in 0..SIZE {
        assert!(first.cells.contains(&at(2, c)), "linha 2, coluna {c}");
    }
    for r in 0..SIZE {
        assert!(first.cells.contains(&at(r, 2)), "linha {r}, coluna 2");
    }
}

#[test]
fn three_colored_specials_fuse_into_one_centered_5x5_blast_in_either_direction() {
    for vertical in [false, true] {
        for types in [
            [TileKind::Burst, TileKind::Burst, TileKind::Burst],
            [TileKind::Cross, TileKind::Burst, TileKind::Cross],
        ] {
            let mut game = fixture();
            let result = nova_move(&mut game, 3, 2, vertical, types);
            let first = clear(&result.events[0]);
            let center = if vertical { at(4, 2) } else { at(3, 3) };
            assert!(result.valid);
            assert_eq!(names(first.activated), vec!["supernova"]);
            let Activation::Supernova(nova) = &first.activated[0] else {
                panic!("a primeira ativação deveria ser a Supernova");
            };
            assert_eq!(nova.index, center);
            assert_eq!(nova.color, 2);
            assert_eq!(nova.sources.len(), 3);
            assert!(first.creations.is_empty());
            let mut expected = Vec::new();
            for dr in -2i32..=2 {
                for dc in -2i32..=2 {
                    expected.push((center as i32 + dr * SIZE as i32 + dc) as usize);
                }
            }
            expected.sort();
            let mut cells = first.cells.to_vec();
            cells.sort();
            assert_eq!(cells, expected);
            assert_eq!(first.earned, 625);
            assert_eq!(result.earned, total_cleared(&result));
            assert!(game.matches().cells.is_empty());
            let ids: HashSet<u32> = game.board.iter().flatten().map(|tile| tile.id).collect();
            assert_eq!(ids.len(), 64);
        }
    }
}

#[test]
fn supernova_clips_at_the_edge_without_wrapping_to_the_opposite_side() {
    let mut game = fixture();
    let types = [TileKind::Burst; 3];
    let result = nova_move(&mut game, 0, 0, true, types);
    let first = clear(&result.events[0]);
    assert_eq!(names(first.activated), vec!["supernova"]);
    let mut expected = Vec::new();
    for r in 0..=3 {
        for c in 0..=2 {
            expected.push(at(r, c));
        }
    }
    let mut cells = first.cells.to_vec();
    cells.sort();
    assert_eq!(cells, expected);
}

#[test]
fn supernova_activates_other_specials_once_including_a_spectrum_beyond_the_fusion() {
    let mut game = fixture();
    place(&mut game, 2, 2, Some(3), TileKind::Cross);
    place(&mut game, 2, 7, None, TileKind::Spectrum);
    let result = nova_move(&mut game, 3, 2, false, [TileKind::Burst; 3]);
    let first = clear(&result.events[0]);
    assert_eq!(names(first.activated), vec!["supernova", "cross", "spectrum"]);
    let unique: HashSet<usize> = first.cells.iter().copied().collect();
    assert_eq!(unique.len(), first.cells.len());
    assert!(first.cells.contains(&at(2, 7)));
    assert!(first.cells.contains(&at(7, 2)));
}

#[test]
fn a_later_cascade_can_align_three_specials_and_trigger_supernova() {
    let mut game = fixture();
    place(&mut game, 3, 2, Some(2), TileKind::Burst);
    place(&mut game, 3, 3, Some(2), TileKind::Cross);
    place(&mut game, 4, 4, Some(2), TileKind::Burst);
    place(&mut game, 4, 1, Some(0), TileKind::Normal);
    place(&mut game, 4, 2, Some(0), TileKind::Normal);
    place(&mut game, 4, 3, Some(1), TileKind::Normal);
    place(&mut game, 5, 3, Some(0), TileKind::Normal);
    let result = game.play(at(5, 3), at(4, 3));
    let clears: Vec<_> = result
        .events
        .iter()
        .filter(|event| event.name() == "clear")
        .map(clear)
        .collect();
    assert!(clears[0].activated.is_empty());
    assert!(clears[1..].iter().any(|step| step.chain > 1
        && step.activated.iter().any(|activation| activation.name() == "supernova")));
}

#[test]
fn two_adjacent_specials_keep_their_own_effects() {
    // Dois especiais seguidos, ou separados por uma pedra comum, não formam Supernova.
    for types in [
        [TileKind::Burst, TileKind::Burst, TileKind::Normal],
        [TileKind::Burst, TileKind::Normal, TileKind::Cross],
    ] {
        let mut game = fixture();
        let result = nova_move(&mut game, 3, 2, false, types);
        let first = clear(&result.events[0]);
        assert!(!first.activated.iter().any(|activation| activation.name() == "supernova"));
        assert_eq!(first.activated.len(), 2);
    }
}

#[test]
fn an_indirectly_triggered_spectrum_clears_a_surviving_color_after_surrounding_bursts() {
    let mut game = fixture();
    place(&mut game, 3, 1, Some(0), TileKind::Normal);
    place(&mut game, 3, 2, Some(0), TileKind::Burst);
    place(&mut game, 3, 3, Some(1), TileKind::Normal);
    place(&mut game, 2, 3, Some(0), TileKind::Normal);
    place(&mut game, 2, 2, None, TileKind::Spectrum);
    place(&mut game, 2, 1, Some(3), TileKind::Burst);
    place(&mut game, 4, 2, Some(5), TileKind::Burst);
    let result = game.play(at(2, 3), at(3, 3));
    let first = clear(&result.events[0]);
    assert!(result.valid);
    assert_eq!(names(first.activated), vec!["burst", "burst", "burst", "spectrum"]);
    assert!(
        first.cells.contains(&at(7, 7)),
        "o Espectro remove uma pedra distante de uma cor que ainda existe depois dos Pulsos"
    );
    let unique: HashSet<usize> = first.cells.iter().copied().collect();
    assert_eq!(unique.len(), first.cells.len());
}

#[test]
fn creating_a_special_never_replaces_an_existing_special_in_the_same_match() {
    let mut game = fixture();
    for c in 0..3 {
        place(&mut game, 3, c, Some(0), TileKind::Normal);
    }
    place(&mut game, 3, 3, Some(1), TileKind::Normal);
    place(&mut game, 2, 3, Some(0), TileKind::Burst);
    let result = game.play(at(2, 3), at(3, 3));
    let first = clear(&result.events[0]);
    assert_eq!(first.creations.len(), 1);
    assert_ne!(first.creations[0].index, at(3, 3));
    assert!(first.activated.iter().any(|activation| activation.name() == "burst"));
}

#[test]
fn spectrum_swaps_clear_target_color_and_two_spectra_clear_all() {
    let mut game = fixture();
    place(&mut game, 1, 1, None, TileKind::Spectrum);
    place(&mut game, 1, 2, Some(4), TileKind::Normal);
    let count = game.board.iter().flatten().filter(|tile| tile.color == Some(4)).count();
    let result = game.play(at(1, 1), at(1, 2));
    let first = clear(&result.events[0]);
    assert_eq!(first.activated[0].name(), "spectrum");
    assert!(first.cells.len() >= count + 1);

    let mut both = fixture();
    place(&mut both, 1, 1, None, TileKind::Spectrum);
    place(&mut both, 1, 2, None, TileKind::Spectrum);
    let result = both.play(at(1, 1), at(1, 2));
    assert_eq!(clear(&result.events[0]).cells.len(), 64);
}

#[test]
fn score_rewards_special_creation_and_cascades_with_a_capped_multiplier() {
    let mut game = fixture();
    for c in 0..3 {
        place(&mut game, 3, c, Some(0), TileKind::Normal);
    }
    place(&mut game, 3, 3, Some(1), TileKind::Normal);
    place(&mut game, 2, 3, Some(0), TileKind::Normal);
    let result = game.play(at(2, 3), at(3, 3));
    let first = clear(&result.events[0]);
    assert_eq!(first.base, 75);
    assert_eq!(first.creation_bonus, 120);
    assert_eq!(first.combo_multiplier, 1.0);
    assert_eq!(first.earned, 195);
    assert_eq!(result.earned, total_cleared(&result));
    for event in result.events.iter().filter(|event| event.name() == "clear") {
        let multiplier = clear(event).combo_multiplier;
        assert!((1.0..=3.0).contains(&multiplier), "multiplicador {multiplier}");
    }
}

#[test]
fn level_goals_grow_gradually_to_6000_and_progress_can_cross_multiple_levels() {
    let levels = [1, 2, 3, 4, 5, 10, 19, 20, 21, 22, 23, 28, 29, 100];
    let goals: Vec<u64> = levels.iter().map(|&level| level_goal(level)).collect();
    assert_eq!(
        goals,
        vec![1800, 1950, 2100, 2250, 2400, 3150, 4500, 4650, 4800, 4950, 5100, 5850, 6000, 6000]
    );

    let mut game = Game::with_random(lcg_random(7654321));
    game.score = level_goal(1) - 50;
    let (a, b) = match_move(&game);
    let result = game.play(a, b);
    assert!(result.levels_gained > 0);
    assert_eq!(game.level, 2);
    assert_eq!(game.level_start_score, level_goal(1));
    assert_eq!(game.score - game.level_start_score, game.score - level_goal(1));
}

#[test]
fn a_level_start_beyond_the_score_does_not_panic_the_level_count() {
    // Os campos de `Game` são públicos: um chamador pode deixá-los inconsistentes.
    // A jogada não pode dar pânico por causa disso (a conta de subtração nunca "desce de zero").
    let mut game = Game::with_random(lcg_random(7654321));
    game.level_start_score = game.score + 10_000;
    let (a, b) = match_move(&game);
    let result = game.play(a, b);
    assert!(result.valid);
    assert_eq!(result.levels_gained, 0, "pontos abaixo do início do nível não sobem de nível");
}
