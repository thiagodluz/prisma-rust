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

/// Pega o valor de uma linha `chave = "valor"` do `Cargo.toml` (só serve para linhas simples).
fn valor_da_chave(texto: &str, chave: &str) -> String {
    let inicio = format!("{chave} = \"");
    texto
        .lines()
        .find_map(|linha| linha.trim_start().strip_prefix(inicio.as_str()))
        .and_then(|resto| resto.split('"').next())
        .unwrap_or_else(|| panic!("não achei {chave} = \"...\" no game/Cargo.toml"))
        .to_owned()
}

/// O cargo-quad-apk procura a biblioteca do app pelo ÚLTIMO pedaço do `package_name`
/// (em `com.thiagodluz.prisma.rust` seria `librust.so`), mas o arquivo é gravado com o nome
/// do programa (`libprisma_game.so`). Se os dois não forem iguais, o APK instala mas o app
/// fecha na hora com `library "librust.so" not found`.
#[test]
fn ultimo_pedaco_do_package_name_e_o_nome_do_programa() {
    let texto = ler("game/Cargo.toml");
    let programa = valor_da_chave(&texto, "name");
    let package_name = valor_da_chave(&texto, "package_name");
    let ultimo_pedaco = package_name.rsplit('.').next().unwrap();
    assert_eq!(
        ultimo_pedaco, programa,
        "package_name = \"{package_name}\" termina em \"{ultimo_pedaco}\", mas o programa se \
         chama \"{programa}\". O app fecharia ao abrir no Android (librust.so not found)."
    );
}
