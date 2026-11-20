//! Tarefa 7: os cheats de desenvolvedor (pontos, nível, pedras especiais, embaralhar, explosões).
mod common;

use common::{at, clear, fixture, place};
use prisma_engine::{Activation, Game, Mode, SIZE, TileKind, lcg_random, level_goal};
use std::collections::HashSet;

/// Um jogo de teste com sorte repetível.
fn new_game() -> Game {
    Game::with_random(lcg_random(7654321))
}

/// O tabuleiro está íntegro: 64 pedras, ids únicos, sem combinações e com jogada (ou partida encerrada).
fn valid_board(game: &Game) {
    assert_eq!(game.board.iter().flatten().count(), SIZE * SIZE);
    let ids: HashSet<u32> = game.board.iter().flatten().map(|tile| tile.id).collect();
    assert_eq!(ids.len(), SIZE * SIZE);
    assert!(game.matches().cells.is_empty());
    assert!(game.has_move() || game.ended);
}

#[test]
fn points_and_level_cheats_preserve_valid_progress_and_reject_invalid_input_without_side_effects() {
    let mut game = new_game();
    let before = game.snapshot();
    for amount in [0, -1, 1_000_001, i64::MIN, i64::MAX] {
        assert!(!game.cheat_add_points(amount), "pontos {amount}");
    }
    for level in [0, -1, 10_001, i64::MIN, i64::MAX] {
        assert!(!game.cheat_set_level(level), "nível {level}");
    }
    assert_eq!(game.snapshot(), before);
    assert!(!game.cheats_used);
    assert!(game.undo_history.is_empty());

    assert!(game.cheat_add_points(1_000_000));
    assert!(game.score - game.level_start_score < level_goal(game.level));
    assert!(game.cheat_set_level(10_000));
    let mut reopened = new_game();
    assert!(reopened.restore(&game.to_saved()));
    assert_eq!(reopened.level, 10_000);
    assert_eq!(reopened.score, game.score);
    assert!(reopened.cheats_used);
}

#[test]
fn add_points_cannot_push_the_score_past_one_billion() {
    let mut game = new_game();
    game.score = 999_999_999;
    let before = game.snapshot();
    assert!(!game.cheat_add_points(2));
    assert_eq!(game.snapshot(), before);
    assert!(!game.cheats_used);
    assert!(game.cheat_add_points(1));
    assert_eq!(game.score, 1_000_000_000);
    assert!(game.score - game.level_start_score < level_goal(game.level));
}

#[test]
fn invalid_cheat_requests_change_nothing() {
    let mut game = new_game();
    let before = game.snapshot();
    for index in [-1, 64, i64::MIN, i64::MAX] {
        assert!(!game.cheat_spawn_special(index, TileKind::Burst), "índice {index}");
        assert!(!game.cheat_supernova(index).valid, "Supernova no índice {index}");
    }
    assert!(!game.cheat_spawn_special(0, TileKind::Normal), "pedra comum não é especial");
    assert!(!game.cheat_no_defeat(false), "já estava desligado");
    assert_eq!(game.snapshot(), before);
    assert!(!game.cheats_used);
    assert!(game.undo_history.is_empty());
    assert!(game.cheat_supernova(-1).events.is_empty());
}

#[test]
fn special_cheats_preserve_other_tiles_and_resolve_spectrum_color_safely() {
    let mut game = new_game();
    let before = game.snapshot();
    assert!(!game.cheat_spawn_special(-1, TileKind::Burst));
    assert!(!game.cheat_spawn_special(64, TileKind::Burst));
    assert!(!game.cheat_spawn_special(0, TileKind::Normal));
    assert!(!game.cheats_used);
    for kind in [TileKind::Spectrum, TileKind::Burst, TileKind::Cross] {
        assert!(game.cheat_spawn_special(27, kind));
        assert_eq!(game.tile(27).map(|tile| tile.kind), Some(kind));
        valid_board(&game);
        for cell in (0..SIZE * SIZE).filter(|&cell| cell != 27) {
            assert_eq!(game.tile(cell), Some(before.board[cell / SIZE][cell % SIZE]), "casa {cell}");
        }
    }
}

#[test]
fn spawning_a_burst_over_a_spectrum_picks_a_color_that_forms_no_match() {
    let mut game = fixture();
    place(&mut game, 3, 3, None, TileKind::Spectrum);
    // A cor 0 formaria uma trinca com (3,1) e (3,2); a cor 1, com (3,4) e (3,5).
    place(&mut game, 3, 1, Some(0), TileKind::Normal);
    place(&mut game, 3, 2, Some(0), TileKind::Normal);
    place(&mut game, 3, 4, Some(1), TileKind::Normal);
    place(&mut game, 3, 5, Some(1), TileKind::Normal);
    assert!(game.matches().cells.is_empty());
    assert!(game.cheat_spawn_special(at(3, 3) as i64, TileKind::Burst));
    let tile = game.tile(at(3, 3)).expect("casa (3,3)");
    assert_eq!(tile.kind, TileKind::Burst);
    assert_eq!(tile.color, Some(2), "as cores 0 e 1 formariam combinação");
    assert!(game.matches().cells.is_empty());
}

#[test]
fn secret_shuffle_resumes_classic_preserves_tiles_and_remains_reversible() {
    let mut game = new_game();
    game.new_game(Mode::Classic);
    assert!(game.cheat_spawn_special(0, TileKind::Cross));
    game.ended = true;
    let before = game.snapshot();
    assert!(!game.shuffle(false), "no Clássico o jogador não embaralha");
    assert!(game.cheat_shuffle());
    assert!(!game.ended);
    let mut ids_before: Vec<u32> = before.board.iter().flatten().map(|tile| tile.id).collect();
    let mut ids_after: Vec<u32> = game.board.iter().flatten().map(|tile| tile.id).collect();
    ids_before.sort();
    ids_after.sort();
    assert_eq!(ids_after, ids_before);
    valid_board(&game);
    assert!(game.undo_last_move());
    assert_eq!(game.snapshot(), before);
}

#[test]
fn no_defeat_recovers_a_deadlocked_classic_and_survives_saving_normal_classic_still_ends() {
    let mut game = new_game();
    game.new_game(Mode::Classic);
    for r in 0..SIZE {
        for c in 0..SIZE {
            game.board[r][c].color = Some(((r * 2 + c) % 7) as u8);
        }
    }
    assert!(!game.has_move());
    game.ended = true;
    assert!(game.cheat_no_defeat(true));
    assert!(!game.ended);
    assert!(game.has_move());

    let mut reopened = new_game();
    assert!(reopened.restore(&game.to_saved()));
    assert!(reopened.no_defeat);
    assert!(reopened.cheats_used);

    let (a, b) = game.hint().expect("depois do resgate há jogada");
    game.force_no_moves = true;
    let result = game.play(a, b);
    game.force_no_moves = false;
    assert!(!result.ended);
    assert!(result.rescued);
    assert!(game.cheat_no_defeat(false));
    assert!(game.undo_last_move());
    assert!(game.no_defeat);
}

#[test]
fn explode_activates_every_special_once_refills_and_undoes_the_complete_cascade() {
    let mut game = new_game();
    for (index, kind) in [(0, TileKind::Burst), (27, TileKind::Cross), (63, TileKind::Spectrum)] {
        assert!(game.cheat_spawn_special(index, kind));
    }
    let before = game.snapshot();
    let ids: Vec<u32> = [0, 27, 63].iter().map(|&i| game.tile(i).expect("casa").id).collect();
    let result = game.cheat_explode();
    assert!(result.valid);
    let first = clear(&result.events[0]);
    assert_eq!(first.cells.len(), 64);
    for id in ids {
        let times = first
            .activated
            .iter()
            .filter(|activation| matches!(activation, Activation::Special { id: found, .. } if *found == id))
            .count();
        assert_eq!(times, 1, "a pedra especial {id} dispara uma única vez");
    }
    assert!(result.earned >= 1600);
    valid_board(&game);
    assert!(game.undo_last_move());
    assert_eq!(game.snapshot(), before);
}

#[test]
fn forced_supernova_uses_real_fusion_at_every_corner_and_is_reversible() {
    for index in [0, 7, 56, 63, 27] {
        let mut game = new_game();
        let before = game.snapshot();
        let result = game.cheat_supernova(index);
        assert!(result.valid, "índice {index}");
        let first = clear(&result.events[0]);
        let nova = first.activated.iter().find_map(|activation| match activation {
            Activation::Supernova(nova) => Some(nova),
            Activation::Special { .. } => None,
        });
        let nova = nova.expect("a fusão cria uma Supernova");
        assert_eq!(nova.sources.len(), 3);
        assert!(first.creations.is_empty());
        valid_board(&game);
        assert!(game.undo_last_move());
        assert_eq!(game.snapshot(), before, "índice {index}");
    }
}

#[test]
fn history_mixes_normal_moves_and_cheats_retains_50_actions_and_keeps_cheat_status_after_undo() {
    let mut game = new_game();
    let before = game.snapshot();
    let (a, b) = game.hint().expect("jogo novo tem jogada");
    assert!(game.play(a, b).valid);
    let moved = game.snapshot();
    assert!(game.cheat_add_points(100));
    assert!(game.cheat_set_level(8));
    assert!(game.undo_last_move());
    assert_eq!(game.score, moved.score + 100);
    assert!(game.undo_last_move());
    assert_eq!(game.snapshot(), moved);
    assert!(game.undo_last_move());
    assert_eq!(game.snapshot(), before);
    assert!(game.cheats_used, "desfazer não apaga a marca de cheat");

    for _ in 0..60 {
        assert!(game.cheat_add_points(100));
    }
    assert_eq!(game.undo_history.len(), 50);
    for _ in 0..50 {
        assert!(game.undo_last_move());
    }
    assert_eq!(game.score, 1000);
    assert!(!game.undo_last_move());

    let mut reopened = new_game();
    assert!(reopened.restore(&game.to_saved()));
    assert!(reopened.undo_history.is_empty());
    assert!(reopened.cheats_used);
    game.restart();
    assert!(!game.cheats_used);
    assert!(!game.no_defeat);
}
