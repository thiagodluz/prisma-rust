//! Tipos que descrevem o que acontece numa jogada (criações, fusões, eventos).
//!
//! A tela usa estes eventos, na ordem, para animar a jogada passo a passo.

use crate::tile::{Board, Tile, TileKind};

/// Uma pedra especial que vai nascer depois de uma combinação.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Creation {
    /// Casa (0 a 63) onde a pedra especial vai aparecer.
    pub index: usize,
    pub kind: TileKind,
    pub color: u8,
}

/// Fusão de 3 ou mais pedras especiais coloridas alinhadas: explode uma área 5×5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Supernova {
    /// Casa central da explosão.
    pub index: usize,
    pub color: u8,
    /// As pedras especiais que foram fundidas.
    pub sources: Vec<usize>,
}

/// Uma pedra especial que nasceu numa limpeza, já com a pedra pronta (com id).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Created {
    pub index: usize,
    pub kind: TileKind,
    pub color: u8,
    pub tile: Tile,
}

/// Algo que "disparou" durante uma limpeza: uma pedra especial ou uma Supernova.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Activation {
    /// Um Pulso, Raio ou Espectro que foi ativado.
    Special { index: usize, kind: TileKind, id: u32 },
    Supernova(Supernova),
}

impl Activation {
    /// Nome curto, igual ao do JS: `"burst"`, `"cross"`, `"spectrum"` ou `"supernova"`.
    pub fn name(&self) -> &'static str {
        match self {
            Activation::Special { kind: TileKind::Burst, .. } => "burst",
            Activation::Special { kind: TileKind::Cross, .. } => "cross",
            Activation::Special { kind: TileKind::Spectrum, .. } => "spectrum",
            Activation::Special { kind: TileKind::Normal, .. } => "normal",
            Activation::Supernova(_) => "supernova",
        }
    }
}

/// Um passo do que aconteceu numa jogada. Uma jogada devolve uma lista deles, em ordem.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Pedras limpas (e especiais que nasceram). `board` é o tabuleiro ANTES da limpeza.
    Clear {
        board: Board,
        cells: Vec<usize>,
        activated: Vec<Activation>,
        creations: Vec<Created>,
        chain: u32,
        base: u64,
        creation_bonus: u64,
        combo_multiplier: f64,
        earned: u64,
    },
    /// As pedras caem e pedras novas entram por cima.
    /// `falls[destino]` é a casa de origem da pedra; um número negativo é uma pedra nova vinda de cima.
    Fall { board: Board, falls: Vec<i32>, chain: u32 },
    /// O tabuleiro foi embaralhado.
    Shuffle { before: Board, board: Board, chain: u32 },
    /// A partida (Clássico) terminou: não há mais jogadas.
    GameOver { board: Board, chain: u32 },
    /// O jogo (Zen) criou um Espectro na casa `index` para destravar o tabuleiro.
    Rescue { board: Board, index: usize, chain: u32 },
}

impl Event {
    /// Nome curto, igual ao do JS.
    pub fn name(&self) -> &'static str {
        match self {
            Event::Clear { .. } => "clear",
            Event::Fall { .. } => "fall",
            Event::Shuffle { .. } => "shuffle",
            Event::GameOver { .. } => "gameover",
            Event::Rescue { .. } => "rescue",
        }
    }
}

/// O resultado de uma tentativa de jogada.
#[derive(Clone, Debug, PartialEq)]
pub struct MoveResult {
    /// `false` se a troca não era permitida (nada mudou).
    pub valid: bool,
    pub events: Vec<Event>,
    /// Pontos ganhos na jogada toda.
    pub earned: u64,
    /// Quantas etapas de reação em cadeia houve.
    pub chain: u32,
    pub rescued: bool,
    pub shuffled: bool,
    pub ended: bool,
    pub levels_gained: u32,
}

impl MoveResult {
    /// Resultado de uma jogada inválida: nada aconteceu.
    pub fn invalid() -> MoveResult {
        MoveResult {
            valid: false,
            events: Vec::new(),
            earned: 0,
            chain: 0,
            rescued: false,
            shuffled: false,
            ended: false,
            levels_gained: 0,
        }
    }
}
