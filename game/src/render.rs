//! Desenho: pega o estado do motor e pinta na tela com as imagens das pedras.
//!
//! Este módulo precisa de uma janela aberta (o Macroquad guarda as imagens na placa de
//! vídeo), por isso não tem testes automáticos: quem confere é a "foto" (`--screenshot`).

use std::collections::HashMap;

use macroquad::prelude::*;
use prisma_engine::{position, Board, Tile, SIZE};

use crate::app::{App, Painel};
use crate::assets::{all_sprites, png_bytes, sprite_for, Sprite, SPRITE_RATIO};
use crate::layout::{self, board_rect, cell_rect, Rect, TelaDiagnostico, TelaJogo};
use crate::linha::{
    deslocamento_troca, embaralhar_alfa, limpeza_da_casa, linhas_de_queda, nivel_faixa, queda_deslocamento,
    queda_ms, resgate_da_casa,
};
use crate::preferencias::{orbe, segundos_restantes};

/// Cor de fundo da tela (a mesma do Prisma em JS).
const FUNDO: Color = Color::from_hex(0x101229);
/// Cor do painel atrás das pedras (a do tabuleiro ilustrado do JS).
const PAINEL: Color = Color::from_hex(0x0d142c);
/// Cor dos botões da tela de teste.
const BOTAO: Color = Color::from_hex(0x2a3f8f);
/// Cores da tela de jogo.
const TEXTO_SUAVE: Color = Color::from_hex(0x9aa5d9);
const ABA_ATIVA: Color = Color::from_hex(0x3b52c4);
const ABA_INATIVA: Color = Color::from_hex(0x1a2350);
const BARRA_PREENCHIDA: Color = Color::from_hex(0x7f93ff);
const DESTAQUE: Color = Color::from_hex(0xffd166);
const CARTAO: Color = Color::from_hex(0x182046);
const BORDA: Color = Color::from_hex(0x4a63d6);
/// O fundo da folha de Som: quase opaco, para as pedras de trás não atrapalharem a leitura.
const FOLHA: Color = Color::new(0.04, 0.06, 0.16, 0.985);
/// O traço das barras de volume (a parte vazia) e o interruptor desligado: um azul mais claro
/// que o das linhas, para se verem sobre elas.
const TRILHO: Color = Color::from_hex(0x2b3a7a);
const INTERRUPTOR_APAGADO: Color = Color::from_hex(0x464f7a);

/// Todas as imagens já carregadas na placa de vídeo, uma `Texture2D` por `Sprite`.
pub struct Sprites {
    texturas: HashMap<Sprite, Texture2D>,
}

impl Sprites {
    /// Carrega as 22 imagens a partir dos PNG embutidos no programa.
    /// Só funciona depois que a janela abriu (ou seja, dentro do `main` do Macroquad).
    pub fn load() -> Sprites {
        let texturas = all_sprites()
            .into_iter()
            // `None`: a biblioteca descobre sozinha que o arquivo é um PNG.
            .map(|sprite| (sprite, Texture2D::from_file_with_format(png_bytes(sprite), None)))
            .collect();
        Sprites { texturas }
    }
}

/// Desenha um quadro inteiro: fundo, painel do tabuleiro e as pedras.
///
/// O tamanho da tela é lido a cada quadro; assim o tabuleiro acompanha a janela quando ela
/// muda de tamanho (ou o celular gira).
pub fn draw_frame(board: &Board, sprites: &Sprites) {
    clear_background(FUNDO);

    let tabuleiro = board_rect(screen_width(), screen_height());
    draw_rectangle(tabuleiro.x, tabuleiro.y, tabuleiro.w, tabuleiro.h, PAINEL);

    for linha in 0..SIZE {
        for coluna in 0..SIZE {
            // Pedra sem imagem (cor ausente ou inválida): a casa fica vazia, sem erro.
            let Some(sprite) = sprite_for(&board[linha][coluna]) else {
                continue;
            };
            let Some(textura) = sprites.texturas.get(&sprite) else {
                continue;
            };
            let casa = cell_rect(tabuleiro, linha, coluna);
            // O PNG é maior que a casa (tem margem); desenhamos ele centrado na casa.
            let lado = casa.w * SPRITE_RATIO;
            draw_texture_ex(
                textura,
                casa.x + (casa.w - lado) / 2.0,
                casa.y + (casa.h - lado) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(lado, lado)),
                    ..Default::default()
                },
            );
        }
    }
}

/// Desenha a tela de teste do M0b: título, três linhas de resultado e dois botões.
/// `linhas` são, de cima para baixo: contador, som e vibração.
pub fn draw_diagnostico(tela: &TelaDiagnostico, linhas: [&str; 3]) {
    clear_background(FUNDO);
    draw_text("Prisma - teste de vida", tela.x_texto, tela.y_titulo, tela.tamanho_titulo, WHITE);
    for (texto, y) in linhas.into_iter().zip(tela.y_linhas) {
        draw_text(texto, tela.x_texto, y, tela.tamanho_texto, LIGHTGRAY);
    }
    desenhar_botao(tela.botao_som, "Tocar som", tela.tamanho_texto);
    desenhar_botao(tela.botao_vibrar, "Vibrar", tela.tamanho_texto);
}

/// Um botão: retângulo cheio com o rótulo centrado.
fn desenhar_botao(area: layout::Rect, rotulo: &str, tamanho: f32) {
    draw_rectangle(area.x, area.y, area.w, area.h, BOTAO);
    let medida = measure_text(rotulo, None, tamanho as u16, 1.0);
    // `draw_text` recebe a linha de base do texto; `offset_y` é a altura do texto acima dela.
    let x = area.x + (area.w - medida.width) / 2.0;
    let y = area.y + (area.h - medida.height) / 2.0 + medida.offset_y;
    draw_text(rotulo, x, y, tamanho, WHITE);
}

// ---- Tela de jogo ------------------------------------------------------------------------------

/// A mesma cor com a opacidade multiplicada por `alfa` (de 0 a 1).
fn com_alfa(cor: Color, alfa: f32) -> Color {
    Color::new(cor.r, cor.g, cor.b, cor.a * alfa.clamp(0.0, 1.0))
}

#[derive(Clone, Copy)]
enum Alinhamento {
    Esquerda,
    Centro,
    Direita,
}

/// Escreve `texto` dentro de `area`, na vertical no meio. Se não couber na largura, a letra
/// encolhe até caber.
fn escrever(texto: &str, area: Rect, tamanho: f32, cor: Color, alinhamento: Alinhamento) {
    if tamanho < 1.0 || area.w <= 0.0 {
        return;
    }
    let mut tamanho = tamanho;
    let medida = measure_text(texto, None, tamanho as u16, 1.0);
    if medida.width > area.w {
        tamanho *= area.w / medida.width;
        if tamanho < 1.0 {
            return;
        }
    }
    let medida = measure_text(texto, None, tamanho as u16, 1.0);
    // A altura vem de uma palavra de referência, para o texto não "pular" de acordo com as letras.
    let referencia = measure_text("Ag", None, tamanho as u16, 1.0);
    let x = match alinhamento {
        Alinhamento::Esquerda => area.x,
        Alinhamento::Centro => area.x + (area.w - medida.width) / 2.0,
        Alinhamento::Direita => area.x + area.w - medida.width,
    };
    let y = area.y + (area.h - referencia.height) / 2.0 + referencia.offset_y;
    draw_text(texto, x, y, tamanho, cor);
}

/// Um botão da tela de jogo: retângulo com borda e rótulo no meio. `ativo` falso apaga o botão.
fn botao_da_tela(area: Rect, rotulo: &str, tamanho: f32, fundo: Color, ativo: bool) {
    let alfa = if ativo { 1.0 } else { 0.4 };
    draw_rectangle(area.x, area.y, area.w, area.h, com_alfa(fundo, alfa));
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, com_alfa(BORDA, alfa));
    let margem = 0.06 * area.w;
    let dentro = Rect { x: area.x + margem, y: area.y, w: area.w - 2.0 * margem, h: area.h };
    escrever(rotulo, dentro, tamanho, com_alfa(WHITE, alfa), Alinhamento::Centro);
}

/// Desenha a imagem de uma pedra no meio de `casa`, com `escala` e `alfa`, deslocada de
/// (`dx`, `dy`) pixels. Pedra sem imagem é pulada.
fn desenhar_pedra(sprites: &Sprites, pedra: &Tile, casa: Rect, escala: f32, alfa: f32, (dx, dy): (f32, f32)) {
    let Some(sprite) = sprite_for(pedra) else { return };
    let Some(textura) = sprites.texturas.get(&sprite) else { return };
    let lado = casa.w * SPRITE_RATIO * escala;
    let x = casa.x + casa.w / 2.0 + dx - lado / 2.0;
    let y = casa.y + casa.h / 2.0 + dy - lado / 2.0;
    draw_texture_ex(
        textura,
        x,
        y,
        com_alfa(WHITE, alfa),
        DrawTextureParams { dest_size: Some(vec2(lado, lado)), ..Default::default() },
    );
}

/// Desenha as 64 casas de `tabuleiro`; `ajuste` decide, para cada casa, a escala, a opacidade
/// e o deslocamento (em pixels) da pedra, ou `None` para não desenhá-la aqui.
fn desenhar_casas(
    sprites: &Sprites,
    pedras: &Board,
    area: Rect,
    mut ajuste: impl FnMut(usize, Rect) -> Option<(f32, f32, (f32, f32))>,
) {
    for linha in 0..SIZE {
        for coluna in 0..SIZE {
            let casa = cell_rect(area, linha, coluna);
            if let Some((escala, alfa, deslocamento)) = ajuste(linha * SIZE + coluna, casa) {
                desenhar_pedra(sprites, &pedras[linha][coluna], casa, escala, alfa, deslocamento);
            }
        }
    }
}

/// As pedras do tabuleiro: paradas, ou como estão no passo atual da animação.
fn desenhar_pedras(app: &App, sprites: &Sprites, area: Rect) {
    let parada = |_: usize, _: Rect| Some((1.0, 1.0, (0.0, 0.0)));
    let Some((passo, t)) = app.passo_atual() else {
        desenhar_casas(sprites, &app.jogo().board, area, parada);
        return;
    };
    use crate::linha::Passo;
    match passo {
        Passo::Troca { antes, a, b, valida } => {
            let (a, b) = (*a, *b);
            // As duas pedras da troca ficam por último (por cima das outras).
            desenhar_casas(sprites, antes, area, |indice, _| (indice != a && indice != b).then_some((1.0, 1.0, (0.0, 0.0))));
            let (linha_a, coluna_a) = position(a);
            let (linha_b, coluna_b) = position(b);
            let (casa_a, casa_b) = (cell_rect(area, linha_a, coluna_a), cell_rect(area, linha_b, coluna_b));
            let d = deslocamento_troca(t, *valida);
            let (dx, dy) = ((casa_b.x - casa_a.x) * d, (casa_b.y - casa_a.y) * d);
            desenhar_pedra(sprites, &antes[linha_a][coluna_a], casa_a, 1.0, 1.0, (dx, dy));
            desenhar_pedra(sprites, &antes[linha_b][coluna_b], casa_b, 1.0, 1.0, (-dx, -dy));
        }
        Passo::Limpeza { tabuleiro, celulas, ativadas, .. } => {
            desenhar_casas(sprites, tabuleiro, area, |indice, _| {
                if celulas.contains(&indice) {
                    let (escala, alfa) = limpeza_da_casa(ativadas, indice, t);
                    Some((escala, alfa, (0.0, 0.0)))
                } else {
                    Some((1.0, 1.0, (0.0, 0.0)))
                }
            });
        }
        Passo::Queda { tabuleiro, origens } => {
            desenhar_casas(sprites, tabuleiro, area, |indice, casa| {
                let origem = origens.get(indice).copied().unwrap_or(indice as i32);
                let linha_destino = indice / SIZE;
                let linhas = linhas_de_queda(origem, linha_destino);
                let acima = queda_deslocamento(origem, linha_destino, t);
                // Pedra nova (nasceu acima do tabuleiro) aparece enquanto desce: 0,45 → 1.
                let alfa = if origem < 0 && linhas > 0 { 0.45 + 0.55 * (t / queda_ms(linhas)).clamp(0.0, 1.0) } else { 1.0 };
                Some((1.0, alfa, (0.0, -acima * casa.h)))
            });
        }
        Passo::Embaralhar { antes, depois } => {
            let (mostra_novo, alfa) = embaralhar_alfa(t);
            let pedras = if mostra_novo { depois } else { antes };
            desenhar_casas(sprites, pedras, area, |_, _| Some((1.0, alfa, (0.0, 0.0))));
        }
        Passo::Resgate { tabuleiro, indice } => {
            desenhar_casas(sprites, tabuleiro, area, |casa_indice, _| {
                if casa_indice == *indice {
                    let (escala, alfa) = resgate_da_casa(t);
                    Some((escala, alfa, (0.0, 0.0)))
                } else {
                    Some((1.0, 1.0, (0.0, 0.0)))
                }
            });
        }
        // Na faixa de nível o tabuleiro já está no estado final.
        Passo::Nivel { .. } => desenhar_casas(sprites, &app.jogo().board, area, parada),
    }
}

/// A casa de número `indice` (0 a 63) do tabuleiro.
fn casa_de(area: Rect, indice: usize) -> Rect {
    cell_rect(area, indice / SIZE, indice % SIZE)
}

/// Realces que ficam ATRÁS das pedras (para não "sujar" a cor delas): fundo da casa marcada,
/// do dedo em cima e da Dica.
fn desenhar_fundos_de_realce(app: &App, area: Rect) {
    let gestos = app.gestos();
    if let Some(indice) = gestos.selecionada() {
        let casa = casa_de(area, indice);
        draw_rectangle(casa.x, casa.y, casa.w, casa.h, Color::new(1.0, 1.0, 1.0, 0.22));
    }
    if let Some(indice) = gestos.pressionada() {
        let casa = casa_de(area, indice);
        draw_rectangle(casa.x, casa.y, casa.w, casa.h, Color::new(1.0, 1.0, 1.0, 0.14));
    }
    if let Some((a, b)) = app.dica() {
        let pulso = pulso_da_dica();
        for indice in [a, b] {
            let casa = casa_de(area, indice);
            draw_rectangle(casa.x, casa.y, casa.w, casa.h, com_alfa(DESTAQUE, 0.25 * pulso));
        }
    }
}

/// A moldura da Dica pulsa: a opacidade vai e volta com o relógio (de 0,1 a 1).
fn pulso_da_dica() -> f32 {
    0.55 + 0.45 * (get_time() * 6.0).sin() as f32
}

/// Realces que ficam POR CIMA das pedras: contornos da casa marcada, do alvo do arrasto e da Dica.
fn desenhar_contornos_de_realce(app: &App, area: Rect) {
    let gestos = app.gestos();
    if let Some(indice) = gestos.selecionada() {
        let casa = casa_de(area, indice);
        draw_rectangle_lines(casa.x, casa.y, casa.w, casa.h, 3.0, WHITE);
    }
    if let Some(indice) = gestos.alvo_do_arrasto() {
        let casa = casa_de(area, indice);
        draw_rectangle_lines(casa.x, casa.y, casa.w, casa.h, 3.0, com_alfa(WHITE, 0.8));
    }
    if let Some((a, b)) = app.dica() {
        let pulso = pulso_da_dica();
        for indice in [a, b] {
            let casa = casa_de(area, indice);
            draw_rectangle_lines(casa.x, casa.y, casa.w, casa.h, 4.0, com_alfa(DESTAQUE, pulso));
        }
    }
}

/// As abas, o painel de números e a barra de progresso.
fn desenhar_cabecalho(app: &App, painel: &Painel, tela: &TelaJogo) {
    let t = tela.texto;
    let zen = app.modo() == prisma_engine::Mode::Zen;
    botao_da_tela(tela.aba_zen, "Zen", 1.05 * t, if zen { ABA_ATIVA } else { ABA_INATIVA }, true);
    botao_da_tela(tela.aba_classico, "Clássico", 1.05 * t, if zen { ABA_INATIVA } else { ABA_ATIVA }, true);
    // O ♪ vira ✕ com a folha aberta. Os dois são desenhados com formas, não com letras: assim não
    // dependem de a fonte embutida ter esses símbolos.
    let folha = app.folha_aberta();
    botao_da_tela(tela.botao_som, "", t, if folha { ABA_ATIVA } else { ABA_INATIVA }, true);
    let icone = encolher(tela.botao_som, 0.2 * tela.botao_som.h);
    if folha {
        desenhar_xis(icone, WHITE);
    } else {
        desenhar_nota(icone, WHITE);
    }

    for (rotulo, valor, area) in [
        ("PONTOS", &painel.pontos, tela.pontos),
        ("NÍVEL", &painel.nivel, tela.nivel),
        ("MELHOR", &painel.melhor, tela.melhor),
    ] {
        draw_rectangle(area.x, area.y, area.w, area.h, PAINEL);
        let em_cima = Rect { h: 0.38 * area.h, ..area };
        let embaixo = Rect { y: area.y + 0.38 * area.h, h: 0.62 * area.h, ..area };
        escrever(rotulo, em_cima, 0.62 * t, TEXTO_SUAVE, Alinhamento::Centro);
        escrever(valor, embaixo, 1.5 * t, WHITE, Alinhamento::Centro);
    }

    let barra = tela.barra;
    draw_rectangle(barra.x, barra.y, barra.w, barra.h, ABA_INATIVA);
    draw_rectangle(barra.x, barra.y, barra.w * painel.progresso, barra.h, BARRA_PREENCHIDA);
    escrever(&painel.rotulo, tela.rotulo, 0.78 * t, TEXTO_SUAVE, Alinhamento::Centro);
    if let Some(ganho) = &painel.ganho {
        escrever(ganho, tela.rotulo, 0.9 * t, DESTAQUE, Alinhamento::Esquerda);
    }
}

/// O aviso ("Supernova", "Tabuleiro reorganizado") numa pílula por cima do tabuleiro.
fn desenhar_aviso(texto: &str, tela: &TelaJogo) {
    let tamanho = tela.texto;
    if tamanho < 1.0 {
        return;
    }
    let medida = measure_text(texto, None, tamanho as u16, 1.0);
    let margem = 0.7 * tamanho;
    let largura = (medida.width + 2.0 * margem).min(tela.tabuleiro.w);
    let altura = 2.2 * tamanho;
    let area = Rect {
        x: tela.tabuleiro.x + (tela.tabuleiro.w - largura) / 2.0,
        y: tela.tabuleiro.y + 0.08 * tela.tabuleiro.h,
        w: largura,
        h: altura,
    };
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.04, 0.06, 0.16, 0.85));
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, BORDA);
    escrever(texto, Rect { x: area.x + margem, w: area.w - 2.0 * margem, ..area }, tamanho, WHITE, Alinhamento::Centro);
}

/// A faixa "Nível N" no meio do tabuleiro, crescendo e sumindo conforme `t` (ms).
fn desenhar_faixa_de_nivel(nivel: u32, t: f32, tela: &TelaJogo) {
    let (escala, alfa) = nivel_faixa(t);
    let tabuleiro = tela.tabuleiro;
    let largura = 0.8 * tabuleiro.w * escala;
    let altura = 0.2 * tabuleiro.h * escala;
    let area = Rect {
        x: tabuleiro.x + (tabuleiro.w - largura) / 2.0,
        y: tabuleiro.y + (tabuleiro.h - altura) / 2.0,
        w: largura,
        h: altura,
    };
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.04, 0.06, 0.16, 0.95 * alfa));
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 3.0, com_alfa(DESTAQUE, alfa));
    escrever(&format!("Nível {nivel}"), area, 2.4 * tela.texto * escala, com_alfa(DESTAQUE, alfa), Alinhamento::Centro);
}

/// O cartão de "Fim de partida", com o tabuleiro escurecido atrás.
fn desenhar_fim(nivel: &str, pontos: &str, tela: &TelaJogo) {
    let t = tela.texto;
    let tabuleiro = tela.tabuleiro;
    draw_rectangle(tabuleiro.x, tabuleiro.y, tabuleiro.w, tabuleiro.h, Color::new(0.0, 0.0, 0.0, 0.55));
    let cartao = tela.cartao_fim;
    draw_rectangle(cartao.x, cartao.y, cartao.w, cartao.h, CARTAO);
    draw_rectangle_lines(cartao.x, cartao.y, cartao.w, cartao.h, 2.0, BORDA);
    let linha = |de: f32, ate: f32| Rect {
        x: cartao.x + 0.05 * cartao.w,
        y: cartao.y + de * cartao.h,
        w: 0.9 * cartao.w,
        h: (ate - de) * cartao.h,
    };
    escrever("Fim de partida", linha(0.05, 0.26), 1.5 * t, WHITE, Alinhamento::Centro);
    escrever("Não há mais jogadas possíveis nesta partida.", linha(0.27, 0.41), 0.72 * t, TEXTO_SUAVE, Alinhamento::Centro);
    escrever(&format!("Nível {nivel}   |   {pontos} pontos"), linha(0.43, 0.62), 1.1 * t, DESTAQUE, Alinhamento::Centro);
    botao_da_tela(tela.botao_jogar_novamente, "Jogar novamente", 1.0 * t, ABA_ATIVA, true);
}

/// A pergunta "Começar novo jogo?", com a tela inteira escurecida atrás.
fn desenhar_dialogo(tela: &TelaJogo) {
    let t = tela.texto;
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.6));
    let dialogo = tela.dialogo;
    draw_rectangle(dialogo.x, dialogo.y, dialogo.w, dialogo.h, CARTAO);
    draw_rectangle_lines(dialogo.x, dialogo.y, dialogo.w, dialogo.h, 2.0, BORDA);
    let linha = |de: f32, ate: f32| Rect {
        x: dialogo.x + 0.05 * dialogo.w,
        y: dialogo.y + de * dialogo.h,
        w: 0.9 * dialogo.w,
        h: (ate - de) * dialogo.h,
    };
    escrever("Começar novo jogo?", linha(0.08, 0.32), 1.35 * t, WHITE, Alinhamento::Centro);
    escrever("O progresso da partida atual será substituído.", linha(0.34, 0.50), 0.72 * t, TEXTO_SUAVE, Alinhamento::Centro);
    botao_da_tela(tela.dialogo_cancelar, "Cancelar", t, ABA_INATIVA, true);
    botao_da_tela(tela.dialogo_confirmar, "Novo jogo", t, ABA_ATIVA, true);
}

// ---- Folha de Som e guia de respiração -----------------------------------------------------------

/// `area` com `margem` pixels a menos em cada lado.
fn encolher(area: Rect, margem: f32) -> Rect {
    Rect { x: area.x + margem, y: area.y + margem, w: (area.w - 2.0 * margem).max(0.0), h: (area.h - 2.0 * margem).max(0.0) }
}

/// Uma colcheia (♪) dentro do quadrado `area`: cabeça redonda, haste e bandeirinha.
fn desenhar_nota(area: Rect, cor: Color) {
    let lado = area.w.min(area.h);
    let (cx, cy) = (area.x + 0.36 * lado + (area.w - lado) / 2.0, area.y + 0.76 * lado + (area.h - lado) / 2.0);
    let raio = 0.17 * lado;
    let haste_x = cx + raio - 0.04 * lado;
    let topo = cy - 0.72 * lado;
    draw_circle(cx, cy, raio, cor);
    draw_line(haste_x, cy, haste_x, topo, 0.08 * lado, cor);
    draw_triangle(vec2(haste_x, topo), vec2(haste_x + 0.30 * lado, topo + 0.26 * lado), vec2(haste_x, topo + 0.30 * lado), cor);
}

/// Um X (✕) dentro do quadrado `area`.
fn desenhar_xis(area: Rect, cor: Color) {
    let grossura = 0.12 * area.w.min(area.h);
    draw_line(area.x, area.y, area.x + area.w, area.y + area.h, grossura, cor);
    draw_line(area.x + area.w, area.y, area.x, area.y + area.h, grossura, cor);
}

/// Uma linha da folha: retângulo escuro com borda.
fn desenhar_cartao(area: Rect) {
    draw_rectangle(area.x, area.y, area.w, area.h, CARTAO);
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 1.5, com_alfa(BORDA, 0.6));
}

/// O interruptor (liga/desliga) encostado na direita de `area`, no meio na vertical.
fn desenhar_interruptor(area: Rect, ligado: bool) {
    let altura = 0.46 * area.h;
    let largura = 1.8 * altura;
    let x = area.x + area.w - 0.18 * area.h - largura;
    let y = area.y + (area.h - altura) / 2.0;
    let raio = altura / 2.0;
    let fundo = if ligado { BARRA_PREENCHIDA } else { INTERRUPTOR_APAGADO };
    draw_circle(x + raio, y + raio, raio, fundo);
    draw_circle(x + largura - raio, y + raio, raio, fundo);
    draw_rectangle(x + raio, y, largura - 2.0 * raio, altura, fundo);
    let (bolinha_x, cor) = if ligado { (x + largura - raio, WHITE) } else { (x + raio, TEXTO_SUAVE) };
    draw_circle(bolinha_x, y + raio, 0.74 * raio, cor);
}

/// Uma linha "nome ......... [interruptor]".
fn desenhar_linha_com_interruptor(area: Rect, rotulo: &str, ligado: bool, t: f32) {
    desenhar_cartao(area);
    let texto = Rect { x: area.x + 0.18 * area.h, w: area.w - 0.18 * area.h - 0.46 * 1.8 * area.h - 0.5 * area.h, ..area };
    escrever(rotulo, texto, 0.95 * t, WHITE, Alinhamento::Esquerda);
    desenhar_interruptor(area, ligado);
}

/// O botão que passa para a próxima opção: o nome atual e um triângulo "para a frente".
fn desenhar_botao_de_escolha(area: Rect, nome: &str, t: f32) {
    botao_da_tela(area, "", t, ABA_INATIVA, true);
    let texto = Rect { x: area.x + 0.18 * area.h, w: area.w - 0.18 * area.h - 0.72 * area.h, ..area };
    escrever(nome, texto, 0.95 * t, WHITE, Alinhamento::Esquerda);
    let (cx, cy, r) = (area.x + area.w - 0.36 * area.h, area.y + area.h / 2.0, 0.13 * area.h);
    draw_triangle(vec2(cx - 0.7 * r, cy - r), vec2(cx - 0.7 * r, cy + r), vec2(cx + r, cy), TEXTO_SUAVE);
}

/// A linha de volume. Ela toda é a barra: o começo é 0 e o fim é 100 (é assim que o dedo é
/// lido). Em cima o nome e o número; embaixo o traço, que vai de ponta a ponta, com a bolinha
/// na posição do volume.
fn desenhar_linha_de_volume(area: Rect, rotulo: &str, volume: u8, t: f32) {
    desenhar_cartao(area);
    let margem = 0.18 * area.h;
    let texto = Rect { x: area.x + margem, y: area.y + 0.06 * area.h, w: area.w - 2.0 * margem, h: 0.5 * area.h };
    let nome = Rect { w: 0.62 * texto.w, ..texto };
    let numero = Rect { x: texto.x + 0.62 * texto.w, w: 0.38 * texto.w, ..texto };
    escrever(rotulo, nome, 0.95 * t, WHITE, Alinhamento::Esquerda);
    escrever(&format!("{volume}%"), numero, 0.95 * t, DESTAQUE, Alinhamento::Direita);

    let espessura = (0.12 * area.h).max(3.0);
    let y = area.y + 0.76 * area.h;
    let fracao = f32::from(volume.min(100)) / 100.0;
    draw_rectangle(area.x, y - espessura / 2.0, area.w, espessura, TRILHO);
    draw_rectangle(area.x, y - espessura / 2.0, area.w * fracao, espessura, BARRA_PREENCHIDA);
    draw_circle(area.x + area.w * fracao, y, 0.11 * area.h, WHITE);
}

/// Quebra `texto` em linhas de no máximo `largura` pixels, nas palavras (uma palavra maior que a
/// largura fica sozinha na linha).
fn quebrar_em_linhas(texto: &str, largura: f32, tamanho: f32) -> Vec<String> {
    let mut linhas = Vec::new();
    let mut atual = String::new();
    for palavra in texto.split_whitespace() {
        let tentativa = if atual.is_empty() { palavra.to_string() } else { format!("{atual} {palavra}") };
        if atual.is_empty() || measure_text(&tentativa, None, tamanho as u16, 1.0).width <= largura {
            atual = tentativa;
        } else {
            linhas.push(std::mem::replace(&mut atual, palavra.to_string()));
        }
    }
    if !atual.is_empty() {
        linhas.push(atual);
    }
    linhas
}

/// Escreve `texto` em várias linhas dentro de `area`: a letra começa em `tamanho` e encolhe até o
/// texto todo caber (na largura e na altura).
fn escrever_em_linhas(texto: &str, area: Rect, tamanho: f32, cor: Color) {
    let mut tamanho = tamanho;
    let mut linhas = quebrar_em_linhas(texto, area.w, tamanho);
    while tamanho > 6.0 && linhas.len() as f32 * 1.2 * tamanho > area.h {
        tamanho *= 0.92;
        linhas = quebrar_em_linhas(texto, area.w, tamanho);
    }
    let altura_da_linha = 1.2 * tamanho;
    let topo = area.y + (area.h - linhas.len() as f32 * altura_da_linha) / 2.0;
    for (i, linha) in linhas.iter().enumerate() {
        let faixa = Rect { y: topo + i as f32 * altura_da_linha, h: altura_da_linha, ..area };
        escrever(linha, faixa, tamanho, cor, Alinhamento::Esquerda);
    }
}

/// A folha de Som, por cima do tabuleiro e dos botões. Só desenha o que o `App` já decidiu
/// (valores, interruptores, avisos); o lugar de cada coisa vem de `tela.som`, o mesmo que serve
/// para ler o toque.
fn desenhar_folha_de_som(app: &App, tela: &TelaJogo) {
    let som = &tela.som;
    if som.folha.w <= 0.0 || som.folha.h <= 0.0 {
        return;
    }
    let t = som.texto;
    let preferencias = app.preferencias();
    let folha = som.folha;
    draw_rectangle(folha.x, folha.y, folha.w, folha.h, FOLHA);
    draw_rectangle_lines(folha.x, folha.y, folha.w, folha.h, 2.0, BORDA);

    // No lugar do título aparece o aviso (o do Testar ou o de áudio), se houver.
    match app.aviso_da_folha() {
        Some(aviso) => escrever_em_linhas(aviso, som.titulo, 0.9 * t, DESTAQUE),
        None => escrever("Som", som.titulo, 1.5 * t, WHITE, Alinhamento::Esquerda),
    }
    botao_da_tela(som.fechar, "", t, ABA_INATIVA, true);
    desenhar_xis(encolher(som.fechar, 0.3 * som.fechar.h), WHITE);

    desenhar_linha_com_interruptor(som.sons_do_jogo, "Sons do jogo", preferencias.sons_do_jogo, t);
    botao_da_tela(som.testar, "Testar som", t, ABA_ATIVA, true);

    desenhar_linha_de_volume(som.barra_efeitos, "Volume dos efeitos", preferencias.volumes.efeitos, t);

    if let (Some(liga), Some(escolhe)) = (som.liga_musica, som.escolhe_musica) {
        desenhar_linha_com_interruptor(liga, "Música", preferencias.musica_ligada, t);
        desenhar_botao_de_escolha(escolhe, preferencias.musica.texto(), t);
    }
    desenhar_linha_de_volume(som.barra_musica, "Volume da música", preferencias.volumes.musica, t);

    if let (Some(liga), Some(escolhe)) = (som.liga_ambiente, som.escolhe_ambiente) {
        desenhar_linha_com_interruptor(liga, "Ambiente", preferencias.ambiente_ligado, t);
        desenhar_botao_de_escolha(escolhe, preferencias.ambiente.texto(), t);
    }
    desenhar_linha_de_volume(som.barra_ambiente, "Volume do ambiente", preferencias.volumes.ambiente, t);

    if let Some(area) = som.respiracao {
        desenhar_cartao(area);
        let margem = 0.18 * area.h;
        let dentro = Rect { x: area.x + margem, w: area.w - 2.0 * margem, ..area };
        let valor = preferencias.respiracao.texto();
        let ligada = valor != crate::preferencias::Respiracao::Desligada.texto();
        escrever("Respiração", Rect { w: 0.36 * dentro.w, ..dentro }, 0.95 * t, WHITE, Alinhamento::Esquerda);
        let cor = if ligada { DESTAQUE } else { TEXTO_SUAVE };
        escrever(valor, Rect { x: dentro.x + 0.38 * dentro.w, w: 0.62 * dentro.w, ..dentro }, 0.95 * t, cor, Alinhamento::Direita);
    }
}

/// O guia de respiração, na linha abaixo dos botões: a bolinha que cresce e encolhe, a palavra
/// ("Inspire" ou "Expire") e os segundos que faltam. Desligado (ou no Clássico), não desenha nada.
fn desenhar_guia_de_respiracao(app: &App, tela: &TelaJogo) {
    let area = tela.guia_respiracao;
    let Some(estado) = app.guia_respiracao() else { return };
    if area.w <= 0.0 || area.h <= 0.0 {
        return;
    }
    let t = tela.texto;
    let (escala, opacidade) = orbe(&estado);
    let lado = area.h;
    let largura_do_texto = 0.34 * area.w;
    let grupo = lado + 0.4 * lado + largura_do_texto;
    let x0 = area.x + (area.w - grupo) / 2.0;
    let (cx, cy) = (x0 + lado / 2.0, area.y + area.h / 2.0);
    let raio = 0.34 * lado * escala;
    draw_circle(cx, cy, 1.25 * raio, com_alfa(BARRA_PREENCHIDA, 0.2 * opacidade));
    draw_circle(cx, cy, raio, com_alfa(BARRA_PREENCHIDA, opacidade));

    let texto_x = x0 + lado + 0.4 * lado;
    let em_cima = Rect { x: texto_x, y: area.y, w: largura_do_texto, h: 0.55 * area.h };
    let embaixo = Rect { y: area.y + 0.55 * area.h, h: 0.40 * area.h, ..em_cima };
    escrever(estado.fase.texto(), em_cima, 1.1 * t, WHITE, Alinhamento::Esquerda);
    escrever(&segundos_restantes(estado.restante_s), embaixo, 0.8 * t, TEXTO_SUAVE, Alinhamento::Esquerda);
}

/// Desenha a tela de jogo inteira: abas, painel, tabuleiro, botões e, por cima, avisos, o
/// cartão de fim de partida e a pergunta de novo jogo. Tudo vem do `App` (e do `Painel` que ele
/// monta); aqui só se escolhe cor e posição.
pub fn draw_jogo(app: &App, sprites: &Sprites, tela: &TelaJogo) {
    clear_background(FUNDO);
    let painel = app.painel();
    desenhar_cabecalho(app, &painel, tela);

    let tabuleiro = tela.tabuleiro;
    draw_rectangle(tabuleiro.x, tabuleiro.y, tabuleiro.w, tabuleiro.h, PAINEL);
    desenhar_fundos_de_realce(app, tabuleiro);
    desenhar_pedras(app, sprites, tabuleiro);
    desenhar_contornos_de_realce(app, tabuleiro);
    if let Some(aviso) = &painel.aviso {
        desenhar_aviso(aviso, tela);
    }
    if let Some((crate::linha::Passo::Nivel { nivel }, t)) = app.passo_atual() {
        desenhar_faixa_de_nivel(*nivel, t, tela);
    }

    // Os botões de baixo. Com a partida acabada, a Dica fica apagada.
    let t = tela.texto;
    if let (true, Some(embaralhar)) = (painel.embaralhar_visivel, tela.botao_embaralhar) {
        botao_da_tela(embaralhar, "Embaralhar", t, ABA_INATIVA, true);
    }
    botao_da_tela(tela.botao_dica, "Dica", t, ABA_INATIVA, painel.dica_ativa);
    botao_da_tela(tela.botao_novo_jogo, "Novo jogo", t, ABA_INATIVA, true);

    desenhar_guia_de_respiracao(app, tela);

    if let Some((nivel, pontos)) = &painel.fim {
        desenhar_fim(nivel, pontos, tela);
    }
    if app.folha_aberta() {
        desenhar_folha_de_som(app, tela);
    }
    if painel.dialogo {
        desenhar_dialogo(tela);
    }
}
