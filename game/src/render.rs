//! Desenho: pega o estado do motor e pinta na tela com as imagens das pedras.
//!
//! Este módulo precisa de uma janela aberta (o Macroquad guarda as imagens na placa de
//! vídeo), por isso não tem testes automáticos: quem confere é a "foto" (`--screenshot`).

use std::collections::HashMap;

use macroquad::prelude::*;
use prisma_engine::{Board, SIZE};

use crate::assets::{all_sprites, png_bytes, sprite_for, Sprite, SPRITE_RATIO};
use crate::layout::{board_rect, cell_rect};

/// Cor de fundo da tela (a mesma do Prisma em JS).
const FUNDO: Color = Color::from_hex(0x101229);
/// Cor do painel atrás das pedras (a do tabuleiro ilustrado do JS).
const PAINEL: Color = Color::from_hex(0x0d142c);

/// Todas as imagens já carregadas na placa de vídeo, uma `Texture2D` por `Sprite`.
pub struct Sprites {
    texturas: HashMap<Sprite, Texture2D>,
}

impl Sprites {
    /// Carrega as 22 imagens a partir dos PNG embutidos no programa.
    /// Só funciona depois que a janela abriu (ou seja, dentro do `main` do Macroquad).
    pub fn load() -> Sprites {
        let texturas = all_sprites()
            .into_iter()
            // `None`: a biblioteca descobre sozinha que o arquivo é um PNG.
            .map(|sprite| (sprite, Texture2D::from_file_with_format(png_bytes(sprite), None)))
            .collect();
        Sprites { texturas }
    }
}

/// Desenha um quadro inteiro: fundo, painel do tabuleiro e as pedras.
///
/// O tamanho da tela é lido a cada quadro; assim o tabuleiro acompanha a janela quando ela
/// muda de tamanho (ou o celular gira).
pub fn draw_frame(board: &Board, sprites: &Sprites) {
    clear_background(FUNDO);

    let tabuleiro = board_rect(screen_width(), screen_height());
    draw_rectangle(tabuleiro.x, tabuleiro.y, tabuleiro.w, tabuleiro.h, PAINEL);

    for linha in 0..SIZE {
        for coluna in 0..SIZE {
            // Pedra sem imagem (cor ausente ou inválida): a casa fica vazia, sem erro.
            let Some(sprite) = sprite_for(&board[linha][coluna]) else {
                continue;
            };
            let Some(textura) = sprites.texturas.get(&sprite) else {
                continue;
            };
            let casa = cell_rect(tabuleiro, linha, coluna);
            // O PNG é maior que a casa (tem margem); desenhamos ele centrado na casa.
            let lado = casa.w * SPRITE_RATIO;
            draw_texture_ex(
                textura,
                casa.x + (casa.w - lado) / 2.0,
                casa.y + (casa.h - lado) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(lado, lado)),
                    ..Default::default()
                },
            );
        }
    }
}
