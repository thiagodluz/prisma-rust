//! Contas de posição: dado o tamanho da tela, onde fica o tabuleiro e cada casa.
//!
//! Tudo aqui é número puro (sem janela, sem imagem), por isso dá para testar de forma simples.

use prisma_engine::SIZE;

/// Margem em volta do tabuleiro: 3% do menor lado da tela, de cada lado.
const MARGEM: f32 = 0.03;

/// Um retângulo na tela, em pixels: canto de cima à esquerda (`x`, `y`) e tamanho (`w`, `h`).
///
/// `f32` é o número com vírgula do Rust (32 bits), o tipo que o Macroquad usa para desenhar.
/// O `derive` pede ao compilador para criar sozinho cópia, impressão (`{:?}`) e comparação.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// Um retângulo sem tamanho, no canto da tela: o que devolvemos quando a tela não existe.
    const VAZIO: Rect = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
}

/// O quadrado do tabuleiro: centrado na tela, com lado igual ao menor lado da tela
/// menos a margem. Funciona igual em pé (retrato) e deitado (paisagem).
///
/// Uma tela "impossível" (tamanho zero, negativo, NaN ou infinito, como acontece por um
/// instante quando a janela é minimizada) devolve um retângulo vazio em vez de quebrar.
pub fn board_rect(screen_w: f32, screen_h: f32) -> Rect {
    let tela_valida = screen_w.is_finite() && screen_h.is_finite() && screen_w > 0.0 && screen_h > 0.0;
    if !tela_valida {
        return Rect::VAZIO;
    }
    let lado = screen_w.min(screen_h) * (1.0 - 2.0 * MARGEM);
    Rect {
        x: (screen_w - lado) / 2.0,
        y: (screen_h - lado) / 2.0,
        w: lado,
        h: lado,
    }
}

/// A casa da linha `row` e da coluna `col` (de 0 a `SIZE - 1`, contando do canto de cima
/// à esquerda) dentro do tabuleiro `board`. As casas se encostam, sem buracos.
pub fn cell_rect(board: Rect, row: usize, col: usize) -> Rect {
    let w = board.w / SIZE as f32;
    let h = board.h / SIZE as f32;
    Rect {
        x: board.x + col as f32 * w,
        y: board.y + row as f32 * h,
        w,
        h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prisma_engine::SIZE;

    /// Compara números decimais (`f32`) com uma folguinha, porque contas com vírgula
    /// raramente dão o valor exato.
    fn perto(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn tabuleiro_e_quadrado_e_cabe_na_tela() {
        for (largura, altura) in [(540.0, 960.0), (960.0, 540.0), (1080.0, 2400.0), (400.0, 400.0)] {
            let b = board_rect(largura, altura);
            assert!(perto(b.w, b.h), "tela {largura}x{altura}: {b:?} não é quadrado");
            assert!(b.x >= 0.0 && b.y >= 0.0, "tela {largura}x{altura}: {b:?} sai pela esquerda/topo");
            assert!(b.x + b.w <= largura + 0.01, "tela {largura}x{altura}: {b:?} sai pela direita");
            assert!(b.y + b.h <= altura + 0.01, "tela {largura}x{altura}: {b:?} sai por baixo");
        }
    }

    #[test]
    fn tabuleiro_fica_centralizado() {
        let b = board_rect(540.0, 960.0);
        assert!(perto(b.x + b.w / 2.0, 270.0), "centro horizontal errado: {b:?}");
        assert!(perto(b.y + b.h / 2.0, 480.0), "centro vertical errado: {b:?}");
    }

    #[test]
    fn retrato_e_paisagem_usam_o_menor_lado() {
        let retrato = board_rect(540.0, 960.0);
        let paisagem = board_rect(960.0, 540.0);
        assert!(perto(retrato.w, paisagem.w));
        // 540 menos 3% de margem de cada lado = 540 × 0,94.
        assert!(perto(retrato.w, 507.6), "lado esperado 507,6, veio {}", retrato.w);
    }

    #[test]
    fn casas_ladrilham_o_tabuleiro() {
        let b = board_rect(540.0, 960.0);
        let primeira = cell_rect(b, 0, 0);
        assert!(perto(primeira.x, b.x) && perto(primeira.y, b.y));
        let ultima = cell_rect(b, SIZE - 1, SIZE - 1);
        assert!(perto(ultima.x + ultima.w, b.x + b.w));
        assert!(perto(ultima.y + ultima.h, b.y + b.h));
        for linha in 0..SIZE {
            for coluna in 0..SIZE {
                let casa = cell_rect(b, linha, coluna);
                assert!(perto(casa.w, b.w / SIZE as f32), "largura da casa ({linha},{coluna})");
                assert!(perto(casa.h, b.h / SIZE as f32), "altura da casa ({linha},{coluna})");
                if coluna + 1 < SIZE {
                    let direita = cell_rect(b, linha, coluna + 1);
                    assert!(perto(casa.x + casa.w, direita.x), "buraco entre ({linha},{coluna}) e a da direita");
                }
                if linha + 1 < SIZE {
                    let abaixo = cell_rect(b, linha + 1, coluna);
                    assert!(perto(casa.y + casa.h, abaixo.y), "buraco entre ({linha},{coluna}) e a de baixo");
                }
            }
        }
    }

    #[test]
    fn tela_degenerada_nao_quebra() {
        for (largura, altura) in [
            (0.0, 0.0),
            (1.0, 1.0),
            (-5.0, 100.0),
            (f32::NAN, 100.0),
            (100.0, f32::NAN),
            (f32::INFINITY, 100.0),
            (100_000.0, 1.0),
        ] {
            let b = board_rect(largura, altura);
            let tudo = [b.x, b.y, b.w, b.h];
            assert!(tudo.iter().all(|n| n.is_finite()), "tela {largura}x{altura}: {b:?} tem NaN ou infinito");
            assert!(b.w >= 0.0 && b.h >= 0.0, "tela {largura}x{altura}: {b:?} com tamanho negativo");
            if b.w > 0.0 {
                // Quando sobra tabuleiro, ele precisa caber na tela.
                assert!(b.x >= 0.0 && b.y >= 0.0, "tela {largura}x{altura}: {b:?} fora da tela");
                assert!(b.x + b.w <= largura + 0.01 && b.y + b.h <= altura + 0.01, "tela {largura}x{altura}: {b:?} fora da tela");
            }
            // As casas de um tabuleiro degenerado também não podem ter NaN.
            let casa = cell_rect(b, SIZE - 1, SIZE - 1);
            assert!([casa.x, casa.y, casa.w, casa.h].iter().all(|n| n.is_finite()));
        }
    }
}
