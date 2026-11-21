//! `prisma_game`: a tela do Prisma em Rust (janela no PC, app no Android).
//!
//! As regras do jogo ficam no pacote `prisma_engine`; aqui só se desenha e se lê toque.

use macroquad::prelude::*;

/// Configuração da janela. No Android o sistema escolhe o tamanho; no PC usamos 540×960,
/// uma tela de celular em pé.
fn window_conf() -> Conf {
    Conf {
        window_title: "Prisma".to_owned(),
        window_width: 540,
        window_height: 960,
        high_dpi: true,
        window_resizable: true,
        // `..Default::default()` preenche o resto da configuração com os valores padrão.
        ..Default::default()
    }
}

// `#[macroquad::main(...)]` transforma esta função no ponto de partida do programa.
// `async` é como o Macroquad "espera o próximo quadro" sem travar o celular: cada volta
// do `loop` desenha um quadro e o `.await` devolve o controle até o quadro seguinte.
#[macroquad::main(window_conf)]
async fn main() {
    loop {
        clear_background(Color::from_hex(0x101229));
        next_frame().await;
    }
}
