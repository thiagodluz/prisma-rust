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
/// O primeiro plano do ícone adaptativo tem 108 dp de lado: (densidade, lado em pixels).
const PRIMEIRO_PLANO: [(&str, u32); 5] = [
    ("mdpi", 108),
    ("hdpi", 162),
    ("xhdpi", 216),
    ("xxhdpi", 324),
    ("xxxhdpi", 432),
];
/// A figura ocupa 61% da altura do primeiro plano: a ponta dela cai a 33 dp do centro (33/108),
/// dentro da zona segura de 66 dp que o Android nunca corta.
const ALTURA_DA_FIGURA: f64 = 0.61;
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

/// Os quatro quadradinhos do ícone de onde sai a cor de fundo: cada um tem 6% do lado e começa a
/// 12% das bordas (por dentro dos cantos arredondados, que são transparentes). Ordem: em cima à
/// esquerda, em cima à direita, embaixo à esquerda, embaixo à direita.
fn cantos_do_icone(icone: &RgbaImage) -> [Vec<[u8; 3]>; 4] {
    let (largura, altura) = icone.dimensions();
    let faixa = |total: u32, depois: bool| {
        let pedaco = (f64::from(total) * 0.06).ceil() as u32;
        let inicio = (f64::from(total) * 0.12).round() as u32;
        let inicio = if depois { total.saturating_sub(inicio + pedaco) } else { inicio };
        inicio..(inicio + pedaco).min(total)
    };
    let mut cantos: [Vec<[u8; 3]>; 4] = Default::default();
    for (numero, canto) in cantos.iter_mut().enumerate() {
        let (xs, ys) = (faixa(largura, numero % 2 == 1), faixa(altura, numero >= 2));
        for y in ys {
            for x in xs.clone() {
                let pixel = icone.get_pixel(x, y);
                if pixel[3] == 255 {
                    canto.push([pixel[0], pixel[1], pixel[2]]);
                }
            }
        }
    }
    cantos
}

/// A cor média de uma lista de pixels, ou `None` se a lista é vazia.
fn media(pixels: &[[u8; 3]]) -> Option<[f64; 3]> {
    if pixels.is_empty() {
        return None;
    }
    let mut soma = [0.0; 3];
    for pixel in pixels {
        for (canal, valor) in soma.iter_mut().zip(pixel) {
            *canal += f64::from(*valor);
        }
    }
    Some(soma.map(|canal| canal / pixels.len() as f64))
}

/// A cor de fundo do ícone (para o fundo do ícone adaptativo): a média dos pixels opacos nos
/// quatro cantos de dentro. Sem nenhum pixel opaco ali, devolve preto.
pub fn cor_de_fundo_do_icone(icone: &RgbaImage) -> [u8; 3] {
    let todos: Vec<[u8; 3]> = cantos_do_icone(icone).into_iter().flatten().collect();
    media(&todos).unwrap_or([0.0; 3]).map(|canal| canal.round() as u8)
}

/// Quanto o pixel `cor` se afasta do fundo `fundo` (soma das diferenças dos três canais, de 0 a 765).
fn distancia_ao_fundo(cor: &Rgba<u8>, fundo: [f64; 3]) -> f64 {
    (0..3).map(|canal| (f64::from(cor[canal]) - fundo[canal]).abs()).sum()
}

/// O `smoothstep`: sobe de 0 a 1 entre `de` e `ate`, suave nas pontas.
fn suavizar(valor: f64, de: f64, ate: f64) -> f64 {
    let t = ((valor - de) / (ate - de)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// O primeiro plano do ícone adaptativo: só a figura do ícone (o losango), sem o fundo, numa tela
/// transparente de `lado` x `lado`, centrada e com 61% da altura (ver `ALTURA_DA_FIGURA`).
///
/// A figura é achada pela diferença para o fundo: o fundo do ícone é um degradê, então a cor de
/// fundo de cada pixel vem dos quatro cantos de dentro (interpolados), e a opacidade cresce com a
/// distância a ela, com suavização nas bordas. Ícone sem figura devolve a tela toda transparente.
pub fn primeiro_plano_do_icone(icone: &RgbaImage, lado: u32) -> RgbaImage {
    let lado = lado.max(1);
    let vazio = RgbaImage::from_pixel(lado, lado, TRANSPARENTE);
    let (largura, altura) = icone.dimensions();
    let cantos = cantos_do_icone(icone);
    let todos: Vec<[u8; 3]> = cantos.iter().flatten().copied().collect();
    let Some(geral) = media(&todos) else { return vazio };
    let cor_do_canto: Vec<[f64; 3]> = cantos.iter().map(|canto| media(canto).unwrap_or(geral)).collect();
    // O centro de cada quadradinho (em fração do lado): 15% e 85%. O degradê é linear, então a
    // interpolação (e a extrapolação, fora dos 15%..85%) dá o fundo de cada ponto.
    let fundo_em = |x: u32, y: u32| -> [f64; 3] {
        let u = (f64::from(x) / f64::from(largura) - 0.15) / 0.70;
        let v = (f64::from(y) / f64::from(altura) - 0.15) / 0.70;
        let mut fundo = [0.0; 3];
        for (canal, valor) in fundo.iter_mut().enumerate() {
            let (a, b, c, d) = (cor_do_canto[0][canal], cor_do_canto[1][canal], cor_do_canto[2][canal], cor_do_canto[3][canal]);
            *valor = (a * (1.0 - u) + b * u) * (1.0 - v) + (c * (1.0 - u) + d * u) * v;
        }
        fundo
    };
    let mut figura = icone.clone();
    for (x, y, pixel) in figura.enumerate_pixels_mut() {
        let sua_opacidade = f64::from(pixel[3]) / 255.0;
        let opacidade = suavizar(distancia_ao_fundo(pixel, fundo_em(x, y)), 30.0, 90.0) * sua_opacidade;
        pixel[3] = (opacidade * 255.0).round() as u8;
    }
    let (mut x0, mut x1, mut y0, mut y1) = (u32::MAX, 0, u32::MAX, 0);
    for (x, y, pixel) in figura.enumerate_pixels() {
        if pixel[3] >= 128 {
            x0 = x0.min(x);
            x1 = x1.max(x + 1);
            y0 = y0.min(y);
            y1 = y1.max(y + 1);
        }
    }
    if x1 == 0 {
        return vazio;
    }
    let recorte = imageops::crop_imm(&figura, x0, y0, x1 - x0, y1 - y0).to_image();
    let nova_altura = ((f64::from(lado) * ALTURA_DA_FIGURA).round() as u32).max(1);
    let fator = f64::from(nova_altura) / f64::from(recorte.height());
    let nova_largura = ((f64::from(recorte.width()) * fator).round() as u32).max(1);
    let reduzida = redimensionar(&recorte, nova_largura, nova_altura);
    let mut tela = vazio;
    let dx = (i64::from(lado) - i64::from(nova_largura)) / 2;
    let dy = (i64::from(lado) - i64::from(nova_altura)) / 2;
    imageops::replace(&mut tela, &reduzida, dx, dy);
    tela
}

/// O `ic_launcher.xml` do ícone adaptativo (Android 8 em diante): o fundo é a cor
/// `ic_launcher_background` e o primeiro plano é a imagem `ic_launcher_foreground`.
pub fn xml_do_icone_adaptativo() -> String {
    concat!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
        "<adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\">\n",
        "    <background android:drawable=\"@color/ic_launcher_background\"/>\n",
        "    <foreground android:drawable=\"@mipmap/ic_launcher_foreground\"/>\n",
        "</adaptive-icon>\n"
    )
    .to_string()
}

/// O `values/ic_launcher_background.xml`: a cor de fundo do ícone adaptativo, em `#RRGGBB`.
pub fn xml_da_cor_do_icone(cor: [u8; 3]) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n    <color name=\"ic_launcher_background\">#{:02X}{:02X}{:02X}</color>\n</resources>\n",
        cor[0], cor[1], cor[2]
    )
}

fn abrir(caminho: &Path) -> Result<RgbaImage, String> {
    image::open(caminho)
        .map(|imagem| imagem.to_rgba8())
        .map_err(|erro| format!("{}: {erro}", caminho.display()))
}

/// Gera as 22 imagens de `<jogo>/assets` (21 pedras e o espectro), os 5 ícones do Android e o
/// ícone adaptativo (5 primeiros planos e 2 arquivos XML) em `<jogo>/android_res`, a partir da
/// pasta do Prisma em JS. Só grava depois de ter tudo pronto: se uma imagem de origem estiver
/// errada, nada muda no jogo. Devolve os arquivos gravados.
pub fn preparar(origem: &Path, jogo: &Path) -> Result<Vec<PathBuf>, String> {
    let assets = jogo.join("assets");
    let mut prontas: Vec<(PathBuf, RgbaImage)> = Vec::new();
    let mut textos: Vec<(PathBuf, String)> = Vec::new();
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
    for (densidade, lado) in PRIMEIRO_PLANO {
        let caminho = jogo.join(format!("android_res/mipmap-{densidade}/ic_launcher_foreground.png"));
        prontas.push((caminho, primeiro_plano_do_icone(&icone, lado)));
    }
    textos.push((jogo.join("android_res/mipmap-anydpi-v26/ic_launcher.xml"), xml_do_icone_adaptativo()));
    textos.push((
        jogo.join("android_res/values/ic_launcher_background.xml"),
        xml_da_cor_do_icone(cor_de_fundo_do_icone(&icone)),
    ));
    for (caminho, imagem) in &prontas {
        if let Some(pasta) = caminho.parent() {
            std::fs::create_dir_all(pasta)
                .map_err(|erro| format!("{}: {erro}", pasta.display()))?;
        }
        imagem
            .save(caminho)
            .map_err(|erro| format!("{}: {erro}", caminho.display()))?;
    }
    for (caminho, texto) in &textos {
        if let Some(pasta) = caminho.parent() {
            std::fs::create_dir_all(pasta)
                .map_err(|erro| format!("{}: {erro}", pasta.display()))?;
        }
        std::fs::write(caminho, texto).map_err(|erro| format!("{}: {erro}", caminho.display()))?;
    }
    Ok(prontas
        .into_iter()
        .map(|(caminho, _)| caminho)
        .chain(textos.into_iter().map(|(caminho, _)| caminho))
        .collect())
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
        icone_de_mentira()
            .save(pasta.0.join("icon-512.png"))
            .unwrap();
        pasta
    }

    /// Um ícone de mentira parecido com o de verdade: fundo em degradê na diagonal, a borda de
    /// fora transparente (os cantos arredondados) e um losango claro no meio (de 50 a 458 de altura).
    fn icone_de_mentira() -> RgbaImage {
        RgbaImage::from_fn(512, 512, |x, y| {
            if x < 16 || y < 16 || x >= 496 || y >= 496 {
                return TRANSPARENTE;
            }
            let (dx, dy) = ((f64::from(x) - 256.0).abs(), (f64::from(y) - 256.0).abs());
            if dx / 118.0 + dy / 206.0 <= 1.0 {
                return Rgba([199, 209, 255, 255]);
            }
            let t = f64::from(x + y) / 1022.0;
            let mistura = |a: f64, b: f64| (a + (b - a) * t).round() as u8;
            Rgba([mistura(46.0, 21.0), mistura(51.0, 23.0), mistura(92.0, 50.0), 255])
        })
    }

    /// Um ícone de quatro quadrantes lisos, com a borda de fora transparente: as cores dos
    /// quatro cantos são conhecidas.
    fn icone_de_quadrantes(cores: [[u8; 3]; 4]) -> RgbaImage {
        RgbaImage::from_fn(512, 512, |x, y| {
            if x < 16 || y < 16 || x >= 496 || y >= 496 {
                return TRANSPARENTE;
            }
            // Uma moldura magenta logo por dentro da borda: quem tirar a cor perto demais da borda erra.
            if x < 48 || y < 48 || x >= 464 || y >= 464 {
                return Rgba([255, 0, 255, 255]);
            }
            let c = cores[usize::from(y >= 256) * 2 + usize::from(x >= 256)];
            Rgba([c[0], c[1], c[2], 255])
        })
    }

    const LADOS_DO_PRIMEIRO_PLANO: [u32; 5] = [108, 162, 216, 324, 432];

    #[test]
    fn primeiro_plano_tem_o_lado_pedido() {
        let icone = icone_de_mentira();
        for lado in LADOS_DO_PRIMEIRO_PLANO {
            assert_eq!(primeiro_plano_do_icone(&icone, lado).dimensions(), (lado, lado));
        }
        // Só a figura sobra: o canto e o fundo ficam transparentes.
        let figura = primeiro_plano_do_icone(&icone, 432);
        assert_eq!(figura.get_pixel(0, 0)[3], 0);
        assert_eq!(figura.get_pixel(60, 216)[3], 0, "fundo ao lado do losango");
        assert!(figura.get_pixel(216, 216)[3] >= 250, "o meio do losango é opaco");
        assert_eq!(figura.get_pixel(216, 216).0[..3], [199, 209, 255], "e mantém a cor");
    }

    #[test]
    fn a_figura_cabe_na_zona_segura_de_66_dp() {
        let icone = icone_de_mentira();
        for lado in LADOS_DO_PRIMEIRO_PLANO {
            let figura = primeiro_plano_do_icone(&icone, lado);
            let centro = f64::from(lado) / 2.0;
            let limite = 0.305 * f64::from(lado) + 1.0;
            let mut maior: f64 = 0.0;
            for (x, y, pixel) in figura.enumerate_pixels() {
                if pixel[3] > 25 {
                    let longe = (f64::from(x) + 0.5 - centro).hypot(f64::from(y) + 0.5 - centro);
                    maior = maior.max(longe);
                    assert!(longe <= limite, "lado {lado}: pixel ({x}, {y}) a {longe} > {limite}");
                }
            }
            // E a figura aproveita a zona: a ponta chega perto do limite (61% da altura).
            assert!(maior >= 0.29 * f64::from(lado), "lado {lado}: a figura ficou pequena ({maior})");
        }
    }

    #[test]
    fn a_figura_fica_centrada() {
        let icone = icone_de_mentira();
        for lado in LADOS_DO_PRIMEIRO_PLANO {
            let (x0, x1, y0, y1) = caixa_visivel(&primeiro_plano_do_icone(&icone, lado));
            let meio = f64::from(lado) / 2.0;
            assert!((f64::from(x0 + x1) / 2.0 - meio).abs() <= 1.0, "lado {lado}: x {x0}..{x1}");
            assert!((f64::from(y0 + y1) / 2.0 - meio).abs() <= 1.0, "lado {lado}: y {y0}..{y1}");
            let altura = f64::from(y1 - y0) / f64::from(lado);
            // As pontas do losango são finas e a redução as deixa meio transparentes: a altura
            // medida pode ficar uns 2 pixels abaixo dos 61%.
            assert!((altura - 0.61).abs() <= 0.025, "lado {lado}: altura {altura}");
        }
    }

    #[test]
    fn icone_sem_figura_vira_primeiro_plano_vazio() {
        let liso = RgbaImage::from_fn(512, 512, |x, y| {
            if x < 16 || y < 16 || x >= 496 || y >= 496 {
                TRANSPARENTE
            } else {
                Rgba([40, 50, 90, 255])
            }
        });
        let figura = primeiro_plano_do_icone(&liso, 108);
        assert_eq!(figura.dimensions(), (108, 108));
        assert!(figura.pixels().all(|pixel| pixel[3] == 0));
    }

    #[test]
    fn fundo_vem_dos_cantos_do_icone() {
        let cores = [[10, 20, 30], [50, 60, 70], [90, 100, 110], [130, 140, 150]];
        assert_eq!(cor_de_fundo_do_icone(&icone_de_quadrantes(cores)), [70, 80, 90]);
        // O losango do meio e a borda transparente não entram na conta.
        let com_figura = icone_de_mentira();
        let fundo = cor_de_fundo_do_icone(&com_figura);
        assert!((30..=40).contains(&fundo[0]) && (33..=44).contains(&fundo[1]) && (68..=76).contains(&fundo[2]), "{fundo:?}");
        // Um canto sem nenhum pixel opaco não conta (e o resto ainda dá a média).
        let mut vazio = icone_de_quadrantes(cores);
        retangulo(&mut vazio, 0, 0, 200, 200, TRANSPARENTE);
        assert_eq!(cor_de_fundo_do_icone(&vazio), [90, 100, 110]);
        // Tudo transparente: nada para tirar a média, devolve preto sem pânico.
        assert_eq!(cor_de_fundo_do_icone(&RgbaImage::from_pixel(512, 512, TRANSPARENTE)), [0, 0, 0]);
    }

    #[test]
    fn xml_adaptativo_cita_o_fundo_e_o_primeiro_plano() {
        let xml = xml_do_icone_adaptativo();
        assert!(xml.contains("<adaptive-icon"), "{xml}");
        assert!(xml.contains(r#"<background android:drawable="@color/ic_launcher_background"/>"#), "{xml}");
        assert!(xml.contains(r#"<foreground android:drawable="@mipmap/ic_launcher_foreground"/>"#), "{xml}");
        assert!(xml.contains("http://schemas.android.com/apk/res/android"), "{xml}");
        let cor = xml_da_cor_do_icone([0x21, 0x25, 0x47]);
        assert!(cor.contains(r##"<color name="ic_launcher_background">#212547</color>"##), "{cor}");
        assert!(cor.contains("<resources>") && cor.contains("</resources>"), "{cor}");
        assert!(xml_da_cor_do_icone([0, 10, 255]).contains("#000AFF"));
    }

    #[test]
    fn preparar_grava_tambem_o_icone_adaptativo() {
        let origem = origem_de_mentira("origem_adaptativo");
        let jogo = Pasta::nova("jogo_adaptativo");
        let arquivos = preparar(&origem.0, &jogo.0).unwrap();
        for (densidade, lado) in [("mdpi", 108), ("hdpi", 162), ("xhdpi", 216), ("xxhdpi", 324), ("xxxhdpi", 432)] {
            let caminho = jogo.0.join(format!("android_res/mipmap-{densidade}/ic_launcher_foreground.png"));
            assert!(arquivos.contains(&caminho), "faltou na lista: {}", caminho.display());
            let figura = image::open(&caminho).unwrap().to_rgba8();
            assert_eq!(figura.dimensions(), (lado, lado), "{}", caminho.display());
            assert!(caixa_visivel(&figura).1 > 0, "{}: a figura não pode vir vazia", caminho.display());
            // Os ícones comuns continuam.
            assert!(jogo.0.join(format!("android_res/mipmap-{densidade}/ic_launcher.png")).exists());
        }
        let xml = jogo.0.join("android_res/mipmap-anydpi-v26/ic_launcher.xml");
        let cor = jogo.0.join("android_res/values/ic_launcher_background.xml");
        assert!(arquivos.contains(&xml) && arquivos.contains(&cor));
        assert_eq!(std::fs::read_to_string(&xml).unwrap(), xml_do_icone_adaptativo());
        let esperada = cor_de_fundo_do_icone(&icone_de_mentira());
        assert_eq!(std::fs::read_to_string(&cor).unwrap(), xml_da_cor_do_icone(esperada));
    }

    #[test]
    fn preparar_gera_22_imagens_e_5_icones_com_os_nomes_certos() {
        let origem = origem_de_mentira("origem_ok");
        let jogo = Pasta::nova("jogo_ok");
        let arquivos = preparar(&origem.0, &jogo.0).unwrap();
        // 22 imagens + 5 ícones + 5 primeiros planos do ícone adaptativo + 2 arquivos de texto.
        assert_eq!(arquivos.len(), 34);
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
        assert_eq!(geradas.len(), 34);
        let mut piores = (0.0f64, String::new());
        for gerada in &geradas {
            let relativo = gerada.strip_prefix(&novo.0).unwrap();
            if gerada.extension().is_some_and(|e| e == "xml") {
                // Os XML do ícone adaptativo são texto: só confere que são iguais.
                let de_hoje = Path::new("../game").join(relativo);
                assert_eq!(
                    std::fs::read_to_string(gerada).unwrap(),
                    std::fs::read_to_string(&de_hoje).unwrap(),
                    "{}",
                    relativo.display()
                );
                continue;
            }
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
