//! Formas pintadas por código: cantos arredondados, degradê, borda e sombra suave.
//!
//! Tudo aqui é conta pura (sem janela, sem Macroquad): `pintar` devolve os pixels de uma imagem
//! pequena, que o `render.rs` vira textura uma vez só, na abertura do app, e desenha esticada em
//! "nove fatias" (os cantos mantêm o tamanho e o meio estica).

use crate::layout::Rect;

/// Uma cor em RGBA (vermelho, verde, azul e opacidade, de 0 a 255), sem pré-multiplicar.
pub type Cor = [u8; 4];

/// Uma cor escrita como no CSS do JS, `0xRRGGBBAA`.
pub const fn rgba(hex: u32) -> Cor {
    [(hex >> 24) as u8, (hex >> 16) as u8, (hex >> 8) as u8, hex as u8]
}

/// A receita de uma forma: tudo que `pintar` precisa para desenhá-la.
///
/// A imagem inteira tem `lado` × `lado` pixels; a forma ocupa o quadrado do meio, de
/// `lado - 2 × sombra` pixels, e a sombra (se houver) fica na margem de `sombra` pixels em volta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Receita {
    pub lado: u32,
    /// O raio dos cantos da forma.
    pub raio: u32,
    /// A espessura da borda, desenhada por cima do degradê, para dentro.
    pub borda: u32,
    pub cor_da_borda: Cor,
    /// As três cores do degradê, em 0, 0,5 e 1 do caminho.
    pub degrade: [Cor; 3],
    /// A direção do degradê, em graus como no CSS: 0 é para cima, 90 para a direita, 145 para
    /// baixo e à direita.
    pub angulo: f32,
    pub sombra: u32,
    pub cor_da_sombra: Cor,
}

fn misturar(a: Cor, b: Cor, t: f32) -> Cor {
    let canal = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    [canal(0), canal(1), canal(2), canal(3)]
}

/// A cor do degradê no ponto (`x`, `y`), de 0 a 1 dentro da forma.
pub fn degrade_em(degrade: &[Cor; 3], angulo: f32, x: f32, y: f32) -> Cor {
    let radianos = angulo.to_radians();
    let (seno, cosseno) = (radianos.sin(), radianos.cos());
    // O caminho vai de uma ponta do quadrado à ponta oposta: os cantos pegam exatamente 0 e 1.
    let comprimento = seno.abs() + cosseno.abs();
    let t = (0.5 + ((x - 0.5) * seno - (y - 0.5) * cosseno) / comprimento).clamp(0.0, 1.0);
    if t < 0.5 {
        misturar(degrade[0], degrade[1], t * 2.0)
    } else {
        misturar(degrade[1], degrade[2], (t - 0.5) * 2.0)
    }
}

/// A distância (com sinal) de um ponto ao retângulo arredondado centrado na origem, de meio lado
/// `meio` e raio `raio`: negativa dentro, positiva fora.
fn distancia(x: f32, y: f32, meio: f32, raio: f32) -> f32 {
    let qx = x.abs() - (meio - raio);
    let qy = y.abs() - (meio - raio);
    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - raio
}

/// Uma cor com a opacidade multiplicada por `fator`, como (R, G, B, A) em 0 a 1 pré-multiplicado.
fn pre_multiplicada(cor: Cor, fator: f32) -> [f32; 4] {
    let alfa = cor[3] as f32 / 255.0 * fator;
    [cor[0] as f32 / 255.0 * alfa, cor[1] as f32 / 255.0 * alfa, cor[2] as f32 / 255.0 * alfa, alfa]
}

/// `de_cima` por cima de `de_baixo` (as duas pré-multiplicadas).
fn por_cima(de_cima: [f32; 4], de_baixo: [f32; 4]) -> [f32; 4] {
    let resto = 1.0 - de_cima[3];
    [
        de_cima[0] + de_baixo[0] * resto,
        de_cima[1] + de_baixo[1] * resto,
        de_cima[2] + de_baixo[2] * resto,
        de_cima[3] + de_baixo[3] * resto,
    ]
}

fn suavizar(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Pinta a forma da `receita`: devolve `lado × lado × 4` bytes (RGBA, linha por linha, sem
/// pré-multiplicar), prontos para virar textura. O arco dos cantos é suavizado em 1 pixel.
pub fn pintar(receita: &Receita) -> Vec<u8> {
    let lado = receita.lado as usize;
    let mut imagem = vec![0u8; lado * lado * 4];
    let sombra = receita.sombra as f32;
    let forma = receita.lado as f32 - 2.0 * sombra;
    if forma <= 0.0 {
        return imagem;
    }
    let meio = forma / 2.0;
    let raio = (receita.raio as f32).min(meio);
    let borda = receita.borda as f32;
    for y in 0..lado {
        for x in 0..lado {
            // O centro do pixel, em relação ao centro da forma.
            let px = x as f32 + 0.5 - sombra;
            let py = y as f32 + 0.5 - sombra;
            let d = distancia(px - meio, py - meio, meio, raio);
            let cobertura = (0.5 - d).clamp(0.0, 1.0);
            // A forma: degradê, com a borda por cima.
            let degrade = degrade_em(&receita.degrade, receita.angulo, px / forma, py / forma);
            let mut cor = pre_multiplicada(degrade, 1.0);
            if borda > 0.0 {
                let peso = (d + borda + 0.5).clamp(0.0, 1.0);
                cor = por_cima(pre_multiplicada(receita.cor_da_borda, peso), cor);
            }
            let forma_pixel = [cor[0] * cobertura, cor[1] * cobertura, cor[2] * cobertura, cor[3] * cobertura];
            // A sombra só existe fora da forma e some com a distância.
            let mut final_ = forma_pixel;
            if sombra > 0.0 {
                let alcance = 1.0 - suavizar(d.max(0.0) / sombra);
                let fora = pre_multiplicada(receita.cor_da_sombra, alcance * (1.0 - cobertura));
                final_ = por_cima(forma_pixel, fora);
            }
            let i = (y * lado + x) * 4;
            let alfa = final_[3];
            if alfa > 0.0 {
                for canal in 0..3 {
                    imagem[i + canal] = (final_[canal] / alfa * 255.0).round().clamp(0.0, 255.0) as u8;
                }
            }
            imagem[i + 3] = (alfa * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    imagem
}

/// Uma das nove partes de uma forma esticada: o pedaço da imagem (`origem`, em pixels dela) e o
/// lugar onde ele é desenhado na tela (`destino`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fatia {
    pub origem: Rect,
    pub destino: Rect,
}

fn finito(valor: f32) -> f32 {
    if valor.is_finite() {
        valor
    } else {
        0.0
    }
}

/// Divide uma imagem quadrada em nove fatias (3 × 3, de cima para baixo e da esquerda para a
/// direita) e diz onde desenhar cada uma para preencher `destino`: os quatro cantos (de
/// `canto_da_imagem` pixels na imagem) vão para cantos de `canto_do_destino` pixels, e as arestas
/// e o miolo esticam. Um canto grande demais para o destino é reduzido (no máximo metade do
/// menor lado), e uma entrada inválida (negativa, `NaN`) vira tamanho zero, nunca `NaN`.
pub fn nove_fatias(lado_da_imagem: f32, canto_da_imagem: f32, destino: Rect, canto_do_destino: f32) -> [Fatia; 9] {
    let (x, y) = (finito(destino.x), finito(destino.y));
    let (w, h) = (finito(destino.w).max(0.0), finito(destino.h).max(0.0));
    let lado = finito(lado_da_imagem).max(0.0);
    let canto_d = finito(canto_do_destino).clamp(0.0, (w / 2.0).min(h / 2.0));
    let canto_o = finito(canto_da_imagem).clamp(0.0, lado / 2.0);
    let colunas_d = [x, x + canto_d, x + w - canto_d, x + w];
    let linhas_d = [y, y + canto_d, y + h - canto_d, y + h];
    let cortes_o = [0.0, canto_o, lado - canto_o, lado];
    std::array::from_fn(|i| {
        let (linha, coluna) = (i / 3, i % 3);
        Fatia {
            origem: Rect {
                x: cortes_o[coluna],
                y: cortes_o[linha],
                w: cortes_o[coluna + 1] - cortes_o[coluna],
                h: cortes_o[linha + 1] - cortes_o[linha],
            },
            destino: Rect {
                x: colunas_d[coluna],
                y: linhas_d[linha],
                w: colunas_d[coluna + 1] - colunas_d[coluna],
                h: linhas_d[linha + 1] - linhas_d[linha],
            },
        }
    })
}

/// O pedaço de uma imagem (em pixels dela) que cobre a tela inteira sem deformar: a imagem é
/// ampliada até preencher a tela e o que sobra é cortado, igualmente de cada lado. Tamanho
/// inválido (zero, negativo, `NaN`) devolve `Rect::VAZIO`.
pub fn cobrir(imagem_w: f32, imagem_h: f32, tela_w: f32, tela_h: f32) -> Rect {
    let valido = |v: f32| v.is_finite() && v > 0.0;
    if !(valido(imagem_w) && valido(imagem_h) && valido(tela_w) && valido(tela_h)) {
        return Rect::VAZIO;
    }
    let escala = (tela_w / imagem_w).max(tela_h / imagem_h);
    let (w, h) = (tela_w / escala, tela_h / escala);
    Rect { x: (imagem_w - w) / 2.0, y: (imagem_h - h) / 2.0, w, h }
}

/// `area` com `margem` a mais em cada um dos quatro lados.
pub fn expandir(area: Rect, margem: f32) -> Rect {
    Rect { x: area.x - margem, y: area.y - margem, w: area.w + 2.0 * margem, h: area.h + 2.0 * margem }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_nove_fatias_cobrem_o_destino() {
        let destino = Rect { x: 10.0, y: 20.0, w: 100.0, h: 60.0 };
        let fatias = nove_fatias(64.0, 12.0, destino, 12.0);
        let area: f32 = fatias.iter().map(|f| f.destino.w * f.destino.h).sum();
        assert!((area - 6000.0).abs() < 1e-3, "a soma das áreas é a do destino: {area}");
        for (i, a) in fatias.iter().enumerate() {
            let d = a.destino;
            assert!(d.x >= destino.x - 1e-3 && d.y >= destino.y - 1e-3, "fatia {i} fora do destino");
            assert!(d.x + d.w <= destino.x + destino.w + 1e-3 && d.y + d.h <= destino.y + destino.h + 1e-3, "fatia {i} fora do destino");
            for (j, b) in fatias.iter().enumerate().skip(i + 1) {
                let e = b.destino;
                let sobrepoe = d.x < e.x + e.w - 1e-3 && e.x < d.x + d.w - 1e-3 && d.y < e.y + e.h - 1e-3 && e.y < d.y + d.h - 1e-3;
                assert!(!sobrepoe, "fatias {i} e {j} se sobrepõem");
            }
        }
        for canto in [0, 2, 6, 8] {
            assert_eq!((fatias[canto].destino.w, fatias[canto].destino.h), (12.0, 12.0), "canto {canto}");
        }
    }

    #[test]
    fn as_fatias_da_origem_cobrem_a_imagem() {
        let fatias = nove_fatias(64.0, 12.0, Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }, 12.0);
        let area: f32 = fatias.iter().map(|f| f.origem.w * f.origem.h).sum();
        assert!((area - 4096.0).abs() < 1e-3, "{area}");
    }

    #[test]
    fn canto_grande_demais_e_reduzido() {
        let fatias = nove_fatias(64.0, 12.0, Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, 20.0);
        assert_eq!((fatias[0].destino.w, fatias[0].destino.h), (5.0, 5.0));
        assert_eq!(fatias[4].destino.w, 0.0, "o miolo some");
    }

    #[test]
    fn destino_sem_tamanho_nao_da_nan() {
        for destino in [Rect::VAZIO, Rect { x: f32::NAN, y: 0.0, w: f32::NAN, h: 5.0 }, Rect { x: 0.0, y: 0.0, w: -4.0, h: 8.0 }] {
            for fatia in nove_fatias(64.0, 12.0, destino, 12.0) {
                for v in [fatia.destino.x, fatia.destino.y, fatia.destino.w, fatia.destino.h, fatia.origem.w, fatia.origem.h] {
                    assert!(v.is_finite(), "{destino:?} gerou {v}");
                }
                assert!(fatia.destino.w >= 0.0 && fatia.destino.h >= 0.0);
            }
        }
    }

    #[test]
    fn cobrir_preserva_a_proporcao_da_tela() {
        let quadrada = cobrir(940.0, 1672.0, 1080.0, 1080.0);
        assert_eq!((quadrada.w, quadrada.h, quadrada.y), (940.0, 940.0, 366.0));
        let larga = cobrir(940.0, 1672.0, 2000.0, 500.0);
        assert_eq!((larga.w, larga.h), (940.0, 235.0));
        let celular = cobrir(940.0, 1672.0, 360.0, 640.0);
        assert!((celular.w / celular.h - 360.0 / 640.0).abs() < 1e-4);
        for recorte in [quadrada, larga, celular] {
            assert!(recorte.x >= -1e-3 && recorte.y >= -1e-3 && recorte.x + recorte.w <= 940.0 + 1e-3 && recorte.y + recorte.h <= 1672.0 + 1e-3);
        }
        assert_eq!(cobrir(940.0, 1672.0, 0.0, 0.0), Rect::VAZIO);
        assert_eq!(cobrir(0.0, 0.0, 360.0, 640.0), Rect::VAZIO);
        assert_eq!(cobrir(940.0, 1672.0, f32::NAN, 640.0), Rect::VAZIO);
    }

    #[test]
    fn expandir_aumenta_os_quatro_lados() {
        assert_eq!(expandir(Rect { x: 10.0, y: 10.0, w: 20.0, h: 20.0 }, 5.0), Rect { x: 5.0, y: 5.0, w: 30.0, h: 30.0 });
    }

    /// Receita simples para os testes: sem borda nem sombra, degradê de três cinzas.
    fn receita(lado: u32, raio: u32) -> Receita {
        Receita {
            lado,
            raio,
            borda: 0,
            cor_da_borda: [0, 0, 0, 0],
            degrade: [[10, 10, 10, 255], [100, 100, 100, 255], [200, 200, 200, 255]],
            angulo: 145.0,
            sombra: 0,
            cor_da_sombra: [0, 0, 0, 0],
        }
    }

    /// O pixel (x, y) de uma imagem de `lado` pixels de lado.
    fn pixel(imagem: &[u8], lado: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * lado + x) * 4) as usize;
        [imagem[i], imagem[i + 1], imagem[i + 2], imagem[i + 3]]
    }

    #[test]
    fn degrade_em_segue_o_angulo() {
        let cores = [[10, 0, 0, 255], [100, 0, 0, 255], [200, 0, 0, 255]];
        // 90 graus: da esquerda para a direita; a altura não conta.
        assert_eq!(degrade_em(&cores, 90.0, 0.0, 0.3)[0], 10);
        assert_eq!(degrade_em(&cores, 90.0, 0.5, 0.9)[0], 100);
        assert_eq!(degrade_em(&cores, 90.0, 1.0, 0.0)[0], 200);
        assert_eq!(degrade_em(&cores, 90.0, 0.25, 0.5)[0], 55);
        // 145 graus: de cima à esquerda para baixo à direita; os cantos pegam as pontas.
        assert_eq!(degrade_em(&cores, 145.0, 0.0, 0.0)[0], 10);
        assert_eq!(degrade_em(&cores, 145.0, 1.0, 1.0)[0], 200);
        assert_eq!(degrade_em(&cores, 145.0, 0.5, 0.5)[0], 100);
    }

    #[test]
    fn pintar_devolve_lado_por_lado_em_rgba() {
        assert_eq!(pintar(&receita(40, 10)).len(), 40 * 40 * 4);
    }

    #[test]
    fn o_canto_de_fora_fica_transparente() {
        let imagem = pintar(&receita(40, 10));
        assert_eq!(pixel(&imagem, 40, 0, 0)[3], 0, "o canto de fora do arredondado é transparente");
        assert_eq!(pixel(&imagem, 40, 20, 20)[3], 255, "o miolo é cheio");
        assert_eq!(pixel(&imagem, 40, 20, 0)[3], 255, "o meio da aresta de cima é cheio");
    }

    #[test]
    fn a_borda_tem_a_cor_da_borda() {
        let mut r = receita(41, 10);
        r.borda = 2;
        r.cor_da_borda = [255, 0, 0, 255];
        let imagem = pintar(&r);
        assert_eq!(pixel(&imagem, 41, 20, 0), [255, 0, 0, 255]);
        assert_eq!(pixel(&imagem, 41, 20, 1), [255, 0, 0, 255]);
        assert_ne!(pixel(&imagem, 41, 20, 2), [255, 0, 0, 255], "a borda tem 2 pixels, não 3");
        let centro = pixel(&imagem, 41, 20, 20);
        assert_ne!(centro, [255, 0, 0, 255]);
        assert!((centro[0] as i32 - 100).abs() <= 1, "o centro tem a cor do meio do degradê: {centro:?}");
    }

    #[test]
    fn o_arco_tem_pixels_meio_transparentes() {
        let imagem = pintar(&receita(40, 10));
        let meio = (0..10).flat_map(|y| (0..10).map(move |x| (x, y))).filter(|&(x, y)| {
            let alfa = pixel(&imagem, 40, x, y)[3];
            alfa > 0 && alfa < 255
        });
        assert!(meio.count() > 0, "o arco do canto é suavizado");
    }

    #[test]
    fn a_forma_e_simetrica() {
        let r = receita(40, 10);
        let imagem = pintar(&r);
        for y in 0..40 {
            for x in 0..40 {
                assert_eq!(pixel(&imagem, 40, x, y)[3], pixel(&imagem, 40, 39 - x, y)[3], "({x}, {y})");
            }
        }
    }

    #[test]
    fn a_sombra_some_para_fora() {
        let mut r = receita(64, 6);
        r.sombra = 8;
        r.cor_da_sombra = [0, 0, 0, 200];
        let imagem = pintar(&r);
        let alfa = |y: u32| pixel(&imagem, 64, 32, y)[3];
        for y in 0..8 {
            assert!(alfa(y) <= alfa(y + 1), "a sombra só cresce em direção à forma (y={y})");
        }
        assert!(alfa(0) < alfa(7), "mais fraca longe da forma");
        assert_eq!(alfa(8), 255, "a forma cobre a sombra");
        assert_eq!(pixel(&imagem, 64, 32, 32)[3], 255);
    }
}
