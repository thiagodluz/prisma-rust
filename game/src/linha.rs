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

use prisma_engine::{position, Activation, Board, Event, MoveResult, SIZE};

// ---- Durações, em milissegundos (as do JS) -----------------------------------------------

const TROCA_VALIDA_MS: f32 = 225.0;
const TROCA_INVALIDA_MS: f32 = 300.0;
/// Na troca inválida as pedras vão até a casa vizinha em 48% do tempo e depois voltam.
const PICO_DA_TROCA_INVALIDA: f32 = 0.48;
const LIMPEZA_MS: f32 = 270.0;
/// Limpeza em que uma pedra especial foi ativada (dura um pouco mais).
const LIMPEZA_ESPECIAL_MS: f32 = 340.0;
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
    /// `ativadas` são as casas de pedras especiais (e Supernovas) que dispararam.
    Limpeza { tabuleiro: Board, celulas: Vec<usize>, ativadas: Vec<usize>, supernova: bool, ganho: u64 },
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

/// Monta os passos de uma jogada.
///
/// `antes` é o tabuleiro de antes de chamar `Game::play(a, b)`, `resultado` é o que o motor
/// devolveu e `nivel_final` é o nível do jogo depois da jogada. Jogada inválida dá só o passo
/// da troca que vai e volta. O fim de partida não vira passo: o cartão de "Fim de partida"
/// aparece depois da animação, como no JS.
pub fn montar(antes: Board, a: usize, b: usize, resultado: &MoveResult, nivel_final: u32) -> Vec<Passo> {
    let mut passos = vec![Passo::Troca { antes, a, b, valida: resultado.valid }];
    if !resultado.valid {
        return passos;
    }
    for evento in &resultado.events {
        match evento {
            Event::Clear { board, cells, activated, earned, .. } => passos.push(Passo::Limpeza {
                tabuleiro: *board,
                celulas: cells.clone(),
                ativadas: activated.iter().map(casa_da_ativacao).collect(),
                supernova: activated.iter().any(|ativacao| matches!(ativacao, Activation::Supernova(_))),
                ganho: *earned,
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
        Passo::Limpeza { celulas, ativadas, .. } => {
            if ativadas.is_empty() {
                LIMPEZA_MS
            } else {
                // A última pedra a começar (a mais longe da especial) define quando acaba.
                let maior_atraso = celulas
                    .iter()
                    .map(|&casa| atraso_da_casa(ativadas, casa))
                    .fold(0.0, f32::max);
                LIMPEZA_ESPECIAL_MS + maior_atraso
            }
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

/// Como está a pedra da `casa` que vai sumir, `t_ms` depois do começo da limpeza:
/// devolve (escala, opacidade). Ela cresce um pouco (1 → 1,1) e depois encolhe e apaga
/// (até 0,72 e opacidade 0).
pub fn limpeza_da_casa(ativadas: &[usize], casa: usize, t_ms: f32) -> (f32, f32) {
    let (duracao, atraso) = if ativadas.is_empty() {
        (LIMPEZA_MS, 0.0)
    } else {
        (LIMPEZA_ESPECIAL_MS, atraso_da_casa(ativadas, casa))
    };
    let tempo = if t_ms.is_nan() { 0.0 } else { t_ms - atraso };
    let p = desacelera(progresso(tempo, duracao));
    if p < 0.38 {
        (entre(1.0, 1.1, p / 0.38), 1.0)
    } else {
        let q = (p - 0.38) / 0.62;
        (entre(1.1, 0.72, q), entre(1.0, 0.0, q))
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
    use prisma_engine::{lcg_random, Game, Mode};

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
        let passos = montar(antes, a, b, &resultado, game.level);
        assert_eq!(passos, vec![Passo::Troca { antes, a, b, valida: false }]);
        assert!(perto(duracao_ms(&passos[0]), 300.0));
    }

    #[test]
    fn troca_valida_dura_225() {
        let mut game = Game::with_random(lcg_random(3));
        let (antes, a, b, resultado) = jogar(&mut game).unwrap();
        assert!(resultado.valid);
        let passos = montar(antes, a, b, &resultado, game.level);
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
                let passos = montar(antes, a, b, &resultado, game.level);
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
        let passos = montar(antes, a, b, &resultado, game.level);
        assert_eq!(passos.last(), Some(&Passo::Nivel { nivel: game.level }));
        // Sem subir de nível não há faixa.
        for seed in 1..=10 {
            let mut game = Game::with_random(lcg_random(seed));
            let (antes, a, b, resultado) = jogar(&mut game).unwrap();
            let passos = montar(antes, a, b, &resultado, game.level);
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
        let passos = montar(antes, a, b, &resultado, game.level);
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
        let passos = montar(antes, a, b, &resultado, game.level);
        let resgate = passos.iter().find(|p| nome(p) == "rescue");
        assert!(matches!(resgate, Some(Passo::Resgate { indice, .. }) if *indice < 64));
    }

    #[test]
    fn supernova_vira_limpeza_com_a_marca() {
        let mut game = Game::with_random(lcg_random(5));
        let antes = game.board;
        let resultado = game.cheat_supernova(27);
        let passos = montar(antes, 0, 1, &resultado, game.level);
        let marcada = passos
            .iter()
            .any(|p| matches!(p, Passo::Limpeza { supernova: true, ativadas, .. } if !ativadas.is_empty()));
        assert!(marcada, "a Supernova do cheat precisa aparecer marcada: {:?}", passos.iter().map(nome).collect::<Vec<_>>());
    }

    #[test]
    fn duracoes_do_js() {
        let limpeza = |ativadas: Vec<usize>, celulas: Vec<usize>| Passo::Limpeza {
            tabuleiro: tabuleiro(),
            celulas,
            ativadas,
            supernova: false,
            ganho: 0,
        };
        assert!(perto(duracao_ms(&limpeza(vec![], vec![1, 2, 3])), 270.0));
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

        let (escala, alfa) = limpeza_da_casa(&[], 10, 0.0);
        assert!(perto(escala, 1.0) && perto(alfa, 1.0));
        let (escala, alfa) = limpeza_da_casa(&[], 10, 270.0);
        assert!(perto(escala, 0.72) && perto(alfa, 0.0));
        // Com uma especial ativada na casa 0, a casa 5 (a 5 casas) espera 100 ms antes de começar.
        assert_eq!(limpeza_da_casa(&[0], 5, 99.0), (1.0, 1.0));
        assert!(limpeza_da_casa(&[0], 5, 130.0).0 > 1.0);
        // A própria casa ativada começa na hora.
        assert!(limpeza_da_casa(&[0], 0, 30.0).0 > 1.0);

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
                limpeza_da_casa(&[0], 5, t).0,
                limpeza_da_casa(&[0], 5, t).1,
                limpeza_da_casa(&[], 5, t).0,
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
                    let passos = montar(antes, a, b, &resultado, game.level);
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
}
