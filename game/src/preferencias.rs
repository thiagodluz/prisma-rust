//! Preferências de som, vibração e guia de respiração: tudo "puro" (só números e textos), sem tocar nada.
//!
//! O arquivo `som.json` guarda, com os mesmos nomes do jogo em JS (`soundOn`, `audio`, `zen`),
//! os volumes, o que está ligado e as escolhas do Zen. A leitura é por campo, como as funções
//! `normalizeAudioSettings` e `normalizeZenSettings` do JS: o que está certo vale, o que está
//! errado ou falta vira o padrão, sem derrubar o resto.

use serde_json::{json, Map, Value};

/// Os três canais de volume, nas barras da folha de Som.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Canal {
    Efeitos,
    Musica,
    Ambiente,
}

/// O volume de cada canal, de 0 a 100.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Volumes {
    pub efeitos: u8,
    pub musica: u8,
    pub ambiente: u8,
}

impl Default for Volumes {
    /// Os padrões do JS: 80 / 70 / 65.
    fn default() -> Volumes {
        Volumes { efeitos: 80, musica: 70, ambiente: 65 }
    }
}

impl Volumes {
    pub fn de(&self, canal: Canal) -> u8 {
        match canal {
            Canal::Efeitos => self.efeitos,
            Canal::Musica => self.musica,
            Canal::Ambiente => self.ambiente,
        }
    }

    /// Troca o volume de um canal; acima de 100 vale 100.
    pub fn definir(&mut self, canal: Canal, valor: u8) {
        let valor = valor.min(100);
        match canal {
            Canal::Efeitos => self.efeitos = valor,
            Canal::Musica => self.musica = valor,
            Canal::Ambiente => self.ambiente = valor,
        }
    }
}

/// A trilha de música do Zen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Musica {
    MagicPuzzle,
    CozyPuzzle,
    SpaceCity,
}

impl Musica {
    pub const TODAS: [Musica; 3] = [Musica::MagicPuzzle, Musica::CozyPuzzle, Musica::SpaceCity];

    /// A seguinte da lista; depois da última, volta à primeira.
    pub fn proximo(self) -> Musica {
        let posicao = Musica::TODAS.iter().position(|&m| m == self).unwrap_or(0);
        Musica::TODAS[(posicao + 1) % Musica::TODAS.len()]
    }

    /// O nome na tela.
    pub fn texto(self) -> &'static str {
        match self {
            Musica::MagicPuzzle => "Magic Puzzle",
            Musica::CozyPuzzle => "Cozy Puzzle",
            Musica::SpaceCity => "Space City",
        }
    }

    /// O nome no arquivo e no jogo em JS.
    pub fn id_js(self) -> &'static str {
        match self {
            Musica::MagicPuzzle => "magicPuzzle",
            Musica::CozyPuzzle => "cozyPuzzle",
            Musica::SpaceCity => "spaceCity",
        }
    }

    fn pelo_id_js(id: &str) -> Option<Musica> {
        Musica::TODAS.into_iter().find(|m| m.id_js() == id)
    }
}

/// O som de ambiente do Zen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ambiente {
    Riacho,
    Chuva,
    Floresta,
    FlorestaAposChuva,
    Trovoada,
    RuidoSuave,
}

impl Ambiente {
    pub const TODOS: [Ambiente; 6] =
        [Ambiente::Riacho, Ambiente::Chuva, Ambiente::Floresta, Ambiente::FlorestaAposChuva, Ambiente::Trovoada, Ambiente::RuidoSuave];

    pub fn proximo(self) -> Ambiente {
        let posicao = Ambiente::TODOS.iter().position(|&a| a == self).unwrap_or(0);
        Ambiente::TODOS[(posicao + 1) % Ambiente::TODOS.len()]
    }

    pub fn texto(self) -> &'static str {
        match self {
            Ambiente::Riacho => "Riacho",
            Ambiente::Chuva => "Chuva",
            Ambiente::Floresta => "Floresta com cigarras",
            Ambiente::FlorestaAposChuva => "Floresta após chuva",
            Ambiente::Trovoada => "Chuva com trovoada",
            Ambiente::RuidoSuave => "Ruído suave",
        }
    }

    pub fn id_js(self) -> &'static str {
        match self {
            Ambiente::Riacho => "stream",
            Ambiente::Chuva => "rain",
            Ambiente::Floresta => "forest",
            Ambiente::FlorestaAposChuva => "rainforest",
            Ambiente::Trovoada => "thunder",
            Ambiente::RuidoSuave => "softNoise",
        }
    }

    fn pelo_id_js(id: &str) -> Option<Ambiente> {
        Ambiente::TODOS.into_iter().find(|a| a.id_js() == id)
    }
}

/// O ritmo do guia de respiração do Zen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Respiracao {
    Desligada,
    Equilibrada,
    Lenta,
}

impl Respiracao {
    pub const TODAS: [Respiracao; 3] = [Respiracao::Desligada, Respiracao::Equilibrada, Respiracao::Lenta];

    pub fn proximo(self) -> Respiracao {
        let posicao = Respiracao::TODAS.iter().position(|&r| r == self).unwrap_or(0);
        Respiracao::TODAS[(posicao + 1) % Respiracao::TODAS.len()]
    }

    pub fn texto(self) -> &'static str {
        match self {
            Respiracao::Desligada => "Desligada",
            Respiracao::Equilibrada => "4 s inspirar · 4 s expirar",
            Respiracao::Lenta => "4 s inspirar · 6 s expirar",
        }
    }

    pub fn id_js(self) -> &'static str {
        match self {
            Respiracao::Desligada => "off",
            Respiracao::Equilibrada => "balanced",
            Respiracao::Lenta => "slow",
        }
    }

    fn pelo_id_js(id: &str) -> Option<Respiracao> {
        Respiracao::TODAS.into_iter().find(|r| r.id_js() == id)
    }

    /// Segundos de inspirar e de expirar; `None` com o guia desligado.
    fn tempos_s(self) -> Option<(u32, u32)> {
        match self {
            Respiracao::Desligada => None,
            Respiracao::Equilibrada => Some((4, 4)),
            Respiracao::Lenta => Some((4, 6)),
        }
    }
}

/// A intensidade dos efeitos visuais do Zen (as partículas das limpezas), a "Efeitos" do JS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Intensidade {
    Suave,
    #[default]
    Padrao,
    Vibrante,
}

impl Intensidade {
    pub const TODAS: [Intensidade; 3] = [Intensidade::Suave, Intensidade::Padrao, Intensidade::Vibrante];

    pub fn proximo(self) -> Intensidade {
        let posicao = Intensidade::TODAS.iter().position(|&i| i == self).unwrap_or(0);
        Intensidade::TODAS[(posicao + 1) % Intensidade::TODAS.len()]
    }

    /// O nome que o jogador lê no botão.
    pub fn texto(self) -> &'static str {
        match self {
            Intensidade::Suave => "Suave",
            Intensidade::Padrao => "Padrão",
            Intensidade::Vibrante => "Vibrante",
        }
    }

    /// O valor gravado em `som.json` (`zen.effects`), o mesmo do jogo em JS.
    pub fn id_js(self) -> &'static str {
        match self {
            Intensidade::Suave => "soft",
            Intensidade::Padrao => "normal",
            Intensidade::Vibrante => "vivid",
        }
    }

    /// O nome que entra nas linhas `PRISMA_M4 intensidade valor=...` do registro.
    pub fn nome_do_registro(self) -> &'static str {
        match self {
            Intensidade::Suave => "suave",
            Intensidade::Padrao => "padrao",
            Intensidade::Vibrante => "vibrante",
        }
    }

    fn pelo_id_js(id: &str) -> Option<Intensidade> {
        Intensidade::TODAS.into_iter().find(|i| i.id_js() == id)
    }
}

/// Tudo o que o jogador escolhe na folha de Som.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preferencias {
    /// O interruptor "Sons do jogo" (os efeitos).
    pub sons_do_jogo: bool,
    pub volumes: Volumes,
    pub musica_ligada: bool,
    pub ambiente_ligado: bool,
    pub musica: Musica,
    pub ambiente: Ambiente,
    pub respiracao: Respiracao,
    /// O interruptor "Vibração" (padrão: desligada, como no JS).
    pub vibracao: bool,
    /// A intensidade das partículas do Zen (padrão: Padrão, como no JS).
    pub intensidade: Intensidade,
}

impl Default for Preferencias {
    fn default() -> Preferencias {
        Preferencias {
            sons_do_jogo: true,
            volumes: Volumes::default(),
            musica_ligada: false,
            ambiente_ligado: false,
            musica: Musica::CozyPuzzle,
            ambiente: Ambiente::Chuva,
            respiracao: Respiracao::Desligada,
            vibracao: false,
            intensidade: Intensidade::Padrao,
        }
    }
}

/// O texto JSON do arquivo `som.json`, com os nomes do jogo em JS.
pub fn preferencias_para_json(preferencias: &Preferencias) -> String {
    json!({
        "soundOn": preferencias.sons_do_jogo,
        "vibration": preferencias.vibracao,
        "audio": {
            "effects": preferencias.volumes.efeitos,
            "music": preferencias.volumes.musica,
            "ambience": preferencias.volumes.ambiente,
        },
        "zen": {
            "music": preferencias.musica_ligada,
            "ambience": preferencias.ambiente_ligado,
            "musicTrack": preferencias.musica.id_js(),
            "ambienceSound": preferencias.ambiente.id_js(),
            "breath": preferencias.respiracao.id_js(),
            "effects": preferencias.intensidade.id_js(),
        },
    })
    .to_string()
}

/// Um volume do arquivo: número finito arredondado e preso em 0..100; qualquer outra coisa
/// (texto, `null`, booleano, campo que falta) vale `padrao`.
fn volume(objeto: Option<&Map<String, Value>>, nome: &str, padrao: u8) -> u8 {
    match objeto.and_then(|o| o.get(nome)).and_then(Value::as_f64) {
        Some(nivel) if nivel.is_finite() => nivel.round().clamp(0.0, 100.0) as u8,
        _ => padrao,
    }
}

/// Lê o texto de `som.json`. Só recusa o que não é um objeto JSON (vazio, cortado, lista, número...);
/// dentro de um objeto, cada campo ruim ou ausente vira o padrão. Um BOM (o caractere invisível
/// que alguns editores de texto põem no começo do arquivo) é aceito.
pub fn json_para_preferencias(texto: &str) -> Result<Preferencias, String> {
    let texto = texto.strip_prefix('\u{feff}').unwrap_or(texto);
    let raiz = match serde_json::from_str::<Value>(texto) {
        Ok(Value::Object(objeto)) => objeto,
        Ok(_) => return Err("preferências de som: o JSON não é um objeto".to_string()),
        Err(erro) => {
            return Err(format!("preferências de som: o texto não é um JSON válido (linha {}, coluna {})", erro.line(), erro.column()));
        }
    };
    let padrao = Preferencias::default();
    let audio = raiz.get("audio").and_then(Value::as_object);
    let zen = raiz.get("zen").and_then(Value::as_object);
    // Booleano só vale se for `true`/`false` de verdade; senão, o padrão.
    let booleano = |objeto: Option<&Map<String, Value>>, nome: &str, padrao: bool| {
        objeto.and_then(|o| o.get(nome)).and_then(Value::as_bool).unwrap_or(padrao)
    };
    let texto_do_zen = |nome: &str| zen.and_then(|o| o.get(nome)).and_then(Value::as_str);
    Ok(Preferencias {
        sons_do_jogo: booleano(Some(&raiz), "soundOn", padrao.sons_do_jogo),
        volumes: Volumes {
            efeitos: volume(audio, "effects", padrao.volumes.efeitos),
            musica: volume(audio, "music", padrao.volumes.musica),
            ambiente: volume(audio, "ambience", padrao.volumes.ambiente),
        },
        // No JS, só `true` liga; qualquer outra coisa (ou nada) deixa desligado.
        musica_ligada: booleano(zen, "music", false),
        ambiente_ligado: booleano(zen, "ambience", false),
        musica: texto_do_zen("musicTrack").and_then(Musica::pelo_id_js).unwrap_or(padrao.musica),
        ambiente: texto_do_zen("ambienceSound").and_then(Ambiente::pelo_id_js).unwrap_or(padrao.ambiente),
        respiracao: texto_do_zen("breath").and_then(Respiracao::pelo_id_js).unwrap_or(padrao.respiracao),
        // Como a música e o ambiente: só `true` de verdade liga; o arquivo do M3 não tem o campo.
        vibracao: booleano(Some(&raiz), "vibration", padrao.vibracao),
        // Texto fora da lista, outro tipo ou campo ausente (o arquivo do M3): Padrão.
        intensidade: texto_do_zen("effects").and_then(Intensidade::pelo_id_js).unwrap_or(padrao.intensidade),
    })
}

// ----- Guia de respiração -----

/// Em que parte do ciclo o guia está.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fase {
    Inspire,
    Expire,
}

impl Fase {
    /// A palavra que o guia mostra.
    pub fn texto(self) -> &'static str {
        match self {
            Fase::Inspire => "Inspire",
            Fase::Expire => "Expire",
        }
    }
}

/// "1 segundo", "2 segundos": o contador do guia.
pub fn segundos_restantes(segundos: u32) -> String {
    if segundos == 1 {
        "1 segundo".to_string()
    } else {
        format!("{segundos} segundos")
    }
}

/// O que o guia mostra num instante: a fase, os segundos que faltam para ela acabar (de 1 em
/// diante, como o contador do JS) e quanto da fase já passou (0 a 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EstadoRespiracao {
    pub fase: Fase,
    pub restante_s: u32,
    pub progresso: f32,
}

/// Porte do `updateBreathStep` do JS. `decorrido_ms` é o tempo desde que o guia começou; NaN,
/// infinito ou negativo valem 0. Com o guia desligado, não há estado.
pub fn estado_da_respiracao(respiracao: Respiracao, decorrido_ms: f32) -> Option<EstadoRespiracao> {
    let (inspirar_s, expirar_s) = respiracao.tempos_s()?;
    let decorrido = if decorrido_ms.is_finite() && decorrido_ms > 0.0 { f64::from(decorrido_ms) } else { 0.0 };
    let inspirar_ms = f64::from(inspirar_s) * 1000.0;
    let ciclo_ms = f64::from(inspirar_s + expirar_s) * 1000.0;
    let posicao = decorrido % ciclo_ms;
    let (fase, duracao_ms, na_fase_ms) =
        if posicao < inspirar_ms { (Fase::Inspire, inspirar_ms, posicao) } else { (Fase::Expire, f64::from(expirar_s) * 1000.0, posicao - inspirar_ms) };
    Some(EstadoRespiracao {
        fase,
        restante_s: (((duracao_ms - na_fase_ms) / 1000.0).ceil() as u32).max(1),
        progresso: (na_fase_ms / duracao_ms).clamp(0.0, 1.0) as f32,
    })
}

/// A bolinha do guia: `(escala, opacidade)`. Inspirando vai de 0,72 / 0,64 a 1,15 / 1,0 e
/// expirando faz o caminho de volta, sempre com suavização nas duas pontas (como o
/// `ease-in-out` do CSS do JS; aqui usa a curva "smoothstep", quase igual).
pub fn orbe(estado: &EstadoRespiracao) -> (f32, f32) {
    const ESCALA: (f32, f32) = (0.72, 1.15);
    const OPACIDADE: (f32, f32) = (0.64, 1.0);
    let progresso = if estado.progresso.is_nan() { 0.0 } else { estado.progresso.clamp(0.0, 1.0) };
    let suave = progresso * progresso * (3.0 - 2.0 * progresso);
    // `cheio` vai de 0 (bolinha pequena) a 1 (grande).
    let cheio = match estado.fase {
        Fase::Inspire => suave,
        Fase::Expire => 1.0 - suave,
    };
    (ESCALA.0 + (ESCALA.1 - ESCALA.0) * cheio, OPACIDADE.0 + (OPACIDADE.1 - OPACIDADE.0) * cheio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ler(texto: &str) -> Preferencias {
        json_para_preferencias(texto).unwrap_or_else(|motivo| panic!("{texto:?} devia ser aceito: {motivo}"))
    }

    fn volumes(texto: &str) -> (u8, u8, u8) {
        let lidas = ler(texto);
        (lidas.volumes.efeitos, lidas.volumes.musica, lidas.volumes.ambiente)
    }

    #[test]
    fn padroes_sao_80_70_65_e_tudo_desligado() {
        let padrao = Preferencias::default();
        assert!(padrao.sons_do_jogo);
        assert_eq!(padrao.volumes, Volumes { efeitos: 80, musica: 70, ambiente: 65 });
        assert!(!padrao.musica_ligada);
        assert!(!padrao.ambiente_ligado);
        assert_eq!(padrao.musica, Musica::CozyPuzzle);
        assert_eq!(padrao.ambiente, Ambiente::Chuva);
        assert_eq!(padrao.respiracao, Respiracao::Desligada);
        assert_eq!(padrao.intensidade, Intensidade::Padrao);
        // Um objeto vazio vale o mesmo que o padrão.
        assert_eq!(ler("{}"), padrao);
    }

    #[test]
    fn numeros_estranhos_entram_na_faixa_ou_viram_padrao() {
        assert_eq!(volumes(r#"{"audio":{"effects":150,"music":-5,"ambience":72.6}}"#), (100, 0, 73));
        // Texto, null e booleano não são número: valem o padrão (80, 70, 65).
        assert_eq!(volumes(r#"{"audio":{"effects":"70","music":null,"ambience":true}}"#), (80, 70, 65));
        // 0 é um volume de verdade (silêncio), não "falta".
        assert_eq!(volumes(r#"{"audio":{"effects":0,"music":0,"ambience":0}}"#), (0, 0, 0));
        // Arredondamento e limites.
        assert_eq!(volumes(r#"{"audio":{"effects":-0.4,"music":99.5,"ambience":100}}"#), (0, 100, 100));
        assert_eq!(volumes(r#"{"audio":{"effects":1e30,"music":-1e30,"ambience":12345678901234567890}}"#), (100, 0, 100));
        assert_eq!(volumes(r#"{"audio":{"effects":4.4,"music":4.5,"ambience":5}}"#), (4, 5, 5));
        // Cada canal vale por si: um ruim não leva os outros.
        assert_eq!(volumes(r#"{"audio":{"effects":30,"music":"x"}}"#), (30, 70, 65));
    }

    #[test]
    fn texto_fora_da_lista_vira_padrao() {
        let lidas = ler(r#"{"zen":{"musicTrack":"x","ambienceSound":"y","breath":"foo"}}"#);
        assert_eq!((lidas.musica, lidas.ambiente, lidas.respiracao), (Musica::CozyPuzzle, Ambiente::Chuva, Respiracao::Desligada));
        // Maiúscula diferente não vale: a lista é exata, como no JS.
        assert_eq!(ler(r#"{"zen":{"musicTrack":"CozyPuzzle","breath":"Slow"}}"#).respiracao, Respiracao::Desligada);
        assert_eq!(ler(r#"{"zen":{"musicTrack":"spaceCity"}}"#).musica, Musica::SpaceCity);
        // Tipo errado (número, null, lista) também vira padrão.
        let lidas = ler(r#"{"zen":{"musicTrack":3,"ambienceSound":null,"breath":["slow"]}}"#);
        assert_eq!((lidas.musica, lidas.ambiente, lidas.respiracao), (Musica::CozyPuzzle, Ambiente::Chuva, Respiracao::Desligada));
        // Booleano só vale se for `true` de verdade; `soundOn` ausente ou estranho vale `true`.
        let lidas = ler(r#"{"zen":{"music":"true","ambience":1},"soundOn":0}"#);
        assert!(!lidas.musica_ligada && !lidas.ambiente_ligado);
        assert!(lidas.sons_do_jogo, "soundOn:0 não é booleano: vale o padrão (true)");
        let lidas = ler(r#"{"zen":{"music":true,"ambience":true},"soundOn":false}"#);
        assert!(lidas.musica_ligada && lidas.ambiente_ligado);
        assert!(!lidas.sons_do_jogo);
        for estranho in ["null", "\"false\"", "[]", "{}"] {
            assert!(ler(&format!(r#"{{"soundOn":{estranho}}}"#)).sons_do_jogo, "soundOn:{estranho}");
        }
    }

    #[test]
    fn objeto_ou_campo_que_falta_vira_padrao() {
        let padrao = Preferencias::default();
        for texto in [
            r#"{"soundOn":true}"#,
            r#"{"audio":null,"zen":null}"#,
            r#"{"audio":5,"zen":"x"}"#,
            r#"{"audio":[],"zen":[1,2]}"#,
            r#"{"outra":"coisa","audio":{"extra":1},"zen":{"extra":true}}"#,
        ] {
            assert_eq!(ler(texto), padrao, "{texto}");
        }
        // Falta só um campo: os outros continuam valendo.
        let lidas = ler(r#"{"audio":{"music":10},"zen":{"ambienceSound":"thunder"}}"#);
        assert_eq!((lidas.volumes.efeitos, lidas.volumes.musica, lidas.volumes.ambiente), (80, 10, 65));
        assert_eq!(lidas.ambiente, Ambiente::Trovoada);
        assert_eq!(lidas.musica, Musica::CozyPuzzle);
    }

    #[test]
    fn ida_e_volta_do_json_mantem_tudo() {
        let mut quantas = 0;
        for sons_do_jogo in [true, false] {
            for (efeitos, musica_v, ambiente_v) in [(80, 70, 65), (0, 0, 0), (100, 100, 100), (5, 55, 95)] {
                for musica_ligada in [true, false] {
                    for ambiente_ligado in [true, false] {
                        for musica in Musica::TODAS {
                            for ambiente in Ambiente::TODOS {
                                for respiracao in Respiracao::TODAS {
                                    for vibracao in [true, false] {
                                        for intensidade in Intensidade::TODAS {
                                            let original = Preferencias {
                                                sons_do_jogo,
                                                volumes: Volumes { efeitos, musica: musica_v, ambiente: ambiente_v },
                                                musica_ligada,
                                                ambiente_ligado,
                                                musica,
                                                ambiente,
                                                respiracao,
                                                vibracao,
                                                intensidade,
                                            };
                                            assert_eq!(ler(&preferencias_para_json(&original)), original);
                                            quantas += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(quantas, 2 * 4 * 2 * 2 * 3 * 6 * 3 * 2 * 3);
    }

    #[test]
    fn o_json_gravado_usa_os_nomes_do_js() {
        let preferencias = Preferencias {
            sons_do_jogo: false,
            volumes: Volumes { efeitos: 10, musica: 20, ambiente: 30 },
            musica_ligada: true,
            ambiente_ligado: false,
            musica: Musica::SpaceCity,
            ambiente: Ambiente::FlorestaAposChuva,
            respiracao: Respiracao::Lenta,
            vibracao: true,
            intensidade: Intensidade::Vibrante,
        };
        let gravado: serde_json::Value = serde_json::from_str(&preferencias_para_json(&preferencias)).unwrap();
        assert_eq!(
            gravado,
            json!({
                "soundOn": false,
                "vibration": true,
                "audio": {"effects": 10, "music": 20, "ambience": 30},
                "zen": {
                    "music": true,
                    "ambience": false,
                    "musicTrack": "spaceCity",
                    "ambienceSound": "rainforest",
                    "breath": "slow",
                    "effects": "vivid"
                }
            })
        );
        // O arquivo do padrão é exatamente o do plano.
        let padrao: serde_json::Value = serde_json::from_str(&preferencias_para_json(&Preferencias::default())).unwrap();
        assert_eq!(
            padrao,
            json!({
                "soundOn": true,
                "vibration": false,
                "audio": {"effects": 80, "music": 70, "ambience": 65},
                "zen": {
                    "music": false,
                    "ambience": false,
                    "musicTrack": "cozyPuzzle",
                    "ambienceSound": "rain",
                    "breath": "off",
                    "effects": "normal"
                }
            })
        );
    }

    #[test]
    fn vibracao_comeca_desligada() {
        assert!(!Preferencias::default().vibracao);
    }

    #[test]
    fn vibracao_vai_e_volta_no_json() {
        for ligada in [true, false] {
            let preferencias = Preferencias { vibracao: ligada, ..Preferencias::default() };
            let texto = preferencias_para_json(&preferencias);
            let gravado: serde_json::Value = serde_json::from_str(&texto).unwrap();
            assert_eq!(gravado["vibration"], json!(ligada), "o nome no arquivo é o do JS");
            assert_eq!(ler(&texto).vibracao, ligada);
        }
    }

    #[test]
    fn json_do_m3_sem_vibration_carrega_desligada() {
        // O arquivo que a versão do M3 gravava não tem o campo novo.
        let lidas = ler(r#"{"soundOn":false,"audio":{"effects":10,"music":20,"ambience":30},"zen":{"music":true,"breath":"slow"}}"#);
        assert!(!lidas.vibracao);
        assert!(!lidas.sons_do_jogo, "o resto continua valendo");
        assert_eq!(lidas.volumes.efeitos, 10);
    }

    #[test]
    fn vibration_so_vale_com_booleano_de_verdade() {
        for estranho in [r#""on""#, "1", "null", r#""true""#, "[]", "{}", "0.5"] {
            let lidas = ler(&format!(r#"{{"vibration":{estranho}}}"#));
            assert!(!lidas.vibracao, "{estranho} devia valer desligada");
        }
        assert!(ler(r#"{"vibration":true}"#).vibracao);
        assert!(!ler(r#"{"vibration":false}"#).vibracao);
    }

    #[test]
    fn json_ilegivel_e_recusado() {
        for texto in ["", "   ", "{", "[]", "null", "42", "\"texto\"", "true", "lixo", "\u{feff}", "{\"audio\":", "{} {}"] {
            let resposta = json_para_preferencias(texto);
            assert!(resposta.as_ref().is_err_and(|motivo| !motivo.trim().is_empty()), "{texto:?} devia ser recusado com um motivo, veio {resposta:?}");
        }
    }

    #[test]
    fn bom_no_comeco_e_aceito() {
        let com_bom = format!("\u{feff}{}", r#"{"audio":{"effects":12},"zen":{"breath":"balanced"}}"#);
        let lidas = ler(&com_bom);
        assert_eq!(lidas.volumes.efeitos, 12);
        assert_eq!(lidas.respiracao, Respiracao::Equilibrada);
        // Só o BOM do começo: um no meio do texto continua sendo erro de JSON.
        assert!(json_para_preferencias("{\u{feff}}").is_err());
    }

    #[test]
    fn proximo_da_a_volta() {
        let mut musica = Musica::MagicPuzzle;
        let mut vistas = Vec::new();
        for _ in 0..Musica::TODAS.len() {
            vistas.push(musica);
            musica = musica.proximo();
        }
        assert_eq!(vistas, Musica::TODAS, "percorre todas, na ordem");
        assert_eq!(musica, Musica::MagicPuzzle, "volta ao primeiro depois do último");

        let mut ambiente = Ambiente::Riacho;
        let mut vistos = Vec::new();
        for _ in 0..Ambiente::TODOS.len() {
            vistos.push(ambiente);
            ambiente = ambiente.proximo();
        }
        assert_eq!(vistos, Ambiente::TODOS);
        assert_eq!(ambiente, Ambiente::Riacho);

        let mut respiracao = Respiracao::Desligada;
        let mut vistas = Vec::new();
        for _ in 0..Respiracao::TODAS.len() {
            vistas.push(respiracao);
            respiracao = respiracao.proximo();
        }
        assert_eq!(vistas, Respiracao::TODAS);
        assert_eq!(respiracao, Respiracao::Desligada);

        let mut intensidade = Intensidade::Suave;
        let mut vistas = Vec::new();
        for _ in 0..Intensidade::TODAS.len() {
            vistas.push(intensidade);
            intensidade = intensidade.proximo();
        }
        assert_eq!(vistas, Intensidade::TODAS, "percorre as três, na ordem");
        assert_eq!(intensidade, Intensidade::Suave, "depois de Vibrante volta a Suave");
        assert_eq!(Intensidade::Padrao.proximo(), Intensidade::Vibrante);
    }

    #[test]
    fn intensidade_padrao_e_padrao() {
        assert_eq!(Intensidade::default(), Intensidade::Padrao);
        assert_eq!(Preferencias::default().intensidade, Intensidade::Padrao);
    }

    #[test]
    fn ida_e_volta_do_json_com_effects() {
        for intensidade in Intensidade::TODAS {
            let preferencias = Preferencias { intensidade, ..Preferencias::default() };
            let texto = preferencias_para_json(&preferencias);
            let gravado: serde_json::Value = serde_json::from_str(&texto).unwrap();
            assert_eq!(gravado["zen"]["effects"], json!(intensidade.id_js()), "o campo mora em zen, com o nome do JS");
            assert_eq!(ler(&texto).intensidade, intensidade);
        }
    }

    #[test]
    fn effects_desconhecido_vira_padrao() {
        // Texto fora da lista (e maiúscula diferente), número, null e campo ausente: tudo vira Padrão.
        for estranho in [r#""neon""#, r#""Vivid""#, r#""""#, "1", "null", "true", "[]", r#"{"a":1}"#] {
            let lidas = ler(&format!(r#"{{"zen":{{"effects":{estranho}}}}}"#));
            assert_eq!(lidas.intensidade, Intensidade::Padrao, "{estranho}");
        }
        assert_eq!(ler(r#"{"zen":{}}"#).intensidade, Intensidade::Padrao, "ausente");
        assert_eq!(ler(r#"{"zen":"x"}"#).intensidade, Intensidade::Padrao, "zen não é objeto");
        // Os três valores de verdade valem.
        assert_eq!(ler(r#"{"zen":{"effects":"soft"}}"#).intensidade, Intensidade::Suave);
        assert_eq!(ler(r#"{"zen":{"effects":"normal"}}"#).intensidade, Intensidade::Padrao);
        assert_eq!(ler(r#"{"zen":{"effects":"vivid"}}"#).intensidade, Intensidade::Vibrante);
        // Um campo ruim não leva os outros.
        let lidas = ler(r#"{"zen":{"effects":"neon","breath":"slow"}}"#);
        assert_eq!((lidas.intensidade, lidas.respiracao), (Intensidade::Padrao, Respiracao::Lenta));
    }

    #[test]
    fn json_do_m3_sem_effects_carrega_com_padrao() {
        // O arquivo que a versão do M3 gravava (com `vibration`, sem `effects`).
        let lidas = ler(r#"{"soundOn":false,"vibration":true,"audio":{"effects":10},"zen":{"music":true,"breath":"slow"}}"#);
        assert_eq!(lidas.intensidade, Intensidade::Padrao);
        assert!(lidas.vibracao && !lidas.sons_do_jogo, "o resto continua valendo");
        assert_eq!(lidas.volumes.efeitos, 10, "o `effects` de dentro de audio é o volume, não a intensidade");
    }

    #[test]
    fn nomes_e_rotulos_sao_os_do_js() {
        let musicas: Vec<(&str, &str)> = Musica::TODAS.iter().map(|m| (m.id_js(), m.texto())).collect();
        assert_eq!(musicas, [("magicPuzzle", "Magic Puzzle"), ("cozyPuzzle", "Cozy Puzzle"), ("spaceCity", "Space City")]);
        let ambientes: Vec<(&str, &str)> = Ambiente::TODOS.iter().map(|a| (a.id_js(), a.texto())).collect();
        assert_eq!(
            ambientes,
            [
                ("stream", "Riacho"),
                ("rain", "Chuva"),
                ("forest", "Floresta com cigarras"),
                ("rainforest", "Floresta após chuva"),
                ("thunder", "Chuva com trovoada"),
                ("softNoise", "Ruído suave"),
            ]
        );
        let respiracoes: Vec<(&str, &str)> = Respiracao::TODAS.iter().map(|r| (r.id_js(), r.texto())).collect();
        assert_eq!(
            respiracoes,
            [("off", "Desligada"), ("balanced", "4 s inspirar · 4 s expirar"), ("slow", "4 s inspirar · 6 s expirar")]
        );
        let intensidades: Vec<(&str, &str, &str)> =
            Intensidade::TODAS.iter().map(|i| (i.id_js(), i.texto(), i.nome_do_registro())).collect();
        assert_eq!(
            intensidades,
            [("soft", "Suave", "suave"), ("normal", "Padrão", "padrao"), ("vivid", "Vibrante", "vibrante")]
        );
    }

    #[test]
    fn volumes_por_canal() {
        let mut volumes = Volumes::default();
        assert_eq!((volumes.de(Canal::Efeitos), volumes.de(Canal::Musica), volumes.de(Canal::Ambiente)), (80, 70, 65));
        volumes.definir(Canal::Musica, 35);
        volumes.definir(Canal::Ambiente, 0);
        volumes.definir(Canal::Efeitos, 250);
        assert_eq!(volumes, Volumes { efeitos: 100, musica: 35, ambiente: 0 }, "acima de 100 vale 100");
    }

    // ----- Respiração -----

    fn estado(r: Respiracao, ms: f32) -> EstadoRespiracao {
        estado_da_respiracao(r, ms).unwrap_or_else(|| panic!("{r:?} em {ms} ms devia ter estado"))
    }

    fn fase_e_restante(r: Respiracao, ms: f32) -> (Fase, u32) {
        let e = estado(r, ms);
        (e.fase, e.restante_s)
    }

    #[test]
    fn os_textos_do_guia_sao_em_portugues() {
        assert_eq!(Fase::Inspire.texto(), "Inspire");
        assert_eq!(Fase::Expire.texto(), "Expire");
        assert_eq!(segundos_restantes(1), "1 segundo");
        assert_eq!(segundos_restantes(2), "2 segundos");
        assert_eq!(segundos_restantes(4), "4 segundos");
        assert_eq!(segundos_restantes(0), "0 segundos");
    }

    #[test]
    fn respiracao_equilibrada_ciclo_de_8s() {
        use Fase::{Expire, Inspire};
        let r = Respiracao::Equilibrada;
        assert_eq!(fase_e_restante(r, 0.0), (Inspire, 4));
        assert_eq!(fase_e_restante(r, 1.0), (Inspire, 4));
        assert_eq!(fase_e_restante(r, 1000.0), (Inspire, 3));
        assert_eq!(fase_e_restante(r, 3900.0), (Inspire, 1));
        assert_eq!(fase_e_restante(r, 3999.0), (Inspire, 1));
        assert_eq!(fase_e_restante(r, 4000.0), (Expire, 4));
        assert_eq!(fase_e_restante(r, 5000.0), (Expire, 3));
        assert_eq!(fase_e_restante(r, 7999.0), (Expire, 1));
        assert_eq!(fase_e_restante(r, 8000.0), (Inspire, 4), "o ciclo recomeça");
        assert_eq!(fase_e_restante(r, 8000.0 * 3.0 + 4500.0), (Expire, 4), "no 4º ciclo");
        assert!((estado(r, 2000.0).progresso - 0.5).abs() < 1e-6);
        assert!((estado(r, 6000.0).progresso - 0.5).abs() < 1e-6);
        assert_eq!(estado(r, 4000.0).progresso, 0.0);
    }

    #[test]
    fn respiracao_lenta_ciclo_de_10s() {
        use Fase::{Expire, Inspire};
        let r = Respiracao::Lenta;
        assert_eq!(fase_e_restante(r, 0.0), (Inspire, 4));
        assert_eq!(fase_e_restante(r, 3999.0), (Inspire, 1));
        assert_eq!(fase_e_restante(r, 4000.0), (Expire, 6));
        assert_eq!(fase_e_restante(r, 9999.0), (Expire, 1));
        assert_eq!(fase_e_restante(r, 10_000.0), (Inspire, 4));
        assert!((estado(r, 7000.0).progresso - 0.5).abs() < 1e-6, "meio da expiração de 6 s");
    }

    #[test]
    fn respiracao_desligada_nao_tem_estado() {
        assert_eq!(estado_da_respiracao(Respiracao::Desligada, 0.0), None);
        assert_eq!(estado_da_respiracao(Respiracao::Desligada, 5000.0), None);
        assert_eq!(estado_da_respiracao(Respiracao::Desligada, f32::NAN), None);
    }

    #[test]
    fn respiracao_tempo_invalido() {
        for r in [Respiracao::Equilibrada, Respiracao::Lenta] {
            for ms in [f32::NAN, -1.0, -1e30, 1e30, f32::MAX, f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0] {
                let e = estado(r, ms);
                assert!((0.0..=1.0).contains(&e.progresso), "{r:?} {ms}: progresso {}", e.progresso);
                assert!(e.restante_s >= 1 && e.restante_s <= 6, "{r:?} {ms}: restante {}", e.restante_s);
            }
            // Tempo negativo ou NaN vale 0: o começo do ciclo.
            for ms in [f32::NAN, -1.0, -5000.0, f32::NEG_INFINITY] {
                assert_eq!(estado(r, ms), estado(r, 0.0), "{r:?} {ms}");
            }
        }
    }

    #[test]
    fn orbe_cresce_ao_inspirar_e_encolhe_ao_expirar() {
        let em = |fase, progresso| orbe(&EstadoRespiracao { fase, restante_s: 1, progresso });
        let perto = |(a, b): (f32, f32), (c, d): (f32, f32)| (a - c).abs() < 1e-4 && (b - d).abs() < 1e-4;
        assert!(perto(em(Fase::Inspire, 0.0), (0.72, 0.64)), "{:?}", em(Fase::Inspire, 0.0));
        assert!(perto(em(Fase::Inspire, 1.0), (1.15, 1.0)), "{:?}", em(Fase::Inspire, 1.0));
        assert!(perto(em(Fase::Expire, 0.0), (1.15, 1.0)), "{:?}", em(Fase::Expire, 0.0));
        assert!(perto(em(Fase::Expire, 1.0), (0.72, 0.64)), "{:?}", em(Fase::Expire, 1.0));
        // Monotônico nas duas fases, sempre dentro das pontas.
        let mut anterior = em(Fase::Inspire, 0.0);
        let mut anterior_saida = em(Fase::Expire, 0.0);
        for passo in 1..=100 {
            let p = passo as f32 / 100.0;
            let agora = em(Fase::Inspire, p);
            assert!(agora.0 >= anterior.0 && agora.1 >= anterior.1, "inspirar devia crescer em {p}");
            let agora_saida = em(Fase::Expire, p);
            assert!(agora_saida.0 <= anterior_saida.0 && agora_saida.1 <= anterior_saida.1, "expirar devia encolher em {p}");
            assert!((0.72 - 1e-4..=1.15 + 1e-4).contains(&agora.0) && (0.64 - 1e-4..=1.0 + 1e-4).contains(&agora.1));
            anterior = agora;
            anterior_saida = agora_saida;
        }
        // Suavização: no começo anda mais devagar do que no meio.
        let comeco = em(Fase::Inspire, 0.1).0 - em(Fase::Inspire, 0.0).0;
        let meio = em(Fase::Inspire, 0.55).0 - em(Fase::Inspire, 0.45).0;
        assert!(comeco < meio, "começo {comeco}, meio {meio}");
        // Progresso fora de 0..1 (ou NaN) não passa das pontas.
        assert!(perto(em(Fase::Inspire, -3.0), (0.72, 0.64)));
        assert!(perto(em(Fase::Inspire, 9.0), (1.15, 1.0)));
        assert!(perto(em(Fase::Inspire, f32::NAN), (0.72, 0.64)));
    }
}
