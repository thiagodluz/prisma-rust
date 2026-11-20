//! Tarefa 8: conferência com o `engine.js` ORIGINAL.
//!
//! Os arquivos em `tests/golden/` foram gerados rodando o JS (`tests/golden/generate.mjs`).
//! Aqui o Rust joga as mesmas partidas, com a mesma fonte de sorte, e o texto de cada rodada
//! (jogada, pontos, nível, próximo id, nomes dos eventos e o tabuleiro inteiro) tem que ser
//! idêntico. Se divergir, o teste aponta a primeira rodada e a primeira linha diferente.

use prisma_engine::{Game, Mode, MoveResult, TileKind, lcg_random};

/// A letra de cada tipo de pedra no arquivo de referência.
fn kind_letter(kind: TileKind) -> char {
    match kind {
        TileKind::Normal => 'n',
        TileKind::Burst => 'b',
        TileKind::Cross => 'c',
        TileKind::Spectrum => 's',
    }
}

/// O texto de uma rodada, no mesmo formato do gerador: uma linha de resumo e 8 do tabuleiro.
fn turn_text(turn: usize, a: usize, b: usize, result: &MoveResult, game: &Game) -> String {
    let events: Vec<&str> = result.events.iter().map(|event| event.name()).collect();
    let mut lines = vec![format!(
        "turn {turn} {a} {b} earned {} chain {} score {} level {} next_id {} events {}",
        result.earned,
        result.chain,
        game.score,
        game.level,
        game.next_id,
        events.join(",")
    )];
    for row in &game.board {
        let tokens: Vec<String> = row
            .iter()
            .map(|tile| {
                let color = tile.color.map_or("-".to_string(), |color| color.to_string());
                format!("{}:{}:{}", tile.id, color, kind_letter(tile.kind))
            })
            .collect();
        lines.push(tokens.join(" "));
    }
    lines.join("\n")
}

/// Joga a partida inteira no Rust e compara, rodada a rodada, com o texto de referência do JS.
fn replay(classic: bool, seed: u32, max_turns: usize, golden: &str) {
    // Cada rodada ocupa 9 linhas: o resumo e as 8 do tabuleiro.
    let lines: Vec<&str> = golden.lines().collect();
    assert_eq!(lines.len() % 9, 0, "o arquivo de referência está truncado");
    let blocks: Vec<&[&str]> = lines.chunks(9).collect();

    let mut game = Game::with_random(lcg_random(seed));
    if classic {
        game.new_game(Mode::Classic);
    }
    let mut played = 0;
    for turn in 1..=max_turns {
        // A jogada vem da dica, então esta conferência também prova que o `hint` é igual ao do JS.
        let Some((a, b)) = game.hint() else { break };
        let result = game.play(a, b);
        assert!(result.valid, "rodada {turn}: o Rust recusou a jogada {a},{b} da dica");
        let Some(expected) = blocks.get(turn - 1) else {
            panic!("rodada {turn}: o Rust ainda tem jogada, mas o JS já tinha parado");
        };
        let actual = turn_text(turn, a, b, &result, &game);
        for (index, (js, rust)) in expected.iter().zip(actual.lines()).enumerate() {
            let where_ = if index == 0 { "resumo".to_string() } else { format!("tabuleiro, linha {}", index - 1) };
            assert_eq!(*js, rust, "primeira diferença: rodada {turn} ({where_})\n  JS:   {js}\n  Rust: {rust}");
        }
        played = turn;
        if game.ended {
            break;
        }
    }
    assert_eq!(played, blocks.len(), "o Rust e o JS jogaram quantidades diferentes de rodadas");
}

#[test]
fn rust_matches_the_original_js_engine_zen_seed_7654321() {
    replay(false, 7654321, 100, include_str!("golden/zen-7654321.txt"));
}

#[test]
fn rust_matches_the_original_js_engine_classic_seed_42() {
    // Esta partida termina (gameover) na rodada 278, então também confere o fim de jogo.
    replay(true, 42, 300, include_str!("golden/classic-42.txt"));
}

#[test]
fn rust_matches_the_original_js_engine_zen_seed_1_including_rescue() {
    replay(false, 1, 150, include_str!("golden/zen-1.txt"));
}
