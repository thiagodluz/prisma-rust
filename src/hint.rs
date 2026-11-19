//! A dica: qual troca vale mais a pena fazer agora.

use crate::game::Game;
use crate::matching::adjacent;
use crate::tile::{SIZE, TileKind};

impl Game {
    /// Procura a melhor jogada e devolve as duas casas a trocar, ou `None` se o jogo acabou.
    ///
    /// Precisa de `&mut self` porque, para avaliar cada troca, ela troca as pedras no próprio
    /// tabuleiro e desfaz logo depois (o tabuleiro termina exatamente como começou).
    pub fn hint(&mut self) -> Option<(usize, usize)> {
        if self.ended {
            return None;
        }
        let mut best: Option<(usize, usize, usize)> = None; // (casa a, casa b, valor)
        for a in 0..SIZE * SIZE {
            for b in [a + 1, a + SIZE] {
                if !adjacent(a, b) {
                    continue;
                }
                let (Some(first), Some(second)) = (self.tile(a), self.tile(b)) else {
                    continue;
                };
                let spectra = [first, second]
                    .iter()
                    .filter(|tile| tile.kind == TileKind::Spectrum)
                    .count();
                let value = if spectra == 2 {
                    100
                } else if spectra == 1 {
                    // Espectro com uma pedra: limpa todas as pedras daquela cor.
                    let color = if first.kind == TileKind::Spectrum { second.color } else { first.color };
                    8 + self.board.iter().flatten().filter(|tile| tile.color == color).count()
                } else {
                    self.swap(a, b);
                    let found = self.matches();
                    let mut value = 0;
                    if !found.cells.is_empty() {
                        let novas = self.supernovas(&found);
                        let fused: Vec<usize> = novas.iter().flat_map(|nova| nova.sources.iter().copied()).collect();
                        let created: usize = self
                            .creations(&found, &[b, a], &fused)
                            .iter()
                            .map(|creation| match creation.kind {
                                TileKind::Spectrum => 10,
                                TileKind::Cross => 7,
                                _ => 5,
                            })
                            .sum();
                        value = found.cells.len() + novas.len() * 20 + created;
                    }
                    self.swap(a, b);
                    value
                };
                // Só troca de favorita se o valor for estritamente maior (empate: fica a primeira).
                if value > 0 && best.is_none_or(|(_, _, top)| value > top) {
                    best = Some((a, b, value));
                }
            }
        }
        best.map(|(a, b, _)| (a, b))
    }
}
