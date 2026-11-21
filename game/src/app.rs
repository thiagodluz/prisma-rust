//! O estado do jogo na tela: as duas partidas (Zen e Clássico), os gestos do dedo, os botões
//! e a animação em andamento.
//!
//! O motor resolve a jogada inteira de uma vez; este módulo guarda o resultado como uma
//! linha do tempo (`linha`) e a tela vai mostrando passo a passo. Enquanto a linha do tempo
//! roda, o painel mostra os pontos "de antes" e só sobe quando cada limpeza acaba, por isso
//! existe o [`Estagio`].

use crate::input::{Acao, Evento, Gestos};
use crate::layout::{Rect, TelaJogo};
use crate::linha::{montar, Passo, Reprodutor};
use prisma_engine::{lcg_random, level_goal, Game, Mode};

/// Quanto tempo a Dica fica destacada.
const DICA_MS: f32 = 2600.0;
/// Quanto tempo o "+250" fica na tela depois de uma limpeza.
const GANHO_MS: f32 = 1000.0;
/// Quanto tempo ficam os avisos ("Supernova", "Tabuleiro reorganizado").
const AVISO_MS: f32 = 1600.0;

/// Os números que o painel mostra AGORA (que podem estar atrás do motor durante a animação).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Estagio {
    pub pontos: u64,
    pub nivel: u32,
    /// Pontos que o jogador tinha quando o nível atual começou (a barra de progresso usa).
    pub inicio_do_nivel: u64,
}

/// Cada coisa que se pode apertar na tela.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Botao {
    Zen,
    Classico,
    Embaralhar,
    Dica,
    NovoJogo,
    JogarNovamente,
    Cancelar,
    Confirmar,
}

/// O texto de um número com ponto de milhar, como no Brasil: `1234567` vira `"1.234.567"`.
pub fn formatar_numero(numero: u64) -> String {
    let digitos = numero.to_string();
    let mut texto = String::with_capacity(digitos.len() + digitos.len() / 3);
    for (posicao, digito) in digitos.chars().enumerate() {
        // Põe um ponto antes de cada grupo de três dígitos (contando da direita).
        if posicao > 0 && (digitos.len() - posicao) % 3 == 0 {
            texto.push('.');
        }
        texto.push(digito);
    }
    texto
}

/// Tudo o que a tela escreve ou decide mostrar, já em texto. É montado a cada chamada.
#[derive(Clone, Debug, PartialEq)]
pub struct Painel {
    pub pontos: String,
    pub nivel: String,
    pub melhor: String,
    /// A barra de progresso do nível, de 0 a 1.
    pub progresso: f32,
    /// "{ganho} / {meta} para o próximo nível".
    pub rotulo: String,
    /// "+250", por um tempo depois de cada limpeza.
    pub ganho: Option<String>,
    /// "Tabuleiro reorganizado" ou "Supernova", por um tempo.
    pub aviso: Option<String>,
    /// (nível, pontos) quando o cartão de "Fim de partida" aparece.
    pub fim: Option<(String, String)>,
    /// A pergunta "Começar novo jogo?" está aberta.
    pub dialogo: bool,
    /// O botão Embaralhar só existe no Zen.
    pub embaralhar_visivel: bool,
    /// A Dica não funciona com a partida acabada.
    pub dica_ativa: bool,
}

/// Um valor que fica na tela por um tempo e some sozinho.
struct Temporizado<T> {
    valor: T,
    restante_ms: f32,
}

impl<T> Temporizado<T> {
    fn por(valor: T, ms: f32) -> Option<Temporizado<T>> {
        Some(Temporizado { valor, restante_ms: ms })
    }
}

/// Passa `dt_ms` no relógio de um valor temporizado; quando o tempo acaba, ele some.
fn passar_tempo<T>(item: &mut Option<Temporizado<T>>, dt_ms: f32) {
    if let Some(temporizado) = item {
        temporizado.restante_ms -= dt_ms;
        if temporizado.restante_ms <= 0.0 {
            *item = None;
        }
    }
}

/// Uma animação em andamento e os números de antes da jogada (de onde o painel parte).
struct EmAndamento {
    reprodutor: Reprodutor,
    base: Estagio,
    /// Aviso a mostrar quando a animação acabar (o embaralhar do botão usa).
    aviso_ao_fim: Option<&'static str>,
    /// Quantos passos já começaram e quantos já acabaram: para reagir a cada um uma só vez.
    iniciados: usize,
    concluidos_vistos: usize,
}

impl EmAndamento {
    fn nova(reprodutor: Reprodutor, base: Estagio, aviso_ao_fim: Option<&'static str>) -> EmAndamento {
        EmAndamento { reprodutor, base, aviso_ao_fim, iniciados: 0, concluidos_vistos: 0 }
    }
}

/// O nome do modo nas linhas de registro.
fn nome_do_modo(modo: Mode) -> &'static str {
    match modo {
        Mode::Zen => "zen",
        Mode::Classic => "classico",
    }
}

/// A posição do modo nas listas de duas casas (Zen, Clássico).
fn posicao_do_modo(modo: Mode) -> usize {
    match modo {
        Mode::Zen => 0,
        Mode::Classic => 1,
    }
}

pub struct App {
    zen: Game,
    classico: Game,
    modo: Mode,
    gestos: Gestos,
    animacao: Option<EmAndamento>,
    dica: Option<Temporizado<(usize, usize)>>,
    ganho: Option<Temporizado<u64>>,
    aviso: Option<Temporizado<&'static str>>,
    /// A pergunta "Começar novo jogo?" está aberta.
    dialogo: bool,
    /// O maior placar visto em cada modo (Zen, Clássico), nesta sessão.
    melhor: [u64; 2],
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
            dica: None,
            ganho: None,
            aviso: None,
            dialogo: false,
            melhor: [0, 0],
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
    /// partida...). O jogo de verdade só muda por `tocar` e `apertar`.
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

    /// O par de casas que a Dica está destacando agora.
    pub fn dica(&self) -> Option<(usize, usize)> {
        self.dica.as_ref().map(|dica| dica.valor)
    }

    /// O tabuleiro não aceita jogadas durante a animação, com a pergunta aberta (que "tampa"
    /// o tabuleiro) nem depois do fim da partida.
    fn bloqueado(&self) -> bool {
        self.ocupado() || self.dialogo || self.jogo().ended
    }

    // ---- Toques e botões ---------------------------------------------------------------

    /// Um evento do dedo (ou do mouse). O botão responde assim que o dedo encosta; o
    /// tabuleiro, quando o dedo solta (ver `input`).
    pub fn tocar(&mut self, evento: Evento, tela: &TelaJogo) {
        if let Evento::Baixo { x, y } = evento {
            if let Some(botao) = self.botao_em(tela, x, y) {
                self.apertar(botao);
                return;
            }
            // Qualquer toque no tabuleiro apaga a Dica.
            if tela.tabuleiro.contem(x, y) {
                self.dica = None;
            }
        }
        let bloqueado = self.bloqueado();
        if let Acao::Trocar(a, b) = self.gestos.tratar(evento, tela.tabuleiro, bloqueado) {
            self.jogar(a, b);
        }
    }

    /// Os botões que existem agora, com o lugar de cada um na tela.
    fn botoes_visiveis(&self, tela: &TelaJogo) -> Vec<(Botao, Rect)> {
        // Com a pergunta aberta, só existem os dois botões dela.
        if self.dialogo {
            return vec![(Botao::Cancelar, tela.dialogo_cancelar), (Botao::Confirmar, tela.dialogo_confirmar)];
        }
        let mut botoes = vec![(Botao::Zen, tela.aba_zen), (Botao::Classico, tela.aba_classico)];
        if self.jogo().ended {
            // Com a partida acabada: o cartão tem "Jogar novamente"; Dica e Embaralhar somem.
            botoes.push((Botao::JogarNovamente, tela.botao_jogar_novamente));
        } else {
            if self.modo == Mode::Zen {
                if let Some(retangulo) = tela.botao_embaralhar {
                    botoes.push((Botao::Embaralhar, retangulo));
                }
            }
            botoes.push((Botao::Dica, tela.botao_dica));
        }
        botoes.push((Botao::NovoJogo, tela.botao_novo_jogo));
        botoes
    }

    /// Qual botão existe no ponto (`x`, `y`)? Só conta quem está visível agora.
    pub fn botao_em(&self, tela: &TelaJogo, x: f32, y: f32) -> Option<Botao> {
        self.botoes_visiveis(tela)
            .into_iter()
            .find(|(_, retangulo)| retangulo.contem(x, y))
            .map(|(botao, _)| botao)
    }

    /// Faz o que o botão pede (se for permitido agora).
    pub fn apertar(&mut self, botao: Botao) {
        // Com a pergunta aberta, só os botões dela valem.
        if self.dialogo && !matches!(botao, Botao::Cancelar | Botao::Confirmar) {
            return;
        }
        match botao {
            Botao::Zen => self.trocar_de_modo(Mode::Zen),
            Botao::Classico => self.trocar_de_modo(Mode::Classic),
            Botao::Embaralhar => self.embaralhar(),
            Botao::Dica => self.pedir_dica(),
            Botao::NovoJogo => self.perguntar_novo_jogo(),
            Botao::JogarNovamente => {
                if self.jogo().ended {
                    self.perguntar_novo_jogo();
                }
            }
            Botao::Cancelar => self.dialogo = false,
            Botao::Confirmar => self.confirmar_novo_jogo(),
        }
    }

    fn pedir_dica(&mut self) {
        if self.ocupado() || self.jogo().ended {
            return;
        }
        if let Some((a, b)) = self.jogo_mut().hint() {
            self.dica = Temporizado::por((a, b), DICA_MS);
            self.registros.push(format!("PRISMA_M1 dica a={a} b={b}"));
        }
    }

    fn embaralhar(&mut self) {
        if self.modo != Mode::Zen || self.ocupado() || self.jogo().ended {
            return;
        }
        let base = self.estagio();
        let jogo = self.jogo_mut();
        let antes = jogo.board;
        // `false` = embaralhar pedido pelo jogador (não é o automático do motor).
        if !jogo.shuffle(false) {
            return;
        }
        let depois = jogo.board;
        self.dica = None;
        self.gestos.limpar();
        let reprodutor = Reprodutor::novo(vec![Passo::Embaralhar { antes, depois }]);
        self.animacao = Some(EmAndamento::nova(reprodutor, base, Some("Tabuleiro reorganizado")));
        self.observar();
        self.encerrar_se_terminou();
    }

    fn trocar_de_modo(&mut self, modo: Mode) {
        if self.ocupado() || self.modo == modo {
            return;
        }
        self.registrar_melhor();
        self.limpar_tela();
        self.modo = modo;
        self.registrar_melhor();
        self.registros.push(format!("PRISMA_M1 modo={} pontos={}", nome_do_modo(modo), self.jogo().score));
    }

    fn perguntar_novo_jogo(&mut self) {
        if self.ocupado() {
            return;
        }
        self.gestos.limpar();
        self.dialogo = true;
    }

    fn confirmar_novo_jogo(&mut self) {
        if !self.dialogo {
            return;
        }
        self.dialogo = false;
        // O placar da partida que acaba aqui conta para o recorde.
        self.registrar_melhor();
        self.jogo_mut().restart();
        self.limpar_tela();
        self.registros.push(format!("PRISMA_M1 novo_jogo modo={} pontos=0", nome_do_modo(self.modo)));
    }

    /// Esquece o que é passageiro: gesto, Dica, "+250" e avisos.
    fn limpar_tela(&mut self) {
        self.gestos.limpar();
        self.dica = None;
        self.ganho = None;
        self.aviso = None;
    }

    /// Guarda o placar atual do modo se ele for o maior já visto.
    fn registrar_melhor(&mut self) {
        let posicao = posicao_do_modo(self.modo);
        self.melhor[posicao] = self.melhor[posicao].max(self.jogo().score);
    }

    // ---- Jogadas e animação ------------------------------------------------------------

    /// Joga a troca no motor (que resolve tudo de uma vez) e prepara a animação.
    fn jogar(&mut self, a: usize, b: usize) {
        let base = self.estagio();
        self.dica = None;
        let jogo = self.jogo_mut();
        let antes = jogo.board;
        let resultado = jogo.play(a, b);
        let (pontos, nivel) = (jogo.score, jogo.level);
        let passos = montar(antes, a, b, &resultado, nivel);
        self.registros.push(format!("PRISMA_M1 jogada valida={} pontos={} nivel={}", resultado.valid, pontos, nivel));
        self.animacao = Some(EmAndamento::nova(Reprodutor::novo(passos), base, None));
        self.observar();
        self.encerrar_se_terminou();
    }

    /// Deixa passar `dt_ms` milissegundos: da animação e dos relógios de Dica, "+250" e avisos.
    /// Tempo inválido conta como zero; tempo enorme termina tudo de uma vez.
    pub fn avancar(&mut self, dt_ms: f32) {
        let dt = if dt_ms.is_nan() || dt_ms < 0.0 { 0.0 } else { dt_ms };
        passar_tempo(&mut self.dica, dt);
        passar_tempo(&mut self.ganho, dt);
        passar_tempo(&mut self.aviso, dt);
        if let Some(andamento) = self.animacao.as_mut() {
            andamento.reprodutor.avancar(dt);
        }
        self.observar();
        self.encerrar_se_terminou();
    }

    /// Reage aos passos que começaram ou acabaram desde a última vez: a limpeza de Supernova
    /// mostra o aviso quando COMEÇA; qualquer limpeza mostra o "+ganho" quando ACABA.
    fn observar(&mut self) {
        let Some(andamento) = self.animacao.as_mut() else { return };
        let reprodutor = &andamento.reprodutor;
        let concluidos = reprodutor.concluidos();
        let atual = reprodutor.atual().map(|(passo, _)| passo);
        let iniciados_agora = concluidos.len() + usize::from(atual.is_some());

        let mut novo_aviso = None;
        for indice in andamento.iniciados..iniciados_agora {
            let passo = concluidos.get(indice).or(atual);
            if let Some(Passo::Limpeza { supernova: true, .. }) = passo {
                novo_aviso = Some("Supernova");
            }
        }
        let mut novo_ganho = None;
        for passo in &concluidos[andamento.concluidos_vistos.min(concluidos.len())..] {
            if let Passo::Limpeza { ganho, .. } = passo {
                novo_ganho = Some(*ganho);
            }
        }
        andamento.iniciados = iniciados_agora;
        andamento.concluidos_vistos = concluidos.len();

        if let Some(texto) = novo_aviso {
            self.aviso = Temporizado::por(texto, AVISO_MS);
        }
        if let Some(ganho) = novo_ganho {
            self.ganho = Temporizado::por(ganho, GANHO_MS);
        }
    }

    /// Se a animação acabou: libera a entrada, esquece o gesto e avisa no registro.
    fn encerrar_se_terminou(&mut self) {
        if !self.animacao.as_ref().is_some_and(|andamento| andamento.reprodutor.terminou()) {
            return;
        }
        let aviso_ao_fim = self.animacao.take().and_then(|andamento| andamento.aviso_ao_fim);
        self.gestos.limpar();
        if let Some(texto) = aviso_ao_fim {
            self.aviso = Temporizado::por(texto, AVISO_MS);
        }
        self.registros.push("PRISMA_M1 animacao_terminou".to_string());
        if self.jogo().ended {
            let jogo = self.jogo();
            let linha = format!("PRISMA_M1 fim pontos={} nivel={}", jogo.score, jogo.level);
            self.registros.push(linha);
        }
    }

    /// O passo da animação em andamento e há quantos milissegundos ele começou.
    pub fn passo_atual(&self) -> Option<(&Passo, f32)> {
        self.animacao.as_ref().and_then(|andamento| andamento.reprodutor.atual())
    }

    // ---- O que a tela mostra -----------------------------------------------------------

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

    /// Os textos e as decisões de tela, calculados na hora (nada fica guardado).
    pub fn painel(&self) -> Painel {
        let estagio = self.estagio();
        let jogo = self.jogo();
        let meta = level_goal(estagio.nivel);
        let no_nivel = estagio.pontos.saturating_sub(estagio.inicio_do_nivel);
        // Parado, o recorde já conta o placar atual; ele só precisa ser guardado antes de o placar
        // sumir (novo jogo) ou de o modo sair de cena. Durante a animação o recorde não "adianta"
        // os pontos que o painel ainda vai mostrar.
        let melhor = self.melhor[posicao_do_modo(self.modo)];
        let melhor = if self.ocupado() { melhor } else { melhor.max(jogo.score) };
        Painel {
            pontos: formatar_numero(estagio.pontos),
            nivel: estagio.nivel.to_string(),
            melhor: formatar_numero(melhor),
            progresso: (no_nivel as f32 / meta as f32).clamp(0.0, 1.0),
            rotulo: format!("{} / {} para o próximo nível", formatar_numero(no_nivel), formatar_numero(meta)),
            ganho: self.ganho.as_ref().map(|ganho| format!("+{}", formatar_numero(ganho.valor))),
            aviso: self.aviso.as_ref().map(|aviso| aviso.valor.to_string()),
            fim: (jogo.ended && !self.ocupado()).then(|| (jogo.level.to_string(), formatar_numero(jogo.score))),
            dialogo: self.dialogo,
            embaralhar_visivel: self.modo == Mode::Zen,
            dica_ativa: !jogo.ended,
        }
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
    use crate::layout::{cell_rect, tela_jogo, Rect, TelaJogo};
    use crate::linha::{Passo, Reprodutor};
    use prisma_engine::{adjacent, lcg_random, level_goal, Board, Game, Mode, SIZE};

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
    // ---- Task 5: botões, painel, dica, diálogo e fim de partida ----------------------------

    fn meio(retangulo: Rect) -> (f32, f32) {
        (retangulo.x + retangulo.w / 2.0, retangulo.y + retangulo.h / 2.0)
    }

    /// Aperta um botão como o dedo faria: encosta no meio dele e solta.
    fn apertar_na_tela(app: &mut App, botao: Rect) {
        let tela = tela();
        let (x, y) = meio(botao);
        app.tocar(Evento::Baixo { x, y }, &tela);
        app.tocar(Evento::Cima { x, y }, &tela);
    }

    /// Os ids das pedras do tabuleiro, em ordem de id (para comparar "as mesmas pedras").
    fn ids(tabuleiro: &Board) -> Vec<u32> {
        let mut ids: Vec<u32> = tabuleiro.iter().flatten().map(|pedra| pedra.id).collect();
        ids.sort_unstable();
        ids
    }

    /// Os ganhos das limpezas que já acabaram na animação em andamento.
    fn ganhos_concluidos(app: &App) -> Vec<u64> {
        let Some(andamento) = &app.animacao else { return Vec::new() };
        andamento
            .reprodutor
            .concluidos()
            .iter()
            .filter_map(|passo| if let Passo::Limpeza { ganho, .. } = passo { Some(*ganho) } else { None })
            .collect()
    }

    /// Põe no app uma animação montada à mão (para testar o que o motor não produz facilmente).
    fn injetar(app: &mut App, passos: Vec<Passo>) {
        let base = app.estagio();
        app.animacao = Some(EmAndamento::nova(Reprodutor::novo(passos), base, None));
        app.avancar(0.0);
    }

    #[test]
    fn formatar_numero_usa_pontos_de_milhar() {
        for (numero, texto) in [
            (0u64, "0"),
            (999, "999"),
            (1000, "1.000"),
            (1800, "1.800"),
            (12450, "12.450"),
            (100_000, "100.000"),
            (1_234_567, "1.234.567"),
        ] {
            assert_eq!(formatar_numero(numero), texto);
        }
        assert!(formatar_numero(u64::MAX).ends_with(".615"));
        assert_eq!(formatar_numero(u64::MAX), "18.446.744.073.709.551.615");
    }

    #[test]
    fn painel_inicial_do_zen() {
        let app = App::com_sementes(7, 9);
        let painel = app.painel();
        assert_eq!(painel.pontos, "0");
        assert_eq!(painel.nivel, "1");
        assert_eq!(painel.melhor, "0");
        assert_eq!(painel.rotulo, "0 / 1.800 para o próximo nível");
        assert_eq!(painel.progresso, 0.0);
        assert!(painel.embaralhar_visivel);
        assert!(painel.dica_ativa);
        assert_eq!(painel.fim, None);
        assert!(!painel.dialogo);
        assert_eq!(painel.ganho, None);
        assert_eq!(painel.aviso, None);
    }

    #[test]
    fn progresso_e_limitado_e_acompanha_o_estagio() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        for _ in 0..10_000 {
            if !app.ocupado() {
                break;
            }
            let estagio = app.estagio();
            let painel = app.painel();
            assert_eq!(painel.pontos, formatar_numero(estagio.pontos));
            assert_eq!(painel.nivel, estagio.nivel.to_string());
            let no_nivel = estagio.pontos - estagio.inicio_do_nivel;
            assert_eq!(
                painel.rotulo,
                format!("{} / {} para o próximo nível", formatar_numero(no_nivel), formatar_numero(level_goal(estagio.nivel)))
            );
            assert!((0.0..=1.0).contains(&painel.progresso));
            app.avancar(16.0);
        }
        assert!((0.0..=1.0).contains(&app.painel().progresso));
        // Metade da meta é metade da barra; passar da meta não passa de 1.
        app.jogo_mut().score = 900;
        assert_eq!(app.painel().progresso, 0.5);
        app.jogo_mut().score = 5000;
        assert_eq!(app.painel().progresso, 1.0);
    }

    #[test]
    fn dica_destaca_por_2600ms_e_some() {
        let mut app = App::com_sementes(7, 9);
        let par = app.jogo_mut().hint().expect("há jogada");
        app.registro();
        app.apertar(Botao::Dica);
        assert_eq!(app.dica(), Some(par));
        app.avancar(2599.0);
        assert_eq!(app.dica(), Some(par));
        app.avancar(2.0);
        assert_eq!(app.dica(), None);
        assert!(app.registro().contains(&format!("PRISMA_M1 dica a={} b={}", par.0, par.1)));

        // Um toque no tabuleiro apaga.
        app.apertar(Botao::Dica);
        assert!(app.dica().is_some());
        tocar_casa(&mut app, 0);
        assert_eq!(app.dica(), None);

        // O botão também responde ao toque na tela.
        let tela = tela();
        apertar_na_tela(&mut app, tela.botao_dica);
        assert_eq!(app.dica(), Some(par));

        // Trocar de modo apaga.
        app.apertar(Botao::Classico);
        assert_eq!(app.dica(), None);
    }

    #[test]
    fn dica_sem_jogada_ou_partida_acabada_nao_faz_nada() {
        let mut app = App::com_sementes(7, 9);
        app.jogo_mut().ended = true;
        app.apertar(Botao::Dica);
        assert_eq!(app.dica(), None);
        assert!(!app.painel().dica_ativa);
        // E o botão nem existe mais na tela.
        let tela = tela();
        let (x, y) = meio(tela.botao_dica);
        assert_eq!(app.botao_em(&tela, x, y), None);
    }

    #[test]
    fn dica_durante_a_animacao_e_ignorada() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        app.apertar(Botao::Dica);
        assert_eq!(app.dica(), None);
    }

    #[test]
    fn embaralhar_so_no_zen_e_so_sem_animacao() {
        let mut app = App::com_sementes(7, 9);
        let antes = app.jogo().board;
        app.apertar(Botao::Embaralhar);
        assert!(app.ocupado());
        assert_eq!(app.passo_atual().map(|(passo, _)| nome(passo)), Some("embaralhar"));
        // O motor já embaralhou: as mesmas pedras (mesmos ids), em outra ordem.
        let depois = app.jogo().board;
        assert_ne!(antes, depois);
        assert_eq!(ids(&antes), ids(&depois));
        assert_eq!(app.painel().aviso, None, "o aviso só aparece quando a animação acaba");
        app.avancar(439.0);
        assert!(app.ocupado(), "embaralhar dura 190 + 250 ms");
        app.avancar(2.0);
        assert!(!app.ocupado());
        assert_eq!(app.painel().aviso.as_deref(), Some("Tabuleiro reorganizado"));
        app.avancar(1599.0);
        assert!(app.painel().aviso.is_some());
        app.avancar(2.0);
        assert_eq!(app.painel().aviso, None);

        // No Clássico não existe embaralhar.
        app.apertar(Botao::Classico);
        let classico = app.jogo().board;
        app.apertar(Botao::Embaralhar);
        assert!(!app.ocupado());
        assert_eq!(app.jogo().board, classico);
        assert!(!app.painel().embaralhar_visivel);

        // Durante uma animação, é ignorado.
        let mut outro = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut outro);
        arrastar(&mut outro, a, b);
        let tabuleiro = outro.jogo().board;
        outro.apertar(Botao::Embaralhar);
        assert_eq!(outro.jogo().board, tabuleiro);
    }

    #[test]
    fn cada_modo_guarda_a_sua_partida() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        app.apertar(Botao::Classico);
        assert_eq!(app.modo(), Mode::Zen, "durante a animação a troca de modo é ignorada");
        terminar(&mut app);
        let pontos = app.jogo().score;
        assert!(pontos > 0);
        app.registro();

        app.apertar(Botao::Zen);
        assert!(app.registro().is_empty(), "trocar para o mesmo modo não faz nada");

        app.apertar(Botao::Classico);
        assert_eq!(app.modo(), Mode::Classic);
        assert_eq!(app.jogo().mode, Mode::Classic);
        assert_eq!(app.jogo().score, 0);
        assert_eq!(app.painel().pontos, "0");
        app.apertar(Botao::Zen);
        assert_eq!(app.jogo().score, pontos, "o Zen continua de onde parou");
        assert_eq!(
            app.registro(),
            vec!["PRISMA_M1 modo=classico pontos=0".to_string(), format!("PRISMA_M1 modo=zen pontos={pontos}")]
        );
    }

    #[test]
    fn trocar_de_modo_apaga_dica_gesto_ganho_e_aviso() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        terminar(&mut app);
        assert!(app.painel().ganho.is_some());
        app.apertar(Botao::Embaralhar);
        terminar(&mut app);
        assert!(app.painel().aviso.is_some());
        app.apertar(Botao::Dica);
        tocar_casa(&mut app, 0);
        app.apertar(Botao::Dica);
        assert!(app.dica().is_some() && app.gestos().selecionada().is_some());

        app.apertar(Botao::Classico);
        assert_eq!(app.dica(), None);
        assert_eq!(app.gestos().selecionada(), None);
        assert_eq!(app.painel().ganho, None);
        assert_eq!(app.painel().aviso, None);
    }

    #[test]
    fn melhor_guarda_o_maior_placar_de_cada_modo() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        terminar(&mut app);
        let pontos = app.jogo().score;
        assert_eq!(app.painel().melhor, formatar_numero(pontos));
        app.apertar(Botao::NovoJogo);
        app.apertar(Botao::Confirmar);
        let painel = app.painel();
        assert_eq!(painel.pontos, "0");
        assert_eq!(painel.melhor, formatar_numero(pontos), "o recorde sobrevive ao novo jogo");
        app.apertar(Botao::Classico);
        assert_eq!(app.painel().melhor, "0", "cada modo tem o seu recorde");
    }

    #[test]
    fn novo_jogo_pede_confirmacao() {
        let mut app = App::com_sementes(7, 9);
        let tela = tela();
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        terminar(&mut app);
        let (pontos, tabuleiro) = (app.jogo().score, app.jogo().board);
        assert!(pontos > 0);

        apertar_na_tela(&mut app, tela.botao_novo_jogo);
        assert!(app.painel().dialogo);
        // Com o diálogo aberto o tabuleiro, a Dica e as abas não respondem.
        arrastar(&mut app, 0, 1);
        assert!(!app.ocupado());
        apertar_na_tela(&mut app, tela.botao_dica);
        assert_eq!(app.dica(), None);
        app.apertar(Botao::Classico);
        assert_eq!(app.modo(), Mode::Zen);

        // Cancelar fecha e nada muda.
        apertar_na_tela(&mut app, tela.dialogo_cancelar);
        assert!(!app.painel().dialogo);
        assert_eq!(app.jogo().score, pontos);
        assert_eq!(app.jogo().board, tabuleiro);

        // Confirmar começa outra partida.
        apertar_na_tela(&mut app, tela.botao_novo_jogo);
        app.registro();
        apertar_na_tela(&mut app, tela.dialogo_confirmar);
        assert!(!app.painel().dialogo);
        assert_eq!(app.jogo().score, 0);
        assert_eq!(app.jogo().level, 1);
        assert_eq!(app.registro(), vec!["PRISMA_M1 novo_jogo modo=zen pontos=0".to_string()]);
    }

    #[test]
    fn novo_jogo_durante_a_animacao_nao_abre_o_dialogo() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        app.apertar(Botao::NovoJogo);
        assert!(!app.painel().dialogo);
        // Confirmar sem o diálogo aberto também não faz nada.
        app.apertar(Botao::Confirmar);
        assert!(app.jogo().score > 0);
    }

    #[test]
    fn fim_de_partida_no_classico() {
        let mut app = App::com_sementes(7, 9);
        app.apertar(Botao::Classico);
        {
            let jogo = app.jogo_mut();
            jogo.ended = true;
            jogo.score = 4321;
            jogo.level = 3;
        }
        assert_eq!(app.painel().fim, Some(("3".to_string(), "4.321".to_string())));
        // O arrasto não joga.
        arrastar(&mut app, 0, 1);
        assert!(!app.ocupado());
        assert_eq!(app.jogo().score, 4321);

        // Jogar novamente pergunta; cancelar mantém o cartão.
        app.apertar(Botao::JogarNovamente);
        assert!(app.painel().dialogo);
        app.apertar(Botao::Cancelar);
        assert!(!app.painel().dialogo);
        assert!(app.painel().fim.is_some());

        // Confirmar começa outra partida, sem o cartão.
        app.apertar(Botao::JogarNovamente);
        app.apertar(Botao::Confirmar);
        assert_eq!(app.painel().fim, None);
        assert!(!app.jogo().ended);
        assert_eq!(app.jogo().score, 0);
        assert_eq!(app.painel().melhor, "4.321", "o recorde da partida acabada fica");
    }

    #[test]
    fn classico_sem_jogadas_termina_pelo_motor() {
        let mut app = App::com_sementes(7, 9);
        app.apertar(Botao::Classico);
        let (a, b) = boa_jogada(&mut app);
        app.jogo_mut().force_no_moves = true;
        arrastar(&mut app, a, b);
        assert_eq!(app.painel().fim, None, "o cartão só aparece depois da animação");
        terminar(&mut app);
        assert!(app.jogo().ended);
        assert!(app.painel().fim.is_some());
        let (pontos, nivel) = (app.jogo().score, app.jogo().level);
        arrastar(&mut app, 0, 1);
        assert!(!app.ocupado());
        assert_eq!(app.jogo().score, pontos);
        assert!(app.registro().contains(&format!("PRISMA_M1 fim pontos={pontos} nivel={nivel}")));
    }

    #[test]
    fn zen_sem_jogadas_mostra_o_resgate() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        app.jogo_mut().force_no_moves = true;
        arrastar(&mut app, a, b);
        let mut vistos: Vec<&str> = Vec::new();
        for _ in 0..10_000 {
            if let Some((passo, _)) = app.passo_atual() {
                if vistos.last() != Some(&nome(passo)) {
                    vistos.push(nome(passo));
                }
            }
            if !app.ocupado() {
                break;
            }
            app.avancar(16.0);
        }
        assert!(vistos.contains(&"resgate"), "passos vistos: {vistos:?}");
        assert!(!app.jogo().ended, "o Zen nunca termina");
        assert!(!app.ocupado());
        // A entrada voltou: dá para jogar de novo.
        let (c, d) = boa_jogada(&mut app);
        arrastar(&mut app, c, d);
        assert!(app.ocupado());
    }

    #[test]
    fn botao_em_acha_so_o_botao_visivel() {
        let tela = tela();
        let mut app = App::com_sementes(7, 9);
        let embaralhar = tela.botao_embaralhar.expect("a tela de teste tem o botão");
        let (x, y) = meio(embaralhar);
        assert_eq!(app.botao_em(&tela, x, y), Some(Botao::Embaralhar));
        assert_eq!(app.botao_em(&tela, meio(tela.botao_dica).0, meio(tela.botao_dica).1), Some(Botao::Dica));
        assert_eq!(app.botao_em(&tela, meio(tela.botao_novo_jogo).0, meio(tela.botao_novo_jogo).1), Some(Botao::NovoJogo));
        assert_eq!(app.botao_em(&tela, meio(tela.aba_zen).0, meio(tela.aba_zen).1), Some(Botao::Zen));
        assert_eq!(app.botao_em(&tela, meio(tela.aba_classico).0, meio(tela.aba_classico).1), Some(Botao::Classico));
        // Na borda exata: esquerda e topo contam, direita e baixo não.
        assert_eq!(app.botao_em(&tela, embaralhar.x, embaralhar.y), Some(Botao::Embaralhar));
        assert_eq!(app.botao_em(&tela, embaralhar.x + embaralhar.w, embaralhar.y), None);
        assert_eq!(app.botao_em(&tela, embaralhar.x, embaralhar.y + embaralhar.h), None);
        // Fora de qualquer botão (no meio do tabuleiro).
        let (tx, ty) = meio(tela.tabuleiro);
        assert_eq!(app.botao_em(&tela, tx, ty), None);

        // No Clássico não há Embaralhar.
        app.apertar(Botao::Classico);
        assert_eq!(app.botao_em(&tela, x, y), None);

        // Com o diálogo, só Cancelar e Confirmar.
        app.apertar(Botao::NovoJogo);
        assert_eq!(app.botao_em(&tela, meio(tela.dialogo_cancelar).0, meio(tela.dialogo_cancelar).1), Some(Botao::Cancelar));
        assert_eq!(app.botao_em(&tela, meio(tela.dialogo_confirmar).0, meio(tela.dialogo_confirmar).1), Some(Botao::Confirmar));
        assert_eq!(app.botao_em(&tela, meio(tela.botao_dica).0, meio(tela.botao_dica).1), None);
        assert_eq!(app.botao_em(&tela, meio(tela.aba_zen).0, meio(tela.aba_zen).1), None);
        app.apertar(Botao::Cancelar);

        // Com a partida acabada: Jogar novamente, Novo jogo e as abas; sem Dica.
        app.jogo_mut().ended = true;
        assert_eq!(
            app.botao_em(&tela, meio(tela.botao_jogar_novamente).0, meio(tela.botao_jogar_novamente).1),
            Some(Botao::JogarNovamente)
        );
        assert_eq!(app.botao_em(&tela, meio(tela.botao_novo_jogo).0, meio(tela.botao_novo_jogo).1), Some(Botao::NovoJogo));
        assert_eq!(app.botao_em(&tela, meio(tela.aba_zen).0, meio(tela.aba_zen).1), Some(Botao::Zen));
        assert_eq!(app.botao_em(&tela, meio(tela.botao_dica).0, meio(tela.botao_dica).1), None);
    }

    #[test]
    fn ganho_aparece_depois_da_limpeza_e_some() {
        // Numa jogada de verdade: logo depois da primeira limpeza concluída.
        let mut app = App::com_sementes(7, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        assert_eq!(app.painel().ganho, None);
        for _ in 0..10_000 {
            app.avancar(16.0);
            if let Some(primeiro) = ganhos_concluidos(&app).first() {
                assert_eq!(app.painel().ganho, Some(format!("+{}", formatar_numero(*primeiro))));
                break;
            }
            assert!(app.ocupado(), "a jogada boa tem pelo menos uma limpeza");
        }

        // Montada à mão: só uma limpeza de 250 pontos; o "+250" some em 1 s.
        let mut app = App::com_sementes(7, 9);
        let tabuleiro = app.jogo().board;
        injetar(&mut app, vec![Passo::Limpeza { tabuleiro, celulas: vec![0], ativadas: vec![], supernova: false, ganho: 250 }]);
        assert_eq!(app.painel().ganho, None, "só depois que a limpeza acaba");
        assert_eq!(app.painel().aviso, None, "limpeza comum não tem aviso");
        app.avancar(300.0);
        assert!(!app.ocupado());
        assert_eq!(app.painel().ganho.as_deref(), Some("+250"));
        app.avancar(999.0);
        assert!(app.painel().ganho.is_some());
        app.avancar(2.0);
        assert_eq!(app.painel().ganho, None);
    }

    #[test]
    fn supernova_mostra_o_aviso_quando_a_limpeza_comeca() {
        let mut app = App::com_sementes(7, 9);
        let tabuleiro = app.jogo().board;
        injetar(
            &mut app,
            vec![Passo::Limpeza { tabuleiro, celulas: vec![0], ativadas: vec![0], supernova: true, ganho: 250 }],
        );
        assert!(app.ocupado());
        assert_eq!(app.painel().aviso.as_deref(), Some("Supernova"), "o aviso aparece no começo da limpeza");
        assert_eq!(app.painel().ganho, None);
        app.avancar(400.0);
        assert_eq!(app.painel().ganho.as_deref(), Some("+250"));
        assert!(app.painel().aviso.is_some());
        app.avancar(1199.0);
        assert!(app.painel().aviso.is_some());
        app.avancar(2.0);
        assert_eq!(app.painel().aviso, None, "o aviso dura 1,6 s");
    }

}
