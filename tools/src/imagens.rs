//! Gera as imagens do jogo (`game/assets/*.png` e os ícones do Android) a partir das imagens do
//! Prisma original em JS. É a versão em Rust do antigo `prepare_assets.py`: mesmos números e
//! mesmo passo a passo, então o resultado só difere em arredondamentos da biblioteca de imagens.

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};
use std::path::{Path, PathBuf};

/// Lado de uma casa da grade 3x3 dos atlas.
const CELULA_ATLAS: u32 = 418;
/// Lado do atlas inteiro: 3 casas.
const LADO_ATLAS: u32 = 3 * CELULA_ATLAS;
/// Lado da casa do tabuleiro no jogo, em pixels: as pedras são reduzidas para caber nela.
const CELULA_PX: f64 = 128.0;
/// Lado da tela transparente em que cada pedra é centralizada.
const TELA_PX: u32 = 160;
/// Peça visível com menos pixels que isso é poeira, não é pedra.
const MIN_PEDRA: usize = 2000;
/// (coluna, linha) de cada pedra na grade do atlas.
const POSICAO: [(u32, u32); 7] = [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1), (0, 2)];
/// Tamanho de cada pedra em relação à casa: o mesmo número do jogo em JS.
const ESCALA: [f64; 7] = [0.98, 1.0, 1.08, 0.87, 0.96, 1.08, 0.90];
/// O Espectro aparece a 118% da casa no JS (`background-size: 118%`).
const ESCALA_ESPECTRO: f64 = 1.18;
const ICONES_ANDROID: [(&str, u32); 5] = [
    ("mdpi", 48),
    ("hdpi", 72),
    ("xhdpi", 96),
    ("xxhdpi", 144),
    ("xxxhdpi", 192),
];
const TRANSPARENTE: Rgba<u8> = Rgba([0, 0, 0, 0]);

/// Um pedaço de pixels visíveis (alfa > 0) ligados entre si; `x1` e `y1` não entram na caixa.
struct Peca {
    pixels: usize,
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
}

/// Separa os pixels visíveis em peças (vizinhança de 4 direções). Devolve, para cada pixel, o
/// número da peça (0 = pixel vazio, `n` = `pecas[n - 1]`), e as peças.
fn ligar_pixels(atlas: &RgbaImage) -> (Vec<u32>, Vec<Peca>) {
    let (largura, altura) = (atlas.width() as usize, atlas.height() as usize);
    let visivel = |x: usize, y: usize| atlas.get_pixel(x as u32, y as u32)[3] > 0;
    let mut numeros = vec![0u32; largura * altura];
    let mut pecas: Vec<Peca> = Vec::new();
    let mut pilha: Vec<(usize, usize)> = Vec::new();
    for y_ini in 0..altura {
        for x_ini in 0..largura {
            if numeros[y_ini * largura + x_ini] != 0 || !visivel(x_ini, y_ini) {
                continue;
            }
            let numero = pecas.len() as u32 + 1;
            let mut peca = Peca {
                pixels: 0,
                x0: x_ini as u32,
                x1: x_ini as u32 + 1,
                y0: y_ini as u32,
                y1: y_ini as u32 + 1,
            };
            numeros[y_ini * largura + x_ini] = numero;
            pilha.push((x_ini, y_ini));
            while let Some((x, y)) = pilha.pop() {
                peca.pixels += 1;
                peca.x0 = peca.x0.min(x as u32);
                peca.x1 = peca.x1.max(x as u32 + 1);
                peca.y0 = peca.y0.min(y as u32);
                peca.y1 = peca.y1.max(y as u32 + 1);
                let vizinhos = [
                    (x > 0).then(|| (x - 1, y)),
                    (x + 1 < largura).then(|| (x + 1, y)),
                    (y > 0).then(|| (x, y - 1)),
                    (y + 1 < altura).then(|| (x, y + 1)),
                ];
                for (vx, vy) in vizinhos.into_iter().flatten() {
                    if numeros[vy * largura + vx] == 0 && visivel(vx, vy) {
                        numeros[vy * largura + vx] = numero;
                        pilha.push((vx, vy));
                    }
                }
            }
            pecas.push(peca);
        }
    }
    (numeros, pecas)
}

/// Recorta as sete pedras de um atlas 3x3, na ordem do jogo (índice 0 a 6). Cada recorte é a caixa
/// da pedra pelo contorno dela, com tudo que não é dela apagado (a ponta de uma pedra pode invadir
/// a casa vizinha: um corte reto da grade a cortaria ou levaria pedaço da outra).
pub fn pedras_do_atlas(atlas: &RgbaImage) -> Result<Vec<RgbaImage>, String> {
    let (largura, altura) = atlas.dimensions();
    if (largura, altura) != (LADO_ATLAS, LADO_ATLAS) {
        return Err(format!(
            "o atlas devia ter {LADO_ATLAS}x{LADO_ATLAS} pixels e tem {largura}x{altura}"
        ));
    }
    let (numeros, pecas) = ligar_pixels(atlas);
    let mut achadas: [Option<RgbaImage>; 7] = Default::default();
    for (posicao, peca) in pecas
        .iter()
        .enumerate()
        .filter(|(_, p)| p.pixels >= MIN_PEDRA)
    {
        let numero = posicao as u32 + 1;
        // A casa da pedra é a do centro da caixa dela.
        let coluna = (peca.x0 + peca.x1) / (2 * CELULA_ATLAS);
        let linha = (peca.y0 + peca.y1) / (2 * CELULA_ATLAS);
        let indice = POSICAO
            .iter()
            .position(|&casa| casa == (coluna, linha))
            .ok_or_else(|| {
                format!("achei uma pedra fora da grade, na casa (coluna {coluna}, linha {linha})")
            })?;
        if achadas[indice].is_some() {
            return Err(format!(
                "achei duas pedras na mesma casa da grade (coluna {coluna}, linha {linha})"
            ));
        }
        achadas[indice] = Some(RgbaImage::from_fn(
            peca.x1 - peca.x0,
            peca.y1 - peca.y0,
            |x, y| {
                let (ax, ay) = (peca.x0 + x, peca.y0 + y);
                if numeros[(ay * LADO_ATLAS + ax) as usize] == numero {
                    *atlas.get_pixel(ax, ay)
                } else {
                    TRANSPARENTE
                }
            },
        ));
    }
    let achados: Vec<usize> = achadas
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_some())
        .map(|(i, _)| i)
        .collect();
    if achados.len() != 7 {
        return Err(format!("esperava 7 pedras, achei as de índice {achados:?}"));
    }
    Ok(achadas.into_iter().flatten().collect())
}

/// Muda o tamanho em alfa pré-multiplicado (o "RGBa" do Pillow): sem isso a margem transparente,
/// que é preta, escurece a borda da pedra ao reduzir.
fn redimensionar(imagem: &RgbaImage, largura: u32, altura: u32) -> RgbaImage {
    let mut pre = imagem.clone();
    for pixel in pre.pixels_mut() {
        let alfa = u32::from(pixel[3]);
        for canal in 0..3 {
            pixel[canal] = ((u32::from(pixel[canal]) * alfa + 127) / 255) as u8;
        }
    }
    let mut nova = imageops::resize(&pre, largura, altura, FilterType::Lanczos3);
    for pixel in nova.pixels_mut() {
        let alfa = u32::from(pixel[3]);
        if alfa == 0 || alfa == 255 {
            continue;
        }
        for canal in 0..3 {
            pixel[canal] = ((u32::from(pixel[canal]) * 255 + alfa / 2) / alfa).min(255) as u8;
        }
    }
    nova
}

/// Reduz a pedra por `fator` e a põe numa tela transparente de 160x160, com o centro da parte
/// visível (alfa de 50% em diante) em (80, 80). O tamanho novo e o deslocamento arredondam para o
/// par mais próximo nos empates, como o `round` do Python.
pub fn centralizar(pedra: &RgbaImage, fator: f64) -> Result<RgbaImage, String> {
    let novo = |lado: u32| ((f64::from(lado) * fator).round_ties_even() as u32).max(1);
    let reduzida = redimensionar(pedra, novo(pedra.width()), novo(pedra.height()));
    let (mut x0, mut x1, mut y0, mut y1) = (u32::MAX, 0, u32::MAX, 0);
    for (x, y, pixel) in reduzida.enumerate_pixels() {
        if pixel[3] >= 128 {
            x0 = x0.min(x);
            x1 = x1.max(x + 1);
            y0 = y0.min(y);
            y1 = y1.max(y + 1);
        }
    }
    if x1 == 0 {
        return Err("a pedra não tem nenhum pixel visível (alfa a partir de 128)".to_string());
    }
    let meio = f64::from(TELA_PX) / 2.0;
    let dx = (meio - f64::from(x0 + x1) / 2.0).round_ties_even() as i64;
    let dy = (meio - f64::from(y0 + y1) / 2.0).round_ties_even() as i64;
    let mut tela = RgbaImage::from_pixel(TELA_PX, TELA_PX, TRANSPARENTE);
    imageops::replace(&mut tela, &reduzida, dx, dy);
    Ok(tela)
}

fn abrir(caminho: &Path) -> Result<RgbaImage, String> {
    image::open(caminho)
        .map(|imagem| imagem.to_rgba8())
        .map_err(|erro| format!("{}: {erro}", caminho.display()))
}

/// Gera as 22 imagens de `<jogo>/assets` (21 pedras e o espectro) e os 5 ícones do Android em
/// `<jogo>/android_res`, a partir da pasta do Prisma em JS. Só grava depois de ter tudo pronto: se
/// uma imagem de origem estiver errada, nada muda no jogo. Devolve os arquivos gravados.
pub fn preparar(origem: &Path, jogo: &Path) -> Result<Vec<PathBuf>, String> {
    let assets = jogo.join("assets");
    let mut prontas: Vec<(PathBuf, RgbaImage)> = Vec::new();
    for prefixo in ["gem", "burst", "cross"] {
        let nome = format!("{prefixo}-atlas.webp");
        let atlas = abrir(&origem.join(&nome))?;
        let pedras = pedras_do_atlas(&atlas).map_err(|erro| format!("{nome}: {erro}"))?;
        for (indice, pedra) in pedras.iter().enumerate() {
            let fator = CELULA_PX * ESCALA[indice] / f64::from(CELULA_ATLAS);
            let tela = centralizar(pedra, fator)
                .map_err(|erro| format!("{nome}, pedra {indice}: {erro}"))?;
            prontas.push((assets.join(format!("{prefixo}-{indice}.png")), tela));
        }
    }
    let espectro = abrir(&origem.join("spectrum-gem.webp"))?;
    let fator = CELULA_PX * ESCALA_ESPECTRO / f64::from(espectro.width());
    let tela =
        centralizar(&espectro, fator).map_err(|erro| format!("spectrum-gem.webp: {erro}"))?;
    prontas.push((assets.join("spectrum.png"), tela));
    let icone = abrir(&origem.join("icon-512.png"))?;
    for (densidade, lado) in ICONES_ANDROID {
        let caminho = jogo.join(format!("android_res/mipmap-{densidade}/ic_launcher.png"));
        prontas.push((caminho, redimensionar(&icone, lado, lado)));
    }
    for (caminho, imagem) in &prontas {
        if let Some(pasta) = caminho.parent() {
            std::fs::create_dir_all(pasta)
                .map_err(|erro| format!("{}: {erro}", pasta.display()))?;
        }
        imagem
            .save(caminho)
            .map_err(|erro| format!("{}: {erro}", caminho.display()))?;
    }
    Ok(prontas.into_iter().map(|(caminho, _)| caminho).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Uma pasta temporária que se apaga sozinha quando o teste acaba.
    struct Pasta(PathBuf);

    impl Pasta {
        fn nova(nome: &str) -> Pasta {
            let caminho = std::env::temp_dir()
                .join(format!("prisma_tools_img_{}_{nome}", std::process::id()));
            let _ = std::fs::remove_dir_all(&caminho);
            std::fs::create_dir_all(&caminho).unwrap();
            Pasta(caminho)
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn retangulo(imagem: &mut RgbaImage, x: u32, y: u32, largura: u32, altura: u32, cor: Rgba<u8>) {
        for yy in y..y + altura {
            for xx in x..x + largura {
                imagem.put_pixel(xx, yy, cor);
            }
        }
    }

    /// A cor e o tamanho da pedra `i` nos atlas de mentira (cada uma diferente das outras).
    fn cor_da_pedra(i: usize) -> Rgba<u8> {
        Rgba([20 + 30 * i as u8, 100, 255 - 30 * i as u8, 255])
    }

    fn tamanho_da_pedra(i: usize) -> (u32, u32) {
        (110 + 10 * i as u32, 90 + 15 * i as u32)
    }

    /// Onde a pedra `i` fica no atlas de mentira (um retângulo no meio-alto da casa dela na grade).
    fn origem_da_pedra(i: usize) -> (u32, u32) {
        let (coluna, linha) = POSICAO[i];
        (coluna * 418 + 60, linha * 418 + 60)
    }

    /// Um atlas 1254x1254 de mentira com uma pedra (retângulo) em cada casa pedida.
    fn atlas_de_mentira(indices: &[usize]) -> RgbaImage {
        let mut atlas = RgbaImage::from_pixel(1254, 1254, TRANSPARENTE);
        for &i in indices {
            let (x, y) = origem_da_pedra(i);
            let (l, a) = tamanho_da_pedra(i);
            retangulo(&mut atlas, x, y, l, a, cor_da_pedra(i));
        }
        atlas
    }

    fn caixa_visivel(imagem: &RgbaImage) -> (u32, u32, u32, u32) {
        let (mut x0, mut x1, mut y0, mut y1) = (u32::MAX, 0, u32::MAX, 0);
        for (x, y, p) in imagem.enumerate_pixels() {
            if p[3] >= 128 {
                x0 = x0.min(x);
                x1 = x1.max(x + 1);
                y0 = y0.min(y);
                y1 = y1.max(y + 1);
            }
        }
        (x0, x1, y0, y1)
    }

    #[test]
    fn acha_as_sete_pedras_na_grade() {
        let pedras = pedras_do_atlas(&atlas_de_mentira(&[0, 1, 2, 3, 4, 5, 6])).unwrap();
        assert_eq!(pedras.len(), 7);
        for (i, pedra) in pedras.iter().enumerate() {
            assert_eq!(
                pedra.dimensions(),
                tamanho_da_pedra(i),
                "tamanho da pedra {i}"
            );
            assert_eq!(
                *pedra.get_pixel(5, 5),
                cor_da_pedra(i),
                "cor da pedra {i} (ordem do POSICAO)"
            );
        }
    }

    #[test]
    fn pedras_que_so_se_encostam_na_quina_continuam_separadas() {
        // A pedra 0 (x 300..400, y 300..400) e a pedra 1 (x 400..500, y 200..300) só dividem uma
        // quina: com vizinhança de 8 direções virariam uma peça só e uma pedra sumiria.
        let mut atlas = atlas_de_mentira(&[2, 3, 4, 5, 6]);
        retangulo(&mut atlas, 300, 300, 100, 100, cor_da_pedra(0));
        retangulo(&mut atlas, 400, 200, 100, 100, cor_da_pedra(1));
        let pedras = pedras_do_atlas(&atlas).unwrap();
        assert_eq!(pedras[0].dimensions(), (100, 100));
        assert_eq!(pedras[1].dimensions(), (100, 100));
        assert_eq!(*pedras[0].get_pixel(50, 50), cor_da_pedra(0));
        assert_eq!(*pedras[1].get_pixel(50, 50), cor_da_pedra(1));
    }

    #[test]
    fn ignora_poeira_e_nao_mistura_a_ponta_que_invade_a_casa_vizinha() {
        let mut atlas = atlas_de_mentira(&[0, 1, 2, 3, 4, 5, 6]);
        // Poeira (25 pixels) numa casa vazia da grade: não é pedra e não pode dar erro.
        retangulo(&mut atlas, 600, 900, 5, 5, Rgba([255, 255, 255, 255]));
        // A ponta da pedra 6 sobe até 3 pixels dentro da casa da pedra 3 (linhas 833 a 895),
        // sem encostar nela.
        retangulo(&mut atlas, 100, 833, 20, 63, cor_da_pedra(6));
        let pedras = pedras_do_atlas(&atlas).unwrap();
        // O recorte da pedra 3 é só a pedra 3: nada da ponta da 6 (um corte reto da grade a traria).
        assert_eq!(pedras[3].dimensions(), tamanho_da_pedra(3));
        assert!(pedras[3]
            .pixels()
            .all(|p| *p == cor_da_pedra(3) || *p == TRANSPARENTE));
        assert!(!pedras[3].pixels().any(|p| *p == cor_da_pedra(6)));
        // O recorte da pedra 6 leva a ponta dela: 63 pixels a mais de altura.
        let (l6, a6) = tamanho_da_pedra(6);
        assert_eq!(pedras[6].dimensions(), (l6, a6 + 63));
    }

    #[test]
    fn apaga_do_recorte_o_que_e_de_outra_pedra_e_o_fundo() {
        // Uma pedra em forma de L: o canto vazio do retângulo da caixa tem de ficar transparente (0,0,0,0).
        let mut atlas = atlas_de_mentira(&[0, 1, 2, 3, 4, 5]);
        let (x, y) = origem_da_pedra(6);
        retangulo(&mut atlas, x, y, 40, 100, cor_da_pedra(6));
        retangulo(&mut atlas, x, y + 60, 100, 40, cor_da_pedra(6));
        // Uma sujeirinha solta dentro da caixa do L (no canto vazio): é de outra peça, não da pedra.
        retangulo(&mut atlas, x + 70, y + 10, 5, 5, Rgba([255, 255, 255, 255]));
        let pedra = pedras_do_atlas(&atlas).unwrap().remove(6);
        assert_eq!(pedra.dimensions(), (100, 100));
        assert_eq!(
            *pedra.get_pixel(99, 0),
            TRANSPARENTE,
            "o canto de fora do L fica vazio"
        );
        assert_eq!(
            *pedra.get_pixel(72, 12),
            TRANSPARENTE,
            "a sujeirinha dentro da caixa é apagada"
        );
        assert_eq!(*pedra.get_pixel(0, 0), cor_da_pedra(6));
    }

    #[test]
    fn erra_com_o_indice_das_pedras_achadas_quando_nao_sao_sete() {
        let erro = pedras_do_atlas(&atlas_de_mentira(&[0, 1, 2, 3, 5, 6])).unwrap_err();
        assert!(erro.contains("esperava 7 pedras"), "{erro}");
        assert!(
            erro.contains("[0, 1, 2, 3, 5, 6]"),
            "o erro tem de dizer quais foram achadas: {erro}"
        );
        let erro = pedras_do_atlas(&atlas_de_mentira(&[])).unwrap_err();
        assert!(
            erro.contains("esperava 7 pedras") && erro.contains("[]"),
            "{erro}"
        );
    }

    #[test]
    fn erra_quando_uma_pedra_cai_fora_da_grade_ou_duas_caem_na_mesma_casa() {
        let mut atlas = atlas_de_mentira(&[0, 1, 2, 3, 4, 5, 6]);
        retangulo(&mut atlas, 500, 900, 100, 100, Rgba([9, 9, 9, 255])); // casa (1, 2): vazia na grade
        let erro = pedras_do_atlas(&atlas).unwrap_err();
        assert!(erro.contains("fora da grade"), "{erro}");

        let mut atlas = atlas_de_mentira(&[0, 1, 2, 3, 4, 5, 6]);
        retangulo(&mut atlas, 300, 300, 100, 100, Rgba([9, 9, 9, 255])); // outra pedra na casa (0, 0)
        let erro = pedras_do_atlas(&atlas).unwrap_err();
        assert!(erro.contains("mesma casa"), "{erro}");
    }

    #[test]
    fn erra_quando_o_atlas_nao_tem_o_tamanho_esperado() {
        let erro = pedras_do_atlas(&RgbaImage::new(100, 100)).unwrap_err();
        assert!(
            erro.contains("1254x1254") && erro.contains("100x100"),
            "{erro}"
        );
    }

    #[test]
    fn centralizar_poe_o_centro_visivel_em_80_80() {
        // Pedra assimétrica: opaca só num canto, o resto do recorte quase transparente (alfa 40 < 128).
        let mut pedra = RgbaImage::from_pixel(100, 100, Rgba([200, 50, 50, 40]));
        retangulo(&mut pedra, 10, 0, 30, 30, Rgba([200, 50, 50, 255]));
        for fator in [1.0, 0.5, 0.3] {
            let tela = centralizar(&pedra, fator).unwrap();
            assert_eq!(tela.dimensions(), (160, 160));
            let (x0, x1, y0, y1) = caixa_visivel(&tela);
            let (cx, cy) = (x0 + x1, y0 + y1); // o dobro do centro
            assert!(
                (159..=161).contains(&cx) && (159..=161).contains(&cy),
                "fator {fator}: centro*2 = ({cx},{cy})"
            );
        }
    }

    #[test]
    fn centralizar_usa_o_tamanho_arredondado_e_nunca_menos_de_1_pixel() {
        let pedra = RgbaImage::from_pixel(10, 10, Rgba([255, 255, 255, 255]));
        let tela = centralizar(&pedra, 0.25).unwrap(); // 2,5 px: arredonda para 2 (par mais próximo, como o Python)
        let (x0, x1, y0, y1) = caixa_visivel(&tela);
        assert_eq!((x1 - x0, y1 - y0), (2, 2));
        let tela = centralizar(&pedra, 0.001).unwrap(); // ~0 px: no mínimo 1
        let (x0, x1, y0, y1) = caixa_visivel(&tela);
        assert_eq!((x1 - x0, y1 - y0), (1, 1));
    }

    #[test]
    fn centralizar_recusa_pedra_sem_nenhum_pixel_visivel() {
        let erro = centralizar(&RgbaImage::new(50, 50), 1.0).unwrap_err();
        assert!(erro.contains("nenhum pixel visível"), "{erro}");
    }

    #[test]
    fn reduzir_em_alfa_pre_multiplicado_nao_escurece_a_borda() {
        // Quadrado branco opaco com margem transparente PRETA: sem pré-multiplicar, a borda
        // reduzida seria um cinza escuro semitransparente.
        let mut pedra = RgbaImage::from_pixel(40, 40, TRANSPARENTE);
        retangulo(&mut pedra, 10, 10, 20, 20, Rgba([255, 255, 255, 255]));
        let tela = centralizar(&pedra, 0.5).unwrap();
        let com_alfa: Vec<&Rgba<u8>> = tela.pixels().filter(|p| p[3] > 0).collect();
        assert!(com_alfa.len() > 50, "devia haver pixels visíveis");
        for p in com_alfa {
            assert!(
                p[0] >= 250 && p[1] >= 250 && p[2] >= 250,
                "borda escurecida: {p:?}"
            );
        }
    }

    #[test]
    fn reduzir_mantem_a_cor_de_uma_pedra_lisa() {
        let pedra = RgbaImage::from_pixel(60, 60, Rgba([200, 100, 50, 255]));
        let tela = centralizar(&pedra, 0.5).unwrap();
        assert_eq!(*tela.get_pixel(80, 80), Rgba([200, 100, 50, 255]));
    }

    /// Grava um WebP sem perdas (a biblioteca só sabe escrever esse tipo) com o conteúdo de `imagem`.
    fn gravar_webp(imagem: &RgbaImage, caminho: &Path) {
        let arquivo = std::fs::File::create(caminho).unwrap();
        imagem
            .write_with_encoder(image::codecs::webp::WebPEncoder::new_lossless(arquivo))
            .unwrap();
    }

    /// Uma pasta "do Prisma em JS" de mentira: os 3 atlas, o espectro e o ícone.
    fn origem_de_mentira(nome: &str) -> Pasta {
        let pasta = Pasta::nova(nome);
        for atlas in ["gem-atlas.webp", "burst-atlas.webp", "cross-atlas.webp"] {
            gravar_webp(
                &atlas_de_mentira(&[0, 1, 2, 3, 4, 5, 6]),
                &pasta.0.join(atlas),
            );
        }
        let mut espectro = RgbaImage::from_pixel(1254, 1254, TRANSPARENTE);
        retangulo(
            &mut espectro,
            100,
            100,
            1000,
            1000,
            Rgba([10, 200, 30, 255]),
        );
        gravar_webp(&espectro, &pasta.0.join("spectrum-gem.webp"));
        RgbaImage::from_fn(512, 512, |x, y| {
            Rgba([(x / 2) as u8, (y / 2) as u8, 128, 255])
        })
        .save(pasta.0.join("icon-512.png"))
        .unwrap();
        pasta
    }

    #[test]
    fn preparar_gera_22_imagens_e_5_icones_com_os_nomes_certos() {
        let origem = origem_de_mentira("origem_ok");
        let jogo = Pasta::nova("jogo_ok");
        let arquivos = preparar(&origem.0, &jogo.0).unwrap();
        assert_eq!(arquivos.len(), 27);
        for prefixo in ["gem", "burst", "cross"] {
            for i in 0..7 {
                let caminho = jogo.0.join(format!("assets/{prefixo}-{i}.png"));
                assert!(
                    arquivos.contains(&caminho),
                    "faltou na lista: {}",
                    caminho.display()
                );
                assert_eq!(
                    image::open(&caminho).unwrap().to_rgba8().dimensions(),
                    (160, 160),
                    "{}",
                    caminho.display()
                );
            }
        }
        let espectro = jogo.0.join("assets/spectrum.png");
        assert_eq!(
            image::open(&espectro).unwrap().to_rgba8().dimensions(),
            (160, 160)
        );
        for (densidade, lado) in [
            ("mdpi", 48),
            ("hdpi", 72),
            ("xhdpi", 96),
            ("xxhdpi", 144),
            ("xxxhdpi", 192),
        ] {
            let caminho = jogo
                .0
                .join(format!("android_res/mipmap-{densidade}/ic_launcher.png"));
            assert_eq!(
                image::open(&caminho).unwrap().to_rgba8().dimensions(),
                (lado, lado),
                "{}",
                caminho.display()
            );
        }
        assert!(arquivos.iter().all(|a| a.exists()));
    }

    #[test]
    fn preparar_com_origem_faltando_ou_invalida_da_erro_claro() {
        let jogo = Pasta::nova("jogo_erro");
        let vazia = Pasta::nova("origem_vazia");
        let erro = preparar(&vazia.0, &jogo.0).unwrap_err();
        assert!(erro.contains("gem-atlas.webp"), "{erro}");

        let origem = origem_de_mentira("origem_lixo");
        std::fs::write(origem.0.join("burst-atlas.webp"), b"isto nao e uma imagem").unwrap();
        let erro = preparar(&origem.0, &jogo.0).unwrap_err();
        assert!(erro.contains("burst-atlas.webp"), "{erro}");

        let origem = origem_de_mentira("origem_seis_pedras");
        gravar_webp(
            &atlas_de_mentira(&[0, 1, 2, 3, 4, 5]),
            &origem.0.join("cross-atlas.webp"),
        );
        let erro = preparar(&origem.0, &jogo.0).unwrap_err();
        assert!(
            erro.contains("cross-atlas.webp") && erro.contains("esperava 7 pedras"),
            "{erro}"
        );
    }

    /// Compara as imagens geradas agora com as que já estão no jogo (geradas pelo script de Python).
    /// Rode com: PRISMA_JS=/caminho/do/prisma-js cargo test --manifest-path tools/Cargo.toml -- --ignored --nocapture
    #[test]
    #[ignore = "precisa das imagens do Prisma em JS (variável PRISMA_JS)"]
    fn confere_com_os_png_de_hoje() {
        let origem = PathBuf::from(
            std::env::var("PRISMA_JS").expect("defina PRISMA_JS com a pasta do Prisma em JS"),
        );
        let novo = Pasta::nova("confere");
        let geradas = preparar(&origem, &novo.0).unwrap();
        assert_eq!(geradas.len(), 27);
        let mut piores = (0.0f64, String::new());
        for gerada in &geradas {
            let relativo = gerada.strip_prefix(&novo.0).unwrap();
            let de_hoje = Path::new("../game").join(relativo);
            let (a, b) = (
                image::open(gerada).unwrap().to_rgba8(),
                image::open(&de_hoje).unwrap().to_rgba8(),
            );
            assert_eq!(
                a.dimensions(),
                b.dimensions(),
                "tamanho de {}",
                relativo.display()
            );
            let total = (a.width() * a.height()) as f64;
            let (mut soma, mut longe) = (0u64, 0u64);
            for (pa, pb) in a.pixels().zip(b.pixels()) {
                let dif: Vec<u64> = (0..4).map(|c| u64::from(pa[c].abs_diff(pb[c]))).collect();
                soma += dif.iter().sum::<u64>();
                if dif.iter().any(|&d| d > 12) {
                    longe += 1;
                }
            }
            let medio = soma as f64 / (total * 4.0);
            let fora = longe as f64 / total * 100.0;
            let (ca, cb) = (caixa_visivel(&a), caixa_visivel(&b));
            println!(
                "{:<42} erro médio {medio:.3}  pixels >12: {fora:.3}%  caixa {ca:?} vs {cb:?}",
                relativo.display()
            );
            assert!(medio <= 1.5, "{}: erro médio {medio}", relativo.display());
            assert!(
                fora <= 1.0,
                "{}: {fora}% dos pixels diferem mais que 12",
                relativo.display()
            );
            for (x, y) in [(ca.0, cb.0), (ca.1, cb.1), (ca.2, cb.2), (ca.3, cb.3)] {
                assert!(
                    x.abs_diff(y) <= 1,
                    "{}: caixa visível {ca:?} vs {cb:?}",
                    relativo.display()
                );
            }
            if medio > piores.0 {
                piores = (medio, relativo.display().to_string());
            }
        }
        println!("pior erro médio: {:.3} em {}", piores.0, piores.1);
    }
}
