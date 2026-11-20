//! `prisma_engine`: o "cérebro" do jogo Prisma, escrito em Rust.
//!
//! Esta biblioteca guarda as regras do jogo. Ela não desenha nada na tela, não toca som
//! e não lê arquivos: isso fica para quem usar a biblioteca.

pub mod event;
pub mod game;
pub mod hint;
pub mod matching;
pub mod progression;
pub mod recovery;
pub mod resolve;
pub mod saves;
pub mod rng;
pub mod specials;
pub mod tile;

pub use event::{Activation, Created, Creation, Event, MoveResult, Supernova};
pub use game::{Game, Snapshot};
pub use matching::{Matches, Run, adjacent, find_matches, swap_on};
pub use progression::level_goal;
pub use rng::{RandomSource, lcg_random, pick, system_random};
pub use saves::{SavedGame, SavedMode, SavedTile};
pub use tile::{Board, COLORS, Mode, SIZE, Tile, TileKind, position};
