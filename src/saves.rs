//! Salvar e restaurar partidas.
//!
//! `SavedGame` é o save já "lido" (a leitura do arquivo em si fica para a Fase 2).
//! `restore` é rigoroso: valida tudo e é **tudo ou nada**. Se qualquer coisa estiver errada,
//! devolve `false` e o jogo atual não muda em nada.

use std::collections::HashSet;

use crate::game::Game;
use crate::progression::{goal_for, level_start_at};
use crate::rng::RandomSource;
use crate::tile::{COLORS, Mode, SIZE, Tile, TileKind};

/// O maior número "seguro" do JS (2^53 − 1). Os saves do JS nunca passam disso, e aqui também não.
const MAX_SAFE: i64 = 9_007_199_254_740_991;

/// Maior id de pedra aceito num save. O contador de ids é `u32`; este limite deixa
/// uns 2 bilhões de ids livres para as pedras novas, então o contador nunca estoura.
const MAX_SAVED_ID: u32 = 2_000_000_000;

/// O modo gravado no save. `Endless` é o nome antigo do modo Clássico.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SavedMode {
    #[default]
    Zen,
    Classic,
    Endless,
}

/// Uma pedra gravada no save.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SavedTile {
    /// Save antigo: só o número da cor (0 a 6). Vira uma pedra comum com id novo.
    Legacy(i64),
    /// Save de hoje: a pedra completa. O Espectro não tem cor.
    Full { id: i64, kind: TileKind, color: Option<i64> },
}

/// Um jogo gravado. Os números são `i64` (com sinal) de propósito: um arquivo estragado
/// pode ter qualquer valor, e é o `restore` que decide se serve.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SavedGame {
    pub mode: SavedMode,
    pub score: i64,
    pub board: Vec<Vec<SavedTile>>,
    /// 2, 3 ou 4 (saves sem versão, ou de versão desconhecida, têm o nível recalculado).
    pub progression_version: Option<i64>,
    pub level: Option<i64>,
    pub level_start_score: Option<i64>,
    pub progression_offset: Option<i64>,
    pub cheats_used: bool,
    pub no_defeat: bool,
    pub ended: bool,
}

/// Uma pedra do save depois de conferida.
enum Parsed {
    Legacy(u8),
    Tile(Tile),
}

/// O nível e os pontos de início de nível que o jogo vai usar depois de restaurar.
struct Progress {
    level: u32,
    level_start_score: u64,
    progression_offset: u64,
}

/// Uma cor válida (0 a 6), ou `None`.
fn color_in_range(color: i64) -> Option<u8> {
    u8::try_from(color).ok().filter(|&color| color < COLORS)
}

/// Confere uma pedra do save. `None` = pedra inválida.
fn parse_tile(tile: &SavedTile) -> Option<Parsed> {
    match *tile {
        SavedTile::Legacy(color) => Some(Parsed::Legacy(color_in_range(color)?)),
        SavedTile::Full { id, kind, color } => {
            let id = u32::try_from(id).ok().filter(|&id| id > 0 && id <= MAX_SAVED_ID)?;
            let color = match (kind, color) {
                (TileKind::Spectrum, None) => None,
                (TileKind::Spectrum, Some(_)) | (_, None) => return None,
                (_, Some(color)) => Some(color_in_range(color)?),
            };
            Some(Parsed::Tile(Tile { id, color, kind }))
        }
    }
}

/// O número só vale se estiver dentro dos limites "seguros" do JS.
fn safe(number: i64) -> Option<i64> {
    // Não usamos `abs()`: `i64::MIN.abs()` estoura. Comparar com os limites nunca estoura.
    (-MAX_SAFE..=MAX_SAFE).contains(&number).then_some(number)
}

/// Confere a progressão do save (versões 2, 3 e 4) ou a recalcula (sem versão).
///
/// Toda conta usa `checked_*` ou `try_from`: se estourar, devolve `None` (= save recusado).
/// `score` já foi conferido (entre 0 e `MAX_SAFE`).
fn saved_progress(saved: &SavedGame, score: i64) -> Option<Progress> {
    match saved.progression_version {
        Some(version @ 2..=4) => {
            // Cada versão usava um teto diferente para a meta de pontos.
            let cap: u64 = match version {
                2 => 4650,
                3 => 5000,
                _ => 6000,
            };
            let level = safe(saved.level?)?;
            if level < 1 {
                return None;
            }
            let start = safe(saved.level_start_score?)?;
            let level_number = u64::try_from(level).ok()?;
            let base = i64::try_from(level_start_at(level_number, cap)?).ok()?;
            let offset = if version == 2 {
                None
            } else {
                let offset = safe(saved.progression_offset?)?;
                if offset < 0 {
                    return None;
                }
                Some(offset)
            };
            // O início do nível tem que bater com a conta (descontado o deslocamento).
            let expected = match offset {
                None => base,
                Some(offset) => base.checked_sub(offset)?,
            };
            if start != expected || start > score {
                return None;
            }
            // O progresso dentro do nível tem que ser menor que a meta do nível.
            let progress = score.checked_sub(start)?;
            if progress >= i64::try_from(goal_for(level_number, cap)).ok()? {
                return None;
            }
            // Os saves 2 e 3 são convertidos para o teto de hoje (6000): o deslocamento
            // guarda a diferença, e assim nível e pontos não se perdem.
            let new_offset = match offset {
                Some(offset) if version == 4 => offset,
                _ => i64::try_from(level_start_at(level_number, 6000)?).ok()?.checked_sub(start)?,
            };
            Some(Progress {
                level: u32::try_from(level).ok()?,
                level_start_score: u64::try_from(start).ok()?,
                progression_offset: u64::try_from(new_offset).ok()?,
            })
        }
        _ => {
            // Sem versão: acha o maior nível cujo início não passa dos pontos (busca binária).
            let score = u64::try_from(score).ok()?;
            let mut low: u64 = 1;
            let mut high: u64 = 1 + score / 1800;
            while low < high {
                let middle = low + (high - low + 1) / 2; // o "meio" arredondado para cima
                if level_start_at(middle, 6000).is_some_and(|start| start <= score) {
                    low = middle;
                } else {
                    high = middle - 1;
                }
            }
            Some(Progress {
                level: u32::try_from(low).ok()?,
                level_start_score: level_start_at(low, 6000)?,
                progression_offset: 0,
            })
        }
    }
}

impl Game {
    /// Restaura uma partida salva. Devolve `true` se o save serviu.
    ///
    /// **Tudo ou nada:** se o save for inválido, devolve `false` e o jogo fica exatamente como estava.
    /// Saves antigos são migrados; um tabuleiro com combinações prontas é reembaralhado;
    /// um tabuleiro sem jogadas termina a partida (Clássico) ou ganha um Espectro (Zen).
    pub fn restore(&mut self, saved: &SavedGame) -> bool {
        // 1. Pontos: de 0 até o limite seguro.
        if !(0..=MAX_SAFE).contains(&saved.score) {
            return false;
        }
        let Ok(score) = u64::try_from(saved.score) else {
            return false;
        };
        // 2. O tabuleiro tem que ser 8×8.
        if saved.board.len() != SIZE || saved.board.iter().any(|row| row.len() != SIZE) {
            return false;
        }
        // 3. Cada pedra tem que ser válida.
        let mut tiles: Vec<Parsed> = Vec::with_capacity(SIZE * SIZE);
        for tile in saved.board.iter().flatten() {
            let Some(parsed) = parse_tile(tile) else {
                return false;
            };
            tiles.push(parsed);
        }
        // 4. Nenhum id pode se repetir.
        let ids: Vec<u32> = tiles
            .iter()
            .filter_map(|parsed| match parsed {
                Parsed::Tile(tile) => Some(tile.id),
                Parsed::Legacy(_) => None,
            })
            .collect();
        let unique: HashSet<u32> = ids.iter().copied().collect();
        if unique.len() != ids.len() {
            return false;
        }
        // 5. A progressão (nível e pontos de início de nível).
        let Some(progress) = saved_progress(saved, saved.score) else {
            return false;
        };

        // A partir daqui nada pode falhar, então a fonte de sorte emprestada sempre é devolvida.
        // `std::mem::replace` tira a fonte do jogo e deixa uma de enfeite no lugar.
        let placeholder: RandomSource = Box::new(|| 0.0);
        let random = std::mem::replace(&mut self.random, placeholder);
        let first_free_id = ids.iter().max().map_or(1, |highest| highest + 1);
        let mut candidate = Game {
            random,
            next_id: self.next_id.max(first_free_id),
            board: [[Tile::BLANK; SIZE]; SIZE],
            score,
            level: progress.level,
            level_start_score: progress.level_start_score,
            progression_offset: progress.progression_offset,
            mode: match saved.mode {
                SavedMode::Zen => Mode::Zen,
                SavedMode::Classic | SavedMode::Endless => Mode::Classic,
            },
            ended: false,
            undo_history: Vec::new(),
            cheats_used: saved.cheats_used || saved.no_defeat,
            no_defeat: saved.no_defeat,
            // O gancho de teste não vale para o candidato (no JS também não valia).
            force_no_moves: false,
        };
        // As pedras de save antigo ganham ids novos, na ordem do tabuleiro.
        for (index, parsed) in tiles.into_iter().enumerate() {
            let tile = match parsed {
                Parsed::Legacy(color) => candidate.gem(Some(color), TileKind::Normal),
                Parsed::Tile(tile) => tile,
            };
            candidate.board[index / SIZE][index % SIZE] = tile;
        }

        // Um save com combinações prontas é reembaralhado (as pedras especiais são mantidas).
        if !candidate.matches().cells.is_empty() && !candidate.shuffle(true) {
            candidate.board = candidate.fresh_board();
        }
        // Sem jogadas: o Clássico termina; o Zen ganha um Espectro.
        let classic_with_defeat = candidate.mode == Mode::Classic && !candidate.no_defeat;
        if !candidate.has_move() {
            if classic_with_defeat {
                candidate.ended = true;
            } else {
                candidate.rescue();
            }
        } else if classic_with_defeat && saved.ended {
            candidate.ended = true;
        }

        // Tudo certo: devolve a fonte de sorte e copia o resultado para o jogo de verdade.
        self.random = candidate.random;
        self.next_id = candidate.next_id;
        self.board = candidate.board;
        self.mode = candidate.mode;
        self.score = candidate.score;
        self.level = candidate.level;
        self.level_start_score = candidate.level_start_score;
        self.progression_offset = candidate.progression_offset;
        self.ended = candidate.ended;
        self.cheats_used = candidate.cheats_used;
        self.no_defeat = candidate.no_defeat;
        self.undo_history.clear();
        true
    }

    /// O jogo atual no formato de save de hoje (versão 4, pedras completas).
    pub fn to_saved(&self) -> SavedGame {
        let to_i64 = |number: u64| i64::try_from(number).unwrap_or(i64::MAX);
        SavedGame {
            mode: match self.mode {
                Mode::Zen => SavedMode::Zen,
                Mode::Classic => SavedMode::Classic,
            },
            score: to_i64(self.score),
            board: self
                .board
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
                .collect(),
            progression_version: Some(4),
            level: Some(i64::from(self.level)),
            level_start_score: Some(to_i64(self.level_start_score)),
            progression_offset: Some(to_i64(self.progression_offset)),
            cheats_used: self.cheats_used,
            no_defeat: self.no_defeat,
            ended: self.ended,
        }
    }
}
