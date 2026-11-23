//! "Cenas": momentos fixos do jogo (sempre os mesmos) para tirar foto e conferir a olho.
//!
//! Cada cena é um `App` com sorte fixa levado a um instante conhecido, usando só o que o
//! jogador também usa (`tocar`, `apertar`, `avancar`). A exceção é o motor, que é mexido
//! direto nas cenas `nivel` e `fim` (para chegar logo ao nível 2 e ao fim de partida).

use crate::app::{App, Botao};
use crate::formato::Recordes;
use crate::input::Evento;
use crate::layout::{cell_rect, tela_jogo, TelaJogo};
use crate::linha::{duracao_ms, Passo};
use crate::preferencias::{Canal, Preferencias, Respiracao};
use prisma_engine::SIZE;

/// A tela de referência das cenas. Os toques são montados a partir dela; como o resultado
/// (qual casa foi tocada) não depende do tamanho da tela, a cena serve para qualquer tamanho.
fn tela() -> TelaJogo {
    tela_jogo(540.0, 960.0, true)
}

/// O ponto do meio de uma casa, em pixels da tela de referência.
fn centro(tela: &TelaJogo, casa: usize) -> (f32, f32) {
    let retangulo = cell_rect(tela.tabuleiro, casa / SIZE, casa % SIZE);
    (retangulo.x + retangulo.w / 2.0, retangulo.y + retangulo.h / 2.0)
}

/// Um toque rápido numa casa (encosta e solta no mesmo lugar).
fn tocar_casa(app: &mut App, casa: usize) {
    let tela = tela();
    let (x, y) = centro(&tela, casa);
    app.tocar(Evento::Baixo { x, y }, &tela);
    app.tocar(Evento::Cima { x, y }, &tela);
}

/// Faz a jogada que a Dica sugeriria, arrastando o dedo de uma casa até a outra.
fn jogada_boa(app: &mut App) {
    let tela = tela();
    let (a, b) = app.jogo_mut().hint().expect("o tabuleiro da cena tem jogada");
    let (ax, ay) = centro(&tela, a);
    let (bx, by) = centro(&tela, b);
    app.tocar(Evento::Baixo { x: ax, y: ay }, &tela);
    app.tocar(Evento::Mover { x: bx, y: by }, &tela);
    app.tocar(Evento::Cima { x: bx, y: by }, &tela);
}

/// Deixa a animação andar (de 1 em 1 ms) até o passo atual ser do tipo procurado.
fn avancar_ate(app: &mut App, procurado: fn(&Passo) -> bool) {
    for _ in 0..20_000 {
        if app.passo_atual().is_some_and(|(passo, _)| procurado(passo)) {
            return;
        }
        app.avancar(1.0);
    }
    panic!("a cena não chegou ao passo procurado");
}

/// Deixa o passo atual andar até `fracao` da duração dele (0,5 = no meio).
fn avancar_ate_a_fracao(app: &mut App, fracao: f32) {
    let (duracao, decorrido) = app
        .passo_atual()
        .map(|(passo, decorrido)| (duracao_ms(passo), decorrido))
        .expect("há um passo em andamento");
    app.avancar((fracao * duracao - decorrido).max(0.0));
}

/// As escolhas de som das cenas de som: volumes de fábrica, música ligada, ambiente desligado
/// (assim a foto mostra os dois estados), respiração no ritmo "4 s inspirar · 4 s expirar" e
/// vibração ligada (as cenas de som são de um aparelho com vibrador: ver `com_vibrador`).
fn preferencias_da_cena() -> Preferencias {
    Preferencias { musica_ligada: true, respiracao: Respiracao::Equilibrada, vibracao: true, ..Preferencias::default() }
}

/// Recordes de mentira para a foto da folha de Recordes: números grandes, com ponto de milhar.
fn recordes_da_cena() -> Recordes {
    Recordes { pontuacao: 1_234_567, nivel: 14, jogada: 8_900, terminadas: 23 }
}

/// Monta a cena `nome`, ou `None` se não existir. Os nomes são: `inicio`, `selecionada`,
/// `dica`, `troca`, `limpando`, `queda`, `nivel`, `fim`, `dialogo`, `classico`, `som`,
/// `som-classico`, `som-aviso`, `respiracao`, `recordes` e `creditos`.
pub fn cena(nome: &str) -> Option<App> {
    let mut app = App::com_sementes(7, 9);
    match nome {
        // Tabuleiro parado.
        "inicio" => {}
        // Uma casa tocada.
        "selecionada" => tocar_casa(&mut app, 27),
        // Botão Dica apertado.
        "dica" => app.apertar(Botao::Dica),
        // No meio da troca das duas pedras.
        "troca" => {
            jogada_boa(&mut app);
            avancar_ate_a_fracao(&mut app, 0.5);
        }
        // A 50% da primeira limpeza: as pedras estão encolhendo.
        "limpando" => {
            jogada_boa(&mut app);
            avancar_ate(&mut app, |passo| matches!(passo, Passo::Limpeza { .. }));
            avancar_ate_a_fracao(&mut app, 0.5);
        }
        // A 40% da primeira queda: pedras no ar.
        "queda" => {
            jogada_boa(&mut app);
            avancar_ate(&mut app, |passo| matches!(passo, Passo::Queda { .. }));
            avancar_ate_a_fracao(&mut app, 0.4);
        }
        // No meio da faixa "Nível 2" (a jogada passa dos 1800 pontos).
        "nivel" => {
            app.jogo_mut().score = 1790;
            jogada_boa(&mut app);
            avancar_ate(&mut app, |passo| matches!(passo, Passo::Nivel { .. }));
            avancar_ate_a_fracao(&mut app, 0.5);
        }
        // Clássico com a partida acabada: nível 3 e 4.321 pontos.
        "fim" => {
            app.apertar(Botao::Classico);
            let jogo = app.jogo_mut();
            jogo.ended = true;
            jogo.score = 4321;
            jogo.level = 3;
        }
        // A pergunta "Começar novo jogo?".
        "dialogo" => app.apertar(Botao::NovoJogo),
        // A aba do Clássico.
        "classico" => app.apertar(Botao::Classico),
        // A folha de Som no Zen: música ligada, ambiente desligado, respiração no ritmo 4-4.
        "som" => {
            app = app.com_preferencias(preferencias_da_cena()).com_vibrador(true);
            app.apertar(Botao::Som);
        }
        // A folha de Som no Clássico: só os efeitos.
        "som-classico" => {
            app = app.com_preferencias(preferencias_da_cena()).com_vibrador(true);
            app.apertar(Botao::Classico);
            app.apertar(Botao::Som);
        }
        // A folha de Som com o aviso de que a música não começou (um erro de áudio de mentira).
        "som-aviso" => {
            app = app.com_preferencias(preferencias_da_cena()).com_vibrador(true);
            app.apertar(Botao::Som);
            app.erro_de_audio(Canal::Musica, Some("arquivo não encontrado".to_string()));
        }
        // A folha do Menu em Recordes, no Clássico (que tem as quatro linhas), com números de verdade.
        "recordes" => {
            app = app.com_recordes(recordes_da_cena(), recordes_da_cena());
            app.apertar(Botao::Classico);
            app.apertar(Botao::Menu);
        }
        // A folha do Menu em Créditos.
        "creditos" => {
            app.apertar(Botao::Menu);
            app.apertar(Botao::AbaCreditos);
        }
        // O guia de respiração com a folha fechada, 2 s depois de começar (no meio do inspirar).
        "respiracao" => {
            app = app.com_preferencias(preferencias_da_cena());
            app.avancar(2000.0);
        }
        _ => return None,
    }
    Some(app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Folha;
    use crate::linha::Passo;
    use crate::preferencias::Fase;
    use prisma_engine::Mode;

    const NOMES: [&str; 16] = [
        "inicio",
        "selecionada",
        "dica",
        "troca",
        "limpando",
        "queda",
        "nivel",
        "fim",
        "dialogo",
        "classico",
        "som",
        "som-classico",
        "respiracao",
        "som-aviso",
        "recordes",
        "creditos",
    ];

    fn passo_da_cena(nome: &str) -> &'static str {
        let app = cena(nome).expect("a cena existe");
        match app.passo_atual().map(|(passo, _)| passo) {
            Some(Passo::Troca { .. }) => "troca",
            Some(Passo::Limpeza { .. }) => "limpeza",
            Some(Passo::Queda { .. }) => "queda",
            Some(Passo::Embaralhar { .. }) => "embaralhar",
            Some(Passo::Resgate { .. }) => "resgate",
            Some(Passo::Nivel { .. }) => "nivel",
            None => "nenhum",
        }
    }

    #[test]
    fn cada_cena_existe_e_nome_estranho_nao() {
        for nome in NOMES {
            assert!(cena(nome).is_some(), "cena {nome}");
        }
        assert!(cena("xyz").is_none());
        assert!(cena("").is_none());
        assert!(cena("INICIO").is_none());
    }

    #[test]
    fn cenas_tem_o_estado_esperado() {
        let inicio = cena("inicio").unwrap();
        assert!(!inicio.ocupado());
        assert_eq!(inicio.gestos().selecionada(), None);
        assert_eq!(inicio.modo(), Mode::Zen);

        assert!(cena("selecionada").unwrap().gestos().selecionada().is_some());
        assert!(cena("dica").unwrap().dica().is_some());
        assert_eq!(passo_da_cena("troca"), "troca");
        assert_eq!(passo_da_cena("limpando"), "limpeza");
        assert_eq!(passo_da_cena("queda"), "queda");
        assert_eq!(passo_da_cena("nivel"), "nivel");
        for nome in ["troca", "limpando", "queda", "nivel"] {
            assert!(cena(nome).unwrap().ocupado(), "cena {nome}");
        }
        assert_eq!(cena("fim").unwrap().painel().fim, Some(("3".to_string(), "4.321".to_string())));
        assert!(cena("dialogo").unwrap().painel().dialogo);
        assert_eq!(cena("classico").unwrap().modo(), Mode::Classic);
    }

    #[test]
    fn as_cenas_de_som_existem_e_montam() {
        use crate::preferencias::Respiracao;

        // A folha de Som no Zen: aberta, com a música ligada e o ambiente desligado.
        let som = cena("som").expect("a cena som existe");
        assert_eq!(som.modo(), Mode::Zen);
        assert!(som.folha_aberta());
        let preferencias = som.preferencias();
        assert!(preferencias.musica_ligada && !preferencias.ambiente_ligado);
        assert_eq!(preferencias.respiracao, Respiracao::Equilibrada);
        assert!(som.guia_respiracao().is_some());
        // O aparelho da cena tem vibrador, com a vibração ligada: a foto mostra a linha nova.
        assert!(som.tem_vibrador() && preferencias.vibracao);

        // No Clássico a folha só tem os efeitos, e nada de música ou ambiente.
        let classico = cena("som-classico").expect("a cena som-classico existe");
        assert_eq!(classico.modo(), Mode::Classic);
        assert!(classico.folha_aberta());
        assert!(classico.guia_respiracao().is_none());
        assert_eq!(classico.som_desejado(), Default::default());
        assert!(classico.tem_vibrador(), "a linha de vibração também existe no Clássico");

        // A folha com o aviso de que a música não começou.
        let aviso = cena("som-aviso").expect("a cena som-aviso existe");
        assert!(aviso.folha_aberta());
        assert!(aviso.aviso_da_folha().is_some());

        // O Menu em Recordes (no Clássico, com números) e em Créditos.
        let recordes = cena("recordes").expect("a cena recordes existe");
        assert_eq!((recordes.folha(), recordes.modo()), (Some(Folha::Recordes), Mode::Classic));
        assert_eq!(recordes.recordes(Mode::Classic).pontuacao, 1_234_567);
        assert_eq!(cena("creditos").expect("a cena creditos existe").folha(), Some(Folha::Creditos));

        // O guia de respiração com a folha fechada, 2 s depois de começar (no meio do inspirar).
        let respiracao = cena("respiracao").expect("a cena respiracao existe");
        assert_eq!(respiracao.modo(), Mode::Zen);
        assert!(!respiracao.folha_aberta());
        let guia = respiracao.guia_respiracao().expect("o guia está ligado");
        assert_eq!(guia.fase, Fase::Inspire);
        assert_eq!(guia.restante_s, 2);
    }

    #[test]
    fn as_cenas_sao_sempre_as_mesmas() {
        for nome in NOMES {
            let (a, b) = (cena(nome).unwrap(), cena(nome).unwrap());
            assert_eq!(a.jogo().board, b.jogo().board, "cena {nome}");
            assert_eq!(a.estagio(), b.estagio(), "cena {nome}");
        }
    }

    #[test]
    fn as_cenas_de_animacao_estao_no_meio_do_passo() {
        // "a meio caminho": nem no começo nem no fim do passo.
        for nome in ["troca", "limpando", "queda", "nivel"] {
            let app = cena(nome).unwrap();
            let (passo, decorrido) = app.passo_atual().expect("em andamento");
            let duracao = crate::linha::duracao_ms(passo);
            assert!(decorrido > 0.1 * duracao && decorrido < 0.9 * duracao, "cena {nome}: {decorrido} de {duracao}");
        }
    }

    #[test]
    fn a_cena_nivel_sobe_de_nivel() {
        let app = cena("nivel").unwrap();
        assert_eq!(app.estagio().nivel, 2);
        assert_eq!(app.painel().nivel, "2");
    }
}
