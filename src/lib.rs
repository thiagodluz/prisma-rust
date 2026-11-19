//! `prisma_engine`: o "cérebro" do jogo Prisma, escrito em Rust.
//!
//! Esta biblioteca guarda as regras do jogo. Ela não desenha nada na tela, não toca som
//! e não lê arquivos: isso fica para quem usar a biblioteca.

pub mod event;
pub mod game;
pub mod hint;
pub mod matching;
pub mod recovery;
pub mod rng;
pub mod specials;
pub mod tile;

pub use event::{Creation, Supernova};
pub use game::{Game, Snapshot};
pub use matching::{Matches, Run, adjacent, find_matches, swap_on};
pub use rng::{RandomSource, lcg_random, pick, system_random};
pub use tile::{Board, COLORS, Mode, SIZE, Tile, TileKind, position};
