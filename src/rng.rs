//! Fonte de sorte (números aleatórios) do jogo.
//!
//! O jogo não sorteia por conta própria: ele recebe uma "fonte de sorte" de fora.
//! Assim os testes podem usar uma fonte repetível e obter sempre o mesmo jogo.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Uma fonte de sorte é uma função que, a cada chamada, devolve um número de 0 (inclusive) a 1 (exclusive).
///
/// - `dyn FnMut() -> f64` é "qualquer função que pode mudar o próprio estado e devolve um decimal";
/// - `Box<...>` guarda essa função no "heap" (memória dinâmica), para o tamanho dela não importar.
pub type RandomSource = Box<dyn FnMut() -> f64>;

/// Fonte de sorte "de verdade", para quando se joga: parte da hora do sistema.
///
/// Usa o método xorshift, uma conta simples que embaralha os bits de um número.
pub fn system_random() -> RandomSource {
    // Contador global: dois jogos criados no mesmo instante ainda recebem sementes diferentes.
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let salt = COUNTER.fetch_add(1, Ordering::Relaxed);
    // `wrapping_mul` multiplica e, se estourar o tamanho do número, "dá a volta" em vez de dar erro.
    // `| 1` garante que o estado nunca é zero (o xorshift travaria em zero).
    let mut state = (nanos ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15)).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    // `move` leva `state` para dentro da função, que passa a ser a dona dele.
    Box::new(move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        // Usa os 53 bits de cima, que é o que cabe sem perda em um decimal `f64`.
        (state >> 11) as f64 / (1u64 << 53) as f64
    })
}

/// Fonte de sorte repetível, igual à usada nos testes do `engine.js`.
///
/// Com a mesma `seed` (semente) sai sempre a mesma sequência de números, idêntica à do JS.
pub fn lcg_random(seed: u32) -> RandomSource {
    let mut seed = seed;
    Box::new(move || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed as f64 / 4294967296.0
    })
}

/// Sorteia um índice de `0` a `len - 1` usando a fonte de sorte.
///
/// O resultado fica sempre dentro dos limites, mesmo que a fonte devolva exatamente
/// `1.0` (o que, sem o limite, daria o índice `len`, fora da lista).
pub fn pick(random: &mut dyn FnMut() -> f64, len: usize) -> usize {
    // `as usize` converte o decimal em inteiro; valores negativos ou "não numéricos" viram 0.
    let index = (random() * len as f64).floor() as usize;
    index.min(len.saturating_sub(1))
}
