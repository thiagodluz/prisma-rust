//! Tarefa 5: o fim da jogada (reembaralhar, resgate, fim de partida) e o "desfazer" de desenvolvedor.
mod common;

use common::{at, fixture, match_move, place};
use prisma_engine::{Board, Event, Game, Mode, SIZE, Tile, TileKind, lcg_random, level_goal};
use std::collections::HashSet;

/// O id da pedra na casa `index` de um tabuleiro.
fn id_at(board: &Board, index: usize) -> u32 {
    board[index / SIZE][index % SIZE].id
}

/// Os campos que o "desfazer" precisa devolver ao estado anterior, num formato comparável.
fn state(game: &Game) -> (u32, Board, u64, u32, u64, u64, Mode, bool) {
    (
        game.next_id,
        game.board,
        game.score,
        game.level,
        game.level_start_score,
        game.progression_offset,
        game.mode,
        game.ended,
    )
}

#[test]
fn both_modes_reshuffle_once_when_leveling_up_and_keep_surviving_specials() {
    for mode in [Mode::Classic, Mode::Zen] {
        let mut game = Game::with_random(lcg_random(9921));
        game.new_game(mode);
        game.board[0][0].kind = TileKind::Burst;
        game.score = level_goal(1) - 75;
        let (a, b) = match_move(&game);
        let result = game.play(a, b);
        assert!(result.valid && result.levels_gained > 0);
        assert!(result.shuffled);

        let shuffles: Vec<(&Board, &Board)> = result
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Shuffle { before, board, .. } => Some((before, board)),
                _ => None,
            })
            .collect();
        assert_eq!(shuffles.len(), 1, "só um embaralhar por jogada ({mode:?})");
        let (before, after) = shuffles[0];

        // As mesmas pedras (id, tipo e cor), só em outros lugares.
        let mut before_tiles: Vec<Tile> = before.iter().flatten().copied().collect();
        let mut after_tiles: Vec<Tile> = after.iter().flatten().copied().collect();
        let before_ids: Vec<u32> = before_tiles.iter().map(|tile| tile.id).collect();
        let after_ids: Vec<u32> = after_tiles.iter().map(|tile| tile.id).collect();
        before_tiles.sort_by_key(|tile| tile.id);
        after_tiles.sort_by_key(|tile| tile.id);
        assert_eq!(after_tiles, before_tiles);
        assert_ne!(before_ids, after_ids, "o embaralhar tem que mudar a ordem");

        assert!(game.matches().cells.is_empty());
        assert!(game.has_move());
        assert!(!game.ended);
        let Some(Event::Shuffle { board, .. }) = result.events.last() else {
            panic!("o último evento deveria ser o embaralhar ({mode:?})");
        };
        assert_eq!(*board, game.board);
    }
}

#[test]
fn classic_ends_without_moves_preserves_the_final_board_and_forbids_shuffling() {
    let mut game = fixture();
    game.new_game(Mode::Classic);
    let (a, b) = match_move(&game);
    game.force_no_moves = true; // um estado sem jogadas, sempre igual, depois da reação em cadeia
    let result = game.play(a, b);
    assert!(result.valid);
    assert!(result.ended);
    let Some(Event::GameOver { board, .. }) = result.events.last() else {
        panic!("o último evento deveria ser o fim de jogo");
    };
    assert_eq!(*board, game.board);

    let frozen = game.board;
    assert!(!game.shuffle(false), "no Clássico o jogador não embaralha");
    assert!(!game.shuffle(true), "e nada embaralha depois do fim");
    assert!(!game.play(a, b).valid, "partida terminada não aceita jogadas");
    assert_eq!(game.board, frozen);

    game.force_no_moves = false;
    game.restart();
    assert!(!game.ended);
}

#[test]
fn zen_recovers_a_move_without_resetting_progress_or_other_specials() {
    let mut game = fixture();
    game.score = 1990;
    place(&mut game, 3, 1, Some(0), TileKind::Normal);
    place(&mut game, 3, 2, Some(0), TileKind::Burst);
    place(&mut game, 3, 3, Some(1), TileKind::Normal);
    place(&mut game, 2, 3, Some(0), TileKind::Normal);
    game.force_no_moves = true;
    let result = game.play(at(2, 3), at(3, 3));
    game.force_no_moves = false;

    assert!(!result.ended);
    assert!(result.rescued);
    let [.., Event::Fall { board: previous, .. }, Event::Rescue { index, .. }] = result.events.as_slice()
    else {
        panic!("os dois últimos eventos deveriam ser a queda e o resgate");
    };
    assert_eq!(game.tile(*index).map(|tile| tile.kind), Some(TileKind::Spectrum));
    assert!(game.has_move());
    assert!(game.score > 1990 && game.level >= 2);
    for i in (0..SIZE * SIZE).filter(|i| i != index) {
        assert_eq!(game.tile(i).map(|tile| tile.id), Some(id_at(previous, i)), "casa {i}");
    }
}

#[test]
fn a_long_sequence_keeps_falling_ids_events_and_board_consistent() {
    let mut game = Game::with_random(lcg_random(7654321));
    for turn in 0..100 {
        let (a, b) = match_move(&game);
        let result = game.play(a, b);
        assert!(result.valid && result.earned > 0, "jogada {turn}");
        for (i, event) in result.events.iter().enumerate() {
            let Event::Fall { board, falls, .. } = event else { continue };
            let Some(Event::Clear { board: before, cells, creations, .. }) =
                i.checked_sub(1).map(|previous| &result.events[previous])
            else {
                panic!("toda queda vem depois de uma limpeza (jogada {turn})");
            };
            assert_eq!(falls.len(), 64);
            let mut sources = HashSet::new();
            for (target, &source) in falls.iter().enumerate() {
                if source < 0 {
                    continue; // pedra nova, vinda de cima
                }
                let source = source as usize;
                assert!(!cells.contains(&source), "pedra limpa nunca cai");
                assert!(sources.insert(source), "cada pedra tem um só destino");
                let expected = creations
                    .iter()
                    .find(|creation| creation.index == source)
                    .map_or_else(|| id_at(before, source), |creation| creation.tile.id);
                assert_eq!(expected, id_at(board, target));
            }
        }
        assert!(game.matches().cells.is_empty());
        assert!(game.has_move());
        let ids: HashSet<u32> = game.board.iter().flatten().map(|tile| tile.id).collect();
        assert_eq!(ids.len(), 64);
    }
}

#[test]
fn developer_undo_restores_the_complete_state_before_the_last_valid_move() {
    let mut game = Game::with_random(lcg_random(7654321));
    let (a, b) = match_move(&game);
    let original = state(&game);
    assert!(!game.can_undo_last_move());
    assert!(game.play(a, b).valid);
    assert!(game.can_undo_last_move());
    assert!(!game.play(0, 63).valid, "jogada inválida não entra no histórico");
    assert!(game.undo_last_move());
    assert_eq!(state(&game), original);
    assert!(game.cheats_used, "desfazer conta como recurso de desenvolvedor");
    assert!(!game.can_undo_last_move());
    assert!(!game.undo_last_move());
}

#[test]
fn undo_history_keeps_at_most_the_last_50_states() {
    let mut game = fixture();
    for score in 0..60 {
        game.score = score;
        let previous = game.snapshot();
        game.remember_state(previous);
    }
    assert_eq!(game.undo_history.len(), 50);
    assert_eq!(game.undo_history[0].score, 10, "os 10 mais antigos foram descartados");
    assert!(game.undo_last_move());
    assert_eq!(game.score, 59);
}

#[test]
fn invalid_play_leaves_the_undo_history_untouched() {
    let mut game = fixture(); // este tabuleiro não tem nenhuma jogada
    assert!(!game.play(at(0, 0), at(0, 1)).valid, "troca que não combina nada");
    assert!(!game.play(0, 63).valid, "casas que não são vizinhas");
    assert!(!game.play(0, 99).valid, "casa fora do tabuleiro");
    assert!(game.undo_history.is_empty());

    let mut game = Game::with_random(lcg_random(7654321));
    let (a, b) = match_move(&game);
    assert!(game.play(a, b).valid);
    assert_eq!(game.undo_history.len(), 1);
    assert!(!game.play(0, 63).valid);
    assert_eq!(game.undo_history.len(), 1);
}

#[test]
fn new_game_clears_developer_undo() {
    let mut game = Game::with_random(lcg_random(7654321));
    let (a, b) = match_move(&game);
    assert!(game.play(a, b).valid);
    assert!(game.can_undo_last_move());
    game.new_game(Mode::Zen);
    assert!(!game.can_undo_last_move());

    let (a, b) = match_move(&game);
    assert!(game.play(a, b).valid);
    game.restart();
    assert!(!game.can_undo_last_move());
}
