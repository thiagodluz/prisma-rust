//! `prisma_game`: a tela do Prisma em Rust (janela no PC, app no Android).
//!
//! As regras do jogo ficam no pacote `prisma_engine`; aqui só se desenha e se lê toque.

mod app;
mod assets;
mod cenas;
mod contador;
mod input;
mod layout;
mod linha;
mod plataforma;
mod render;
mod tom;

use crate::app::App;
use crate::input::Evento;
use macroquad::audio::{load_sound_from_bytes, play_sound_once, Sound};
// Atenção: o `info!` do miniquad não aceita `{nome}` dentro do texto; os valores vão como
// argumentos (`info!("x={}", x)`), senão a chave aparece escrita no registro.
use macroquad::logging::info;
use macroquad::prelude::*;
use prisma_engine::{lcg_random, Game, Mode, Tile, TileKind};

/// Configuração da janela. No Android o sistema escolhe o tamanho; no PC usamos 540×960,
/// uma tela de celular em pé.
fn window_conf() -> Conf {
    Conf {
        window_title: "Prisma".to_owned(),
        window_width: 540,
        window_height: 960,
        high_dpi: true,
        window_resizable: true,
        // `..Default::default()` preenche o resto da configuração com os valores padrão.
        ..Default::default()
    }
}

/// Lê o pedido de "foto" nos argumentos do programa: `--screenshot arquivo.png`.
/// Se faltar o nome do arquivo, o pedido é ignorado e o jogo abre normalmente.
fn screenshot_path(args: &[String]) -> Option<String> {
    let posicao = args.iter().position(|argumento| argumento == "--screenshot")?;
    args.get(posicao + 1).cloned()
}

/// A partida de vitrine do M0a: motor com semente fixa (sempre o mesmo tabuleiro), com
/// as primeiras linhas trocadas para mostrar todas as imagens: sete Pulsos, sete Raios e
/// um Espectro. Só serve para ver as imagens; ninguém joga com ela.
fn demo_game() -> Game {
    let mut jogo = Game::with_random(lcg_random(1));
    // Os ids das pedras novas só precisam ser diferentes entre si.
    let mut proximo_id = 1000;
    let mut nova = |kind: TileKind, color: Option<u8>| {
        proximo_id += 1;
        Tile { id: proximo_id, color, kind }
    };
    for cor in 0..7u8 {
        jogo.board[0][usize::from(cor)] = nova(TileKind::Burst, Some(cor));
        jogo.board[1][usize::from(cor)] = nova(TileKind::Cross, Some(cor));
    }
    jogo.board[2][0] = nova(TileKind::Spectrum, None);
    jogo
}

/// Qual tela abrir. No celular não há argumentos, então abre sempre o jogo.
#[derive(Debug, PartialEq)]
enum Tela {
    /// A tela de teste do M0b: contador, som e vibração (`--tela diagnostico`).
    Diagnostico,
    /// O tabuleiro de vitrine do M0a (só no PC, com `--tela vitrine`).
    Vitrine,
    /// O jogo (o padrão). Com `--cena nome` mostra um momento fixo dele, para foto.
    Jogo,
}

/// Lê `--tela vitrine|diagnostico|jogo` nos argumentos; qualquer outra coisa (ou nada) abre o jogo.
fn tela_pedida(args: &[String]) -> Tela {
    let valor = args
        .iter()
        .position(|argumento| argumento == "--tela")
        .and_then(|posicao| args.get(posicao + 1));
    match valor.map(String::as_str) {
        Some("vitrine") => Tela::Vitrine,
        Some("diagnostico") => Tela::Diagnostico,
        _ => Tela::Jogo,
    }
}

/// Lê `--cena nome` nos argumentos (um momento fixo do jogo, para tirar foto).
fn cena_pedida(args: &[String]) -> Option<String> {
    let posicao = args.iter().position(|argumento| argumento == "--cena")?;
    args.get(posicao + 1).cloned()
}

/// No modo "foto": no segundo quadro (o primeiro "aquece" a tela) grava a imagem da tela
/// num PNG e fecha o programa.
fn foto_do_quadro(foto: &Option<String>, quadro: u32) {
    if let Some(caminho) = foto {
        if quadro == 1 {
            let mut imagem = get_screen_data();
            // Peças com transparência (véus, faixas) deixam a foto com a "opacidade" do quadro
            // abaixo de 255; na tela de verdade isso é ignorado, então a foto também ignora.
            for pixel in imagem.bytes.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
            imagem.export_png(caminho);
            std::process::exit(0);
        }
    }
}

// `#[macroquad::main(...)]` transforma esta função no ponto de partida do programa.
// `async` é como o Macroquad "espera o próximo quadro" sem travar o celular: cada volta
// do `loop` desenha um quadro e o `.await` devolve o controle até o quadro seguinte.
#[macroquad::main(window_conf)]
async fn main() {
    let argumentos: Vec<String> = std::env::args().collect();
    let foto = screenshot_path(&argumentos);
    match tela_pedida(&argumentos) {
        Tela::Vitrine => rodar_vitrine(foto).await,
        Tela::Diagnostico => rodar_diagnostico(foto).await,
        Tela::Jogo => rodar_jogo(foto, cena_pedida(&argumentos)).await,
    }
}

/// O tabuleiro de vitrine do M0a.
async fn rodar_vitrine(foto: Option<String>) {
    let jogo = demo_game();
    let sprites = render::Sprites::load();

    let mut quadro = 0u32;
    loop {
        render::draw_frame(&jogo.board, &sprites);
        foto_do_quadro(&foto, quadro);
        quadro += 1;
        next_frame().await;
    }
}

/// As linhas de registro com a posição do tabuleiro e dos botões, em PIXELS REAIS da tela
/// (posição lógica × `escala`): o teste do celular virtual lê estas linhas para saber onde
/// tocar. `jogar_novamente`, `cancelar` e `confirmar` são sempre registrados, mesmo quando
/// estão escondidos; `embaralhar` só existe no Zen.
fn linhas_de_geometria(tela: &layout::TelaJogo, escala: f32) -> Vec<String> {
    let tabuleiro = tela.tabuleiro;
    let mut linhas = vec![format!(
        "PRISMA_M1 tabuleiro x={:.0} y={:.0} lado={:.0}",
        tabuleiro.x * escala,
        tabuleiro.y * escala,
        tabuleiro.w * escala
    )];
    let mut botoes = vec![("zen", tela.aba_zen), ("classico", tela.aba_classico)];
    if let Some(embaralhar) = tela.botao_embaralhar {
        botoes.push(("embaralhar", embaralhar));
    }
    botoes.extend([
        ("dica", tela.botao_dica),
        ("novo_jogo", tela.botao_novo_jogo),
        ("jogar_novamente", tela.botao_jogar_novamente),
        ("cancelar", tela.dialogo_cancelar),
        ("confirmar", tela.dialogo_confirmar),
    ]);
    for (nome, area) in botoes {
        linhas.push(format!(
            "PRISMA_M1 botao={} x={:.0} y={:.0}",
            nome,
            (area.x + area.w / 2.0) * escala,
            (area.y + area.h / 2.0) * escala
        ));
    }
    linhas
}

/// O jogo: lê o dedo (ou o mouse), anda com a animação, desenha e escreve no registro do
/// Android as linhas `PRISMA_M1 ...` que o teste do celular virtual lê.
/// Com `cena`, mostra um momento fixo do jogo (para foto) em vez de uma partida nova.
async fn rodar_jogo(foto: Option<String>, cena: Option<String>) {
    let mut app = match cena {
        Some(nome) => cenas::cena(&nome).unwrap_or_else(|| {
            eprintln!("cena desconhecida: {nome}");
            std::process::exit(2);
        }),
        None => App::novo(),
    };
    let sprites = render::Sprites::load();
    let calcular_tela = |app: &App| layout::tela_jogo(screen_width(), screen_height(), app.modo() == Mode::Zen);

    let mut quadro = 0u32;
    // Tamanho e modo para os quais a posição dos botões já foi escrita no registro.
    let mut geometria_registrada: Option<(f32, f32, Mode)> = None;
    let mut ultima_posicao = (-1.0f32, -1.0f32);
    loop {
        // No celular o toque vira clique do mouse (o Macroquad faz isso sozinho, como no M0b).
        let tela = calcular_tela(&app);
        let (x, y) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left) {
            app.tocar(Evento::Baixo { x, y }, &tela);
        } else if is_mouse_button_down(MouseButton::Left) && (x, y) != ultima_posicao {
            app.tocar(Evento::Mover { x, y }, &tela);
        }
        if is_mouse_button_released(MouseButton::Left) {
            app.tocar(Evento::Cima { x, y }, &tela);
        }
        ultima_posicao = (x, y);

        // Sem limitar o tempo: ao voltar do segundo plano a animação simplesmente termina.
        // (No modo foto o jogo fica parado, para a cena sair exatamente como foi montada.)
        if foto.is_none() {
            app.avancar(get_frame_time() * 1000.0);
        }

        // O modo pode ter mudado com o toque: o desenho e o registro usam o layout novo.
        let tela = calcular_tela(&app);
        let situacao = (screen_width(), screen_height(), app.modo());
        if geometria_registrada != Some(situacao) {
            geometria_registrada = Some(situacao);
            for linha in linhas_de_geometria(&tela, screen_dpi_scale()) {
                info!("{}", linha);
            }
        }

        render::draw_jogo(&app, &sprites, &tela);
        for linha in app.registro() {
            info!("{}", linha);
        }
        foto_do_quadro(&foto, quadro);
        quadro += 1;
        next_frame().await;
    }
}

/// Toca o som de teste. Se o áudio estiver morto (por exemplo, um PC sem placa de som), o
/// `play_sound_once` pode dar pânico; o `catch_unwind` segura o pânico para o app não fechar.
fn tocar_som(som: &Result<Sound, String>) -> String {
    let resultado = match som {
        Err(erro) => Err(erro.clone()),
        Ok(som) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| play_sound_once(som)))
            .map_err(|_| "o áudio não respondeu".to_string()),
    };
    match resultado {
        Ok(()) => {
            info!("PRISMA_M0B som=ok");
            "Som: tocado".to_string()
        }
        Err(erro) => {
            info!("PRISMA_M0B som=erro: {}", erro);
            format!("Som: erro: {erro}")
        }
    }
}

/// Faz o aparelho vibrar e devolve o texto da linha de vibração.
fn vibrar_e_contar() -> String {
    match plataforma::vibrar(300) {
        Ok(()) => {
            info!("PRISMA_M0B vibracao=ok");
            // No PC a vibração é de mentira; o texto avisa para ninguém se confundir.
            if cfg!(target_os = "android") {
                "Vibração: ok".to_string()
            } else {
                "Vibração: ok (simulada no PC)".to_string()
            }
        }
        Err(erro) => {
            info!("PRISMA_M0B vibracao=erro: {}", erro);
            format!("Vibração: erro: {erro}")
        }
    }
}

/// A tela de teste do M0b: conta a abertura, carrega o som e espera toque nos botões.
/// Cada fato importante também vai para o registro do Android (linhas `PRISMA_M0B ...`),
/// que o teste do celular virtual do GitHub lê.
async fn rodar_diagnostico(foto: Option<String>) {
    let aberturas = plataforma::pasta_de_dados()
        .and_then(|pasta| contador::registrar_abertura(&pasta));
    let linha_aberturas = match &aberturas {
        Ok(numero) => {
            info!("PRISMA_M0B aberturas={}", numero);
            format!("Aberturas: {numero}")
        }
        Err(erro) => {
            info!("PRISMA_M0B aberturas=erro: {}", erro);
            format!("Aberturas: erro: {erro}")
        }
    };

    let som: Result<Sound, String> = load_sound_from_bytes(&tom::tom_wav(440.0, 300, 0.4))
        .await
        .map_err(|erro| format!("{erro:?}"));
    let mut linha_som = match &som {
        Ok(_) => {
            info!("PRISMA_M0B som_carregado=ok");
            "Som: ainda não testado".to_string()
        }
        Err(erro) => {
            info!("PRISMA_M0B som_carregado=erro: {}", erro);
            format!("Som: erro ao carregar: {erro}")
        }
    };
    let mut linha_vibracao = "Vibração: ainda não testado".to_string();

    let mut quadro = 0u32;
    // Último tamanho de tela cujos botões foram escritos no registro.
    let mut tamanho_registrado = (0.0f32, 0.0f32);
    loop {
        let (largura, altura) = (screen_width(), screen_height());
        let tela = layout::tela_diagnostico(largura, altura);

        // O teste do celular virtual precisa saber onde tocar. Escreve o centro de cada
        // botão (em pixels reais da tela: posição lógica × escala) sempre que o tamanho muda.
        if (largura, altura) != tamanho_registrado {
            tamanho_registrado = (largura, altura);
            let escala = screen_dpi_scale();
            for (nome, botao) in [("som", tela.botao_som), ("vibrar", tela.botao_vibrar)] {
                let x = (botao.x + botao.w / 2.0) * escala;
                let y = (botao.y + botao.h / 2.0) * escala;
                info!("PRISMA_M0B botao={} x={:.0} y={:.0}", nome, x, y);
            }
        }

        // No celular o toque vira clique do mouse (o Macroquad faz isso sozinho).
        if is_mouse_button_pressed(MouseButton::Left) {
            let (x, y) = mouse_position();
            if tela.botao_som.contem(x, y) {
                linha_som = tocar_som(&som);
            } else if tela.botao_vibrar.contem(x, y) {
                linha_vibracao = vibrar_e_contar();
            }
        }

        render::draw_diagnostico(&tela, [&linha_aberturas, &linha_som, &linha_vibracao]);
        foto_do_quadro(&foto, quadro);
        quadro += 1;
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{all_sprites, sprite_for};
    use std::collections::HashSet;

    /// Transforma uma lista de textos fixos em `Vec<String>`, como `std::env::args` entrega.
    fn args(itens: &[&str]) -> Vec<String> {
        itens.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn foto_sem_caminho_e_ignorada() {
        assert_eq!(screenshot_path(&args(&[])), None);
        assert_eq!(screenshot_path(&args(&["--screenshot"])), None);
        assert_eq!(screenshot_path(&args(&["--screenshot", "a.png"])), Some("a.png".to_string()));
        assert_eq!(
            screenshot_path(&args(&["x", "--screenshot", "b.png"])),
            Some("b.png".to_string())
        );
    }

    #[test]
    fn tela_pedida_le_o_argumento_tela() {
        assert_eq!(tela_pedida(&args(&["--tela", "diagnostico"])), Tela::Diagnostico);
        assert_eq!(tela_pedida(&args(&["--tela", "vitrine"])), Tela::Vitrine);
        assert_eq!(tela_pedida(&args(&["--tela", "jogo"])), Tela::Jogo);
        assert_eq!(tela_pedida(&args(&["x", "--tela", "vitrine", "y"])), Tela::Vitrine);
        // Sem argumento (como no celular) ou com argumento desconhecido: o jogo.
        assert_eq!(tela_pedida(&args(&[])), Tela::Jogo);
        assert_eq!(tela_pedida(&args(&["--tela"])), Tela::Jogo);
        assert_eq!(tela_pedida(&args(&["--tela", "xyz"])), Tela::Jogo);
    }

    #[test]
    fn cena_pedida_le_o_argumento_cena() {
        assert_eq!(cena_pedida(&args(&["--tela", "jogo", "--cena", "limpando"])), Some("limpando".to_string()));
        assert_eq!(cena_pedida(&args(&["x", "--cena", "queda", "y"])), Some("queda".to_string()));
        assert_eq!(cena_pedida(&args(&["--cena"])), None);
        assert_eq!(cena_pedida(&args(&[])), None);
    }

    #[test]
    fn registro_de_geometria_usa_pixels_reais() {
        let tela = layout::tela_jogo(540.0, 960.0, true);
        let linhas = linhas_de_geometria(&tela, 2.0);
        assert_eq!(
            linhas[0],
            format!(
                "PRISMA_M1 tabuleiro x={:.0} y={:.0} lado={:.0}",
                tela.tabuleiro.x * 2.0,
                tela.tabuleiro.y * 2.0,
                tela.tabuleiro.w * 2.0
            )
        );
        let nomes: Vec<&str> = linhas[1..].iter().map(|linha| linha.split(' ').nth(1).unwrap()).collect();
        assert_eq!(
            nomes,
            [
                "botao=zen",
                "botao=classico",
                "botao=embaralhar",
                "botao=dica",
                "botao=novo_jogo",
                "botao=jogar_novamente",
                "botao=cancelar",
                "botao=confirmar"
            ]
        );
        // O centro do botão Dica, em pixels reais (lógico x 2).
        let dica = tela.botao_dica;
        let esperado = format!(
            "PRISMA_M1 botao=dica x={:.0} y={:.0}",
            (dica.x + dica.w / 2.0) * 2.0,
            (dica.y + dica.h / 2.0) * 2.0
        );
        assert!(linhas.contains(&esperado), "faltou: {esperado}");
        // No Clássico não há Embaralhar, mas os botões escondidos continuam registrados.
        let classico = layout::tela_jogo(540.0, 960.0, false);
        let linhas = linhas_de_geometria(&classico, 2.0);
        assert!(!linhas.iter().any(|linha| linha.contains("botao=embaralhar")));
        for escondido in ["botao=jogar_novamente", "botao=cancelar", "botao=confirmar"] {
            assert!(linhas.iter().any(|linha| linha.contains(escondido)), "faltou {escondido}");
        }
    }

    #[test]
    fn a_vitrine_mostra_as_22_imagens() {
        let jogo = demo_game();
        let mostradas: HashSet<_> = jogo
            .board
            .iter()
            .flatten()
            .filter_map(sprite_for)
            .collect();
        for sprite in all_sprites() {
            assert!(mostradas.contains(&sprite), "a vitrine não mostra {sprite:?}");
        }
    }
}
