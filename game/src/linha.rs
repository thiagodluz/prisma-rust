//! Linha do tempo das animações.
//!
//! O motor resolve uma jogada inteira de uma vez e devolve uma lista de "eventos" (limpou,
//! caiu, embaralhou...). A tela precisa mostrar isso passo a passo. Este módulo:
//! 1. transforma os eventos numa sequência de [`Passo`]s, cada um com a sua duração;
//! 2. sabe dizer "como está cada pedra X milissegundos depois do começo do passo";
//! 3. tem o [`Reprodutor`], que anda pelos passos conforme o tempo passa.
//!
//! É um módulo puro (só números): não desenha nada, por isso dá para testar sem janela.
//! As durações são as do `app.js` do Prisma em JS (efeitos em escala 1). As curvas de
//! suavização são nossas (a spec deixa isso para quem constrói).

use crate::preferencias::Intensidade;
use prisma_engine::{position, Activation, Board, Event, MoveResult, TileKind, SIZE};

// ---- Durações, em milissegundos (as do JS) -----------------------------------------------

const TROCA_VALIDA_MS: f32 = 225.0;
const TROCA_INVALIDA_MS: f32 = 300.0;
/// Na troca inválida as pedras vão até a casa vizinha em 48% do tempo e depois voltam.
const PICO_DA_TROCA_INVALIDA: f32 = 0.48;
/// Quanto a PEDRA leva para sumir (o passo todo dura mais: ver `duracao_ms`).
const LIMPEZA_MS: f32 = 270.0;
/// Pedra que some numa limpeza em que uma pedra especial foi ativada (dura um pouco mais).
const LIMPEZA_ESPECIAL_MS: f32 = 340.0;
/// As animações dos efeitos, em escala 1 (as do JS). O passo de limpeza dura até a mais longa.
pub(crate) const FAISCA_MS: f32 = 380.0;
pub(crate) const ONDA_MS: f32 = 360.0;
pub(crate) const FEIXE_MS: f32 = 330.0;
pub(crate) const VEU_MS: f32 = 370.0;
pub(crate) const SUPERNOVA_MS: f32 = 480.0;
/// A fusão das pedras de origem de uma Supernova, antes da explosão.
pub(crate) const FUSAO_MS: f32 = 240.0;
/// O maior "filtro de brilho" do JS (o do Vibrante). O brilho que sai daqui vai de 0 (sem brilho,
/// filtro 1) a 1 (filtro 2,1): `(filtro − 1) / (2,1 − 1)`.
const BRILHO_MAXIMO: f32 = 2.1;
/// Quando há especial ativada, cada pedra espera 22 ms por casa de distância dela, até 100 ms.
const ATRASO_POR_CASA_MS: f32 = 22.0;
const ATRASO_MAXIMO_MS: f32 = 100.0;
/// Uma pedra que cai leva 250 ms mais 32 ms por linha, até 470 ms.
const QUEDA_BASE_MS: f32 = 250.0;
const QUEDA_POR_LINHA_MS: f32 = 32.0;
const QUEDA_MAXIMA_MS: f32 = 470.0;
const EMBARALHAR_SAI_MS: f32 = 190.0;
const EMBARALHAR_ENTRA_MS: f32 = 250.0;
const RESGATE_MS: f32 = 300.0;
const NIVEL_MS: f32 = 580.0;

// ---- Os passos ---------------------------------------------------------------------------

/// Que efeito uma ativação faz na tela.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoVisual {
    /// Pulso: uma onda em volta da pedra.
    Pulso,
    /// Raio: dois feixes, na linha e na coluna.
    Raio,
    /// Espectro: um véu sobre o tabuleiro todo.
    Espectro,
    /// Supernova: a explosão grande (antes dela, as pedras de origem se fundem).
    Supernova,
}

/// Uma ativação (pedra especial ou Supernova) que o passo de limpeza mostra com efeitos.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Visual {
    /// A casa de onde o efeito sai.
    pub casa: usize,
    pub tipo: TipoVisual,
    /// A cor da Supernova (0 nos outros tipos).
    pub cor: u8,
    /// As pedras fundidas numa Supernova (vazio nos outros tipos).
    pub origens: Vec<usize>,
}

impl Intensidade {
    /// Quanto as animações esticam (ou encolhem) nesta intensidade: Suave 0,82, Padrão 1, Vibrante 1,1.
    /// (Uma função solta de `linha.rs`: só quem mexe com tempo precisa dela.)
    pub(crate) fn escala_de_duracao(self) -> f32 {
        match self {
            Intensidade::Suave => 0.82,
            Intensidade::Padrao => 1.0,
            Intensidade::Vibrante => 1.1,
        }
    }

    /// O tamanho máximo que a pedra chega a ter ao sumir (1 é o tamanho normal).
    pub(crate) fn pico_da_pedra(self) -> f32 {
        match self {
            Intensidade::Suave | Intensidade::Padrao => 1.1,
            Intensidade::Vibrante => 1.18,
        }
    }

    /// O filtro de brilho que a pedra chega a ter ao sumir (1 é sem brilho), como no JS.
    pub(crate) fn brilho_da_pedra(self) -> f32 {
        match self {
            Intensidade::Suave => 1.35,
            Intensidade::Padrao => 1.8,
            Intensidade::Vibrante => 2.1,
        }
    }
}

/// Um passo da animação de uma jogada.
///
/// Um `enum` é "uma coisa que é UMA destas opções"; cada opção aqui carrega os dados que a
/// tela precisa para desenhar aquele passo.
#[derive(Clone, Debug, PartialEq)]
pub enum Passo {
    /// As pedras das casas `a` e `b` trocam de lugar. `antes` é o tabuleiro de antes da troca.
    /// Se `valida` for falso, elas vão e voltam (a troca não formou combinação).
    Troca { antes: Board, a: usize, b: usize, valida: bool },
    /// Pedras somem. `tabuleiro` é o tabuleiro ANTES de limpar; `celulas` são as que somem;
    /// `ativadas` são as casas de pedras especiais (e Supernovas) que dispararam. `cadeia` é o
    /// número da limpeza na jogada (1 = a primeira, 2 = a primeira cascata...) e `criou` diz se
    /// ela fez nascer uma pedra especial. Os dois servem para escolher o efeito sonoro.
    /// `visuais` são todas as ativações, na ordem do motor, com o que a tela desenha de cada uma, e
    /// `intensidade` é a escolha do jogador para os efeitos (só mexe no visual e no tempo).
    Limpeza {
        tabuleiro: Board,
        celulas: Vec<usize>,
        ativadas: Vec<usize>,
        supernova: bool,
        ganho: u64,
        cadeia: u32,
        criou: bool,
        visuais: Vec<Visual>,
        intensidade: Intensidade,
    },
    /// As pedras caem. `tabuleiro` é o tabuleiro DEPOIS da queda; `origens[destino]` é a casa
    /// de onde a pedra veio (`linha * 8 + coluna`); número negativo = pedra nova, que nasceu
    /// acima do tabuleiro.
    Queda { tabuleiro: Board, origens: Vec<i32> },
    /// O tabuleiro foi reorganizado: some o `antes` e aparece o `depois`.
    Embaralhar { antes: Board, depois: Board },
    /// O jogo (Zen) criou um Espectro na casa `indice` para destravar o tabuleiro.
    Resgate { tabuleiro: Board, indice: usize },
    /// A faixa "Nível N" depois de subir de nível.
    Nivel { nivel: u32 },
}

/// A casa de uma ativação (pedra especial ou Supernova).
fn casa_da_ativacao(ativacao: &Activation) -> usize {
    match ativacao {
        Activation::Special { index, .. } => *index,
        Activation::Supernova(nova) => nova.index,
    }
}

/// O efeito visual de uma ativação. Uma pedra "especial" comum não faz efeito (não acontece).
fn visual_da_ativacao(ativacao: &Activation) -> Option<Visual> {
    match ativacao {
        Activation::Special { index, kind, .. } => {
            let tipo = match kind {
                TileKind::Burst => TipoVisual::Pulso,
                TileKind::Cross => TipoVisual::Raio,
                TileKind::Spectrum => TipoVisual::Espectro,
                TileKind::Normal => return None,
            };
            Some(Visual { casa: *index, tipo, cor: 0, origens: Vec::new() })
        }
        Activation::Supernova(nova) => {
            Some(Visual { casa: nova.index, tipo: TipoVisual::Supernova, cor: nova.color, origens: nova.sources.clone() })
        }
    }
}

/// Monta os passos de uma jogada.
///
/// `antes` é o tabuleiro de antes de chamar `Game::play(a, b)`, `resultado` é o que o motor
/// devolveu e `nivel_final` é o nível do jogo depois da jogada. Jogada inválida dá só o passo
/// da troca que vai e volta. O fim de partida não vira passo: o cartão de "Fim de partida"
/// aparece depois da animação, como no JS. `intensidade` vai para cada passo de limpeza.
pub fn montar(
    antes: Board,
    a: usize,
    b: usize,
    resultado: &MoveResult,
    nivel_final: u32,
    intensidade: Intensidade,
) -> Vec<Passo> {
    let mut passos = vec![Passo::Troca { antes, a, b, valida: resultado.valid }];
    if !resultado.valid {
        return passos;
    }
    for evento in &resultado.events {
        match evento {
            Event::Clear { board, cells, activated, creations, chain, earned, .. } => passos.push(Passo::Limpeza {
                tabuleiro: *board,
                celulas: cells.clone(),
                ativadas: activated.iter().map(casa_da_ativacao).collect(),
                supernova: activated.iter().any(|ativacao| matches!(ativacao, Activation::Supernova(_))),
                ganho: *earned,
                cadeia: *chain,
                criou: !creations.is_empty(),
                visuais: activated.iter().filter_map(visual_da_ativacao).collect(),
                intensidade,
            }),
            Event::Fall { board, falls, .. } => {
                passos.push(Passo::Queda { tabuleiro: *board, origens: falls.clone() });
            }
            Event::Shuffle { before, board, .. } => {
                passos.push(Passo::Embaralhar { antes: *before, depois: *board });
            }
            Event::Rescue { board, index, .. } => {
                passos.push(Passo::Resgate { tabuleiro: *board, indice: *index });
            }
            Event::GameOver { .. } => {}
        }
    }
    if resultado.levels_gained > 0 {
        passos.push(Passo::Nivel { nivel: nivel_final });
    }
    passos
}

/// Quanto tempo (em milissegundos) dura um passo.
pub fn duracao_ms(passo: &Passo) -> f32 {
    match passo {
        Passo::Troca { valida: true, .. } => TROCA_VALIDA_MS,
        Passo::Troca { valida: false, .. } => TROCA_INVALIDA_MS,
        // Primeiro a fusão (só na Supernova) e depois o que durar mais: as pedras ou os efeitos
        // (como o `waitAnimations` do JS, que espera a mais longa das animações).
        Passo::Limpeza { celulas, ativadas, visuais, intensidade, .. } => {
            let escala = intensidade.escala_de_duracao();
            // A última pedra a começar (a mais longe da especial) define quando as pedras acabam.
            let maior_atraso = celulas.iter().map(|&casa| atraso_da_casa(ativadas, casa)).fold(0.0, f32::max);
            let pedras = duracao_da_pedra(ativadas, *intensidade) + maior_atraso;
            let efeitos = escala * visuais.iter().map(|visual| duracao_do_efeito(visual.tipo)).fold(FAISCA_MS, f32::max);
            fusao_ms(passo) + pedras.max(efeitos)
        }
        // A queda dura o tempo da pedra que mais demora; pedra que não se mexe não conta.
        Passo::Queda { origens, .. } => origens
            .iter()
            .enumerate()
            .map(|(destino, &origem)| linhas_de_queda(origem, destino / SIZE))
            .filter(|&linhas| linhas > 0)
            .map(queda_ms)
            .fold(0.0, f32::max),
        Passo::Embaralhar { .. } => EMBARALHAR_SAI_MS + EMBARALHAR_ENTRA_MS,
        Passo::Resgate { .. } => RESGATE_MS,
        Passo::Nivel { .. } => NIVEL_MS,
    }
}

/// Quanto uma pedra leva para sumir: com especial ativada demora mais. Escalada pela intensidade.
fn duracao_da_pedra(ativadas: &[usize], intensidade: Intensidade) -> f32 {
    let base = if ativadas.is_empty() { LIMPEZA_MS } else { LIMPEZA_ESPECIAL_MS };
    base * intensidade.escala_de_duracao()
}

/// Quanto dura a animação de um efeito, em escala 1.
fn duracao_do_efeito(tipo: TipoVisual) -> f32 {
    match tipo {
        TipoVisual::Pulso => ONDA_MS,
        TipoVisual::Raio => FEIXE_MS,
        TipoVisual::Espectro => VEU_MS,
        TipoVisual::Supernova => SUPERNOVA_MS,
    }
}

/// Quanto tempo a fusão das pedras de origem de uma Supernova leva, no começo do passo de
/// limpeza (0 se o passo não tem Supernova com pedras fundidas).
pub fn fusao_ms(passo: &Passo) -> f32 {
    match passo {
        Passo::Limpeza { visuais, intensidade, .. } => fusao_dos_visuais(visuais, *intensidade),
        _ => 0.0,
    }
}

/// O mesmo de [`fusao_ms`], para quem só tem os `visuais` e a intensidade do passo (as partículas).
pub fn fusao_dos_visuais<'a>(visuais: impl IntoIterator<Item = &'a Visual>, intensidade: Intensidade) -> f32 {
    if visuais.into_iter().any(|visual| visual.tipo == TipoVisual::Supernova && !visual.origens.is_empty()) {
        FUSAO_MS * intensidade.escala_de_duracao()
    } else {
        0.0
    }
}

// ---- Contas de tempo e suavização ----------------------------------------------------------

/// Que parte do passo já passou: 0 no começo, 1 no fim. Tempo inválido (NaN) ou negativo
/// vale 0; passou do fim (ou é infinito) vale 1. Assim o resultado é sempre um número de 0 a 1.
fn progresso(t_ms: f32, duracao_ms: f32) -> f32 {
    if t_ms.is_nan() || t_ms <= 0.0 {
        0.0
    } else if t_ms >= duracao_ms {
        1.0
    } else {
        t_ms / duracao_ms
    }
}

/// Começa e termina devagar (usada na troca).
fn suave(p: f32) -> f32 {
    p * p * (3.0 - 2.0 * p)
}

/// Começa rápido e termina devagar (usada em queda, limpeza e entradas).
fn desacelera(p: f32) -> f32 {
    1.0 - (1.0 - p).powi(3)
}

/// Um número entre `a` (quando `p` é 0) e `b` (quando `p` é 1).
fn entre(a: f32, b: f32, p: f32) -> f32 {
    a + (b - a) * p
}

// ---- Troca ---------------------------------------------------------------------------------

/// Quanto do caminho até a casa vizinha cada pedra da troca já andou (de 0 a 1).
/// Troca inválida vai até 1 (em 48% do tempo) e volta a 0.
pub fn deslocamento_troca(t_ms: f32, valida: bool) -> f32 {
    if valida {
        suave(progresso(t_ms, TROCA_VALIDA_MS))
    } else {
        let p = progresso(t_ms, TROCA_INVALIDA_MS);
        if p < PICO_DA_TROCA_INVALIDA {
            suave(p / PICO_DA_TROCA_INVALIDA)
        } else {
            suave(1.0 - (p - PICO_DA_TROCA_INVALIDA) / (1.0 - PICO_DA_TROCA_INVALIDA))
        }
    }
}

// ---- Limpeza -------------------------------------------------------------------------------

/// Quanto a pedra da `casa` espera antes de começar a sumir, por causa das pedras especiais.
fn atraso_da_casa(ativadas: &[usize], casa: usize) -> f32 {
    let (linha, coluna) = position(casa);
    ativadas
        .iter()
        .map(|&ativada| {
            let (linha_a, coluna_a) = position(ativada);
            linha.abs_diff(linha_a) + coluna.abs_diff(coluna_a)
        })
        .min()
        // `map_or(0.0, ...)`: se não há nenhuma ativada, o atraso é 0.
        .map_or(0.0, |distancia| (distancia as f32 * ATRASO_POR_CASA_MS).min(ATRASO_MAXIMO_MS))
}

/// O brilho (de 0 a 1) de um filtro de brilho do JS (1 é sem brilho; 2,1 é o máximo).
fn brilho_do_filtro(filtro: f32) -> f32 {
    ((filtro - 1.0) / (BRILHO_MAXIMO - 1.0)).clamp(0.0, 1.0)
}

/// Como está a pedra da `casa` que vai sumir, `t_ms` depois do fim da fusão (ou do começo da
/// limpeza, se não há fusão): devolve (escala, opacidade, brilho de 0 a 1).
/// Ela cresce até o pico (1,1; 1,18 no Vibrante) e brilha, depois encolhe e apaga (até 0,72 e
/// opacidade 0); o brilho vai do pico do filtro de cada intensidade até o filtro 1,5 do JS.
pub fn limpeza_da_casa(ativadas: &[usize], casa: usize, t_ms: f32, intensidade: Intensidade) -> (f32, f32, f32) {
    let atraso = if ativadas.is_empty() { 0.0 } else { atraso_da_casa(ativadas, casa) };
    let tempo = if t_ms.is_nan() { 0.0 } else { t_ms - atraso };
    let p = desacelera(progresso(tempo, duracao_da_pedra(ativadas, intensidade)));
    let (pico, filtro_do_pico) = (intensidade.pico_da_pedra(), intensidade.brilho_da_pedra());
    if p < 0.38 {
        let q = p / 0.38;
        (entre(1.0, pico, q), 1.0, brilho_do_filtro(entre(1.0, filtro_do_pico, q)))
    } else {
        let q = (p - 0.38) / 0.62;
        (entre(pico, 0.72, q), entre(1.0, 0.0, q), brilho_do_filtro(entre(filtro_do_pico, 1.5, q)))
    }
}

/// Como está cada pedra de origem de uma Supernova, `t_ms` depois do começo da fusão: devolve
/// (escala, brilho de 0 a 1). Cresce a 1,16 (filtro 1,8) em 60% do tempo e depois encolhe a 0,9
/// (filtro 2). A fusão dura `FUSAO_MS` vezes a escala da intensidade.
pub fn fusao_da_casa(t_ms: f32, intensidade: Intensidade) -> (f32, f32) {
    let p = progresso(t_ms, FUSAO_MS * intensidade.escala_de_duracao());
    if p < 0.6 {
        let q = p / 0.6;
        (entre(1.0, 1.16, q), brilho_do_filtro(entre(1.0, 1.8, q)))
    } else {
        let q = (p - 0.6) / 0.4;
        (entre(1.16, 0.9, q), brilho_do_filtro(entre(1.8, 2.0, q)))
    }
}

// ---- Queda ---------------------------------------------------------------------------------

/// Quantas linhas uma pedra desce: da linha de onde veio até a `linha_destino`.
///
/// `origem` é `linha * 8 + coluna`. Pedra nova nasce acima do tabuleiro: a origem é negativa
/// (linha −1, −2...). Cuidado: em Rust `-5 / 8` dá 0 (arredonda para o zero), mas a linha de
/// `-5` é −1. Por isso usamos `div_euclid`, que arredonda para baixo.
pub fn linhas_de_queda(origem: i32, linha_destino: usize) -> i32 {
    let linha_da_origem = origem.div_euclid(SIZE as i32);
    (linha_destino as i32 - linha_da_origem).max(0)
}

/// Quanto tempo uma pedra leva para descer `linhas` linhas.
pub fn queda_ms(linhas: i32) -> f32 {
    (QUEDA_BASE_MS + QUEDA_POR_LINHA_MS * linhas.max(0) as f32).min(QUEDA_MAXIMA_MS)
}

/// Quantas casas acima do lugar final a pedra está, `t_ms` depois do começo da queda.
/// Começa em "o quanto ela vai descer" e chega a 0.
pub fn queda_deslocamento(origem: i32, linha_destino: usize, t_ms: f32) -> f32 {
    let linhas = linhas_de_queda(origem, linha_destino);
    if linhas == 0 {
        return 0.0;
    }
    let p = progresso(t_ms, queda_ms(linhas));
    linhas as f32 * (1.0 - desacelera(p))
}

// ---- Embaralhar, resgate e nível -----------------------------------------------------------

/// Embaralhar: primeiro o tabuleiro de antes apaga (opacidade 1 → 0,12 em 190 ms) e depois o
/// novo aparece (0,12 → 1 em 250 ms). Devolve (mostra o tabuleiro novo?, opacidade).
pub fn embaralhar_alfa(t_ms: f32) -> (bool, f32) {
    let saindo = progresso(t_ms, EMBARALHAR_SAI_MS);
    if saindo < 1.0 {
        (false, entre(1.0, 0.12, saindo * saindo))
    } else {
        let entrando = progresso(t_ms - EMBARALHAR_SAI_MS, EMBARALHAR_ENTRA_MS);
        (true, entre(0.12, 1.0, desacelera(entrando)))
    }
}

/// A pedra criada pelo resgate: nasce pequena e transparente, passa um pouco do tamanho
/// (1,16) e assenta em 1. Devolve (escala, opacidade).
pub fn resgate_da_casa(t_ms: f32) -> (f32, f32) {
    let p = desacelera(progresso(t_ms, RESGATE_MS));
    if p < 0.65 {
        (entre(0.25, 1.16, p / 0.65), p / 0.65)
    } else {
        (entre(1.16, 1.0, (p - 0.65) / 0.35), 1.0)
    }
}

/// A faixa "Nível N": aparece (0,82 → 1), fica, e some crescendo (1 → 1,08).
/// Devolve (escala, opacidade).
pub fn nivel_faixa(t_ms: f32) -> (f32, f32) {
    let p = progresso(t_ms, NIVEL_MS);
    if p < 0.25 {
        let q = p / 0.25;
        (entre(0.82, 1.0, q), q)
    } else if p < 0.7 {
        (1.0, 1.0)
    } else {
        let q = (p - 0.7) / 0.3;
        (entre(1.0, 1.08, q), 1.0 - q)
    }
}

// ---- O reprodutor --------------------------------------------------------------------------

/// Anda pelos passos conforme o tempo passa. A tela pergunta "em que passo estamos e há
/// quanto tempo?" a cada quadro.
#[derive(Clone, Debug)]
pub struct Reprodutor {
    passos: Vec<Passo>,
    /// Posição do passo em andamento (igual ao total quando tudo terminou).
    indice: usize,
    /// Quanto do passo em andamento já passou.
    decorrido_ms: f32,
}

impl Reprodutor {
    /// Começa a reproduzir `passos`. Passos que não duram nada (uma queda em que nenhuma pedra
    /// se mexe) são pulados logo de saída.
    pub fn novo(passos: Vec<Passo>) -> Reprodutor {
        let mut reprodutor = Reprodutor { passos, indice: 0, decorrido_ms: 0.0 };
        reprodutor.avancar(0.0);
        reprodutor
    }

    /// Deixa passar `dt_ms` milissegundos. Tempo inválido (NaN) ou negativo conta como zero.
    /// Tempo enorme (o app voltando do segundo plano) ou infinito pula tudo até o fim.
    pub fn avancar(&mut self, dt_ms: f32) {
        let dt = if dt_ms.is_nan() || dt_ms < 0.0 { 0.0 } else { dt_ms };
        self.decorrido_ms += dt;
        while let Some(passo) = self.passos.get(self.indice) {
            let duracao = duracao_ms(passo);
            if self.decorrido_ms < duracao {
                return;
            }
            self.decorrido_ms -= duracao;
            self.indice += 1;
        }
        // Acabaram os passos: não sobra "tempo guardado".
        self.decorrido_ms = 0.0;
    }

    /// O passo em andamento e quanto tempo (ms) já passou nele. `None` se tudo terminou.
    pub fn atual(&self) -> Option<(&Passo, f32)> {
        self.passos.get(self.indice).map(|passo| (passo, self.decorrido_ms))
    }

    /// Os passos que já terminaram, na ordem.
    pub fn concluidos(&self) -> &[Passo] {
        &self.passos[..self.indice]
    }

    pub fn terminou(&self) -> bool {
        self.indice >= self.passos.len()
    }

    /// Termina tudo de uma vez.
    pub fn pular_para_o_fim(&mut self) {
        self.indice = self.passos.len();
        self.decorrido_ms = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prisma_engine::{lcg_random, Game, Mode, TileKind};

    fn perto(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    /// Um tabuleiro qualquer, só para encher os passos de teste.
    fn tabuleiro() -> Board {
        Game::with_random(lcg_random(1)).board
    }

    /// Uma jogada boa (a dica do motor): devolve o tabuleiro de antes, as casas e o resultado.
    fn jogar(game: &mut Game) -> Option<(Board, usize, usize, MoveResult)> {
        let (a, b) = game.hint()?;
        let antes = game.board;
        let resultado = game.play(a, b);
        Some((antes, a, b, resultado))
    }

    /// Acha, por força bruta, uma troca que o motor recusa (não forma combinação).
    fn jogada_invalida(game: &mut Game) -> (Board, usize, usize, MoveResult) {
        for a in 0..SIZE * SIZE {
            for b in [a + 1, a + SIZE] {
                if b >= SIZE * SIZE {
                    continue;
                }
                let antes = game.board;
                let resultado = game.play(a, b);
                if !resultado.valid && prisma_engine::adjacent(a, b) {
                    return (antes, a, b, resultado);
                }
            }
        }
        panic!("não achei nenhuma troca inválida");
    }

    /// Uma limpeza montada à mão (sem passar pelo motor), só com o que importa para os tempos.
    fn limpeza_a_mao(celulas: Vec<usize>, ativadas: Vec<usize>, visuais: Vec<Visual>, intensidade: Intensidade) -> Passo {
        Passo::Limpeza {
            tabuleiro: tabuleiro(),
            celulas,
            ativadas,
            supernova: visuais.iter().any(|visual| visual.tipo == TipoVisual::Supernova),
            ganho: 0,
            cadeia: 1,
            criou: false,
            visuais,
            intensidade,
        }
    }

    fn visual(casa: usize, tipo: TipoVisual) -> Visual {
        Visual { casa, tipo, cor: 0, origens: vec![] }
    }

    /// Uma Supernova com fusão (as três pedras de origem na linha da `casa`).
    fn supernova(casa: usize) -> Visual {
        Visual { casa, tipo: TipoVisual::Supernova, cor: 1, origens: vec![casa - 1, casa, casa + 1] }
    }

    fn nome(passo: &Passo) -> &'static str {
        match passo {
            Passo::Troca { .. } => "troca",
            Passo::Limpeza { .. } => "clear",
            Passo::Queda { .. } => "fall",
            Passo::Embaralhar { .. } => "shuffle",
            Passo::Resgate { .. } => "rescue",
            Passo::Nivel { .. } => "nivel",
        }
    }

    #[test]
    fn troca_invalida_vira_um_passo_so() {
        let mut game = Game::with_random(lcg_random(3));
        let (antes, a, b, resultado) = jogada_invalida(&mut game);
        let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
        assert_eq!(passos, vec![Passo::Troca { antes, a, b, valida: false }]);
        assert!(perto(duracao_ms(&passos[0]), 300.0));
    }

    #[test]
    fn troca_valida_dura_225() {
        let mut game = Game::with_random(lcg_random(3));
        let (antes, a, b, resultado) = jogar(&mut game).unwrap();
        assert!(resultado.valid);
        let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
        assert_eq!(passos[0], Passo::Troca { antes, a, b, valida: true });
        assert!(perto(duracao_ms(&passos[0]), 225.0));
    }

    #[test]
    fn jogada_valida_segue_a_ordem_dos_eventos() {
        let mut jogadas = 0;
        let mut viu_queda = false;
        for seed in 1..=20 {
            let mut game = Game::with_random(lcg_random(seed));
            for _ in 0..15 {
                let Some((antes, a, b, resultado)) = jogar(&mut game) else { break };
                let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
                let mut esperado = vec!["troca"];
                for evento in &resultado.events {
                    match evento.name() {
                        "gameover" => {}
                        outro => esperado.push(outro),
                    }
                }
                if resultado.levels_gained > 0 {
                    esperado.push("nivel");
                }
                let obtido: Vec<&str> = passos.iter().map(nome).collect();
                assert_eq!(obtido, esperado, "seed {seed}");
                viu_queda |= obtido.contains(&"fall");
                jogadas += 1;
            }
        }
        assert!(jogadas > 100 && viu_queda, "o teste precisa ter jogado de verdade");
    }

    #[test]
    fn nivel_so_aparece_se_subiu() {
        // Subiu de nível: o último passo é a faixa com o nível final.
        let mut game = Game::with_random(lcg_random(5));
        game.score = 1790;
        let (antes, a, b, resultado) = jogar(&mut game).unwrap();
        assert!(resultado.levels_gained > 0);
        let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
        assert_eq!(passos.last(), Some(&Passo::Nivel { nivel: game.level }));
        // Sem subir de nível não há faixa.
        for seed in 1..=10 {
            let mut game = Game::with_random(lcg_random(seed));
            let (antes, a, b, resultado) = jogar(&mut game).unwrap();
            let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
            assert_eq!(passos.iter().any(|p| nome(p) == "nivel"), resultado.levels_gained > 0);
        }
    }

    #[test]
    fn fim_de_jogo_nao_vira_passo() {
        let mut game = Game::with_random(lcg_random(5));
        game.new_game(Mode::Classic);
        game.force_no_moves = true; // o motor age como se não houvesse mais jogadas
        let (antes, a, b, resultado) = jogar(&mut game).unwrap();
        assert!(resultado.ended);
        assert_eq!(resultado.events.last().map(|e| e.name()), Some("gameover"));
        let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
        // 1 troca + todos os eventos menos o "gameover" + (a faixa de nível, se subiu).
        let faixa = usize::from(resultado.levels_gained > 0);
        assert_eq!(passos.len(), 1 + resultado.events.len() - 1 + faixa);
    }

    #[test]
    fn zen_sem_jogadas_vira_passo_de_resgate() {
        let mut game = Game::with_random(lcg_random(5));
        game.force_no_moves = true;
        let (antes, a, b, resultado) = jogar(&mut game).unwrap();
        assert!(resultado.rescued);
        let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
        let resgate = passos.iter().find(|p| nome(p) == "rescue");
        assert!(matches!(resgate, Some(Passo::Resgate { indice, .. }) if *indice < 64));
    }

    #[test]
    fn supernova_vira_limpeza_com_a_marca() {
        let mut game = Game::with_random(lcg_random(5));
        let antes = game.board;
        let resultado = game.cheat_supernova(27);
        let passos = montar(antes, 0, 1, &resultado, game.level, Intensidade::Padrao);
        let marcada = passos
            .iter()
            .any(|p| matches!(p, Passo::Limpeza { supernova: true, ativadas, .. } if !ativadas.is_empty()));
        assert!(marcada, "a Supernova do cheat precisa aparecer marcada: {:?}", passos.iter().map(nome).collect::<Vec<_>>());
    }

    #[test]
    fn duracoes_do_js() {
        let limpeza = |ativadas: Vec<usize>, celulas: Vec<usize>| limpeza_a_mao(celulas, ativadas, vec![], Intensidade::Padrao);
        // A limpeza dura o que a mais longa das animações dura: as faíscas (380 ms) passam das 270 ms das pedras.
        assert!(perto(duracao_ms(&limpeza(vec![], vec![1, 2, 3])), 380.0));
        assert!(duracao_ms(&limpeza(vec![0], vec![0, 1])) >= 340.0);
        // Pedra a 5 casas da ativada: espera 100 ms (teto) e depois dura 340.
        assert!(perto(duracao_ms(&limpeza(vec![0], vec![5])), 440.0));
        assert!(perto(queda_ms(0), 250.0));
        assert!(perto(queda_ms(1), 282.0));
        assert!(perto(queda_ms(7), 470.0), "teto de 470 ms");

        // A queda dura o tempo da pedra que mais demora.
        let mut origens: Vec<i32> = (0..64).collect();
        let queda = |origens: &Vec<i32>| Passo::Queda { tabuleiro: tabuleiro(), origens: origens.clone() };
        assert!(perto(duracao_ms(&queda(&origens)), 0.0), "ninguém se mexeu: nada a animar");
        origens[8] = 0; // casa 8 (linha 1) veio da casa 0 (linha 0): 1 linha
        assert!(perto(duracao_ms(&queda(&origens)), 282.0));
        origens[56] = 0; // casa 56 (linha 7) veio da linha 0: 7 linhas, teto
        assert!(perto(duracao_ms(&queda(&origens)), 470.0));

        let (antes, depois) = (tabuleiro(), tabuleiro());
        assert!(perto(duracao_ms(&Passo::Embaralhar { antes, depois }), 440.0));
        assert!(perto(duracao_ms(&Passo::Resgate { tabuleiro: antes, indice: 3 }), 300.0));
        assert!(perto(duracao_ms(&Passo::Nivel { nivel: 2 }), 580.0));
    }

    #[test]
    fn queda_de_pedra_nova_usa_divisao_para_baixo() {
        // A origem é `linha * 8 + coluna`; pedra nova vem de uma linha negativa.
        assert_eq!(linhas_de_queda(-5, 0), 1, "linha -1, coluna 3");
        assert_eq!(linhas_de_queda(-1, 0), 1, "linha -1, coluna 7");
        assert_eq!(linhas_de_queda(-8, 0), 1, "linha -1, coluna 0");
        assert_eq!(linhas_de_queda(-9, 0), 2, "linha -2, coluna 7");
        assert_eq!(linhas_de_queda(27, 3), 0, "a casa 27 já está na linha 3");
        assert_eq!(linhas_de_queda(3, 3), 3, "da linha 0 até a linha 3");

        for (origem, destino) in [(-5, 0), (-17, 2), (3, 3), (27, 3), (0, 7)] {
            let linhas = linhas_de_queda(origem, destino) as f32;
            assert!(perto(queda_deslocamento(origem, destino, 0.0), linhas), "começa em {linhas} casas");
            let fim = queda_ms(linhas as i32);
            assert!(perto(queda_deslocamento(origem, destino, fim), 0.0), "termina no lugar");
            let mut anterior = f32::MAX;
            for passo in 0..=40 {
                let t = fim * passo as f32 / 40.0;
                let atual = queda_deslocamento(origem, destino, t);
                assert!(atual >= 0.0 && atual <= anterior + 1e-4, "nunca sobe nem fica negativo");
                anterior = atual;
            }
        }
    }

    #[test]
    fn amostras_nos_extremos() {
        assert!(perto(deslocamento_troca(0.0, true), 0.0));
        assert!(perto(deslocamento_troca(225.0, true), 1.0));
        assert!(perto(deslocamento_troca(1e9, true), 1.0));
        // Troca inválida: vai até a casa vizinha (pico em 48%) e volta.
        assert!(perto(deslocamento_troca(0.0, false), 0.0));
        assert!(perto(deslocamento_troca(0.48 * 300.0, false), 1.0));
        assert!(perto(deslocamento_troca(300.0, false), 0.0));

        let padrao = Intensidade::Padrao;
        let (escala, alfa, _) = limpeza_da_casa(&[], 10, 0.0, padrao);
        assert!(perto(escala, 1.0) && perto(alfa, 1.0));
        let (escala, alfa, _) = limpeza_da_casa(&[], 10, 270.0, padrao);
        assert!(perto(escala, 0.72) && perto(alfa, 0.0));
        // Com uma especial ativada na casa 0, a casa 5 (a 5 casas) espera 100 ms antes de começar.
        assert_eq!(limpeza_da_casa(&[0], 5, 99.0, padrao), (1.0, 1.0, 0.0));
        assert!(limpeza_da_casa(&[0], 5, 130.0, padrao).0 > 1.0);
        // A própria casa ativada começa na hora.
        assert!(limpeza_da_casa(&[0], 0, 30.0, padrao).0 > 1.0);

        assert!(perto(embaralhar_alfa(0.0).1, 1.0) && !embaralhar_alfa(0.0).0);
        assert!(perto(embaralhar_alfa(190.0).1, 0.12));
        assert!(embaralhar_alfa(190.0).0, "a partir dos 190 ms mostra o tabuleiro novo");
        assert!(perto(embaralhar_alfa(440.0).1, 1.0));

        let (escala, alfa) = resgate_da_casa(0.0);
        assert!(perto(escala, 0.25) && perto(alfa, 0.0));
        let (escala, alfa) = resgate_da_casa(300.0);
        assert!(perto(escala, 1.0) && perto(alfa, 1.0));

        let (escala, alfa) = nivel_faixa(0.0);
        assert!(perto(escala, 0.82) && perto(alfa, 0.0));
        let (escala, alfa) = nivel_faixa(0.25 * 580.0);
        assert!(perto(escala, 1.0) && perto(alfa, 1.0));
        let (escala, alfa) = nivel_faixa(580.0);
        assert!(perto(escala, 1.08) && perto(alfa, 0.0));
    }

    #[test]
    fn amostras_com_tempo_estranho_sao_finitas() {
        for t in [f32::NAN, -5.0, f32::INFINITY, f32::NEG_INFINITY, 1e30] {
            let numeros = [
                deslocamento_troca(t, true),
                deslocamento_troca(t, false),
                queda_deslocamento(-5, 0, t),
                queda_deslocamento(3, 3, t),
                limpeza_da_casa(&[0], 5, t, Intensidade::Padrao).0,
                limpeza_da_casa(&[0], 5, t, Intensidade::Padrao).1,
                limpeza_da_casa(&[], 5, t, Intensidade::Padrao).0,
                embaralhar_alfa(t).1,
                resgate_da_casa(t).0,
                resgate_da_casa(t).1,
                nivel_faixa(t).0,
                nivel_faixa(t).1,
            ];
            for numero in numeros {
                assert!(numero.is_finite(), "t = {t}");
            }
        }
    }

    #[test]
    fn reprodutor_passa_passo_a_passo_e_conta_concluidos() {
        let passos = vec![
            Passo::Troca { antes: tabuleiro(), a: 0, b: 1, valida: true },
            Passo::Nivel { nivel: 2 },
        ];
        let mut reprodutor = Reprodutor::novo(passos);
        assert!(!reprodutor.terminou());
        assert!(reprodutor.concluidos().is_empty());

        reprodutor.avancar(224.0);
        assert!(reprodutor.concluidos().is_empty());
        assert!(perto(reprodutor.atual().unwrap().1, 224.0));

        // Tempo inválido (NaN ou negativo) conta como zero: nada muda.
        reprodutor.avancar(f32::NAN);
        reprodutor.avancar(-5.0);
        assert!(perto(reprodutor.atual().unwrap().1, 224.0));

        reprodutor.avancar(1.0);
        assert_eq!(reprodutor.concluidos().len(), 1);
        assert!(matches!(reprodutor.atual(), Some((Passo::Nivel { .. }, _))));

        reprodutor.avancar(579.0);
        assert!(!reprodutor.terminou());
        reprodutor.avancar(1.0);
        assert!(reprodutor.terminou());
        assert_eq!(reprodutor.concluidos().len(), 2);
        assert!(reprodutor.atual().is_none());
    }

    #[test]
    fn reprodutor_pula_tudo_com_tempo_enorme_ou_infinito() {
        for tempo in [f32::INFINITY, 1e30] {
            let mut reprodutor = Reprodutor::novo(vec![
                Passo::Troca { antes: tabuleiro(), a: 0, b: 1, valida: false },
                Passo::Nivel { nivel: 3 },
            ]);
            reprodutor.avancar(tempo);
            assert!(reprodutor.terminou(), "tempo {tempo}");
            assert_eq!(reprodutor.concluidos().len(), 2);
        }
        let mut reprodutor = Reprodutor::novo(vec![Passo::Nivel { nivel: 3 }]);
        reprodutor.pular_para_o_fim();
        assert!(reprodutor.terminou());
        assert!(Reprodutor::novo(vec![]).terminou());
        assert!(Reprodutor::novo(vec![]).atual().is_none());
    }

    #[test]
    fn passo_sem_duracao_nao_prende_o_reprodutor() {
        // Uma queda em que ninguém se mexe dura 0 ms: o reprodutor passa direto por ela.
        let parada = Passo::Queda { tabuleiro: tabuleiro(), origens: (0..64).collect() };
        let mut reprodutor = Reprodutor::novo(vec![parada, Passo::Nivel { nivel: 2 }]);
        assert!(matches!(reprodutor.atual(), Some((Passo::Nivel { .. }, _))), "já começa no passo seguinte");
        reprodutor.avancar(580.0);
        assert!(reprodutor.terminou());
    }

    #[test]
    fn jogadas_longas_terminam() {
        let mut jogadas = 0;
        for modo in [Mode::Zen, Mode::Classic] {
            for seed in 1..=20 {
                let mut game = Game::with_random(lcg_random(seed));
                game.new_game(modo);
                for _ in 0..15 {
                    let Some((antes, a, b, resultado)) = jogar(&mut game) else { break };
                    let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
                    let total: f32 = passos.iter().map(duracao_ms).sum();
                    assert!(total.is_finite() && total < 60_000.0, "total {total}");
                    let mut reprodutor = Reprodutor::novo(passos);
                    let mut quadros = 0;
                    while !reprodutor.terminou() {
                        reprodutor.avancar(16.0);
                        quadros += 1;
                        assert!(quadros < 10_000, "a animação não termina");
                    }
                    jogadas += 1;
                }
            }
        }
        assert!(jogadas > 200);
    }

    #[test]
    fn limpeza_guarda_a_cadeia_e_se_criou_pedra_especial() {
        // Semente 4: a 1ª limpeza cria uma pedra especial e a 2ª (a cascata) não.
        let mut game = Game::with_random(lcg_random(4));
        let (antes, a, b, resultado) = jogar(&mut game).unwrap();
        let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
        let limpezas: Vec<(u32, bool)> = passos
            .iter()
            .filter_map(|passo| match passo {
                Passo::Limpeza { cadeia, criou, .. } => Some((*cadeia, *criou)),
                _ => None,
            })
            .collect();
        assert_eq!(limpezas, vec![(1, true), (2, false)]);
        // Semente 9: quatro limpezas seguidas, nenhuma cria pedra.
        let mut game = Game::with_random(lcg_random(9));
        let (antes, a, b, resultado) = jogar(&mut game).unwrap();
        let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
        let limpezas: Vec<(u32, bool)> = passos
            .iter()
            .filter_map(|passo| match passo {
                Passo::Limpeza { cadeia, criou, .. } => Some((*cadeia, *criou)),
                _ => None,
            })
            .collect();
        assert_eq!(limpezas, vec![(1, false), (2, false), (3, false), (4, false)]);
    }

    // ---- Efeitos, intensidade e tempos da limpeza (M4) ----

    /// Os `visuais` e a `intensidade` de cada passo de limpeza de uma jogada montada.
    fn visuais_e_intensidades(passos: &[Passo]) -> Vec<(&Vec<Visual>, Intensidade)> {
        passos
            .iter()
            .filter_map(|passo| match passo {
                Passo::Limpeza { visuais, intensidade, .. } => Some((visuais, *intensidade)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn montar_guarda_os_visuais_e_a_intensidade() {
        // Um Pulso no meio do tabuleiro, ativado por uma explosão geral: a limpeza leva o Pulso.
        let mut game = Game::with_random(lcg_random(5));
        assert!(game.cheat_spawn_special(27, TileKind::Burst));
        let antes = game.board;
        let resultado = game.cheat_explode();
        let passos = montar(antes, 0, 1, &resultado, game.level, Intensidade::Vibrante);
        let limpezas = visuais_e_intensidades(&passos);
        assert!(!limpezas.is_empty());
        assert!(limpezas.iter().all(|(_, intensidade)| *intensidade == Intensidade::Vibrante), "a intensidade vai em todo passo");
        let pulso = limpezas.iter().flat_map(|(visuais, _)| visuais.iter()).find(|visual| visual.casa == 27);
        assert_eq!(pulso, Some(&visual(27, TipoVisual::Pulso)), "cor 0 e sem origens fora da Supernova");

        // Uma Supernova: o visual leva a cor e as pedras fundidas do motor, e vem marcada.
        let mut game = Game::with_random(lcg_random(5));
        let antes = game.board;
        let resultado = game.cheat_supernova(27);
        let passos = montar(antes, 0, 1, &resultado, game.level, Intensidade::Suave);
        let do_motor = resultado
            .events
            .iter()
            .find_map(|evento| match evento {
                Event::Clear { activated, .. } => activated.iter().find_map(|a| match a {
                    Activation::Supernova(nova) => Some(nova.clone()),
                    Activation::Special { .. } => None,
                }),
                _ => None,
            })
            .expect("o cheat faz uma Supernova");
        let esperado = Visual { casa: do_motor.index, tipo: TipoVisual::Supernova, cor: do_motor.color, origens: do_motor.sources.clone() };
        let limpezas = visuais_e_intensidades(&passos);
        assert!(limpezas.iter().any(|(visuais, intensidade)| visuais.contains(&esperado) && *intensidade == Intensidade::Suave));
        assert!(!esperado.origens.is_empty());
    }

    #[test]
    fn limpeza_sem_especial_tem_visuais_vazio() {
        let mut game = Game::with_random(lcg_random(9));
        let (antes, a, b, resultado) = jogar(&mut game).unwrap();
        let passos = montar(antes, a, b, &resultado, game.level, Intensidade::Padrao);
        let limpezas = visuais_e_intensidades(&passos);
        assert_eq!(limpezas.len(), 4, "a semente 9 faz quatro limpezas, sem nenhuma especial");
        assert!(limpezas.iter().all(|(visuais, _)| visuais.is_empty()));
    }

    #[test]
    fn limpeza_comum_dura_380_ms_no_padrao() {
        let comum = limpeza_a_mao(vec![1, 2, 3], vec![], vec![], Intensidade::Padrao);
        assert!(perto(duracao_ms(&comum), 380.0), "as faíscas (380) passam das pedras (270)");
        assert!(perto(fusao_ms(&comum), 0.0));
    }

    #[test]
    fn limpeza_com_especial_dura_o_maior_entre_pedras_e_efeitos() {
        let com = |celulas: Vec<usize>, visuais: Vec<Visual>| {
            duracao_ms(&limpeza_a_mao(celulas, vec![0], visuais, Intensidade::Padrao))
        };
        // Pedras 340 + 22 (a casa 1 está a 1 casa da ativada) = 362: vencem as faíscas (380).
        assert!(perto(com(vec![0, 1], vec![visual(0, TipoVisual::Pulso)]), 380.0));
        // Pedra a 5 casas: espera 100 (o atraso não escala) e dura 340 = 440: vencem as pedras.
        assert!(perto(com(vec![5], vec![visual(0, TipoVisual::Raio)]), 440.0));
        // Onda, feixes e véu (360, 330, 370) nunca passam das faíscas (380).
        for tipo in [TipoVisual::Pulso, TipoVisual::Raio, TipoVisual::Espectro] {
            assert!(perto(com(vec![0], vec![visual(0, tipo)]), 380.0), "{tipo:?}");
        }
        // A Supernova (480) passa das faíscas, mesmo sem fusão (sem pedras de origem).
        assert!(perto(com(vec![0], vec![visual(0, TipoVisual::Supernova)]), 480.0));
    }

    #[test]
    fn supernova_ganha_fusao_antes_da_explosao() {
        let passo = limpeza_a_mao(vec![27], vec![27], vec![supernova(27)], Intensidade::Padrao);
        assert!(perto(fusao_ms(&passo), 240.0));
        assert!(perto(duracao_ms(&passo), 240.0 + 480.0), "fusão e depois a explosão");
        // Sem pedras de origem não há o que fundir.
        let sem_origens = limpeza_a_mao(vec![27], vec![27], vec![visual(27, TipoVisual::Supernova)], Intensidade::Padrao);
        assert!(perto(fusao_ms(&sem_origens), 0.0));
        // Pulso, Raio e Espectro não têm fusão; os outros passos também não.
        let pulso = limpeza_a_mao(vec![27], vec![27], vec![visual(27, TipoVisual::Pulso)], Intensidade::Padrao);
        assert!(perto(fusao_ms(&pulso), 0.0));
        assert!(perto(fusao_ms(&Passo::Nivel { nivel: 2 }), 0.0));
    }

    #[test]
    fn escala_suave_e_vibrante_multiplicam_a_duracao() {
        let comum = |intensidade| duracao_ms(&limpeza_a_mao(vec![1, 2, 3], vec![], vec![], intensidade));
        assert!((comum(Intensidade::Suave) - 380.0 * 0.82).abs() < 0.01);
        assert!((comum(Intensidade::Padrao) - 380.0).abs() < 0.01);
        assert!((comum(Intensidade::Vibrante) - 380.0 * 1.1).abs() < 0.01);
        // A fusão e a Supernova escalam juntas; o atraso por distância da pedra não escala.
        let nova = |intensidade| limpeza_a_mao(vec![27], vec![27], vec![supernova(27)], intensidade);
        assert!((fusao_ms(&nova(Intensidade::Suave)) - 240.0 * 0.82).abs() < 0.01);
        assert!((duracao_ms(&nova(Intensidade::Vibrante)) - (240.0 + 480.0) * 1.1).abs() < 0.01);
        let longe = |intensidade| duracao_ms(&limpeza_a_mao(vec![5], vec![0], vec![visual(0, TipoVisual::Raio)], intensidade));
        assert!((longe(Intensidade::Suave) - (340.0 * 0.82 + 100.0)).abs() < 0.01);
        assert!((longe(Intensidade::Vibrante) - (340.0 * 1.1 + 100.0)).abs() < 0.01);
    }

    /// O instante (ms) em que a pedra chega ao pico: 38% do progresso já "suavizado".
    fn instante_do_pico(duracao: f32) -> f32 {
        duracao * (1.0 - 0.62_f32.powf(1.0 / 3.0))
    }

    #[test]
    fn limpeza_da_casa_pico_1_1_no_padrao_e_1_18_no_vibrante() {
        for (intensidade, pico, escala_de_tempo) in
            [(Intensidade::Suave, 1.1, 0.82), (Intensidade::Padrao, 1.1, 1.0), (Intensidade::Vibrante, 1.18, 1.1)]
        {
            let t = instante_do_pico(270.0 * escala_de_tempo);
            let (escala, opacidade, _) = limpeza_da_casa(&[], 10, t, intensidade);
            assert!((escala - pico).abs() < 1e-3, "{intensidade:?}: pico {escala}");
            assert!((opacidade - 1.0).abs() < 1e-3, "{intensidade:?}: ainda inteira no pico");
            // No fim a pedra sumiu (escala 0,72, opacidade 0), em qualquer intensidade.
            let (escala, opacidade, _) = limpeza_da_casa(&[], 10, 270.0 * escala_de_tempo, intensidade);
            assert!(perto(escala, 0.72) && perto(opacidade, 0.0), "{intensidade:?}");
        }
    }

    #[test]
    fn limpeza_da_casa_brilho_sobe_e_volta() {
        // O brilho vai de 0 (nada) até o pico de cada intensidade e desce depois (o JS vai do pico a 1,5).
        // Escala do brilho: (filtro − 1) / 1,1, onde 2,1 é o maior filtro (o Vibrante).
        for (intensidade, pico, escala_de_tempo) in
            [(Intensidade::Suave, 0.35 / 1.1, 0.82), (Intensidade::Padrao, 0.8 / 1.1, 1.0), (Intensidade::Vibrante, 1.0, 1.1)]
        {
            let duracao = 270.0 * escala_de_tempo;
            let comeco = limpeza_da_casa(&[], 10, 0.0, intensidade).2;
            let no_pico = limpeza_da_casa(&[], 10, instante_do_pico(duracao), intensidade).2;
            let no_fim = limpeza_da_casa(&[], 10, duracao, intensidade).2;
            assert!(perto(comeco, 0.0), "{intensidade:?}: começa sem brilho");
            assert!((no_pico - pico).abs() < 1e-3, "{intensidade:?}: pico {no_pico}, esperado {pico}");
            assert!((no_fim - 0.5 / 1.1).abs() < 1e-3, "{intensidade:?}: o fim é o filtro 1,5");
            if intensidade != Intensidade::Suave {
                assert!(no_pico > no_fim, "{intensidade:?}: sobe e depois desce");
            }
        }
        // O brilho cresce até o pico (nunca desce antes dele).
        let mut anterior = -1.0;
        for passo in 0..=20 {
            let t = instante_do_pico(270.0) * passo as f32 / 20.0;
            let atual = limpeza_da_casa(&[], 10, t, Intensidade::Padrao).2;
            assert!(atual >= anterior - 1e-6);
            anterior = atual;
        }
    }

    #[test]
    fn fusao_da_casa_chega_a_1_16() {
        for (intensidade, escala_de_tempo) in [(Intensidade::Suave, 0.82), (Intensidade::Padrao, 1.0), (Intensidade::Vibrante, 1.1)] {
            let duracao = 240.0 * escala_de_tempo;
            let (escala, brilho) = fusao_da_casa(0.0, intensidade);
            assert!(perto(escala, 1.0) && perto(brilho, 0.0), "{intensidade:?}: começa quieta");
            // Em 60% a pedra está no pico: 1,16 e filtro 1,8.
            let (escala, brilho) = fusao_da_casa(0.6 * duracao, intensidade);
            assert!((escala - 1.16).abs() < 1e-3, "{intensidade:?}: {escala}");
            assert!((brilho - 0.8 / 1.1).abs() < 1e-3, "{intensidade:?}: {brilho}");
            // No fim encolhe a 0,9, com o filtro 2.
            let (escala, brilho) = fusao_da_casa(duracao, intensidade);
            assert!((escala - 0.9).abs() < 1e-3 && (brilho - 1.0 / 1.1).abs() < 1e-3, "{intensidade:?}");
        }
    }

    #[test]
    fn tempo_estranho_nao_gera_nan() {
        for intensidade in Intensidade::TODAS {
            for t in [f32::NAN, -5.0, f32::INFINITY, f32::NEG_INFINITY, 1e30, -1e30] {
                let (escala, opacidade, brilho) = limpeza_da_casa(&[0], 5, t, intensidade);
                let (escala_f, brilho_f) = fusao_da_casa(t, intensidade);
                for numero in [escala, opacidade, brilho, escala_f, brilho_f] {
                    assert!(numero.is_finite(), "{intensidade:?} t = {t}");
                }
                assert!((0.0..=1.0).contains(&opacidade) && (0.0..=1.0).contains(&brilho) && (0.0..=1.0).contains(&brilho_f));
                assert!((0.7..=1.2).contains(&escala) && (0.8..=1.2).contains(&escala_f), "{intensidade:?} t = {t}");
            }
        }
    }
}
