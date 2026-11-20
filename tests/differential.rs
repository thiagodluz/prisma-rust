//! Conferência DIFERENCIAL com o `engine.js` ORIGINAL: partidas aleatórias, não só a dica.
//!
//! O `cross_check.rs` repete 3 partidas em que só se joga a dica. Aqui o JS (via
//! `tests/golden/generate-diff.mjs`) jogou 20 partidas de 100 ações MISTURADAS: dica, jogadas em
//! casas quaisquer (muitas inválidas), todos os cheats, desfazer, embaralhar e restaurar saves
//! (certos e estragados de propósito). O Rust repete as mesmas ações, com as mesmas fontes de
//! sorte, e cada ação tem que dar o mesmo resumo e o mesmo "carimbo" (hash) do texto completo:
//! todos os eventos, todos os tabuleiros com ids e o estado do jogo depois.
//!
//! Se divergir, o teste mostra a semente, o número da ação, a linha esperada e a obtida, e o texto
//! completo do Rust. Para ver o texto completo do JS da mesma ação:
//!   node tests/golden/generate-diff.mjs <engine.js> --detail <semente> <ação>

use prisma_engine::{
    Activation, Board, Event, Game, Mode, MoveResult, RandomSource, SavedGame, SavedMode,
    SavedTile, Tile, TileKind, lcg_random,
};

// Têm que ser iguais aos do gerador (o cabeçalho do arquivo confere).
const SEEDS: u32 = 20;
const ACTIONS: u32 = 100;

/// FNV-1a de 64 bits: um "carimbo" do texto. Qualquer letra diferente muda o resultado.
fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        // `wrapping_mul` multiplica "dando a volta" quando passa de 64 bits, em vez de dar pânico
        // (que é o que `*` faz em Rust quando estoura). A conta do FNV precisa dessa volta.
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn kind_letter(kind: TileKind) -> char {
    match kind {
        TileKind::Normal => 'n',
        TileKind::Burst => 'b',
        TileKind::Cross => 'c',
        TileKind::Spectrum => 's',
    }
}

fn tile_text(tile: &Tile) -> String {
    let color = tile.color.map_or("-".to_string(), |color| color.to_string());
    format!("{}:{}:{}", tile.id, color, kind_letter(tile.kind))
}

fn board_text(board: &Board) -> String {
    let rows: Vec<String> = board
        .iter()
        .map(|row| row.iter().map(tile_text).collect::<Vec<_>>().join(" "))
        .collect();
    rows.join("|")
}

/// Junta números (ou qualquer coisa com `to_string`) separados por um texto, como o `join` do JS.
fn join<T: ToString>(items: &[T], separator: &str) -> String {
    items.iter().map(T::to_string).collect::<Vec<_>>().join(separator)
}

fn activated_text(item: &Activation) -> String {
    match item {
        Activation::Supernova(nova) => {
            format!("supernova@{}/{}/{}", nova.index, nova.color, join(&nova.sources, "."))
        }
        Activation::Special { index, id, .. } => format!("{}@{index}#{id}", item.name()),
    }
}

fn event_text(event: &Event) -> String {
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
        } => {
            let activated: Vec<String> = activated.iter().map(activated_text).collect();
            let created: Vec<String> = creations
                .iter()
                .map(|c| format!("{}@{}/{}#{}", kind_letter(c.kind), c.index, c.color, c.tile.id))
                .collect();
            format!(
                "clear c={} a={} cr={} ch={chain} b={base} cb={creation_bonus} \
                 m={combo_multiplier} e={earned} B={}",
                join(cells, ","),
                activated.join(","),
                created.join(","),
                board_text(board)
            )
        }
        Event::Fall { falls, chain, .. } => format!("fall {} ch={chain}", join(falls, ",")),
        Event::Shuffle { board, chain, .. } => format!("shuffle ch={chain} B={}", board_text(board)),
        Event::Rescue { index, chain, .. } => format!("rescue {index} ch={chain}"),
        Event::GameOver { chain, .. } => format!("gameover ch={chain}"),
    }
}

fn result_text(result: &MoveResult) -> String {
    if !result.valid {
        return "valid=false".to_string();
    }
    let events: Vec<String> = result.events.iter().map(event_text).collect();
    format!(
        "valid=true earned={} chain={} rescued={} shuffled={} ended={} lg={}\n  {}",
        result.earned,
        result.chain,
        result.rescued,
        result.shuffled,
        result.ended,
        result.levels_gained,
        events.join("\n  ")
    )
}

fn state_text(game: &Game) -> String {
    let mode = match game.mode {
        Mode::Zen => "zen",
        Mode::Classic => "classic",
    };
    format!(
        "S score={} level={} lss={} off={} next={} mode={mode} ended={} cheats={} nd={} undo={} B={}",
        game.score,
        game.level,
        game.level_start_score,
        game.progression_offset,
        game.next_id,
        game.ended,
        game.cheats_used,
        game.no_defeat,
        game.undo_history.len(),
        board_text(&game.board)
    )
}

/// Sorteia as ações. É uma fonte de sorte SEPARADA da do jogo (como no gerador).
struct Picker(RandomSource);

impl Picker {
    /// Um número inteiro de 0 a `k - 1`.
    fn n(&mut self, k: u32) -> u32 {
        // `(self.0)()` chama a função guardada dentro do `Picker`.
        ((self.0)() * f64::from(k)).floor() as u32
    }
}

/// Troca o `id` de uma pedra do save (ignora saves no formato antigo, que não têm id).
fn set_saved_id(tile: &mut SavedTile, new_id: i64) {
    if let SavedTile::Full { id, .. } = tile {
        *id = new_id;
    }
}

fn set_saved_color(tile: &mut SavedTile, new_color: i64) {
    if let SavedTile::Full { color, .. } = tile {
        *color = Some(new_color);
    }
}

/// Faz UMA ação aleatória e devolve o texto dela (a primeira linha é o resumo).
/// A ORDEM dos sorteios `picker.n(...)` é igual à do gerador: não mexer sem mexer nos dois.
fn do_action(game: &mut Game, picker: &mut Picker, seed: u32) -> String {
    let op = picker.n(100);
    if op < 55 {
        // jogar a dica (e recomeçar se não houver jogada possível)
        return match game.hint() {
            None => {
                game.new_game(game.mode);
                "hint none restart".to_string()
            }
            Some((a, b)) => format!("hint {a},{b} {}", result_text(&game.play(a, b))),
        };
    }
    if op < 62 {
        let a = picker.n(64) as usize;
        let b = picker.n(64) as usize;
        return format!("move {a} {b} {}", result_text(&game.play(a, b)));
    }
    if op < 65 {
        let amount = picker.n(3000) + 1;
        return format!("addpts {amount} {}", game.cheat_add_points(i64::from(amount)));
    }
    if op < 67 {
        let level = picker.n(40) + 1;
        return format!("setlvl {level} {}", game.cheat_set_level(i64::from(level)));
    }
    if op < 73 {
        let index = picker.n(64);
        let (kind, name) = [
            (TileKind::Burst, "burst"),
            (TileKind::Cross, "cross"),
            (TileKind::Spectrum, "spectrum"),
        ][picker.n(3) as usize];
        return format!("spawn {index} {name} {}", game.cheat_spawn_special(i64::from(index), kind));
    }
    if op < 75 {
        return format!("cshuffle {}", game.cheat_shuffle());
    }
    if op < 77 {
        let enabled = picker.n(2) == 1;
        return format!("nodefeat {enabled} {}", game.cheat_no_defeat(enabled));
    }
    if op < 79 {
        return format!("explode {}", result_text(&game.cheat_explode()));
    }
    if op < 82 {
        let index = picker.n(64);
        return format!("supernova {index} {}", result_text(&game.cheat_supernova(i64::from(index))));
    }
    if op < 88 {
        return format!("undo {}", game.undo_last_move());
    }
    if op < 91 {
        return format!("shuffle {}", game.shuffle(false));
    }
    if op < 95 {
        // restaurar um save da versão 4, às vezes com uma trinca injetada ou estragado
        let mut saved = game.to_saved();
        if picker.n(2) == 1 {
            let row = picker.n(8) as usize;
            let col = picker.n(6) as usize;
            let color = i64::from(picker.n(7));
            for k in 0..3 {
                if game.board[row][col + k].kind != TileKind::Spectrum {
                    set_saved_color(&mut saved.board[row][col + k], color);
                }
            }
        }
        if picker.n(6) == 0 {
            saved.ended = true; // às vezes o save diz "encerrado" (só vale no Clássico)
        }
        match picker.n(12) {
            0 => {
                // id repetido: a pedra [7][7] ganha o id da [0][0]
                if let SavedTile::Full { id, .. } = saved.board[0][0] {
                    set_saved_id(&mut saved.board[7][7], id);
                }
            }
            1 => saved.level = Some(i64::from(game.level) + 1),
            2 => {
                if game.board[3][3].kind != TileKind::Spectrum {
                    set_saved_color(&mut saved.board[3][3], 7); // cor fora de 0..6
                }
            }
            3 => saved.level_start_score = Some(game.score as i64 + 1),
            4 => saved.progression_offset = Some(-1),
            5 => {
                // pontos abaixo do início do nível
                if game.level_start_score > 0 {
                    saved.score = game.level_start_score as i64 - 1;
                }
            }
            _ => {} // sem estrago
        }
        // Metade dos restores acontece no próprio jogo em andamento (confere o histórico de
        // desfazer e a fonte de sorte); a outra metade, num jogo novo.
        let live = picker.n(2) == 1;
        let mut fresh = Game::with_random(lcg_random(seed + 1000));
        // `&mut *game` empresta o jogo de novo, sem gastá-lo, para escolher qual dos dois usar.
        let target: &mut Game = if live { &mut *game } else { &mut fresh };
        let ok = target.restore(&saved);
        return format!("restore {}{ok}\n{}", if live { "live " } else { "" }, state_text(target));
    }
    // formato antigo (sem versão): só os números das cores
    let tiles: Vec<Vec<SavedTile>> = game
        .board
        .iter()
        .map(|row| {
            row.iter()
                .map(|tile| SavedTile::Legacy(i64::from(tile.color.unwrap_or(0))))
                .collect()
        })
        .collect();
    let mode = if picker.n(2) == 1 { SavedMode::Endless } else { SavedMode::Zen };
    let live = picker.n(2) == 1;
    let mut fresh = Game::with_random(lcg_random(seed + 2000));
    let saved = SavedGame {
        mode,
        score: game.score as i64,
        board: tiles,
        ..Default::default() // sem versão: o nível é recalculado pelos pontos
    };
    let target: &mut Game = if live { &mut *game } else { &mut fresh };
    let ok = target.restore(&saved);
    format!("legacy {}{ok}\n{}", if live { "live " } else { "" }, state_text(target))
}

#[test]
fn random_games_match_the_original_js() {
    // `include_str!` coloca o texto do arquivo dentro do programa na hora de compilar.
    let golden = include_str!("golden/differential.txt");
    let mut expected = golden.lines();
    let header = format!("# sementes {SEEDS} ações {ACTIONS}");
    assert_eq!(expected.next(), Some(header.as_str()), "cabeçalho do arquivo de referência");

    for seed in 0..SEEDS {
        let mut game = Game::with_random(lcg_random(seed));
        if seed % 2 == 1 {
            game.new_game(Mode::Classic);
        }
        let mut picker = Picker(lcg_random(seed * 7 + 3));
        for step in 0..ACTIONS {
            let text = format!("{}\n{}", do_action(&mut game, &mut picker, seed), state_text(&game));
            let summary = text.lines().next().unwrap_or_default();
            let line = format!(
                "{seed} {step} {summary} score={} level={} next_id={} undo={} #{:016x}",
                game.score,
                game.level,
                game.next_id,
                game.undo_history.len(),
                fnv1a(&text)
            );
            let want = expected.next().unwrap_or("(o arquivo acabou antes)");
            if line != want {
                let shown: String = text.chars().take(3000).collect();
                panic!(
                    "semente {seed}, ação {step} diverge do JS\n  JS:   {want}\n  Rust: {line}\n\
                     texto completo do Rust (até 3000 letras):\n{shown}\n\
                     texto do JS: node tests/golden/generate-diff.mjs <engine.js> --detail {seed} {step}"
                );
            }
        }
    }
    assert_eq!(expected.next(), None, "o arquivo de referência tem ações a mais");
}
