//! Tarefa 6: restaurar partidas salvas (inclusive saves antigos e saves estragados).
mod common;

use common::{counting_random, fixture, match_move, place};
use prisma_engine::{
    Board, Game, Mode, SavedGame, SavedMode, SavedTile, Tile, TileKind, lcg_random, level_goal,
};

/// O maior número "seguro" do JS (2^53 - 1): os saves nunca passam disso.
const MAX_SAFE: i64 = 9_007_199_254_740_991;

/// Um jogo de teste com sorte repetível.
fn new_game() -> Game {
    Game::with_random(lcg_random(7654321))
}

/// O tabuleiro como o save de hoje o guarda: pedra por pedra, com id, tipo e cor.
fn full_board(board: &Board) -> Vec<Vec<SavedTile>> {
    board
        .iter()
        .map(|row| {
            row.iter()
                .map(|tile| SavedTile::Full {
                    id: i64::from(tile.id),
                    kind: tile.kind,
                    color: tile.color.map(i64::from),
                })
                .collect()
        })
        .collect()
}

/// O tabuleiro como os saves antigos o guardavam: só o número da cor de cada pedra.
fn legacy_board(board: &Board) -> Vec<Vec<SavedTile>> {
    board
        .iter()
        .map(|row| {
            row.iter()
                .map(|tile| SavedTile::Legacy(i64::from(tile.color.expect("só pedras coloridas"))))
                .collect()
        })
        .collect()
}

fn save(mode: SavedMode, score: i64, board: Vec<Vec<SavedTile>>) -> SavedGame {
    SavedGame { mode, score, board, ..Default::default() }
}

/// Tudo o que um save recusado não pode ter mexido, num formato que o `assert_eq!` compara.
#[derive(Debug, PartialEq)]
struct Untouched {
    board: Board,
    next_id: u32,
    mode: Mode,
    score: u64,
    level: u32,
    level_start_score: u64,
    progression_offset: u64,
    ended: bool,
    cheats_used: bool,
    /// Quantas jogadas o "desfazer" ainda guarda.
    undo_len: usize,
}

fn untouched(game: &Game) -> Untouched {
    Untouched {
        board: game.board,
        next_id: game.next_id,
        mode: game.mode,
        score: game.score,
        level: game.level,
        level_start_score: game.level_start_score,
        progression_offset: game.progression_offset,
        ended: game.ended,
        cheats_used: game.cheats_used,
        undo_len: game.undo_history.len(),
    }
}

fn sorted_tiles(board: &Board) -> Vec<Tile> {
    let mut tiles: Vec<Tile> = board.iter().flatten().copied().collect();
    tiles.sort_by_key(|tile| tile.id);
    tiles
}

#[test]
fn old_numeric_save_migrates_specials_persist_ids_remain_unique() {
    let old = fixture();
    let mut game = new_game();
    assert!(game.restore(&save(SavedMode::Endless, 2120, legacy_board(&old.board))));
    assert_eq!(game.mode, Mode::Classic, "o modo Endless virou Clássico");
    assert_eq!(game.level, 2);

    let special = game.gem(Some(2), TileKind::Burst);
    game.set(0, special);
    let mut copy = new_game();
    assert!(copy.restore(&save(SavedMode::Classic, game.score as i64, full_board(&game.board))));
    let restored = copy.tile(0).expect("casa 0");
    assert_eq!(restored.id, special.id);
    assert_eq!(restored.kind, TileKind::Burst);
    let highest = copy.board.iter().flatten().map(|tile| tile.id).max().expect("64 pedras");
    assert!(copy.random_gem().id > highest, "as próximas pedras não repetem ids");
}

#[test]
fn restoring_an_unsettled_save_rearranges_its_specials_instead_of_discarding_them() {
    let mut original = new_game();
    place(&mut original, 0, 0, Some(0), TileKind::Burst);
    place(&mut original, 0, 1, Some(0), TileKind::Cross);
    place(&mut original, 0, 2, Some(0), TileKind::Normal);
    place(&mut original, 1, 1, None, TileKind::Spectrum);
    assert!(!original.matches().cells.is_empty(), "o save começa com uma combinação pronta");
    let expected = sorted_tiles(&original.board);

    let mut restored = new_game();
    assert!(restored.restore(&save(SavedMode::Zen, 200, full_board(&original.board))));
    assert_eq!(sorted_tiles(&restored.board), expected, "nenhuma pedra some nem muda");
    assert!(restored.matches().cells.is_empty());
    assert!(restored.has_move());
}

#[test]
fn a_finished_classic_save_comes_back_finished_even_with_moves_left() {
    let game = new_game();
    assert!(game.has_move(), "o tabuleiro tem jogadas; só o save diz que acabou");
    let mut saved = game.to_saved();
    saved.mode = SavedMode::Classic;
    saved.ended = true;

    let mut restored = new_game();
    assert!(restored.restore(&saved));
    assert!(restored.ended);
}

#[test]
fn the_ended_mark_is_ignored_where_the_game_cannot_end() {
    let mut saved = new_game().to_saved();
    saved.ended = true;

    // O Zen nunca termina.
    saved.mode = SavedMode::Zen;
    let mut zen = new_game();
    assert!(zen.restore(&saved));
    assert!(!zen.ended);

    // O Clássico com "sem derrota" ligado também não.
    saved.mode = SavedMode::Classic;
    saved.no_defeat = true;
    let mut no_defeat = new_game();
    assert!(no_defeat.restore(&saved));
    assert!(!no_defeat.ended);
}

#[test]
fn a_no_moves_save_restores_as_finished_classic_or_playable_zen() {
    let original = fixture();
    assert!(!original.has_move());
    let board = full_board(&original.board);

    let mut classic = new_game();
    assert!(classic.restore(&save(SavedMode::Classic, 2400, board.clone())));
    assert_eq!(classic.level, 2);
    assert!(classic.ended);
    assert!(!classic.has_move());

    let mut zen = new_game();
    assert!(zen.restore(&save(SavedMode::Zen, 2400, board)));
    assert_eq!(zen.level, 2);
    assert!(!zen.ended);
    assert!(zen.has_move());
    let spectra = zen.board.iter().flatten().filter(|tile| tile.kind == TileKind::Spectrum).count();
    assert_eq!(spectra, 1);
}

#[test]
fn v2_and_v3_progression_migrate_to_6000_without_losing_level_points_or_progress() {
    let original = fixture();
    let board = full_board(&original.board);
    let versioned = |version, score, level, start, offset| SavedGame {
        mode: SavedMode::Zen,
        score,
        board: board.clone(),
        progression_version: Some(version),
        level: Some(level),
        level_start_score: Some(start),
        progression_offset: offset,
        ..Default::default()
    };

    let mut copy = new_game();
    assert!(copy.restore(&versioned(2, 4321, 3, 3750, None)));
    assert_eq!(copy.level, 3);
    assert_eq!(copy.level_start_score, 3750);
    assert_eq!(copy.score - copy.level_start_score, 571);

    let legacy_start = 19 * 1800 + 19 * 18 * 75 + 5 * 4650;
    let mut high = new_game();
    assert!(high.restore(&versioned(2, legacy_start + 4400, 25, legacy_start, None)));
    assert_eq!(high.level, 25);
    assert_eq!(high.score - high.level_start_score, 4400);

    let mut resumed = new_game();
    assert!(resumed.restore(&versioned(
        4,
        high.score as i64,
        i64::from(high.level),
        high.level_start_score as i64,
        Some(high.progression_offset as i64),
    )));
    assert_eq!(resumed.level, high.level);
    assert_eq!(resumed.level_start_score, high.level_start_score);
    assert_eq!(resumed.progression_offset, high.progression_offset);

    let old_cap_start = 22 * 1800 + 22 * 21 * 75 + 6 * 5000;
    let mut v3 = new_game();
    assert!(v3.restore(&versioned(3, old_cap_start + 4900, 29, old_cap_start, Some(0))));
    assert_eq!(v3.level, 29);
    assert_eq!(v3.score - v3.level_start_score, 4900);
    assert_eq!(level_goal(v3.level), 6000);

    let mut next = new_game();
    assert!(next.restore(&versioned(
        4,
        v3.score as i64,
        i64::from(v3.level),
        v3.level_start_score as i64,
        Some(v3.progression_offset as i64),
    )));
    assert_eq!(next.level_start_score, old_cap_start as u64);
    assert_eq!(next.progression_offset, v3.progression_offset);
}

#[test]
fn corrupt_saves_leave_the_current_game_untouched() {
    let (random, calls) = counting_random(5);
    let mut game = Game::with_random(random);
    let (a, b) = match_move(&game);
    assert!(game.play(a, b).valid); // agora o histórico de "desfazer" tem uma jogada guardada
    assert_eq!(game.undo_history.len(), 1);
    let before = untouched(&game);
    let draws = calls.get();

    let mut duplicate = full_board(&game.board);
    let SavedTile::Full { id: first_id, .. } = duplicate[0][0].clone() else { unreachable!() };
    if let SavedTile::Full { id, .. } = &mut duplicate[7][7] {
        *id = first_id; // dois tiles com o mesmo id
    }
    assert!(!game.restore(&save(SavedMode::Classic, 20, duplicate)));

    let wrong_level = SavedGame {
        mode: SavedMode::Classic,
        score: 10,
        board: full_board(&game.board),
        progression_version: Some(2),
        level: Some(999),
        level_start_score: Some(0),
        ..Default::default()
    };
    assert!(!game.restore(&wrong_level));
    assert_eq!(untouched(&game), before, "inclui o histórico de desfazer");
    assert_eq!(calls.get(), draws, "um save recusado não gasta sorte");
}

#[test]
fn restore_clears_developer_undo() {
    let mut game = new_game();
    let (a, b) = match_move(&game);
    assert!(game.play(a, b).valid);
    assert!(game.can_undo_last_move());

    let saved = SavedGame {
        mode: SavedMode::Zen,
        score: game.score as i64,
        board: full_board(&game.board),
        progression_version: Some(4),
        level: Some(i64::from(game.level)),
        level_start_score: Some(game.level_start_score as i64),
        progression_offset: Some(game.progression_offset as i64),
        ..Default::default()
    };
    let mut restored = new_game();
    let (c, d) = match_move(&restored);
    assert!(restored.play(c, d).valid);
    assert!(restored.can_undo_last_move());
    assert!(restored.restore(&saved));
    assert!(!restored.can_undo_last_move());
}

#[test]
fn restore_rejects_boards_with_the_wrong_shape() {
    let source = new_game();
    let good = full_board(&source.board);
    assert!(new_game().restore(&save(SavedMode::Zen, 0, good.clone())), "o save sem defeito carrega");

    let mut seven_rows = good.clone();
    seven_rows.pop();
    let mut nine_tiles = good.clone();
    nine_tiles[3].push(SavedTile::Legacy(0));
    let mut seven_tiles = good.clone();
    seven_tiles[5].pop();
    let mut nine_rows = good.clone();
    nine_rows.push(good[0].clone());

    for (name, board) in [
        ("7 linhas", seven_rows),
        ("uma linha com 9 pedras", nine_tiles),
        ("uma linha com 7 pedras", seven_tiles),
        ("9 linhas", nine_rows),
        ("tabuleiro vazio", Vec::new()),
    ] {
        let mut game = new_game();
        let before = untouched(&game);
        assert!(!game.restore(&save(SavedMode::Zen, 0, board)), "{name}");
        assert_eq!(untouched(&game), before, "{name}");
    }
}

#[test]
fn restore_rejects_invalid_tiles() {
    let source = new_game();
    let good = full_board(&source.board);
    let bad_tiles = [
        SavedTile::Legacy(7),
        SavedTile::Legacy(-1),
        SavedTile::Full { id: 0, kind: TileKind::Normal, color: Some(0) },
        SavedTile::Full { id: -4, kind: TileKind::Normal, color: Some(0) },
        SavedTile::Full { id: i64::MAX, kind: TileKind::Normal, color: Some(0) },
        SavedTile::Full { id: 3_000_000_000, kind: TileKind::Normal, color: Some(0) },
        SavedTile::Full { id: 900, kind: TileKind::Normal, color: None },
        SavedTile::Full { id: 900, kind: TileKind::Burst, color: Some(7) },
        SavedTile::Full { id: 900, kind: TileKind::Spectrum, color: Some(1) },
    ];
    for tile in bad_tiles {
        let mut board = good.clone();
        board[2][2] = tile.clone();
        let mut game = new_game();
        let before = untouched(&game);
        assert!(!game.restore(&save(SavedMode::Zen, 0, board)), "{tile:?}");
        assert_eq!(untouched(&game), before, "{tile:?}");
    }
    let mut spectrum_board = good;
    spectrum_board[2][2] = SavedTile::Full { id: 900, kind: TileKind::Spectrum, color: None };
    assert!(new_game().restore(&save(SavedMode::Zen, 0, spectrum_board)), "Espectro sem cor vale");
}

#[test]
fn restore_rejects_absurdly_large_numbers_without_overflow() {
    let source = new_game();
    let board = full_board(&source.board);
    for version in [Some(2), Some(3), Some(4), None] {
        for huge in [i64::MAX, MAX_SAFE, MAX_SAFE - 1] {
            let saved = SavedGame {
                mode: SavedMode::Zen,
                score: huge,
                board: board.clone(),
                progression_version: version,
                level: Some(huge),
                level_start_score: Some(huge),
                progression_offset: Some(huge),
                ..Default::default()
            };
            let mut game = new_game();
            let before = untouched(&game);
            assert!(!game.restore(&saved), "versão {version:?}, valor {huge}");
            assert_eq!(untouched(&game), before, "versão {version:?}, valor {huge}");
        }
        // Negativos, inclusive o menor número possível.
        for negative in [-1, i64::MIN] {
            let saved = SavedGame {
                mode: SavedMode::Zen,
                score: negative,
                board: board.clone(),
                progression_version: version,
                level: Some(1),
                level_start_score: Some(0),
                progression_offset: Some(0),
                ..Default::default()
            };
            let mut game = new_game();
            let before = untouched(&game);
            assert!(!game.restore(&saved), "pontos {negative}, versão {version:?}");
            assert_eq!(untouched(&game), before);
        }
        // Nível, início e deslocamento absurdos com pontos normais.
        for weird in [i64::MAX, i64::MIN, -1] {
            let saved = SavedGame {
                mode: SavedMode::Zen,
                score: 10,
                board: board.clone(),
                progression_version: version,
                level: Some(weird),
                level_start_score: Some(weird),
                progression_offset: Some(weird),
                ..Default::default()
            };
            let mut game = new_game();
            let before = untouched(&game);
            let accepted = game.restore(&saved);
            if version.is_some() {
                assert!(!accepted, "versão {version:?}, valor {weird}");
                assert_eq!(untouched(&game), before);
            }
        }
    }
}

#[test]
fn restore_without_a_version_rejects_scores_whose_level_does_not_fit() {
    let source = new_game();
    // Seguro para o JS, mas o nível passaria de 4 bilhões.
    let saved = save(SavedMode::Zen, MAX_SAFE, full_board(&source.board));
    let mut game = new_game();
    let before = untouched(&game);
    assert!(!game.restore(&saved));
    assert_eq!(untouched(&game), before);
}

#[test]
fn restore_hands_the_random_source_back_even_when_it_fails() {
    let (random, calls) = counting_random(3);
    let mut game = Game::with_random(random);
    let source = new_game();
    let board = full_board(&source.board);

    let draws = calls.get();
    assert!(!game.restore(&SavedGame::default()), "tabuleiro vazio");
    (game.random)();
    assert_eq!(calls.get(), draws + 1, "depois de um save recusado");

    assert!(game.restore(&save(SavedMode::Zen, 0, board)));
    let draws = calls.get();
    (game.random)();
    assert_eq!(calls.get(), draws + 1, "depois de um save aceito");

    // Um save com combinação pronta precisa de sorte para embaralhar: tem que ser a MESMA fonte.
    let mut unsettled = source;
    place(&mut unsettled, 0, 0, Some(0), TileKind::Normal);
    place(&mut unsettled, 0, 1, Some(0), TileKind::Normal);
    place(&mut unsettled, 0, 2, Some(0), TileKind::Normal);
    let draws = calls.get();
    assert!(game.restore(&save(SavedMode::Zen, 0, full_board(&unsettled.board))));
    assert!(calls.get() > draws, "o embaralhar usou a fonte de sorte do jogo");
    let draws = calls.get();
    (game.random)();
    assert_eq!(calls.get(), draws + 1, "e ela continua no jogo depois do embaralhar");
}

#[test]
fn to_saved_round_trips_through_restore() {
    let mut game = new_game();
    for _ in 0..30 {
        let (a, b) = match_move(&game);
        assert!(game.play(a, b).valid);
    }
    game.cheats_used = true;
    let saved = game.to_saved();
    assert_eq!(saved.progression_version, Some(4));
    assert_eq!(saved.mode, SavedMode::Zen);

    let mut copy = new_game();
    assert!(copy.restore(&saved));
    assert_eq!(copy.board, game.board);
    assert_eq!(copy.score, game.score);
    assert_eq!(copy.level, game.level);
    assert_eq!(copy.level_start_score, game.level_start_score);
    assert_eq!(copy.progression_offset, game.progression_offset);
    assert_eq!(copy.mode, game.mode);
    assert_eq!(copy.ended, game.ended);
    assert!(copy.cheats_used && !copy.no_defeat);
    let highest = copy.board.iter().flatten().map(|tile| tile.id).max().expect("64 pedras");
    assert!(copy.next_id > highest, "as próximas pedras não repetem ids do tabuleiro");
}

#[test]
fn a_save_at_the_highest_level_can_still_be_played_without_overflow() {
    let source = new_game();
    // Nível 4294967295 (o maior que cabe) com 5999 pontos de progresso: qualquer jogada sobe de nível.
    let level = i64::from(u32::MAX);
    let start = 107_100 + (level - 29) * 6000; // início desse nível, com a meta já no teto de 6000
    let saved = SavedGame {
        mode: SavedMode::Zen,
        score: start + 5999,
        board: full_board(&source.board),
        progression_version: Some(4),
        level: Some(level),
        level_start_score: Some(start),
        progression_offset: Some(0),
        ..Default::default()
    };
    let mut game = new_game();
    assert!(game.restore(&saved));
    let (a, b) = match_move(&game);
    let result = game.play(a, b);
    assert!(result.valid);
    assert_eq!(game.level, u32::MAX, "o nível para no máximo em vez de estourar");
}

#[test]
fn restore_refuses_when_the_id_counter_has_no_room_left_for_new_tiles() {
    let source = new_game();
    // Um save antigo precisa de ids novos para as 64 pedras: com o contador no limite não há de onde tirar.
    let old_save = save(SavedMode::Zen, 0, legacy_board(&source.board));
    let (random, calls) = counting_random(3);
    let mut game = Game::with_random(random);
    game.next_id = u32::MAX;
    let before = untouched(&game);
    let draws = calls.get();
    assert!(!game.restore(&old_save), "sem folga nos ids o save é recusado em vez de dar pânico");
    assert_eq!(untouched(&game), before);
    (game.random)();
    assert_eq!(calls.get(), draws + 1, "a fonte de sorte continua a original");
}
