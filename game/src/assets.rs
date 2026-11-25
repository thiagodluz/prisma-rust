//! As imagens das pedras: quais existem, qual serve para cada pedra do motor, e os bytes
//! dos arquivos PNG (embutidos dentro do próprio programa).

use prisma_engine::{Tile, TileKind, COLORS};

/// O logo do topo (PNG 192×192 com os cantos já arredondados), embutido no programa.
pub const MARCA_PNG: &[u8] = include_bytes!("../assets/marca.png");

/// A foto de fundo do jogo (JPEG, 940×1672), embutida no programa.
pub const FUNDO_JPG: &[u8] = include_bytes!("../assets/prisma-bg.jpg");

/// Tamanho da casa do tabuleiro dentro de cada PNG, em pixels.
pub const CELL_PX: u32 = 128;
/// Tamanho do PNG: a casa mais uma margem de 16 px de cada lado (para a pedra não ser cortada).
pub const SPRITE_PX: u32 = 160;
/// Quantas vezes o PNG é maior que a casa (160 / 128 = 1,25): ao desenhar, o PNG ocupa
/// `casa × SPRITE_RATIO`, centrado na casa.
pub const SPRITE_RATIO: f32 = SPRITE_PX as f32 / CELL_PX as f32;

/// Uma das 22 imagens. O número é a cor (0 a 6) e só existe de 0 a 6: quem cria `Sprite` é
/// `sprite_for` (que confere) e `all_sprites`.
///
/// O `derive` pede ao compilador as habilidades de copiar, imprimir, comparar e usar como
/// chave de tabela (`Hash`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sprite {
    /// Pedra comum.
    Gem(u8),
    /// Pulso (especial que limpa em volta).
    Burst(u8),
    /// Raio (especial que limpa linha e coluna).
    Cross(u8),
    /// Espectro (especial sem cor).
    Spectrum,
}

/// A imagem que representa uma pedra do motor, ou `None` se a pedra não tem imagem
/// (pedra comum, Pulso ou Raio sem cor ou com cor fora de 0 a 6, como num save velho ou
/// estragado). Quem desenha simplesmente pula essa casa.
pub fn sprite_for(tile: &Tile) -> Option<Sprite> {
    // Cor válida = existe e é menor que `COLORS` (7).
    let cor = tile.color.filter(|&cor| cor < COLORS);
    match tile.kind {
        TileKind::Normal => cor.map(Sprite::Gem),
        TileKind::Burst => cor.map(Sprite::Burst),
        TileKind::Cross => cor.map(Sprite::Cross),
        // O Espectro não tem cor: se vier alguma, é ignorada.
        TileKind::Spectrum => Some(Sprite::Spectrum),
    }
}

/// As 22 imagens, sem repetir: 7 comuns, 7 Pulsos, 7 Raios e o Espectro.
pub fn all_sprites() -> Vec<Sprite> {
    let mut todas = Vec::with_capacity(22);
    for cor in 0..COLORS {
        todas.push(Sprite::Gem(cor));
        todas.push(Sprite::Burst(cor));
        todas.push(Sprite::Cross(cor));
    }
    todas.push(Sprite::Spectrum);
    todas
}

// `include_bytes!` copia o arquivo para dentro do programa na hora de compilar. Assim o jogo
// não precisa achar as imagens em nenhuma pasta (no PC ou no celular). O caminho é relativo
// a este arquivo.
const GEMS: [&[u8]; 7] = [
    include_bytes!("../assets/gem-0.png"),
    include_bytes!("../assets/gem-1.png"),
    include_bytes!("../assets/gem-2.png"),
    include_bytes!("../assets/gem-3.png"),
    include_bytes!("../assets/gem-4.png"),
    include_bytes!("../assets/gem-5.png"),
    include_bytes!("../assets/gem-6.png"),
];
const BURSTS: [&[u8]; 7] = [
    include_bytes!("../assets/burst-0.png"),
    include_bytes!("../assets/burst-1.png"),
    include_bytes!("../assets/burst-2.png"),
    include_bytes!("../assets/burst-3.png"),
    include_bytes!("../assets/burst-4.png"),
    include_bytes!("../assets/burst-5.png"),
    include_bytes!("../assets/burst-6.png"),
];
const CROSSES: [&[u8]; 7] = [
    include_bytes!("../assets/cross-0.png"),
    include_bytes!("../assets/cross-1.png"),
    include_bytes!("../assets/cross-2.png"),
    include_bytes!("../assets/cross-3.png"),
    include_bytes!("../assets/cross-4.png"),
    include_bytes!("../assets/cross-5.png"),
    include_bytes!("../assets/cross-6.png"),
];
const SPECTRUM: &[u8] = include_bytes!("../assets/spectrum.png");

/// Os bytes do arquivo PNG de uma imagem.
///
/// Só entra em pânico se alguém montar um `Sprite` com cor de 7 em diante à mão; os
/// `Sprite` que vêm de `sprite_for` e `all_sprites` sempre servem.
pub fn png_bytes(sprite: Sprite) -> &'static [u8] {
    match sprite {
        Sprite::Gem(cor) => GEMS[usize::from(cor)],
        Sprite::Burst(cor) => BURSTS[usize::from(cor)],
        Sprite::Cross(cor) => CROSSES[usize::from(cor)],
        Sprite::Spectrum => SPECTRUM,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::texture::Image;
    use prisma_engine::{Tile, TileKind};
    use std::collections::HashSet;

    /// Monta uma pedra de teste (o número de identificação não importa aqui).
    fn pedra(kind: TileKind, color: Option<u8>) -> Tile {
        Tile { id: 1, color, kind }
    }

    /// Abre o PNG embutido como imagem. `None` deixa a biblioteca descobrir o formato
    /// sozinha pelos primeiros bytes do arquivo. Não precisa de janela.
    fn abre(sprite: Sprite) -> Image {
        Image::from_file_with_format(png_bytes(sprite), None)
            .unwrap_or_else(|erro| panic!("o PNG de {sprite:?} não abriu: {erro:?}"))
    }

    /// Transparência (0 a 255) do pixel `(x, y)`: o quarto byte de cada pixel RGBA.
    fn alfa(imagem: &Image, x: usize, y: usize) -> u8 {
        imagem.bytes[4 * (y * imagem.width as usize + x) + 3]
    }

    /// Caixa que envolve os pixels visíveis (alfa de 128 em diante, isto é, mais de 50%):
    /// `(x_min, y_min, x_max, y_max)`, ou `None` se a imagem for toda transparente.
    fn caixa_visivel(imagem: &Image) -> Option<(usize, usize, usize, usize)> {
        let (largura, altura) = (imagem.width as usize, imagem.height as usize);
        let mut caixa: Option<(usize, usize, usize, usize)> = None;
        for y in 0..altura {
            for x in 0..largura {
                if alfa(imagem, x, y) >= 128 {
                    caixa = Some(match caixa {
                        None => (x, y, x, y),
                        Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                    });
                }
            }
        }
        caixa
    }

    #[test]
    fn cada_tipo_e_cor_escolhe_a_imagem_certa() {
        assert_eq!(sprite_for(&pedra(TileKind::Normal, Some(3))), Some(Sprite::Gem(3)));
        assert_eq!(sprite_for(&pedra(TileKind::Burst, Some(3))), Some(Sprite::Burst(3)));
        assert_eq!(sprite_for(&pedra(TileKind::Cross, Some(3))), Some(Sprite::Cross(3)));
        assert_eq!(sprite_for(&pedra(TileKind::Spectrum, None)), Some(Sprite::Spectrum));
        // O Espectro não tem cor: se vier uma, ela é ignorada.
        assert_eq!(sprite_for(&pedra(TileKind::Spectrum, Some(2))), Some(Sprite::Spectrum));
    }

    #[test]
    fn pedra_sem_cor_valida_nao_tem_imagem() {
        for tile in [
            pedra(TileKind::Normal, None),
            pedra(TileKind::Normal, Some(7)),
            pedra(TileKind::Normal, Some(255)),
            pedra(TileKind::Burst, None),
            pedra(TileKind::Cross, Some(9)),
        ] {
            assert_eq!(sprite_for(&tile), None, "{tile:?} não deveria ter imagem");
        }
    }

    #[test]
    fn existem_22_imagens_diferentes() {
        let todas = all_sprites();
        assert_eq!(todas.len(), 22);
        let distintas: HashSet<Sprite> = todas.iter().copied().collect();
        assert_eq!(distintas.len(), 22);
    }

    #[test]
    fn todo_png_decodifica_com_o_tamanho_combinado() {
        assert_eq!(SPRITE_RATIO, 1.25);
        for sprite in all_sprites() {
            let imagem = abre(sprite);
            assert_eq!(
                (imagem.width as u32, imagem.height as u32),
                (SPRITE_PX, SPRITE_PX),
                "tamanho de {sprite:?}"
            );
        }
    }

    #[test]
    fn todo_png_esta_centrado_pelo_pixel_visivel() {
        // O centro da tela de 160×160 fica entre os pixels 79 e 80, isto é, em 79,5.
        let meio = (SPRITE_PX as f32 - 1.0) / 2.0;
        for sprite in all_sprites() {
            let imagem = abre(sprite);
            let (x0, y0, x1, y1) =
                caixa_visivel(&imagem).unwrap_or_else(|| panic!("{sprite:?} está todo transparente"));
            let centro_x = (x0 + x1) as f32 / 2.0;
            let centro_y = (y0 + y1) as f32 / 2.0;
            assert!(
                (centro_x - meio).abs() <= 2.0 && (centro_y - meio).abs() <= 2.0,
                "{sprite:?} fora do centro: ({centro_x}, {centro_y}) em vez de perto de {meio}"
            );
        }
    }

    #[test]
    fn todo_png_tem_corpo_visivel() {
        let minimo = (CELL_PX * CELL_PX) as f32 * 0.15;
        for sprite in all_sprites() {
            let imagem = abre(sprite);
            let visiveis = (0..SPRITE_PX as usize)
                .flat_map(|y| (0..SPRITE_PX as usize).map(move |x| (x, y)))
                .filter(|&(x, y)| alfa(&imagem, x, y) >= 128)
                .count();
            assert!(
                visiveis as f32 >= minimo,
                "{sprite:?} tem só {visiveis} pixels visíveis (mínimo {minimo})"
            );
        }
    }

    #[test]
    fn as_sete_pedras_de_cada_familia_sao_diferentes() {
        type Faz = fn(u8) -> Sprite;
        let familias: [(&str, Faz); 3] = [
            ("Gem", Sprite::Gem),
            ("Burst", Sprite::Burst),
            ("Cross", Sprite::Cross),
        ];
        for (nome, faz) in familias {
            let imagens: HashSet<Vec<u8>> = (0..7).map(|i| abre(faz(i)).bytes).collect();
            assert_eq!(imagens.len(), 7, "a família {nome} tem imagens repetidas (atlas fatiado errado?)");
        }
    }
}
