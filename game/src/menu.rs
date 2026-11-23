//! O que a folha do menu ≡ escreve: os Recordes e os Créditos. Só texto, sem desenho.
//!
//! O texto dos créditos mora aqui, num lugar só (e um teste confere que ele cita o que as
//! licenças exigem); o desenho vem de `render.rs`.

use crate::app::formatar_numero;
use crate::formato::Recordes;
use prisma_engine::Mode;

/// Uma linha da lista de recordes: o nome e o valor já formatado ("1.234").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinhaDeRecorde {
    pub rotulo: &'static str,
    pub valor: String,
}

/// As linhas de recordes de um modo: no Zen, Maior pontuação, Maior nível e Melhor jogada; no
/// Clássico, essas e mais Partidas concluídas (só o Clássico termina partidas).
pub fn linhas_de_recordes(modo: Mode, recordes: Recordes) -> Vec<LinhaDeRecorde> {
    let linha = |rotulo, valor: u64| LinhaDeRecorde { rotulo, valor: formatar_numero(valor) };
    let mut linhas = vec![
        linha("Maior pontuação", recordes.pontuacao),
        linha("Maior nível", u64::from(recordes.nivel)),
        linha("Melhor jogada", recordes.jogada),
    ];
    if modo == Mode::Classic {
        linhas.push(linha("Partidas concluídas", u64::from(recordes.terminadas)));
    }
    linhas
}

/// Um bloco da folha de Recordes: o título ("Zen" ou "Clássico") e as linhas daquele modo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlocoDeRecordes {
    pub titulo: &'static str,
    pub linhas: Vec<LinhaDeRecorde>,
}

/// Os dois blocos da folha de Recordes, o do Zen e depois o do Clássico, cada um com os recordes do
/// seu modo (como a lista "Ver recordes" do Prisma em JS, que mostra os dois ao mesmo tempo).
pub fn blocos_de_recordes(zen: Recordes, classico: Recordes) -> Vec<BlocoDeRecordes> {
    vec![
        BlocoDeRecordes { titulo: "Zen", linhas: linhas_de_recordes(Mode::Zen, zen) },
        BlocoDeRecordes { titulo: "Clássico", linhas: linhas_de_recordes(Mode::Classic, classico) },
    ]
}

/// O texto dos créditos, uma linha (ou frase) por item, sem links (não há navegador a abrir).
/// `versao` é a versão do pacote (`CARGO_PKG_VERSION`).
pub fn creditos(versao: &str) -> Vec<String> {
    vec![
        format!("Prisma (Rust) · versão {versao}"),
        "Jogo original criado por Thiago Luz (2026).".to_string(),
        "Versões derivadas devem preservar os créditos e identificar suas alterações.".to_string(),
        "Código: GPLv3. Arte das pedras e fundo: CC BY-SA 4.0. Nome e ícone não integram a licença da arte.".to_string(),
        "Ruído suave: \"Gentle White Noise Air Tone\", Geoff Bremner, CC BY 4.0.".to_string(),
        "Demais músicas e ambientes: CC0. Autores em docs/CREDITOS-AUDIO.md, no repositório thiagodluz/prisma-rust.".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rotulos(linhas: &[LinhaDeRecorde]) -> Vec<&'static str> {
        linhas.iter().map(|linha| linha.rotulo).collect()
    }

    #[test]
    fn zen_tem_tres_linhas_e_classico_quatro() {
        let zen = linhas_de_recordes(Mode::Zen, Recordes::zerados());
        assert_eq!(rotulos(&zen), vec!["Maior pontuação", "Maior nível", "Melhor jogada"]);
        let classico = linhas_de_recordes(Mode::Classic, Recordes::zerados());
        assert_eq!(rotulos(&classico), vec!["Maior pontuação", "Maior nível", "Melhor jogada", "Partidas concluídas"]);
    }

    #[test]
    fn a_folha_de_recordes_tem_os_dois_blocos_cada_um_com_os_seus_numeros() {
        let zen = Recordes { pontuacao: 2_025, nivel: 2, jogada: 718, terminadas: 0 };
        let classico = Recordes { pontuacao: 9_000, nivel: 7, jogada: 450, terminadas: 3 };
        let blocos = blocos_de_recordes(zen, classico);
        let titulos: Vec<&str> = blocos.iter().map(|bloco| bloco.titulo).collect();
        assert_eq!(titulos, vec!["Zen", "Clássico"], "os dois modos aparecem, o Zen primeiro");
        let valores = |i: usize| blocos[i].linhas.iter().map(|linha| linha.valor.clone()).collect::<Vec<_>>();
        assert_eq!(valores(0), vec!["2.025", "2", "718"], "o bloco do Zen lê os recordes do Zen");
        assert_eq!(valores(1), vec!["9.000", "7", "450", "3"], "o bloco do Clássico lê os do Clássico");
        assert_eq!(blocos[0].linhas.len(), 3);
        assert_eq!(blocos[1].linhas.len(), 4);
    }

    #[test]
    fn valores_usam_ponto_de_milhar() {
        let recordes = Recordes { pontuacao: 1_234_567, nivel: 12, jogada: 9_800, terminadas: 1_000 };
        let zen = linhas_de_recordes(Mode::Zen, recordes);
        let valores: Vec<&str> = zen.iter().map(|linha| linha.valor.as_str()).collect();
        assert_eq!(valores, vec!["1.234.567", "12", "9.800"]);
        let classico = linhas_de_recordes(Mode::Classic, recordes);
        assert_eq!(classico[3].valor, "1.000", "partidas concluídas também leva ponto de milhar");
    }

    #[test]
    fn recordes_zerados_mostram_0_e_nivel_1() {
        let classico = linhas_de_recordes(Mode::Classic, Recordes::zerados());
        let valores: Vec<&str> = classico.iter().map(|linha| linha.valor.as_str()).collect();
        assert_eq!(valores, vec!["0", "1", "0", "0"]);
    }

    #[test]
    fn creditos_citam_o_que_a_licenca_exige() {
        let texto = creditos("1.2.3").join("\n");
        for esperado in ["GPLv3", "CC BY-SA 4.0", "CC BY 4.0", "Geoff Bremner", "Thiago Luz", "CC0", "CREDITOS-AUDIO.md", "1.2.3"] {
            assert!(texto.contains(esperado), "os créditos não citam {esperado:?}:\n{texto}");
        }
        // A primeira linha leva o nome do jogo e a versão.
        assert_eq!(creditos("1.2.3")[0], "Prisma (Rust) · versão 1.2.3");
    }

    #[test]
    fn creditos_nao_tem_link() {
        for linha in creditos("9.9.9") {
            assert!(!linha.contains("http"), "link num crédito: {linha}");
            assert!(!linha.contains("www."), "link num crédito: {linha}");
        }
    }

    #[test]
    fn creditos_sao_poucas_linhas() {
        let linhas = creditos("9.9.9");
        assert!(linhas.len() <= 10, "{} linhas", linhas.len());
        for linha in &linhas {
            assert!(!linha.trim().is_empty(), "linha vazia");
            assert!(linha.chars().count() <= 120, "linha longa ({}): {linha}", linha.chars().count());
        }
    }
}
