//! Formato dos arquivos de partida e de recordes: texto JSON, igual ao do Prisma em JS.
//!
//! Aqui só se transforma texto em dados e dados em texto. Quem confere se a partida faz
//! sentido (tabuleiro 8x8, cores, pontos) é o `Game::restore` do motor; este módulo só confere
//! o **formato**: se o texto é JSON, se os campos têm o tipo certo. Qualquer coisa fora do
//! formato recusa o save inteiro (devolve `Err` com o motivo), nunca dá pânico.
//!
//! `serde_json::Value` é "um JSON qualquer" já lido: pode ser objeto, lista, número, texto,
//! `true`/`false` ou `null`. Vamos abrindo o `Value` campo a campo e conferindo cada um.

use prisma_engine::{SavedGame, SavedMode, SavedTile, TileKind};
use serde_json::{json, Map, Value};

/// O maior número "seguro" do JS (2^53 − 1): os recordes do JS nunca passam disso.
const MAX_SEGURO: i64 = 9_007_199_254_740_991;

/// Os quatro recordes de um modo, os mesmos números que o Prisma em JS guarda.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recordes {
    /// Maior pontuação de uma partida (`bestScore`).
    pub pontuacao: u64,
    /// Maior nível alcançado (`bestLevel`).
    pub nivel: u32,
    /// Maior ganho de pontos numa só jogada (`bestMove`).
    pub jogada: u64,
    /// Partidas terminadas (`finished`; só o Clássico termina).
    pub terminadas: u32,
}

impl Recordes {
    /// Sem nenhum recorde ainda (o nível começa em 1, como no JS).
    pub fn zerados() -> Recordes {
        Recordes { pontuacao: 0, nivel: 1, jogada: 0, terminadas: 0 }
    }

    /// Depois de uma jogada: sobe a pontuação, o nível e o maior ganho se forem maiores.
    /// `terminou` soma 1 às partidas terminadas (quem chama só passa `true` no Clássico).
    /// Devolve se algo mudou, para quem chama saber se precisa gravar.
    pub fn registrar(&mut self, pontos: u64, nivel: u32, ganho_da_jogada: u64, terminou: bool) -> bool {
        let mut mudou = self.alcancar(pontos, nivel);
        if ganho_da_jogada > self.jogada {
            self.jogada = ganho_da_jogada;
            mudou = true;
        }
        if terminou {
            // `saturating_add` soma sem estourar: no máximo, fica no maior número possível.
            self.terminadas = self.terminadas.saturating_add(1);
            mudou = true;
        }
        mudou
    }

    /// Com o jogo parado: só sobe a pontuação e o nível para o placar visto agora (o `hud()`
    /// do JS faz o mesmo). Devolve se mudou.
    pub fn alcancar(&mut self, pontos: u64, nivel: u32) -> bool {
        let mut mudou = false;
        if pontos > self.pontuacao {
            self.pontuacao = pontos;
            mudou = true;
        }
        if nivel > self.nivel {
            self.nivel = nivel;
            mudou = true;
        }
        mudou
    }
}

/// Lê um JSON e exige que seja um objeto (`{ ... }`). `o_que` entra na mensagem de erro.
fn ler_objeto(texto: &str, o_que: &str) -> Result<Map<String, Value>, String> {
    match serde_json::from_str::<Value>(texto) {
        Ok(Value::Object(objeto)) => Ok(objeto),
        Ok(_) => Err(format!("{o_que}: o JSON não é um objeto")),
        Err(erro) => Err(format!("{o_que}: o texto não é um JSON válido (linha {}, coluna {})", erro.line(), erro.column())),
    }
}

/// Um número inteiro de verdade. `as_i64` já recusa fracionário (`1.5`), expoente (`1e3`),
/// texto (`"10"`) e número grande demais para caber num `i64`.
fn inteiro(valor: &Value, nome: &str) -> Result<i64, String> {
    valor.as_i64().ok_or_else(|| format!("'{nome}' não é um número inteiro"))
}

/// Campo de número inteiro obrigatório.
fn inteiro_obrigatorio(objeto: &Map<String, Value>, nome: &str) -> Result<i64, String> {
    match objeto.get(nome) {
        Some(valor) => inteiro(valor, nome),
        None => Err(format!("falta o campo '{nome}'")),
    }
}

/// Campo de número inteiro que pode faltar (`None`); se está lá, tem de ser inteiro.
fn inteiro_opcional(objeto: &Map<String, Value>, nome: &str) -> Result<Option<i64>, String> {
    objeto.get(nome).map(|valor| inteiro(valor, nome)).transpose()
}

/// Campo `true`/`false` que pode faltar (vale `false`); se está lá, tem de ser mesmo booleano.
fn booleano(objeto: &Map<String, Value>, nome: &str) -> Result<bool, String> {
    match objeto.get(nome) {
        None => Ok(false),
        Some(Value::Bool(valor)) => Ok(*valor),
        Some(_) => Err(format!("'{nome}' não é true nem false")),
    }
}

fn nome_do_tipo(tipo: TileKind) -> &'static str {
    match tipo {
        TileKind::Normal => "normal",
        TileKind::Burst => "burst",
        TileKind::Cross => "cross",
        TileKind::Spectrum => "spectrum",
    }
}

fn tipo_pelo_nome(nome: &str) -> Option<TileKind> {
    match nome {
        "normal" => Some(TileKind::Normal),
        "burst" => Some(TileKind::Burst),
        "cross" => Some(TileKind::Cross),
        "spectrum" => Some(TileKind::Spectrum),
        _ => None,
    }
}

/// Uma casa do tabuleiro: um objeto `{"id", "type", "color"}` (save de hoje) ou só o número da
/// cor (save antigo).
fn ler_pedra(valor: &Value) -> Result<SavedTile, String> {
    let Value::Object(objeto) = valor else {
        return inteiro(valor, "pedra").map(SavedTile::Legacy);
    };
    let id = inteiro_obrigatorio(objeto, "id")?;
    let tipo = match objeto.get("type") {
        Some(Value::String(nome)) => tipo_pelo_nome(nome).ok_or_else(|| format!("tipo de pedra desconhecido: '{nome}'"))?,
        Some(_) => return Err("o 'type' da pedra não é um texto".to_string()),
        None => return Err("falta o 'type' da pedra".to_string()),
    };
    let cor = match (objeto.get("color"), tipo) {
        (Some(valor), _) if !valor.is_null() => Some(inteiro(valor, "color")?),
        // Só o Espectro não tem cor: lá `null` (ou faltar) é o normal.
        (_, TileKind::Spectrum) => None,
        _ => return Err("pedra com cor vazia (só o Espectro pode)".to_string()),
    };
    Ok(SavedTile::Full { id, kind: tipo, color: cor })
}

/// Transforma a partida em texto JSON. O Clássico é sempre gravado como `"classic"` (o nome
/// antigo `"endless"` só é lido, nunca escrito).
pub fn partida_para_json(salvo: &SavedGame) -> String {
    let modo = match salvo.mode {
        SavedMode::Zen => "zen",
        SavedMode::Classic | SavedMode::Endless => "classic",
    };
    let tabuleiro: Vec<Vec<Value>> = salvo
        .board
        .iter()
        .map(|linha| {
            linha
                .iter()
                .map(|pedra| match pedra {
                    SavedTile::Legacy(cor) => json!(cor),
                    SavedTile::Full { id, kind, color } => json!({ "id": id, "type": nome_do_tipo(*kind), "color": color }),
                })
                .collect()
        })
        .collect();
    let mut objeto = Map::new();
    objeto.insert("mode".to_string(), json!(modo));
    objeto.insert("score".to_string(), json!(salvo.score));
    objeto.insert("board".to_string(), json!(tabuleiro));
    objeto.insert("ended".to_string(), json!(salvo.ended));
    objeto.insert("cheatsUsed".to_string(), json!(salvo.cheats_used));
    objeto.insert("noDefeat".to_string(), json!(salvo.no_defeat));
    // Estes quatro só entram se existirem (um save antigo não os tem).
    for (nome, valor) in [
        ("progressionVersion", salvo.progression_version),
        ("level", salvo.level),
        ("levelStartScore", salvo.level_start_score),
        ("progressionOffset", salvo.progression_offset),
    ] {
        if let Some(valor) = valor {
            objeto.insert(nome.to_string(), json!(valor));
        }
    }
    Value::Object(objeto).to_string()
}

/// Lê o texto de uma partida. `Err` traz o motivo curto, em português, de ter recusado o save
/// inteiro. Campos que o formato não conhece são ignorados.
pub fn json_para_partida(texto: &str) -> Result<SavedGame, String> {
    let objeto = ler_objeto(texto, "partida")?;
    let modo = match objeto.get("mode") {
        Some(Value::String(nome)) => match nome.as_str() {
            "zen" => SavedMode::Zen,
            "classic" => SavedMode::Classic,
            "endless" => SavedMode::Endless,
            outro => return Err(format!("modo desconhecido: '{outro}'")),
        },
        Some(_) => return Err("o 'mode' não é um texto".to_string()),
        None => return Err("falta o campo 'mode'".to_string()),
    };
    let linhas = match objeto.get("board") {
        Some(Value::Array(linhas)) => linhas,
        Some(_) => return Err("o 'board' não é uma lista".to_string()),
        None => return Err("falta o campo 'board'".to_string()),
    };
    let mut tabuleiro = Vec::with_capacity(linhas.len());
    for linha in linhas {
        let Value::Array(casas) = linha else {
            return Err("uma linha do 'board' não é uma lista".to_string());
        };
        tabuleiro.push(casas.iter().map(ler_pedra).collect::<Result<Vec<_>, _>>()?);
    }
    Ok(SavedGame {
        mode: modo,
        score: inteiro_obrigatorio(&objeto, "score")?,
        board: tabuleiro,
        progression_version: inteiro_opcional(&objeto, "progressionVersion")?,
        level: inteiro_opcional(&objeto, "level")?,
        level_start_score: inteiro_opcional(&objeto, "levelStartScore")?,
        progression_offset: inteiro_opcional(&objeto, "progressionOffset")?,
        cheats_used: booleano(&objeto, "cheatsUsed")?,
        no_defeat: booleano(&objeto, "noDefeat")?,
        ended: booleano(&objeto, "ended")?,
    })
}

/// Transforma os recordes em texto JSON (`bestScore`, `bestLevel`, `bestMove`, `finished`).
pub fn recordes_para_json(recordes: &Recordes) -> String {
    json!({
        "bestScore": recordes.pontuacao,
        "bestLevel": recordes.nivel,
        "bestMove": recordes.jogada,
        "finished": recordes.terminadas,
    })
    .to_string()
}

/// Lê os recordes. Só recusa se o texto não for um objeto JSON; um campo ruim (fracionário,
/// negativo, texto, grande demais) vale 0, como o `safe` do JS, e o nível vale no mínimo 1.
pub fn json_para_recordes(texto: &str) -> Result<Recordes, String> {
    let objeto = ler_objeto(texto, "recordes")?;
    // Um inteiro seguro do JS (0 a 2^53 − 1), ou 0.
    let seguro = |nome: &str| -> u64 {
        objeto
            .get(nome)
            .and_then(Value::as_i64)
            .filter(|numero| (0..=MAX_SEGURO).contains(numero))
            .map_or(0, |numero| numero as u64)
    };
    Ok(Recordes {
        pontuacao: seguro("bestScore"),
        nivel: u32::try_from(seguro("bestLevel")).unwrap_or(0).max(1),
        jogada: seguro("bestMove"),
        terminadas: u32::try_from(seguro("finished")).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use prisma_engine::{lcg_random, Game, Mode};

    /// Um jogo com semente fixa e algumas jogadas já feitas (sempre a primeira dica).
    fn jogo_de_teste(semente: u32, modo: Mode, jogadas: usize) -> Game {
        let mut jogo = Game::with_random(lcg_random(semente));
        jogo.new_game(modo);
        for _ in 0..jogadas {
            if let Some((a, b)) = jogo.hint() {
                jogo.play(a, b);
            }
        }
        jogo
    }

    /// Um save pequeno e válido para o leitor (o formato 8x8 só quem confere é o `restore`),
    /// com `pedra` como única casa do tabuleiro.
    fn com_pedra(pedra: &str) -> String {
        format!(r#"{{"mode":"zen","score":0,"board":[[{pedra}]]}}"#)
    }

    const PEDRA_BOA: &str = r#"{"id":1,"type":"normal","color":0}"#;

    #[test]
    fn ida_e_volta_da_partida_e_identica() {
        for semente in 1..=20u32 {
            let modo = if semente % 2 == 0 { Mode::Zen } else { Mode::Classic };
            let jogo = jogo_de_teste(semente, modo, (semente % 6) as usize);
            let salvo = jogo.to_saved();
            let texto = partida_para_json(&salvo);
            let lido = json_para_partida(&texto).unwrap_or_else(|e| panic!("semente {semente}: {e}"));
            assert_eq!(lido, salvo, "semente {semente}");

            let mut outro = Game::new();
            assert!(outro.restore(&lido), "semente {semente}: o motor recusou o save lido");
            assert_eq!(outro.board, jogo.board, "semente {semente}");
            assert_eq!(outro.score, jogo.score, "semente {semente}");
            assert_eq!(outro.level, jogo.level, "semente {semente}");
            assert_eq!(outro.level_start_score, jogo.level_start_score, "semente {semente}");
        }
    }

    #[test]
    fn modo_classico_e_gravado_como_classic_e_endless_e_lido() {
        let texto = partida_para_json(&jogo_de_teste(3, Mode::Classic, 2).to_saved());
        assert!(texto.contains(r#""mode":"classic""#), "{texto}");
        assert!(!texto.contains("endless"));
        let antigo = texto.replace(r#""mode":"classic""#, r#""mode":"endless""#);
        assert_eq!(json_para_partida(&antigo).unwrap().mode, SavedMode::Endless);
        assert_eq!(json_para_partida(&texto).unwrap().mode, SavedMode::Classic);

        // Um save que veio como `endless` e foi lido: ao gravar de novo vira `classic`.
        let mut lido = json_para_partida(&antigo).unwrap();
        lido.mode = SavedMode::Endless;
        let regravado = partida_para_json(&lido);
        assert!(regravado.contains(r#""mode":"classic""#) && !regravado.contains("endless"), "{regravado}");
    }

    #[test]
    fn le_um_save_do_js_com_pecas_de_todos_os_tipos() {
        let texto = r#"{
            "mode": "classic", "score": 4200, "ended": true, "cheatsUsed": true, "noDefeat": false,
            "progressionVersion": 4, "level": 3, "levelStartScore": 3000, "progressionOffset": 0,
            "board": [
                [{"id": 1, "type": "normal", "color": 0}, {"id": 2, "type": "burst", "color": 5}],
                [{"id": 3, "type": "cross", "color": 6}, {"id": 4, "type": "spectrum", "color": null}]
            ],
            "campoDesconhecido": [1, 2, 3]
        }"#;
        let salvo = json_para_partida(texto).unwrap();
        assert_eq!(salvo.mode, SavedMode::Classic);
        assert_eq!(salvo.score, 4200);
        assert_eq!(salvo.progression_version, Some(4));
        assert_eq!(salvo.level, Some(3));
        assert_eq!(salvo.level_start_score, Some(3000));
        assert_eq!(salvo.progression_offset, Some(0));
        assert!(salvo.ended && salvo.cheats_used && !salvo.no_defeat);
        let cheia = |id, kind, color| SavedTile::Full { id, kind, color };
        assert_eq!(
            salvo.board,
            vec![
                vec![cheia(1, TileKind::Normal, Some(0)), cheia(2, TileKind::Burst, Some(5))],
                vec![cheia(3, TileKind::Cross, Some(6)), cheia(4, TileKind::Spectrum, None)],
            ]
        );
    }

    #[test]
    fn le_tabuleiro_antigo_de_numeros() {
        let salvo = json_para_partida(r#"{"mode":"zen","score":5,"board":[[0,6],[3,1]]}"#).unwrap();
        assert_eq!(
            salvo.board,
            vec![
                vec![SavedTile::Legacy(0), SavedTile::Legacy(6)],
                vec![SavedTile::Legacy(3), SavedTile::Legacy(1)]
            ]
        );
        assert!(json_para_partida(r#"{"mode":"zen","score":5,"board":[[1.5]]}"#).is_err());
    }

    #[test]
    fn recusa_o_save_inteiro_quando() {
        let valido = com_pedra(PEDRA_BOA);
        assert!(json_para_partida(&valido).is_ok(), "o save de base tem de ser válido");
        let casos: Vec<(&str, String)> = vec![
            ("texto vazio", String::new()),
            ("lixo", "lixo".to_string()),
            ("JSON cortado", r#"{"mode":"zen","score":1"#.to_string()),
            ("lista em vez de objeto", "[]".to_string()),
            ("BOM antes do {", format!("\u{feff}{valido}")),
            ("modo hard", r#"{"mode":"hard","score":0,"board":[]}"#.to_string()),
            ("modo número", r#"{"mode":1,"score":0,"board":[]}"#.to_string()),
            ("sem modo", r#"{"score":0,"board":[]}"#.to_string()),
            ("sem score", r#"{"mode":"zen","board":[]}"#.to_string()),
            ("sem tabuleiro", r#"{"mode":"zen","score":0}"#.to_string()),
            ("score fracionário", r#"{"mode":"zen","score":12.5,"board":[]}"#.to_string()),
            ("score com expoente", r#"{"mode":"zen","score":1e3,"board":[]}"#.to_string()),
            ("score em texto", r#"{"mode":"zen","score":"10","board":[]}"#.to_string()),
            ("level -1.0", r#"{"mode":"zen","score":0,"level":-1.0,"board":[]}"#.to_string()),
            ("ended 1", r#"{"mode":"zen","score":0,"ended":1,"board":[]}"#.to_string()),
            ("cheatsUsed em texto", r#"{"mode":"zen","score":0,"cheatsUsed":"true","board":[]}"#.to_string()),
            ("noDefeat 0", r#"{"mode":"zen","score":0,"noDefeat":0,"board":[]}"#.to_string()),
            ("pedra de gelo", com_pedra(r#"{"id":1,"type":"gelo","color":0}"#)),
            ("pedra sem type", com_pedra(r#"{"id":1,"color":0}"#)),
            ("cor 1.5", com_pedra(r#"{"id":1,"type":"normal","color":1.5}"#)),
            ("cor null numa pedra comum", com_pedra(r#"{"id":1,"type":"normal","color":null}"#)),
            ("cor em texto", com_pedra(r#"{"id":1,"type":"normal","color":"1"}"#)),
            ("id em texto", com_pedra(r#"{"id":"a","type":"normal","color":0}"#)),
            ("tabuleiro que é número", r#"{"mode":"zen","score":0,"board":5}"#.to_string()),
            ("linha do tabuleiro que é número", r#"{"mode":"zen","score":0,"board":[5]}"#.to_string()),
        ];
        for (nome, texto) in casos {
            match json_para_partida(&texto) {
                Ok(salvo) => panic!("{nome}: devia recusar, mas leu {salvo:?}"),
                Err(motivo) => assert!(!motivo.trim().is_empty(), "{nome}: o motivo veio vazio"),
            }
        }
    }

    #[test]
    fn numeros_grandes_nao_dao_pane() {
        let com_score = |score: &str| json_para_partida(&format!(r#"{{"mode":"zen","score":{score},"board":[]}}"#));
        assert_eq!(com_score("9007199254740991").unwrap().score, 9_007_199_254_740_991);
        // Não cabe em i64: recusa em vez de dar pânico ou arredondar.
        assert!(com_score("99999999999999999999999").is_err());
        // Negativo é problema do `restore`, não do leitor.
        assert_eq!(com_score("-5").unwrap().score, -5);
    }

    #[test]
    fn campos_ausentes_e_extras() {
        let salvo = json_para_partida(r#"{"mode":"zen","score":7,"board":[],"outro":{"a":1}}"#).unwrap();
        assert_eq!(salvo.progression_version, None);
        assert_eq!(salvo.level, None);
        assert_eq!(salvo.level_start_score, None);
        assert_eq!(salvo.progression_offset, None);
        assert!(!salvo.cheats_used && !salvo.no_defeat && !salvo.ended);
        assert_eq!(salvo.score, 7);
    }

    #[test]
    fn recordes_ida_e_volta_e_leitura_gentil() {
        let grandes = Recordes { pontuacao: 9_007_199_254_740_991, nivel: 7, jogada: 123, terminadas: 4 };
        assert_eq!(json_para_recordes(&recordes_para_json(&grandes)), Ok(grandes));
        assert_eq!(json_para_recordes(&recordes_para_json(&Recordes::zerados())), Ok(Recordes::zerados()));

        let ruim = r#"{"bestScore":-1,"bestLevel":0,"bestMove":2.5,"finished":"x"}"#;
        assert_eq!(json_para_recordes(ruim), Ok(Recordes { pontuacao: 0, nivel: 1, jogada: 0, terminadas: 0 }));
        let enorme = r#"{"bestScore":9007199254740991,"bestLevel":99999999999,"finished":4294967296}"#;
        let lido = json_para_recordes(enorme).unwrap();
        assert_eq!(lido.pontuacao, 9_007_199_254_740_991);
        assert_eq!((lido.nivel, lido.terminadas), (1, 0), "acima de u32::MAX vale 0 (nível, no mínimo 1)");
        assert_eq!(json_para_recordes("{}"), Ok(Recordes::zerados()));
        for texto in ["lixo", "", "[]", "5", "\u{feff}{}"] {
            assert!(json_para_recordes(texto).is_err(), "{texto:?} devia ser recusado");
        }
        assert_eq!(Recordes::zerados(), Recordes { pontuacao: 0, nivel: 1, jogada: 0, terminadas: 0 });
    }

    #[test]
    fn registrar_sobe_e_nunca_desce() {
        let mut r = Recordes::zerados();
        assert!(r.registrar(500, 2, 120, false));
        assert_eq!(r, Recordes { pontuacao: 500, nivel: 2, jogada: 120, terminadas: 0 });
        assert!(!r.registrar(100, 1, 30, false), "valores menores não mudam nada");
        assert_eq!(r, Recordes { pontuacao: 500, nivel: 2, jogada: 120, terminadas: 0 });
        assert!(!r.registrar(500, 2, 120, false), "valores iguais também não");
        assert!(r.registrar(900, 2, 50, true));
        assert_eq!(r, Recordes { pontuacao: 900, nivel: 2, jogada: 120, terminadas: 1 });
        assert!(r.registrar(100, 1, 1, true), "terminar uma partida sempre conta, mesmo sem recorde");
        assert_eq!(r.terminadas, 2);
        let antes = r;
        assert!(!r.registrar(10, 1, 1, false));
        assert_eq!(r, antes, "terminou == false nunca soma");
    }

    #[test]
    fn alcancar_so_mexe_em_pontuacao_e_nivel() {
        let mut r = Recordes { pontuacao: 500, nivel: 2, jogada: 120, terminadas: 3 };
        assert!(r.alcancar(700, 3));
        assert_eq!(r, Recordes { pontuacao: 700, nivel: 3, jogada: 120, terminadas: 3 });
        assert!(!r.alcancar(699, 3));
        assert!(!r.alcancar(10, 1));
        assert!(r.alcancar(700, 4), "só o nível subiu: mudou");
        assert_eq!(r.nivel, 4);
    }
}
