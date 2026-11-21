//! O que muda de um sistema para outro: achar a pasta de dados do app e vibrar.
//!
//! No Android essas duas coisas dependem de perguntar ao sistema (por JNI, a "ponte" entre
//! o Rust e o Java do Android). No PC a pasta vem das variáveis de ambiente e a vibração é
//! só de mentira. O resto do jogo chama `plataforma::pasta_de_dados()` e
//! `plataforma::vibrar(...)` sem saber qual das duas versões está ligada.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
pub use android::{pasta_de_dados, vibrar};

#[cfg(not(target_os = "android"))]
mod pc;
#[cfg(not(target_os = "android"))]
pub use pc::{pasta_de_dados, vibrar};
