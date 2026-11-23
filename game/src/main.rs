//! `prisma_game`: a tela do Prisma em Rust (janela no PC, app no Android).
//!
//! As regras do jogo ficam no pacote `prisma_engine`; aqui só se desenha e se lê toque.

mod app;
mod assets;
mod audio;
mod cenas;
mod contador;
mod efeitos;
mod formato;
mod fundo;
mod input;
mod layout;
mod linha;
mod menu;
mod particulas;
mod plataforma;
mod preferencias;
mod render;
mod salvar;
mod tom;
mod vibracao;

use crate::app::{App, Folha};
use crate::input::Evento;
use crate::salvar::Salvador;
use crate::vibracao::Toque;
use macroquad::audio::{load_sound_from_bytes, play_sound_once, Sound};
// Atenção: o `info!` do miniquad não aceita `{nome}` dentro do texto; os valores vão como
// argumentos (`info!("x={}", x)`), senão a chave aparece escrita no registro.
use macroquad::logging::info;
use macroquad::prelude::*;
use prisma_engine::{lcg_random, Game, Mode, Tile, TileKind};
use std::path::PathBuf;

/// Lê `--tamanho LARGURAxALTURA` (por exemplo `360x640`), para ver o jogo em outro tamanho de
/// tela no PC. Valor que não é um tamanho razoável (de 100 a 4000 pixels) é ignorado.
fn tamanho_pedido(args: &[String]) -> Option<(i32, i32)> {
    let posicao = args.iter().position(|argumento| argumento == "--tamanho")?;
    let (largura, altura) = args.get(posicao + 1)?.split_once('x')?;
    let (largura, altura) = (largura.parse::<i32>().ok()?, altura.parse::<i32>().ok()?);
    let razoavel = |lado: i32| (100..=4000).contains(&lado);
    (razoavel(largura) && razoavel(altura)).then_some((largura, altura))
}

/// Configuração da janela. No Android o sistema escolhe o tamanho; no PC usamos 540×960,
/// uma tela de celular em pé (ou o `--tamanho` pedido).
fn window_conf() -> Conf {
    let argumentos: Vec<String> = std::env::args().collect();
    let (largura, altura) = tamanho_pedido(&argumentos).unwrap_or((540, 960));
    Conf {
        window_title: "Prisma".to_owned(),
        window_width: largura,
        window_height: altura,
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

/// Lê `--dados pasta` nos argumentos: no PC, usa essa pasta de dados em vez da padrão (para
/// testar sem mexer nos dados de verdade). Sem valor (ou com outro argumento no lugar dele),
/// o pedido é ignorado.
fn dados_pedidos(args: &[String]) -> Option<String> {
    let posicao = args.iter().position(|argumento| argumento == "--dados")?;
    args.get(posicao + 1).filter(|valor| !valor.is_empty() && !valor.starts_with("--")).cloned()
}

/// O jogo lê e grava arquivos? Sim no jogo de verdade; não numa cena (momento fixo); e a foto
/// só abre a pasta de dados se `--dados` pediu uma (assim a foto não depende dos dados de quem a
/// tira nem mexe neles).
fn usa_arquivos(cena: &Option<String>, foto: &Option<String>, dados: &Option<String>) -> bool {
    cena.is_none() && (foto.is_none() || dados.is_some())
}

/// Abre o jogo com os arquivos da pasta pedida (ou da pasta de dados do app). Se a pasta não
/// puder ser achada, o jogo abre assim mesmo, só que sem salvar.
fn abrir_com_arquivos(dados: Option<String>) -> App {
    let pasta = match dados {
        Some(pasta) => Ok(PathBuf::from(pasta)),
        None => plataforma::pasta_de_dados(),
    };
    match pasta {
        Ok(pasta) => App::abrir(Salvador::em(pasta)),
        Err(erro) => {
            info!("PRISMA_M2 sem_pasta: {}", erro);
            App::abrir(Salvador::nenhum())
        }
    }
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
        Tela::Jogo => rodar_jogo(foto, cena_pedida(&argumentos), dados_pedidos(&argumentos)).await,
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

/// A linha de registro de um toque (início do dedo), em pixels reais da tela do app. O teste
/// do celular virtual toca num ponto conhecido da tela e compara com esta linha para saber
/// quanto a janela do app está deslocada (por exemplo pela barra de status): o `adb` toca em
/// coordenadas da tela, e o app mede o toque a partir do canto da própria janela.
fn linha_de_toque(x: f32, y: f32, escala: f32) -> String {
    format!("PRISMA_M1 toque x={:.0} y={:.0}", x * escala, y * escala)
}

/// As linhas de registro com a posição do tabuleiro e dos botões, em PIXELS REAIS da tela
/// (posição lógica × `escala`): o teste do celular virtual lê estas linhas para saber onde
/// tocar. `jogar_novamente`, `cancelar` e `confirmar` são sempre registrados, mesmo quando
/// estão escondidos; `embaralhar` só existe no Zen. O ♪ (`som`) e o ≡ (`menu`) estão sempre lá;
/// com a folha de Som aberta entram também os botões dela: `fechar_som`, `liga_vibracao` (só em
/// aparelho com vibrador) e, no Zen, `liga_musica`, `liga_ambiente` e `escolhe_efeitos` (o teste
/// do celular virtual os usa para ligar a vibração, a música e o ambiente e para mudar a
/// intensidade dos efeitos). Com a folha do Menu aberta entram
/// `fechar_menu`, `aba_recordes` e `aba_creditos`.
fn linhas_de_geometria(tela: &layout::TelaJogo, escala: f32, folha: Option<Folha>) -> Vec<String> {
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
        ("som", tela.botao_som),
        ("menu", tela.botao_menu),
    ]);
    if folha.is_some_and(Folha::e_do_menu) {
        botoes.extend([
            ("fechar_menu", tela.menu.fechar),
            ("aba_recordes", tela.menu.aba_recordes),
            ("aba_creditos", tela.menu.aba_creditos),
        ]);
    }
    if folha == Some(Folha::Som) {
        botoes.push(("fechar_som", tela.som.fechar));
        for (nome, interruptor) in [
            ("liga_vibracao", tela.som.vibracao),
            ("liga_musica", tela.som.liga_musica),
            ("liga_ambiente", tela.som.liga_ambiente),
            ("escolhe_efeitos", tela.som.efeitos),
        ] {
            if let Some(area) = interruptor {
                botoes.push((nome, area));
            }
        }
    }
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
/// Fora isso o jogo abre com o que está gravado na pasta de dados (ou na de `--dados`); a foto
/// sem `--dados` não lê nem grava nada (ver `usa_arquivos`).
async fn rodar_jogo(foto: Option<String>, cena: Option<String>, dados: Option<String>) {
    let com_arquivos = usa_arquivos(&cena, &foto, &dados);
    // O som só liga no jogo de verdade: foto e cena são imagens paradas e não carregam áudio.
    let com_audio = cena.is_none() && foto.is_none();
    let mut app = match cena {
        Some(nome) => cenas::cena(&nome).unwrap_or_else(|| {
            eprintln!("cena desconhecida: {nome}");
            std::process::exit(2);
        }),
        None if com_arquivos => abrir_com_arquivos(dados).com_vibrador(tem_vibrador_sem_risco()),
        None => App::novo().com_vibrador(tem_vibrador_sem_risco()),
    };
    let sprites = render::Sprites::load();
    let mut audio_do_jogo = com_audio.then(audio::Audio::novo);
    let calcular_tela =
        |app: &App| layout::tela_jogo_com(screen_width(), screen_height(), app.modo() == Mode::Zen, app.tem_vibrador());

    let mut quadro = 0u32;
    // Tamanho, modo e folha de Som (aberta ou fechada) para os quais a posição dos botões já foi
    // escrita no registro.
    let mut geometria_registrada: Option<(f32, f32, Mode, Option<Folha>)> = None;
    let mut ultima_posicao = (-1.0f32, -1.0f32);
    loop {
        // No celular o toque vira clique do mouse (o Macroquad faz isso sozinho, como no M0b).
        let tela = calcular_tela(&app);
        let (x, y) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left) {
            info!("{}", linha_de_toque(x, y, screen_dpi_scale()));
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

        // Sons: os efeitos que chegaram a hora de tocar e o que a música e o ambiente do Zen
        // deveriam estar fazendo agora (o planejador compara com o que está carregado).
        let efeitos = app.sons();
        // Vibrações pedidas neste quadro (só com vibrador). Se o aparelho recusar o toque de
        // "ligou", o interruptor volta a desligado, para não mentir.
        for toque in app.vibracoes() {
            if !app.tem_vibrador() {
                continue;
            }
            if let Err(erro) = vibrar_sem_risco(toque) {
                info!("PRISMA_M4 vibracao erro texto={}", erro);
                if toque == Toque::Ligou {
                    app.vibracao_recusada();
                }
            }
        }
        if let Some(audio) = audio_do_jogo.as_mut() {
            if !efeitos.is_empty() {
                let volumes = app.preferencias().volumes;
                for efeito in efeitos {
                    audio.tocar(efeito, &volumes);
                }
            }
            let comandos = fundo::planejar(&audio.atual(), &app.som_desejado());
            // Carregar uma música trava o quadro por um instante: antes, um quadro com o aviso
            // "Carregando…" (a carga só começa no quadro seguinte). O tempo vai para o registro.
            match fundo::decidir(&comandos, app.carregando()) {
                fundo::Decisao::Avisar(canal) => app.marcar_carregando(canal),
                fundo::Decisao::Aplicar => {
                    audio.aplicar(comandos, &mut app).await;
                    app.fim_do_carregamento();
                }
                fundo::Decisao::Nada => app.fim_do_carregamento(),
            }
        }

        // O modo pode ter mudado com o toque: o desenho e o registro usam o layout novo.
        let tela = calcular_tela(&app);
        let situacao = (screen_width(), screen_height(), app.modo(), app.folha());
        if geometria_registrada != Some(situacao) {
            geometria_registrada = Some(situacao);
            for linha in linhas_de_geometria(&tela, screen_dpi_scale(), app.folha()) {
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

/// O aparelho tem vibrador? Qualquer falha (até um pânico da ponte com o Android) vale "não":
/// o interruptor some e o jogo segue.
fn tem_vibrador_sem_risco() -> bool {
    std::panic::catch_unwind(plataforma::tem_vibrador).unwrap_or(false)
}

/// Vibra com o padrão do `toque`. O `catch_unwind` segura um pânico da ponte com o Android (como
/// em `tocar_som`): o pior que pode acontecer é o toque não sair.
fn vibrar_sem_risco(toque: Toque) -> Result<(), String> {
    std::panic::catch_unwind(|| plataforma::vibrar_padrao(toque.padrao_ms()))
        .unwrap_or_else(|_| Err("a vibração não respondeu".to_string()))
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
    fn tamanho_pedido_le_largura_x_altura() {
        assert_eq!(tamanho_pedido(&args(&["--tamanho", "360x640"])), Some((360, 640)));
        assert_eq!(tamanho_pedido(&args(&["x", "--tamanho", "800x600", "--cena", "som"])), Some((800, 600)));
        // Sem valor, com valor torto ou fora do razoável: nada (o jogo abre no tamanho de sempre).
        assert_eq!(tamanho_pedido(&args(&[])), None);
        assert_eq!(tamanho_pedido(&args(&["--tamanho"])), None);
        assert_eq!(tamanho_pedido(&args(&["--tamanho", "360"])), None);
        assert_eq!(tamanho_pedido(&args(&["--tamanho", "axb"])), None);
        assert_eq!(tamanho_pedido(&args(&["--tamanho", "-5x640"])), None);
        assert_eq!(tamanho_pedido(&args(&["--tamanho", "0x0"])), None);
        assert_eq!(tamanho_pedido(&args(&["--tamanho", "99999x99999"])), None);
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
    fn dados_pedidos_le_o_argumento_dados() {
        assert_eq!(dados_pedidos(&args(&["--dados", "/tmp/x"])), Some("/tmp/x".to_string()));
        assert_eq!(dados_pedidos(&args(&["a", "--dados", "pasta", "--screenshot", "f.png"])), Some("pasta".to_string()));
        assert_eq!(dados_pedidos(&args(&["--dados"])), None, "sem valor");
        assert_eq!(dados_pedidos(&args(&[])), None);
        assert_eq!(dados_pedidos(&args(&["--screenshot", "f.png"])), None);
        // Outro argumento no lugar do valor não vira o nome de uma pasta.
        assert_eq!(dados_pedidos(&args(&["--dados", "--screenshot", "f.png"])), None);
        assert_eq!(dados_pedidos(&args(&["--dados", ""])), None, "texto vazio não é pasta");
    }

    #[test]
    fn so_o_jogo_de_verdade_ou_a_foto_com_dados_usam_arquivos() {
        let algo = || Some("x".to_string());
        // Jogo de verdade (sem foto, sem cena): usa a pasta de dados, a padrão ou a pedida.
        assert!(usa_arquivos(&None, &None, &None));
        assert!(usa_arquivos(&None, &None, &algo()));
        // Foto sem `--dados`: nunca lê nem grava (a foto não depende dos dados de quem tira).
        assert!(!usa_arquivos(&None, &algo(), &None));
        // Foto com `--dados`: abre essa pasta (para fotografar um save restaurado).
        assert!(usa_arquivos(&None, &algo(), &algo()));
        // Cena: sempre a cena fixa, com ou sem foto, com ou sem `--dados`.
        assert!(!usa_arquivos(&algo(), &None, &None));
        assert!(!usa_arquivos(&algo(), &None, &algo()));
        assert!(!usa_arquivos(&algo(), &algo(), &None));
        assert!(!usa_arquivos(&algo(), &algo(), &algo()));
    }

    #[test]
    fn registro_de_toque_usa_pixels_reais() {
        // Toque em (80,200) lógicos numa tela com escala 2: o teste do emulador compara isto com
        // o ponto onde ele mesmo tocou, para descobrir quanto a janela do app está deslocada
        // (barra de status) em relação à tela.
        assert_eq!(linha_de_toque(80.0, 200.0, 2.0), "PRISMA_M1 toque x=160 y=400");
        assert_eq!(linha_de_toque(0.4, 12.6, 1.0), "PRISMA_M1 toque x=0 y=13");
    }

    #[test]
    fn registro_de_geometria_usa_pixels_reais() {
        let tela = layout::tela_jogo(540.0, 960.0, true);
        let linhas = linhas_de_geometria(&tela, 2.0, None);
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
                "botao=confirmar",
                "botao=som",
                "botao=menu"
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
        let linhas = linhas_de_geometria(&classico, 2.0, None);
        assert!(!linhas.iter().any(|linha| linha.contains("botao=embaralhar")));
        for escondido in ["botao=jogar_novamente", "botao=cancelar", "botao=confirmar"] {
            assert!(linhas.iter().any(|linha| linha.contains(escondido)), "faltou {escondido}");
        }
    }

    #[test]
    fn o_registro_de_geometria_leva_o_botao_do_som_e_os_da_folha_so_com_ela_aberta() {
        let centro = |area: layout::Rect| format!("x={:.0} y={:.0}", (area.x + area.w / 2.0) * 2.0, (area.y + area.h / 2.0) * 2.0);
        let tem = |linhas: &[String], nome: &str| linhas.iter().any(|linha| linha.starts_with(&format!("PRISMA_M1 botao={nome} ")));
        let zen = layout::tela_jogo(540.0, 960.0, true);
        // Fechada: o ♪ está lá (é por ele que o teste abre a folha), os botões da folha não.
        let fechada = linhas_de_geometria(&zen, 2.0, None);
        assert!(fechada.contains(&format!("PRISMA_M1 botao=som {}", centro(zen.botao_som))));
        for nome in ["fechar_som", "liga_musica", "liga_ambiente"] {
            assert!(!tem(&fechada, nome), "{nome} só é registrado com a folha aberta");
        }
        // Aberta no Zen: o ♪ e mais o ✕ e os dois interruptores, no centro de cada um.
        let aberta = linhas_de_geometria(&zen, 2.0, Some(Folha::Som));
        for (nome, area) in [
            ("som", zen.botao_som),
            ("fechar_som", zen.som.fechar),
            ("liga_musica", zen.som.liga_musica.unwrap()),
            ("liga_ambiente", zen.som.liga_ambiente.unwrap()),
        ] {
            let esperado = format!("PRISMA_M1 botao={nome} {}", centro(area));
            assert!(aberta.contains(&esperado), "faltou: {esperado}");
        }
        // Aberta no Clássico: só o ✕, porque música e ambiente são do Zen.
        let classico = layout::tela_jogo(540.0, 960.0, false);
        let aberta = linhas_de_geometria(&classico, 2.0, Some(Folha::Som));
        assert!(aberta.contains(&format!("PRISMA_M1 botao=fechar_som {}", centro(classico.som.fechar))));
        assert!(!tem(&aberta, "liga_musica") && !tem(&aberta, "liga_ambiente"), "o Clássico não tem os interruptores");
    }

    #[test]
    fn o_registro_de_geometria_leva_liga_vibracao_com_a_folha_aberta() {
        let tem = |linhas: &[String], nome: &str| linhas.iter().any(|linha| linha.starts_with(&format!("PRISMA_M1 botao={nome} ")));
        for zen in [true, false] {
            let com_vibrador = layout::tela_jogo_com(540.0, 960.0, zen, true);
            // Fechada: não aparece.
            assert!(!tem(&linhas_de_geometria(&com_vibrador, 2.0, None), "liga_vibracao"), "zen={zen}");
            // Aberta: aparece, no centro da linha.
            let area = com_vibrador.som.vibracao.expect("tem a linha");
            let esperado = format!(
                "PRISMA_M1 botao=liga_vibracao x={:.0} y={:.0}",
                (area.x + area.w / 2.0) * 2.0,
                (area.y + area.h / 2.0) * 2.0
            );
            assert!(linhas_de_geometria(&com_vibrador, 2.0, Some(Folha::Som)).contains(&esperado), "zen={zen}: faltou {esperado}");
            // Sem vibrador: nunca aparece.
            let sem_vibrador = layout::tela_jogo_com(540.0, 960.0, zen, false);
            assert!(!tem(&linhas_de_geometria(&sem_vibrador, 2.0, Some(Folha::Som)), "liga_vibracao"), "zen={zen}");
        }
    }


    #[test]
    fn a_geometria_leva_escolhe_efeitos_so_no_zen_com_a_folha_aberta() {
        let tem = |linhas: &[String], nome: &str| linhas.iter().any(|linha| linha.starts_with(&format!("PRISMA_M1 botao={nome} ")));
        let zen = layout::tela_jogo(540.0, 960.0, true);
        let area = zen.som.efeitos.expect("o Zen tem a linha");
        let esperado = format!(
            "PRISMA_M1 botao=escolhe_efeitos x={:.0} y={:.0}",
            (area.x + area.w / 2.0) * 2.0,
            (area.y + area.h / 2.0) * 2.0
        );
        assert!(linhas_de_geometria(&zen, 2.0, Some(Folha::Som)).contains(&esperado), "faltou {esperado}");
        for folha in [None, Some(Folha::Recordes), Some(Folha::Creditos)] {
            assert!(!tem(&linhas_de_geometria(&zen, 2.0, folha), "escolhe_efeitos"), "{folha:?}");
        }
        let classico = layout::tela_jogo(540.0, 960.0, false);
        assert!(!tem(&linhas_de_geometria(&classico, 2.0, Some(Folha::Som)), "escolhe_efeitos"));
    }

    #[test]
    fn o_registro_de_geometria_leva_menu_e_os_botoes_do_menu_so_com_ele_aberto() {
        let centro = |area: layout::Rect| format!("x={:.0} y={:.0}", (area.x + area.w / 2.0) * 2.0, (area.y + area.h / 2.0) * 2.0);
        let tem = |linhas: &[String], nome: &str| linhas.iter().any(|linha| linha.starts_with(&format!("PRISMA_M1 botao={nome} ")));
        for zen in [true, false] {
            let tela = layout::tela_jogo(540.0, 960.0, zen);
            let menu = [("fechar_menu", tela.menu.fechar), ("aba_recordes", tela.menu.aba_recordes), ("aba_creditos", tela.menu.aba_creditos)];
            // O ≡ está sempre lá (é por ele que o teste do celular virtual abre o Menu) ...
            for folha in [None, Some(Folha::Som), Some(Folha::Recordes), Some(Folha::Creditos)] {
                let linhas = linhas_de_geometria(&tela, 2.0, folha);
                assert!(linhas.contains(&format!("PRISMA_M1 botao=menu {}", centro(tela.botao_menu))), "{folha:?}");
            }
            // ... e os botões do Menu só entram com ele aberto (em qualquer aba), no centro de cada um.
            for folha in [None, Some(Folha::Som)] {
                let linhas = linhas_de_geometria(&tela, 2.0, folha);
                for (nome, _) in menu {
                    assert!(!tem(&linhas, nome), "{nome} só é registrado com o Menu aberto ({folha:?})");
                }
            }
            for folha in [Folha::Recordes, Folha::Creditos] {
                let linhas = linhas_de_geometria(&tela, 2.0, Some(folha));
                for (nome, area) in menu {
                    assert!(linhas.contains(&format!("PRISMA_M1 botao={nome} {}", centro(area))), "faltou {nome} ({folha:?})");
                }
                // Com o Menu aberto, os botões da folha de Som não aparecem.
                assert!(!tem(&linhas, "fechar_som") && !tem(&linhas, "liga_vibracao"), "{folha:?}");
            }
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
