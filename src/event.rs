//! Tipos que descrevem o que acontece numa jogada (criações, fusões, eventos).

use crate::tile::TileKind;

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
