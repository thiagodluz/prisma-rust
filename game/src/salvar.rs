//! Ler e gravar os arquivos do jogo na pasta de dados do app: uma partida por modo, os recordes
//! de cada modo, o modo em que o jogador estava e as preferências de som.
//!
//! Regras de ouro: **nunca dá pânico** e **nunca perde um save bom**.
//! - Arquivo que não dá para usar (vazio, lixo, cortado, grande demais, de outro modo, recusado
//!   pelo motor) é copiado para `<nome>.corrupt` (o original fica onde está) e aquele item
//!   recomeça do zero. Os outros arquivos não são mexidos.
//! - Gravar é feito em duas etapas: escreve um arquivo temporário e só depois troca o nome. Se
//!   a escrita falhar, o arquivo antigo continua intacto.

use crate::formato::{json_para_partida, json_para_recordes, partida_para_json, recordes_para_json, Recordes};
use crate::preferencias::{json_para_preferencias, preferencias_para_json, Preferencias};
use prisma_engine::{Game, Mode, SavedMode};
use std::fs;
use std::path::{Path, PathBuf};

/// Arquivo maior que 1 MiB conta como estragado, sem nem ser lido inteiro.
const LIMITE_BYTES: u64 = 1024 * 1024;
const ARQUIVO_MODO_ATIVO: &str = "modo-ativo.txt";
const ARQUIVO_PREFERENCIAS: &str = "som.json";

/// De onde veio a partida que o jogo abriu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origem {
    /// Não havia arquivo (ou não havia pasta): partida nova.
    Novo,
    /// Foi lida do arquivo.
    Arquivo,
    /// O arquivo existia mas não serviu (o motivo vem aqui): partida nova.
    Estragado(String),
}

/// Quem lê e grava os arquivos. Pode não ter pasta nenhuma (testes, fotos e o caso de a pasta de
/// dados não existir): aí não lê nem grava nada e tudo funciona como "primeira vez".
#[derive(Clone, Debug)]
pub struct Salvador {
    pasta: Option<PathBuf>,
}

fn nome_da_partida(modo: Mode) -> &'static str {
    match modo {
        Mode::Zen => "zen.json",
        Mode::Classic => "classico.json",
    }
}

fn nome_dos_recordes(modo: Mode) -> &'static str {
    match modo {
        Mode::Zen => "recordes-zen.json",
        Mode::Classic => "recordes-classico.json",
    }
}

/// Grava em duas etapas: cria a pasta se faltar, escreve `<nome>.tmp` e troca o nome para `<nome>`.
/// Se alguma etapa falhar, devolve `Err` e o arquivo `<nome>` que já existia não é tocado.
fn gravar_com_seguranca(pasta: &Path, nome: &str, texto: &str) -> Result<(), String> {
    fs::create_dir_all(pasta).map_err(|erro| format!("não consegui criar a pasta {}: {erro}", pasta.display()))?;
    let arquivo = pasta.join(nome);
    let temporario = pasta.join(format!("{nome}.tmp"));
    fs::write(&temporario, texto).map_err(|erro| format!("não consegui gravar {}: {erro}", temporario.display()))?;
    fs::rename(&temporario, &arquivo).map_err(|erro| {
        // Não deixa o temporário para trás se a troca de nome falhou.
        let _ = fs::remove_file(&temporario);
        format!("não consegui trocar o nome para {}: {erro}", arquivo.display())
    })
}

impl Salvador {
    /// Um salvador que usa essa pasta (ela é criada na primeira gravação, se não existir).
    pub fn em(pasta: PathBuf) -> Salvador {
        Salvador { pasta: Some(pasta) }
    }

    /// Um salvador que não lê nem grava nada.
    pub fn nenhum() -> Salvador {
        Salvador { pasta: None }
    }

    pub fn tem_pasta(&self) -> bool {
        self.pasta.is_some()
    }

    /// Copia o arquivo ruim para `<nome>.corrupt` (por cima de uma cópia anterior, se houver) e
    /// devolve o motivo. Se a cópia falhar, o jogo segue assim mesmo: ela é só uma ajuda.
    fn marcar_estragado(&self, nome: &str, motivo: impl Into<String>) -> String {
        if let Some(pasta) = &self.pasta {
            let _ = fs::copy(pasta.join(nome), pasta.join(format!("{nome}.corrupt")));
        }
        motivo.into()
    }

    /// Lê um arquivo de texto da pasta. `Ok(None)` = não existe (ou não há pasta); `Err` = existe
    /// mas não serve (grande demais, não é texto, ou não deu para ler), já com a cópia `.corrupt`.
    fn ler_texto(&self, nome: &str) -> Result<Option<String>, String> {
        let Some(pasta) = &self.pasta else { return Ok(None) };
        let arquivo = pasta.join(nome);
        // Sem arquivo, ou pasta impossível (por exemplo, "dentro" de um arquivo): não há o que ler.
        let Ok(dados) = fs::metadata(&arquivo) else { return Ok(None) };
        if dados.len() > LIMITE_BYTES {
            return Err(self.marcar_estragado(nome, "o arquivo tem mais de 1 MiB"));
        }
        match fs::read(&arquivo) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| self.marcar_estragado(nome, "o arquivo não é um texto válido")),
            Err(erro) => Err(self.marcar_estragado(nome, format!("não consegui ler o arquivo: {erro}"))),
        }
    }

    /// Lê o texto, confere o formato e o modo e pede ao motor para restaurar (tudo ou nada).
    fn restaurar(&self, nome: &str, modo: Mode, texto: &str, jogo: &mut Game) -> Result<(), String> {
        let salvo = json_para_partida(texto).map_err(|motivo| self.marcar_estragado(nome, motivo))?;
        // `endless` é o nome antigo do Clássico.
        let do_modo_certo = match modo {
            Mode::Zen => salvo.mode == SavedMode::Zen,
            Mode::Classic => matches!(salvo.mode, SavedMode::Classic | SavedMode::Endless),
        };
        if !do_modo_certo {
            return Err(self.marcar_estragado(nome, "o arquivo é de outro modo"));
        }
        if !jogo.restore(&salvo) {
            return Err(self.marcar_estragado(nome, "o motor recusou a partida gravada"));
        }
        Ok(())
    }

    /// Abre a partida do `modo` em `jogo`: a do arquivo, se servir; senão uma partida nova.
    pub fn carregar_partida(&self, modo: Mode, jogo: &mut Game) -> Origem {
        let nome = nome_da_partida(modo);
        let motivo = match self.ler_texto(nome) {
            Ok(None) => {
                jogo.new_game(modo);
                return Origem::Novo;
            }
            Ok(Some(texto)) => match self.restaurar(nome, modo, &texto, jogo) {
                Ok(()) => return Origem::Arquivo,
                Err(motivo) => motivo,
            },
            Err(motivo) => motivo,
        };
        // `restore` é tudo ou nada: se falhou, o jogo não mudou. Agora vira partida nova.
        jogo.new_game(modo);
        Origem::Estragado(motivo)
    }

    /// Grava a partida no arquivo do modo do próprio jogo.
    pub fn gravar_partida(&self, jogo: &Game) -> Result<(), String> {
        let Some(pasta) = &self.pasta else { return Ok(()) };
        gravar_com_seguranca(pasta, nome_da_partida(jogo.mode), &partida_para_json(&jogo.to_saved()))
    }

    /// Os recordes do modo. Sem arquivo ou com arquivo estragado: zerados.
    pub fn carregar_recordes(&self, modo: Mode) -> Recordes {
        let nome = nome_dos_recordes(modo);
        match self.ler_texto(nome) {
            Ok(Some(texto)) => json_para_recordes(&texto).unwrap_or_else(|motivo| {
                self.marcar_estragado(nome, motivo);
                Recordes::zerados()
            }),
            Ok(None) | Err(_) => Recordes::zerados(),
        }
    }

    pub fn gravar_recordes(&self, modo: Mode, recordes: &Recordes) -> Result<(), String> {
        let Some(pasta) = &self.pasta else { return Ok(()) };
        gravar_com_seguranca(pasta, nome_dos_recordes(modo), &recordes_para_json(recordes))
    }

    /// O modo em que o jogador estava. Arquivo ausente ou com qualquer coisa que não seja
    /// `zen` ou `classic` (sem contar espaços nas pontas): Zen.
    pub fn carregar_modo_ativo(&self) -> Mode {
        match self.ler_texto(ARQUIVO_MODO_ATIVO) {
            Ok(Some(texto)) if texto.trim() == "classic" => Mode::Classic,
            _ => Mode::Zen,
        }
    }

    pub fn gravar_modo_ativo(&self, modo: Mode) -> Result<(), String> {
        let Some(pasta) = &self.pasta else { return Ok(()) };
        let nome = match modo {
            Mode::Zen => "zen",
            Mode::Classic => "classic",
        };
        gravar_com_seguranca(pasta, ARQUIVO_MODO_ATIVO, nome)
    }

    /// As preferências de som. Sem arquivo (ou sem pasta): os padrões e `Origem::Novo`. Arquivo
    /// que não serve (vazio, lixo, cortado, grande demais): os padrões, uma cópia `som.json.corrupt`
    /// e `Origem::Estragado`. Campos ruins dentro de um objeto não estragam o arquivo: viram padrão.
    pub fn carregar_preferencias(&self) -> (Preferencias, Origem) {
        let motivo = match self.ler_texto(ARQUIVO_PREFERENCIAS) {
            Ok(None) => return (Preferencias::default(), Origem::Novo),
            Ok(Some(texto)) => match json_para_preferencias(&texto) {
                Ok(preferencias) => return (preferencias, Origem::Arquivo),
                Err(motivo) => self.marcar_estragado(ARQUIVO_PREFERENCIAS, motivo),
            },
            Err(motivo) => motivo,
        };
        (Preferencias::default(), Origem::Estragado(motivo))
    }

    pub fn gravar_preferencias(&self, preferencias: &Preferencias) -> Result<(), String> {
        let Some(pasta) = &self.pasta else { return Ok(()) };
        gravar_com_seguranca(pasta, ARQUIVO_PREFERENCIAS, &preferencias_para_json(preferencias))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formato::{partida_para_json, recordes_para_json};
    use crate::preferencias::{Ambiente, Musica, Respiracao, Volumes};
    use prisma_engine::lcg_random;
    use serde_json::Value;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Uma pasta de teste só deste teste (nome único: processo + contador). Ela **não** é criada:
    /// quem precisa dela faz `create_dir_all`, e o `Drop` apaga tudo no fim.
    struct Pasta(PathBuf);

    fn pasta_de_teste() -> Pasta {
        static CONTADOR: AtomicUsize = AtomicUsize::new(0);
        let numero = CONTADOR.fetch_add(1, Ordering::SeqCst);
        let caminho = std::env::temp_dir().join(format!("prisma-salvar-{}-{numero}", std::process::id()));
        let _ = fs::remove_dir_all(&caminho);
        Pasta(caminho)
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Um jogo com semente fixa e algumas jogadas já feitas.
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

    /// Edita um save em texto: lê o JSON, deixa `f` mexer e escreve de volta.
    fn editar(texto: &str, f: impl FnOnce(&mut Value)) -> String {
        let mut valor: Value = serde_json::from_str(texto).unwrap();
        f(&mut valor);
        valor.to_string()
    }

    fn mesma_partida(a: &Game, b: &Game) {
        assert_eq!(a.board, b.board);
        assert_eq!(a.score, b.score);
        assert_eq!(a.level, b.level);
        assert_eq!(a.level_start_score, b.level_start_score);
        assert_eq!(a.ended, b.ended);
        assert_eq!(a.mode, b.mode);
    }

    fn partida_nova_do_modo(jogo: &Game, modo: Mode) {
        assert_eq!(jogo.mode, modo);
        assert_eq!(jogo.score, 0);
        assert!(!jogo.ended);
    }

    #[test]
    fn partida_gravada_volta_igual() {
        for (modo, nome) in [(Mode::Zen, "zen.json"), (Mode::Classic, "classico.json")] {
            let pasta = pasta_de_teste();
            let salvador = Salvador::em(pasta.0.clone());
            let jogo = jogo_de_teste(11, modo, 3);
            assert!(jogo.score > 0, "o jogo de teste devia ter pontuado");
            salvador.gravar_partida(&jogo).unwrap();
            assert!(pasta.0.join(nome).is_file(), "devia existir {nome}");

            let mut outro = Game::with_random(lcg_random(99));
            assert_eq!(salvador.carregar_partida(modo, &mut outro), Origem::Arquivo);
            mesma_partida(&outro, &jogo);
        }
    }

    #[test]
    fn sem_arquivo_comeca_jogo_novo() {
        let pasta = pasta_de_teste();
        for modo in [Mode::Zen, Mode::Classic] {
            // Pasta que nem existe, e depois pasta que existe mas está vazia.
            let mut jogo = jogo_de_teste(4, Mode::Classic, 3);
            assert_eq!(Salvador::em(pasta.0.clone()).carregar_partida(modo, &mut jogo), Origem::Novo);
            partida_nova_do_modo(&jogo, modo);
            fs::create_dir_all(&pasta.0).unwrap();
        }
    }

    #[test]
    fn nenhum_nao_le_nem_grava_nada() {
        let salvador = Salvador::nenhum();
        assert!(!salvador.tem_pasta());
        let jogo = jogo_de_teste(2, Mode::Zen, 2);
        assert_eq!(salvador.gravar_partida(&jogo), Ok(()));
        assert_eq!(salvador.gravar_recordes(Mode::Zen, &Recordes { pontuacao: 5, ..Recordes::zerados() }), Ok(()));
        assert_eq!(salvador.gravar_modo_ativo(Mode::Classic), Ok(()));
        let mut outro = jogo_de_teste(3, Mode::Classic, 2);
        assert_eq!(salvador.carregar_partida(Mode::Zen, &mut outro), Origem::Novo);
        partida_nova_do_modo(&outro, Mode::Zen);
        assert_eq!(salvador.carregar_recordes(Mode::Zen), Recordes::zerados());
        assert_eq!(salvador.carregar_modo_ativo(), Mode::Zen);
    }

    #[test]
    fn arquivo_estragado_vira_copia_corrupt() {
        let boa_zen = partida_para_json(&jogo_de_teste(5, Mode::Zen, 2).to_saved());
        let boa_classico = partida_para_json(&jogo_de_teste(5, Mode::Classic, 2).to_saved());
        let casos: Vec<(&str, Vec<u8>)> = vec![
            ("vazio", Vec::new()),
            ("lixo", b"lixo".to_vec()),
            ("bytes que não são texto", vec![0xff, 0xfe, 0x00, 0x41]),
            ("JSON cortado", boa_zen.as_bytes()[..boa_zen.len() / 2].to_vec()),
            ("modo hard", boa_zen.replace("\"zen\"", "\"hard\"").into_bytes()),
            ("zen.json com classic", boa_classico.clone().into_bytes()),
            ("zen.json com endless", boa_classico.replace("classic", "endless").into_bytes()),
            ("tabuleiro 7x8", editar(&boa_zen, |v| { v["board"].as_array_mut().unwrap().pop(); }).into_bytes()),
            (
                "pedra com id repetido",
                editar(&boa_zen, |v| {
                    let id = v["board"][0][0]["id"].clone();
                    v["board"][0][1]["id"] = id;
                })
                .into_bytes(),
            ),
            ("2 MiB de x", "x".repeat(2 * 1024 * 1024).into_bytes()),
        ];
        for (nome, conteudo) in casos {
            let pasta = pasta_de_teste();
            fs::create_dir_all(&pasta.0).unwrap();
            fs::write(pasta.0.join("zen.json"), &conteudo).unwrap();
            fs::write(pasta.0.join("classico.json"), &boa_classico).unwrap();

            let mut jogo = jogo_de_teste(9, Mode::Classic, 4);
            match Salvador::em(pasta.0.clone()).carregar_partida(Mode::Zen, &mut jogo) {
                Origem::Estragado(motivo) => assert!(!motivo.trim().is_empty(), "{nome}: motivo vazio"),
                outra => panic!("{nome}: devia ser Estragado, veio {outra:?}"),
            }
            partida_nova_do_modo(&jogo, Mode::Zen);
            assert_eq!(fs::read(pasta.0.join("zen.json.corrupt")).unwrap(), conteudo, "{nome}: a cópia tem de ser exata");
            assert_eq!(fs::read(pasta.0.join("zen.json")).unwrap(), conteudo, "{nome}: o original fica intacto");
            assert_eq!(fs::read(pasta.0.join("classico.json")).unwrap(), boa_classico.as_bytes(), "{nome}: o outro modo não muda");
            assert!(!pasta.0.join("classico.json.corrupt").exists(), "{nome}");
        }
    }

    #[test]
    fn endless_antigo_vale_como_classico() {
        let pasta = pasta_de_teste();
        fs::create_dir_all(&pasta.0).unwrap();
        let jogo = jogo_de_teste(6, Mode::Classic, 3);
        let antigo = partida_para_json(&jogo.to_saved()).replace("classic", "endless");
        fs::write(pasta.0.join("classico.json"), antigo).unwrap();
        let mut outro = Game::with_random(lcg_random(1));
        assert_eq!(Salvador::em(pasta.0.clone()).carregar_partida(Mode::Classic, &mut outro), Origem::Arquivo);
        mesma_partida(&outro, &jogo);
        assert!(!pasta.0.join("classico.json.corrupt").exists());
    }

    #[test]
    fn gravar_nao_destroi_o_arquivo_antigo_se_falhar() {
        let pasta = pasta_de_teste();
        let salvador = Salvador::em(pasta.0.clone());
        let antiga = jogo_de_teste(1, Mode::Zen, 3);
        salvador.gravar_partida(&antiga).unwrap();
        let texto_antigo = fs::read(pasta.0.join("zen.json")).unwrap();

        // O nome do temporário agora é uma pasta: escrever nele falha.
        fs::create_dir(pasta.0.join("zen.json.tmp")).unwrap();
        let nova = jogo_de_teste(2, Mode::Zen, 5);
        assert!(salvador.gravar_partida(&nova).is_err());

        assert_eq!(fs::read(pasta.0.join("zen.json")).unwrap(), texto_antigo, "o save antigo tem de continuar lá");
        let mut lida = Game::new();
        assert_eq!(salvador.carregar_partida(Mode::Zen, &mut lida), Origem::Arquivo);
        mesma_partida(&lida, &antiga);
    }

    #[test]
    fn pasta_impossivel_devolve_erro_sem_pane() {
        let pasta = pasta_de_teste();
        fs::create_dir_all(&pasta.0).unwrap();
        let arquivo = pasta.0.join("sou-um-arquivo");
        fs::write(&arquivo, "x").unwrap();
        // A pasta de dados ficaria "dentro" de um arquivo: nunca dá para criar.
        let salvador = Salvador::em(arquivo.join("dados"));
        assert!(salvador.tem_pasta());
        let jogo = jogo_de_teste(2, Mode::Zen, 2);
        assert!(salvador.gravar_partida(&jogo).is_err());
        assert!(salvador.gravar_recordes(Mode::Zen, &Recordes::zerados()).is_err());
        assert!(salvador.gravar_modo_ativo(Mode::Classic).is_err());

        let mut outro = jogo_de_teste(3, Mode::Classic, 2);
        assert_eq!(salvador.carregar_partida(Mode::Zen, &mut outro), Origem::Novo);
        partida_nova_do_modo(&outro, Mode::Zen);
        assert_eq!(salvador.carregar_recordes(Mode::Classic), Recordes::zerados());
        assert_eq!(salvador.carregar_modo_ativo(), Mode::Zen);
    }

    #[test]
    fn recordes_ida_volta_e_estragado() {
        let pasta = pasta_de_teste();
        let salvador = Salvador::em(pasta.0.clone());
        let zen = Recordes { pontuacao: 5000, nivel: 4, jogada: 300, terminadas: 0 };
        let classico = Recordes { pontuacao: 900, nivel: 2, jogada: 80, terminadas: 3 };
        salvador.gravar_recordes(Mode::Zen, &zen).unwrap();
        salvador.gravar_recordes(Mode::Classic, &classico).unwrap();
        assert!(pasta.0.join("recordes-zen.json").is_file());
        assert!(pasta.0.join("recordes-classico.json").is_file());
        assert_eq!(salvador.carregar_recordes(Mode::Zen), zen);
        assert_eq!(salvador.carregar_recordes(Mode::Classic), classico);

        // Sem arquivo: zerados, sem criar cópia.
        let vazia = pasta_de_teste();
        assert_eq!(Salvador::em(vazia.0.clone()).carregar_recordes(Mode::Zen), Recordes::zerados());

        // Estragado só no Zen: zerados, cópia exata, e o Clássico continua como estava.
        fs::write(pasta.0.join("recordes-zen.json"), "lixo").unwrap();
        assert_eq!(salvador.carregar_recordes(Mode::Zen), Recordes::zerados());
        assert_eq!(fs::read_to_string(pasta.0.join("recordes-zen.json.corrupt")).unwrap(), "lixo");
        assert_eq!(salvador.carregar_recordes(Mode::Classic), classico);
        assert!(!pasta.0.join("recordes-classico.json.corrupt").exists());

        // Um objeto com campos ruins não é "estragado": só os campos ruins valem 0.
        fs::write(pasta.0.join("recordes-zen.json"), r#"{"bestScore":-1,"bestLevel":3}"#).unwrap();
        assert_eq!(salvador.carregar_recordes(Mode::Zen), Recordes { pontuacao: 0, nivel: 3, jogada: 0, terminadas: 0 });
        // Muito grande: estragado.
        fs::write(pasta.0.join("recordes-zen.json"), recordes_para_json(&zen) + &" ".repeat(2 * 1024 * 1024)).unwrap();
        assert_eq!(salvador.carregar_recordes(Mode::Zen), Recordes::zerados());
    }

    #[test]
    fn modo_ativo() {
        let pasta = pasta_de_teste();
        let salvador = Salvador::em(pasta.0.clone());
        assert_eq!(salvador.carregar_modo_ativo(), Mode::Zen, "arquivo ausente");
        salvador.gravar_modo_ativo(Mode::Classic).unwrap();
        assert_eq!(fs::read_to_string(pasta.0.join("modo-ativo.txt")).unwrap(), "classic");
        assert_eq!(salvador.carregar_modo_ativo(), Mode::Classic);
        salvador.gravar_modo_ativo(Mode::Zen).unwrap();
        assert_eq!(salvador.carregar_modo_ativo(), Mode::Zen);

        for (texto, esperado) in [
            ("", Mode::Zen),
            ("   \n", Mode::Zen),
            (" classic \n", Mode::Classic),
            ("zen\n", Mode::Zen),
            ("Classic", Mode::Zen),
            ("endless", Mode::Zen),
            ("qualquer coisa", Mode::Zen),
        ] {
            fs::write(pasta.0.join("modo-ativo.txt"), texto).unwrap();
            assert_eq!(salvador.carregar_modo_ativo(), esperado, "texto {texto:?}");
        }
        fs::write(pasta.0.join("modo-ativo.txt"), [0xffu8, 0xfe]).unwrap();
        assert_eq!(salvador.carregar_modo_ativo(), Mode::Zen, "bytes que não são texto");
    }

    // ----- Preferências de som (som.json) -----

    fn preferencias_de_teste() -> Preferencias {
        Preferencias {
            sons_do_jogo: false,
            volumes: Volumes { efeitos: 35, musica: 0, ambiente: 100 },
            musica_ligada: true,
            ambiente_ligado: true,
            musica: Musica::SpaceCity,
            ambiente: Ambiente::Trovoada,
            respiracao: Respiracao::Lenta,
        }
    }

    #[test]
    fn salvador_sem_arquivo_devolve_padrao_e_origem_nova() {
        let pasta = pasta_de_teste();
        assert_eq!(Salvador::em(pasta.0.clone()).carregar_preferencias(), (Preferencias::default(), Origem::Novo));
        assert!(!pasta.0.exists(), "ler não cria a pasta nem arquivo");
        // Pasta que existe mas sem som.json: igual.
        fs::create_dir_all(&pasta.0).unwrap();
        assert_eq!(Salvador::em(pasta.0.clone()).carregar_preferencias(), (Preferencias::default(), Origem::Novo));
        assert!(!pasta.0.join("som.json.corrupt").exists());
    }

    #[test]
    fn salvador_grava_e_le() {
        let pasta = pasta_de_teste();
        let salvador = Salvador::em(pasta.0.clone());
        let preferencias = preferencias_de_teste();
        salvador.gravar_preferencias(&preferencias).unwrap();
        assert!(pasta.0.join("som.json").is_file());
        assert_eq!(salvador.carregar_preferencias(), (preferencias.clone(), Origem::Arquivo));
        // Gravar de novo troca o conteúdo.
        let outra = Preferencias { sons_do_jogo: true, ..preferencias };
        salvador.gravar_preferencias(&outra).unwrap();
        assert_eq!(salvador.carregar_preferencias(), (outra, Origem::Arquivo));
        // Os outros arquivos não são tocados.
        assert!(!pasta.0.join("zen.json").exists() && !pasta.0.join("modo-ativo.txt").exists());
    }

    #[test]
    fn salvador_com_arquivo_estragado_guarda_copia_corrupt_e_devolve_padrao() {
        let casos: Vec<(&str, Vec<u8>)> = vec![
            ("vazio", Vec::new()),
            ("lixo", b"lixo".to_vec()),
            ("JSON cortado", br#"{"soundOn":false,"audio":{"effec"#.to_vec()),
            ("lista", b"[]".to_vec()),
            ("bytes que não são texto", vec![0xff, 0xfe, 0x00, 0x41]),
            ("2 MiB de x", "x".repeat(2 * 1024 * 1024).into_bytes()),
        ];
        for (nome, conteudo) in casos {
            let pasta = pasta_de_teste();
            fs::create_dir_all(&pasta.0).unwrap();
            fs::write(pasta.0.join("som.json"), &conteudo).unwrap();
            fs::write(pasta.0.join("recordes-zen.json"), "{}").unwrap();

            match Salvador::em(pasta.0.clone()).carregar_preferencias() {
                (lidas, Origem::Estragado(motivo)) => {
                    assert_eq!(lidas, Preferencias::default(), "{nome}");
                    assert!(!motivo.trim().is_empty(), "{nome}: motivo vazio");
                }
                outra => panic!("{nome}: devia ser Estragado, veio {outra:?}"),
            }
            assert_eq!(fs::read(pasta.0.join("som.json.corrupt")).unwrap(), conteudo, "{nome}: a cópia tem de ser exata");
            assert_eq!(fs::read(pasta.0.join("som.json")).unwrap(), conteudo, "{nome}: o original fica intacto");
            assert!(!pasta.0.join("recordes-zen.json.corrupt").exists(), "{nome}: os outros arquivos não mudam");
        }
        // Um objeto com campos ruins não é "estragado": só os campos ruins voltam ao padrão.
        let pasta = pasta_de_teste();
        fs::create_dir_all(&pasta.0).unwrap();
        fs::write(pasta.0.join("som.json"), r#"{"audio":{"effects":"alto","music":10}}"#).unwrap();
        let (lidas, origem) = Salvador::em(pasta.0.clone()).carregar_preferencias();
        assert_eq!(origem, Origem::Arquivo);
        assert_eq!((lidas.volumes.efeitos, lidas.volumes.musica), (80, 10));
        assert!(!pasta.0.join("som.json.corrupt").exists());
    }

    #[test]
    fn salvador_sem_pasta_nao_da_panico() {
        let nenhum = Salvador::nenhum();
        // Sem pasta, gravar não faz nada e devolve Ok, como os outros gravadores.
        assert_eq!(nenhum.gravar_preferencias(&preferencias_de_teste()), Ok(()));
        assert_eq!(nenhum.gravar_recordes(Mode::Zen, &Recordes::zerados()), Ok(()));
        assert_eq!(nenhum.carregar_preferencias(), (Preferencias::default(), Origem::Novo));

        // Pasta impossível (dentro de um arquivo): gravar dá erro, sem pânico; ler dá o padrão.
        let pasta = pasta_de_teste();
        fs::create_dir_all(&pasta.0).unwrap();
        let arquivo = pasta.0.join("sou-um-arquivo");
        fs::write(&arquivo, "x").unwrap();
        let impossivel = Salvador::em(arquivo.join("dados"));
        assert!(impossivel.gravar_preferencias(&preferencias_de_teste()).is_err());
        assert_eq!(impossivel.carregar_preferencias(), (Preferencias::default(), Origem::Novo));
    }

    #[test]
    fn gravacao_em_duas_etapas_nao_deixa_tmp() {
        let pasta = pasta_de_teste();
        let salvador = Salvador::em(pasta.0.clone());
        salvador.gravar_preferencias(&preferencias_de_teste()).unwrap();
        salvador.gravar_preferencias(&Preferencias::default()).unwrap();
        let nomes: Vec<String> = fs::read_dir(&pasta.0).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(nomes, ["som.json"], "só o arquivo final, sem .tmp");

        // Se a escrita falha (o nome do temporário virou uma pasta), o som.json bom continua lá.
        let boa = preferencias_de_teste();
        salvador.gravar_preferencias(&boa).unwrap();
        let texto_antigo = fs::read(pasta.0.join("som.json")).unwrap();
        fs::create_dir(pasta.0.join("som.json.tmp")).unwrap();
        assert!(salvador.gravar_preferencias(&Preferencias::default()).is_err());
        assert_eq!(fs::read(pasta.0.join("som.json")).unwrap(), texto_antigo);
        assert_eq!(salvador.carregar_preferencias(), (boa, Origem::Arquivo));
    }
}
