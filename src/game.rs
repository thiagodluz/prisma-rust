//! O estado do jogo (`Game`) e as consultas básicas ao tabuleiro.

use crate::matching::{Matches, adjacent, find_matches, swap_on};
use crate::rng::{RandomSource, pick, system_random};
use crate::tile::{Board, COLORS, Mode, SIZE, Tile, TileKind, position};

/// Uma "fotografia" do jogo num instante, guardada para poder desfazer a jogada.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub next_id: u32,
    pub board: Board,
    pub score: u64,
    pub level: u32,
    pub level_start_score: u64,
    pub progression_offset: u64,
    pub mode: Mode,
    pub ended: bool,
    pub no_defeat: bool,
}

/// O jogo inteiro: tabuleiro, pontos, nível e modo.
///
/// Todos os campos são `pub` (públicos), como no JS, para o resto do programa (e os testes)
/// poderem ler e mexer neles.
pub struct Game {
    /// Fonte de sorte; pode ser trocada de fora (os testes fazem isso).
    pub random: RandomSource,
    /// Próximo id de pedra a ser usado.
    pub next_id: u32,
    pub board: Board,
    pub score: u64,
    pub level: u32,
    pub level_start_score: u64,
    pub progression_offset: u64,
    pub mode: Mode,
    pub ended: bool,
    /// Fotografias para o "desfazer" (de desenvolvedor).
    pub undo_history: Vec<Snapshot>,
    pub cheats_used: bool,
    pub no_defeat: bool,
    /// Gancho de teste: quando `true`, o jogo age como se nunca houvesse jogadas possíveis.
    pub force_no_moves: bool,
}

impl Game {
    /// Cria um jogo novo com sorte "de verdade" (baseada na hora do sistema).
    pub fn new() -> Game {
        Game::with_random(system_random())
    }

    /// Cria um jogo novo com a fonte de sorte dada (útil para testes repetíveis).
    pub fn with_random(random: RandomSource) -> Game {
        let mut game = Game {
            random,
            next_id: 1,
            board: [[Tile::BLANK; SIZE]; SIZE],
            score: 0,
            level: 1,
            level_start_score: 0,
            progression_offset: 0,
            mode: Mode::Zen,
            ended: false,
            undo_history: Vec::new(),
            cheats_used: false,
            no_defeat: false,
            force_no_moves: false,
        };
        game.new_game(Mode::Zen);
        game
    }

    /// Começa uma partida nova no modo dado. Os números de id continuam de onde estavam.
    pub fn new_game(&mut self, mode: Mode) {
        self.mode = mode;
        self.score = 0;
        self.level = 1;
        self.level_start_score = 0;
        self.progression_offset = 0;
        self.ended = false;
        self.undo_history.clear();
        self.cheats_used = false;
        self.no_defeat = false;
        self.board = self.fresh_board();
    }

    /// Começa uma partida nova no mesmo modo da atual.
    pub fn restart(&mut self) {
        self.new_game(self.mode);
    }

    /// Cria uma pedra nova, com um id novo. O Espectro nunca tem cor.
    pub fn gem(&mut self, color: Option<u8>, kind: TileKind) -> Tile {
        let id = self.next_id;
        self.next_id += 1;
        let color = if kind == TileKind::Spectrum { None } else { color };
        Tile { id, color, kind }
    }

    /// Cria uma pedra comum de cor sorteada.
    pub fn random_gem(&mut self) -> Tile {
        // `&mut *self.random`: pega a função de sorte guardada na caixa e a empresta para `pick`.
        let color = pick(&mut *self.random, COLORS as usize) as u8;
        self.gem(Some(color), TileKind::Normal)
    }

    /// Monta um tabuleiro inicial sem combinações prontas e com pelo menos uma jogada.
    pub(crate) fn fresh_board(&mut self) -> Board {
        for _attempt in 0..1000 {
            let mut board = [[Tile::BLANK; SIZE]; SIZE];
            for r in 0..SIZE {
                for c in 0..SIZE {
                    // Cores que NÃO formariam uma trinca com as duas casas à esquerda ou acima.
                    let choices: Vec<u8> = (0..COLORS)
                        .filter(|&color| {
                            let horizontal = c >= 2
                                && board[r][c - 1].color == Some(color)
                                && board[r][c - 2].color == Some(color);
                            let vertical = r >= 2
                                && board[r - 1][c].color == Some(color)
                                && board[r - 2][c].color == Some(color);
                            !horizontal && !vertical
                        })
                        .collect();
                    let color = choices[pick(&mut *self.random, choices.len())];
                    board[r][c] = self.gem(Some(color), TileKind::Normal);
                }
            }
            if self.has_move_on(&board) {
                return board;
            }
        }
        // `panic!` para o programa com uma mensagem (como o `throw` do JS). Na prática nunca acontece.
        panic!("Não foi possível criar um tabuleiro jogável");
    }

    /// A pedra na casa `index`, ou `None` se o índice está fora do tabuleiro (64 ou mais).
    pub fn tile(&self, index: usize) -> Option<Tile> {
        if index >= SIZE * SIZE {
            return None;
        }
        let (r, c) = position(index);
        Some(self.board[r][c])
    }

    /// Coloca uma pedra na casa `index`. Só para uso interno e dos testes: índice de 64 em diante dá erro.
    pub fn set(&mut self, index: usize, tile: Tile) {
        let (r, c) = position(index);
        self.board[r][c] = tile;
    }

    /// As combinações que existem agora no tabuleiro.
    pub fn matches(&self) -> Matches {
        find_matches(&self.board)
    }

    /// Troca de lugar duas pedras do tabuleiro do jogo.
    pub fn swap(&mut self, a: usize, b: usize) {
        swap_on(&mut self.board, a, b);
    }

    /// Existe alguma jogada possível no tabuleiro atual?
    pub fn has_move(&self) -> bool {
        self.has_move_on(&self.board)
    }

    /// Existe alguma jogada possível neste tabuleiro? (Pode ser um tabuleiro de teste, não o do jogo.)
    pub fn has_move_on(&self, board: &Board) -> bool {
        if self.force_no_moves {
            return false;
        }
        for a in 0..SIZE * SIZE {
            // Cada casa só precisa ser testada com a da direita (a + 1) e a de baixo (a + 8).
            for b in [a + 1, a + SIZE] {
                if !adjacent(a, b) {
                    continue;
                }
                let kind_of = |index: usize| {
                    let (r, c) = position(index);
                    board[r][c].kind
                };
                // Trocar de lugar um Espectro com qualquer pedra sempre vale uma jogada.
                if kind_of(a) == TileKind::Spectrum || kind_of(b) == TileKind::Spectrum {
                    return true;
                }
                let mut copy = *board; // cópia do tabuleiro: o original não é alterado
                swap_on(&mut copy, a, b);
                if !find_matches(&copy).cells.is_empty() {
                    return true;
                }
            }
        }
        false
    }
}
