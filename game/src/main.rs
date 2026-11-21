//! `prisma_game`: a tela do Prisma em Rust (janela no PC, app no Android).
//!
//! As regras do jogo ficam no pacote `prisma_engine`; aqui só se desenha e se lê toque.

mod assets;
mod contador;
mod layout;
mod render;
mod tom;

use macroquad::prelude::*;
use prisma_engine::{lcg_random, Game, Tile, TileKind};

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

/// Lê o pedido de "foto" nos argumentos do programa: `--screenshot arquivo.png`.
/// Se faltar o nome do arquivo, o pedido é ignorado e o jogo abre normalmente.
fn screenshot_path(args: &[String]) -> Option<String> {
    let posicao = args.iter().position(|argumento| argumento == "--screenshot")?;
    args.get(posicao + 1).cloned()
}

/// A partida de vitrine do M0a: motor com semente fixa (sempre o mesmo tabuleiro), com
/// as primeiras linhas trocadas para mostrar todas as imagens: sete Pulsos, sete Raios e
/// um Espectro. Só serve para ver as imagens; ninguém joga com ela.
fn demo_game() -> Game {
    let mut jogo = Game::with_random(lcg_random(1));
    // Os ids das pedras novas só precisam ser diferentes entre si.
    let mut proximo_id = 1000;
    let mut nova = |kind: TileKind, color: Option<u8>| {
        proximo_id += 1;
        Tile { id: proximo_id, color, kind }
    };
    for cor in 0..7u8 {
        jogo.board[0][usize::from(cor)] = nova(TileKind::Burst, Some(cor));
        jogo.board[1][usize::from(cor)] = nova(TileKind::Cross, Some(cor));
    }
    jogo.board[2][0] = nova(TileKind::Spectrum, None);
    jogo
}

// `#[macroquad::main(...)]` transforma esta função no ponto de partida do programa.
// `async` é como o Macroquad "espera o próximo quadro" sem travar o celular: cada volta
// do `loop` desenha um quadro e o `.await` devolve o controle até o quadro seguinte.
#[macroquad::main(window_conf)]
async fn main() {
    let argumentos: Vec<String> = std::env::args().collect();
    let foto = screenshot_path(&argumentos);
    let jogo = demo_game();
    let sprites = render::Sprites::load();

    let mut quadro = 0u32;
    loop {
        render::draw_frame(&jogo.board, &sprites);
        // No modo "foto": desenha dois quadros (o primeiro "aquece" a tela), grava o
        // segundo num PNG e fecha o programa.
        if let Some(caminho) = &foto {
            if quadro == 1 {
                get_screen_data().export_png(caminho);
                std::process::exit(0);
            }
        }
        quadro += 1;
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{all_sprites, sprite_for};
    use std::collections::HashSet;

    /// Transforma uma lista de textos fixos em `Vec<String>`, como `std::env::args` entrega.
    fn args(itens: &[&str]) -> Vec<String> {
        itens.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn foto_sem_caminho_e_ignorada() {
        assert_eq!(screenshot_path(&args(&[])), None);
        assert_eq!(screenshot_path(&args(&["--screenshot"])), None);
        assert_eq!(screenshot_path(&args(&["--screenshot", "a.png"])), Some("a.png".to_string()));
        assert_eq!(
            screenshot_path(&args(&["x", "--screenshot", "b.png"])),
            Some("b.png".to_string())
        );
    }

    #[test]
    fn a_vitrine_mostra_as_22_imagens() {
        let jogo = demo_game();
        let mostradas: HashSet<_> = jogo
            .board
            .iter()
            .flatten()
            .filter_map(sprite_for)
            .collect();
        for sprite in all_sprites() {
            assert!(mostradas.contains(&sprite), "a vitrine não mostra {sprite:?}");
        }
    }
}
