//! Pedras especiais: quais nascem de uma combinação e quando elas se fundem em Supernova.

use crate::event::{Creation, Supernova};
use crate::game::Game;
use crate::matching::{Matches, Run};
use crate::tile::TileKind;

/// Um conjunto de sequências da mesma cor que se tocam (por exemplo, um "L" ou um "T").
struct Component<'a> {
    color: u8,
    cells: Vec<usize>,
    runs: Vec<&'a Run>,
}

impl Game {
    /// Diz quais pedras especiais nascem de uma combinação.
    ///
    /// - `preferred`: casas que preferimos para o nascimento (a casa para onde o jogador moveu);
    /// - `fused`: casas já usadas numa Supernova; combinações que as tocam não criam nada.
    ///
    /// `&[usize]` é uma "fatia": um empréstimo de uma lista de números, de qualquer tamanho.
    pub fn creations(&self, matches: &Matches, preferred: &[usize], fused: &[usize]) -> Vec<Creation> {
        // Passo 1: juntar sequências da mesma cor que dividem alguma casa num só conjunto.
        let mut components: Vec<Component> = Vec::new();
        for run in &matches.runs {
            let mut cells = run.cells.clone();
            let mut runs: Vec<&Run> = vec![run];
            let mut kept = Vec::new();
            for component in components {
                let touches = component.color == run.color
                    && run.cells.iter().any(|cell| component.cells.contains(cell));
                if touches {
                    for &cell in &component.cells {
                        if !cells.contains(&cell) {
                            cells.push(cell);
                        }
                    }
                    runs.extend(component.runs);
                } else {
                    kept.push(component);
                }
            }
            kept.push(Component { color: run.color, cells, runs });
            components = kept;
        }

        // Passo 2: cada conjunto vira, no máximo, uma pedra especial.
        components
            .into_iter()
            .filter_map(|component| {
                if component.cells.iter().any(|cell| fused.contains(cell)) {
                    return None;
                }
                let has_five = component.runs.iter().any(|run| run.length >= 5);
                let has_four = component.runs.iter().any(|run| run.length >= 4);
                let total: usize = component.runs.iter().map(|run| run.length).sum();
                let crossing = component.runs.len() > 1 && component.cells.len() < total;
                let kind = if has_five {
                    TileKind::Spectrum
                } else if crossing {
                    TileKind::Cross
                } else if has_four {
                    TileKind::Burst
                } else {
                    return None;
                };
                // Só uma pedra comum pode ser trocada por uma especial.
                let candidates: Vec<usize> = component
                    .cells
                    .iter()
                    .copied()
                    .filter(|&cell| self.tile(cell).map(|tile| tile.kind) == Some(TileKind::Normal))
                    .collect();
                let highest = *candidates.iter().max()?;
                let index = preferred
                    .iter()
                    .copied()
                    .find(|cell| candidates.contains(cell))
                    .unwrap_or(highest);
                Some(Creation { index, kind, color: component.color })
            })
            .collect()
    }

    /// Procura fusões: 3 ou mais pedras especiais coloridas (Pulso ou Raio) seguidas, na mesma combinação.
    /// Cada pedra especial entra em uma fusão só.
    pub fn supernovas(&self, matches: &Matches) -> Vec<Supernova> {
        let mut effects = Vec::new();
        let mut consumed: Vec<usize> = Vec::new();
        for run in &matches.runs {
            let mut group: Vec<usize> = Vec::new();
            for &cell in &run.cells {
                let kind = self.tile(cell).map(|tile| tile.kind);
                let colored_special = matches!(kind, Some(TileKind::Burst | TileKind::Cross));
                if colored_special && !consumed.contains(&cell) {
                    group.push(cell);
                } else {
                    close_group(&mut group, run.color, &mut effects, &mut consumed);
                }
            }
            close_group(&mut group, run.color, &mut effects, &mut consumed);
        }
        effects
    }
}

/// Fecha um grupo de pedras especiais seguidas: se tem 3 ou mais, vira uma Supernova no meio do grupo.
fn close_group(group: &mut Vec<usize>, color: u8, effects: &mut Vec<Supernova>, consumed: &mut Vec<usize>) {
    if group.len() >= 3 {
        effects.push(Supernova {
            index: group[(group.len() - 1) / 2],
            color,
            sources: group.clone(),
        });
        consumed.extend(group.iter().copied());
    }
    group.clear();
}
