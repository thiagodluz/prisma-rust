//! O estado do jogo na tela: as duas partidas (Zen e Clássico), os gestos do dedo, os botões
//! e a animação em andamento.
//!
//! O motor resolve a jogada inteira de uma vez; este módulo guarda o resultado como uma
//! linha do tempo (`linha`) e a tela vai mostrando passo a passo. Enquanto a linha do tempo
//! roda, o painel mostra os pontos "de antes" e só sobe quando cada limpeza acaba, por isso
//! existe o [`Estagio`].

use crate::efeitos::{elevacao_da_cadeia, sinal_da_limpeza, Efeito, Sinal};
use crate::formato::Recordes;
use crate::fundo::Desejado;
use crate::input::{Acao, Evento, Gestos};
use crate::layout::{Rect, TelaJogo, TelaSom};
use crate::linha::{montar, Passo, Reprodutor};
use crate::preferencias::{estado_da_respiracao, Canal, EstadoRespiracao, Preferencias};
use crate::salvar::{Origem, Salvador};
use prisma_engine::{lcg_random, level_goal, Game, Mode, TileKind, SIZE};

/// Quanto tempo a Dica fica destacada.
const DICA_MS: f32 = 2600.0;
/// Quanto tempo o "+250" fica na tela depois de uma limpeza.
const GANHO_MS: f32 = 1000.0;
/// Quanto tempo ficam os avisos ("Supernova", "Tabuleiro reorganizado").
const AVISO_MS: f32 = 1600.0;
/// O relógio do guia de respiração volta a zero a cada 40 s, que é múltiplo dos dois ciclos
/// (8 s e 10 s): a fase não muda e o número nunca cresce sem limite (um salto enorme de tempo
/// não congela o guia).
const CICLO_DA_RESPIRACAO_MS: f32 = 40_000.0;
/// O volume que "Sons do jogo" devolve quando o jogador liga os sons com o volume dos efeitos em 0.
const VOLUME_RESTAURADO: u8 = 80;
/// O que a folha diz quando o áudio de fundo não começou (o texto do JS).
const AVISO_DE_AUDIO: &str = "O áudio não começou. Toque novamente em Música ou Ambiente para tentar de novo.";
/// O que o "Testar som" diz com o volume dos efeitos em 0.
const AVISO_SEM_VOLUME: &str = "Aumente o volume dos efeitos";

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
    /// O ♪ ao lado das abas: abre e fecha a folha de Som.
    Som,
    // Daqui para baixo, só valem com a folha de Som aberta.
    FecharSom,
    SonsDoJogo,
    Testar,
    // Estes cinco só existem no Zen.
    LigaMusica,
    EscolheMusica,
    LigaAmbiente,
    EscolheAmbiente,
    Respiracao,
}

impl Botao {
    /// É um botão de dentro da folha de Som (o ♪ que abre e fecha a folha não conta)?
    fn da_folha(self) -> bool {
        !matches!(
            self,
            Botao::Zen
                | Botao::Classico
                | Botao::Embaralhar
                | Botao::Dica
                | Botao::NovoJogo
                | Botao::JogarNovamente
                | Botao::Cancelar
                | Botao::Confirmar
                | Botao::Som
        )
    }

    /// É um dos botões que só existem no modo Zen?
    fn so_do_zen(self) -> bool {
        matches!(
            self,
            Botao::LigaMusica | Botao::EscolheMusica | Botao::LigaAmbiente | Botao::EscolheAmbiente | Botao::Respiracao
        )
    }
}

/// O volume (de 0 a 100, de 5 em 5) que o dedo na posição `x` escolhe numa barra: o começo da
/// barra é 0 e o fim é 100; fora dela vale o extremo mais perto. Posição inválida: nada.
fn volume_do_dedo(barra: Rect, x: f32) -> Option<u8> {
    if !x.is_finite() || barra.w.is_nan() || barra.w <= 0.0 {
        return None;
    }
    let fracao = ((x - barra.x) / barra.w).clamp(0.0, 1.0);
    Some((fracao * 20.0).round() as u8 * 5)
}

/// A barra de volume de um canal.
fn barra_do_canal(som: &TelaSom, canal: Canal) -> Rect {
    match canal {
        Canal::Efeitos => som.barra_efeitos,
        Canal::Musica => som.barra_musica,
        Canal::Ambiente => som.barra_ambiente,
    }
}

/// O texto de um número com ponto de milhar, como no Brasil: `1234567` vira `"1.234.567"`.
pub fn formatar_numero(numero: u64) -> String {
    let digitos = numero.to_string();
    let mut texto = String::with_capacity(digitos.len() + digitos.len() / 3);
    for (posicao, digito) in digitos.chars().enumerate() {
        // Põe um ponto antes de cada grupo de três dígitos (contando da direita).
        if posicao > 0 && (digitos.len() - posicao).is_multiple_of(3) {
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
    /// O MELHOR de antes da jogada: o painel só mostra o novo quando a animação acaba.
    melhor_antes: u64,
    /// Quantos passos já começaram e quantos já acabaram: para reagir a cada um uma só vez.
    iniciados: usize,
    concluidos_vistos: usize,
}

impl EmAndamento {
    fn nova(
        reprodutor: Reprodutor,
        base: Estagio,
        aviso_ao_fim: Option<&'static str>,
        melhor_antes: u64,
    ) -> EmAndamento {
        EmAndamento { reprodutor, base, aviso_ao_fim, melhor_antes, iniciados: 0, concluidos_vistos: 0 }
    }
}

/// Se um único `avancar` deixa passar mais do que isto (o app voltando do segundo plano, um quadro
/// que travou), os passos que ele pula não tocam som: senão viria uma rajada de efeitos atrasados.
const SALTO_SEM_SOM_MS: f32 = 2000.0;

/// O efeito que toca quando um passo COMEÇA (porte do `animateResult` do JS): a jogada inválida
/// no começo da troca, cada limpeza (pelo tipo da pedra especial que disparou, ou combinação e
/// cascata), o resgate e a subida de nível. Embaralhar, queda e troca válida são mudos.
fn efeito_ao_comecar(passo: &Passo) -> Option<Efeito> {
    let sem_elevacao = |sinal| Some(Efeito { sinal, elevacao: 0 });
    match passo {
        Passo::Troca { valida: false, .. } => sem_elevacao(Sinal::Invalida),
        Passo::Limpeza { tabuleiro, ativadas, supernova, cadeia, .. } => {
            let tipos: Vec<TileKind> = ativadas
                .iter()
                .filter_map(|&casa| tabuleiro.get(casa / SIZE).and_then(|linha| linha.get(casa % SIZE)))
                .map(|pedra| pedra.kind)
                .collect();
            let sinal = sinal_da_limpeza(*supernova, &tipos, *cadeia);
            // Só a combinação e a cascata sobem de tom a cada limpeza da cadeia.
            let elevacao = if matches!(sinal, Sinal::Combinacao | Sinal::Cascata) { elevacao_da_cadeia(*cadeia) } else { 0 };
            Some(Efeito { sinal, elevacao })
        }
        Passo::Resgate { .. } => sem_elevacao(Sinal::Resgate),
        Passo::Nivel { .. } => sem_elevacao(Sinal::Nivel),
        Passo::Troca { valida: true, .. } | Passo::Queda { .. } | Passo::Embaralhar { .. } => None,
    }
}

/// O efeito que toca quando um passo ACABA: a criação de uma pedra especial, no fim da limpeza
/// que a fez nascer.
fn efeito_ao_acabar(passo: &Passo) -> Option<Efeito> {
    match passo {
        Passo::Limpeza { criou: true, .. } => Some(Efeito { sinal: Sinal::Criacao, elevacao: 0 }),
        _ => None,
    }
}

/// O nome do modo nas linhas de registro.
fn nome_do_modo(modo: Mode) -> &'static str {
    match modo {
        Mode::Zen => "zen",
        Mode::Classic => "classico",
    }
}

/// Como a abertura descreve de onde veio cada partida, e os pontos dela.
fn descrever_origem(origem: &Origem, jogo: &Game) -> String {
    let nome = match origem {
        Origem::Novo => "novo",
        Origem::Arquivo => "arquivo",
        Origem::Estragado(_) => "estragado",
    };
    format!("{nome}:{}", jogo.score)
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
    /// Os recordes de cada modo (Zen, Clássico): lidos na abertura e gravados quando mudam.
    recordes: [Recordes; 2],
    /// Quem lê e grava os arquivos (pode não ter pasta nenhuma).
    salvador: Salvador,
    /// Os erros de gravação da última vez que gravei, para não repetir a mesma linha no registro.
    ultimos_erros: Vec<String>,
    /// Linhas "PRISMA_M1 ...", "PRISMA_M2 ..." e "PRISMA_M3 ..." que ainda não foram entregues (o
    /// teste do celular virtual lê).
    registros: Vec<String>,
    /// As escolhas da folha de Som (só os efeitos importam aqui; a música é com a camada de áudio).
    preferencias: Preferencias,
    /// Os efeitos sonoros que já "chegaram a hora" e ainda não foram entregues ao áudio.
    sons: Vec<Efeito>,
    /// A folha de Som está aberta: ela cobre o tabuleiro e os botões, e só os controles dela reagem.
    folha: bool,
    /// O canal cuja barra o dedo está arrastando agora (a gravação vem quando o dedo solta).
    arrasto: Option<Canal>,
    /// Há quanto tempo o guia de respiração começou (volta a zero ao entrar no Zen, ao trocar o
    /// ritmo e ao confirmar Novo jogo).
    respiracao_ms: f32,
    /// O erro de áudio mais recente da Música e do Ambiente (`None` = está bem).
    erros_de_audio: [Option<String>; 2],
}

impl App {
    /// Um app com sorte "de verdade", começando no Zen, que não lê nem grava nada em arquivo.
    pub fn novo() -> App {
        let mut classico = Game::new();
        classico.new_game(Mode::Classic);
        App::de_jogos(Game::new(), classico, Mode::Zen, Salvador::nenhum(), [Recordes::zerados(); 2])
    }

    /// Um app com sorte repetível (para testes e para as fotos de conferência), sem arquivos.
    pub fn com_sementes(zen: u32, classico: u32) -> App {
        let mut jogo_classico = Game::with_random(lcg_random(classico));
        jogo_classico.new_game(Mode::Classic);
        App::de_jogos(
            Game::with_random(lcg_random(zen)),
            jogo_classico,
            Mode::Zen,
            Salvador::nenhum(),
            [Recordes::zerados(); 2],
        )
    }

    /// Abre o app com o que está gravado: o modo em que o jogador estava, a partida de cada modo
    /// e os recordes. O que faltar ou estiver estragado recomeça do zero (só aquele item).
    pub fn abrir(salvador: Salvador) -> App {
        let modo = salvador.carregar_modo_ativo();
        // Preferências de som estragadas ou ausentes viram os padrões (o arquivo ruim vai para `.corrupt`).
        let (preferencias, _origem) = salvador.carregar_preferencias();
        let mut zen = Game::new();
        let mut classico = Game::new();
        let origem_zen = salvador.carregar_partida(Mode::Zen, &mut zen);
        let origem_classico = salvador.carregar_partida(Mode::Classic, &mut classico);
        let recordes = [salvador.carregar_recordes(Mode::Zen), salvador.carregar_recordes(Mode::Classic)];
        let descricao_zen = descrever_origem(&origem_zen, &zen);
        let descricao_classico = descrever_origem(&origem_classico, &classico);
        let mut app = App::de_jogos(zen, classico, modo, salvador, recordes);
        app.preferencias = preferencias;

        // A partida carregada pode ter passado dos recordes (como o `hud()` do JS, que confere
        // sempre que desenha): se sim, o recorde sobe e é gravado.
        let mudaram = [app.alcancar_recordes_de(Mode::Zen), app.alcancar_recordes_de(Mode::Classic)];
        app.registros.push(format!(
            "PRISMA_M2 aberto modo={} zen={descricao_zen} classico={descricao_classico} melhor_zen={} melhor_classico={}",
            nome_do_modo(modo),
            app.recordes[posicao_do_modo(Mode::Zen)].pontuacao,
            app.recordes[posicao_do_modo(Mode::Classic)].pontuacao,
        ));
        let mut erros = Vec::new();
        app.gravar_recordes_que_mudaram(mudaram, &mut erros);
        app.avisar_erros(erros);
        app
    }

    fn de_jogos(zen: Game, classico: Game, modo: Mode, salvador: Salvador, recordes: [Recordes; 2]) -> App {
        App {
            zen,
            classico,
            modo,
            gestos: Gestos::default(),
            animacao: None,
            dica: None,
            ganho: None,
            aviso: None,
            dialogo: false,
            recordes,
            salvador,
            ultimos_erros: Vec::new(),
            registros: vec![format!("PRISMA_M1 pronto modo={}", nome_do_modo(modo))],
            preferencias: Preferencias::default(),
            sons: Vec::new(),
            folha: false,
            arrasto: None,
            respiracao_ms: 0.0,
            erros_de_audio: [None, None],
        }
    }

    /// O mesmo app com outras preferências de som (nos testes e nas cenas de conferência; o app de
    /// verdade lê do arquivo).
    // TEMPORÁRIO: sem uso fora dos testes até a Task 7 (cenas `som` e `respiracao`).
    #[allow(dead_code)]
    pub fn com_preferencias(mut self, preferencias: Preferencias) -> App {
        self.preferencias = preferencias;
        self
    }

    pub fn modo(&self) -> Mode {
        self.modo
    }

    /// A partida do modo atual.
    pub fn jogo(&self) -> &Game {
        self.partida(self.modo)
    }

    /// A partida de um dos modos.
    fn partida(&self, modo: Mode) -> &Game {
        match modo {
            Mode::Zen => &self.zen,
            Mode::Classic => &self.classico,
        }
    }

    /// Os recordes de um modo.
    pub fn recordes(&self, modo: Mode) -> Recordes {
        self.recordes[posicao_do_modo(modo)]
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
        // Com a folha de Som aberta, só ela reage: o tabuleiro, as abas e os botões ficam "tampados".
        if self.folha {
            self.tocar_na_folha(evento, tela);
            return;
        }
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

    /// Um evento do dedo com a folha de Som aberta: botões respondem ao encostar; a barra de
    /// volume escolhe o valor ao encostar e a cada movimento, e grava quando o dedo solta.
    fn tocar_na_folha(&mut self, evento: Evento, tela: &TelaJogo) {
        match evento {
            Evento::Baixo { x, y } => {
                // Segundo dedo durante um arrasto não conta.
                if self.arrasto.is_some() {
                    return;
                }
                if let Some(botao) = self.botao_em(tela, x, y) {
                    self.apertar(botao);
                    return;
                }
                let som = &tela.som;
                let barras = [Canal::Efeitos, Canal::Musica, Canal::Ambiente];
                if let Some(canal) = barras.into_iter().find(|&canal| barra_do_canal(som, canal).contem(x, y)) {
                    self.arrasto = Some(canal);
                    self.escolher_volume(canal, barra_do_canal(som, canal), x);
                }
            }
            Evento::Mover { x, .. } => {
                if let Some(canal) = self.arrasto {
                    self.escolher_volume(canal, barra_do_canal(&tela.som, canal), x);
                }
            }
            Evento::Cima { x, .. } => {
                if let Some(canal) = self.arrasto {
                    self.escolher_volume(canal, barra_do_canal(&tela.som, canal), x);
                    self.terminar_arrasto();
                }
            }
            Evento::Cancelar => self.terminar_arrasto(),
        }
    }

    fn escolher_volume(&mut self, canal: Canal, barra: Rect, x: f32) {
        if let Some(valor) = volume_do_dedo(barra, x) {
            self.preferencias.volumes.definir(canal, valor);
        }
    }

    /// O dedo largou a barra (ou o gesto foi cancelado): o valor de agora vale e é gravado, uma vez.
    fn terminar_arrasto(&mut self) {
        if self.arrasto.take().is_some() {
            self.gravar_preferencias();
        }
    }

    /// Os botões que existem agora, com o lugar de cada um na tela.
    fn botoes_visiveis(&self, tela: &TelaJogo) -> Vec<(Botao, Rect)> {
        // Com a pergunta aberta, só existem os dois botões dela.
        if self.dialogo {
            return vec![(Botao::Cancelar, tela.dialogo_cancelar), (Botao::Confirmar, tela.dialogo_confirmar)];
        }
        // Com a folha de Som aberta, só existem o ♪ (que fecha) e os controles dela.
        if self.folha {
            let som = &tela.som;
            let mut botoes = vec![
                (Botao::Som, tela.botao_som),
                (Botao::FecharSom, som.fechar),
                (Botao::SonsDoJogo, som.sons_do_jogo),
                (Botao::Testar, som.testar),
            ];
            if self.modo == Mode::Zen {
                let do_zen = [
                    (Botao::LigaMusica, som.liga_musica),
                    (Botao::EscolheMusica, som.escolhe_musica),
                    (Botao::LigaAmbiente, som.liga_ambiente),
                    (Botao::EscolheAmbiente, som.escolhe_ambiente),
                    (Botao::Respiracao, som.respiracao),
                ];
                botoes.extend(do_zen.into_iter().filter_map(|(botao, retangulo)| retangulo.map(|r| (botao, r))));
            }
            return botoes;
        }
        let mut botoes = vec![(Botao::Zen, tela.aba_zen), (Botao::Classico, tela.aba_classico), (Botao::Som, tela.botao_som)];
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
        // Os botões da folha só valem com ela aberta; com ela aberta, só valem eles (e o ♪).
        if botao != Botao::Som && botao.da_folha() != self.folha {
            return;
        }
        if botao.so_do_zen() && self.modo != Mode::Zen {
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
            Botao::Som => {
                if self.folha {
                    self.fechar_folha();
                } else {
                    self.gestos.limpar();
                    self.folha = true;
                }
            }
            Botao::FecharSom => self.fechar_folha(),
            Botao::SonsDoJogo => self.alternar_sons_do_jogo(),
            Botao::Testar => self.testar_som(),
            Botao::LigaMusica => {
                self.preferencias.musica_ligada = !self.preferencias.musica_ligada;
                self.tentar_de_novo(Canal::Musica);
            }
            Botao::EscolheMusica => {
                self.preferencias.musica = self.preferencias.musica.proximo();
                self.tentar_de_novo(Canal::Musica);
            }
            Botao::LigaAmbiente => {
                self.preferencias.ambiente_ligado = !self.preferencias.ambiente_ligado;
                self.tentar_de_novo(Canal::Ambiente);
            }
            Botao::EscolheAmbiente => {
                self.preferencias.ambiente = self.preferencias.ambiente.proximo();
                self.tentar_de_novo(Canal::Ambiente);
            }
            Botao::Respiracao => {
                self.preferencias.respiracao = self.preferencias.respiracao.proximo();
                self.respiracao_ms = 0.0;
                self.gravar_preferencias();
            }
        }
    }

    fn fechar_folha(&mut self) {
        // Fechar no meio de um arrasto vale o último valor e grava.
        self.terminar_arrasto();
        self.folha = false;
    }

    /// "Sons do jogo": liga e desliga os efeitos. Se estiver ligado mas com o volume em 0 (não toca
    /// nada), o toque devolve o volume a 80 em vez de desligar.
    fn alternar_sons_do_jogo(&mut self) {
        if self.preferencias.sons_do_jogo && self.preferencias.volumes.efeitos == 0 {
            self.preferencias.volumes.efeitos = VOLUME_RESTAURADO;
        } else {
            self.preferencias.sons_do_jogo = !self.preferencias.sons_do_jogo;
        }
        self.gravar_preferencias();
    }

    /// "Testar som": toca uma combinação, mesmo com "Sons do jogo" desligado; com o volume dos
    /// efeitos em 0, avisa em vez de tocar.
    fn testar_som(&mut self) {
        if self.preferencias.volumes.efeitos == 0 {
            self.aviso = Temporizado::por(AVISO_SEM_VOLUME, AVISO_MS);
            return;
        }
        self.emitir_efeito(Efeito { sinal: Sinal::Combinacao, elevacao: 0 });
    }

    /// O jogador mexeu em Música ou Ambiente: grava, e o erro de áudio daquele canal (se havia) sai
    /// da tela, porque a camada de áudio vai tentar de novo.
    fn tentar_de_novo(&mut self, canal: Canal) {
        self.erro_de_audio(canal, None);
        self.gravar_preferencias();
    }

    /// Grava as preferências de som. Falha de gravação nunca interrompe o jogo: a escolha vale
    /// até fechar o app e o erro vai para o registro (uma vez, como os outros).
    fn gravar_preferencias(&mut self) {
        let erros = self.salvador.gravar_preferencias(&self.preferencias).err().into_iter().collect();
        self.avisar_erros(erros);
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
        let melhor_antes = self.melhor_parado();
        self.animacao = Some(EmAndamento::nova(reprodutor, base, Some("Tabuleiro reorganizado"), melhor_antes));
        self.gravar_tudo([false; 2]);
        self.observar(true);
        self.encerrar_se_terminou();
    }

    fn trocar_de_modo(&mut self, modo: Mode) {
        if self.ocupado() || self.modo == modo {
            return;
        }
        let mut mudaram = [false; 2];
        // O placar do modo que sai conta para o recorde dele; o do modo que entra também (o `hud()`
        // do JS confere o placar sempre que desenha).
        mudaram[posicao_do_modo(self.modo)] = self.alcancar_recordes_de(self.modo);
        self.limpar_tela();
        self.modo = modo;
        if modo == Mode::Zen {
            self.respiracao_ms = 0.0;
        }
        mudaram[posicao_do_modo(modo)] = self.alcancar_recordes_de(modo);
        self.registros.push(format!("PRISMA_M1 modo={} pontos={}", nome_do_modo(modo), self.jogo().score));
        self.gravar_tudo(mudaram);
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
        let mut mudaram = [false; 2];
        mudaram[posicao_do_modo(self.modo)] = self.alcancar_recordes_de(self.modo);
        self.jogo_mut().restart();
        self.limpar_tela();
        self.respiracao_ms = 0.0;
        self.registros.push(format!("PRISMA_M1 novo_jogo modo={} pontos=0", nome_do_modo(self.modo)));
        self.gravar_tudo(mudaram);
    }

    /// Esquece o que é passageiro: gesto, Dica, "+250" e avisos.
    fn limpar_tela(&mut self) {
        self.gestos.limpar();
        self.dica = None;
        self.ganho = None;
        self.aviso = None;
    }

    /// O MELHOR que o painel mostra com o jogo parado: o recorde, ou o placar atual se já o passou.
    fn melhor_parado(&self) -> u64 {
        self.recordes(self.modo).pontuacao.max(self.jogo().score)
    }

    /// Sobe os recordes do modo (pontuação e nível) até o placar que a partida dele mostra agora.
    /// Devolve se algo mudou.
    fn alcancar_recordes_de(&mut self, modo: Mode) -> bool {
        let (pontos, nivel) = (self.partida(modo).score, self.partida(modo).level);
        self.recordes[posicao_do_modo(modo)].alcancar(pontos, nivel)
    }

    // ---- Gravação ----------------------------------------------------------------------

    /// Grava os recordes dos modos marcados em `mudaram` (os erros vão para `erros`).
    fn gravar_recordes_que_mudaram(&mut self, mudaram: [bool; 2], erros: &mut Vec<String>) {
        for modo in [Mode::Zen, Mode::Classic] {
            let posicao = posicao_do_modo(modo);
            if mudaram[posicao] {
                if let Err(erro) = self.salvador.gravar_recordes(modo, &self.recordes[posicao]) {
                    erros.push(erro);
                }
            }
        }
    }

    /// Grava a partida do modo atual, o modo ativo e os recordes que mudaram. Falha de gravação
    /// nunca interrompe o jogo: vai para o registro (uma vez, não a cada jogada) e o jogo segue.
    fn gravar_tudo(&mut self, recordes_mudaram: [bool; 2]) {
        if !self.salvador.tem_pasta() {
            return;
        }
        let mut erros = Vec::new();
        match self.salvador.gravar_partida(self.jogo()) {
            Ok(()) => {
                let linha = format!("PRISMA_M2 gravado modo={} pontos={}", nome_do_modo(self.modo), self.jogo().score);
                self.registros.push(linha);
            }
            Err(erro) => erros.push(erro),
        }
        if let Err(erro) = self.salvador.gravar_modo_ativo(self.modo) {
            erros.push(erro);
        }
        self.gravar_recordes_que_mudaram(recordes_mudaram, &mut erros);
        self.avisar_erros(erros);
    }

    /// Escreve no registro os erros de gravação, a não ser que sejam os mesmos da última vez
    /// (uma pasta que não grava falharia em toda jogada). Uma gravação sem erro "zera" a lista:
    /// se falhar de novo depois, o aviso volta.
    fn avisar_erros(&mut self, erros: Vec<String>) {
        let mut diferentes: Vec<String> = Vec::new();
        for erro in erros {
            // Uma linha de registro é uma linha só.
            let erro = erro.replace(['\n', '\r'], " ");
            if !diferentes.contains(&erro) {
                diferentes.push(erro);
            }
        }
        if diferentes != self.ultimos_erros {
            for erro in &diferentes {
                self.registros.push(format!("PRISMA_M2 erro_ao_gravar {erro}"));
            }
        }
        self.ultimos_erros = diferentes;
    }

    // ---- Jogadas e animação ------------------------------------------------------------

    /// Joga a troca no motor (que resolve tudo de uma vez) e prepara a animação.
    fn jogar(&mut self, a: usize, b: usize) {
        let base = self.estagio();
        // Guardado antes de a jogada contar: durante a animação o painel mostra este MELHOR.
        let melhor_antes = self.melhor_parado();
        self.dica = None;
        let jogo = self.jogo_mut();
        let antes = jogo.board;
        let resultado = jogo.play(a, b);
        let (pontos, nivel) = (jogo.score, jogo.level);
        let passos = montar(antes, a, b, &resultado, nivel);
        self.registros.push(format!("PRISMA_M1 jogada valida={} pontos={} nivel={}", resultado.valid, pontos, nivel));
        self.animacao = Some(EmAndamento::nova(Reprodutor::novo(passos), base, None, melhor_antes));
        if resultado.valid {
            // Grava já, antes de a animação acabar: o motor resolveu a jogada inteira, e se o app
            // fechasse agora o jogador voltaria com o resultado dela. Jogada inválida não grava.
            let terminou = resultado.ended && self.modo == Mode::Classic;
            let mut mudaram = [false; 2];
            mudaram[posicao_do_modo(self.modo)] =
                self.recordes[posicao_do_modo(self.modo)].registrar(pontos, nivel, resultado.earned, terminou);
            self.gravar_tudo(mudaram);
        }
        self.observar(true);
        self.encerrar_se_terminou();
    }

    /// Deixa passar `dt_ms` milissegundos: da animação e dos relógios de Dica, "+250" e avisos.
    /// Tempo inválido conta como zero; tempo enorme termina tudo de uma vez.
    pub fn avancar(&mut self, dt_ms: f32) {
        let dt = if dt_ms.is_nan() || dt_ms < 0.0 { 0.0 } else { dt_ms };
        passar_tempo(&mut self.dica, dt);
        passar_tempo(&mut self.ganho, dt);
        passar_tempo(&mut self.aviso, dt);
        // O guia de respiração só anda no Zen. Tempo infinito não conta (nem congela o relógio).
        if self.modo == Mode::Zen && dt.is_finite() {
            self.respiracao_ms = (self.respiracao_ms + dt) % CICLO_DA_RESPIRACAO_MS;
        }
        if let Some(andamento) = self.animacao.as_mut() {
            andamento.reprodutor.avancar(dt);
        }
        // Um salto grande pula os passos sem tocar os sons deles (ver `SALTO_SEM_SOM_MS`).
        self.observar(dt <= SALTO_SEM_SOM_MS);
        self.encerrar_se_terminou();
    }

    /// Reage aos passos que começaram ou acabaram desde a última vez: a limpeza de Supernova
    /// mostra o aviso quando COMEÇA; qualquer limpeza mostra o "+ganho" quando ACABA; e cada passo
    /// pede o seu efeito sonoro uma vez só (`com_som` falso pula os sons, mas os passos contam
    /// como vistos).
    fn observar(&mut self, com_som: bool) {
        let Some(andamento) = self.animacao.as_mut() else { return };
        let reprodutor = &andamento.reprodutor;
        let concluidos = reprodutor.concluidos();
        let atual = reprodutor.atual().map(|(passo, _)| passo);
        let iniciados_agora = concluidos.len() + usize::from(atual.is_some());

        let mut novo_aviso = None;
        let mut efeitos = Vec::new();
        // Do passo mais antigo ainda não visto ao mais novo, em ordem: assim os sons saem na ordem
        // do tempo (começo de um passo, fim dele, começo do seguinte...), qualquer que seja o
        // tamanho do quadro.
        for indice in andamento.concluidos_vistos.min(andamento.iniciados)..iniciados_agora {
            let Some(passo) = concluidos.get(indice).or(atual) else { continue };
            if indice >= andamento.iniciados {
                if let Passo::Limpeza { supernova: true, .. } = passo {
                    novo_aviso = Some("Supernova");
                }
                efeitos.extend(efeito_ao_comecar(passo));
            }
            if indice >= andamento.concluidos_vistos && indice < concluidos.len() {
                efeitos.extend(efeito_ao_acabar(passo));
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
        if com_som {
            for efeito in efeitos {
                self.enfileirar_efeito(efeito);
            }
        }
    }

    /// Põe um efeito na fila do áudio e anota no registro. Com "Sons do jogo" desligado ou o
    /// volume dos efeitos em 0, nada entra na fila (e a animação é a mesma).
    fn enfileirar_efeito(&mut self, efeito: Efeito) {
        if !self.preferencias.sons_do_jogo || self.preferencias.volumes.efeitos == 0 {
            return;
        }
        self.emitir_efeito(efeito);
    }

    /// Põe o efeito na fila do áudio e anota no registro, sem olhar "Sons do jogo".
    fn emitir_efeito(&mut self, efeito: Efeito) {
        self.sons.push(efeito);
        self.registros.push(format!("PRISMA_M3 efeito sinal={} elevacao={}", efeito.sinal.nome(), efeito.elevacao));
    }

    /// Entrega (e esvazia) os efeitos sonoros que chegaram a hora de tocar.
    pub fn sons(&mut self) -> Vec<Efeito> {
        std::mem::take(&mut self.sons)
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
        // Parado, o recorde já conta o placar atual. Durante a animação o recorde não "adianta" os
        // pontos que o painel ainda vai mostrar: vale o MELHOR de antes da jogada.
        let melhor = match &self.animacao {
            Some(andamento) => andamento.melhor_antes,
            None => self.melhor_parado(),
        };
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

    /// A folha de Som está aberta?
    // TEMPORÁRIO: sem uso fora dos testes até a Task 7 (o desenho da folha).
    #[allow(dead_code)]
    pub fn folha_aberta(&self) -> bool {
        self.folha
    }

    /// As escolhas de som de agora (volumes, interruptores, faixas, respiração).
    pub fn preferencias(&self) -> Preferencias {
        self.preferencias.clone()
    }

    /// O que a música e o ambiente deveriam estar tocando agora: nada fora do Zen; no Zen, só o que
    /// está ligado, com o volume do canal dividido por 100 (0,0 a 1,0).
    pub fn som_desejado(&self) -> Desejado {
        if self.modo != Mode::Zen {
            return Desejado::default();
        }
        let preferencias = &self.preferencias;
        let volume = |canal| f32::from(preferencias.volumes.de(canal)) / 100.0;
        Desejado {
            musica: preferencias.musica_ligada.then(|| (preferencias.musica, volume(Canal::Musica))),
            ambiente: preferencias.ambiente_ligado.then(|| (preferencias.ambiente, volume(Canal::Ambiente))),
        }
    }

    /// Onde o guia de respiração está agora; `None` no Clássico e com o guia desligado.
    // TEMPORÁRIO: sem uso fora dos testes até a Task 7 (o desenho do guia).
    #[allow(dead_code)]
    pub fn guia_respiracao(&self) -> Option<EstadoRespiracao> {
        if self.modo != Mode::Zen {
            return None;
        }
        estado_da_respiracao(self.preferencias.respiracao, self.respiracao_ms)
    }

    /// O aviso "O áudio não começou..." enquanto a Música ou o Ambiente estiver com erro.
    // TEMPORÁRIO: sem uso fora dos testes até a Task 7 (o desenho da folha).
    #[allow(dead_code)]
    pub fn aviso_de_audio(&self) -> Option<&'static str> {
        self.erros_de_audio.iter().any(Option::is_some).then_some(AVISO_DE_AUDIO)
    }

    /// A camada de áudio conta que a Música ou o Ambiente falhou (`Some(texto)`) ou está bem
    /// (`None`). Os efeitos não têm aviso próprio.
    pub fn erro_de_audio(&mut self, canal: Canal, texto: Option<String>) {
        match canal {
            Canal::Musica => self.erros_de_audio[0] = texto,
            Canal::Ambiente => self.erros_de_audio[1] = texto,
            Canal::Efeitos => {}
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
    use crate::formato::partida_para_json;
    use crate::fundo::Desejado;
    use crate::input::Evento;
    use crate::layout::{cell_rect, tela_jogo, Rect, TelaJogo};
    use crate::linha::{Passo, Reprodutor};
    use crate::preferencias::{Ambiente, Canal, Fase, Musica, Respiracao, Volumes};
    use prisma_engine::{adjacent, lcg_random, level_goal, Board, Game, Mode, TileKind, SIZE};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
            // A semente 7 faz uma combinação e uma cascata; cada efeito sonoro ganha uma linha.
            "PRISMA_M3 efeito sinal=combinacao elevacao=0".to_string(),
            "PRISMA_M3 efeito sinal=cascata elevacao=1".to_string(),
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
        let melhor_antes = app.melhor_parado();
        app.animacao = Some(EmAndamento::nova(Reprodutor::novo(passos), base, None, melhor_antes));
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
        injetar(&mut app, vec![Passo::Limpeza { tabuleiro, celulas: vec![0], ativadas: vec![], supernova: false, ganho: 250, cadeia: 1, criou: false }]);
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
            vec![Passo::Limpeza { tabuleiro, celulas: vec![0], ativadas: vec![0], supernova: true, ganho: 250, cadeia: 1, criou: false }],
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

    // ---- M2: salvar e retomar ---------------------------------------------------------------

    /// Uma pasta de teste só deste teste (nome único: processo + contador). Ela **não** é criada
    /// aqui: o `Salvador` cria quando precisa, e o `Drop` apaga tudo no fim.
    struct Pasta(PathBuf);

    fn pasta_de_teste() -> Pasta {
        static CONTADOR: AtomicUsize = AtomicUsize::new(0);
        let numero = CONTADOR.fetch_add(1, Ordering::SeqCst);
        let caminho = std::env::temp_dir().join(format!("prisma-app-{}-{numero}", std::process::id()));
        let _ = fs::remove_dir_all(&caminho);
        Pasta(caminho)
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn abrir(pasta: &Pasta) -> App {
        App::abrir(Salvador::em(pasta.0.clone()))
    }

    /// Uma jogada boa, do dedo até o fim da animação.
    fn jogar_inteira(app: &mut App) {
        let (a, b) = boa_jogada(app);
        arrastar(app, a, b);
        terminar(app);
    }

    fn ler(pasta: &Pasta, nome: &str) -> String {
        fs::read_to_string(pasta.0.join(nome)).unwrap_or_else(|erro| panic!("não li {nome}: {erro}"))
    }

    fn linhas_m2(registro: &[String], tipo: &str) -> Vec<String> {
        let prefixo = format!("PRISMA_M2 {tipo}");
        registro.iter().filter(|linha| linha.starts_with(&prefixo)).cloned().collect()
    }

    #[test]
    fn abrir_pasta_vazia_comeca_no_zen_e_registra_a_abertura() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        assert_eq!(app.modo(), Mode::Zen);
        assert_eq!(app.jogo().score, 0);
        assert_eq!(app.recordes(Mode::Zen), Recordes::zerados());
        assert_eq!(
            app.registro(),
            vec![
                "PRISMA_M1 pronto modo=zen".to_string(),
                "PRISMA_M2 aberto modo=zen zen=novo:0 classico=novo:0 melhor_zen=0 melhor_classico=0".to_string(),
            ]
        );
        assert!(!pasta.0.exists(), "abrir sem nada para mudar não cria a pasta");
    }

    #[test]
    fn fechar_e_abrir_retoma_a_partida_de_cada_modo() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        for _ in 0..3 {
            jogar_inteira(&mut app);
        }
        let (tabuleiro_zen, pontos_zen) = (app.jogo().board, app.jogo().score);
        assert!(pontos_zen > 0);
        app.apertar(Botao::Classico);
        assert_eq!(app.jogo().score, 0, "o Clássico começa uma partida nova");
        for _ in 0..2 {
            jogar_inteira(&mut app);
        }
        let (tabuleiro_classico, pontos_classico) = (app.jogo().board, app.jogo().score);
        assert!(pontos_classico > 0 && !app.jogo().ended);
        drop(app);

        let mut outro = abrir(&pasta);
        assert_eq!(outro.modo(), Mode::Classic, "volta no modo em que fechou");
        assert_eq!(outro.jogo().board, tabuleiro_classico);
        assert_eq!(outro.jogo().score, pontos_classico);
        assert_eq!(
            outro.registro(),
            vec![
                "PRISMA_M1 pronto modo=classico".to_string(),
                format!(
                    "PRISMA_M2 aberto modo=classico zen=arquivo:{pontos_zen} classico=arquivo:{pontos_classico} \
                     melhor_zen={pontos_zen} melhor_classico={pontos_classico}"
                ),
            ]
        );
        outro.apertar(Botao::Zen);
        assert_eq!(outro.jogo().board, tabuleiro_zen);
        assert_eq!(outro.jogo().score, pontos_zen);
    }

    #[test]
    fn grava_logo_apos_a_jogada_antes_da_animacao_acabar() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        app.registro();
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        assert!(app.ocupado(), "a animação ainda não acabou");
        let pontos = app.jogo().score;
        assert!(pontos > 0);
        let registro = app.registro();
        assert!(registro.contains(&format!("PRISMA_M2 gravado modo=zen pontos={pontos}")), "{registro:?}");

        // Se o app fechasse agora, o jogo voltaria já com o resultado da jogada.
        let outro = abrir(&pasta);
        assert_eq!(outro.jogo().board, app.jogo().board);
        assert_eq!(outro.jogo().score, pontos);
    }

    #[test]
    fn jogada_invalida_nao_grava() {
        let pasta = pasta_de_teste();
        // Um Zen de sorte conhecida, gravado antes de o app abrir, para eu saber uma troca inválida.
        let salvador = Salvador::em(pasta.0.clone());
        let jogo = Game::with_random(lcg_random(7));
        salvador.gravar_partida(&jogo).unwrap();
        let (a, b) = par_invalido(7);

        let mut app = abrir(&pasta);
        assert_eq!(app.jogo().board, jogo.board, "o app abriu o jogo gravado");
        app.registro();
        fs::remove_file(pasta.0.join("zen.json")).unwrap();
        arrastar(&mut app, a, b);
        terminar(&mut app);
        let registro = app.registro();
        assert!(registro.contains(&"PRISMA_M1 jogada valida=false pontos=0 nivel=1".to_string()), "{registro:?}");
        assert!(linhas_m2(&registro, "gravado").is_empty(), "{registro:?}");
        assert!(!pasta.0.join("zen.json").exists(), "jogada inválida não grava a partida");
        assert!(!pasta.0.join("modo-ativo.txt").exists());

        // Uma jogada válida em seguida grava.
        jogar_inteira(&mut app);
        assert!(pasta.0.join("zen.json").is_file());
    }

    #[test]
    fn novo_jogo_embaralhar_e_troca_de_modo_gravam() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        let igual_ao_jogo = |app: &App, nome: &str| {
            assert_eq!(ler(&pasta, nome), partida_para_json(&app.jogo().to_saved()), "{nome} diferente do jogo");
        };

        // Embaralhar (Zen).
        app.apertar(Botao::Embaralhar);
        assert!(app.ocupado());
        igual_ao_jogo(&app, "zen.json");
        terminar(&mut app);

        // Novo jogo: o arquivo passa a ser a partida zerada.
        jogar_inteira(&mut app);
        fs::remove_file(pasta.0.join("zen.json")).unwrap();
        app.apertar(Botao::NovoJogo);
        app.apertar(Botao::Confirmar);
        assert_eq!(app.jogo().score, 0);
        igual_ao_jogo(&app, "zen.json");

        // Trocar de modo: grava o modo ativo e a partida do modo que entrou.
        fs::remove_file(pasta.0.join("modo-ativo.txt")).unwrap();
        assert!(!pasta.0.join("classico.json").exists());
        app.apertar(Botao::Classico);
        assert_eq!(ler(&pasta, "modo-ativo.txt"), "classic");
        igual_ao_jogo(&app, "classico.json");
        fs::remove_file(pasta.0.join("modo-ativo.txt")).unwrap();
        app.apertar(Botao::Zen);
        assert_eq!(ler(&pasta, "modo-ativo.txt"), "zen");
        igual_ao_jogo(&app, "zen.json");
    }

    #[test]
    fn partida_acabada_reabre_com_o_cartao_de_fim() {
        let pasta = pasta_de_teste();
        let salvador = Salvador::em(pasta.0.clone());
        let mut jogo = Game::with_random(lcg_random(5));
        jogo.new_game(Mode::Classic);
        let (a, b) = jogo.hint().unwrap();
        assert!(jogo.play(a, b).valid);
        jogo.ended = true;
        let (pontos, nivel) = (jogo.score, jogo.level);
        salvador.gravar_partida(&jogo).unwrap();
        salvador.gravar_modo_ativo(Mode::Classic).unwrap();

        let mut app = App::abrir(salvador);
        assert_eq!(app.modo(), Mode::Classic);
        assert!(app.jogo().ended);
        assert_eq!(app.painel().fim, Some((nivel.to_string(), formatar_numero(pontos))));
        app.registro();
        arrastar(&mut app, 0, 1);
        tocar_casa(&mut app, 3);
        assert!(!app.ocupado());
        app.apertar(Botao::Dica);
        assert_eq!(app.dica(), None, "a Dica não faz nada com a partida acabada");
        let registro = app.registro();
        assert!(registro.iter().all(|linha| !linha.contains("jogada") && !linha.contains("dica")), "{registro:?}");
    }

    #[test]
    fn arquivo_estragado_de_um_modo_nao_afeta_o_outro_nem_os_recordes() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        for _ in 0..2 {
            jogar_inteira(&mut app);
        }
        let (tabuleiro_zen, pontos_zen) = (app.jogo().board, app.jogo().score);
        app.apertar(Botao::Classico);
        jogar_inteira(&mut app);
        let pontos_classico = app.jogo().score;
        assert!(pontos_classico > 0);
        let (recordes_zen, recordes_classico) = (app.recordes(Mode::Zen), app.recordes(Mode::Classic));
        drop(app);

        fs::write(pasta.0.join("classico.json"), "isto não é um save").unwrap();
        let mut outro = abrir(&pasta);
        assert_eq!(
            linhas_m2(&outro.registro(), "aberto"),
            vec![format!(
                "PRISMA_M2 aberto modo=classico zen=arquivo:{pontos_zen} classico=estragado:0 \
                 melhor_zen={pontos_zen} melhor_classico={pontos_classico}"
            )]
        );
        assert_eq!(outro.modo(), Mode::Classic);
        assert_eq!(outro.jogo().score, 0, "o Clássico recomeça do zero");
        assert!(!outro.jogo().ended);
        assert_eq!((outro.recordes(Mode::Zen), outro.recordes(Mode::Classic)), (recordes_zen, recordes_classico));
        assert_eq!(ler(&pasta, "classico.json.corrupt"), "isto não é um save");
        outro.apertar(Botao::Zen);
        assert_eq!((outro.jogo().board, outro.jogo().score), (tabuleiro_zen, pontos_zen));
    }

    #[test]
    fn abrir_alcanca_os_recordes_com_a_partida_carregada() {
        let pasta = pasta_de_teste();
        let salvador = Salvador::em(pasta.0.clone());
        let mut jogo = Game::with_random(lcg_random(3));
        for _ in 0..3 {
            let (a, b) = jogo.hint().unwrap();
            jogo.play(a, b);
        }
        let (pontos, nivel) = (jogo.score, jogo.level);
        assert!(pontos > 0);
        salvador.gravar_partida(&jogo).unwrap(); // sem arquivo de recordes

        let app = App::abrir(salvador.clone());
        assert_eq!(app.recordes(Mode::Zen).pontuacao, pontos);
        assert_eq!(app.recordes(Mode::Zen).nivel, nivel);
        assert_eq!(salvador.carregar_recordes(Mode::Zen), app.recordes(Mode::Zen), "e foi gravado");
        assert_eq!(app.painel().melhor, formatar_numero(pontos));
    }

    #[test]
    fn recordes_sobem_e_sobrevivem() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        for _ in 0..4 {
            jogar_inteira(&mut app);
        }
        let recordes = app.recordes(Mode::Zen);
        assert_eq!(recordes.pontuacao, app.jogo().score);
        assert_eq!(recordes.nivel, app.jogo().level);
        assert!(recordes.jogada > 0 && recordes.jogada <= recordes.pontuacao);
        assert_eq!(recordes.terminadas, 0);

        app.apertar(Botao::NovoJogo);
        app.apertar(Botao::Confirmar);
        assert_eq!(app.jogo().score, 0);
        assert_eq!(app.recordes(Mode::Zen), recordes, "novo jogo não reduz os recordes");
        app.apertar(Botao::Classico);
        app.apertar(Botao::Zen);
        assert_eq!(app.recordes(Mode::Zen), recordes, "trocar de modo também não");
        assert_eq!(app.recordes(Mode::Classic), Recordes::zerados());
        drop(app);

        let mut outro = abrir(&pasta);
        assert_eq!(outro.recordes(Mode::Zen), recordes);
        assert_eq!(outro.painel().melhor, formatar_numero(recordes.pontuacao));
        assert_eq!(outro.painel().pontos, "0");
        outro.apertar(Botao::Classico);
        assert_eq!(outro.painel().melhor, "0", "os recordes do Zen não aparecem no Clássico");
    }

    #[test]
    fn terminadas_so_conta_no_classico_e_uma_vez() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        // No Zen, mesmo sem jogadas (o jogo cria um Espectro), nada termina.
        let (a, b) = boa_jogada(&mut app);
        app.jogo_mut().force_no_moves = true;
        arrastar(&mut app, a, b);
        terminar(&mut app);
        assert!(!app.jogo().ended);
        assert_eq!(app.recordes(Mode::Zen).terminadas, 0);
        jogar_inteira(&mut app);
        assert_eq!(app.recordes(Mode::Zen).terminadas, 0);

        app.apertar(Botao::Classico);
        jogar_inteira(&mut app);
        assert_eq!(app.recordes(Mode::Classic).terminadas, 0, "uma jogada comum não termina a partida");
        let (a, b) = boa_jogada(&mut app);
        app.jogo_mut().force_no_moves = true;
        arrastar(&mut app, a, b);
        terminar(&mut app);
        assert!(app.jogo().ended);
        assert_eq!(app.recordes(Mode::Classic).terminadas, 1);
        for _ in 0..200 {
            app.avancar(16.0);
        }
        assert_eq!(app.recordes(Mode::Classic).terminadas, 1);
        drop(app);

        let mut outro = abrir(&pasta);
        assert!(outro.jogo().ended);
        for _ in 0..200 {
            outro.avancar(16.0);
        }
        assert_eq!(outro.recordes(Mode::Classic).terminadas, 1, "reabrir não conta de novo");
        assert_eq!(outro.recordes(Mode::Zen).terminadas, 0);
    }

    #[test]
    fn melhor_nao_adianta_durante_a_animacao() {
        let mut app = App::com_sementes(7, 9);
        jogar_inteira(&mut app);
        let antes = app.jogo().score;
        assert_eq!(app.painel().melhor, formatar_numero(antes));
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        assert!(app.ocupado());
        assert!(app.jogo().score > antes, "o motor já somou a jogada");
        assert_eq!(app.painel().melhor, formatar_numero(antes), "o MELHOR só sobe quando a animação acaba");
        terminar(&mut app);
        assert_eq!(app.painel().melhor, formatar_numero(app.jogo().score));
    }

    #[test]
    fn sem_pasta_o_jogo_funciona_sem_salvar() {
        let pasta = pasta_de_teste();
        fs::create_dir_all(&pasta.0).unwrap();
        let arquivo = pasta.0.join("sou-um-arquivo");
        fs::write(&arquivo, "x").unwrap();
        let mut app = App::abrir(Salvador::em(arquivo.join("dados")));
        for _ in 0..3 {
            jogar_inteira(&mut app);
        }
        assert!(app.jogo().score > 0);
        let registro = app.registro();
        assert_eq!(linhas_m2(&registro, "erro_ao_gravar").len(), 1, "o erro é avisado uma vez só: {registro:?}");
        assert!(linhas_m2(&registro, "gravado").is_empty(), "{registro:?}");
    }

    #[test]
    fn erro_repetido_so_e_avisado_uma_vez_e_volta_a_ser_avisado_depois_de_dar_certo() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        app.registro();
        // O temporário da partida vira uma pasta: gravar a partida falha, o resto não.
        fs::create_dir_all(pasta.0.join("zen.json.tmp")).unwrap();
        jogar_inteira(&mut app);
        jogar_inteira(&mut app);
        let registro = app.registro();
        assert_eq!(linhas_m2(&registro, "erro_ao_gravar").len(), 1, "{registro:?}");
        assert!(linhas_m2(&registro, "gravado").is_empty());
        assert_eq!(ler(&pasta, "modo-ativo.txt"), "zen", "o que deu certo foi gravado");

        fs::remove_dir(pasta.0.join("zen.json.tmp")).unwrap();
        jogar_inteira(&mut app);
        let registro = app.registro();
        assert_eq!(linhas_m2(&registro, "gravado").len(), 1, "{registro:?}");
        assert!(linhas_m2(&registro, "erro_ao_gravar").is_empty());

        fs::create_dir_all(pasta.0.join("zen.json.tmp")).unwrap();
        jogar_inteira(&mut app);
        assert_eq!(linhas_m2(&app.registro(), "erro_ao_gravar").len(), 1, "falhou de novo depois de dar certo");
    }

    #[test]
    fn trocar_de_modo_conta_o_placar_do_modo_que_sai_e_grava_os_recordes() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        app.apertar(Botao::Classico);
        // Um placar que não veio de jogada nenhuma (como uma partida carregada ou mexida no motor).
        app.jogo_mut().score = 777;
        assert_eq!(app.recordes(Mode::Classic).pontuacao, 0);
        app.apertar(Botao::Zen);
        assert_eq!(app.recordes(Mode::Classic).pontuacao, 777, "o placar do modo que saiu virou recorde");
        let gravado = Salvador::em(pasta.0.clone()).carregar_recordes(Mode::Classic);
        assert_eq!(gravado, app.recordes(Mode::Classic), "e foi para o arquivo");
    }

    // ----- Efeitos sonoros: quando cada um toca -----

    fn efeito(sinal: Sinal, elevacao: u8) -> Efeito {
        Efeito { sinal, elevacao }
    }

    /// Anda a animação até o fim em quadros de `quadro_ms` e junta os sons que saíram (mais os
    /// que já estavam na fila).
    fn sons_ate_o_fim(app: &mut App, quadro_ms: f32) -> Vec<Efeito> {
        let mut sons = app.sons();
        for _ in 0..100_000 {
            if !app.ocupado() {
                break;
            }
            app.avancar(quadro_ms);
            sons.extend(app.sons());
        }
        assert!(!app.ocupado(), "a animação não terminou");
        sons
    }

    /// A jogada da Dica, do dedo até o fim da animação: devolve os sons que ela fez tocar.
    fn sons_de_uma_jogada(app: &mut App) -> Vec<Efeito> {
        let (a, b) = boa_jogada(app);
        arrastar(app, a, b);
        sons_ate_o_fim(app, 16.0)
    }

    /// Os sons da jogada de número `n` (1 = a primeira) da Dica, num jogo Zen da `semente`.
    fn sons_da_jogada_numero(semente: u32, n: usize) -> Vec<Efeito> {
        let mut app = App::com_sementes(semente, 9);
        for _ in 1..n {
            jogar_inteira(&mut app);
        }
        app.sons(); // esvazia o que as jogadas de antes deixaram
        sons_de_uma_jogada(&mut app)
    }

    fn linhas_de_efeito(registro: &[String]) -> Vec<String> {
        registro.iter().filter(|linha| linha.starts_with("PRISMA_M3 efeito")).cloned().collect()
    }

    #[test]
    fn jogada_invalida_toca_so_invalida() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = par_invalido(7);
        arrastar(&mut app, a, b);
        // Toca no começo da troca, antes de passar qualquer tempo.
        assert_eq!(app.sons(), vec![efeito(Sinal::Invalida, 0)]);
        assert_eq!(sons_ate_o_fim(&mut app, 16.0), vec![]);
    }

    #[test]
    fn limpeza_simples_toca_combinacao_elevacao_0() {
        let mut app = App::com_sementes(2, 9);
        assert_eq!(sons_de_uma_jogada(&mut app), vec![efeito(Sinal::Combinacao, 0)]);
    }

    #[test]
    fn cascata_toca_cascata_com_elevacao_subindo() {
        // Semente 9: a 1ª jogada da Dica faz 4 limpezas seguidas, nenhuma cria pedra especial.
        assert_eq!(
            sons_da_jogada_numero(9, 1),
            vec![
                efeito(Sinal::Combinacao, 0),
                efeito(Sinal::Cascata, 1),
                efeito(Sinal::Cascata, 2),
                efeito(Sinal::Cascata, 3),
            ]
        );
        // Semente 7: duas limpezas.
        assert_eq!(sons_da_jogada_numero(7, 1), vec![efeito(Sinal::Combinacao, 0), efeito(Sinal::Cascata, 1)]);
    }

    #[test]
    fn limpeza_que_cria_especial_toca_criacao_depois_da_combinacao() {
        // Semente 1: uma limpeza que cria uma pedra especial. A criação toca no FIM da limpeza.
        assert_eq!(sons_da_jogada_numero(1, 1), vec![efeito(Sinal::Combinacao, 0), efeito(Sinal::Criacao, 0)]);
        // Semente 4: a criação toca antes de a cascata começar.
        assert_eq!(
            sons_da_jogada_numero(4, 1),
            vec![efeito(Sinal::Combinacao, 0), efeito(Sinal::Criacao, 0), efeito(Sinal::Cascata, 1)]
        );
        // Semente 33: três limpezas, a 1ª e a 3ª criam.
        assert_eq!(
            sons_da_jogada_numero(33, 1),
            vec![
                efeito(Sinal::Combinacao, 0),
                efeito(Sinal::Criacao, 0),
                efeito(Sinal::Cascata, 1),
                efeito(Sinal::Cascata, 2),
                efeito(Sinal::Criacao, 0),
            ]
        );
    }

    #[test]
    fn especial_ativado_toca_pulso() {
        // Semente 13, 4ª jogada da Dica: ativa um Pulso (sem cascata, sem criação).
        assert_eq!(sons_da_jogada_numero(13, 4), vec![efeito(Sinal::Pulso, 0)]);
    }

    #[test]
    fn especial_ativado_toca_raio() {
        // Semente 3, 14ª jogada: ativa um Raio e a mesma limpeza cria uma pedra especial.
        assert_eq!(sons_da_jogada_numero(3, 14), vec![efeito(Sinal::Raio, 0), efeito(Sinal::Criacao, 0)]);
    }

    #[test]
    fn especial_ativado_toca_espectro() {
        // Semente 14, 2ª jogada: ativa um Espectro e depois vem uma cascata.
        assert_eq!(sons_da_jogada_numero(14, 2), vec![efeito(Sinal::Espectro, 0), efeito(Sinal::Cascata, 1)]);
    }

    /// Uma limpeza montada à mão, com pedras especiais nas casas `ativadas`.
    fn sons_de_limpeza_a_mao(especiais: &[(usize, TileKind)], supernova: bool, cadeia: u32) -> Vec<Efeito> {
        let mut app = App::com_sementes(7, 9);
        let mut tabuleiro = app.jogo().board;
        for &(casa, tipo) in especiais {
            tabuleiro[casa / SIZE][casa % SIZE].kind = tipo;
        }
        let ativadas = especiais.iter().map(|&(casa, _)| casa).collect();
        injetar(&mut app, vec![Passo::Limpeza { tabuleiro, celulas: vec![0], ativadas, supernova, ganho: 250, cadeia, criou: false }]);
        sons_ate_o_fim(&mut app, 16.0)
    }

    #[test]
    fn especial_ativado_toca_supernova() {
        assert_eq!(sons_de_limpeza_a_mao(&[(10, TileKind::Burst)], true, 1), vec![efeito(Sinal::Supernova, 0)]);
        assert_eq!(sons_de_limpeza_a_mao(&[(10, TileKind::Spectrum)], true, 4), vec![efeito(Sinal::Supernova, 0)]);
    }

    #[test]
    fn varios_especiais_ativados_tocam_o_de_maior_prioridade() {
        let burst = (10, TileKind::Burst);
        let cross = (20, TileKind::Cross);
        let espectro = (30, TileKind::Spectrum);
        assert_eq!(sons_de_limpeza_a_mao(&[burst, cross], false, 1), vec![efeito(Sinal::Raio, 0)]);
        assert_eq!(sons_de_limpeza_a_mao(&[burst, cross, espectro], false, 1), vec![efeito(Sinal::Espectro, 0)]);
        assert_eq!(sons_de_limpeza_a_mao(&[burst], false, 5), vec![efeito(Sinal::Pulso, 0)], "a elevação só vale na combinação e na cascata");
        // Sem especial: a cadeia decide entre combinação e cascata, e a elevação sobe até 5.
        assert_eq!(sons_de_limpeza_a_mao(&[], false, 1), vec![efeito(Sinal::Combinacao, 0)]);
        assert_eq!(sons_de_limpeza_a_mao(&[], false, 3), vec![efeito(Sinal::Cascata, 2)]);
        assert_eq!(sons_de_limpeza_a_mao(&[], false, 40), vec![efeito(Sinal::Cascata, 5)]);
    }

    #[test]
    fn subir_de_nivel_toca_nivel_depois_das_limpezas() {
        // Faltando 1 ponto para o nível 2, qualquer limpeza sobe de nível.
        let mut app = App::com_sementes(2, 9);
        app.jogo_mut().score = level_goal(1) - 1;
        assert_eq!(sons_de_uma_jogada(&mut app), vec![efeito(Sinal::Combinacao, 0), efeito(Sinal::Nivel, 0)]);
        // Semente 15, 13ª jogada: um Pulso, um embaralhar automático (mudo) e a subida de nível.
        assert_eq!(sons_da_jogada_numero(15, 13), vec![efeito(Sinal::Pulso, 0), efeito(Sinal::Nivel, 0)]);
    }

    #[test]
    fn resgate_toca_resgate() {
        // Semente 1, 26ª jogada: uma limpeza que cria pedra, a queda e o Resgate do Zen.
        assert_eq!(
            sons_da_jogada_numero(1, 26),
            vec![efeito(Sinal::Combinacao, 0), efeito(Sinal::Criacao, 0), efeito(Sinal::Resgate, 0)]
        );
        // Montado à mão, o Resgate toca já no começo do passo.
        let mut app = App::com_sementes(7, 9);
        let tabuleiro = app.jogo().board;
        injetar(&mut app, vec![Passo::Resgate { tabuleiro, indice: 3 }]);
        assert_eq!(app.sons(), vec![efeito(Sinal::Resgate, 0)]);
        assert_eq!(sons_ate_o_fim(&mut app, 16.0), vec![]);
    }

    #[test]
    fn embaralhar_nao_toca() {
        let mut app = App::com_sementes(7, 9);
        app.registro();
        app.apertar(Botao::Embaralhar);
        assert!(app.ocupado(), "o embaralhar do botão anima");
        assert_eq!(sons_ate_o_fim(&mut app, 16.0), vec![]);
        assert_eq!(linhas_de_efeito(&app.registro()), Vec::<String>::new());
    }

    #[test]
    fn sons_do_jogo_desligado_ou_efeitos_zero_nao_enfileira_nada_e_a_animacao_e_a_mesma() {
        let ligado = App::com_sementes(9, 9);
        let sem_sons = App::com_sementes(9, 9).com_preferencias(Preferencias { sons_do_jogo: false, ..Preferencias::default() });
        let mut sem_efeitos = Preferencias::default();
        sem_efeitos.volumes.efeitos = 0;
        let sem_efeitos = App::com_sementes(9, 9).com_preferencias(sem_efeitos);

        let mut apps = [ligado, sem_sons, sem_efeitos];
        for app in &mut apps {
            let (a, b) = boa_jogada(app);
            arrastar(app, a, b);
        }
        let mut sons: [Vec<Efeito>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for _ in 0..10_000 {
            for (app, juntos) in apps.iter_mut().zip(sons.iter_mut()) {
                juntos.extend(app.sons());
                app.avancar(16.0);
            }
            // Os três andam passo a passo iguais: o mesmo passo, o mesmo tempo e o mesmo placar.
            let [primeiro, segundo, terceiro] = &apps;
            for outro in [segundo, terceiro] {
                assert_eq!(primeiro.passo_atual(), outro.passo_atual());
                assert_eq!(primeiro.estagio(), outro.estagio());
                assert_eq!(primeiro.painel().pontos, outro.painel().pontos);
            }
            if !primeiro.ocupado() {
                break;
            }
        }
        for app in &mut apps {
            sons_ate_o_fim(app, 16.0);
        }
        assert!(!sons[0].is_empty(), "com os sons ligados, a jogada toca");
        assert_eq!(sons[1], vec![], "Sons do jogo desligado");
        assert_eq!(sons[2], vec![], "volume de efeitos em 0");
        assert_eq!(apps[0].jogo().score, apps[1].jogo().score);
        assert_eq!(apps[0].jogo().score, apps[2].jogo().score);
        for app in &mut apps[1..] {
            assert_eq!(linhas_de_efeito(&app.registro()), Vec::<String>::new(), "sem som, sem linha de registro");
        }
    }

    #[test]
    fn salto_de_tempo_maior_que_2s_nao_enfileira_rajada() {
        let mut app = App::com_sementes(9, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        app.avancar(300.0); // a 1ª limpeza já começou
        assert_eq!(app.sons(), vec![efeito(Sinal::Combinacao, 0)]);
        // O app voltou do segundo plano: 5 s de uma vez pulam o resto da animação, sem tocar nada.
        app.avancar(5000.0);
        assert!(!app.ocupado());
        assert_eq!(app.sons(), vec![]);
        assert_eq!(linhas_de_efeito(&app.registro()).len(), 1, "só a combinação que tocou antes do salto");
        // Já com o salto antes de qualquer limpeza: nada toca.
        let mut outro = App::com_sementes(9, 9);
        let (a, b) = boa_jogada(&mut outro);
        arrastar(&mut outro, a, b);
        outro.avancar(2001.0);
        assert_eq!(outro.sons(), vec![]);
    }

    #[test]
    fn um_salto_de_ate_2s_ainda_toca_tudo() {
        let mut app = App::com_sementes(9, 9);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        app.avancar(2000.0);
        let sons = app.sons();
        // Os passos que começaram nesses 2 s tocam, na ordem; o último pode ainda estar em curso.
        assert!(sons.len() >= 3 && sons[0] == efeito(Sinal::Combinacao, 0) && sons[1] == efeito(Sinal::Cascata, 1), "{sons:?}");
    }

    #[test]
    fn cada_efeito_sai_uma_vez_so() {
        let esperado = vec![
            efeito(Sinal::Combinacao, 0),
            efeito(Sinal::Criacao, 0),
            efeito(Sinal::Cascata, 1),
            efeito(Sinal::Cascata, 2),
            efeito(Sinal::Criacao, 0),
        ];
        // Qualquer tamanho de quadro (até 2 s) dá a mesma lista, na mesma ordem, sem repetir.
        for quadro in [1.0, 16.0, 50.0, 333.0, 700.0, 1999.0] {
            let mut app = App::com_sementes(33, 9);
            let (a, b) = boa_jogada(&mut app);
            arrastar(&mut app, a, b);
            assert_eq!(sons_ate_o_fim(&mut app, quadro), esperado, "quadros de {quadro} ms");
        }
    }

    #[test]
    fn sons_esvazia_a_fila() {
        let mut app = App::com_sementes(7, 9);
        let (a, b) = par_invalido(7);
        arrastar(&mut app, a, b);
        assert_eq!(app.sons().len(), 1);
        assert_eq!(app.sons(), vec![]);
        app.avancar(16.0);
        assert_eq!(app.sons(), vec![]);
    }

    #[test]
    fn o_registro_ganha_uma_linha_por_efeito() {
        let mut app = App::com_sementes(4, 9);
        jogar_inteira(&mut app);
        assert_eq!(
            linhas_de_efeito(&app.registro()),
            vec![
                "PRISMA_M3 efeito sinal=combinacao elevacao=0".to_string(),
                "PRISMA_M3 efeito sinal=criacao elevacao=0".to_string(),
                "PRISMA_M3 efeito sinal=cascata elevacao=1".to_string(),
            ]
        );
        // Jogada inválida: uma linha, sinal `invalida`.
        let mut outro = App::com_sementes(7, 9);
        let (a, b) = par_invalido(7);
        arrastar(&mut outro, a, b);
        terminar(&mut outro);
        assert_eq!(linhas_de_efeito(&outro.registro()), vec!["PRISMA_M3 efeito sinal=invalida elevacao=0".to_string()]);
    }

    // ----- Folha de Som, preferências e respiração -----

    fn tela_classico() -> TelaJogo {
        tela_jogo(540.0, 960.0, false)
    }

    /// Um toque rápido no meio de um retângulo.
    fn tocar_em(app: &mut App, tela: &TelaJogo, retangulo: Rect) {
        let (x, y) = meio(retangulo);
        app.tocar(Evento::Baixo { x, y }, tela);
        app.tocar(Evento::Cima { x, y }, tela);
    }

    fn abrir_folha(app: &mut App) {
        let tela = if app.modo() == Mode::Zen { tela() } else { tela_classico() };
        tocar_em(app, &tela, tela.botao_som);
        assert!(app.folha_aberta(), "o toque no ♪ devia abrir a folha");
    }

    /// O x que fica a `fracao` (0 a 1) da largura da barra.
    fn x_da_barra(barra: Rect, fracao: f32) -> f32 {
        barra.x + fracao * barra.w
    }

    /// O `som.json` gravado, já lido.
    fn som_json(pasta: &Pasta) -> serde_json::Value {
        serde_json::from_str(&ler(pasta, "som.json")).expect("som.json é um JSON válido")
    }

    fn sem_efeitos() -> Preferencias {
        let mut preferencias = Preferencias::default();
        preferencias.volumes.efeitos = 0;
        preferencias
    }

    #[test]
    fn tocar_no_botao_som_abre_e_fecha() {
        let mut app = App::com_sementes(7, 9);
        let tela = tela();
        assert!(!app.folha_aberta());
        tocar_em(&mut app, &tela, tela.botao_som);
        assert!(app.folha_aberta());
        tocar_em(&mut app, &tela, tela.botao_som);
        assert!(!app.folha_aberta());
        // Pelo botão Fechar da folha.
        tocar_em(&mut app, &tela, tela.botao_som);
        assert!(app.folha_aberta());
        tocar_em(&mut app, &tela, tela.som.fechar);
        assert!(!app.folha_aberta());
        // Pelo `Botao`, e também no Clássico.
        app.apertar(Botao::Som);
        assert!(app.folha_aberta());
        app.apertar(Botao::FecharSom);
        assert!(!app.folha_aberta());
        app.apertar(Botao::Classico);
        abrir_folha(&mut app);
        let classico = tela_classico();
        tocar_em(&mut app, &classico, classico.som.fechar);
        assert!(!app.folha_aberta());
        // Os botões da folha não fazem nada com a folha fechada.
        app.apertar(Botao::FecharSom);
        app.apertar(Botao::SonsDoJogo);
        assert_eq!(app.preferencias(), Preferencias::default());
    }

    #[test]
    fn folha_aberta_bloqueia_tabuleiro_abas_e_botoes() {
        let mut app = App::com_sementes(7, 9);
        let tela = tela();
        abrir_folha(&mut app);
        let (a, b) = boa_jogada(&mut app);
        arrastar(&mut app, a, b);
        assert!(!app.ocupado(), "o tabuleiro não reage com a folha aberta");
        assert_eq!(app.jogo().score, 0);
        tocar_casa(&mut app, a);
        assert_eq!(app.gestos().selecionada(), None);
        tocar_em(&mut app, &tela, tela.aba_classico);
        assert_eq!(app.modo(), Mode::Zen, "as abas não reagem");
        tocar_em(&mut app, &tela, tela.botao_dica);
        assert_eq!(app.dica(), None);
        tocar_em(&mut app, &tela, tela.botao_novo_jogo);
        assert!(!app.painel().dialogo, "o Novo jogo não abre");
        if let Some(embaralhar) = tela.botao_embaralhar {
            tocar_em(&mut app, &tela, embaralhar);
        }
        assert!(!app.ocupado());
        // Pelo `Botao`, nada além da folha vale.
        for botao in [Botao::Classico, Botao::Dica, Botao::Embaralhar, Botao::NovoJogo, Botao::JogarNovamente] {
            app.apertar(botao);
        }
        assert_eq!((app.modo(), app.dica(), app.ocupado(), app.painel().dialogo), (Mode::Zen, None, false, false));
        // Fechada a folha, tudo volta a funcionar.
        app.apertar(Botao::FecharSom);
        arrastar(&mut app, a, b);
        assert!(app.ocupado());
    }

    #[test]
    fn arrastar_a_barra_muda_o_valor_de_5_em_5_e_grava_uma_vez() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        let tela = tela();
        abrir_folha(&mut app);
        let barra = tela.som.barra_efeitos;
        let y = meio(barra).1;
        app.tocar(Evento::Baixo { x: x_da_barra(barra, 0.5), y }, &tela);
        assert_eq!(app.preferencias().volumes.efeitos, 50, "o toque já escolhe o valor");
        assert!(!pasta.0.join("som.json").exists(), "nada é gravado antes de soltar o dedo");
        // Arrastando por toda a barra, o valor anda de 5 em 5 e acompanha o dedo.
        for passo in 0..=40 {
            let fracao = passo as f32 / 40.0;
            app.tocar(Evento::Mover { x: x_da_barra(barra, fracao), y }, &tela);
            let valor = app.preferencias().volumes.efeitos;
            assert_eq!(valor % 5, 0, "valor {valor} não é de 5 em 5");
            assert!((f32::from(valor) - fracao * 100.0).abs() <= 2.6, "valor {valor} longe do dedo ({fracao})");
        }
        app.tocar(Evento::Mover { x: x_da_barra(barra, 0.77), y }, &tela);
        assert_eq!(app.preferencias().volumes.efeitos, 75);
        assert!(!pasta.0.join("som.json").exists(), "nada é gravado durante o arrasto");
        app.tocar(Evento::Cima { x: x_da_barra(barra, 0.77), y }, &tela);
        assert_eq!(som_json(&pasta)["audio"]["effects"], 75);
        // Gravou uma vez: soltar de novo (ou cancelar) sem arrasto não grava mais nada.
        fs::remove_file(pasta.0.join("som.json")).unwrap();
        app.tocar(Evento::Cima { x: 0.0, y: 0.0 }, &tela);
        app.tocar(Evento::Cancelar, &tela);
        assert!(!pasta.0.join("som.json").exists());
        // Os outros dois volumes seguem iguais.
        assert_eq!((app.preferencias().volumes.musica, app.preferencias().volumes.ambiente), (70, 65));
    }

    #[test]
    fn cada_barra_muda_o_seu_canal() {
        let mut app = App::com_sementes(7, 9);
        let tela = tela();
        abrir_folha(&mut app);
        for (barra, canal) in [
            (tela.som.barra_efeitos, Canal::Efeitos),
            (tela.som.barra_musica, Canal::Musica),
            (tela.som.barra_ambiente, Canal::Ambiente),
        ] {
            let (x, y) = (x_da_barra(barra, 0.25), meio(barra).1);
            app.tocar(Evento::Baixo { x, y }, &tela);
            app.tocar(Evento::Cima { x, y }, &tela);
            assert_eq!(app.preferencias().volumes.de(canal), 25, "{canal:?}");
        }
        assert_eq!(app.preferencias().volumes, Volumes { efeitos: 25, musica: 25, ambiente: 25 });
    }

    #[test]
    fn arrasto_que_sai_da_barra_prende_em_0_e_100() {
        let mut app = App::com_sementes(7, 9);
        let tela = tela();
        abrir_folha(&mut app);
        let barra = tela.som.barra_musica;
        let y = meio(barra).1;
        app.tocar(Evento::Baixo { x: x_da_barra(barra, 0.5), y }, &tela);
        app.tocar(Evento::Mover { x: barra.x - 500.0, y }, &tela);
        assert_eq!(app.preferencias().volumes.musica, 0);
        app.tocar(Evento::Mover { x: barra.x + barra.w + 500.0, y }, &tela);
        assert_eq!(app.preferencias().volumes.musica, 100);
        // O dedo pode sair da barra para cima ou para baixo: só o x conta.
        app.tocar(Evento::Mover { x: x_da_barra(barra, 0.3), y: y + 400.0 }, &tela);
        assert_eq!(app.preferencias().volumes.musica, 30);
        // Posição inválida é ignorada.
        app.tocar(Evento::Mover { x: f32::NAN, y }, &tela);
        assert_eq!(app.preferencias().volumes.musica, 30);
        app.tocar(Evento::Cima { x: barra.x - 9999.0, y }, &tela);
        assert_eq!(app.preferencias().volumes.musica, 0, "soltar longe prende em 0");
        // Os outros dois canais não mexeram.
        assert_eq!((app.preferencias().volumes.efeitos, app.preferencias().volumes.ambiente), (80, 65));
    }

    #[test]
    fn cancelar_no_meio_do_arrasto_deixa_o_ultimo_valor_e_grava() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        let tela = tela();
        abrir_folha(&mut app);
        let barra = tela.som.barra_ambiente;
        let y = meio(barra).1;
        app.tocar(Evento::Baixo { x: x_da_barra(barra, 0.5), y }, &tela);
        app.tocar(Evento::Mover { x: x_da_barra(barra, 0.8), y }, &tela);
        assert!(!pasta.0.join("som.json").exists());
        app.tocar(Evento::Cancelar, &tela);
        assert_eq!(app.preferencias().volumes.ambiente, 80);
        assert_eq!(som_json(&pasta)["audio"]["ambience"], 80);
        // O arrasto acabou: mover o dedo agora não muda mais nada.
        app.tocar(Evento::Mover { x: x_da_barra(barra, 0.1), y }, &tela);
        assert_eq!(app.preferencias().volumes.ambiente, 80);
    }

    #[test]
    fn fechar_a_folha_no_meio_do_arrasto_grava() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        let tela = tela();
        abrir_folha(&mut app);
        let barra = tela.som.barra_efeitos;
        let y = meio(barra).1;
        app.tocar(Evento::Baixo { x: x_da_barra(barra, 0.6), y }, &tela);
        app.apertar(Botao::FecharSom);
        assert!(!app.folha_aberta());
        assert_eq!(som_json(&pasta)["audio"]["effects"], 60);
        app.tocar(Evento::Mover { x: x_da_barra(barra, 0.1), y }, &tela);
        assert_eq!(app.preferencias().volumes.efeitos, 60, "sem folha, não há arrasto");
    }

    #[test]
    fn sons_do_jogo_com_efeitos_zero_restaura_80_em_vez_de_desligar() {
        let mut app = App::com_sementes(7, 9).com_preferencias(sem_efeitos());
        let tela = tela();
        abrir_folha(&mut app);
        tocar_em(&mut app, &tela, tela.som.sons_do_jogo);
        assert!(app.preferencias().sons_do_jogo, "continua ligado");
        assert_eq!(app.preferencias().volumes.efeitos, 80);
        // Com volume normal, o botão liga e desliga.
        tocar_em(&mut app, &tela, tela.som.sons_do_jogo);
        assert!(!app.preferencias().sons_do_jogo);
        assert_eq!(app.preferencias().volumes.efeitos, 80);
        tocar_em(&mut app, &tela, tela.som.sons_do_jogo);
        assert!(app.preferencias().sons_do_jogo);
        // Desligado e com efeitos em 0: só liga (o volume fica em 0).
        let mut desligado = sem_efeitos();
        desligado.sons_do_jogo = false;
        let mut app = App::com_sementes(7, 9).com_preferencias(desligado);
        abrir_folha(&mut app);
        tocar_em(&mut app, &tela, tela.som.sons_do_jogo);
        assert!(app.preferencias().sons_do_jogo);
        assert_eq!(app.preferencias().volumes.efeitos, 0);
    }

    #[test]
    fn testar_com_efeitos_zero_avisa() {
        let mut app = App::com_sementes(7, 9).com_preferencias(sem_efeitos());
        let tela = tela();
        abrir_folha(&mut app);
        assert_eq!(app.painel().aviso, None);
        tocar_em(&mut app, &tela, tela.som.testar);
        assert_eq!(app.painel().aviso.as_deref(), Some("Aumente o volume dos efeitos"));
        assert_eq!(app.sons(), vec![]);
        // O aviso some sozinho, como os outros.
        app.avancar(2000.0);
        assert_eq!(app.painel().aviso, None);
    }

    #[test]
    fn testar_toca_mesmo_com_sons_do_jogo_desligado() {
        let desligado = Preferencias { sons_do_jogo: false, ..Preferencias::default() };
        let mut app = App::com_sementes(7, 9).com_preferencias(desligado);
        let tela = tela();
        abrir_folha(&mut app);
        app.registro();
        tocar_em(&mut app, &tela, tela.som.testar);
        assert_eq!(app.sons(), vec![efeito(Sinal::Combinacao, 0)]);
        assert_eq!(linhas_de_efeito(&app.registro()), vec!["PRISMA_M3 efeito sinal=combinacao elevacao=0".to_string()]);
        assert_eq!(app.painel().aviso, None);
        // Com os sons ligados também toca, e cada toque toca uma vez.
        tocar_em(&mut app, &tela, tela.som.sons_do_jogo);
        assert!(app.preferencias().sons_do_jogo);
        tocar_em(&mut app, &tela, tela.som.testar);
        tocar_em(&mut app, &tela, tela.som.testar);
        assert_eq!(app.sons().len(), 2);
    }

    #[test]
    fn liga_e_escolhe_da_musica_e_do_ambiente_gravam() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        let tela = tela();
        abrir_folha(&mut app);
        let (liga_musica, escolhe_musica) = (tela.som.liga_musica.unwrap(), tela.som.escolhe_musica.unwrap());
        let (liga_ambiente, escolhe_ambiente) = (tela.som.liga_ambiente.unwrap(), tela.som.escolhe_ambiente.unwrap());

        tocar_em(&mut app, &tela, liga_musica);
        assert!(app.preferencias().musica_ligada);
        assert_eq!(som_json(&pasta)["zen"]["music"], true);
        tocar_em(&mut app, &tela, escolhe_musica);
        assert_eq!(app.preferencias().musica, Musica::SpaceCity);
        assert_eq!(som_json(&pasta)["zen"]["musicTrack"], "spaceCity");
        tocar_em(&mut app, &tela, escolhe_musica);
        assert_eq!(app.preferencias().musica, Musica::MagicPuzzle, "depois da última, volta à primeira");
        tocar_em(&mut app, &tela, liga_musica);
        assert!(!app.preferencias().musica_ligada);
        assert_eq!(som_json(&pasta)["zen"]["music"], false);

        tocar_em(&mut app, &tela, liga_ambiente);
        assert!(app.preferencias().ambiente_ligado);
        assert_eq!(som_json(&pasta)["zen"]["ambience"], true);
        tocar_em(&mut app, &tela, escolhe_ambiente);
        assert_eq!(app.preferencias().ambiente, Ambiente::Floresta);
        assert_eq!(som_json(&pasta)["zen"]["ambienceSound"], "forest");
        for _ in 0..5 {
            tocar_em(&mut app, &tela, escolhe_ambiente);
        }
        assert_eq!(app.preferencias().ambiente, Ambiente::Chuva, "seis toques dão a volta completa");
        // Escolher a faixa não liga nem desliga nada.
        assert!(app.preferencias().ambiente_ligado && !app.preferencias().musica_ligada);
    }

    #[test]
    fn no_classico_os_controles_do_zen_nao_reagem() {
        let mut app = App::com_sementes(7, 9);
        app.apertar(Botao::Classico);
        abrir_folha(&mut app);
        let zen_so = [Botao::LigaMusica, Botao::EscolheMusica, Botao::LigaAmbiente, Botao::EscolheAmbiente, Botao::Respiracao];
        for botao in zen_so {
            app.apertar(botao);
        }
        assert_eq!(app.preferencias(), Preferencias::default());
        // E em lugar nenhum da folha do Clássico há um desses botões para tocar.
        let classico = tela_classico();
        let folha = classico.som.folha;
        for linha in 0..40 {
            for coluna in 0..40 {
                let (x, y) = (folha.x + folha.w * (coluna as f32 + 0.5) / 40.0, folha.y + folha.h * (linha as f32 + 0.5) / 40.0);
                let achado = app.botao_em(&classico, x, y);
                assert!(!achado.is_some_and(|botao| zen_so.contains(&botao)), "{achado:?} em ({x}, {y})");
            }
        }
    }

    #[test]
    fn reabrir_mantem_as_preferencias() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        let tela = tela();
        abrir_folha(&mut app);
        tocar_em(&mut app, &tela, tela.som.sons_do_jogo);
        let barra = tela.som.barra_musica;
        app.tocar(Evento::Baixo { x: x_da_barra(barra, 0.35), y: meio(barra).1 }, &tela);
        app.tocar(Evento::Cima { x: x_da_barra(barra, 0.35), y: meio(barra).1 }, &tela);
        tocar_em(&mut app, &tela, tela.som.liga_musica.unwrap());
        tocar_em(&mut app, &tela, tela.som.escolhe_musica.unwrap());
        tocar_em(&mut app, &tela, tela.som.liga_ambiente.unwrap());
        for _ in 0..3 {
            tocar_em(&mut app, &tela, tela.som.escolhe_ambiente.unwrap());
        }
        tocar_em(&mut app, &tela, tela.som.respiracao.unwrap());
        tocar_em(&mut app, &tela, tela.som.respiracao.unwrap());
        let esperado = app.preferencias();
        assert!(!esperado.sons_do_jogo);
        assert_eq!(esperado.volumes.musica, 35);
        assert_eq!((esperado.musica, esperado.ambiente, esperado.respiracao), (Musica::SpaceCity, Ambiente::Trovoada, Respiracao::Lenta));
        drop(app);

        let reaberto = abrir(&pasta);
        assert_eq!(reaberto.preferencias(), esperado);
        assert_eq!(reaberto.guia_respiracao().map(|estado| estado.fase), Some(Fase::Inspire), "a respiração volta ligada");
    }

    #[test]
    fn falha_ao_gravar_preferencias_vai_para_o_registro_uma_vez_e_o_jogo_segue() {
        let pasta = pasta_de_teste();
        let mut app = abrir(&pasta);
        app.registro();
        // O temporário do som.json vira uma pasta: gravar as preferências falha.
        fs::create_dir_all(pasta.0.join("som.json.tmp")).unwrap();
        let tela = tela();
        abrir_folha(&mut app);
        tocar_em(&mut app, &tela, tela.som.liga_musica.unwrap());
        tocar_em(&mut app, &tela, tela.som.liga_ambiente.unwrap());
        tocar_em(&mut app, &tela, tela.som.sons_do_jogo);
        let registro = app.registro();
        assert_eq!(linhas_m2(&registro, "erro_ao_gravar").len(), 1, "o erro é avisado uma vez só: {registro:?}");
        // A escolha vale mesmo sem gravar, e o jogo continua jogável.
        assert!(app.preferencias().musica_ligada && app.preferencias().ambiente_ligado && !app.preferencias().sons_do_jogo);
        app.apertar(Botao::FecharSom);
        jogar_inteira(&mut app);
        assert!(app.jogo().score > 0);
    }

    #[test]
    fn arquivo_de_preferencias_estragado_abre_com_padroes_e_o_jogo_funciona() {
        for conteudo in ["lixo", "", "[]", "{\"audio\":"] {
            let pasta = pasta_de_teste();
            fs::create_dir_all(&pasta.0).unwrap();
            fs::write(pasta.0.join("som.json"), conteudo).unwrap();
            let mut app = abrir(&pasta);
            assert_eq!(app.preferencias(), Preferencias::default(), "{conteudo:?}");
            assert_eq!(ler(&pasta, "som.json.corrupt"), conteudo, "a cópia do arquivo ruim");
            jogar_inteira(&mut app);
            assert!(app.jogo().score > 0, "o jogo funciona");
            abrir_folha(&mut app);
            let tela = tela();
            tocar_em(&mut app, &tela, tela.som.liga_musica.unwrap());
            assert!(app.preferencias().musica_ligada);
            assert_eq!(som_json(&pasta)["zen"]["music"], true, "e a próxima gravação conserta o arquivo");
        }
    }

    #[test]
    fn abrir_a_folha_no_meio_da_animacao_nao_muda_o_resultado() {
        let mut sem_folha = App::com_sementes(9, 9);
        let mut com_folha = App::com_sementes(9, 9);
        let (a, b) = boa_jogada(&mut sem_folha);
        arrastar(&mut sem_folha, a, b);
        arrastar(&mut com_folha, a, b);
        sem_folha.avancar(100.0);
        com_folha.avancar(100.0);
        abrir_folha(&mut com_folha);
        assert!(com_folha.ocupado(), "a animação continua por baixo da folha");
        for _ in 0..10_000 {
            if !sem_folha.ocupado() && !com_folha.ocupado() {
                break;
            }
            sem_folha.avancar(16.0);
            com_folha.avancar(16.0);
            assert_eq!(sem_folha.ocupado(), com_folha.ocupado(), "a folha não muda a duração");
            assert_eq!(sem_folha.painel().pontos, com_folha.painel().pontos);
        }
        assert!(com_folha.folha_aberta() && !com_folha.ocupado());
        assert_eq!(sem_folha.jogo().score, com_folha.jogo().score);
        assert_eq!(sem_folha.jogo().board, com_folha.jogo().board);
        // Os sons continuam: os mesmos efeitos, na mesma ordem.
        assert_eq!(sem_folha.sons(), com_folha.sons());
    }

    #[test]
    fn abrir_a_folha_com_fim_de_partida_ou_dialogo_aberto_nao_quebra() {
        let tela = tela();
        // Partida acabada: a folha abre, os controles funcionam e o cartão de fim continua lá.
        let mut app = App::com_sementes(7, 9);
        app.jogo_mut().ended = true;
        abrir_folha(&mut app);
        tocar_em(&mut app, &tela, tela.som.sons_do_jogo);
        tocar_em(&mut app, &tela, tela.som.testar);
        tocar_em(&mut app, &tela, tela.botao_jogar_novamente);
        assert!(!app.painel().dialogo, "o cartão de fim fica coberto pela folha");
        app.apertar(Botao::FecharSom);
        assert!(app.painel().fim.is_some());

        // Com a pergunta "Começar novo jogo?" aberta, o ♪ não abre a folha.
        let mut outro = App::com_sementes(7, 9);
        outro.apertar(Botao::NovoJogo);
        assert!(outro.painel().dialogo);
        outro.apertar(Botao::Som);
        tocar_em(&mut outro, &tela, tela.botao_som);
        assert!(!outro.folha_aberta());
        assert!(outro.painel().dialogo);
        outro.apertar(Botao::Cancelar);
        // E com a folha aberta, o Novo jogo não abre a pergunta.
        abrir_folha(&mut outro);
        outro.apertar(Botao::NovoJogo);
        assert!(!outro.painel().dialogo);
    }

    #[test]
    fn respiracao_anda_so_no_zen_e_recomeca_ao_voltar_ao_zen() {
        let preferencias = Preferencias { respiracao: Respiracao::Equilibrada, ..Preferencias::default() };
        let mut app = App::com_sementes(7, 9).com_preferencias(preferencias);
        let ponto = |app: &App| app.guia_respiracao().map(|estado| (estado.fase, estado.restante_s));
        assert_eq!(ponto(&app), Some((Fase::Inspire, 4)));
        app.avancar(1500.0);
        assert_eq!(ponto(&app), Some((Fase::Inspire, 3)));
        app.avancar(2600.0);
        assert_eq!(ponto(&app), Some((Fase::Expire, 4)), "4,1 s depois: acabou de começar a expirar");

        app.apertar(Botao::Classico);
        assert_eq!(app.guia_respiracao(), None, "no Clássico não há guia");
        app.avancar(3000.0);
        app.apertar(Botao::Zen);
        let estado = app.guia_respiracao().expect("de volta ao Zen o guia existe");
        assert_eq!((estado.fase, estado.restante_s, estado.progresso), (Fase::Inspire, 4, 0.0), "recomeça do Inspire");

        // Desligada, não há guia no Zen.
        let desligada = App::com_sementes(7, 9);
        assert_eq!(desligada.guia_respiracao(), None);
    }

    #[test]
    fn respiracao_recomeca_ao_mudar_a_opcao_e_no_novo_jogo() {
        let preferencias = Preferencias { respiracao: Respiracao::Equilibrada, ..Preferencias::default() };
        let mut app = App::com_sementes(7, 9).com_preferencias(preferencias);
        let tela = tela();
        let ponto = |app: &App| app.guia_respiracao().map(|estado| (estado.fase, estado.restante_s));
        app.avancar(2500.0);
        assert_eq!(ponto(&app), Some((Fase::Inspire, 2)));
        abrir_folha(&mut app);
        tocar_em(&mut app, &tela, tela.som.respiracao.unwrap());
        assert_eq!(app.preferencias().respiracao, Respiracao::Lenta);
        let estado = app.guia_respiracao().unwrap();
        assert_eq!((estado.fase, estado.restante_s, estado.progresso), (Fase::Inspire, 4, 0.0), "mudar a opção recomeça");
        app.apertar(Botao::FecharSom);
        // 5 s no ritmo lento: 1 s dentro da expiração de 6 s.
        app.avancar(5000.0);
        assert_eq!(ponto(&app), Some((Fase::Expire, 5)));
        // Novo jogo confirmado recomeça.
        app.apertar(Botao::NovoJogo);
        assert_eq!(ponto(&app), Some((Fase::Expire, 5)), "só a pergunta aberta ainda não recomeça");
        app.apertar(Botao::Confirmar);
        let estado = app.guia_respiracao().unwrap();
        assert_eq!((estado.fase, estado.restante_s, estado.progresso), (Fase::Inspire, 4, 0.0));
        // Cancelar a pergunta não recomeça.
        app.avancar(1000.0);
        app.apertar(Botao::NovoJogo);
        app.apertar(Botao::Cancelar);
        assert_eq!(ponto(&app), Some((Fase::Inspire, 3)));
        // Passando para "Desligada" e voltando, o guia some e reaparece do começo.
        abrir_folha(&mut app);
        tocar_em(&mut app, &tela, tela.som.respiracao.unwrap());
        assert_eq!(app.preferencias().respiracao, Respiracao::Desligada);
        assert_eq!(app.guia_respiracao(), None);
        tocar_em(&mut app, &tela, tela.som.respiracao.unwrap());
        assert_eq!(app.preferencias().respiracao, Respiracao::Equilibrada);
        assert_eq!(app.guia_respiracao().unwrap().restante_s, 4);
    }

    #[test]
    fn salto_de_tempo_na_respiracao_nao_da_panico() {
        let preferencias = Preferencias { respiracao: Respiracao::Lenta, ..Preferencias::default() };
        let mut app = App::com_sementes(7, 9).com_preferencias(preferencias);
        for dt in [1e30, f32::MAX, f32::INFINITY, f32::NAN, -5.0, 0.0, 123_456_789.0, 1e30] {
            app.avancar(dt);
            let estado = app.guia_respiracao().expect("o guia continua existindo");
            assert!((0.0..=1.0).contains(&estado.progresso), "dt {dt}: progresso {}", estado.progresso);
            assert!((1..=6).contains(&estado.restante_s), "dt {dt}: restante {}", estado.restante_s);
        }
        // Depois de tudo isso o tempo ainda anda normalmente.
        let antes = app.guia_respiracao().unwrap();
        app.avancar(16.0);
        let depois = app.guia_respiracao().unwrap();
        assert!(depois.fase != antes.fase || depois.progresso != antes.progresso || depois.restante_s != antes.restante_s || antes.progresso == 0.0);
    }

    #[test]
    fn erro_de_audio_aparece_e_some() {
        let texto = "O áudio não começou. Toque novamente em Música ou Ambiente para tentar de novo.";
        let mut app = App::com_sementes(7, 9);
        assert_eq!(app.aviso_de_audio(), None);
        app.erro_de_audio(Canal::Musica, Some("arquivo não encontrado".to_string()));
        assert_eq!(app.aviso_de_audio(), Some(texto));
        // Os dois canais falhando juntos: o aviso fica até os dois ficarem bem.
        app.erro_de_audio(Canal::Ambiente, Some("sem placa de som".to_string()));
        app.erro_de_audio(Canal::Musica, None);
        assert_eq!(app.aviso_de_audio(), Some(texto));
        app.erro_de_audio(Canal::Ambiente, None);
        assert_eq!(app.aviso_de_audio(), None);
        // Efeitos não têm aviso de áudio de fundo.
        app.erro_de_audio(Canal::Efeitos, Some("x".to_string()));
        assert_eq!(app.aviso_de_audio(), None);
        // Tocar em Música (ou Ambiente) de novo limpa o aviso daquele canal: é a nova tentativa.
        app.erro_de_audio(Canal::Musica, Some("falhou".to_string()));
        app.erro_de_audio(Canal::Ambiente, Some("falhou".to_string()));
        let tela = tela();
        abrir_folha(&mut app);
        tocar_em(&mut app, &tela, tela.som.liga_musica.unwrap());
        assert_eq!(app.aviso_de_audio(), Some(texto), "o Ambiente ainda falhou");
        tocar_em(&mut app, &tela, tela.som.liga_ambiente.unwrap());
        assert_eq!(app.aviso_de_audio(), None);
    }

    // ----- O que o som de fundo deve tocar -----

    fn tudo_ligado() -> Preferencias {
        Preferencias { musica_ligada: true, ambiente_ligado: true, ..Preferencias::default() }
    }

    #[test]
    fn som_desejado_e_vazio_no_classico_e_volta_ao_trocar_para_o_zen() {
        let mut app = App::com_sementes(7, 9).com_preferencias(tudo_ligado());
        let no_zen = app.som_desejado();
        assert_eq!(no_zen.musica, Some((Musica::CozyPuzzle, 0.70)));
        assert_eq!(no_zen.ambiente, Some((Ambiente::Chuva, 0.65)));
        app.apertar(Botao::Classico);
        assert_eq!(app.som_desejado(), Desejado::default(), "no Clássico não há música nem ambiente");
        app.apertar(Botao::Zen);
        assert_eq!(app.som_desejado(), no_zen, "ao voltar para o Zen, volta tudo como estava");
    }

    #[test]
    fn som_desejado_usa_o_volume_do_canal_dividido_por_100() {
        let mut preferencias = tudo_ligado();
        preferencias.volumes = Volumes { efeitos: 10, musica: 35, ambiente: 100 };
        let app = App::com_sementes(7, 9).com_preferencias(preferencias);
        let desejado = app.som_desejado();
        assert_eq!(desejado.musica, Some((Musica::CozyPuzzle, 35.0 / 100.0)));
        assert_eq!(desejado.ambiente, Some((Ambiente::Chuva, 1.0)));
        // O volume 0 continua pedindo a faixa (em silêncio): subir a barra não recarrega nada.
        let mut mudo = tudo_ligado();
        mudo.volumes = Volumes { efeitos: 0, musica: 0, ambiente: 0 };
        let desejado = App::com_sementes(7, 9).com_preferencias(mudo).som_desejado();
        assert_eq!((desejado.musica, desejado.ambiente), (Some((Musica::CozyPuzzle, 0.0)), Some((Ambiente::Chuva, 0.0))));
    }

    #[test]
    fn som_desejado_so_inclui_o_que_esta_ligado() {
        let com = |musica_ligada, ambiente_ligado| {
            let preferencias = Preferencias { musica_ligada, ambiente_ligado, musica: Musica::SpaceCity, ambiente: Ambiente::Trovoada, ..Preferencias::default() };
            App::com_sementes(7, 9).com_preferencias(preferencias).som_desejado()
        };
        assert_eq!(com(false, false), Desejado::default());
        assert_eq!(com(true, false), Desejado { musica: Some((Musica::SpaceCity, 0.70)), ambiente: None });
        assert_eq!(com(false, true), Desejado { musica: None, ambiente: Some((Ambiente::Trovoada, 0.65)) });
        // Os efeitos não entram: "Sons do jogo" desligado não mexe na música nem no ambiente.
        let preferencias = Preferencias { sons_do_jogo: false, ..tudo_ligado() };
        let desejado = App::com_sementes(7, 9).com_preferencias(preferencias).som_desejado();
        assert!(desejado.musica.is_some() && desejado.ambiente.is_some());
    }
}
