//! Os ícones dos botões de baixo (Embaralhar, Dica, Novo jogo) como polilinhas.
//!
//! Cada função devolve linhas (listas de pontos, em pixels da tela) que o `render.rs` desenha
//! com `draw_line`. São desenhos por código, não letras, para não dependerem de a fonte ter
//! símbolos como ⤨ ✧ ↻. Tudo é conta pura, com teste.

use crate::layout::Rect;

/// Uma polilinha: os pontos, em pixels da tela, ligados um ao seguinte por segmentos.
pub type Linha = Vec<(f32, f32)>;

/// O meio da área e o raio do círculo que cabe nela. Área inválida (`NaN`, negativa) vira um
/// ponto: o ícone some em vez de dar `NaN`.
fn centro_e_raio(area: Rect) -> ((f32, f32), f32) {
    let numero = |v: f32| if v.is_finite() { v } else { 0.0 };
    let (x, y, w, h) = (numero(area.x), numero(area.y), numero(area.w).max(0.0), numero(area.h).max(0.0));
    ((x + w / 2.0, y + h / 2.0), w.min(h) / 2.0)
}

/// Embaralhar (⤨): duas setas que se cruzam, uma descendo e outra subindo. As duas primeiras
/// linhas são as diagonais; as outras duas são as pontas das setas.
pub fn embaralhar(area: Rect) -> Vec<Linha> {
    let ((cx, cy), r) = centro_e_raio(area);
    let ponta = 0.8 * r;
    let s = 0.35 * r;
    let (esq, dir, cima, baixo) = (cx - ponta, cx + ponta, cy - ponta, cy + ponta);
    vec![
        vec![(esq, cima), (dir, baixo)],
        vec![(esq, baixo), (dir, cima)],
        vec![(dir - s, baixo), (dir, baixo), (dir, baixo - s)],
        vec![(dir - s, cima), (dir, cima), (dir, cima + s)],
    ]
}

/// Dica (✧): o contorno fechado de uma estrela de quatro pontas (oito pontos e o fecho).
pub fn faisca(area: Rect) -> Vec<Linha> {
    let ((cx, cy), r) = centro_e_raio(area);
    let (fora, dentro) = (0.95 * r, 0.30 * r);
    let mut pontos: Linha = (0..8)
        .map(|i| {
            let angulo = (i as f32) * std::f32::consts::FRAC_PI_4 - std::f32::consts::FRAC_PI_2;
            let raio = if i % 2 == 0 { fora } else { dentro };
            (cx + raio * angulo.cos(), cy + raio * angulo.sin())
        })
        .collect();
    pontos.push(pontos[0]);
    vec![pontos]
}

/// Novo jogo (↻): um arco de 300 graus em volta do centro e, no fim dele, a ponta da seta.
pub fn novo_jogo(area: Rect) -> Vec<Linha> {
    let ((cx, cy), r) = centro_e_raio(area);
    let raio = 0.75 * r;
    let inicio = -60.0_f32.to_radians();
    let varredura = 300.0_f32.to_radians();
    let passos = 24;
    let ponto = |angulo: f32| (cx + raio * angulo.cos(), cy + raio * angulo.sin());
    let arco: Linha = (0..=passos).map(|i| ponto(inicio + varredura * i as f32 / passos as f32)).collect();
    // A ponta da seta: duas pernas que saem do fim do arco, para trás e para os lados.
    let fim = inicio + varredura;
    let (ex, ey) = ponto(fim);
    let sentido = (-fim.sin(), fim.cos());
    let radial = (fim.cos(), fim.sin());
    let s = 0.35 * r;
    let perna = |lado: f32| (ex - sentido.0 * s + radial.0 * s * 0.6 * lado, ey - sentido.1 * s + radial.1 * s * 0.6 * lado);
    vec![arco, vec![perna(1.0), (ex, ey), perna(-1.0)]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Rect;

    const AREA: Rect = Rect { x: 100.0, y: 200.0, w: 40.0, h: 40.0 };

    fn todos(linhas: &[Linha]) -> Vec<(f32, f32)> {
        linhas.iter().flatten().copied().collect()
    }

    /// Os três ícones, com o nome, para os testes que valem para todos.
    fn icones(area: Rect) -> [(&'static str, Vec<Linha>); 3] {
        [("embaralhar", embaralhar(area)), ("faisca", faisca(area)), ("novo_jogo", novo_jogo(area))]
    }

    #[test]
    fn todos_os_pontos_ficam_dentro_da_area() {
        for (nome, linhas) in icones(AREA) {
            let pontos = todos(&linhas);
            assert!(!pontos.is_empty(), "{nome} sem pontos");
            for (x, y) in pontos {
                assert!(x >= AREA.x - 0.5 && x <= AREA.x + AREA.w + 0.5, "{nome}: x={x} fora");
                assert!(y >= AREA.y - 0.5 && y <= AREA.y + AREA.h + 0.5, "{nome}: y={y} fora");
            }
        }
    }

    #[test]
    fn area_vazia_nao_da_nan() {
        for area in [Rect::VAZIO, Rect { x: f32::NAN, y: 0.0, w: f32::NAN, h: 3.0 }] {
            for (nome, linhas) in icones(area) {
                for (x, y) in todos(&linhas) {
                    assert!(x.is_finite() && y.is_finite(), "{nome} gerou NaN em {area:?}");
                }
            }
        }
    }

    #[test]
    fn a_faisca_e_fechada_e_simetrica() {
        let linhas = faisca(AREA);
        assert_eq!(linhas.len(), 1);
        let pontos = &linhas[0];
        assert_eq!(pontos.len(), 9, "oito pontos e o fecho");
        assert_eq!(pontos[0], pontos[8]);
        let (cx, cy) = (AREA.x + AREA.w / 2.0, AREA.y + AREA.h / 2.0);
        for &(x, y) in &pontos[..8] {
            let oposto = (2.0 * cx - x, 2.0 * cy - y);
            assert!(
                pontos[..8].iter().any(|&(a, b)| (a - oposto.0).abs() < 1e-3 && (b - oposto.1).abs() < 1e-3),
                "falta o oposto de ({x}, {y})"
            );
        }
    }

    #[test]
    fn o_arco_do_novo_jogo_e_redondo_e_aberto() {
        let linhas = novo_jogo(AREA);
        let arco = &linhas[0];
        let (cx, cy) = (AREA.x + AREA.w / 2.0, AREA.y + AREA.h / 2.0);
        let distancias: Vec<f32> = arco.iter().map(|&(x, y)| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt()).collect();
        let (menor, maior) = distancias.iter().fold((f32::MAX, f32::MIN), |(a, b), &d| (a.min(d), b.max(d)));
        assert!(maior - menor < 0.5, "o arco não é redondo: de {menor} a {maior}");
        let (primeiro, ultimo) = (arco[0], arco[arco.len() - 1]);
        let corda = ((primeiro.0 - ultimo.0).powi(2) + (primeiro.1 - ultimo.1).powi(2)).sqrt();
        assert!(corda > 0.1 * AREA.w, "o arco está fechado: {corda}");
    }

    /// Os segmentos (a, b) e (c, d) se cruzam?
    fn cruzam(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> bool {
        let lado = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
        lado(a, b, c) * lado(a, b, d) < 0.0 && lado(c, d, a) * lado(c, d, b) < 0.0
    }

    #[test]
    fn as_duas_diagonais_do_embaralhar_se_cruzam() {
        let linhas = embaralhar(AREA);
        assert!(linhas.len() >= 2);
        let (primeira, segunda) = (&linhas[0], &linhas[1]);
        assert!(cruzam(primeira[0], primeira[1], segunda[0], segunda[1]), "as diagonais não se cruzam");
    }
}
