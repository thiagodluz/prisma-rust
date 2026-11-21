//! Guarda do projeto: confere como os arquivos `Cargo.toml` estão escritos.
//!
//! Este teste não olha para o jogo em si. Ele existe por causa do Android: a ferramenta que
//! gera o APK (`cargo-quad-apk`) carrega um Cargo antigo por dentro e não entende a
//! "edition" 2024 do Rust. Se alguém (eu, no futuro) subir a edition sem lembrar disso,
//! o erro só apareceria no primeiro envio para o GitHub. Aqui ele aparece já no `cargo test`.

use std::fs;
use std::path::Path;

/// Lê um arquivo de texto relativo à pasta raiz do projeto.
///
/// `env!("CARGO_MANIFEST_DIR")` é preenchido pelo compilador com o caminho da pasta onde
/// está o `Cargo.toml` deste pacote, então o teste funciona de qualquer lugar.
fn ler(caminho: &str) -> String {
    let completo = Path::new(env!("CARGO_MANIFEST_DIR")).join(caminho);
    fs::read_to_string(&completo)
        .unwrap_or_else(|erro| panic!("não consegui ler {}: {erro}", completo.display()))
}

#[test]
fn nenhum_cargo_toml_usa_edition_2024() {
    for arquivo in ["Cargo.toml", "game/Cargo.toml"] {
        let texto = ler(arquivo);
        assert!(
            !texto.contains("edition = \"2024\""),
            "{arquivo} usa edition 2024, que o cargo-quad-apk (gerador de APK) não lê. \
             Use edition = \"2021\"."
        );
        assert!(
            texto.contains("edition = \"2021\""),
            "{arquivo} deveria declarar edition = \"2021\" (exigência do cargo-quad-apk)."
        );
    }
}
