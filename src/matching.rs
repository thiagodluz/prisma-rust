//! Regras "puras" do tabuleiro: achar combinações e vizinhos.
//!
//! "Puras" quer dizer que só olham o tabuleiro que recebem; não dependem do estado do jogo.

use crate::tile::{Board, SIZE, Tile, position};

/// Uma sequência de 3 ou mais pedras da mesma cor, em linha ou em coluna.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    /// Índices (0 a 63) das casas da sequência, em ordem.
    pub cells: Vec<usize>,
    pub color: u8,
    pub length: usize,
}

/// Todas as combinações do tabuleiro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Matches {
    /// Todas as casas que combinam, sem repetir (uma casa em duas sequências conta uma vez),
    /// na ordem em que foram encontradas.
    pub cells: Vec<usize>,
    pub runs: Vec<Run>,
}

/// Procura as combinações: primeiro varre as linhas, depois as colunas.
///
/// `&Board` é um "empréstimo": a função só olha o tabuleiro, sem tomá-lo para si.
pub fn find_matches(board: &Board) -> Matches {
    let mut runs = Vec::new();
    for r in 0..SIZE {
        let line: Vec<usize> = (0..SIZE).map(|c| r * SIZE + c).collect();
        scan(board, &line, &mut runs);
    }
    for c in 0..SIZE {
        let line: Vec<usize> = (0..SIZE).map(|r| r * SIZE + c).collect();
        scan(board, &line, &mut runs);
    }
    let mut cells: Vec<usize> = Vec::new();
    for run in &runs {
        for &cell in &run.cells {
            if !cells.contains(&cell) {
                cells.push(cell);
            }
        }
    }
    Matches { cells, runs }
}

/// Anda ao longo de uma linha (ou coluna) e guarda as sequências de 3 ou mais.
///
/// `&mut Vec<Run>` é um empréstimo que permite alterar a lista de fora (acrescentar sequências).
fn scan(board: &Board, points: &[usize], runs: &mut Vec<Run>) {
    let mut start = 0;
    while start < points.len() {
        let mut end = start + 1;
        // `if let Some(x) = ...` só entra no bloco se houver um valor. O Espectro não tem cor e não combina.
        if let Some(color) = tile_at(board, points[start]).color {
            while end < points.len() && tile_at(board, points[end]).color == Some(color) {
                end += 1;
            }
            if end - start >= 3 {
                runs.push(Run {
                    cells: points[start..end].to_vec(),
                    color,
                    length: end - start,
                });
            }
        }
        start = end;
    }
}

fn tile_at(board: &Board, index: usize) -> Tile {
    let (r, c) = position(index);
    board[r][c]
}

/// Duas casas são vizinhas se estão lado a lado ou uma em cima da outra (não vale diagonal).
/// Índices de 64 para cima estão fora do tabuleiro e nunca são vizinhos.
pub fn adjacent(a: usize, b: usize) -> bool {
    if a >= SIZE * SIZE || b >= SIZE * SIZE {
        return false;
    }
    let (ar, ac) = position(a);
    let (br, bc) = position(b);
    // `abs_diff` é a distância entre dois números sem sinal.
    ar.abs_diff(br) + ac.abs_diff(bc) == 1
}

/// Troca de lugar as pedras das casas `a` e `b`.
///
/// `&mut Board` é um empréstimo que permite alterar o tabuleiro.
pub fn swap_on(board: &mut Board, a: usize, b: usize) {
    let (ar, ac) = position(a);
    let (br, bc) = position(b);
    let first = board[ar][ac];
    board[ar][ac] = board[br][bc];
    board[br][bc] = first;
}
