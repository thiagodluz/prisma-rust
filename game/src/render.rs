//! Desenho: pega o estado do motor e pinta na tela com as imagens das pedras.
//!
//! Este módulo precisa de uma janela aberta (o Macroquad guarda as imagens na placa de
//! vídeo), por isso não tem testes automáticos: quem confere é a "foto" (`--screenshot`).

use std::collections::HashMap;

use macroquad::prelude::*;
use prisma_engine::{position, Board, Tile, SIZE};

use crate::app::{App, Folha, Painel};
use crate::assets::{all_sprites, png_bytes, sprite_for, Sprite, FUNDO_JPG, MARCA_PNG, SPRITE_RATIO};
use crate::texto::{self, Estilo};
use crate::formas::{self, cobrir, expandir, nove_fatias, pintar, rgba, Receita};
use crate::icones::{self, Linha};
use crate::layout::{self, board_rect, cell_rect, Rect, TelaDiagnostico, TelaJogo};
use crate::linha::{
    deslocamento_troca, embaralhar_alfa, fusao_da_casa, fusao_ms, limpeza_da_casa, linhas_de_queda, nivel_faixa,
    queda_deslocamento, queda_ms, resgate_da_casa, Passo, TipoVisual,
};
use crate::menu::{blocos_de_recordes, creditos};
use crate::particulas::{ajustar, particulas, realce_da_faixa, Forma as FormaDaParticula};
use crate::preferencias::{orbe, segundos_restantes, Intensidade};

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
    formas: Formas,
}

/// Uma forma desenhada por código (retângulo arredondado com degradê, borda e sombra), pintada
/// UMA vez em `Formas::load` e guardada como textura. Para ocupar qualquer tamanho sem entortar
/// os cantos, `desenhar_forma` a corta em nove fatias.
struct Forma {
    textura: Texture2D,
    /// O lado da imagem, em pixels dela.
    lado: f32,
    /// O tamanho do canto (sombra + raio) na imagem, em pixels dela.
    canto: f32,
}

impl Forma {
    fn nova(receita: Receita) -> Forma {
        let lado = receita.lado as u16;
        let textura = Texture2D::from_rgba8(lado, lado, &pintar(&receita));
        textura.set_filter(FilterMode::Linear);
        Forma { textura, lado: receita.lado as f32, canto: (receita.sombra + receita.raio) as f32 }
    }
}

/// A foto de fundo e as formas do tabuleiro, prontas na placa de vídeo.
struct Formas {
    fundo: Texture2D,
    moldura: Forma,
    /// O painel escuro que fica dentro da moldura.
    grade: Forma,
    /// As duas casas do tabuleiro, de tons um pouco diferentes (listras verticais, como no JS).
    casa_par: Forma,
    casa_impar: Forma,
    /// O cartão de vidro do painel de pontos.
    cartao: Forma,
    /// Um retângulo arredondado branco e liso, para ser tingido na hora de desenhar.
    solida: Forma,
    /// O fundo dos botões de baixo (Embaralhar, Dica, Novo jogo).
    botao: Forma,
    /// O fundo dos botões quadrados do topo (♫ e ≡), e o dele quando a folha está aberta.
    botao_icone: Forma,
    botao_icone_ativo: Forma,
    /// O logo do topo.
    marca: Texture2D,
    /// Os cartões de modo (Zen e Clássico): o normal e o escolhido.
    modo: Forma,
    modo_escolhido: Forma,
}

/// Uma cor `[r, g, b, a]` de `formas.rs` como `Color` do Macroquad.
fn cor_de(cor: formas::Cor) -> Color {
    Color::from_rgba(cor[0], cor[1], cor[2], cor[3])
}

impl Formas {
    /// As receitas são as do CSS "illustrated" do Prisma em JS (`style.css`): `lado` é o tamanho
    /// da imagem, `raio` e `borda` e `sombra` estão em pixels dela.
    fn load() -> Formas {
        let fundo = Texture2D::from_file_with_format(FUNDO_JPG, None);
        fundo.set_filter(FilterMode::Linear);
        let marca = Texture2D::from_file_with_format(MARCA_PNG, None);
        marca.set_filter(FilterMode::Linear);
        let sem_borda = rgba(0);
        let casa = |degrade| Receita {
            lado: 64,
            raio: 12,
            borda: 0,
            cor_da_borda: sem_borda,
            degrade,
            angulo: 155.0,
            sombra: 0,
            cor_da_sombra: sem_borda,
        };
        Formas {
            fundo,
            moldura: Forma::nova(Receita {
                lado: 168,
                raio: 22,
                borda: 2,
                cor_da_borda: rgba(0xcee6ff5c),
                degrade: [rgba(0x7790bd65), rgba(0x202a4eff), rgba(0x785b9b72)],
                angulo: 145.0,
                sombra: 20,
                cor_da_sombra: rgba(0x030616bd),
            }),
            grade: Forma::nova(Receita {
                lado: 64,
                raio: 14,
                borda: 0,
                cor_da_borda: sem_borda,
                degrade: [rgba(0x0d142cff); 3],
                angulo: 145.0,
                sombra: 0,
                cor_da_sombra: sem_borda,
            }),
            casa_par: Forma::nova(casa([rgba(0xbfd5ff19), rgba(0x869ec912), rgba(0x4d67940b)])),
            casa_impar: Forma::nova(casa([rgba(0xc8c1ff1b), rgba(0x8a94c913), rgba(0x4d67940b)])),
            cartao: Forma::nova(Receita {
                lado: 64,
                raio: 12,
                borda: 1,
                cor_da_borda: rgba(0xffffff17),
                degrade: [rgba(0xffffff0d); 3],
                angulo: 145.0,
                sombra: 0,
                cor_da_sombra: sem_borda,
            }),
            botao_icone: Forma::nova(Receita {
                lado: 64,
                raio: 12,
                borda: 1,
                cor_da_borda: rgba(0xffffff27),
                degrade: [rgba(0xffffff10); 3],
                angulo: 145.0,
                sombra: 0,
                cor_da_sombra: sem_borda,
            }),
            botao_icone_ativo: Forma::nova(Receita {
                lado: 64,
                raio: 12,
                borda: 1,
                cor_da_borda: rgba(0xc4b2ffff),
                degrade: [rgba(0xac9aff38); 3],
                angulo: 145.0,
                sombra: 0,
                cor_da_sombra: sem_borda,
            }),
            marca,
            modo: Forma::nova(Receita {
                lado: 64,
                raio: 12,
                borda: 1,
                cor_da_borda: rgba(0xffffff23),
                degrade: [rgba(0xffffff0b); 3],
                angulo: 145.0,
                sombra: 0,
                cor_da_sombra: sem_borda,
            }),
            modo_escolhido: Forma::nova(Receita {
                lado: 64,
                raio: 12,
                borda: 1,
                cor_da_borda: rgba(0xad9bff99),
                degrade: [rgba(0x756aad32); 3],
                angulo: 145.0,
                sombra: 0,
                cor_da_sombra: sem_borda,
            }),
            botao: Forma::nova(Receita {
                lado: 64,
                raio: 12,
                borda: 1,
                cor_da_borda: rgba(0xb8a7ea70),
                degrade: [rgba(0x60538763), rgba(0x443e6988), rgba(0x292a4cad)],
                angulo: 155.0,
                sombra: 0,
                cor_da_sombra: sem_borda,
            }),
            solida: Forma::nova(Receita {
                lado: 64,
                raio: 12,
                borda: 0,
                cor_da_borda: sem_borda,
                degrade: [rgba(0xffffffff); 3],
                angulo: 145.0,
                sombra: 0,
                cor_da_sombra: sem_borda,
            }),
        }
    }
}

/// Desenha `forma` cobrindo `area`, com cantos de `raio` pixels e uma sombra de `sombra` pixels
/// em volta (a sombra fica FORA de `area`).
fn desenhar_forma(forma: &Forma, area: Rect, raio: f32, sombra: f32) {
    desenhar_forma_tingida(forma, area, raio, sombra, WHITE);
}

/// Como `desenhar_forma`, mas multiplicando a cor da forma por `tinta` (a forma `solida`, que é
/// branca, vira qualquer cor).
fn desenhar_forma_tingida(forma: &Forma, area: Rect, raio: f32, sombra: f32, tinta: Color) {
    let destino = expandir(area, sombra);
    for fatia in nove_fatias(forma.lado, forma.canto, destino, sombra + raio) {
        if fatia.destino.w <= 0.0 || fatia.destino.h <= 0.0 {
            continue;
        }
        let (o, d) = (fatia.origem, fatia.destino);
        draw_texture_ex(
            &forma.textura,
            d.x,
            d.y,
            tinta,
            DrawTextureParams {
                dest_size: Some(vec2(d.w, d.h)),
                source: Some(macroquad::math::Rect::new(o.x, o.y, o.w, o.h)),
                ..Default::default()
            },
        );
    }
}

/// Um retângulo com cor diferente em cada canto (o degradê vem da mistura entre eles):
/// `cores` é cima-esquerda, cima-direita, baixo-direita, baixo-esquerda.
fn desenhar_degrade(area: Rect, cores: [Color; 4]) {
    let cantos = [(area.x, area.y), (area.x + area.w, area.y), (area.x + area.w, area.y + area.h), (area.x, area.y + area.h)];
    let vertices = cantos.iter().zip(cores).map(|(&(x, y), cor)| Vertex::new(x, y, 0.0, 0.0, 0.0, cor)).collect();
    draw_mesh(&Mesh { vertices, indices: vec![0, 1, 2, 0, 2, 3], texture: None });
}

/// A foto de fundo cobrindo a tela toda (cortando o que sobra, sem deformar), com o véu escuro
/// por cima: mais leve em cima e mais pesado embaixo, como no JS.
fn desenhar_fundo(formas: &Formas) {
    let (largura, altura) = (screen_width(), screen_height());
    let recorte = cobrir(formas.fundo.width(), formas.fundo.height(), largura, altura);
    draw_texture_ex(
        &formas.fundo,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(largura, altura)),
            source: Some(macroquad::math::Rect::new(recorte.x, recorte.y, recorte.w, recorte.h)),
            ..Default::default()
        },
    );
    let (cima, baixo) = (cor_de(rgba(0x0b102242)), cor_de(rgba(0x0b10226b)));
    desenhar_degrade(Rect { x: 0.0, y: 0.0, w: largura, h: altura }, [cima, cima, baixo, baixo]);
}

/// A moldura brilhante, o painel escuro e as 64 casas do tabuleiro.
fn desenhar_tabuleiro_vazio(formas: &Formas, tela: &TelaJogo) {
    let moldura = tela.tabuleiro;
    desenhar_forma(&formas.moldura, moldura, 0.059 * moldura.w, 0.05 * moldura.w);
    desenhar_forma(&formas.grade, layout::fundo_da_grade(moldura), 0.04 * moldura.w, 0.0);
    for linha in 0..SIZE {
        for coluna in 0..SIZE {
            let passo = cell_rect(tela.grade, linha, coluna);
            let lado = 0.90 * passo.w;
            let casa = Rect { x: passo.x + (passo.w - lado) / 2.0, y: passo.y + (passo.h - lado) / 2.0, w: lado, h: lado };
            // O JS conta as casas a partir de 1; a "ímpar" do CSS é a de índice par daqui.
            let forma = if (linha * SIZE + coluna) % 2 == 0 { &formas.casa_impar } else { &formas.casa_par };
            desenhar_forma(forma, casa, 0.20 * lado, 0.0);
        }
    }
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
        texto::carregar();
        Sprites { texturas, formas: Formas::load() }
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
    escrever_com_estilo(texto, area, tamanho, cor, alinhamento, Estilo::Texto);
}

/// Como `escrever`, no estilo dado (`TextoForte` para rótulos de botão e valores).
fn escrever_com_estilo(texto: &str, area: Rect, tamanho: f32, cor: Color, alinhamento: Alinhamento, estilo: Estilo) {
    if tamanho < 1.0 || area.w <= 0.0 {
        return;
    }
    let tamanho = texto::tamanho_que_cabe(texto::medir(texto, tamanho, estilo).width, area.w, tamanho);
    if tamanho < 1.0 {
        return;
    }
    let medida = texto::medir(texto, tamanho, estilo);
    // A altura vem de uma palavra de referência, para o texto não "pular" de acordo com as letras.
    let referencia = texto::medir("Ag", tamanho, estilo);
    let x = match alinhamento {
        Alinhamento::Esquerda => area.x,
        Alinhamento::Centro => area.x + (area.w - medida.width) / 2.0,
        Alinhamento::Direita => area.x + area.w - medida.width,
    };
    let y = area.y + (area.h - referencia.height) / 2.0 + referencia.offset_y;
    texto::desenhar(texto, x, y, tamanho, cor, estilo);
}

/// Um botão da tela de jogo: retângulo com borda e rótulo no meio. `ativo` falso apaga o botão.
fn botao_da_tela(area: Rect, rotulo: &str, tamanho: f32, fundo: Color, ativo: bool) {
    let alfa = if ativo { 1.0 } else { 0.4 };
    draw_rectangle(area.x, area.y, area.w, area.h, com_alfa(fundo, alfa));
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, com_alfa(BORDA, alfa));
    let margem = 0.06 * area.w;
    let dentro = Rect { x: area.x + margem, y: area.y, w: area.w - 2.0 * margem, h: area.h };
    escrever_com_estilo(rotulo, dentro, tamanho, com_alfa(WHITE, alfa), Alinhamento::Centro, Estilo::TextoForte);
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
        Passo::Limpeza { tabuleiro, celulas, ativadas, intensidade, visuais, .. } => {
            // Primeiro a fusão da Supernova (as pedras de origem pulsam), depois as pedras somem.
            let fusao = fusao_ms(passo);
            let t_da_limpeza = t - fusao;
            let origens: Vec<usize> = visuais
                .iter()
                .filter(|visual| visual.tipo == TipoVisual::Supernova)
                .flat_map(|visual| visual.origens.iter().copied())
                .collect();
            // Cada casa: (escala, opacidade, brilho de 0 a 1).
            let estado = |indice: usize| -> (f32, f32, f32) {
                if t < fusao && origens.contains(&indice) {
                    let (escala, brilho) = fusao_da_casa(t, *intensidade);
                    (escala, 1.0, brilho)
                } else if celulas.contains(&indice) {
                    limpeza_da_casa(ativadas, indice, t_da_limpeza, *intensidade)
                } else {
                    (1.0, 1.0, 0.0)
                }
            };
            desenhar_casas(sprites, tabuleiro, area, |indice, _| {
                let (escala, alfa, _) = estado(indice);
                Some((escala, alfa, (0.0, 0.0)))
            });
            // O brilho da pedra: uma luz branca translúcida por cima dela (`0,33 × brilho × opacidade`).
            for indice in (0..SIZE * SIZE).filter(|indice| celulas.contains(indice) || origens.contains(indice)) {
                let (escala, alfa, brilho) = estado(indice);
                if brilho <= 0.0 || alfa <= 0.0 {
                    continue;
                }
                // Três círculos, do maior e mais fraco ao menor e mais forte: um brilho sem borda dura.
                let casa = casa_de(area, indice);
                let raio = casa.w * SPRITE_RATIO * escala * 0.5;
                for (fator, forca) in [(1.0, 0.18), (0.72, 0.3), (0.45, 0.5)] {
                    let cor = com_alfa(WHITE, 0.33 * brilho * alfa * forca * 2.0);
                    draw_circle(casa.x + casa.w / 2.0, casa.y + casa.h / 2.0, raio * fator, cor);
                }
            }
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

/// As partículas da limpeza (faíscas, ondas, feixes, véu, anel da Supernova e linhas de fusão),
/// por cima das pedras. A conta é de `particulas::particulas`; aqui só se pinta: as medidas
/// vêm em casas e viram pixels pelo lado de uma casa, e a intensidade escolhida (Suave, Padrão,
/// Vibrante) ajusta cor e opacidade.
fn desenhar_particulas(passo: &Passo, t_ms: f32, area: Rect) {
    let Passo::Limpeza { celulas, visuais, intensidade, .. } = passo else { return };
    let lado = area.w / SIZE as f32;
    for particula in particulas(visuais, celulas, t_ms, *intensidade) {
        let (cor, alfa) = ajustar(particula.cor, particula.alfa, *intensidade);
        if alfa <= 0.0 {
            continue;
        }
        let cor = Color::new(cor[0], cor[1], cor[2], alfa.clamp(0.0, 1.0));
        let (x, y) = (area.x + particula.x * lado, area.y + particula.y * lado);
        let tamanho = particula.tamanho * lado;
        match particula.forma {
            FormaDaParticula::Faisca => {
                // Um quadrado girado, com um maior e mais fraco por baixo (o "brilho").
                // O miolo usa a raiz do alfa: acende mais cedo e a faísca continua bem visível.
                for (fator, fraqueza) in [(2.4, 0.3), (1.0, 1.0)] {
                    let lado_da_faisca = tamanho * fator;
                    let cor = if fator > 1.0 { cor } else { Color::new(cor.r, cor.g, cor.b, cor.a.sqrt()) };
                    draw_rectangle_ex(
                        x,
                        y,
                        lado_da_faisca,
                        lado_da_faisca,
                        DrawRectangleParams {
                            offset: vec2(0.5, 0.5),
                            rotation: particula.angulo,
                            color: com_alfa(cor, fraqueza),
                        },
                    );
                }
            }
            FormaDaParticula::Anel => draw_circle_lines(x, y, tamanho / 2.0, (particula.espessura * lado).max(1.0), cor),
            FormaDaParticula::Linha => draw_rectangle_ex(
                x,
                y,
                tamanho,
                particula.espessura * lado,
                DrawRectangleParams { offset: vec2(0.0, 0.5), rotation: particula.angulo, color: cor },
            ),
            FormaDaParticula::Veu => draw_rectangle(area.x, area.y, area.w, area.h, cor),
        }
    }
}

/// A casa de número `indice` (0 a 63) do tabuleiro.
fn casa_de(area: Rect, indice: usize) -> Rect {
    cell_rect(area, indice / SIZE, indice % SIZE)
}

/// A parte visível da casa (0,90 do passo da grade, no meio): é onde ficam os realces, o mesmo
/// retângulo que as casas desenhadas em `desenhar_tabuleiro_vazio`.
fn casa_visivel(area: Rect, indice: usize) -> Rect {
    let passo = casa_de(area, indice);
    let lado = 0.90 * passo.w;
    Rect { x: passo.x + (passo.w - lado) / 2.0, y: passo.y + (passo.h - lado) / 2.0, w: lado, h: lado }
}

/// Pinta o fundo de um realce dentro da casa `indice`, arredondado como ela.
fn pintar_realce(formas: &Formas, area: Rect, indice: usize, cor: Color) {
    let casa = casa_visivel(area, indice);
    desenhar_forma_tingida(&formas.solida, casa, 0.20 * casa.w, 0.0, cor);
}

/// Realces que ficam ATRÁS das pedras (para não "sujar" a cor delas): fundo da casa marcada,
/// do dedo em cima, do alvo do arrasto e da Dica.
fn desenhar_fundos_de_realce(app: &App, formas: &Formas, area: Rect) {
    let gestos = app.gestos();
    if let Some(indice) = gestos.selecionada() {
        pintar_realce(formas, area, indice, cor_de(rgba(0xf7edab5c)));
    }
    if let Some(indice) = gestos.pressionada() {
        pintar_realce(formas, area, indice, cor_de(rgba(0xe5d8ff3d)));
    }
    if let Some(indice) = gestos.alvo_do_arrasto() {
        pintar_realce(formas, area, indice, cor_de(rgba(0xc4ffe451)));
    }
    if let Some((a, b)) = app.dica() {
        let pulso = pulso_da_dica();
        for indice in [a, b] {
            pintar_realce(formas, area, indice, com_alfa(cor_de(rgba(0xffe9933d)), pulso));
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
        let casa = casa_visivel(area, indice);
        draw_rectangle_lines(casa.x, casa.y, casa.w, casa.h, 3.0, WHITE);
    }
    if let Some(indice) = gestos.alvo_do_arrasto() {
        let casa = casa_visivel(area, indice);
        draw_rectangle_lines(casa.x, casa.y, casa.w, casa.h, 3.0, com_alfa(WHITE, 0.8));
    }
    if let Some((a, b)) = app.dica() {
        let pulso = pulso_da_dica();
        for indice in [a, b] {
            let casa = casa_visivel(area, indice);
            draw_rectangle_lines(casa.x, casa.y, casa.w, casa.h, 4.0, com_alfa(DESTAQUE, pulso));
        }
    }
}

/// As abas, o painel de números e a barra de progresso.
fn desenhar_cabecalho(app: &App, painel: &Painel, tela: &TelaJogo, formas: &Formas) {
    let t = tela.texto;
    desenhar_topo(formas, app, tela);
    let zen = app.modo() == prisma_engine::Mode::Zen;
    desenhar_cartao_de_modo(formas, tela.aba_zen, Icone::Lua, "Zen", "Sem pressa, sem derrota", zen);
    desenhar_cartao_de_modo(formas, tela.aba_classico, Icone::Faisca, "Clássico", "Sem cronômetro · pode acabar", !zen);

    // O cartão de vidro com as três colunas: pontos à esquerda, nível no meio, melhor à direita.
    let cartao = tela.cartao_pontos;
    desenhar_forma(&formas.cartao, cartao, 0.045 * cartao.w, 0.0);
    for (rotulo, valor, area, cor_do_valor, alinhamento) in [
        ("PONTOS", &painel.pontos, tela.pontos, WHITE, Alinhamento::Esquerda),
        ("NÍVEL", &painel.nivel, tela.nivel, cor_de(rgba(0xa2edddff)), Alinhamento::Centro),
        ("MELHOR", &painel.melhor, tela.melhor, WHITE, Alinhamento::Direita),
    ] {
        let em_cima = Rect { h: 0.38 * area.h, ..area };
        let embaixo = Rect { y: area.y + 0.38 * area.h, h: 0.62 * area.h, ..area };
        escrever(rotulo, em_cima, 0.62 * t, cor_de(rgba(0xb3b0d4ff)), alinhamento);
        escrever_com_estilo(valor, embaixo, 1.5 * t, cor_do_valor, alinhamento, Estilo::Titulo);
    }

    // A barra de nível: trilho claro arredondado e, por cima, o preenchimento em degradê da
    // esquerda (verde-água) para a direita (lilás), com a ponta direita reta.
    let barra = tela.barra;
    desenhar_forma_tingida(&formas.solida, barra, barra.h / 2.0, 0.0, cor_de(rgba(0xffffff17)));
    let progresso = if painel.progresso.is_finite() { painel.progresso.clamp(0.0, 1.0) } else { 0.0 };
    let (esquerda, direita) = (cor_de(rgba(0x6be2d6ff)), cor_de(rgba(0xd09cffff)));
    desenhar_degrade(Rect { w: barra.w * progresso, ..barra }, [esquerda, direita, direita, esquerda]);
    escrever(&painel.rotulo, tela.rotulo, 0.78 * t, cor_de(rgba(0x9996b9ff)), Alinhamento::Direita);
    if let Some(ganho) = &painel.ganho {
        escrever(ganho, tela.rotulo, 0.9 * t, DESTAQUE, Alinhamento::Esquerda);
    } else if painel.sem_salvar {
        // Fixo até voltar a gravar; o "+250" tem a vez quando aparece.
        escrever("Não está salvando", tela.rotulo, 0.78 * t, cor_de(rgba(0xffc069ff)), Alinhamento::Esquerda);
    }
}

/// A linha de topo: o logo e "PRISMA" à esquerda; à direita os botões ♫ e ≡. O ♫ vira ✕ com a
/// folha de Som aberta, e o ≡ vira ✕ com a do Menu aberta. Os símbolos são desenhados com formas,
/// não com letras: as fontes não têm o ♫.
fn desenhar_topo(formas: &Formas, app: &App, tela: &TelaJogo) {
    desenhar_marca(formas, tela.marca);
    let som_aberto = app.folha() == Some(Folha::Som);
    let menu_aberto = app.folha().is_some_and(Folha::e_do_menu);
    for (area, aberto) in [(tela.botao_som, som_aberto), (tela.botao_menu, menu_aberto)] {
        let forma = if aberto { &formas.botao_icone_ativo } else { &formas.botao_icone };
        desenhar_forma(forma, area, 0.316 * area.h, 0.0);
    }
    let cor = if som_aberto { WHITE } else { cor_de(rgba(0xd7d5f8ff)) };
    let icone = encolher(tela.botao_som, 0.26 * tela.botao_som.h);
    if som_aberto {
        desenhar_xis(icone, cor);
    } else {
        desenhar_nota(icone, cor);
    }
    let cor = if menu_aberto { WHITE } else { cor_de(rgba(0xd7d5f8ff)) };
    let icone = encolher(tela.botao_menu, 0.26 * tela.botao_menu.h);
    if menu_aberto {
        desenhar_xis(icone, cor);
    } else {
        desenhar_hamburguer(icone, cor);
    }
}

/// O logo (quadrado, da altura da faixa) e "PRISMA" em negrito, com letras espaçadas (0,22 em).
fn desenhar_marca(formas: &Formas, marca: Rect) {
    if marca.w <= 0.0 || marca.h <= 0.0 {
        return;
    }
    let lado = marca.h;
    draw_texture_ex(
        &formas.marca,
        marca.x,
        marca.y,
        WHITE,
        DrawTextureParams { dest_size: Some(vec2(lado, lado)), ..Default::default() },
    );
    let texto_x = marca.x + lado + 0.26 * lado;
    let disponivel = marca.x + marca.w - texto_x;
    let mut tamanho = 0.5 * lado;
    let largura = |tamanho: f32| -> f32 {
        "PRISMA".chars().map(|letra| texto::medir(&letra.to_string(), tamanho, Estilo::Titulo).width + 0.22 * tamanho).sum::<f32>() - 0.22 * tamanho
    };
    tamanho = texto::tamanho_que_cabe(largura(tamanho), disponivel, tamanho);
    if tamanho < 1.0 {
        return;
    }
    let referencia = texto::medir("Ag", tamanho, Estilo::Titulo);
    let y = marca.y + (marca.h - referencia.height) / 2.0 + referencia.offset_y;
    let mut x = texto_x;
    for letra in "PRISMA".chars() {
        let letra = letra.to_string();
        texto::desenhar(&letra, x, y, tamanho, cor_de(rgba(0xf8f7ffff)), Estilo::Titulo);
        x += texto::medir(&letra, tamanho, Estilo::Titulo).width + 0.22 * tamanho;
    }
}

/// O aviso ("Supernova", "Tabuleiro reorganizado") numa pílula por cima do tabuleiro.
fn desenhar_aviso(texto: &str, tela: &TelaJogo) {
    let tamanho = tela.texto;
    if tamanho < 1.0 {
        return;
    }
    let medida = texto::medir(texto, tamanho, Estilo::TextoForte);
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
    escrever_com_estilo(texto, Rect { x: area.x + margem, w: area.w - 2.0 * margem, ..area }, tamanho, WHITE, Alinhamento::Centro, Estilo::TextoForte);
}

/// A faixa "Nível N" no meio do tabuleiro, crescendo e sumindo conforme `t` (ms).
fn desenhar_faixa_de_nivel(nivel: u32, t: f32, tela: &TelaJogo, intensidade: Intensidade) {
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
    // A sombra e o brilho roxo em volta da faixa mudam com a intensidade (Suave, Padrão, Vibrante).
    let (sombra, brilho) = realce_da_faixa(intensidade);
    let passo = 0.012 * tabuleiro.w;
    for camada in 1..=4 {
        let folga = camada as f32 * passo;
        let cor = Color::new(0.71, 0.64, 1.0, 0.11 * brilho * alfa);
        draw_rectangle(area.x - folga, area.y - folga, area.w + 2.0 * folga, area.h + 2.0 * folga, cor);
    }
    let desce = 0.025 * tabuleiro.h * sombra;
    draw_rectangle(area.x, area.y + desce, area.w, area.h, Color::new(0.1, 0.09, 0.25, 0.45 * sombra * alfa));
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

/// Os desenhos dos três botões de baixo.
#[derive(Clone, Copy)]
enum Icone {
    Embaralhar,
    Dica,
    NovoJogo,
    Lua,
    Faisca,
}

/// Um botão de baixo: fundo arredondado, o ícone no alto e o rótulo embaixo. `ativo` falso deixa
/// tudo a 40% de opacidade.
fn desenhar_botao_de_acao(formas: &Formas, area: Rect, rotulo: &str, icone: Icone, tamanho: f32, ativo: bool) {
    let alfa = if ativo { 1.0 } else { 0.4 };
    desenhar_forma_tingida(&formas.botao, area, 0.21 * area.h, 0.0, com_alfa(WHITE, alfa));
    let cor = com_alfa(cor_de(rgba(0xeeeaffff)), alfa);
    let lado = 0.38 * area.h;
    let quadrado = Rect { x: area.x + (area.w - lado) / 2.0, y: area.y + 0.12 * area.h, w: lado, h: lado };
    let linhas: Vec<Linha> = match icone {
        Icone::Embaralhar => icones::embaralhar(quadrado),
        Icone::Dica => icones::faisca(quadrado),
        Icone::NovoJogo => icones::novo_jogo(quadrado),
        Icone::Lua => icones::lua(quadrado),
        Icone::Faisca => icones::faisca(quadrado),
    };
    let grossura = 0.07 * area.h;
    for linha in &linhas {
        for par in linha.windows(2) {
            draw_line(par[0].0, par[0].1, par[1].0, par[1].1, grossura, cor);
        }
        // Uma bolinha em cada ponto arredonda as quinas e as pontas.
        for &(x, y) in linha {
            draw_circle(x, y, grossura / 2.0, cor);
        }
    }
    let margem = 0.06 * area.w;
    let embaixo = Rect { x: area.x + margem, y: area.y + 0.54 * area.h, w: area.w - 2.0 * margem, h: 0.38 * area.h };
    escrever_com_estilo(rotulo, embaixo, tamanho, cor, Alinhamento::Centro, Estilo::TextoForte);
}

/// Um cartão de modo: a caixa do ícone à esquerda e, ao lado, o nome (negrito) e a frase (menor).
/// `escolhido` troca o fundo e a borda pelos do modo em uso.
fn desenhar_cartao_de_modo(formas: &Formas, area: Rect, icone: Icone, nome: &str, frase: &str, escolhido: bool) {
    let forma = if escolhido { &formas.modo_escolhido } else { &formas.modo };
    desenhar_forma(forma, area, 0.28 * area.h, 0.0);
    let folga = 0.2 * area.h;
    let lado = 0.56 * area.h;
    let caixa = Rect { x: area.x + folga, y: area.y + (area.h - lado) / 2.0, w: lado, h: lado };
    desenhar_forma_tingida(&formas.solida, caixa, 0.30 * lado, 0.0, cor_de(rgba(0x9785e438)));
    let cor_do_icone = cor_de(rgba(0xcfbdffff));
    let grossura = 0.07 * lado;
    for linha in &match icone {
        Icone::Lua => icones::lua(encolher(caixa, 0.2 * lado)),
        _ => icones::faisca(encolher(caixa, 0.2 * lado)),
    } {
        for par in linha.windows(2) {
            draw_line(par[0].0, par[0].1, par[1].0, par[1].1, grossura, cor_do_icone);
        }
    }
    let texto_x = caixa.x + caixa.w + 0.7 * folga;
    let largura = area.x + area.w - folga * 0.6 - texto_x;
    let nome_area = Rect { x: texto_x, y: area.y + 0.14 * area.h, w: largura, h: 0.38 * area.h };
    let frase_area = Rect { x: texto_x, y: area.y + 0.52 * area.h, w: largura, h: 0.32 * area.h };
    escrever_com_estilo(nome, nome_area, 0.30 * area.h, WHITE, Alinhamento::Esquerda, Estilo::TextoForte);
    escrever(frase, frase_area, 0.22 * area.h, cor_de(rgba(0xb9b5d4ff)), Alinhamento::Esquerda);
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

/// Três traços (≡) dentro do quadrado `area`.
fn desenhar_hamburguer(area: Rect, cor: Color) {
    let grossura = 0.14 * area.w.min(area.h);
    for i in 0..3 {
        let y = area.y + area.h * (0.12 + 0.38 * i as f32);
        draw_line(area.x, y, area.x + area.w, y, grossura, cor);
    }
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
        if atual.is_empty() || texto::medir(&tentativa, tamanho, Estilo::Texto).width <= largura {
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

    if let Some(area) = som.vibracao {
        desenhar_linha_com_interruptor(area, "Vibração", preferencias.vibracao, t);
    }

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

    // A intensidade dos efeitos visuais (só no Zen): o nome à esquerda e o valor escolhido à direita.
    if let Some(area) = som.efeitos {
        desenhar_cartao(area);
        let margem = 0.18 * area.h;
        let dentro = Rect { x: area.x + margem, w: area.w - 2.0 * margem, ..area };
        escrever("Intensidade", Rect { w: 0.45 * dentro.w, ..dentro }, 0.95 * t, WHITE, Alinhamento::Esquerda);
        let valor = preferencias.intensidade.texto();
        escrever(valor, Rect { x: dentro.x + 0.47 * dentro.w, w: 0.53 * dentro.w, ..dentro }, 0.95 * t, DESTAQUE, Alinhamento::Direita);
    }
}

/// Escreve `paragrafos` um abaixo do outro, de cima para baixo, quebrando cada um em linhas dentro
/// de `area`; a letra começa em `tamanho` e encolhe até tudo caber (largura e altura), sem rolagem.
/// O primeiro parágrafo sai em branco e os outros em `cor`.
fn escrever_paragrafos(paragrafos: &[String], area: Rect, tamanho: f32, cor: Color) {
    let medir = |tamanho: f32| -> (Vec<Vec<String>>, f32) {
        let blocos: Vec<Vec<String>> = paragrafos.iter().map(|texto| quebrar_em_linhas(texto, area.w, tamanho)).collect();
        let linhas: usize = blocos.iter().map(Vec::len).sum();
        let vaos = blocos.len().saturating_sub(1) as f32 * 0.5 * tamanho;
        (blocos, linhas as f32 * 1.2 * tamanho + vaos)
    };
    let mut tamanho = tamanho;
    let (mut blocos, mut altura) = medir(tamanho);
    while tamanho > 6.0 && altura > area.h {
        tamanho *= 0.94;
        (blocos, altura) = medir(tamanho);
    }
    let mut y = area.y;
    for (i, bloco) in blocos.iter().enumerate() {
        for linha in bloco {
            let faixa = Rect { y, h: 1.2 * tamanho, ..area };
            escrever(linha, faixa, tamanho, if i == 0 { WHITE } else { cor }, Alinhamento::Esquerda);
            y += 1.2 * tamanho;
        }
        y += 0.5 * tamanho;
    }
}

/// A folha do Menu, no mesmo lugar da de Som: as abas Recordes e Créditos, o ✕ e, embaixo, o
/// conteúdo da aba aberta (`aba`). Os Recordes são os do modo em que o jogador está.
fn desenhar_folha_do_menu(app: &App, aba: Folha, tela: &TelaJogo) {
    let menu = &tela.menu;
    if menu.folha.w <= 0.0 || menu.folha.h <= 0.0 {
        return;
    }
    let t = menu.texto;
    let folha = menu.folha;
    draw_rectangle(folha.x, folha.y, folha.w, folha.h, FOLHA);
    draw_rectangle_lines(folha.x, folha.y, folha.w, folha.h, 2.0, BORDA);

    let recordes_ativos = aba == Folha::Recordes;
    botao_da_tela(menu.aba_recordes, "Recordes", 1.05 * t, if recordes_ativos { ABA_ATIVA } else { ABA_INATIVA }, true);
    botao_da_tela(menu.aba_creditos, "Créditos", 1.05 * t, if recordes_ativos { ABA_INATIVA } else { ABA_ATIVA }, true);
    botao_da_tela(menu.fechar, "", t, ABA_INATIVA, true);
    desenhar_xis(encolher(menu.fechar, 0.3 * menu.fechar.h), WHITE);

    let conteudo = menu.conteudo;
    if recordes_ativos {
        // Os dois modos de uma vez (como a lista "Ver recordes" do JS): Zen e depois Clássico.
        let blocos = blocos_de_recordes(app.recordes(prisma_engine::Mode::Zen), app.recordes(prisma_engine::Mode::Classic));
        let linhas_por_bloco: Vec<usize> = blocos.iter().map(|bloco| bloco.linhas.len()).collect();
        let lugares = layout::repartir_recordes(conteudo, t, &linhas_por_bloco);
        for (bloco, lugar) in blocos.iter().zip(&lugares) {
            escrever(bloco.titulo, lugar.legenda, 0.95 * t, TEXTO_SUAVE, Alinhamento::Esquerda);
            for (linha, area) in bloco.linhas.iter().zip(&lugar.cartoes) {
                desenhar_cartao(*area);
                let margem = 0.5 * t;
                let dentro = Rect { x: area.x + margem, w: area.w - 2.0 * margem, ..*area };
                escrever(linha.rotulo, Rect { w: 0.62 * dentro.w, ..dentro }, 0.95 * t, WHITE, Alinhamento::Esquerda);
                escrever(&linha.valor, Rect { x: dentro.x + 0.64 * dentro.w, w: 0.36 * dentro.w, ..dentro }, 1.1 * t, DESTAQUE, Alinhamento::Direita);
            }
        }
    } else {
        escrever_paragrafos(&creditos(env!("CARGO_PKG_VERSION")), conteudo, 0.95 * t, TEXTO_SUAVE);
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
    desenhar_fundo(&sprites.formas);
    let painel = app.painel();
    desenhar_cabecalho(app, &painel, tela, &sprites.formas);

    desenhar_tabuleiro_vazio(&sprites.formas, tela);
    let tabuleiro = tela.grade;
    desenhar_fundos_de_realce(app, &sprites.formas, tabuleiro);
    desenhar_pedras(app, sprites, tabuleiro);
    if let Some((passo, t)) = app.passo_atual() {
        desenhar_particulas(passo, t, tabuleiro);
    }
    desenhar_contornos_de_realce(app, tabuleiro);
    if let Some(aviso) = &painel.aviso {
        desenhar_aviso(aviso, tela);
    }
    if let Some((crate::linha::Passo::Nivel { nivel }, t)) = app.passo_atual() {
        desenhar_faixa_de_nivel(*nivel, t, tela, app.intensidade());
    }

    // Os botões de baixo. Com a partida acabada, a Dica fica apagada.
    let t = tela.texto;
    if let (true, Some(embaralhar)) = (painel.embaralhar_visivel, tela.botao_embaralhar) {
        desenhar_botao_de_acao(&sprites.formas, embaralhar, "Embaralhar", Icone::Embaralhar, t, true);
    }
    desenhar_botao_de_acao(&sprites.formas, tela.botao_dica, "Dica", Icone::Dica, t, painel.dica_ativa);
    desenhar_botao_de_acao(&sprites.formas, tela.botao_novo_jogo, "Novo jogo", Icone::NovoJogo, t, true);

    desenhar_guia_de_respiracao(app, tela);

    if let Some((nivel, pontos)) = &painel.fim {
        desenhar_fim(nivel, pontos, tela);
    }
    match app.folha() {
        Some(Folha::Som) => desenhar_folha_de_som(app, tela),
        Some(aba) => desenhar_folha_do_menu(app, aba, tela),
        None => {}
    }
    if painel.dialogo {
        desenhar_dialogo(tela);
    }
}
