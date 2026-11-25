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

/// Lua (☾): um crescente aberto para cima e para a direita, uma polilinha fechada. É o que sobra
/// de um círculo grande (raio 0,92 do raio da área) menos um círculo menor (0,82 do grande)
/// deslocado 0,55 do raio grande para a abertura: o contorno é o arco de fora do grande e o
/// arco de dentro do pequeno, que se encontram nos dois bicos.
pub fn lua(area: Rect) -> Vec<Linha> {
    let ((cx, cy), r) = centro_e_raio(area);
    let grande = 0.92 * r;
    let pequeno = 0.82 * grande;
    let desvio = 0.55 * grande;
    let direcao = -30.0_f32.to_radians();
    let (centro_pequeno_x, centro_pequeno_y) = (cx + desvio * direcao.cos(), cy + desvio * direcao.sin());
    // Onde os dois círculos se cruzam: a distância `a` do centro grande, ao longo da direção da
    // abertura, e a meia-corda `h` para os lados.
    let a = (grande * grande - pequeno * pequeno + desvio * desvio) / (2.0 * desvio.max(f32::EPSILON));
    let h = (grande * grande - a * a).max(0.0).sqrt();
    let meia_grande = h.atan2(a);
    let meia_pequeno = h.atan2(a - desvio);
    let passos = 20;
    let arco = |centro: (f32, f32), raio: f32, de: f32, ate: f32| -> Vec<(f32, f32)> {
        (0..=passos)
            .map(|i| {
                let angulo = de + (ate - de) * i as f32 / passos as f32;
                (centro.0 + raio * angulo.cos(), centro.1 + raio * angulo.sin())
            })
            .collect()
    };
    let volta = 2.0 * std::f32::consts::PI;
    // Por fora: o arco do círculo grande, pelo lado oposto à abertura, de um bico ao outro.
    let mut contorno = arco((cx, cy), grande, direcao + meia_grande, direcao + volta - meia_grande);
    // Por dentro: o arco do círculo pequeno, de volta ao primeiro bico, pelo lado do centro grande.
    contorno.extend(arco((centro_pequeno_x, centro_pequeno_y), pequeno, direcao + volta - meia_pequeno, direcao + meia_pequeno));
    let primeiro = contorno[0];
    contorno.push(primeiro);
    vec![contorno]
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

    /// O sentido para onde o crescente abre (para cima e para a direita, 30° acima da horizontal).
    fn abertura() -> (f32, f32) {
        let angulo = -30.0_f32.to_radians();
        (angulo.cos(), angulo.sin())
    }

    #[test]
    fn a_lua_cabe_na_area_e_e_uma_linha_so() {
        let linhas = lua(AREA);
        assert_eq!(linhas.len(), 1, "a lua é uma polilinha fechada só");
        assert!(linhas[0].len() >= 20, "pontos de sobra para as curvas ficarem lisas");
        assert_eq!(linhas[0].first(), linhas[0].last().or(None), "a linha fecha");
        for (x, y) in todos(&linhas) {
            assert!(x >= AREA.x - 0.5 && x <= AREA.x + AREA.w + 0.5, "x={x} fora");
            assert!(y >= AREA.y - 0.5 && y <= AREA.y + AREA.h + 0.5, "y={y} fora");
        }
    }

    #[test]
    fn a_lua_e_um_crescente_aberto_para_cima_e_a_direita() {
        let (cx, cy) = (AREA.x + AREA.w / 2.0, AREA.y + AREA.h / 2.0);
        let (ux, uy) = abertura();
        let projecoes: Vec<f32> = todos(&lua(AREA)).iter().map(|(x, y)| (x - cx) * ux + (y - cy) * uy).collect();
        let para_a_abertura = projecoes.iter().cloned().fold(f32::MIN, f32::max);
        let contra_a_abertura = projecoes.iter().cloned().fold(f32::MAX, f32::min);
        let raio = 0.92 * AREA.w / 2.0;
        assert!(para_a_abertura < 0.75 * raio, "a abertura é cortada: {para_a_abertura}");
        assert!(-contra_a_abertura > 0.9 * raio, "o lado de trás é redondo e cheio: {contra_a_abertura}");
    }

    #[test]
    fn a_lua_resiste_a_area_impossivel() {
        for area in [Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, Rect { x: f32::NAN, y: 1.0, w: f32::NAN, h: -4.0 }, Rect { x: 5.0, y: 5.0, w: -10.0, h: 10.0 }] {
            for (x, y) in todos(&lua(area)) {
                assert!(x.is_finite() && y.is_finite(), "ponto não finito para {area:?}");
            }
        }
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
    fn a_faisca_tem_pontas_longas_e_reentrancias_curtas() {
        let pontos = &faisca(AREA)[0];
        let (cx, cy) = (AREA.x + AREA.w / 2.0, AREA.y + AREA.h / 2.0);
        let distancia = |i: usize| ((pontos[i].0 - cx).powi(2) + (pontos[i].1 - cy).powi(2)).sqrt();
        for i in (0..8).step_by(2) {
            assert!(distancia(i) > 2.0 * distancia(i + 1), "a ponta {i} tem de ser bem mais longa que a reentrância ao lado");
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
