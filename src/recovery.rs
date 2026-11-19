//! Recuperação do jogo: embaralhar as pedras e resgatar um tabuleiro sem jogadas.
//!
//! Em Rust, um tipo pode ter vários blocos `impl`, até em arquivos diferentes.
//! Aqui acrescentamos mais métodos ao `Game`, separados do resto por assunto.

use crate::game::Game;
use crate::matching::find_matches;
use crate::rng::pick;
use crate::tile::{Board, Mode, SIZE, Tile, TileKind};

impl Game {
    /// Embaralha as pedras que já estão no tabuleiro (nenhuma é criada nem perdida).
    ///
    /// Devolve `true` se conseguiu. O resultado nunca tem combinação pronta e sempre tem jogada.
    /// `automatic` indica que o próprio jogo pediu (por exemplo, ao subir de nível);
    /// o Clássico só embaralha nesse caso, nunca por pedido do jogador.
    pub fn shuffle(&mut self, automatic: bool) -> bool {
        if self.ended || (self.mode == Mode::Classic && !automatic) {
            return false;
        }
        // As 64 pedras atuais, em fila (linha por linha).
        let original: Vec<Tile> = self.board.iter().flatten().copied().collect();

        // Primeiro método: embaralhar de verdade (Fisher–Yates), até 128 tentativas.
        for _attempt in 0..128 {
            let mut tiles = original.clone();
            for i in (1..tiles.len()).rev() {
                let j = pick(&mut *self.random, i + 1);
                tiles.swap(i, j);
            }
            if let Some(board) = self.accept_layout(&tiles, &original) {
                return self.apply_shuffle(board, automatic);
            }
        }

        // Segundo método (se a sorte for muito ruim): trocar só duas pedras, testando todos os pares.
        for a in 0..original.len() {
            for b in (a + 1)..original.len() {
                let mut tiles = original.clone();
                tiles.swap(a, b);
                if let Some(board) = self.accept_layout(&tiles, &original) {
                    return self.apply_shuffle(board, automatic);
                }
            }
        }
        false
    }

    /// Uma arrumação é aceita se mudou alguma pedra de lugar, não tem combinação pronta e tem jogada.
    /// Devolve o tabuleiro montado, ou `None` se a arrumação não serve.
    fn accept_layout(&self, tiles: &[Tile], original: &[Tile]) -> Option<Board> {
        let changed = tiles.iter().zip(original).any(|(tile, old)| tile.id != old.id);
        if !changed {
            return None;
        }
        let mut board = [[Tile::BLANK; SIZE]; SIZE];
        for (index, tile) in tiles.iter().enumerate() {
            board[index / SIZE][index % SIZE] = *tile;
        }
        if !find_matches(&board).cells.is_empty() || !self.has_move_on(&board) {
            return None;
        }
        Some(board)
    }

    fn apply_shuffle(&mut self, board: Board, automatic: bool) -> bool {
        self.board = board;
        if !automatic {
            // Um embaralhar pedido pelo jogador não pode ser desfeito junto com jogadas antigas.
            self.undo_history.clear();
        }
        true
    }

    /// Resgata um tabuleiro sem jogadas: coloca um Espectro, que garante uma jogada.
    ///
    /// Escolhe a primeira casa de pedra comum entre as do centro (27, 28, 35, 36) e depois
    /// as demais, em ordem. Devolve o índice da casa escolhida.
    pub fn rescue(&mut self) -> usize {
        let center = [27, 28, 35, 36];
        let found = center
            .into_iter()
            .chain(0..SIZE * SIZE)
            .find(|&cell| self.tile(cell).map(|tile| tile.kind) == Some(TileKind::Normal));
        // Se nenhuma pedra é comum (só com cheats), usa o centro mesmo.
        let target = found.unwrap_or(27);
        let spectrum = self.gem(None, TileKind::Spectrum);
        self.set(target, spectrum);
        target
    }
}
