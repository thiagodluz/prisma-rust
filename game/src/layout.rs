//! Contas de posição: dado o tamanho da tela, onde fica o tabuleiro e cada casa.
//!
//! Tudo aqui é número puro (sem janela, sem imagem), por isso dá para testar de forma simples.

use prisma_engine::SIZE;

/// Margem em volta do tabuleiro: 3% do menor lado da tela, de cada lado.
const MARGEM: f32 = 0.03;

/// Um retângulo na tela, em pixels: canto de cima à esquerda (`x`, `y`) e tamanho (`w`, `h`).
///
/// `f32` é o número com vírgula do Rust (32 bits), o tipo que o Macroquad usa para desenhar.
/// O `derive` pede ao compilador para criar sozinho cópia, impressão (`{:?}`) e comparação.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// Um retângulo sem tamanho, no canto da tela: o que devolvemos quando a tela não existe.
    const VAZIO: Rect = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };

    /// O ponto (`x`, `y`) está dentro do retângulo? As bordas da esquerda e de cima contam
    /// como dentro; as da direita e de baixo, como fora (assim dois retângulos lado a lado
    /// nunca "disputam" o mesmo toque). Número inválido (NaN) nunca está dentro.
    pub fn contem(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// O quadrado do tabuleiro: centrado na tela, com lado igual ao menor lado da tela
/// menos a margem. Funciona igual em pé (retrato) e deitado (paisagem).
///
/// Uma tela "impossível" (tamanho zero, negativo, NaN ou infinito, como acontece por um
/// instante quando a janela é minimizada) devolve um retângulo vazio em vez de quebrar.
pub fn board_rect(screen_w: f32, screen_h: f32) -> Rect {
    let tela_valida = screen_w.is_finite() && screen_h.is_finite() && screen_w > 0.0 && screen_h > 0.0;
    if !tela_valida {
        return Rect::VAZIO;
    }
    let lado = screen_w.min(screen_h) * (1.0 - 2.0 * MARGEM);
    Rect {
        x: (screen_w - lado) / 2.0,
        y: (screen_h - lado) / 2.0,
        w: lado,
        h: lado,
    }
}

/// A casa da linha `row` e da coluna `col` (de 0 a `SIZE - 1`, contando do canto de cima
/// à esquerda) dentro do tabuleiro `board`. As casas se encostam, sem buracos.
pub fn cell_rect(board: Rect, row: usize, col: usize) -> Rect {
    let w = board.w / SIZE as f32;
    let h = board.h / SIZE as f32;
    Rect {
        x: board.x + col as f32 * w,
        y: board.y + row as f32 * h,
        w,
        h,
    }
}

/// Onde ficam os textos e os dois botões da tela de teste do M0b.
/// As posições verticais (`y_...`) são a linha de base do texto, como o Macroquad pede.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TelaDiagnostico {
    /// Margem esquerda do texto.
    pub x_texto: f32,
    pub y_titulo: f32,
    /// As três linhas de resultado, de cima para baixo: contador, som, vibração.
    pub y_linhas: [f32; 3],
    pub tamanho_titulo: f32,
    pub tamanho_texto: f32,
    pub botao_som: Rect,
    pub botao_vibrar: Rect,
}

/// Posição de tudo na tela de teste, a partir do tamanho da tela. Tudo é proporcional ao
/// tamanho, então serve para celular em pé, tela alta e janela de PC.
///
/// Uma tela "impossível" (tamanho zero, negativo, NaN ou infinito) devolve tudo zerado, com
/// botões sem tamanho, que nunca recebem toque.
pub fn tela_diagnostico(largura: f32, altura: f32) -> TelaDiagnostico {
    let tela_valida = largura.is_finite() && altura.is_finite() && largura > 0.0 && altura > 0.0;
    if !tela_valida {
        return TelaDiagnostico {
            x_texto: 0.0,
            y_titulo: 0.0,
            y_linhas: [0.0; 3],
            tamanho_titulo: 0.0,
            tamanho_texto: 0.0,
            botao_som: Rect::VAZIO,
            botao_vibrar: Rect::VAZIO,
        };
    }
    let menor_lado = largura.min(altura);
    let botao = |topo: f32| Rect {
        x: 0.06 * largura,
        y: topo * altura,
        w: 0.88 * largura,
        h: 0.09 * altura,
    };
    TelaDiagnostico {
        x_texto: 0.06 * largura,
        y_titulo: 0.10 * altura,
        y_linhas: [0.20 * altura, 0.28 * altura, 0.36 * altura],
        tamanho_titulo: 0.075 * menor_lado,
        tamanho_texto: 0.045 * menor_lado,
        botao_som: botao(0.50),
        botao_vibrar: botao(0.62),
    }
}

// ---- Tela de jogo ----------------------------------------------------------------------------

/// Alturas das peças, em frações da unidade `u` (o menor lado da tela).
const ABAS_ALTURA: f32 = 0.085;
const PAINEL_ALTURA: f32 = 0.11;
const BARRA_ALTURA: f32 = 0.022;
const ROTULO_ALTURA: f32 = 0.05;
const BOTOES_ALTURA: f32 = 0.10;
/// Espaço entre uma peça e a seguinte (e margem em cima e embaixo do conjunto).
const ESPACO: f32 = 0.02;
/// A linha do guia de respiração, logo abaixo dos botões (só aparece no Zen, mas o espaço dela
/// é reservado nos dois modos, para o tabuleiro não mudar de lugar ao trocar de modo).
const GUIA_ALTURA: f32 = 0.07;
/// Altura máxima de uma linha da folha de Som, para os controles não ficarem enormes em tela alta.
const FILA_MAXIMA: f32 = 0.15;
/// O tabuleiro (e tudo que fica alinhado com ele) ocupa no máximo 94% da largura.
const LARGURA_MAXIMA: f32 = 0.94;
/// Tamanho base da letra: 4,5% do menor lado.
const TEXTO: f32 = 0.045;

/// Onde fica cada controle da folha de Som, a janela que cobre da barra de progresso até os
/// botões. Os controles do Zen (ligar e escolher Música e Ambiente, Respiração) só existem no
/// modo Zen: no Clássico são `None`, e as linhas que sobram ficam maiores.
///
/// As "barras" são a linha inteira: o valor sai da posição `x` do dedo dentro dela.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TelaSom {
    pub folha: Rect,
    pub titulo: Rect,
    pub sons_do_jogo: Rect,
    pub testar: Rect,
    pub barra_efeitos: Rect,
    pub liga_musica: Option<Rect>,
    pub escolhe_musica: Option<Rect>,
    pub barra_musica: Rect,
    pub liga_ambiente: Option<Rect>,
    pub escolhe_ambiente: Option<Rect>,
    pub barra_ambiente: Rect,
    pub respiracao: Option<Rect>,
    /// A linha "Intensidade" dos efeitos visuais (Suave, Padrão, Vibrante): a última da folha do
    /// Zen, depois da respiração. No Clássico não existe (lá os efeitos valem o Padrão).
    pub efeitos: Option<Rect>,
    /// O interruptor "Vibração", logo depois do volume dos efeitos, nos dois modos. `None` em
    /// aparelho sem vibrador (e aí as outras linhas ocupam o lugar dela).
    pub vibracao: Option<Rect>,
    pub fechar: Rect,
    /// Tamanho base da letra.
    pub texto: f32,
}

impl TelaSom {
    /// Tudo vazio: o que devolvemos quando a tela não existe.
    pub const VAZIA: TelaSom = TelaSom {
        folha: Rect::VAZIO,
        titulo: Rect::VAZIO,
        sons_do_jogo: Rect::VAZIO,
        testar: Rect::VAZIO,
        barra_efeitos: Rect::VAZIO,
        liga_musica: None,
        escolhe_musica: None,
        barra_musica: Rect::VAZIO,
        liga_ambiente: None,
        escolhe_ambiente: None,
        barra_ambiente: Rect::VAZIO,
        respiracao: None,
        efeitos: None,
        vibracao: None,
        fechar: Rect::VAZIO,
        texto: 0.0,
    };
}

/// A folha do Menu (≡): duas abas, Recordes e Créditos, o ✕ para fechar e, abaixo, a área do
/// conteúdo. Ocupa o mesmo lugar da folha de Som (só uma das duas abre por vez).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TelaMenu {
    pub folha: Rect,
    pub aba_recordes: Rect,
    pub aba_creditos: Rect,
    pub fechar: Rect,
    /// Onde entram as linhas de recordes ou o texto dos créditos.
    pub conteudo: Rect,
    /// Tamanho base da letra.
    pub texto: f32,
}

impl TelaMenu {
    /// Tudo vazio: o que devolvemos quando a tela não existe.
    pub const VAZIA: TelaMenu = TelaMenu {
        folha: Rect::VAZIO,
        aba_recordes: Rect::VAZIO,
        aba_creditos: Rect::VAZIO,
        fechar: Rect::VAZIO,
        conteudo: Rect::VAZIO,
        texto: 0.0,
    };
}

/// Onde fica cada peça da tela de jogo (abas, painel, tabuleiro, botões, cartão de fim e
/// diálogo). Tudo em pixels, calculado a partir do tamanho da tela.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TelaJogo {
    /// As duas abas de modo, lado a lado.
    pub aba_zen: Rect,
    pub aba_classico: Rect,
    /// O botão ♪ (Som): quadrado, na linha das abas, à direita delas.
    pub botao_som: Rect,
    /// O botão ≡ (Menu): quadrado, no fim da linha das abas, logo depois do ♪.
    pub botao_menu: Rect,
    /// As três colunas do painel: PONTOS, NÍVEL, MELHOR.
    pub pontos: Rect,
    pub nivel: Rect,
    pub melhor: Rect,
    /// A barra de progresso do nível e a linha de texto logo abaixo dela.
    pub barra: Rect,
    pub rotulo: Rect,
    /// O tabuleiro: sempre um quadrado.
    pub tabuleiro: Rect,
    /// Só existe no modo Zen (no Clássico não dá para embaralhar).
    pub botao_embaralhar: Option<Rect>,
    pub botao_dica: Rect,
    pub botao_novo_jogo: Rect,
    /// A linha do guia de respiração, logo abaixo dos botões. Só existe no Zen (no Clássico é
    /// vazia), mas o espaço dela é reservado nos dois modos.
    pub guia_respiracao: Rect,
    /// A folha de Som, que abre por cima do tabuleiro e dos botões.
    pub som: TelaSom,
    /// A folha do Menu (Recordes e Créditos), que abre no mesmo lugar.
    pub menu: TelaMenu,
    /// O cartão de "Fim de partida", por cima do tabuleiro, e o botão dele.
    pub cartao_fim: Rect,
    pub botao_jogar_novamente: Rect,
    /// A pergunta "Começar novo jogo?", no meio da tela, e seus dois botões.
    pub dialogo: Rect,
    pub dialogo_cancelar: Rect,
    pub dialogo_confirmar: Rect,
    /// Tamanho base da letra.
    pub texto: f32,
}

impl TelaJogo {
    /// Tudo vazio: o que devolvemos quando a tela não existe. Nenhum toque cai em lugar nenhum.
    const VAZIA: TelaJogo = TelaJogo {
        aba_zen: Rect::VAZIO,
        aba_classico: Rect::VAZIO,
        botao_som: Rect::VAZIO,
        botao_menu: Rect::VAZIO,
        pontos: Rect::VAZIO,
        nivel: Rect::VAZIO,
        melhor: Rect::VAZIO,
        barra: Rect::VAZIO,
        rotulo: Rect::VAZIO,
        tabuleiro: Rect::VAZIO,
        botao_embaralhar: None,
        botao_dica: Rect::VAZIO,
        botao_novo_jogo: Rect::VAZIO,
        guia_respiracao: Rect::VAZIO,
        som: TelaSom::VAZIA,
        menu: TelaMenu::VAZIA,
        cartao_fim: Rect::VAZIO,
        botao_jogar_novamente: Rect::VAZIO,
        dialogo: Rect::VAZIO,
        dialogo_cancelar: Rect::VAZIO,
        dialogo_confirmar: Rect::VAZIO,
        texto: 0.0,
    };
}

/// Posição de tudo na tela de jogo.
///
/// De cima para baixo: abas (com o botão ♪ ao lado), painel, barra, rótulo, tabuleiro, botões
/// e a linha do guia de respiração. O conjunto fica centrado na vertical. O tabuleiro é o maior
/// quadrado que cabe: no máximo 94% da largura e no máximo a altura que sobra depois de
/// reservar o resto (inclusive a linha do guia, que no Clássico fica vazia, para o tabuleiro
/// não mudar de lugar entre os modos). As outras peças têm a mesma largura do tabuleiro, para
/// ficarem alinhadas com ele.
///
/// `com_embaralhar` falso (modo Clássico) tira o botão Embaralhar; os dois que sobram ficam
/// centrados e do mesmo tamanho que ficariam entre três.
///
/// Uma tela "impossível" (tamanho zero, negativo, NaN ou infinito) devolve tudo vazio.
pub fn tela_jogo(largura: f32, altura: f32, com_embaralhar: bool) -> TelaJogo {
    tela_jogo_com(largura, altura, com_embaralhar, true)
}

/// Como [`tela_jogo`], dizendo também se o aparelho tem vibrador: sem ele a folha de Som não
/// tem a linha "Vibração" (as outras linhas ficam um pouco maiores). O resto da tela, o
/// tabuleiro inclusive, é igual com ou sem vibrador.
pub fn tela_jogo_com(largura: f32, altura: f32, com_embaralhar: bool, vibrador: bool) -> TelaJogo {
    let tela_valida = largura.is_finite() && altura.is_finite() && largura > 0.0 && altura > 0.0;
    if !tela_valida {
        return TelaJogo::VAZIA;
    }
    let unidade = largura.min(altura);
    let espaco = ESPACO * unidade;
    let abas_h = ABAS_ALTURA * unidade;
    let painel_h = PAINEL_ALTURA * unidade;
    let barra_h = BARRA_ALTURA * unidade;
    let rotulo_h = ROTULO_ALTURA * unidade;
    let botoes_h = BOTOES_ALTURA * unidade;
    let guia_h = GUIA_ALTURA * unidade;

    // Tudo menos o tabuleiro: seis peças e seis espaços entre as sete peças.
    let fixo = abas_h + painel_h + barra_h + rotulo_h + botoes_h + guia_h + 6.0 * espaco;
    // O tabuleiro nunca passa da largura máxima nem da altura livre (reservando também uma
    // margem de um espaço em cima e outra embaixo). A altura livre é sempre positiva, porque
    // `fixo` mais as margens é 0,597 do menor lado, que nunca passa da altura.
    let lado = (LARGURA_MAXIMA * largura).min(altura - fixo - 2.0 * espaco);

    let conjunto = fixo + lado;
    let esquerda = (largura - lado) / 2.0;
    let faixa = |x: f32, y: f32, w: f32, h: f32| Rect { x, y, w, h };

    // As posições de cima, descendo peça a peça.
    let mut y = (altura - conjunto) / 2.0;
    let abas_y = y;
    y += abas_h + espaco;
    let painel_y = y;
    y += painel_h + espaco;
    let barra_y = y;
    y += barra_h + espaco;
    let rotulo_y = y;
    y += rotulo_h + espaco;
    let tabuleiro_y = y;
    y += lado + espaco;
    let botoes_y = y;
    y += botoes_h + espaco;
    let guia_y = y;

    // Abas: duas metades com um espaço no meio, e os botões ♪ e ≡ (quadrados) no fim da linha.
    let aba_w = (lado - 2.0 * abas_h - 3.0 * espaco) / 2.0;
    // Painel: três colunas, a dos pontos mais larga (os números crescem).
    let util = lado - 2.0 * espaco;
    let (pontos_w, nivel_w, melhor_w) = (0.40 * util, 0.24 * util, 0.36 * util);
    let nivel_x = esquerda + pontos_w + espaco;
    let melhor_x = nivel_x + nivel_w + espaco;
    // Botões: a largura é a de "três botões"; com dois, eles ficam centrados.
    let botao_w = (lado - 2.0 * espaco) / 3.0;
    let quantos = if com_embaralhar { 3.0 } else { 2.0 };
    let primeiro_x = esquerda + (lado - (quantos * botao_w + (quantos - 1.0) * espaco)) / 2.0;
    let passo = botao_w + espaco;
    let (botao_embaralhar, botao_dica_x) = if com_embaralhar {
        (Some(faixa(primeiro_x, botoes_y, botao_w, botoes_h)), primeiro_x + passo)
    } else {
        (None, primeiro_x)
    };

    let tabuleiro = faixa(esquerda, tabuleiro_y, lado, lado);
    let guia_respiracao = if com_embaralhar { faixa(esquerda, guia_y, lado, guia_h) } else { Rect::VAZIO };
    // A folha de Som cobre da barra de progresso até o fim dos botões.
    let folha = faixa(esquerda, barra_y, lado, botoes_y + botoes_h - barra_y);

    // Cartão de "Fim de partida": centrado no tabuleiro, com o botão perto da borda de baixo.
    let cartao_w = 0.84 * lado;
    let cartao_h = 0.56 * lado;
    let cartao_fim = faixa(esquerda + (lado - cartao_w) / 2.0, tabuleiro_y + (lado - cartao_h) / 2.0, cartao_w, cartao_h);
    let jogar_w = 0.64 * cartao_w;
    let jogar_h = 0.22 * cartao_h;
    let botao_jogar_novamente = faixa(
        cartao_fim.x + (cartao_w - jogar_w) / 2.0,
        cartao_fim.y + cartao_h - 0.08 * cartao_h - jogar_h,
        jogar_w,
        jogar_h,
    );

    // Diálogo de novo jogo: no meio da tela, com Cancelar à esquerda e confirmar à direita.
    let dialogo_w = 0.86 * lado;
    let dialogo_h = 0.44 * lado;
    let dialogo = faixa((largura - dialogo_w) / 2.0, (altura - dialogo_h) / 2.0, dialogo_w, dialogo_h);
    let margem_d = 0.05 * dialogo_w;
    let escolha_w = (dialogo_w - 3.0 * margem_d) / 2.0;
    let escolha_h = 0.26 * dialogo_h;
    let escolha_y = dialogo.y + dialogo_h - 0.08 * dialogo_h - escolha_h;

    TelaJogo {
        aba_zen: faixa(esquerda, abas_y, aba_w, abas_h),
        aba_classico: faixa(esquerda + aba_w + espaco, abas_y, aba_w, abas_h),
        botao_som: faixa(esquerda + lado - 2.0 * abas_h - espaco, abas_y, abas_h, abas_h),
        botao_menu: faixa(esquerda + lado - abas_h, abas_y, abas_h, abas_h),
        pontos: faixa(esquerda, painel_y, pontos_w, painel_h),
        nivel: faixa(nivel_x, painel_y, nivel_w, painel_h),
        melhor: faixa(melhor_x, painel_y, melhor_w, painel_h),
        barra: faixa(esquerda, barra_y, lado, barra_h),
        rotulo: faixa(esquerda, rotulo_y, lado, rotulo_h),
        tabuleiro,
        botao_embaralhar,
        botao_dica: faixa(botao_dica_x, botoes_y, botao_w, botoes_h),
        botao_novo_jogo: faixa(botao_dica_x + passo, botoes_y, botao_w, botoes_h),
        guia_respiracao,
        som: folha_de_som(folha, unidade, com_embaralhar, vibrador),
        menu: folha_do_menu(folha, unidade),
        cartao_fim,
        botao_jogar_novamente,
        dialogo,
        dialogo_cancelar: faixa(dialogo.x + margem_d, escolha_y, escolha_w, escolha_h),
        dialogo_confirmar: faixa(dialogo.x + 2.0 * margem_d + escolha_w, escolha_y, escolha_w, escolha_h),
        texto: TEXTO * unidade,
    }
}

/// Divide uma linha em duas partes lado a lado: a da esquerda com `esquerda_w` de largura e a
/// da direita com o que sobra, separadas por `vao`.
fn dividir(linha: Rect, esquerda_w: f32, vao: f32) -> (Rect, Rect) {
    let esquerda = Rect { w: esquerda_w, ..linha };
    let direita = Rect { x: linha.x + esquerda_w + vao, w: linha.w - esquerda_w - vao, ..linha };
    (esquerda, direita)
}

/// Posição dos controles da folha de Som dentro de `folha`. As linhas têm todas a mesma altura
/// (no máximo `FILA_MAXIMA`) e o bloco fica centrado na vertical. No Zen são oito linhas:
/// título e Fechar; Sons do jogo e Testar; Efeitos; ligar e escolher a Música; volume da Música;
/// ligar e escolher o Ambiente; volume do Ambiente; Respiração. No Clássico, cinco: as
/// três primeiras e os volumes da Música e do Ambiente. Com vibrador entra mais uma linha, a da
/// Vibração, logo depois do volume dos Efeitos, nos dois modos.
fn folha_de_som(folha: Rect, unidade: f32, zen: bool, vibrador: bool) -> TelaSom {
    let espaco = ESPACO * unidade;
    let vao = espaco / 2.0;
    let borda = 1.5 * espaco;
    // Quantas linhas da folha vêm depois do volume dos Efeitos é decidido aqui: a da Vibração (se
    // houver) empurra todas as seguintes uma linha para baixo.
    let deslocamento = usize::from(vibrador);
    let linhas = ((if zen { 9 } else { 5 }) + deslocamento) as f32;
    let altura_livre = folha.h - 2.0 * borda;
    let altura = ((altura_livre - (linhas - 1.0) * vao) / linhas).min(FILA_MAXIMA * unidade);
    let bloco = linhas * altura + (linhas - 1.0) * vao;
    let topo = folha.y + (folha.h - bloco) / 2.0;
    let largura = folha.w - 2.0 * borda;
    let linha = |i: usize| Rect { x: folha.x + borda, y: topo + i as f32 * (altura + vao), w: largura, h: altura };
    // A linha de número `i` entre as que vêm depois do volume dos Efeitos (a 0 é a primeira).
    let depois = |i: usize| linha(3 + deslocamento + i);

    let (titulo, fechar) = dividir(linha(0), largura - altura - vao, vao);
    let (sons_do_jogo, testar) = dividir(linha(1), 0.62 * largura, vao);
    let barra_efeitos = linha(2);
    let vibracao = vibrador.then(|| linha(3));
    let (liga_musica, escolhe_musica, barra_musica, liga_ambiente, escolhe_ambiente, barra_ambiente, respiracao, efeitos);
    if zen {
        let (liga, escolhe) = dividir(depois(0), 0.40 * largura, vao);
        (liga_musica, escolhe_musica, barra_musica) = (Some(liga), Some(escolhe), depois(1));
        let (liga, escolhe) = dividir(depois(2), 0.40 * largura, vao);
        (liga_ambiente, escolhe_ambiente, barra_ambiente) = (Some(liga), Some(escolhe), depois(3));
        respiracao = Some(depois(4));
        efeitos = Some(depois(5));
    } else {
        (liga_musica, escolhe_musica, barra_musica) = (None, None, depois(0));
        (liga_ambiente, escolhe_ambiente, barra_ambiente) = (None, None, depois(1));
        respiracao = None;
        efeitos = None;
    }
    TelaSom {
        folha,
        titulo,
        sons_do_jogo,
        testar,
        barra_efeitos,
        liga_musica,
        escolhe_musica,
        barra_musica,
        liga_ambiente,
        escolhe_ambiente,
        barra_ambiente,
        respiracao,
        efeitos,
        vibracao,
        fechar,
        texto: TEXTO * unidade,
    }
}

/// Posição dos controles da folha do Menu dentro de `folha`: no alto, as abas Recordes e
/// Créditos e o ✕ (quadrado, no canto direito), todos da altura das abas da tela; abaixo deles,
/// a área do conteúdo, até o fim da folha.
fn folha_do_menu(folha: Rect, unidade: f32) -> TelaMenu {
    let espaco = ESPACO * unidade;
    let borda = 1.5 * espaco;
    let altura = ABAS_ALTURA * unidade;
    let largura = folha.w - 2.0 * borda;
    let topo = folha.y + borda;
    let aba_w = (largura - altura - 2.0 * espaco) / 2.0;
    let faixa = |x: f32, y: f32, w: f32, h: f32| Rect { x, y, w, h };
    let x = folha.x + borda;
    let conteudo_y = topo + altura + espaco;
    TelaMenu {
        folha,
        aba_recordes: faixa(x, topo, aba_w, altura),
        aba_creditos: faixa(x + aba_w + espaco, topo, aba_w, altura),
        fechar: faixa(x + largura - altura, topo, altura, altura),
        conteudo: faixa(x, conteudo_y, largura, (folha.y + folha.h - borda - conteudo_y).max(0.0)),
        texto: TEXTO * unidade,
    }
}

/// Um bloco da folha de Recordes já com o lugar de cada coisa: a legenda ("Zen") e um cartão por linha.
#[derive(Clone, Debug, PartialEq)]
pub struct BlocoDaFolha {
    pub legenda: Rect,
    pub cartoes: Vec<Rect>,
}

/// Reparte a área `conteudo` da folha entre os blocos de recordes (um por modo, de cima para baixo).
/// `linhas_por_bloco` diz quantas linhas cada bloco tem e `texto` é o tamanho base da letra. Cada
/// bloco é uma legenda seguida dos cartões; os cartões dividem igualmente o que sobra, sem passar
/// de 3 tamanhos de letra de altura. Área pequena demais dá alturas zero, nunca negativas nem `NaN`.
pub fn repartir_recordes(conteudo: Rect, texto: f32, linhas_por_bloco: &[usize]) -> Vec<BlocoDaFolha> {
    let legenda = 1.6 * texto;
    let vao = 0.5 * texto;
    let entre_blocos = texto;
    let total_de_linhas: usize = linhas_por_bloco.iter().sum();
    let fixo = linhas_por_bloco.len() as f32 * legenda
        + total_de_linhas as f32 * vao
        + linhas_por_bloco.len().saturating_sub(1) as f32 * entre_blocos;
    let altura = if total_de_linhas == 0 { 0.0 } else { ((conteudo.h - fixo) / total_de_linhas as f32).clamp(0.0, 3.0 * texto) };
    let legenda = legenda.min(conteudo.h.max(0.0));
    let mut y = conteudo.y;
    let mut blocos = Vec::with_capacity(linhas_por_bloco.len());
    for &linhas in linhas_por_bloco {
        let rotulo = Rect { y, h: legenda, ..conteudo };
        y += legenda;
        let mut cartoes = Vec::with_capacity(linhas);
        for _ in 0..linhas {
            y += vao;
            cartoes.push(Rect { y, h: altura, ..conteudo });
            y += altura;
        }
        y += entre_blocos;
        blocos.push(BlocoDaFolha { legenda: rotulo, cartoes });
    }
    blocos
}

/// A folha de Som para uma tela e um modo (`zen` falso é o Clássico). É a mesma que
/// `tela_jogo` guarda em `som`. Tela "impossível" devolve tudo vazio.
#[cfg(test)]
pub fn tela_som(largura: f32, altura: f32, zen: bool) -> TelaSom {
    tela_jogo(largura, altura, zen).som
}

#[cfg(test)]
mod tests {
    use super::*;
    use prisma_engine::SIZE;

    /// Compara números decimais (`f32`) com uma folguinha, porque contas com vírgula
    /// raramente dão o valor exato.
    fn perto(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn tabuleiro_e_quadrado_e_cabe_na_tela() {
        for (largura, altura) in [(540.0, 960.0), (960.0, 540.0), (1080.0, 2400.0), (400.0, 400.0)] {
            let b = board_rect(largura, altura);
            assert!(perto(b.w, b.h), "tela {largura}x{altura}: {b:?} não é quadrado");
            assert!(b.x >= 0.0 && b.y >= 0.0, "tela {largura}x{altura}: {b:?} sai pela esquerda/topo");
            assert!(b.x + b.w <= largura + 0.01, "tela {largura}x{altura}: {b:?} sai pela direita");
            assert!(b.y + b.h <= altura + 0.01, "tela {largura}x{altura}: {b:?} sai por baixo");
        }
    }

    #[test]
    fn tabuleiro_fica_centralizado() {
        let b = board_rect(540.0, 960.0);
        assert!(perto(b.x + b.w / 2.0, 270.0), "centro horizontal errado: {b:?}");
        assert!(perto(b.y + b.h / 2.0, 480.0), "centro vertical errado: {b:?}");
    }

    #[test]
    fn retrato_e_paisagem_usam_o_menor_lado() {
        let retrato = board_rect(540.0, 960.0);
        let paisagem = board_rect(960.0, 540.0);
        assert!(perto(retrato.w, paisagem.w));
        // 540 menos 3% de margem de cada lado = 540 × 0,94.
        assert!(perto(retrato.w, 507.6), "lado esperado 507,6, veio {}", retrato.w);
    }

    #[test]
    fn casas_ladrilham_o_tabuleiro() {
        let b = board_rect(540.0, 960.0);
        let primeira = cell_rect(b, 0, 0);
        assert!(perto(primeira.x, b.x) && perto(primeira.y, b.y));
        let ultima = cell_rect(b, SIZE - 1, SIZE - 1);
        assert!(perto(ultima.x + ultima.w, b.x + b.w));
        assert!(perto(ultima.y + ultima.h, b.y + b.h));
        for linha in 0..SIZE {
            for coluna in 0..SIZE {
                let casa = cell_rect(b, linha, coluna);
                assert!(perto(casa.w, b.w / SIZE as f32), "largura da casa ({linha},{coluna})");
                assert!(perto(casa.h, b.h / SIZE as f32), "altura da casa ({linha},{coluna})");
                if coluna + 1 < SIZE {
                    let direita = cell_rect(b, linha, coluna + 1);
                    assert!(perto(casa.x + casa.w, direita.x), "buraco entre ({linha},{coluna}) e a da direita");
                }
                if linha + 1 < SIZE {
                    let abaixo = cell_rect(b, linha + 1, coluna);
                    assert!(perto(casa.y + casa.h, abaixo.y), "buraco entre ({linha},{coluna}) e a de baixo");
                }
            }
        }
    }

    #[test]
    fn tela_degenerada_nao_quebra() {
        for (largura, altura) in [
            (0.0, 0.0),
            (1.0, 1.0),
            (-5.0, 100.0),
            (f32::NAN, 100.0),
            (100.0, f32::NAN),
            (f32::INFINITY, 100.0),
            (100_000.0, 1.0),
        ] {
            let b = board_rect(largura, altura);
            let tudo = [b.x, b.y, b.w, b.h];
            assert!(tudo.iter().all(|n| n.is_finite()), "tela {largura}x{altura}: {b:?} tem NaN ou infinito");
            assert!(b.w >= 0.0 && b.h >= 0.0, "tela {largura}x{altura}: {b:?} com tamanho negativo");
            if b.w > 0.0 {
                // Quando sobra tabuleiro, ele precisa caber na tela.
                assert!(b.x >= 0.0 && b.y >= 0.0, "tela {largura}x{altura}: {b:?} fora da tela");
                assert!(b.x + b.w <= largura + 0.01 && b.y + b.h <= altura + 0.01, "tela {largura}x{altura}: {b:?} fora da tela");
            }
            // As casas de um tabuleiro degenerado também não podem ter NaN.
            let casa = cell_rect(b, SIZE - 1, SIZE - 1);
            assert!([casa.x, casa.y, casa.w, casa.h].iter().all(|n| n.is_finite()));
        }
    }

    /// Tamanhos de tela usados nos testes da tela de diagnóstico: celulares em pé, um celular
    /// bem alto, uma tela deitada (PC) e uma quadrada.
    const TELAS: [(f32, f32); 5] =
        [(540.0, 960.0), (360.0, 640.0), (1080.0, 2400.0), (960.0, 540.0), (300.0, 300.0)];

    #[test]
    fn botoes_ficam_dentro_da_tela() {
        for (largura, altura) in TELAS {
            let tela = tela_diagnostico(largura, altura);
            for botao in [tela.botao_som, tela.botao_vibrar] {
                assert!(botao.x >= 0.0 && botao.y >= 0.0, "tela {largura}x{altura}: {botao:?}");
                assert!(botao.x + botao.w <= largura + 0.01, "tela {largura}x{altura}: {botao:?}");
                assert!(botao.y + botao.h <= altura + 0.01, "tela {largura}x{altura}: {botao:?}");
            }
        }
    }

    #[test]
    fn botoes_nao_se_sobrepoem() {
        for (largura, altura) in TELAS {
            let tela = tela_diagnostico(largura, altura);
            assert!(
                tela.botao_som.y + tela.botao_som.h < tela.botao_vibrar.y,
                "tela {largura}x{altura}: botões se encostam ou se sobrepõem"
            );
        }
    }

    #[test]
    fn texto_fica_acima_dos_botoes_e_as_linhas_nao_se_sobrepoem() {
        for (largura, altura) in TELAS {
            let tela = tela_diagnostico(largura, altura);
            assert!(tela.y_linhas[2] < tela.botao_som.y, "tela {largura}x{altura}: texto cai nos botões");
            assert!(tela.y_titulo < tela.y_linhas[0], "tela {largura}x{altura}: título abaixo das linhas");
            for par in tela.y_linhas.windows(2) {
                assert!(
                    par[1] - par[0] >= 1.2 * tela.tamanho_texto,
                    "tela {largura}x{altura}: linhas muito juntas"
                );
            }
        }
    }

    #[test]
    fn botoes_em_pe_tem_altura_boa_para_o_dedo() {
        for (largura, altura) in [(540.0, 960.0), (360.0, 640.0)] {
            let tela = tela_diagnostico(largura, altura);
            assert!(tela.botao_som.h >= 0.08 * altura, "tela {largura}x{altura}");
            assert!(tela.botao_vibrar.h >= 0.08 * altura, "tela {largura}x{altura}");
        }
    }

    #[test]
    fn contem_inclui_esquerda_e_topo_e_exclui_direita_e_baixo() {
        let r = Rect { x: 10.0, y: 20.0, w: 100.0, h: 50.0 };
        assert!(r.contem(10.0, 20.0), "canto de cima à esquerda é dentro");
        assert!(r.contem(60.0, 45.0), "meio é dentro");
        assert!(r.contem(109.9, 69.9));
        assert!(!r.contem(110.0, 45.0), "borda direita é fora");
        assert!(!r.contem(60.0, 70.0), "borda de baixo é fora");
        assert!(!r.contem(9.9, 45.0));
        assert!(!r.contem(60.0, 19.9));
    }

    #[test]
    fn contem_rejeita_nan() {
        let r = Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 };
        assert!(!r.contem(f32::NAN, 50.0));
        assert!(!r.contem(50.0, f32::NAN));
    }

    #[test]
    fn tela_invalida_da_botoes_vazios() {
        for (largura, altura) in [(0.0, 0.0), (-1.0, 5.0), (f32::NAN, 10.0), (f32::INFINITY, 10.0)] {
            let tela = tela_diagnostico(largura, altura);
            for botao in [tela.botao_som, tela.botao_vibrar] {
                assert_eq!((botao.w, botao.h), (0.0, 0.0), "tela {largura}x{altura}");
                assert!(!botao.contem(0.0, 0.0), "tela {largura}x{altura}");
            }
        }
    }
}

#[cfg(test)]
mod tests_tela_jogo {
    use super::*;

    /// Telas de teste: celulares em pé, tela alta, janela deitada, quadrada, estreita, larga e minúscula.
    const TELAS: [(f32, f32); 9] = [
        (540.0, 960.0),
        (1080.0, 2340.0),
        (1080.0, 2400.0),
        (360.0, 640.0),
        (800.0, 600.0),
        (1000.0, 1000.0),
        (200.0, 2000.0),
        (2000.0, 200.0),
        (50.0, 50.0),
    ];

    /// Todos os retângulos da tela de jogo que mostram algo ou recebem toque (sem os do
    /// cartão de fim e do diálogo, que ficam por cima e têm testes próprios).
    fn pecas(tela: &TelaJogo) -> Vec<(&'static str, Rect)> {
        let mut pecas = vec![
            ("aba_zen", tela.aba_zen),
            ("aba_classico", tela.aba_classico),
            ("pontos", tela.pontos),
            ("nivel", tela.nivel),
            ("melhor", tela.melhor),
            ("barra", tela.barra),
            ("rotulo", tela.rotulo),
            ("tabuleiro", tela.tabuleiro),
            ("botao_dica", tela.botao_dica),
            ("botao_novo_jogo", tela.botao_novo_jogo),
            ("botao_som", tela.botao_som),
            ("botao_menu", tela.botao_menu),
        ];
        if let Some(embaralhar) = tela.botao_embaralhar {
            pecas.push(("botao_embaralhar", embaralhar));
        }
        pecas
    }

    fn se_cruzam(a: Rect, b: Rect) -> bool {
        // Estritamente: retângulos que só se encostam não se cruzam.
        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
    }

    fn dentro(interno: Rect, externo: Rect) -> bool {
        let folga = 1e-3;
        interno.x >= externo.x - folga
            && interno.y >= externo.y - folga
            && interno.x + interno.w <= externo.x + externo.w + folga
            && interno.y + interno.h <= externo.y + externo.h + folga
    }

    #[test]
    fn tudo_cabe_na_tela() {
        for (largura, altura) in TELAS {
            for com_embaralhar in [true, false] {
                let tela = tela_jogo(largura, altura, com_embaralhar);
                let janela = Rect { x: 0.0, y: 0.0, w: largura, h: altura };
                let mut todos = pecas(&tela);
                todos.extend([
                    ("cartao_fim", tela.cartao_fim),
                    ("botao_jogar_novamente", tela.botao_jogar_novamente),
                    ("dialogo", tela.dialogo),
                    ("dialogo_cancelar", tela.dialogo_cancelar),
                    ("dialogo_confirmar", tela.dialogo_confirmar),
                ]);
                for (nome, retangulo) in todos {
                    assert!(retangulo.w > 0.0 && retangulo.h > 0.0, "{nome} sem tamanho em {largura}x{altura}");
                    assert!(dentro(retangulo, janela), "{nome} fora da tela {largura}x{altura}: {retangulo:?}");
                }
                assert!(tela.texto > 0.0);
            }
        }
    }

    #[test]
    fn elementos_nao_se_sobrepoem() {
        for (largura, altura) in TELAS {
            for com_embaralhar in [true, false] {
                let tela = tela_jogo(largura, altura, com_embaralhar);
                let todos = pecas(&tela);
                for (i, (nome_a, a)) in todos.iter().enumerate() {
                    for (nome_b, b) in &todos[i + 1..] {
                        assert!(!se_cruzam(*a, *b), "{nome_a} e {nome_b} se cruzam em {largura}x{altura}");
                    }
                }
                // O cartão de fim fica dentro do tabuleiro e o botão dele dentro do cartão.
                assert!(dentro(tela.cartao_fim, tela.tabuleiro), "cartão fora do tabuleiro em {largura}x{altura}");
                assert!(dentro(tela.botao_jogar_novamente, tela.cartao_fim));
                // O diálogo: botões dentro dele e um ao lado do outro.
                assert!(dentro(tela.dialogo_cancelar, tela.dialogo));
                assert!(dentro(tela.dialogo_confirmar, tela.dialogo));
                assert!(!se_cruzam(tela.dialogo_cancelar, tela.dialogo_confirmar));
            }
        }
    }

    #[test]
    fn ordem_vertical() {
        for (largura, altura) in TELAS {
            let tela = tela_jogo(largura, altura, true);
            assert!(tela.aba_zen.y < tela.pontos.y, "{largura}x{altura}");
            assert!(tela.pontos.y < tela.barra.y);
            assert!(tela.barra.y < tela.rotulo.y);
            assert!(tela.rotulo.y < tela.tabuleiro.y);
            assert!(tela.tabuleiro.y < tela.botao_dica.y);
            // As abas e as colunas do painel ficam na ordem da esquerda para a direita.
            assert!(tela.aba_zen.x < tela.aba_classico.x);
            assert!(tela.pontos.x < tela.nivel.x && tela.nivel.x < tela.melhor.x);
        }
    }

    #[test]
    fn tabuleiro_e_quadrado_e_grande_em_retrato() {
        for (largura, altura) in TELAS {
            let tela = tela_jogo(largura, altura, true);
            assert!((tela.tabuleiro.w - tela.tabuleiro.h).abs() < 1e-3, "{largura}x{altura}");
        }
        let tela = tela_jogo(540.0, 960.0, true);
        assert!(tela.tabuleiro.w >= 0.9 * 540.0, "em pé o tabuleiro ocupa quase a largura toda");
    }

    #[test]
    fn conjunto_fica_centrado_na_vertical() {
        for (largura, altura) in TELAS {
            let tela = tela_jogo(largura, altura, true);
            let em_cima = tela.aba_zen.y;
            // O conjunto vai das abas até o fim da linha da respiração, que fica reservada logo
            // abaixo dos botões (e continua reservada, vazia, no Clássico).
            let embaixo = altura - (tela.guia_respiracao.y + tela.guia_respiracao.h);
            assert!((em_cima - embaixo).abs() < 1e-2, "{largura}x{altura}: {em_cima} em cima, {embaixo} embaixo");
        }
    }

    #[test]
    fn embaralhar_some_e_os_outros_continuam_centrados() {
        for (largura, altura) in TELAS {
            let com = tela_jogo(largura, altura, true);
            let sem = tela_jogo(largura, altura, false);
            let embaralhar = com.botao_embaralhar.expect("com embaralhar tem o botão");
            assert_eq!(sem.botao_embaralhar, None);
            // Os botões que sobram têm o mesmo tamanho dos de antes.
            assert!((sem.botao_dica.w - com.botao_dica.w).abs() < 1e-3);
            assert!((sem.botao_dica.w - embaralhar.w).abs() < 1e-3);
            // E ficam centrados no tabuleiro: sobra a mesma folga dos dois lados.
            let esquerda = sem.botao_dica.x - sem.tabuleiro.x;
            let direita = (sem.tabuleiro.x + sem.tabuleiro.w) - (sem.botao_novo_jogo.x + sem.botao_novo_jogo.w);
            assert!((esquerda - direita).abs() < 1e-2, "{largura}x{altura}");
            // O tabuleiro e o resto do layout não mudam de lugar.
            assert_eq!(sem.tabuleiro, com.tabuleiro);
            assert_eq!(sem.aba_zen, com.aba_zen);
        }
    }

    #[test]
    fn tela_invalida_da_tudo_vazio() {
        for (largura, altura) in [(0.0, 0.0), (-1.0, 10.0), (10.0, -1.0), (f32::NAN, 10.0), (10.0, f32::NAN), (f32::INFINITY, 10.0)] {
            for com_embaralhar in [true, false] {
                let tela = tela_jogo(largura, altura, com_embaralhar);
                let vazio = Rect::VAZIO;
                for (nome, retangulo) in pecas(&tela) {
                    assert_eq!(retangulo, vazio, "{nome} com tela {largura}x{altura}");
                }
                assert_eq!(tela.botao_embaralhar, None);
                assert_eq!(tela.cartao_fim, vazio);
                assert_eq!(tela.dialogo, vazio);
                assert_eq!(tela.texto, 0.0);
                // Nenhum toque cai em lugar nenhum.
                assert!(!tela.tabuleiro.contem(0.0, 0.0));
            }
        }
    }

    // ---- Botão ♪, linha da respiração e folha de Som ----

    /// Os tamanhos de celular que a folha de Som precisa atender com folga.
    const CELULARES: [(f32, f32); 4] = [(360.0, 640.0), (720.0, 1280.0), (1080.0, 1920.0), (1080.0, 2400.0)];

    /// Todos os retângulos da folha (menos ela mesma) que existem naquele modo.
    fn controles(som: &TelaSom) -> Vec<(&'static str, Rect)> {
        let mut todos = vec![
            ("titulo", som.titulo),
            ("sons_do_jogo", som.sons_do_jogo),
            ("testar", som.testar),
            ("barra_efeitos", som.barra_efeitos),
            ("barra_musica", som.barra_musica),
            ("barra_ambiente", som.barra_ambiente),
            ("fechar", som.fechar),
        ];
        for (nome, retangulo) in [
            ("liga_musica", som.liga_musica),
            ("escolhe_musica", som.escolhe_musica),
            ("liga_ambiente", som.liga_ambiente),
            ("escolhe_ambiente", som.escolhe_ambiente),
            ("respiracao", som.respiracao),
            ("efeitos", som.efeitos),
            ("vibracao", som.vibracao),
        ] {
            if let Some(retangulo) = retangulo {
                todos.push((nome, retangulo));
            }
        }
        todos
    }

    #[test]
    fn tela_som_cabe_e_nao_se_sobrepoe() {
        for (largura, altura) in CELULARES {
            for zen in [true, false] {
                let tela = tela_jogo(largura, altura, zen);
                let som = tela_som(largura, altura, zen);
                assert_eq!(som, tela.som, "tela_som é a mesma folha de tela_jogo");
                let unidade = largura.min(altura);
                let janela = Rect { x: 0.0, y: 0.0, w: largura, h: altura };
                assert!(dentro(som.folha, janela), "folha fora da tela {largura}x{altura}");
                // A folha cobre o tabuleiro e os botões, sem tapar as abas nem o botão ♪.
                assert!(dentro(tela.tabuleiro, som.folha), "{largura}x{altura}: a folha tem de cobrir o tabuleiro");
                assert!(dentro(tela.botao_dica, som.folha) && dentro(tela.botao_novo_jogo, som.folha));
                assert!(som.folha.y >= tela.botao_som.y + tela.botao_som.h - 1e-3, "{largura}x{altura}: folha tapa o ♪");
                let todos = controles(&som);
                for (nome, retangulo) in &todos {
                    assert!(retangulo.w > 0.0 && retangulo.h > 0.0, "{nome} sem tamanho em {largura}x{altura} zen={zen}");
                    assert!(dentro(*retangulo, som.folha), "{nome} fora da folha em {largura}x{altura} zen={zen}: {retangulo:?}");
                    if *nome != "titulo" {
                        assert!(
                            retangulo.h >= 0.07 * unidade,
                            "{nome} baixo demais para o dedo em {largura}x{altura} zen={zen}: {} < {}",
                            retangulo.h,
                            0.07 * unidade
                        );
                    }
                }
                for (i, (nome_a, a)) in todos.iter().enumerate() {
                    for (nome_b, b) in &todos[i + 1..] {
                        assert!(!se_cruzam(*a, *b), "{nome_a} e {nome_b} se cruzam em {largura}x{altura} zen={zen}");
                    }
                }
                assert!(som.texto > 0.0);
            }
        }
    }

    #[test]
    fn tela_som_existe_e_cabe_em_qualquer_tela_possivel() {
        for (largura, altura) in TELAS {
            for zen in [true, false] {
                let som = tela_som(largura, altura, zen);
                let janela = Rect { x: 0.0, y: 0.0, w: largura, h: altura };
                for (nome, retangulo) in controles(&som) {
                    assert!(retangulo.w > 0.0 && retangulo.h > 0.0, "{nome} sem tamanho em {largura}x{altura}");
                    assert!(dentro(retangulo, janela) && dentro(retangulo, som.folha), "{nome} fora em {largura}x{altura}");
                }
            }
        }
    }

    #[test]
    fn no_classico_os_controles_do_zen_nao_existem() {
        for (largura, altura) in CELULARES {
            let classico = tela_som(largura, altura, false);
            assert_eq!(
                (classico.liga_musica, classico.escolhe_musica, classico.liga_ambiente, classico.escolhe_ambiente, classico.respiracao),
                (None, None, None, None, None),
                "{largura}x{altura}"
            );
            let zen = tela_som(largura, altura, true);
            assert!(zen.liga_musica.is_some() && zen.escolhe_musica.is_some());
            assert!(zen.liga_ambiente.is_some() && zen.escolhe_ambiente.is_some());
            assert!(zen.respiracao.is_some());
            // Os controles que existem nos dois modos ficam no mesmo lugar? Não precisam; mas a folha, sim.
            assert_eq!(zen.folha, classico.folha);
        }
    }

    #[test]
    fn a_folha_do_zen_com_10_linhas_cabe_em_360x640_e_1080x2400() {
        for (largura, altura) in [(360.0, 640.0), (1080.0, 2400.0)] {
            let tela = tela_jogo_com(largura, altura, true, true);
            let som = tela.som;
            let unidade = largura.min(altura);
            let efeitos = som.efeitos.expect("o Zen tem a linha de efeitos");
            let respiracao = som.respiracao.expect("o Zen tem a respiração");
            // A linha de efeitos é a última da folha, logo depois da respiração.
            assert!(efeitos.y >= respiracao.y + respiracao.h - 1e-3, "{largura}x{altura}: efeitos vem depois da respiração");
            assert!(dentro(efeitos, som.folha), "{largura}x{altura}: {efeitos:?} fora da folha");
            // Com a vibração, são 10 linhas: título, sons, volume, vibração, música (2), ambiente (2), respiração, efeitos.
            let todos = controles(&som);
            let mut topos: Vec<f32> = todos.iter().map(|(_, retangulo)| (retangulo.y * 100.0).round() / 100.0).collect();
            topos.sort_by(f32::total_cmp);
            topos.dedup();
            assert_eq!(topos.len(), 10, "{largura}x{altura}: a folha tem 10 linhas");
            for (nome, retangulo) in &todos {
                if *nome != "titulo" {
                    assert!(retangulo.h >= 0.07 * unidade, "{nome} baixo demais em {largura}x{altura}: {} < {}", retangulo.h, 0.07 * unidade);
                }
            }
            for (i, (nome_a, a)) in todos.iter().enumerate() {
                for (nome_b, b) in &todos[i + 1..] {
                    assert!(!se_cruzam(*a, *b), "{nome_a} e {nome_b} se cruzam em {largura}x{altura}");
                }
            }
            // Sem vibrador são 9 linhas, e a de efeitos continua lá.
            let sem_vibrador = tela_jogo_com(largura, altura, true, false).som;
            assert!(sem_vibrador.vibracao.is_none() && sem_vibrador.efeitos.is_some());
            let altura_de_cada = sem_vibrador.efeitos.unwrap().h;
            assert!(altura_de_cada >= efeitos.h - 1e-3, "sem a vibração as linhas não ficam menores");
        }
    }

    #[test]
    fn no_classico_nao_ha_linha_de_efeitos() {
        for (largura, altura) in CELULARES {
            for vibrador in [true, false] {
                let classico = tela_jogo_com(largura, altura, false, vibrador).som;
                assert_eq!(classico.efeitos, None, "{largura}x{altura} vibrador={vibrador}");
                assert!(tela_jogo_com(largura, altura, true, vibrador).som.efeitos.is_some());
            }
        }
        assert_eq!(tela_som(0.0, 0.0, true).efeitos, None, "tela impossível: tudo vazio");
    }

    #[test]
    fn tela_impossivel_devolve_tudo_vazio() {
        for (largura, altura) in [(0.0, 0.0), (-1.0, 10.0), (10.0, -1.0), (f32::NAN, 10.0), (10.0, f32::NAN), (f32::INFINITY, 10.0)] {
            for zen in [true, false] {
                let som = tela_som(largura, altura, zen);
                assert_eq!(som, TelaSom::VAZIA, "{largura}x{altura}");
                assert_eq!(som.folha, Rect::VAZIO);
                assert_eq!(som.respiracao, None);
                assert_eq!(som.texto, 0.0);
                for (nome, retangulo) in controles(&som) {
                    assert_eq!(retangulo, Rect::VAZIO, "{nome}");
                }
                let tela = tela_jogo(largura, altura, zen);
                assert_eq!(tela.botao_som, Rect::VAZIO);
                assert_eq!(tela.botao_menu, Rect::VAZIO);
                assert_eq!(tela.menu, TelaMenu::VAZIA, "{largura}x{altura}");
                assert_eq!(tela.guia_respiracao, Rect::VAZIO);
            }
        }
    }

    #[test]
    fn botao_som_fica_ao_lado_das_abas_e_nao_as_cobre() {
        for (largura, altura) in TELAS {
            for zen in [true, false] {
                let tela = tela_jogo(largura, altura, zen);
                let som = tela.botao_som;
                // Na mesma linha das abas e quadrado.
                assert!((som.y - tela.aba_zen.y).abs() < 1e-3 && (som.h - tela.aba_zen.h).abs() < 1e-3, "{largura}x{altura}");
                assert!((som.w - som.h).abs() < 1e-3, "{largura}x{altura}: o botão ♪ é quadrado");
                // À direita das duas abas, sem cobri-las, e no fim da largura do tabuleiro.
                assert!(som.x >= tela.aba_classico.x + tela.aba_classico.w, "{largura}x{altura}: ♪ cobre a aba do Clássico");
                assert!(!se_cruzam(som, tela.aba_zen) && !se_cruzam(som, tela.aba_classico));
                assert!(som.x + som.w <= tela.botao_menu.x + 1e-3, "{largura}x{altura}: o ♪ fica antes do ≡");
                // As abas continuam do mesmo tamanho uma da outra, com largura de verdade.
                assert!((tela.aba_zen.w - tela.aba_classico.w).abs() < 1e-3 && tela.aba_zen.w > 0.0);
            }
        }
    }

    #[test]
    fn a_linha_da_respiracao_nao_encolhe_o_tabuleiro() {
        for (largura, altura) in CELULARES {
            let zen = tela_jogo(largura, altura, true);
            let classico = tela_jogo(largura, altura, false);
            assert_eq!(zen.tabuleiro, classico.tabuleiro, "{largura}x{altura}: o tabuleiro não muda de um modo para o outro");
            // E continua do tamanho máximo (94% da largura): a linha nova coube na sobra de altura.
            assert!((zen.tabuleiro.w - 0.94 * largura).abs() < 1e-2, "{largura}x{altura}: tabuleiro encolheu: {}", zen.tabuleiro.w);
        }
    }

    #[test]
    fn guia_respiracao_so_existe_no_zen() {
        for (largura, altura) in TELAS {
            let classico = tela_jogo(largura, altura, false);
            assert_eq!(classico.guia_respiracao, Rect::VAZIO, "{largura}x{altura}: no Clássico não há guia");
            let zen = tela_jogo(largura, altura, true);
            let guia = zen.guia_respiracao;
            let janela = Rect { x: 0.0, y: 0.0, w: largura, h: altura };
            assert!(guia.w > 0.0 && guia.h > 0.0, "{largura}x{altura}");
            assert!(dentro(guia, janela), "{largura}x{altura}: guia fora da tela");
            // Logo abaixo dos botões, alinhada com o tabuleiro e sem cruzar nada.
            assert!(guia.y >= zen.botao_dica.y + zen.botao_dica.h, "{largura}x{altura}: guia acima dos botões");
            assert!((guia.x - zen.tabuleiro.x).abs() < 1e-2 && (guia.w - zen.tabuleiro.w).abs() < 1e-2);
            for (nome, retangulo) in pecas(&zen) {
                assert!(!se_cruzam(guia, retangulo), "guia cruza {nome} em {largura}x{altura}");
            }
        }
    }

    // ---- Linha "Vibração" da folha de Som (M4) ----

    #[test]
    fn folha_com_vibracao_cabe_e_nao_se_sobrepoe() {
        for (largura, altura) in CELULARES.into_iter().chain(TELAS) {
            for zen in [true, false] {
                let som = tela_jogo_com(largura, altura, zen, true).som;
                let janela = Rect { x: 0.0, y: 0.0, w: largura, h: altura };
                let vibracao = som.vibracao.unwrap_or_else(|| panic!("sem linha de vibração em {largura}x{altura} zen={zen}"));
                assert!(vibracao.w > 0.0 && vibracao.h > 0.0, "{largura}x{altura} zen={zen}");
                assert!(dentro(vibracao, som.folha) && dentro(vibracao, janela), "{largura}x{altura} zen={zen}: fora da folha");
                // Logo depois do volume dos efeitos, e antes de tudo que vem da Música para baixo.
                assert!(vibracao.y >= som.barra_efeitos.y + som.barra_efeitos.h - 1e-3, "{largura}x{altura}: acima do volume dos efeitos");
                let antes = ["titulo", "fechar", "sons_do_jogo", "testar", "barra_efeitos", "vibracao"];
                for (nome, outro) in controles(&som) {
                    if !antes.contains(&nome) {
                        assert!(outro.y >= vibracao.y + vibracao.h - 1e-3, "{nome} devia ficar abaixo da vibração em {largura}x{altura} zen={zen}");
                    }
                }
                let todos = controles(&som);
                for (i, (nome_a, a)) in todos.iter().enumerate() {
                    for (nome_b, b) in &todos[i + 1..] {
                        assert!(!se_cruzam(*a, *b), "{nome_a} e {nome_b} se cruzam em {largura}x{altura} zen={zen}");
                    }
                }
            }
        }
        // Nos celulares de verdade, a linha tem altura para o dedo (e o resto da folha também).
        for (largura, altura) in CELULARES {
            for zen in [true, false] {
                let som = tela_jogo_com(largura, altura, zen, true).som;
                let unidade = largura.min(altura);
                for (nome, retangulo) in controles(&som) {
                    if nome != "titulo" {
                        assert!(retangulo.h >= 0.07 * unidade, "{nome} baixo demais em {largura}x{altura} zen={zen}: {}", retangulo.h);
                    }
                }
            }
        }
    }

    #[test]
    fn sem_vibrador_a_linha_some_e_o_resto_continua_cabendo() {
        for (largura, altura) in CELULARES.into_iter().chain(TELAS) {
            for zen in [true, false] {
                let som = tela_jogo_com(largura, altura, zen, false).som;
                assert_eq!(som.vibracao, None, "{largura}x{altura} zen={zen}");
                let janela = Rect { x: 0.0, y: 0.0, w: largura, h: altura };
                let todos = controles(&som);
                for (nome, retangulo) in &todos {
                    assert!(retangulo.w > 0.0 && retangulo.h > 0.0, "{nome} sem tamanho em {largura}x{altura} zen={zen}");
                    assert!(dentro(*retangulo, som.folha) && dentro(*retangulo, janela), "{nome} fora em {largura}x{altura} zen={zen}");
                }
                for (i, (nome_a, a)) in todos.iter().enumerate() {
                    for (nome_b, b) in &todos[i + 1..] {
                        assert!(!se_cruzam(*a, *b), "{nome_a} e {nome_b} se cruzam em {largura}x{altura} zen={zen}");
                    }
                }
            }
        }
    }

    #[test]
    fn o_tabuleiro_nao_muda_com_a_linha_nova() {
        for (largura, altura) in CELULARES.into_iter().chain(TELAS) {
            for zen in [true, false] {
                let com = tela_jogo_com(largura, altura, zen, true);
                let sem = tela_jogo_com(largura, altura, zen, false);
                assert_eq!(com.tabuleiro, sem.tabuleiro, "{largura}x{altura} zen={zen}");
                assert_eq!(com.som.folha, sem.som.folha, "{largura}x{altura} zen={zen}");
                assert_eq!((com.botao_dica, com.botao_novo_jogo, com.botao_som), (sem.botao_dica, sem.botao_novo_jogo, sem.botao_som));
            }
        }
    }

    #[test]
    fn tela_jogo_e_o_atalho_com_vibrador() {
        for (largura, altura) in CELULARES.into_iter().chain(TELAS) {
            for zen in [true, false] {
                assert_eq!(tela_jogo(largura, altura, zen), tela_jogo_com(largura, altura, zen, true));
            }
        }
        // Tela impossível continua vazia, com ou sem vibrador.
        assert_eq!(tela_jogo_com(f32::NAN, 10.0, true, true).som, TelaSom::VAZIA);
    }

    // ---- Botão ≡ e a folha do Menu (M4) ----

    #[test]
    fn botao_menu_fica_ao_lado_do_som_sem_cobrir_as_abas() {
        for (largura, altura) in CELULARES.into_iter().chain(TELAS) {
            for zen in [true, false] {
                let tela = tela_jogo(largura, altura, zen);
                let (menu, som) = (tela.botao_menu, tela.botao_som);
                assert!(menu.w > 0.0 && menu.h > 0.0, "{largura}x{altura}");
                // Quadrado, na linha das abas e do mesmo tamanho do ♪.
                assert!((menu.w - menu.h).abs() < 1e-3, "{largura}x{altura}: o ≡ é quadrado");
                assert!((menu.y - tela.aba_zen.y).abs() < 1e-3 && (menu.h - tela.aba_zen.h).abs() < 1e-3, "{largura}x{altura}");
                assert!((menu.w - som.w).abs() < 1e-3 && (menu.y - som.y).abs() < 1e-3, "{largura}x{altura}");
                // À direita do ♪, no fim da largura do tabuleiro, sem cruzar nada.
                assert!(menu.x >= som.x + som.w, "{largura}x{altura}: o ≡ vem depois do ♪");
                assert!((menu.x + menu.w - (tela.tabuleiro.x + tela.tabuleiro.w)).abs() < 1e-2, "{largura}x{altura}");
                for (nome, outro) in [("aba_zen", tela.aba_zen), ("aba_classico", tela.aba_classico), ("botao_som", som)] {
                    assert!(!se_cruzam(menu, outro), "o ≡ cruza {nome} em {largura}x{altura}");
                }
                assert!(!se_cruzam(som, tela.aba_zen) && !se_cruzam(som, tela.aba_classico), "{largura}x{altura}");
                // As abas ficam do mesmo tamanho uma da outra, com largura de verdade.
                assert!((tela.aba_zen.w - tela.aba_classico.w).abs() < 1e-3 && tela.aba_zen.w > 0.0, "{largura}x{altura}");
                // Nada da linha de cima passa do tabuleiro: abas, ♪ e ≡ somam a largura dele.
                assert!(tela.aba_classico.x + tela.aba_classico.w <= som.x + 1e-3, "{largura}x{altura}");
            }
        }
    }

    #[test]
    fn tela_menu_cabe_e_nao_se_sobrepoe() {
        for (largura, altura) in CELULARES {
            for zen in [true, false] {
                let tela = tela_jogo(largura, altura, zen);
                let menu = tela.menu;
                let unidade = largura.min(altura);
                let janela = Rect { x: 0.0, y: 0.0, w: largura, h: altura };
                assert_eq!(menu.folha, tela.som.folha, "a folha do menu é a mesma da de Som");
                assert!(dentro(menu.folha, janela));
                let todos = [("aba_recordes", menu.aba_recordes), ("aba_creditos", menu.aba_creditos), ("fechar", menu.fechar), ("conteudo", menu.conteudo)];
                for (nome, retangulo) in todos {
                    assert!(retangulo.w > 0.0 && retangulo.h > 0.0, "{nome} sem tamanho em {largura}x{altura} zen={zen}");
                    assert!(dentro(retangulo, menu.folha), "{nome} fora da folha em {largura}x{altura} zen={zen}");
                }
                for (nome, retangulo) in &todos[..3] {
                    assert!(retangulo.h >= 0.07 * unidade, "{nome} baixo demais em {largura}x{altura}: {} < {}", retangulo.h, 0.07 * unidade);
                }
                for (i, (nome_a, a)) in todos.iter().enumerate() {
                    for (nome_b, b) in &todos[i + 1..] {
                        assert!(!se_cruzam(*a, *b), "{nome_a} e {nome_b} se cruzam em {largura}x{altura} zen={zen}");
                    }
                }
                // As abas ficam no alto da folha e o ✕ no canto direito; o conteúdo, abaixo de tudo.
                assert!(menu.aba_recordes.x < menu.aba_creditos.x && menu.aba_creditos.x < menu.fechar.x);
                assert!(menu.conteudo.y >= menu.fechar.y + menu.fechar.h && menu.conteudo.y >= menu.aba_recordes.y + menu.aba_recordes.h);
                // Sobra espaço de verdade para as linhas de recordes e os créditos.
                assert!(menu.conteudo.h >= 0.4 * unidade, "conteúdo baixo em {largura}x{altura}: {}", menu.conteudo.h);
                assert!(menu.texto > 0.0);
            }
        }
    }

    #[test]
    fn os_dois_blocos_de_recordes_cabem_na_folha_em_qualquer_celular() {
        for (largura, altura) in CELULARES {
            for zen in [true, false] {
                let menu = tela_jogo(largura, altura, zen).menu;
                let blocos = repartir_recordes(menu.conteudo, menu.texto, &[3, 4]);
                assert_eq!(blocos.len(), 2, "{largura}x{altura}");
                assert_eq!((blocos[0].cartoes.len(), blocos[1].cartoes.len()), (3, 4));
                // Tudo dentro do conteúdo, em ordem de cima para baixo e sem cruzar nada.
                let mut todos = Vec::new();
                for bloco in &blocos {
                    todos.push(bloco.legenda);
                    todos.extend(bloco.cartoes.iter().copied());
                }
                for retangulo in &todos {
                    assert!(retangulo.w > 0.0 && retangulo.h > 0.0, "retângulo sem tamanho em {largura}x{altura}");
                    assert!(dentro(*retangulo, menu.conteudo), "retângulo fora do conteúdo em {largura}x{altura}: {retangulo:?} / {:?}", menu.conteudo);
                }
                for (i, a) in todos.iter().enumerate() {
                    for b in &todos[i + 1..] {
                        assert!(!se_cruzam(*a, *b), "retângulos se cruzam em {largura}x{altura}: {a:?} e {b:?}");
                        assert!(a.y < b.y, "fora de ordem em {largura}x{altura}");
                    }
                }
                // Os cartões têm de ser grandes o bastante para a letra (e nunca passam de 3 letras).
                for retangulo in blocos.iter().flat_map(|bloco| bloco.cartoes.iter()) {
                    assert!(retangulo.h >= 1.5 * menu.texto, "cartão baixo em {largura}x{altura}: {} < {}", retangulo.h, 1.5 * menu.texto);
                    assert!(retangulo.h <= 3.0 * menu.texto + 1e-3);
                }
            }
        }
    }

    #[test]
    fn repartir_recordes_nao_da_pane_com_area_vazia_ou_sem_linhas() {
        for conteudo in [Rect::VAZIO, Rect { x: 5.0, y: 5.0, w: 100.0, h: 0.0 }, Rect { x: 5.0, y: 5.0, w: 100.0, h: 10.0 }] {
            let blocos = repartir_recordes(conteudo, 20.0, &[3, 4]);
            assert_eq!(blocos.len(), 2);
            for retangulo in blocos.iter().flat_map(|bloco| std::iter::once(&bloco.legenda).chain(bloco.cartoes.iter())) {
                assert!(retangulo.h.is_finite() && retangulo.h >= 0.0, "altura {} em {conteudo:?}", retangulo.h);
                assert!(retangulo.y.is_finite() && retangulo.w.is_finite());
            }
        }
        assert_eq!(repartir_recordes(Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }, 10.0, &[]).len(), 0);
        let sem_linhas = repartir_recordes(Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }, 10.0, &[0, 0]);
        assert_eq!(sem_linhas.len(), 2);
        assert!(sem_linhas.iter().all(|bloco| bloco.cartoes.is_empty()));
    }

    #[test]
    fn tela_menu_existe_e_cabe_em_qualquer_tela_possivel() {
        for (largura, altura) in TELAS {
            for zen in [true, false] {
                let menu = tela_jogo(largura, altura, zen).menu;
                let janela = Rect { x: 0.0, y: 0.0, w: largura, h: altura };
                let todos = [("aba_recordes", menu.aba_recordes), ("aba_creditos", menu.aba_creditos), ("fechar", menu.fechar), ("conteudo", menu.conteudo)];
                for (nome, retangulo) in todos {
                    assert!(retangulo.w > 0.0 && retangulo.h > 0.0, "{nome} sem tamanho em {largura}x{altura}");
                    assert!(dentro(retangulo, janela) && dentro(retangulo, menu.folha), "{nome} fora em {largura}x{altura}");
                }
                for (i, (nome_a, a)) in todos.iter().enumerate() {
                    for (nome_b, b) in &todos[i + 1..] {
                        assert!(!se_cruzam(*a, *b), "{nome_a} e {nome_b} se cruzam em {largura}x{altura}");
                    }
                }
            }
        }
    }

    #[test]
    fn o_tabuleiro_nao_muda_com_o_botao_menu() {
        // Posições do M3 (antes do botão ≡), calculadas à mão: o tabuleiro não se mexe.
        for ((largura, altura), (x, y, lado)) in [((540.0, 960.0), (16.2, 263.19, 507.6)), ((720.0, 1280.0), (21.6, 350.92, 676.8))] {
            let tabuleiro = tela_jogo(largura, altura, true).tabuleiro;
            for (obtido, esperado, nome) in [(tabuleiro.x, x, "x"), (tabuleiro.y, y, "y"), (tabuleiro.w, lado, "lado")] {
                assert!((obtido - esperado).abs() < 0.01, "{largura}x{altura}: {nome} = {obtido}, esperado {esperado}");
            }
        }
        for (largura, altura) in CELULARES {
            let zen = tela_jogo(largura, altura, true);
            let classico = tela_jogo(largura, altura, false);
            assert_eq!(zen.tabuleiro, classico.tabuleiro, "{largura}x{altura}");
            assert!((zen.tabuleiro.w - 0.94 * largura).abs() < 1e-2, "{largura}x{altura}: o tabuleiro continua no tamanho máximo");
        }
    }
}
