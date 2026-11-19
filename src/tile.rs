//! Tipos básicos do jogo: as pedras, os modos de jogo e o tabuleiro.

/// Número de linhas e de colunas do tabuleiro (8×8).
///
/// `const` é um valor fixo, conhecido na hora de compilar. `usize` é o tipo de
/// número usado para posições e tamanhos em Rust.
pub const SIZE: usize = 8;

/// Quantidade de cores diferentes. As cores são os números de 0 a 6.
pub const COLORS: u8 = 7;

/// O tipo de uma pedra.
///
/// Um `enum` é "uma coisa que é UMA destas opções". Cada pedra é exatamente uma delas.
///
/// O `#[derive(...)]` pede ao compilador para criar sozinho algumas habilidades:
/// - `Clone` e `Copy`: a pedra pode ser copiada de graça (como um número);
/// - `Debug`: pode ser mostrada nas mensagens dos testes;
/// - `PartialEq` e `Eq`: duas pedras podem ser comparadas com `==`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
    /// Pedra comum.
    Normal,
    /// Pulso: quando limpo, explode as 8 casas ao redor.
    Burst,
    /// Raio: quando limpo, apaga a linha e a coluna inteiras.
    Cross,
    /// Espectro: não tem cor e apaga todas as pedras de uma cor.
    Spectrum,
}

/// Uma pedra do tabuleiro.
///
/// Uma `struct` junta vários valores com nome em um só (aqui, três campos).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    /// Número único da pedra. Serve para a tela saber qual pedra é qual quando elas caem.
    pub id: u32,
    /// `Option<u8>` significa "um número OU nada". Só o Espectro não tem cor (`None`).
    pub color: Option<u8>,
    pub kind: TileKind,
}

impl Tile {
    /// Pedra "em branco", usada só para encher o tabuleiro antes de preenchê-lo de verdade.
    pub(crate) const BLANK: Tile = Tile {
        id: 0,
        color: None,
        kind: TileKind::Normal,
    };
}

/// O modo de jogo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Sem derrota: quando acabam as jogadas, o jogo cria uma nova possibilidade.
    #[default]
    Zen,
    /// A partida termina quando não restam jogadas.
    Classic,
}

/// O tabuleiro: 8 linhas, cada uma com 8 pedras.
///
/// `[T; N]` é um array de tamanho fixo. Como `Tile` é `Copy`, o tabuleiro inteiro
/// também é: copiar o tabuleiro é só escrever `let copia = tabuleiro;`.
pub type Board = [[Tile; SIZE]; SIZE];

/// Converte um índice (0 a 63) em (linha, coluna). O índice é `linha * 8 + coluna`.
pub fn position(index: usize) -> (usize, usize) {
    (index / SIZE, index % SIZE)
}
