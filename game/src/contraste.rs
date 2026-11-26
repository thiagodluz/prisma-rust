//! A razão de contraste da WCAG entre duas cores: serve para os testes provarem que o texto do
//! jogo se lê sobre o fundo onde é desenhado (mínimo de 4,5 para 1 em texto comum).

/// A luminância relativa de uma cor sRGB (0 = preto, 1 = branco), pela fórmula da WCAG.
fn luminancia(cor: [u8; 3]) -> f32 {
    let linear = |canal: u8| {
        let c = canal as f32 / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(cor[0]) + 0.7152 * linear(cor[1]) + 0.0722 * linear(cor[2])
}

/// A razão de contraste entre duas cores, de 1,0 (iguais) a 21,0 (branco sobre preto); não
/// depende de qual é o texto e qual é o fundo.
pub fn razao(a: [u8; 3], b: [u8; 3]) -> f32 {
    let (la, lb) = (luminancia(a), luminancia(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branco_sobre_preto_e_o_maximo() {
        assert!((razao([255, 255, 255], [0, 0, 0]) - 21.0).abs() < 0.01);
    }

    #[test]
    fn a_mesma_cor_tem_razao_um() {
        assert!((razao([40, 90, 200], [40, 90, 200]) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_razao_nao_depende_da_ordem() {
        let (a, b) = ([244, 242, 255], [42, 34, 80]);
        assert_eq!(razao(a, b), razao(b, a));
    }
}
