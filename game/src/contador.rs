//! Contador de aberturas do app, gravado num arquivo de texto.
//!
//! É o teste do M0b para "gravar arquivo no Android": a cada abertura soma 1 ao número
//! guardado e grava de volta. O arquivo fica na pasta de dados do app (no Android, a pasta
//! privada que o sistema dá ao app; no PC, uma pasta do usuário).

use std::fs;
use std::path::{Path, PathBuf};

/// Nome do arquivo do contador, dentro da pasta de dados.
pub const NOME_ARQUIVO: &str = "contador.txt";

/// Lê o número guardado no arquivo. Texto vazio, com letras, negativo, com vírgula ou
/// grande demais para caber num `u32` vale 0: o contador simplesmente recomeça.
pub fn interpretar(texto: &str) -> u32 {
    // `parse::<u32>` só aceita dígitos de 0 a 9 e já recusa sinal negativo e estouro.
    texto.trim().parse::<u32>().unwrap_or(0)
}

/// Conta mais uma abertura: lê o número do arquivo (se não existir, parte de 0), soma 1,
/// grava e devolve o novo valor. Se o número já estiver no máximo, fica nele em vez de
/// "dar a volta" para 0.
///
/// A gravação é feita em duas etapas (arquivo temporário e depois troca de nome), para que
/// uma queda no meio nunca deixe o `contador.txt` pela metade.
pub fn registrar_abertura(pasta: &Path) -> Result<u32, String> {
    fs::create_dir_all(pasta)
        .map_err(|erro| format!("não consegui criar a pasta {}: {erro}", pasta.display()))?;

    let arquivo = pasta.join(NOME_ARQUIVO);
    // Sem arquivo (primeira abertura) ou com arquivo ilegível (bytes que não são texto),
    // o contador parte de 0.
    let anterior = fs::read_to_string(&arquivo).map(|texto| interpretar(&texto)).unwrap_or(0);
    let novo = anterior.saturating_add(1);

    let temporario = pasta.join(format!("{NOME_ARQUIVO}.tmp"));
    fs::write(&temporario, novo.to_string())
        .map_err(|erro| format!("não consegui gravar {}: {erro}", temporario.display()))?;
    fs::rename(&temporario, &arquivo)
        .map_err(|erro| format!("não consegui trocar o nome para {}: {erro}", arquivo.display()))?;
    Ok(novo)
}

/// Pasta de dados do app no PC, a partir das variáveis de ambiente (passadas de fora para
/// o teste poder dar valores falsos). Ordem: `APPDATA` (Windows), `XDG_DATA_HOME` (Linux),
/// `HOME` + `.local/share`. Texto vazio conta como ausente. No Mac cai no caso do `HOME`.
pub fn pasta_de_dados_no_pc(
    appdata: Option<&str>,
    xdg: Option<&str>,
    home: Option<&str>,
) -> Option<PathBuf> {
    fn valida(valor: Option<&str>) -> Option<&str> {
        valor.filter(|texto| !texto.is_empty())
    }
    if let Some(base) = valida(appdata) {
        return Some(PathBuf::from(base).join("prisma-rust"));
    }
    if let Some(base) = valida(xdg) {
        return Some(PathBuf::from(base).join("prisma-rust"));
    }
    valida(home).map(|casa| PathBuf::from(casa).join(".local").join("share").join("prisma-rust"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Pasta de teste só deste teste: apaga o que sobrou de uma rodada anterior e some
    /// sozinha quando o teste termina (o `Drop` abaixo).
    struct PastaTemporaria(PathBuf);

    impl PastaTemporaria {
        fn nova(nome: &str) -> Self {
            let caminho = std::env::temp_dir()
                .join(format!("prisma-teste-{}-{nome}", std::process::id()));
            let _ = fs::remove_dir_all(&caminho);
            PastaTemporaria(caminho)
        }

        fn caminho(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for PastaTemporaria {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn interpretar_aceita_numero_com_espacos_e_quebra_de_linha() {
        assert_eq!(interpretar("7\n"), 7);
        assert_eq!(interpretar(" 12 "), 12);
    }

    #[test]
    fn interpretar_devolve_zero_para_vazio_lixo_negativo_e_estouro() {
        for texto in ["", "abc", "-3", "1.5", "99999999999999999999", "٣"] {
            assert_eq!(interpretar(texto), 0, "texto {texto:?} deveria virar 0");
        }
    }

    #[test]
    fn registrar_abertura_comeca_em_1_e_conta_a_cada_chamada() {
        let pasta = PastaTemporaria::nova("conta");
        assert_eq!(registrar_abertura(pasta.caminho()), Ok(1));
        assert_eq!(registrar_abertura(pasta.caminho()), Ok(2));
        assert_eq!(registrar_abertura(pasta.caminho()), Ok(3));
        let gravado = fs::read_to_string(pasta.caminho().join(NOME_ARQUIVO)).unwrap();
        assert_eq!(gravado.trim(), "3");
    }

    #[test]
    fn registrar_abertura_recomeca_de_1_com_arquivo_corrompido() {
        let pasta = PastaTemporaria::nova("corrompido");
        fs::create_dir_all(pasta.caminho()).unwrap();
        fs::write(pasta.caminho().join(NOME_ARQUIVO), "lixo").unwrap();
        assert_eq!(registrar_abertura(pasta.caminho()), Ok(1));
    }

    #[test]
    fn registrar_abertura_satura_em_vez_de_dar_a_volta() {
        let pasta = PastaTemporaria::nova("satura");
        fs::create_dir_all(pasta.caminho()).unwrap();
        fs::write(pasta.caminho().join(NOME_ARQUIVO), "4294967295").unwrap();
        assert_eq!(registrar_abertura(pasta.caminho()), Ok(4_294_967_295));
    }

    #[test]
    fn registrar_abertura_cria_a_pasta_se_nao_existir() {
        let pasta = PastaTemporaria::nova("cria");
        let funda = pasta.caminho().join("um").join("dois");
        assert_eq!(registrar_abertura(&funda), Ok(1));
    }

    #[test]
    fn registrar_abertura_devolve_erro_quando_a_pasta_e_um_arquivo() {
        let pasta = PastaTemporaria::nova("arquivo-no-lugar");
        fs::create_dir_all(pasta.caminho()).unwrap();
        let arquivo = pasta.caminho().join("sou-um-arquivo");
        fs::write(&arquivo, "x").unwrap();
        let erro = registrar_abertura(&arquivo).unwrap_err();
        assert!(!erro.is_empty());
    }

    #[test]
    fn registrar_abertura_nao_deixa_arquivo_temporario() {
        let pasta = PastaTemporaria::nova("sem-temporario");
        registrar_abertura(pasta.caminho()).unwrap();
        registrar_abertura(pasta.caminho()).unwrap();
        let mut nomes: Vec<String> = fs::read_dir(pasta.caminho())
            .unwrap()
            .map(|entrada| entrada.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        nomes.sort();
        assert_eq!(nomes, vec![NOME_ARQUIVO.to_string()]);
    }

    #[test]
    fn pasta_de_dados_no_pc_prefere_appdata_depois_xdg_depois_home() {
        assert_eq!(
            pasta_de_dados_no_pc(Some("C:/A"), Some("/x/share"), Some("/home/t")),
            Some(PathBuf::from("C:/A").join("prisma-rust"))
        );
        assert_eq!(
            pasta_de_dados_no_pc(None, Some("/x/share"), Some("/home/t")),
            Some(PathBuf::from("/x/share").join("prisma-rust"))
        );
        assert_eq!(
            pasta_de_dados_no_pc(None, None, Some("/home/t")),
            Some(PathBuf::from("/home/t").join(".local").join("share").join("prisma-rust"))
        );
        assert_eq!(pasta_de_dados_no_pc(None, None, None), None);
        // Texto vazio conta como se a variável não existisse.
        assert_eq!(
            pasta_de_dados_no_pc(Some(""), Some(""), Some("/home/t")),
            Some(PathBuf::from("/home/t").join(".local").join("share").join("prisma-rust"))
        );
    }
}
