//! `prisma_game`: a tela do Prisma em Rust (janela no PC, app no Android).
//!
//! As regras do jogo ficam no pacote `prisma_engine`; aqui só se desenha e se lê toque.

mod assets;
mod contador;
mod input;
mod layout;
mod linha;
mod plataforma;
mod render;
mod tom;

use macroquad::audio::{load_sound_from_bytes, play_sound_once, Sound};
// Atenção: o `info!` do miniquad não aceita `{nome}` dentro do texto; os valores vão como
// argumentos (`info!("x={}", x)`), senão a chave aparece escrita no registro.
use macroquad::logging::info;
use macroquad::prelude::*;
use prisma_engine::{lcg_random, Game, Tile, TileKind};

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

/// Qual tela abrir. No celular não há argumentos, então abre sempre a de teste.
#[derive(Debug, PartialEq)]
enum Tela {
    /// A tela de teste do M0b: contador, som e vibração.
    Diagnostico,
    /// O tabuleiro de vitrine do M0a (só no PC, com `--tela vitrine`).
    Vitrine,
}

/// Lê `--tela vitrine` nos argumentos; qualquer outra coisa (ou nada) abre o diagnóstico.
fn tela_pedida(args: &[String]) -> Tela {
    let valor = args
        .iter()
        .position(|argumento| argumento == "--tela")
        .and_then(|posicao| args.get(posicao + 1));
    match valor.map(String::as_str) {
        Some("vitrine") => Tela::Vitrine,
        _ => Tela::Diagnostico,
    }
}

/// No modo "foto": no segundo quadro (o primeiro "aquece" a tela) grava a imagem da tela
/// num PNG e fecha o programa.
fn foto_do_quadro(foto: &Option<String>, quadro: u32) {
    if let Some(caminho) = foto {
        if quadro == 1 {
            get_screen_data().export_png(caminho);
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
        assert_eq!(tela_pedida(&args(&["--tela", "vitrine"])), Tela::Vitrine);
        assert_eq!(tela_pedida(&args(&["x", "--tela", "vitrine", "y"])), Tela::Vitrine);
        assert_eq!(tela_pedida(&args(&[])), Tela::Diagnostico);
        assert_eq!(tela_pedida(&args(&["--tela"])), Tela::Diagnostico);
        assert_eq!(tela_pedida(&args(&["--tela", "xyz"])), Tela::Diagnostico);
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
