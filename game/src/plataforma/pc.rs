//! Versão do PC da camada da plataforma.

use std::path::PathBuf;

use crate::contador::pasta_de_dados_no_pc;

/// Pasta onde o jogo guarda seus arquivos no PC (veja `pasta_de_dados_no_pc`).
pub fn pasta_de_dados() -> Result<PathBuf, String> {
    let variavel = |nome: &str| std::env::var(nome).ok();
    pasta_de_dados_no_pc(
        variavel("APPDATA").as_deref(),
        variavel("XDG_DATA_HOME").as_deref(),
        variavel("HOME").as_deref(),
    )
    .ok_or_else(|| {
        "não achei a pasta de dados do usuário (faltam APPDATA, XDG_DATA_HOME e HOME)".to_string()
    })
}

/// No PC não há o que vibrar: finge que deu certo.
pub fn vibrar(_milissegundos: u32) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vibrar_no_pc_nao_da_erro() {
        assert_eq!(vibrar(200), Ok(()));
    }
}
