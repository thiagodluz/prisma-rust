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

/// O trecho de `texto` que vai da linha `cabecalho` (por exemplo `[package.metadata.android]`)
/// até a linha anterior ao próximo cabeçalho `[...]`. Serve para conferir em que seção do
/// `Cargo.toml` uma linha está.
fn secao<'a>(texto: &'a str, cabecalho: &str) -> &'a str {
    let mut inicio = None;
    let mut posicao = 0;
    for linha in texto.split_inclusive('\n') {
        match inicio {
            None if linha.trim() == cabecalho => inicio = Some(posicao),
            Some(comeco) if linha.trim_start().starts_with('[') => return &texto[comeco..posicao],
            _ => {}
        }
        posicao += linha.len();
    }
    match inicio {
        Some(comeco) => &texto[comeco..],
        None => panic!("não achei a seção {cabecalho}"),
    }
}

/// As músicas e os ambientes são preparados na pasta `game/sons` e o gerador de APK a leva para
/// dentro do APK (`assets/musica/...`, `assets/ambiente/...`). Sem esta linha o APK sai sem
/// som de fundo e o jogo só mostra o aviso de que o áudio não começou.
#[test]
fn os_sons_vao_como_assets_do_android() {
    let texto = ler("game/Cargo.toml");
    let android = secao(&texto, "[package.metadata.android]");
    assert!(
        android.lines().any(|linha| linha.trim() == "assets = \"sons\""),
        "falta `assets = \"sons\"` em [package.metadata.android] do game/Cargo.toml: o APK sairia sem as músicas e os ambientes."
    );
}

/// O Macroquad descompacta o OGG inteiro ao carregar. Em modo de desenvolvimento (que é o que o
/// CI usa para o APK) isso leva uns 5 s por música sem otimização; com as dependências
/// otimizadas, bem menos. O código do jogo continua em modo de desenvolvimento (fácil de depurar).
#[test]
fn dependencias_otimizadas_no_modo_de_desenvolvimento() {
    let texto = ler("Cargo.toml");
    let secao_do_perfil = secao(&texto, "[profile.dev.package.\"*\"]");
    assert!(
        secao_do_perfil.lines().any(|linha| linha.trim() == "opt-level = 2"),
        "falta `opt-level = 2` em [profile.dev.package.\"*\"] do Cargo.toml da raiz: carregar uma música ficaria lento no celular."
    );
}

/// Os sons são recriados por `prisma_tools audio` e nunca entram no git (a spec pede isso).
#[test]
fn a_pasta_de_sons_nao_entra_no_git() {
    let texto = ler(".gitignore");
    assert!(
        texto.lines().any(|linha| linha.trim().trim_start_matches('/') == "game/sons/"),
        ".gitignore deveria ignorar game/sons/ (os sons são gerados por `prisma_tools audio`)."
    );
}

/// As nove obras de terceiros precisam estar citadas, e o Ruído suave (o único CC BY, que exige
/// atribuição) com a licença certa.
#[test]
fn os_creditos_de_audio_citam_os_9_trabalhos_e_a_licenca_cc_by() {
    let texto = ler("docs/CREDITOS-AUDIO.md");
    let obras = [
        "Magic Puzzle In-Game 1",
        "Cozy Puzzle In-Game 3",
        "Space City",
        "Soft Rain Loop",
        "cicadas.wav",
        "Rain Sound and Rainforest.mp3",
        "creek loop",
        "distant rain and thunder",
        "Gentle White Noise Air Tone",
    ];
    for obra in obras {
        assert!(texto.contains(obra), "docs/CREDITOS-AUDIO.md não cita a obra \"{obra}\".");
    }
    assert!(texto.contains("CC BY 4.0"), "docs/CREDITOS-AUDIO.md não cita a licença CC BY 4.0 do Ruído suave.");
    assert!(texto.contains("Geoff Bremner"), "docs/CREDITOS-AUDIO.md não atribui o Ruído suave a Geoff Bremner.");
}

/// O fluxo do Android tem de preparar os sons antes de gerar o APK e reprovar um APK sem os nove
/// arquivos ou grande demais. Aqui só se confere que os passos existem e citam os nove arquivos;
/// quem os executa é o CI.
#[test]
fn o_fluxo_do_android_prepara_os_sons_e_confere_o_apk() {
    let texto = ler(".github/workflows/android.yml");
    for trecho in ["Preparar os sons", "audio game/sons", "60 * 1024 * 1024"] {
        assert!(texto.contains(trecho), ".github/workflows/android.yml não tem \"{trecho}\".");
    }
    let sons = [
        "musica/magic-puzzle",
        "musica/cozy-puzzle",
        "musica/space-city",
        "ambiente/stream",
        "ambiente/rain-soft",
        "ambiente/forest-cicadas",
        "ambiente/rainforest",
        "ambiente/rain-thunder",
        "ambiente/soft-noise",
    ];
    for som in sons {
        assert!(texto.contains(som), ".github/workflows/android.yml não confere o som {som} dentro do APK.");
    }
}

/// O texto de um job do fluxo: da linha `  nome:` (dois espaços) até o próximo job.
fn job<'a>(texto: &'a str, nome: &str) -> &'a str {
    let cabecalho = format!("  {nome}:");
    let mut inicio = None;
    let mut posicao = 0;
    for linha in texto.split_inclusive('\n') {
        let e_job = linha.starts_with("  ") && !linha.starts_with("   ") && linha.trim_end().ends_with(':');
        match inicio {
            None if linha.trim_end() == cabecalho => inicio = Some(posicao),
            Some(comeco) if e_job => return &texto[comeco..posicao],
            _ => {}
        }
        posicao += linha.len();
    }
    match inicio {
        Some(comeco) => &texto[comeco..],
        None => panic!("o fluxo não tem o job `{nome}`."),
    }
}

/// O APK só vai para o endereço fixo da release depois que os emuladores passaram, e a release
/// antiga nunca é apagada antes da nova estar pronta (senão o link some se algo falhar no meio).
/// Aqui só se confere a forma do fluxo; quem o executa é o CI.
#[test]
fn o_fluxo_do_android_so_publica_depois_dos_emuladores_e_sem_apagar_a_release() {
    let texto = ler(".github/workflows/android.yml");
    let publicar = job(&texto, "publicar");
    assert!(publicar.contains("needs: [apk, emulador]"), "o job `publicar` tem de esperar o apk e os emuladores.");
    assert!(publicar.contains("refs/heads/main"), "só a main publica.");
    assert!(publicar.contains("--clobber"), "o APK novo substitui o arquivo da release sem apagar a release.");
    assert!(!texto.contains("gh release delete"), "apagar a release deixa o link fora do ar se a criação falhar.");
    let apk = job(&texto, "apk");
    assert!(!apk.contains("gh release"), "o job `apk` não pode mexer na release: os emuladores ainda não rodaram.");
}

/// O Android mínimo e o alvo ficam fixos: 26 (o ícone adaptativo e o vibrador novo valem a partir
/// daí) e 34. As duas chaves têm de estar na seção do gerador de APK, senão ele as ignora.
#[test]
fn as_versoes_do_android_estao_fixadas() {
    let texto = ler("game/Cargo.toml");
    let android = secao(&texto, "[package.metadata.android]");
    for linha in ["min_sdk_version = 26", "target_sdk_version = 34"] {
        assert!(
            android.lines().any(|atual| atual.trim() == linha),
            "game/Cargo.toml não tem \"{linha}\" dentro de [package.metadata.android]."
        );
    }
}

/// Depois de gerar o APK, o fluxo confere o que o Android vai ler dele: o mínimo, o alvo e o
/// ícone adaptativo. Aqui só se confere que o passo existe e cita as três coisas; quem o executa
/// é o CI.
#[test]
fn o_fluxo_do_android_confere_versoes_e_icone() {
    let texto = ler(".github/workflows/android.yml");
    for trecho in ["Conferir versões e ícone do APK", "minSdkVersion", "targetSdkVersion", "anydpi"] {
        assert!(texto.contains(trecho), ".github/workflows/android.yml não tem \"{trecho}\".");
    }
    // O mínimo e o alvo esperados aparecem na conferência.
    for trecho in ["'26'", "'34'"] {
        assert!(texto.contains(trecho), ".github/workflows/android.yml não confere o valor {trecho}.");
    }
}
