//! Tarefa 3: criação de pedras especiais, Supernova e dica.
mod common;

use common::{at, fixture, place};
use prisma_engine::{Game, TileKind, adjacent, find_matches, lcg_random, swap_on};

#[test]
fn creations_pick_burst_for_four_spectrum_for_five_and_cross_for_an_intersection() {
    // Três iguais não criam nada.
    let mut game = fixture();
    for c in 0..3 {
        place(&mut game, 3, c, Some(0), TileKind::Normal);
    }
    let found = game.matches();
    assert!(game.creations(&found, &[], &[]).is_empty());

    // Quatro na linha 3: nasce um Pulso, na maior casa (ou na casa preferida).
    let mut game = fixture();
    for c in 0..4 {
        place(&mut game, 3, c, Some(0), TileKind::Normal);
    }
    let found = game.matches();
    let created = game.creations(&found, &[], &[]);
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].kind, TileKind::Burst);
    assert_eq!(created[0].color, 0);
    assert_eq!(created[0].index, at(3, 3));
    let preferred = game.creations(&found, &[at(3, 1)], &[]);
    assert_eq!(preferred[0].index, at(3, 1));

    // Cinco na linha 3: nasce um Espectro.
    let mut game = fixture();
    for c in 0..5 {
        place(&mut game, 3, c, Some(0), TileKind::Normal);
    }
    let found = game.matches();
    let created = game.creations(&found, &[], &[]);
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].kind, TileKind::Spectrum);
    assert_eq!(created[0].index, at(3, 4));

    // Forma em L (duas sequências que se cruzam em (3,4)): nasce um Raio.
    let mut game = fixture();
    for (r, c) in [(3, 4), (3, 5), (3, 6), (4, 4), (5, 4)] {
        place(&mut game, r, c, Some(4), TileKind::Normal);
    }
    let found = game.matches();
    let created = game.creations(&found, &[], &[]);
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].kind, TileKind::Cross);
    assert_eq!(created[0].color, 4);
    assert_eq!(created[0].index, at(5, 4));
    assert_eq!(game.creations(&found, &[at(3, 4)], &[])[0].index, at(3, 4));

    // Se a combinação já foi fundida numa Supernova, ela não cria especial nenhum.
    assert!(game.creations(&found, &[], &[at(3, 4)]).is_empty());
}

#[test]
fn long_and_crossing_runs_fuse_each_special_at_most_once() {
    // Quatro Raios em linha: uma só Supernova, centrada na segunda pedra do grupo.
    let mut long = fixture();
    for c in 1..=4 {
        place(&mut long, 3, c, Some(2), TileKind::Cross);
    }
    let found = long.matches();
    let nova = long.supernovas(&found);
    assert_eq!(nova.len(), 1);
    assert_eq!(nova[0].index, at(3, 2));
    assert_eq!(nova[0].sources.len(), 4);
    assert!(long.creations(&found, &[], &nova[0].sources).is_empty());

    // Cruz de cinco Pulsos: cada pedra especial entra em uma fusão só.
    let mut crossing = fixture();
    for (r, c) in [(3, 2), (3, 3), (3, 4), (2, 3), (4, 3)] {
        place(&mut crossing, r, c, Some(2), TileKind::Burst);
    }
    let found = crossing.matches();
    let sources: Vec<usize> = crossing
        .supernovas(&found)
        .iter()
        .flat_map(|effect| effect.sources.iter().copied())
        .collect();
    assert_eq!(sources.len(), 3);
    let mut unique = sources.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), sources.len());
}

#[test]
fn specials_separated_by_a_normal_gem_never_form_a_supernova() {
    let mut game = fixture();
    for c in 1..=5 {
        let kind = if c % 2 == 1 { TileKind::Burst } else { TileKind::Normal };
        place(&mut game, 3, c, Some(2), kind);
    }
    let found = game.matches();
    assert!(game.supernovas(&found).is_empty());
}

#[test]
fn hint_returns_a_legal_move_and_prioritizes_a_pair_of_spectra() {
    let mut game = fixture();
    place(&mut game, 1, 1, None, TileKind::Spectrum);
    place(&mut game, 1, 2, None, TileKind::Spectrum);
    assert_eq!(game.hint(), Some((at(1, 1), at(1, 2))));
    game.ended = true;
    assert_eq!(game.hint(), None);
}

#[test]
fn hint_on_fresh_boards_is_always_a_legal_move() {
    for seed in 1..=20 {
        let mut game = Game::with_random(lcg_random(seed));
        let before = game.board;
        let (a, b) = game.hint().expect("tabuleiro novo sempre tem dica");
        assert_eq!(game.board, before, "a dica não pode deixar o tabuleiro trocado");
        assert!(adjacent(a, b));
        let mut swapped = game.board;
        swap_on(&mut swapped, a, b);
        assert!(!find_matches(&swapped).cells.is_empty(), "semente {seed}: a troca precisa combinar");
    }
}
