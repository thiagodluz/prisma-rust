//! O texto do jogo: Inter (texto e texto forte) e Sora (títulos e números grandes), embutidas.
//!
//! Toda letra que o jogo desenha passa por aqui (`medir` e `desenhar`), para a fonte ser uma só.
//! Antes de `carregar` (ou se ela falhar) vale a fonte padrão do Macroquad, então nada quebra.

use macroquad::prelude::*;
use std::cell::RefCell;

// Estáticas e reduzidas ao Latim (ver `docs/notes/visual.md`).
const TEXTO: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");
const TEXTO_FORTE: &[u8] = include_bytes!("../assets/Inter-SemiBold.ttf");
const TITULO: &[u8] = include_bytes!("../assets/Sora-Bold.ttf");

/// O estilo da letra: texto comum, texto forte (rótulos e botões) e título (marca, números grandes
/// e títulos de caixas).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estilo {
    Texto,
    TextoForte,
    Titulo,
}

thread_local! {
    static FONTES: RefCell<Option<[Font; 3]>> = const { RefCell::new(None) };
}

/// Lê as três fontes embutidas. Chamada uma vez, depois que a janela abriu. Se alguma falhar,
/// nenhuma é guardada e vale a fonte padrão do Macroquad.
pub fn carregar() {
    let lidas = [TEXTO, TEXTO_FORTE, TITULO].map(load_ttf_font_from_bytes);
    if let [Ok(texto), Ok(forte), Ok(titulo)] = lidas {
        FONTES.with(|fontes| *fontes.borrow_mut() = Some([texto, forte, titulo]));
    }
}

fn com_fonte<R>(estilo: Estilo, usar: impl FnOnce(Option<&Font>) -> R) -> R {
    FONTES.with(|fontes| {
        let fontes = fontes.borrow();
        usar(fontes.as_ref().map(|fontes| &fontes[estilo as usize]))
    })
}

/// As medidas de `texto` no `tamanho` (em pixels) e no `estilo` dados.
pub fn medir(texto: &str, tamanho: f32, estilo: Estilo) -> TextDimensions {
    com_fonte(estilo, |fonte| measure_text(texto, fonte, tamanho as u16, 1.0))
}

/// Desenha `texto` com a linha de base em (`x`, `y`).
pub fn desenhar(texto: &str, x: f32, y: f32, tamanho: f32, cor: Color, estilo: Estilo) {
    com_fonte(estilo, |fonte| {
        draw_text_ex(texto, x, y, TextParams { font: fonte, font_size: tamanho as u16, color: cor, ..Default::default() });
    });
}

/// O tamanho de letra que faz um texto de `largura_do_texto` (medida em `tamanho`) caber em
/// `largura_da_area`: o mesmo tamanho se já cabe, menor (na mesma proporção) se não cabe, e 0 se a
/// área não tem largura ou algum número não vale.
pub fn tamanho_que_cabe(largura_do_texto: f32, largura_da_area: f32, tamanho: f32) -> f32 {
    if largura_da_area.is_nan() || largura_da_area <= 0.0 || !largura_do_texto.is_finite() || !tamanho.is_finite() {
        return 0.0;
    }
    if largura_do_texto <= largura_da_area {
        tamanho
    } else {
        tamanho * largura_da_area / largura_do_texto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_fontes_embutidas_sao_estaticas() {
        for (nome, bytes) in [("Inter Regular", TEXTO), ("Inter SemiBold", TEXTO_FORTE), ("Sora Bold", TITULO)] {
            assert_eq!(&bytes[..4], &[0x00, 0x01, 0x00, 0x00], "{nome} tem de ser TrueType");
            assert!(bytes.len() > 10_000 && bytes.len() < 300_000, "{nome}: tamanho estranho ({})", bytes.len());
            assert!(!bytes.windows(4).any(|janela| janela == b"fvar"), "{nome} não pode ser a variável");
        }
    }

    /// Os caracteres fora do ASCII que aparecem dentro de aspas no código de produção (antes do
    /// `#[cfg(test)]`) dos arquivos que escrevem texto na tela. O ASCII todo existe nas fontes.
    fn caracteres_de_texto_do_jogo() -> Vec<(&'static str, char)> {
        let arquivos = [
            ("render.rs", include_str!("render.rs")),
            ("menu.rs", include_str!("menu.rs")),
            ("app.rs", include_str!("app.rs")),
            ("cenas.rs", include_str!("cenas.rs")),
            ("preferencias.rs", include_str!("preferencias.rs")),
            ("fundo.rs", include_str!("fundo.rs")),
            ("layout.rs", include_str!("layout.rs")),
        ];
        let mut achados = Vec::new();
        for (nome, fonte) in arquivos {
            let producao = fonte.split("#[cfg(test)]").next().unwrap_or(fonte);
            for linha in producao.lines() {
                let mut dentro = false;
                let mut anterior = ' ';
                for c in linha.chars() {
                    if c == '"' && anterior != '\\' {
                        dentro = !dentro;
                    } else if !dentro && c == '/' && anterior == '/' {
                        break; // comentário até o fim da linha
                    } else if dentro && !c.is_ascii() && !achados.contains(&(nome, c)) {
                        achados.push((nome, c));
                    }
                    anterior = c;
                }
            }
        }
        achados
    }

    #[test]
    fn todo_texto_do_jogo_tem_glifo_nas_fontes_embutidas() {
        let fontes: Vec<(&str, fontdue::Font)> = [("Inter Regular", TEXTO), ("Inter SemiBold", TEXTO_FORTE), ("Sora Bold", TITULO)]
            .into_iter()
            .map(|(nome, bytes)| (nome, fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).unwrap()))
            .collect();
        let caracteres = caracteres_de_texto_do_jogo();
        assert!(caracteres.len() > 5, "a varredura achou pouca coisa: {caracteres:?}");
        for (arquivo, c) in caracteres {
            for (nome, fonte) in &fontes {
                assert!(fonte.lookup_glyph_index(c) != 0, "{arquivo}: '{c}' (U+{:04X}) não existe na fonte {nome}", c as u32);
            }
        }
    }

    #[test]
    fn a_varredura_pega_um_glifo_que_as_fontes_nao_tem() {
        let fonte = fontdue::Font::from_bytes(TEXTO, fontdue::FontSettings::default()).unwrap();
        for ausente in ['♫', '≡'] {
            assert_eq!(fonte.lookup_glyph_index(ausente), 0, "{ausente} devia faltar (é por isso que é desenhado)");
        }
        assert!(fonte.lookup_glyph_index('ç') != 0 && fonte.lookup_glyph_index('·') != 0);
    }

    #[test]
    fn sem_fonte_carregada_vale_a_fonte_padrao_do_macroquad() {
        // Medir e desenhar precisam da janela do Macroquad, que o teste não tem; aqui se confere o
        // que decide o caminho: sem `carregar`, nenhum estilo tem fonte própria (None = a padrão).
        for estilo in [Estilo::Texto, Estilo::TextoForte, Estilo::Titulo] {
            assert!(com_fonte(estilo, |fonte| fonte.is_none()), "{estilo:?}");
        }
    }

    #[test]
    fn tamanho_que_cabe_encolhe_so_quando_precisa() {
        assert_eq!(tamanho_que_cabe(50.0, 100.0, 20.0), 20.0);
        assert_eq!(tamanho_que_cabe(100.0, 100.0, 20.0), 20.0);
        assert!((tamanho_que_cabe(200.0, 100.0, 20.0) - 10.0).abs() < 1e-4);
        assert_eq!(tamanho_que_cabe(50.0, 0.0, 20.0), 0.0);
        assert_eq!(tamanho_que_cabe(50.0, -3.0, 20.0), 0.0);
        assert_eq!(tamanho_que_cabe(50.0, f32::NAN, 20.0), 0.0);
        assert_eq!(tamanho_que_cabe(f32::NAN, 100.0, 20.0), 0.0);
    }
}
