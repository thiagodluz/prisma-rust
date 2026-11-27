//! Desenho: pega o estado do motor e pinta na tela com as imagens das pedras.
//!
//! Este módulo precisa de uma janela aberta (o Macroquad guarda as imagens na placa de
//! vídeo), por isso não tem testes automáticos: quem confere é a "foto" (`--screenshot`).

use std::collections::HashMap;

use macroquad::prelude::*;
use prisma_engine::{position, Board, Tile, SIZE};

use crate::app::{App, Foco, Folha, Painel};
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
/// As três cores de texto do jogo, da mais forte à mais discreta; `PARES_DE_TEXTO` prova que
/// cada uma se lê (4,5 para 1) sobre o fundo onde é usada.
/// O degradê do botão principal (Jogar novamente, Novo jogo, aba ativa), nas duas pontas.
const HEX_PRINCIPAL_A: u32 = 0x4d5df0;
const HEX_PRINCIPAL_B: u32 = 0x7c4fe6;
const HEX_PRINCIPAL_M: u32 = 0x6456eb;
const HEX_TEXTO_PRINCIPAL: u32 = 0xf4f2ff;
const HEX_TEXTO_SECUNDARIO: u32 = 0xc9c6e6;
const HEX_TEXTO_ROTULO: u32 = 0xaaa6cf;
const TEXTO_PRINCIPAL: Color = Color::from_hex(HEX_TEXTO_PRINCIPAL);
const TEXTO_SECUNDARIO: Color = Color::from_hex(HEX_TEXTO_SECUNDARIO);
const TEXTO_ROTULO: Color = Color::from_hex(HEX_TEXTO_ROTULO);
/// Os fundos mais claros que o vidro pode ter atrás do texto: o fundo do painel (`0x2a2250`, mais
/// claro que o real) e o do botão ativo (o lilás de `botao_icone_ativo` já misturado a ele).
#[cfg(test)]
const FUNDO_DO_PAINEL: u32 = 0x2a2250;
#[cfg(test)]
const FUNDO_DO_ATIVO: u32 = 0x463c76;
/// (nome, cor do texto, fundo): o que o teste de contraste confere.
#[cfg(test)]
const PARES_DE_TEXTO: [(&str, u32, u32); 8] = [
    ("texto principal no painel", HEX_TEXTO_PRINCIPAL, FUNDO_DO_PAINEL),
    ("texto secundário no painel", HEX_TEXTO_SECUNDARIO, FUNDO_DO_PAINEL),
    ("rótulo no painel", HEX_TEXTO_ROTULO, FUNDO_DO_PAINEL),
    ("texto principal no botão ativo", HEX_TEXTO_PRINCIPAL, FUNDO_DO_ATIVO),
    ("destaque dourado no painel", 0xffd166, FUNDO_DO_PAINEL),
    ("aviso de não salvar no painel", 0xffc069, FUNDO_DO_PAINEL),
    ("rótulo do botão principal (começo do degradê)", 0xffffff, HEX_PRINCIPAL_A),
    ("rótulo do botão principal (fim do degradê)", 0xffffff, HEX_PRINCIPAL_B),
];
const BARRA_PREENCHIDA: Color = Color::from_hex(0xa28fff);
const DESTAQUE: Color = Color::from_hex(0xffd166);
/// O traço das barras de volume (a parte vazia) e o interruptor desligado: branco translúcido,
/// que se vê sobre o vidro das linhas.
const TRILHO: Color = Color::new(1.0, 1.0, 1.0, 0.16);
const INTERRUPTOR_APAGADO: Color = Color::new(1.0, 1.0, 1.0, 0.22);

/// Todas as imagens já carregadas na placa de vídeo, uma `Texture2D` por `Sprite`.
pub struct Sprites {
    texturas: HashMap<Sprite, Texture2D>,
    formas: Formas,
}

/// O vidro grande: raio dos cantos e sombra, em pixels da imagem (na tela, a sombra é sempre
/// `SOMBRA_DO_PAINEL / RAIO_DO_PAINEL` do raio com que o painel é desenhado).
const RAIO_DO_PAINEL: u32 = 20;
const SOMBRA_DO_PAINEL: u32 = 14;

/// Desenha o vidro grande em `area`, com cantos de raio `raio` (em pixels da tela).
fn desenhar_painel(formas: &Formas, area: Rect, raio: f32) {
    desenhar_forma(&formas.painel, area, raio, raio * SOMBRA_DO_PAINEL as f32 / RAIO_DO_PAINEL as f32);
}

/// O raio dos cantos da moldura do tabuleiro: o do vidro dela e o da película do fim de partida.
fn raio_da_moldura(tabuleiro: Rect) -> f32 {
    0.059 * tabuleiro.w
}

/// Onde, com que cantos e com que opacidade se escurece o tabuleiro no fim da partida: a moldura
/// toda, arredondada como ela.
fn pelicula_do_fim(tabuleiro: Rect) -> (Rect, f32, f32) {
    (tabuleiro, raio_da_moldura(tabuleiro), 0.55)
}

/// Escurece `area` com cantos arredondados de raio `raio`: preto com opacidade `alfa`.
fn pelicula(formas: &Formas, area: Rect, raio: f32, alfa: f32) {
    desenhar_forma_tingida(&formas.solida, area, raio, 0.0, Color::new(0.0, 0.0, 0.0, alfa));
}

/// O raio dos cantos (em pixels da imagem) e a sombra das formas de vidro: na tela, a sombra é
/// sempre `SOMBRA_DO_VIDRO / RAIO_DO_VIDRO` do raio com que a forma é desenhada.
const RAIO_DO_VIDRO: u32 = 12;
const SOMBRA_DO_VIDRO: u32 = 6;

/// A receita do vidro fosco: borda de 1 px, um degradê de três paradas com brilho no topo (o
/// preenchimento mais claro e mais cheio), o preenchimento no meio e uma base mais escura, e uma
/// sombra suave em volta.
fn receita_de_vidro(raio: u32, preenchimento: formas::Cor, borda: formas::Cor) -> Receita {
    let [r, g, b, a] = preenchimento;
    let rumo = |canal: u8, alvo: u8, parte: f32| (canal as f32 + (alvo as f32 - canal as f32) * parte).round() as u8;
    let topo = [rumo(r, 255, 0.35), rumo(g, 255, 0.35), rumo(b, 255, 0.35), (a as f32 * 1.5).round().min(255.0) as u8];
    let base = [rumo(r, 0, 0.30), rumo(g, 0, 0.30), rumo(b, 0, 0.30), a];
    Receita {
        lado: 64,
        raio,
        borda: 1,
        cor_da_borda: borda,
        degrade: [topo, preenchimento, base],
        angulo: 180.0,
        sombra: SOMBRA_DO_VIDRO,
        cor_da_sombra: rgba(0x03061633),
    }
}

/// Desenha uma forma de vidro: com a sombra na proporção da receita.
fn desenhar_vidro(forma: &Forma, area: Rect, raio: f32) {
    desenhar_forma(forma, area, raio, raio * SOMBRA_DO_VIDRO as f32 / RAIO_DO_VIDRO as f32);
}

/// O tamanho de letra de um número do placar: o `tamanho` pedido, ou menos se `largura` (a
/// largura que o texto tem a cada tamanho) não couber na coluna de `coluna_w`.
fn tamanho_do_valor_com(largura: impl Fn(f32) -> f32, coluna_w: f32, tamanho: f32) -> f32 {
    texto::tamanho_que_cabe(largura(tamanho), coluna_w, tamanho)
}

/// Como `tamanho_do_valor_com`, medindo com a fonte do título.
fn tamanho_do_valor(valor: &str, coluna_w: f32, tamanho: f32) -> f32 {
    tamanho_do_valor_com(|t| texto::medir(valor, t, Estilo::Titulo).width, coluna_w, tamanho)
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
    /// O trilho da barra de progresso (a parte vazia): vidro com borda fina.
    trilho: Forma,
    /// O vidro grande e quase opaco do cartão de fim, do diálogo e das folhas de Som e Menu.
    painel: Forma,
    /// O botão que pede a ação principal (Jogar novamente, Novo jogo, aba aberta): degradê azul-lilás.
    botao_principal: Forma,
    /// O anel luminoso das casas marcadas: sem preenchimento, borda branca e um brilho lilás por fora.
    anel: Forma,
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
            cartao: Forma::nova(receita_de_vidro(RAIO_DO_VIDRO, rgba(0xffffff0d), rgba(0xffffff17))),
            painel: Forma::nova(Receita {
                lado: 96,
                raio: RAIO_DO_PAINEL,
                borda: 1,
                cor_da_borda: rgba(0xffffff2e),
                degrade: [rgba(0x2b2560fa), rgba(0x1a1844f8), rgba(0x121033f8)],
                angulo: 180.0,
                sombra: SOMBRA_DO_PAINEL,
                cor_da_sombra: rgba(0x03061666),
            }),
            botao_principal: Forma::nova(Receita {
                lado: 64,
                raio: 14,
                borda: 1,
                cor_da_borda: rgba(0xffffff59),
                degrade: [rgba((HEX_PRINCIPAL_A << 8) | 0xff), rgba((HEX_PRINCIPAL_M << 8) | 0xff), rgba((HEX_PRINCIPAL_B << 8) | 0xff)],
                angulo: 135.0,
                sombra: 0,
                cor_da_sombra: sem_borda,
            }),
            anel: Forma::nova(Receita {
                lado: 64,
                raio: RAIO_DO_ANEL,
                borda: 3,
                cor_da_borda: rgba(0xffffffff),
                degrade: [sem_borda; 3],
                angulo: 180.0,
                sombra: BRILHO_DO_ANEL,
                cor_da_sombra: rgba(0xc4b2ff55),
            }),
            trilho: Forma::nova(Receita {
                lado: 32,
                raio: 4,
                borda: 1,
                cor_da_borda: rgba(0xffffff26),
                degrade: [rgba(0xffffff14); 3],
                angulo: 180.0,
                sombra: 0,
                cor_da_sombra: sem_borda,
            }),
            botao_icone: Forma::nova(receita_de_vidro(RAIO_DO_VIDRO, rgba(0xffffff10), rgba(0xffffff27))),
            botao_icone_ativo: Forma::nova(receita_de_vidro(RAIO_DO_VIDRO, rgba(0xb8a8ff48), rgba(0xc4b2ffff))),
            marca,
            modo: Forma::nova(receita_de_vidro(RAIO_DO_VIDRO, rgba(0xffffff0b), rgba(0xffffff23))),
            modo_escolhido: Forma::nova(receita_de_vidro(RAIO_DO_VIDRO, rgba(0x8678c040), rgba(0xc4b2ffff))),
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
    desenhar_forma(&formas.moldura, moldura, raio_da_moldura(moldura), 0.05 * moldura.w);
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

/// Texto em letras espaçadas (`espaco` em "em", fração do tamanho), do jeito dos rótulos em
/// maiúsculas do placar. Encolhe, se preciso, para caber na largura da área.
fn escrever_espacado(texto: &str, area: Rect, tamanho: f32, cor: Color, alinhamento: Alinhamento, espaco: f32) {
    if tamanho < 1.0 || area.w <= 0.0 {
        return;
    }
    let letras: Vec<String> = texto.chars().map(|letra| letra.to_string()).collect();
    let largura = |tamanho: f32| -> f32 {
        let soma: f32 = letras.iter().map(|letra| texto::medir(letra, tamanho, Estilo::TextoForte).width).sum();
        soma + espaco * tamanho * letras.len().saturating_sub(1) as f32
    };
    let tamanho = texto::tamanho_que_cabe(largura(tamanho), area.w, tamanho);
    if tamanho < 1.0 {
        return;
    }
    let total = largura(tamanho);
    let mut x = match alinhamento {
        Alinhamento::Esquerda => area.x,
        Alinhamento::Centro => area.x + (area.w - total) / 2.0,
        Alinhamento::Direita => area.x + area.w - total,
    };
    let referencia = texto::medir("Ag", tamanho, Estilo::TextoForte);
    let y = area.y + (area.h - referencia.height) / 2.0 + referencia.offset_y;
    for letra in &letras {
        texto::desenhar(letra, x, y, tamanho, cor, Estilo::TextoForte);
        x += texto::medir(letra, tamanho, Estilo::TextoForte).width + espaco * tamanho;
    }
}

/// Um botão da tela de jogo: vidro com o rótulo no meio. `principal` usa o degradê azul-lilás
/// (a ação que a tela espera); `ativo` falso apaga o botão.
fn botao_da_tela(formas: &Formas, area: Rect, rotulo: &str, tamanho: f32, principal: bool, ativo: bool) {
    let alfa = if ativo { 1.0 } else { 0.4 };
    let forma = if principal { &formas.botao_principal } else { &formas.botao };
    desenhar_forma_tingida(forma, area, 0.21 * area.h, 0.0, com_alfa(WHITE, alfa));
    let margem = 0.06 * area.w;
    let dentro = Rect { x: area.x + margem, y: area.y, w: area.w - 2.0 * margem, h: area.h };
    escrever_com_estilo(rotulo, dentro, tamanho, com_alfa(TEXTO_PRINCIPAL, alfa), Alinhamento::Centro, Estilo::TextoForte);
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

/// O anel das casas marcadas: o raio dos cantos e o brilho, em pixels da imagem do anel. Na tela
/// o brilho tem sempre `BRILHO_DO_ANEL / RAIO_DO_ANEL` do raio com que o anel é desenhado.
const RAIO_DO_ANEL: u32 = 12;
const BRILHO_DO_ANEL: u32 = 8;

/// A casa visível aumentada de 4% do lado em cada ponta: o anel fica folgado em volta da pedra
/// e o brilho tem para onde sair sem sujar a vizinha.
fn anel_da_casa(area: Rect, indice: usize) -> Rect {
    let casa = casa_visivel(area, indice);
    expandir(casa, 0.04 * casa.w)
}

/// Desenha o anel luminoso em volta da casa `indice`, multiplicando as cores do anel por `tinta`
/// (branco puro deixa o anel como é; as outras cores o tingem, e o alfa da tinta o apaga).
fn desenhar_anel(formas: &Formas, area: Rect, indice: usize, tinta: Color) {
    let anel = anel_da_casa(area, indice);
    let raio = 0.22 * anel.w;
    desenhar_forma_tingida(&formas.anel, anel, raio, raio * BRILHO_DO_ANEL as f32 / RAIO_DO_ANEL as f32, tinta);
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
fn desenhar_contornos_de_realce(app: &App, formas: &Formas, area: Rect) {
    let gestos = app.gestos();
    if let Some(indice) = gestos.selecionada() {
        desenhar_anel(formas, area, indice, WHITE);
    }
    if let Some(indice) = gestos.alvo_do_arrasto() {
        desenhar_anel(formas, area, indice, com_alfa(cor_de(rgba(0xc4ffe4ff)), 0.8));
    }
    if let Some((a, b)) = app.dica() {
        let pulso = pulso_da_dica();
        for indice in [a, b] {
            desenhar_anel(formas, area, indice, com_alfa(DESTAQUE, pulso));
        }
    }
}

/// As abas, o painel de números e a barra de progresso.
fn desenhar_cabecalho(app: &App, painel: &Painel, tela: &TelaJogo, formas: &Formas) {
    let t = tela.texto;
    desenhar_topo(formas, app, tela);
    let zen = app.modo() == prisma_engine::Mode::Zen;
    desenhar_cartao_de_modo(formas, tela.aba_zen, Icone::Lua, "Zen", FRASE_ZEN, zen);
    desenhar_cartao_de_modo(formas, tela.aba_classico, Icone::Faisca, "Clássico", FRASE_CLASSICO, !zen);

    // O cartão de vidro com as três colunas: pontos à esquerda, nível no meio, melhor à direita,
    // separadas por duas divisórias finas.
    let cartao = tela.cartao_pontos;
    desenhar_vidro(&formas.cartao, cartao, 0.045 * cartao.w);
    for (esquerda, direita) in [(tela.pontos, tela.nivel), (tela.nivel, tela.melhor)] {
        let x = ((esquerda.x + esquerda.w + direita.x) / 2.0).round();
        draw_rectangle(x, cartao.y + 0.22 * cartao.h, 1.0, 0.56 * cartao.h, cor_de(rgba(0xffffff1f)));
    }
    for (rotulo, valor, area, cor_do_valor, alinhamento) in [
        ("PONTOS", &painel.pontos, tela.pontos, TEXTO_PRINCIPAL, Alinhamento::Esquerda),
        ("NÍVEL", &painel.nivel, tela.nivel, cor_de(rgba(0xa2edddff)), Alinhamento::Centro),
        ("MELHOR", &painel.melhor, tela.melhor, TEXTO_PRINCIPAL, Alinhamento::Direita),
    ] {
        // 6% de respiro em cima; o rótulo ocupa o resto do terço de cima, o valor os dois de baixo.
        let em_cima = Rect { y: area.y + 0.06 * area.h, h: 0.32 * area.h, ..area };
        let embaixo = Rect { y: area.y + 0.38 * area.h, h: 0.62 * area.h, ..area };
        escrever_espacado(rotulo, em_cima, 0.62 * t, TEXTO_ROTULO, alinhamento, 0.12);
        let tamanho = tamanho_do_valor(valor, embaixo.w, 1.5 * t);
        escrever_com_estilo(valor, embaixo, tamanho, cor_do_valor, alinhamento, Estilo::Titulo);
    }

    // A barra de nível: trilho de vidro com borda fina e, por cima, o preenchimento em degradê
    // da esquerda (verde-água) para a direita (lilás), com a ponta direita reta.
    let barra = tela.barra;
    desenhar_forma(&formas.trilho, barra, barra.h / 2.0, 0.0);
    let progresso = if painel.progresso.is_finite() { painel.progresso.clamp(0.0, 1.0) } else { 0.0 };
    let (esquerda, direita) = (cor_de(rgba(0x6be2d6ff)), cor_de(rgba(0xd09cffff)));
    desenhar_degrade(Rect { w: barra.w * progresso, ..barra }, [esquerda, direita, direita, esquerda]);
    escrever(&painel.rotulo, tela.rotulo, 0.86 * t, TEXTO_SECUNDARIO, Alinhamento::Direita);
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
        desenhar_vidro(forma, area, 0.316 * area.h);
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
fn desenhar_aviso(formas: &Formas, texto: &str, tela: &TelaJogo) {
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
    desenhar_painel(formas, area, 0.45 * area.h);
    escrever_com_estilo(texto, Rect { x: area.x + margem, w: area.w - 2.0 * margem, ..area }, tamanho, TEXTO_PRINCIPAL, Alinhamento::Centro, Estilo::TextoForte);
}

/// A faixa "Nível N" no meio do tabuleiro, crescendo e sumindo conforme `t` (ms).
fn desenhar_faixa_de_nivel(formas: &Formas, nivel: u32, t: f32, tela: &TelaJogo, intensidade: Intensidade) {
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
    let raio = 0.3 * area.h;
    let passo = 0.012 * tabuleiro.w;
    for camada in 1..=4 {
        let folga = camada as f32 * passo;
        let cor = Color::new(0.71, 0.64, 1.0, 0.11 * brilho * alfa);
        desenhar_forma_tingida(&formas.solida, expandir(area, folga), raio + folga, 0.0, cor);
    }
    let desce = 0.025 * tabuleiro.h * sombra;
    desenhar_forma_tingida(&formas.solida, Rect { y: area.y + desce, ..area }, raio, 0.0, Color::new(0.1, 0.09, 0.25, 0.45 * sombra * alfa));
    desenhar_forma_tingida(&formas.painel, area, raio, raio * SOMBRA_DO_PAINEL as f32 / RAIO_DO_PAINEL as f32, com_alfa(WHITE, alfa));
    escrever_com_estilo(&format!("Nível {nivel}"), area, 2.4 * tela.texto * escala, com_alfa(DESTAQUE, alfa), Alinhamento::Centro, Estilo::Titulo);
}

/// O cartão de "Fim de partida", com o tabuleiro escurecido atrás (nos cantos da moldura).
fn desenhar_fim(formas: &Formas, nivel: &str, pontos: &str, tela: &TelaJogo) {
    let t = tela.texto;
    let (area, raio, alfa) = pelicula_do_fim(tela.tabuleiro);
    pelicula(formas, area, raio, alfa);
    let cartao = tela.cartao_fim;
    desenhar_painel(formas, cartao, 0.08 * cartao.w.min(cartao.h * 1.6));
    let linha = |de: f32, ate: f32| Rect {
        x: cartao.x + 0.05 * cartao.w,
        y: cartao.y + de * cartao.h,
        w: 0.9 * cartao.w,
        h: (ate - de) * cartao.h,
    };
    escrever_com_estilo("Fim de partida", linha(0.05, 0.26), 1.5 * t, TEXTO_PRINCIPAL, Alinhamento::Centro, Estilo::Titulo);
    escrever("Não há mais jogadas possíveis nesta partida.", linha(0.27, 0.41), 0.72 * t, TEXTO_SECUNDARIO, Alinhamento::Centro);
    escrever(&format!("Nível {nivel}   |   {pontos} pontos"), linha(0.43, 0.62), 1.1 * t, DESTAQUE, Alinhamento::Centro);
    botao_da_tela(formas, tela.botao_jogar_novamente, "Jogar novamente", t, true, true);
}

/// A pergunta "Começar novo jogo?", com a tela inteira escurecida atrás.
fn desenhar_dialogo(formas: &Formas, tela: &TelaJogo) {
    let t = tela.texto;
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.6));
    let dialogo = tela.dialogo;
    desenhar_painel(formas, dialogo, 0.08 * dialogo.w.min(dialogo.h * 1.6));
    let linha = |de: f32, ate: f32| Rect {
        x: dialogo.x + 0.05 * dialogo.w,
        y: dialogo.y + de * dialogo.h,
        w: 0.9 * dialogo.w,
        h: (ate - de) * dialogo.h,
    };
    escrever_com_estilo("Começar novo jogo?", linha(0.08, 0.32), 1.35 * t, TEXTO_PRINCIPAL, Alinhamento::Centro, Estilo::Titulo);
    escrever("O progresso da partida atual será substituído.", linha(0.34, 0.50), 0.72 * t, TEXTO_SECUNDARIO, Alinhamento::Centro);
    botao_da_tela(formas, tela.dialogo_cancelar, "Cancelar", t, false, true);
    botao_da_tela(formas, tela.dialogo_confirmar, "Novo jogo", t, true, true);
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

/// A frase embaixo do nome de cada modo, nos cartões. A do Clássico é a mais comprida: o teste
/// confere que ela cabe nos cartões de todos os tamanhos de tela.
const FRASE_ZEN: &str = "Sem derrota";
const FRASE_CLASSICO: &str = "Pode acabar";
/// O rótulo dos botões de ação: tamanho (em `tela.texto`) e margem lateral (em largura do botão).
const ROTULO_DO_BOTAO: f32 = 0.9;
const MARGEM_DO_BOTAO: f32 = 0.1;

/// Um rótulo cabe na largura se, ao `tamanho` pedido, a largura que ele tem (`largura_do_texto`,
/// dada a cada tamanho) não passa de `largura`.
#[cfg(test)]
fn rotulo_cabe_com(largura_do_texto: impl Fn(f32) -> f32, largura: f32, tamanho: f32) -> bool {
    largura > 0.0 && largura_do_texto(tamanho) <= largura + 0.5
}

/// Onde ficam, num cartão de modo, a caixa do ícone, o x onde começa o texto e a largura útil dele.
fn colunas_do_cartao_de_modo(area: Rect) -> (Rect, f32, f32) {
    let folga = 0.2 * area.h;
    let lado = 0.56 * area.h;
    let caixa = Rect { x: area.x + folga, y: area.y + (area.h - lado) / 2.0, w: lado, h: lado };
    let texto_x = caixa.x + caixa.w + 0.7 * folga;
    let largura = area.x + area.w - folga * 0.6 - texto_x;
    (caixa, texto_x, largura)
}

/// Um botão de baixo: fundo arredondado, o ícone no alto e o rótulo embaixo. `ativo` falso deixa
/// tudo a 40% de opacidade.
fn desenhar_botao_de_acao(formas: &Formas, area: Rect, rotulo: &str, icone: Icone, tamanho: f32, ativo: bool) {
    let alfa = if ativo { 1.0 } else { 0.4 };
    desenhar_forma_tingida(&formas.botao, area, 0.21 * area.h, 0.0, com_alfa(WHITE, alfa));
    let cor = com_alfa(cor_de(rgba(0xeeeaffff)), alfa);
    let lado = 0.36 * area.h;
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
    let margem = MARGEM_DO_BOTAO * area.w;
    let embaixo = Rect { x: area.x + margem, y: area.y + 0.54 * area.h, w: area.w - 2.0 * margem, h: 0.38 * area.h };
    escrever_com_estilo(rotulo, embaixo, ROTULO_DO_BOTAO * tamanho, cor, Alinhamento::Centro, Estilo::TextoForte);
}

/// Um cartão de modo: a caixa do ícone à esquerda e, ao lado, o nome (negrito) e a frase (menor).
/// `escolhido` troca o fundo e a borda pelos do modo em uso.
fn desenhar_cartao_de_modo(formas: &Formas, area: Rect, icone: Icone, nome: &str, frase: &str, escolhido: bool) {
    let forma = if escolhido { &formas.modo_escolhido } else { &formas.modo };
    desenhar_vidro(forma, area, 0.28 * area.h);
    let (caixa, texto_x, largura) = colunas_do_cartao_de_modo(area);
    let lado = caixa.w;
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
    let nome_area = Rect { x: texto_x, y: area.y + 0.14 * area.h, w: largura, h: 0.38 * area.h };
    let frase_area = Rect { x: texto_x, y: area.y + 0.52 * area.h, w: largura, h: 0.32 * area.h };
    escrever_com_estilo(nome, nome_area, 0.30 * area.h, TEXTO_PRINCIPAL, Alinhamento::Esquerda, Estilo::TextoForte);
    escrever(frase, frase_area, 0.24 * area.h, TEXTO_SECUNDARIO, Alinhamento::Esquerda);
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

/// Uma linha da folha: vidro fosco.
fn desenhar_cartao(formas: &Formas, area: Rect) {
    desenhar_vidro(&formas.cartao, area, 0.22 * area.h);
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
fn desenhar_linha_com_interruptor(formas: &Formas, area: Rect, rotulo: &str, ligado: bool, t: f32) {
    desenhar_cartao(formas, area);
    let texto = Rect { x: area.x + 0.18 * area.h, w: area.w - 0.18 * area.h - 0.46 * 1.8 * area.h - 0.5 * area.h, ..area };
    escrever(rotulo, texto, 0.95 * t, WHITE, Alinhamento::Esquerda);
    desenhar_interruptor(area, ligado);
}

/// O botão que passa para a próxima opção: o nome atual e um triângulo "para a frente".
fn desenhar_botao_de_escolha(formas: &Formas, area: Rect, nome: &str, t: f32) {
    botao_da_tela(formas, area, "", t, false, true);
    let texto = Rect { x: area.x + 0.18 * area.h, w: area.w - 0.18 * area.h - 0.72 * area.h, ..area };
    escrever(nome, texto, 0.95 * t, WHITE, Alinhamento::Esquerda);
    let (cx, cy, r) = (area.x + area.w - 0.36 * area.h, area.y + area.h / 2.0, 0.13 * area.h);
    draw_triangle(vec2(cx - 0.7 * r, cy - r), vec2(cx - 0.7 * r, cy + r), vec2(cx + r, cy), TEXTO_SUAVE);
}

/// A linha de volume. Ela toda é a barra: o começo é 0 e o fim é 100 (é assim que o dedo é
/// lido). Em cima o nome e o número; embaixo o traço, que vai de ponta a ponta, com a bolinha
/// na posição do volume.
fn desenhar_linha_de_volume(formas: &Formas, area: Rect, rotulo: &str, volume: u8, t: f32) {
    desenhar_cartao(formas, area);
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
fn desenhar_folha_de_som(formas: &Formas, app: &App, tela: &TelaJogo) {
    let som = &tela.som;
    if som.folha.w <= 0.0 || som.folha.h <= 0.0 {
        return;
    }
    let t = som.texto;
    let preferencias = app.preferencias();
    let folha = som.folha;
    desenhar_painel(formas, folha, 0.04 * folha.w);

    // No lugar do título aparece o aviso (o do Testar ou o de áudio), se houver.
    match app.aviso_da_folha() {
        Some(aviso) => escrever_em_linhas(aviso, som.titulo, 0.9 * t, DESTAQUE),
        None => escrever_com_estilo("Som", som.titulo, 1.5 * t, TEXTO_PRINCIPAL, Alinhamento::Esquerda, Estilo::Titulo),
    }
    botao_da_tela(formas, som.fechar, "", t, false, true);
    desenhar_xis(encolher(som.fechar, 0.3 * som.fechar.h), WHITE);

    desenhar_linha_com_interruptor(formas, som.sons_do_jogo, "Sons do jogo", preferencias.sons_do_jogo, t);
    botao_da_tela(formas, som.testar, "Testar som", t, true, true);

    desenhar_linha_de_volume(formas, som.barra_efeitos, "Volume dos efeitos", preferencias.volumes.efeitos, t);

    if let Some(area) = som.vibracao {
        desenhar_linha_com_interruptor(formas, area, "Vibração", preferencias.vibracao, t);
    }

    if let (Some(liga), Some(escolhe)) = (som.liga_musica, som.escolhe_musica) {
        desenhar_linha_com_interruptor(formas, liga, "Música", preferencias.musica_ligada, t);
        desenhar_botao_de_escolha(formas, escolhe, preferencias.musica.texto(), t);
    }
    desenhar_linha_de_volume(formas, som.barra_musica, "Volume da música", preferencias.volumes.musica, t);

    if let (Some(liga), Some(escolhe)) = (som.liga_ambiente, som.escolhe_ambiente) {
        desenhar_linha_com_interruptor(formas, liga, "Ambiente", preferencias.ambiente_ligado, t);
        desenhar_botao_de_escolha(formas, escolhe, preferencias.ambiente.texto(), t);
    }
    desenhar_linha_de_volume(formas, som.barra_ambiente, "Volume do ambiente", preferencias.volumes.ambiente, t);

    if let Some(area) = som.respiracao {
        desenhar_cartao(formas, area);
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
        desenhar_cartao(formas, area);
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
fn desenhar_folha_do_menu(formas: &Formas, app: &App, aba: Folha, tela: &TelaJogo) {
    let menu = &tela.menu;
    if menu.folha.w <= 0.0 || menu.folha.h <= 0.0 {
        return;
    }
    let t = menu.texto;
    let folha = menu.folha;
    desenhar_painel(formas, folha, 0.04 * folha.w);

    let recordes_ativos = aba == Folha::Recordes;
    if app.ferramentas_liberadas() {
        let abas = [(Folha::Recordes, "Recordes"), (Folha::Creditos, "Créditos"), (Folha::Ferramentas, "Ferramentas")];
        for ((dona, rotulo), area) in abas.into_iter().zip(menu.abas_com_ferramentas) {
            botao_da_tela(formas, area, rotulo, 1.05 * t, aba == dona, true);
        }
    } else {
        botao_da_tela(formas, menu.aba_recordes, "Recordes", 1.05 * t, recordes_ativos, true);
        botao_da_tela(formas, menu.aba_creditos, "Créditos", 1.05 * t, !recordes_ativos, true);
    }
    botao_da_tela(formas, menu.fechar, "", t, false, true);
    desenhar_xis(encolher(menu.fechar, 0.3 * menu.fechar.h), WHITE);

    let conteudo = menu.conteudo;
    if recordes_ativos {
        // Os dois modos de uma vez (como a lista "Ver recordes" do JS): Zen e depois Clássico.
        let blocos = blocos_de_recordes(app.recordes(prisma_engine::Mode::Zen), app.recordes(prisma_engine::Mode::Classic));
        let linhas_por_bloco: Vec<usize> = blocos.iter().map(|bloco| bloco.linhas.len()).collect();
        let lugares = layout::repartir_recordes(conteudo, t, &linhas_por_bloco);
        for (bloco, lugar) in blocos.iter().zip(&lugares) {
            escrever(bloco.titulo, lugar.legenda, 0.95 * t, TEXTO_SECUNDARIO, Alinhamento::Esquerda);
            for (linha, area) in bloco.linhas.iter().zip(&lugar.cartoes) {
                desenhar_cartao(formas, *area);
                let margem = 0.5 * t;
                let dentro = Rect { x: area.x + margem, w: area.w - 2.0 * margem, ..*area };
                escrever(linha.rotulo, Rect { w: 0.62 * dentro.w, ..dentro }, 0.95 * t, WHITE, Alinhamento::Esquerda);
                escrever(&linha.valor, Rect { x: dentro.x + 0.64 * dentro.w, w: 0.36 * dentro.w, ..dentro }, 1.1 * t, DESTAQUE, Alinhamento::Direita);
            }
        }
    } else if aba == Folha::Ferramentas {
        desenhar_ferramentas(formas, app, menu);
    } else {
        // A primeira linha (a versão) fica sozinha em `versao`, onde os 7 toques contam.
        let linhas = creditos(env!("CARGO_PKG_VERSION"));
        if let Some((versao, resto)) = linhas.split_first() {
            escrever(versao, menu.versao, 0.95 * t, TEXTO_SECUNDARIO, Alinhamento::Esquerda);
            let abaixo = menu.versao.y + menu.versao.h;
            let resto_area = Rect { y: abaixo, h: (conteudo.y + conteudo.h - abaixo).max(0.0), ..conteudo };
            escrever_paragrafos(resto, resto_area, 0.95 * t, TEXTO_SECUNDARIO);
        }
    }
}

/// A aba Ferramentas (o menu secreto): estado da partida, Desfazer, pontos e nível com – e +,
/// atalhos, especiais, poderes e o interruptor "Impedir derrota". Com animação, tudo apagado.
fn desenhar_ferramentas(formas: &Formas, app: &App, menu: &layout::TelaMenu) {
    let t = menu.texto;
    let f = &menu.ferramentas;
    let estado = app.ferramentas();
    let ativo = !estado.ocupado;
    let situacao = if estado.com_cheats { "Com cheats, sem recordes" } else { "Partida normal" };
    let linha = format!("{situacao} · {} de 50 para desfazer", estado.acoes);
    escrever(&linha, f.estado, 0.9 * t, TEXTO_SECUNDARIO, Alinhamento::Centro);
    botao_da_tela(formas, f.desfazer, "Desfazer última ação", t, true, ativo && estado.pode_desfazer);
    for (menos, valor, mais, acao, texto, rotulo) in [
        (f.pontos_menos, f.pontos_valor, f.pontos_mais, f.somar, estado.pontos.as_str(), "Somar pontos"),
        (f.nivel_menos, f.nivel_valor, f.nivel_mais, f.ir, estado.nivel.as_str(), "Ir ao nível"),
    ] {
        botao_da_tela(formas, menos, "–", 1.2 * t, false, ativo);
        desenhar_cartao(formas, valor);
        escrever_com_estilo(texto, valor, 1.1 * t, TEXTO_PRINCIPAL, Alinhamento::Centro, Estilo::Titulo);
        botao_da_tela(formas, mais, "+", 1.2 * t, false, ativo);
        botao_da_tela(formas, acao, rotulo, 0.9 * t, true, ativo);
    }
    let comuns = [
        (f.mais_100, "+100"),
        (f.mais_1000, "+1.000"),
        (f.proximo_nivel, "Próx. nível"),
        (f.especiais[0], "Pulso"),
        (f.especiais[1], "Raio"),
        (f.especiais[2], "Espectro"),
        (f.especiais[3], "Supernova"),
        (f.embaralhar, "Embaralhar"),
        (f.jogada_automatica, "Jogada auto"),
        (f.explodir, "Explodir"),
    ];
    for (area, rotulo) in comuns {
        botao_da_tela(formas, area, rotulo, 0.9 * t, false, ativo);
    }
    desenhar_linha_com_interruptor(formas, f.impedir_derrota, "Impedir derrota", estado.impedir_derrota, t);
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
    escrever(&segundos_restantes(estado.restante_s), embaixo, 0.8 * t, TEXTO_SECUNDARIO, Alinhamento::Esquerda);
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
    desenhar_contornos_de_realce(app, &sprites.formas, tabuleiro);
    if let Some(aviso) = &painel.aviso {
        desenhar_aviso(&sprites.formas, aviso, tela);
    }
    if let Some((crate::linha::Passo::Nivel { nivel }, t)) = app.passo_atual() {
        desenhar_faixa_de_nivel(&sprites.formas, *nivel, t, tela, app.intensidade());
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
        desenhar_fim(&sprites.formas, nivel, pontos, tela);
    }
    match app.folha() {
        Some(Folha::Som) => desenhar_folha_de_som(&sprites.formas, app, tela),
        Some(aba) => desenhar_folha_do_menu(&sprites.formas, app, aba, tela),
        None => {}
    }
    if painel.dialogo {
        desenhar_dialogo(&sprites.formas, tela);
    }
    desenhar_foco_do_teclado(app, &sprites.formas, tela);
}

/// O contorno do teclado (só depois de usar o teclado): numa casa, o anel verde-água; num botão
/// ou numa barra de volume, o anel em volta dele, 3 px para fora.
fn desenhar_foco_do_teclado(app: &App, formas: &Formas, tela: &TelaJogo) {
    let verde_agua = cor_de(rgba(0x8ff5e6ff));
    match app.retangulo_do_foco(tela) {
        Some((Foco::Casa(casa), _)) => desenhar_anel(formas, tela.grade, casa, verde_agua),
        Some((_, area)) => {
            let anel = expandir(area, 3.0);
            let raio = 0.3 * anel.w.min(anel.h);
            desenhar_forma_tingida(&formas.anel, anel, raio, raio * BRILHO_DO_ANEL as f32 / RAIO_DO_ANEL as f32, verde_agua);
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_texto_passa_de_4_5_para_1_de_contraste() {
        for (nome, texto, fundo) in PARES_DE_TEXTO {
            let (t, f) = (cor_de_hex(texto), cor_de_hex(fundo));
            let razao = crate::contraste::razao(t, f);
            assert!(razao >= 4.5, "{nome}: contraste {razao:.2} (texto {texto:06x} sobre {fundo:06x})");
        }
    }

    fn cor_de_hex(hex: u32) -> [u8; 3] {
        [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]
    }

    #[test]
    fn o_vidro_tem_brilho_no_topo_borda_fina_e_sombra_suave() {
        let receita = receita_de_vidro(12, rgba(0xffffff0d), rgba(0xffffff17));
        assert_eq!(receita.borda, 1);
        assert_eq!(receita.sombra, 6);
        assert_eq!(receita.cor_da_sombra[..3], [0x03, 0x06, 0x16]);
        let soma = |c: formas::Cor| (c[0] as u32 + c[1] as u32 + c[2] as u32) * c[3] as u32;
        let [topo, meio, base] = receita.degrade;
        assert!(soma(topo) > soma(meio) && soma(meio) > soma(base), "topo > meio > base: {topo:?} {meio:?} {base:?}");
        assert!(topo[3] >= meio[3], "o brilho do topo não some");
        assert!(receita.cor_da_sombra[3] > 0 && receita.cor_da_sombra[3] < 0x60, "sombra suave");
    }

    /// A largura de um texto em Sora Bold, medida sem janela (o Macroquad exige uma para medir).
    fn largura_em_sora(texto: &str, tamanho: f32) -> f32 {
        let fonte = fontdue::Font::from_bytes(texto::TITULO, fontdue::FontSettings::default()).expect("Sora carrega");
        texto.chars().map(|c| fonte.metrics(c, tamanho).advance_width).sum()
    }

    #[test]
    fn o_placar_encolhe_numero_enorme_para_caber_na_coluna() {
        let valor = "12.345.678";
        let tamanho = tamanho_do_valor_com(|t| largura_em_sora(valor, t), 100.0, 40.0);
        assert!(tamanho <= 40.0);
        assert!(largura_em_sora(valor, tamanho) <= 100.5, "largura {}", largura_em_sora(valor, tamanho));
        assert!(tamanho < 40.0, "esse número não cabe a 40 px: tem de encolher");
        // Número curto: a letra não muda.
        assert_eq!(tamanho_do_valor_com(|t| largura_em_sora("4.321", t), 100.0, 20.0), 20.0);
    }

    /// A largura de um texto em Inter (Texto ou TextoForte), medida sem janela.
    fn largura_em_inter(estilo: Estilo, texto: &str, tamanho: f32) -> f32 {
        let bytes = match estilo {
            Estilo::TextoForte => texto::TEXTO_FORTE,
            _ => texto::TEXTO,
        };
        let fonte = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).expect("Inter carrega");
        texto.chars().map(|c| fonte.metrics(c, tamanho).advance_width).sum()
    }

    /// As telas em pé: o Android do jogo é travado em retrato. Em tela deitada (800×600, só no
    /// computador) a coluna do jogo fica estreita e o texto encolhe até sumir: não é reprovado aqui.
    const TELAS_DO_TESTE: [(f32, f32); 4] = [(320.0, 480.0), (360.0, 640.0), (540.0, 960.0), (1080.0, 2400.0)];

    #[test]
    fn os_rotulos_dos_botoes_cabem_em_todas_as_telas() {
        for (largura, altura) in TELAS_DO_TESTE {
            for com_embaralhar in [true, false] {
                let tela = layout::tela_jogo(largura, altura, com_embaralhar);
                let mut botoes = vec![("Dica", tela.botao_dica), ("Novo jogo", tela.botao_novo_jogo)];
                if let Some(embaralhar) = tela.botao_embaralhar {
                    botoes.push(("Embaralhar", embaralhar));
                }
                for (rotulo, area) in botoes {
                    let tamanho = ROTULO_DO_BOTAO * tela.texto;
                    let util = area.w * (1.0 - 2.0 * MARGEM_DO_BOTAO);
                    let medida = |t: f32| largura_em_inter(Estilo::TextoForte, rotulo, t);
                    if largura >= 360.0 {
                        assert!(rotulo_cabe_com(medida, util, tamanho), "{largura}x{altura}: \"{rotulo}\" a {tamanho:.1} px não cabe em {util:.1} px");
                    } else {
                        // O menor celular: o rótulo pode encolher, mas continua legível (70% do texto).
                        let desenhado = texto::tamanho_que_cabe(medida(tamanho), util, tamanho);
                        assert!(desenhado >= 0.7 * tela.texto, "{largura}x{altura}: \"{rotulo}\" encolheria para {desenhado:.1} px");
                    }
                }
            }
        }
    }

    #[test]
    fn a_frase_de_cada_modo_cabe_nos_cartoes() {
        for (largura, altura) in TELAS_DO_TESTE {
            let tela = layout::tela_jogo(largura, altura, false);
            for (frase, area) in [(FRASE_ZEN, tela.aba_zen), (FRASE_CLASSICO, tela.aba_classico)] {
                let (_, _, util) = colunas_do_cartao_de_modo(area);
                let minimo = 0.20 * area.h;
                assert!(
                    rotulo_cabe_com(|t| largura_em_inter(Estilo::Texto, frase, t), util, minimo),
                    "{largura}x{altura}: \"{frase}\" a {minimo:.1} px não cabe em {util:.1} px (encurte a frase)"
                );
            }
        }
    }

    #[test]
    fn o_anel_da_casa_e_maior_que_a_casa_e_cabe_no_tabuleiro() {
        for (largura, altura) in [(360.0, 640.0), (1080.0, 2400.0)] {
            let tela = layout::tela_jogo(largura, altura, true);
            let moldura = tela.tabuleiro;
            for indice in [0, 7, 56, 63] {
                let casa = casa_visivel(tela.grade, indice);
                let anel = anel_da_casa(tela.grade, indice);
                let envolve = anel.x < casa.x && anel.y < casa.y && anel.x + anel.w > casa.x + casa.w && anel.y + anel.h > casa.y + casa.h;
                assert!(envolve, "{largura}x{altura} casa {indice}: o anel {anel:?} não envolve a casa {casa:?}");
                let dentro = anel.x >= moldura.x - 2.0
                    && anel.y >= moldura.y - 2.0
                    && anel.x + anel.w <= moldura.x + moldura.w + 2.0
                    && anel.y + anel.h <= moldura.y + moldura.h + 2.0;
                assert!(dentro, "{largura}x{altura} casa {indice}: o anel {anel:?} passa da moldura {moldura:?}");
                // E não invade a casa do lado: fica dentro do passo da própria casa.
                let passo = casa_de(tela.grade, indice);
                assert!(
                    anel.x >= passo.x && anel.y >= passo.y && anel.x + anel.w <= passo.x + passo.w && anel.y + anel.h <= passo.y + passo.h,
                    "{largura}x{altura} casa {indice}: o anel {anel:?} invade as vizinhas (passo {passo:?})"
                );
            }
        }
    }

    #[test]
    fn a_pelicula_do_fim_segue_o_raio_da_moldura() {
        for (largura, altura) in [(360.0, 640.0), (1080.0, 2400.0)] {
            let moldura = layout::tela_jogo(largura, altura, true).tabuleiro;
            assert!(raio_da_moldura(moldura) > 0.0);
            let (area, raio, alfa) = pelicula_do_fim(moldura);
            assert_eq!(area, moldura, "a película cobre a moldura inteira");
            assert_eq!(raio, raio_da_moldura(moldura), "e arredonda como ela");
            assert!(alfa > 0.0 && alfa < 1.0);
        }
    }
}
