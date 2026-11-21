//! O estado do jogo na tela: as duas partidas (Zen e Clássico), os gestos do dedo e a
//! animação em andamento.
//!
//! O motor resolve a jogada inteira de uma vez; este módulo guarda o resultado como uma
//! linha do tempo (`linha`) e a tela vai mostrando passo a passo. Enquanto a linha do tempo
//! roda, o painel mostra os pontos "de antes" e só sobe quando cada limpeza acaba, por isso
//! existe o [`Estagio`].

use crate::input::{Acao, Evento, Gestos};
use crate::layout::TelaJogo;
use crate::linha::{montar, Passo, Reprodutor};
use prisma_engine::{lcg_random, level_goal, Game, Mode};

/// Os números que o painel mostra AGORA (que podem estar atrás do motor durante a animação).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Estagio {
    pub pontos: u64,
    pub nivel: u32,
    /// Pontos que o jogador tinha quando o nível atual começou (a barra de progresso usa).
    pub inicio_do_nivel: u64,
}

/// Uma animação em andamento e os números de antes da jogada (de onde o painel parte).
struct EmAndamento {
    reprodutor: Reprodutor,
    base: Estagio,
}

/// O nome do modo nas linhas de registro.
fn nome_do_modo(modo: Mode) -> &'static str {
    match modo {
        Mode::Zen => "zen",
        Mode::Classic => "classico",
    }
}

pub struct App {
    zen: Game,
    classico: Game,
    modo: Mode,
    gestos: Gestos,
    animacao: Option<EmAndamento>,
    /// Linhas "PRISMA_M1 ..." que ainda não foram entregues (o teste do celular virtual lê).
    registros: Vec<String>,
}

impl App {
    /// Um app com sorte "de verdade", começando no Zen.
    pub fn novo() -> App {
        let mut classico = Game::new();
        classico.new_game(Mode::Classic);
        App::de_jogos(Game::new(), classico)
    }

    /// Um app com sorte repetível (para testes e para as fotos de conferência).
    pub fn com_sementes(zen: u32, classico: u32) -> App {
        let mut jogo_classico = Game::with_random(lcg_random(classico));
        jogo_classico.new_game(Mode::Classic);
        App::de_jogos(Game::with_random(lcg_random(zen)), jogo_classico)
    }

    fn de_jogos(zen: Game, classico: Game) -> App {
        App {
            zen,
            classico,
            modo: Mode::Zen,
            gestos: Gestos::default(),
            animacao: None,
            registros: vec![format!("PRISMA_M1 pronto modo={}", nome_do_modo(Mode::Zen))],
        }
    }

    pub fn modo(&self) -> Mode {
        self.modo
    }

    /// A partida do modo atual.
    pub fn jogo(&self) -> &Game {
        match self.modo {
            Mode::Zen => &self.zen,
            Mode::Classic => &self.classico,
        }
    }

    /// Só para testes e para as cenas de conferência: mexer direto no motor (pontuação, fim de
    /// partida...). O jogo de verdade só muda por `tocar`.
    pub(crate) fn jogo_mut(&mut self) -> &mut Game {
        match self.modo {
            Mode::Zen => &mut self.zen,
            Mode::Classic => &mut self.classico,
        }
    }

    pub fn gestos(&self) -> &Gestos {
        &self.gestos
    }

    /// Há uma animação em andamento? Enquanto houver, o tabuleiro não aceita toques.
    pub fn ocupado(&self) -> bool {
        self.animacao.is_some()
    }

    /// O tabuleiro não aceita jogadas durante a animação nem depois do fim da partida.
    fn bloqueado(&self) -> bool {
        self.ocupado() || self.jogo().ended
    }

    /// Um evento do dedo (ou do mouse). Por enquanto só o tabuleiro; os botões vêm na Task 5.
    pub fn tocar(&mut self, evento: Evento, tela: &TelaJogo) {
        let bloqueado = self.bloqueado();
        if let Acao::Trocar(a, b) = self.gestos.tratar(evento, tela.tabuleiro, bloqueado) {
            self.jogar(a, b);
        }
    }

    /// Joga a troca no motor (que resolve tudo de uma vez) e prepara a animação.
    fn jogar(&mut self, a: usize, b: usize) {
        let base = self.estagio();
        let jogo = self.jogo_mut();
        let antes = jogo.board;
        let resultado = jogo.play(a, b);
        let (pontos, nivel) = (jogo.score, jogo.level);
        let passos = montar(antes, a, b, &resultado, nivel);
        self.registros.push(format!("PRISMA_M1 jogada valida={} pontos={} nivel={}", resultado.valid, pontos, nivel));
        self.animacao = Some(EmAndamento { reprodutor: Reprodutor::novo(passos), base });
        self.encerrar_se_terminou();
    }

    /// Deixa passar `dt_ms` milissegundos da animação. Tempo inválido conta como zero; tempo
    /// enorme termina tudo de uma vez (ver `Reprodutor::avancar`).
    pub fn avancar(&mut self, dt_ms: f32) {
        if let Some(andamento) = self.animacao.as_mut() {
            andamento.reprodutor.avancar(dt_ms);
        }
        self.encerrar_se_terminou();
    }

    /// Se a animação acabou: libera a entrada, esquece o gesto e avisa no registro.
    fn encerrar_se_terminou(&mut self) {
        if self.animacao.as_ref().is_some_and(|andamento| andamento.reprodutor.terminou()) {
            self.animacao = None;
            self.gestos.limpar();
            self.registros.push("PRISMA_M1 animacao_terminou".to_string());
        }
    }

    /// O passo da animação em andamento e há quantos milissegundos ele começou.
    pub fn passo_atual(&self) -> Option<(&Passo, f32)> {
        self.animacao.as_ref().and_then(|andamento| andamento.reprodutor.atual())
    }

    /// O que o painel mostra agora. Sem animação são os números do motor. Com animação: os
    /// números de antes da jogada mais os pontos de cada limpeza que já acabou, subindo de
    /// nível do mesmo jeito que o motor (o laço do JS).
    pub fn estagio(&self) -> Estagio {
        let Some(andamento) = &self.animacao else {
            let jogo = self.jogo();
            return Estagio { pontos: jogo.score, nivel: jogo.level, inicio_do_nivel: jogo.level_start_score };
        };
        let mut estagio = andamento.base;
        for passo in andamento.reprodutor.concluidos() {
            if let Passo::Limpeza { ganho, .. } = passo {
                estagio.pontos += ganho;
            }
        }
        // Cada nível tem uma meta de pontos; passou da meta, o próximo nível começa aqui.
        // `saturating_sub` é a subtração que para no zero em vez de dar erro.
        while estagio.pontos.saturating_sub(estagio.inicio_do_nivel) >= level_goal(estagio.nivel) {
            estagio.inicio_do_nivel += level_goal(estagio.nivel);
            estagio.nivel += 1;
        }
        estagio
    }

    /// Entrega (e esvazia) as linhas de registro acumuladas.
    pub fn registro(&mut self) -> Vec<String> {
        std::mem::take(&mut self.registros)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Evento;
    use crate::layout::{cell_rect, tela_jogo, TelaJogo};
    use crate::linha::Passo;
    use prisma_engine::{adjacent, lcg_random, Game, Mode, SIZE};

    fn tela() -> TelaJogo {
        tela_jogo(540.0, 960.0, true)
    }

    /// O ponto do meio de uma casa, em pixels da tela.
    fn centro(tela: &TelaJogo, casa: usize) -> (f32, f32) {
        let retangulo = cell_rect(tela.tabuleiro, casa / SIZE, casa % SIZE);
        (retangulo.x + retangulo.w / 2.0, retangulo.y + retangulo.h / 2.0)
    }

    /// Põe o dedo na casa `a`, arrasta até `b` e solta.
    fn arrastar(app: &mut App, a: usize, b: usize) {
        let tela = tela();
        let (ax, ay) = centro(&tela, a);
        let (bx, by) = centro(&tela, b);
        app.tocar(Evento::Baixo { x: ax, y: ay }, &tela);
        app.tocar(Evento::Mover { x: bx, y: by }, &tela);
        app.tocar(Evento::Cima { x: bx, y: by }, &tela);
    }

    /// Um toque rápido na casa (encosta e solta no mesmo lugar).
    fn tocar_casa(app: &mut App, casa: usize) {
        let tela = tela();
        let (x, y) = centro(&tela, casa);
        app.tocar(Evento::Baixo { x, y }, &tela);
        app.tocar(Evento::Cima { x, y }, &tela);
    }

    /// A jogada que a Dica sugere (sempre válida).
    fn boa_jogada(app: &mut App) -> (usize, usize) {
        app.jogo_mut().hint().expect("o tabuleiro tem jogada")
    }

    /// Uma troca que o motor recusa, num jogo novo da `semente` (força bruta; cada tentativa
    /// usa um jogo novo, para uma jogada válida não mudar o tabuleiro das seguintes).
    fn par_invalido(semente: u32) -> (usize, usize) {
        for a in 0..SIZE * SIZE {
            for b in [a + 1, a + SIZE] {
                if b < SIZE * SIZE && adjacent(a, b) {
                    let mut jogo = Game::with_random(lcg_random(semente));
                    if !jogo.play(a, b).valid {
                        return (a, b);
                    }
                }
            }
        }
        panic!("não achei nenhuma troca inválida");
    }

    /// Deixa a animação rodar até o fim, em quadros de 16 ms.
    fn terminar(app: &mut App) {
        for _ in 0..10_000 {
            if !app.ocupado() {
                return;
            }
            app.avancar(16.0);
        }
        panic!("a animação não terminou");
    }

    fn nome(passo: &Passo) -> &'static str {
        match passo {
            Passo::Troca { .. } => "troca",
            Passo::Limpeza { .. } => "limpeza",
            Passo::Queda { .. } => "queda",
            Passo::Embaralhar { .. } => "embaralhar",
            Passo::Resgate { .. } => "resgate",
            Passo::Nivel { .. } => "nivel",
        }
    }

    fn do_motor(app: &App) -> Estagio {
        let jogo = app.jogo();
        Estagio { pontos: jogo.score, nivel: jogo.level, inicio_do_nivel: jogo.level_start_score }
    }

    #[test]
    fn comeca_no_zen_e_sem_animacao() {
        let app = App::com_sementes(7, 9);
        assert_eq!(app.modo(), Mode::Zen);
        assert!(!app.ocupado());
        assert!(app.passo_atual().is_none());
        assert_eq!(app.estagio(), Estagio { pontos: 0, nivel: 1, inicio_do_nivel: 0 });
    }

    #[test]
    fn arrasto_valido_inicia_linha_e_trava_a_entrada() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        assert!(app.ocupado());
        assert!(app.jogo().score > 0, "o motor já jogou; a tela só está mostrando devagar");
        // O painel ainda mostra os pontos de antes (a primeira etapa é a troca).
        assert_eq!(app.estagio().pontos, 0);
        // Um dedo novo durante a animação não cria gesto.
        let tela = tela();
        let (x, y) = centro(&tela, 0);
        app.tocar(Evento::Baixo { x, y }, &tela);
        assert_eq!(app.gestos().pressionada(), None);
    }

    #[test]
    fn jogada_invalida_so_balanca() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = par_invalido(7);
        let tabuleiro = app.jogo().board;
        let pontos = app.jogo().score;
        arrastar(&mut app, a, b);
        assert!(app.ocupado());
        app.avancar(299.0);
        assert!(app.ocupado(), "a troca inválida dura 300 ms");
        app.avancar(2.0);
        assert!(!app.ocupado());
        assert_eq!(app.jogo().board, tabuleiro);
        assert_eq!(app.jogo().score, pontos);
        assert!(app.registro().contains(&"PRISMA_M1 jogada valida=false pontos=0 nivel=1".to_string()));
    }

    #[test]
    fn toque_toque_tambem_joga() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        tocar_casa(&mut app, a);
        assert_eq!(app.gestos().selecionada(), Some(a));
        assert!(!app.ocupado(), "um toque só marca a casa");
        tocar_casa(&mut app, b);
        assert!(app.ocupado());

        let mut gemeo = App::com_sementes(7, 9);
        arrastar(&mut gemeo, a, b);
        assert_eq!(app.jogo().score, gemeo.jogo().score);
        assert_eq!(app.jogo().board, gemeo.jogo().board);
    }

    #[test]
    fn no_fim_o_estagio_e_igual_ao_motor() {
        for modo in [Mode::Zen, Mode::Classic] {
            for semente in 1..=15u32 {
                let mut app = App::com_sementes(semente, semente);
                if modo == Mode::Classic {
                    // A partida em andamento vira Clássico (só para o teste; o botão da aba vem na Task 5).
                    app.jogo_mut().new_game(Mode::Classic);
                }
                for _ in 0..12 {
                    let Some((a, b)) = app.jogo_mut().hint() else { break };
                    let base = app.estagio();
                    arrastar(&mut app, a, b);
                    assert!(app.ocupado(), "{modo:?} semente {semente}");
                    let final_do_motor = do_motor(&app);
                    let mut anterior = base;
                    for _ in 0..10_000 {
                        if !app.ocupado() {
                            break;
                        }
                        let agora = app.estagio();
                        assert!(agora.pontos >= anterior.pontos, "os pontos nunca diminuem ({modo:?} semente {semente})");
                        assert!(agora.pontos >= base.pontos && agora.pontos <= final_do_motor.pontos);
                        assert!(agora.nivel >= anterior.nivel && agora.nivel <= final_do_motor.nivel);
                        anterior = agora;
                        app.avancar(16.0);
                    }
                    assert!(!app.ocupado(), "{modo:?} semente {semente}");
                    assert_eq!(app.estagio(), do_motor(&app), "{modo:?} semente {semente}");
                    // Na última etapa (a queda) todas as limpezas já acabaram: o painel já tem os pontos todos.
                    assert_eq!(anterior.pontos, final_do_motor.pontos, "{modo:?} semente {semente}");
                }
            }
        }
    }

    #[test]
    fn subir_de_nivel_aparece_no_estagio_no_passo_certo() {
        let mut app = App::com_sementes(7, 9);
        app.jogo_mut().score = 1790;
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        assert_eq!(app.jogo().level, 2, "a jogada boa passa dos 1800 pontos");
        assert_eq!(app.estagio().nivel, 1, "no começo o painel ainda mostra o nível 1");
        let mut ultimo = "";
        for _ in 0..10_000 {
            if let Some((passo, _)) = app.passo_atual() {
                ultimo = nome(passo);
                match ultimo {
                    "troca" => assert_eq!(app.estagio().nivel, 1),
                    "nivel" => assert_eq!(app.estagio().nivel, 2, "a faixa só aparece com o painel já no nível 2"),
                    _ => {}
                }
            }
            if !app.ocupado() {
                break;
            }
            app.avancar(16.0);
        }
        assert_eq!(ultimo, "nivel", "o último passo é a faixa de nível");
        assert_eq!(app.estagio().nivel, 2);
    }

    #[test]
    fn avancar_com_tempo_estranho_nao_trava() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        for dt in [f32::NAN, -3.0, 0.0] {
            app.avancar(dt);
            assert!(app.ocupado(), "dt = {dt}");
        }
        app.avancar(1e30);
        assert!(!app.ocupado());

        // Infinito também termina.
        let mut outro = App::com_sementes(7, 9);
        arrastar(&mut outro, a, b);
        outro.avancar(f32::INFINITY);
        assert!(!outro.ocupado());
    }

    #[test]
    fn toque_com_animacao_nao_faz_nada() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        let tabuleiro = app.jogo().board;
        let pontos = app.jogo().score;
        // Dois toques em casas vizinhas trocariam as pedras; durante a animação nada acontece.
        tocar_casa(&mut app, 0);
        assert_eq!(app.gestos().selecionada(), None, "o primeiro toque nem marca a casa");
        tocar_casa(&mut app, 1);
        assert_eq!(app.gestos().selecionada(), None);
        assert_eq!(app.jogo().board, tabuleiro);
        assert_eq!(app.jogo().score, pontos);
    }

    #[test]
    fn partida_acabada_trava_o_tabuleiro() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        app.jogo_mut().ended = true;
        arrastar(&mut app, a, b);
        assert!(!app.ocupado());
        assert_eq!(app.jogo().score, 0);
        tocar_casa(&mut app, a);
        assert_eq!(app.gestos().selecionada(), None);
    }

    #[test]
    fn registro_tem_o_formato_esperado() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        terminar(&mut app);
        let jogo = app.jogo();
        let esperado = vec![
            "PRISMA_M1 pronto modo=zen".to_string(),
            format!("PRISMA_M1 jogada valida=true pontos={} nivel={}", jogo.score, jogo.level),
            "PRISMA_M1 animacao_terminou".to_string(),
        ];
        assert_eq!(app.registro(), esperado);
        assert!(app.registro().is_empty(), "o registro é esvaziado ao ser lido");
    }

    #[test]
    fn avancar_sem_animacao_nao_registra_nada() {
        let mut app = App::com_sementes(7, 9);
        app.registro();
        app.avancar(16.0);
        assert!(app.registro().is_empty());
    }
}
