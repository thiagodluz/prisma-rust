//! "Cenas": momentos fixos do jogo (sempre os mesmos) para tirar foto e conferir a olho.
//!
//! Cada cena é um `App` com sorte fixa levado a um instante conhecido, usando só o que o
//! jogador também usa (`tocar`, `apertar`, `avancar`). A exceção é o motor, que é mexido
//! direto nas cenas `nivel` e `fim` (para chegar logo ao nível 2 e ao fim de partida).

use crate::app::{App, Botao};
use crate::formato::Recordes;
use crate::input::Evento;
use crate::layout::{cell_rect, tela_jogo, TelaJogo};
use crate::linha::{duracao_ms, Passo, TipoVisual};
use crate::preferencias::{Canal, Intensidade, Preferencias, Respiracao};
use prisma_engine::{MoveResult, TileKind, SIZE};

/// A tela de referência das cenas. Os toques são montados a partir dela; como o resultado
/// (qual casa foi tocada) não depende do tamanho da tela, a cena serve para qualquer tamanho.
fn tela() -> TelaJogo {
    tela_jogo(540.0, 960.0, true)
}

/// O ponto do meio de uma casa, em pixels da tela de referência.
fn centro(tela: &TelaJogo, casa: usize) -> (f32, f32) {
    let retangulo = cell_rect(tela.grade, casa / SIZE, casa % SIZE);
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
fn avancar_ate(app: &mut App, procurado: impl Fn(&Passo) -> bool) {
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

/// Prepara uma cena de partículas: `atalho` mexe no motor (um atalho `cheat_*`) e devolve o
/// resultado, que a cena anima; depois a animação anda até uma limpeza em que o efeito
/// `procurado` aparece. A foto fica `ms` milissegundos depois do começo desse passo.
fn cena_de_particulas(
    app: &mut App,
    atalho: impl FnOnce(&mut prisma_engine::Game) -> MoveResult,
    procurado: TipoVisual,
    ms: f32,
) {
    let antes = app.jogo().board;
    let resultado = atalho(app.jogo_mut());
    app.animar_atalho(antes, &resultado);
    let tem_o_efeito = |passo: &Passo| match passo {
        Passo::Limpeza { visuais, .. } => visuais.iter().any(|visual| visual.tipo == procurado),
        _ => false,
    };
    avancar_ate(app, tem_o_efeito);
    let decorrido = app.passo_atual().map_or(0.0, |(_, decorrido)| decorrido);
    app.avancar((ms - decorrido).max(0.0));
}

/// Troca a pedra da casa 27 por uma especial de `tipo` e explode o tabuleiro (cada especial
/// dispara uma vez), do jeito que o atalho de testes do motor faz.
fn explodir_com_especial(jogo: &mut prisma_engine::Game, tipo: TileKind) -> MoveResult {
    assert!(jogo.cheat_spawn_special(27, tipo), "a casa 27 aceita a pedra especial");
    jogo.cheat_explode()
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

/// Os do Zen na mesma foto: diferentes dos do Clássico, para a foto mostrar que cada bloco lê o seu.
fn recordes_do_zen_da_cena() -> Recordes {
    Recordes { pontuacao: 98_765, nivel: 6, jogada: 3_200, terminadas: 0 }
}

/// Libera e abre a aba Ferramentas (7 toques na versão, nos Créditos).
fn liberar_ferramentas(app: &mut App) {
    app.apertar(Botao::Menu);
    app.apertar(Botao::AbaCreditos);
    for _ in 0..7 {
        app.apertar(Botao::Versao);
    }
    app.apertar(Botao::AbaFerramentas);
}

/// Monta a cena `nome`, ou `None` se não existir. Os nomes são: `inicio`, `selecionada`,
/// `dica`, `troca`, `limpando`, `queda`, `nivel`, `fim`, `dialogo`, `classico`, `som`,
/// `som-classico`, `som-aviso`, `respiracao`, `recordes`, `creditos`, `particulas-pulso`,
/// `particulas-raio`, `particulas-espectro`, `particulas-espectro-suave`,
/// `particulas-espectro-vibrante`, `particulas-supernova`, `ferramentas` e `escolher-casa`.
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
            app = app.com_recordes(recordes_do_zen_da_cena(), recordes_da_cena());
            app.apertar(Botao::Classico);
            app.apertar(Botao::Menu);
        }
        // A folha do Menu em Créditos.
        "creditos" => {
            app.apertar(Botao::Menu);
            app.apertar(Botao::AbaCreditos);
        }
        // A aba Ferramentas aberta, depois de um cheat (o Desfazer aceso e a partida marcada).
        "ferramentas" => {
            liberar_ferramentas(&mut app);
            app.apertar(Botao::Mais100);
            app.avancar(3000.0);
            app.apertar(Botao::Menu);
            app.apertar(Botao::AbaFerramentas);
        }
        // O Pulso esperando o toque numa casa.
        "escolher-casa" => {
            liberar_ferramentas(&mut app);
            app.avancar(3000.0);
            app.apertar(Botao::Pulso);
        }
        // O guia de respiração com a folha fechada, 2 s depois de começar (no meio do inspirar).
        "respiracao" => {
            app = app.com_preferencias(preferencias_da_cena());
            app.avancar(2000.0);
        }
        // As partículas de cada especial, no meio da limpeza em que ela dispara. As cenas usam os
        // atalhos do motor (como `nivel` e `fim`), porque achar jogadas reais que façam cada uma
        // dependeria da sorte; o que se vê na tela é o mesmo.
        "particulas-pulso" => {
            cena_de_particulas(&mut app, |jogo| explodir_com_especial(jogo, TileKind::Burst), TipoVisual::Pulso, 60.0)
        }
        "particulas-raio" => {
            cena_de_particulas(&mut app, |jogo| explodir_com_especial(jogo, TileKind::Cross), TipoVisual::Raio, 60.0)
        }
        "particulas-espectro" => cena_de_particulas(
            &mut app,
            |jogo| explodir_com_especial(jogo, TileKind::Spectrum),
            TipoVisual::Espectro,
            60.0,
        ),
        // O mesmo Espectro nas outras duas intensidades do Zen (para comparar com a de cima).
        "particulas-espectro-suave" | "particulas-espectro-vibrante" => {
            let intensidade =
                if nome == "particulas-espectro-suave" { Intensidade::Suave } else { Intensidade::Vibrante };
            app = app.com_preferencias(Preferencias { intensidade, ..Preferencias::default() });
            cena_de_particulas(
                &mut app,
                |jogo| explodir_com_especial(jogo, TileKind::Spectrum),
                TipoVisual::Espectro,
                60.0,
            );
        }
        "particulas-supernova" => {
            cena_de_particulas(&mut app, |jogo| jogo.cheat_supernova(27), TipoVisual::Supernova, 330.0)
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

    const NOMES: [&str; 24] = [
        "inicio",
        "ferramentas",
        "escolher-casa",
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
        "particulas-pulso",
        "particulas-raio",
        "particulas-espectro",
        "particulas-supernova",
        "particulas-espectro-suave",
        "particulas-espectro-vibrante",
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
        assert_eq!(recordes.recordes(Mode::Zen).pontuacao, 98_765, "a foto mostra números diferentes em cada bloco");
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
    fn as_cenas_do_menu_secreto() {
        let ferramentas = cena("ferramentas").unwrap();
        assert!(ferramentas.ferramentas_liberadas());
        assert_eq!(ferramentas.folha(), Some(Folha::Ferramentas));
        assert!(ferramentas.ferramentas().pode_desfazer, "a foto mostra o Desfazer aceso");
        let escolher = cena("escolher-casa").unwrap();
        assert_eq!(escolher.folha(), None);
        assert_eq!(escolher.painel().aviso.as_deref(), Some("Toque numa casa"));
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
    fn cenas_de_particulas_existem_e_estao_no_passo_de_limpeza_certo() {
        use crate::linha::TipoVisual;

        let esperadas = [
            ("particulas-pulso", TipoVisual::Pulso),
            ("particulas-raio", TipoVisual::Raio),
            ("particulas-espectro", TipoVisual::Espectro),
            ("particulas-supernova", TipoVisual::Supernova),
        ];
        for (nome, tipo) in esperadas {
            let app = cena(nome).unwrap_or_else(|| panic!("a cena {nome} existe"));
            let (passo, decorrido) = app.passo_atual().unwrap_or_else(|| panic!("{nome}: há um passo em andamento"));
            let Passo::Limpeza { visuais, .. } = passo else { panic!("{nome}: o passo é de limpeza") };
            assert!(visuais.iter().any(|visual| visual.tipo == tipo), "{nome}: falta o visual {tipo:?} em {visuais:?}");
            // No meio do passo: as partículas já saíram e ainda não sumiram.
            let duracao = crate::linha::duracao_ms(passo);
            assert!(decorrido > 0.1 * duracao && decorrido < 0.9 * duracao, "{nome}: {decorrido} de {duracao}");
            let (celulas, intensidade) = match passo {
                Passo::Limpeza { celulas, intensidade, .. } => (celulas, *intensidade),
                _ => unreachable!(),
            };
            let lista = crate::particulas::particulas(visuais, celulas, decorrido, intensidade);
            assert!(lista.iter().any(|particula| particula.alfa > 0.2), "{nome}: tem partícula bem visível");
        }
    }

    #[test]
    fn as_cenas_do_espectro_mostram_cada_intensidade() {
        use crate::preferencias::Intensidade;

        for (nome, esperada) in [
            ("particulas-espectro-suave", Intensidade::Suave),
            ("particulas-espectro", Intensidade::Padrao),
            ("particulas-espectro-vibrante", Intensidade::Vibrante),
        ] {
            let app = cena(nome).unwrap_or_else(|| panic!("a cena {nome} existe"));
            assert_eq!(app.intensidade(), esperada, "{nome}");
            let (passo, _) = app.passo_atual().expect("em andamento");
            let Passo::Limpeza { visuais, intensidade, .. } = passo else { panic!("{nome}: o passo é de limpeza") };
            assert_eq!(*intensidade, esperada, "{nome}: a limpeza leva a intensidade da cena");
            assert!(visuais.iter().any(|visual| visual.tipo == TipoVisual::Espectro), "{nome}");
        }
    }

    #[test]
    fn a_cena_nivel_sobe_de_nivel() {
        let app = cena("nivel").unwrap();
        assert_eq!(app.estagio().nivel, 2);
        assert_eq!(app.painel().nivel, "2");
    }
}
