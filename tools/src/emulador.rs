//! O teste do celular virtual: abre o APK num emulador Android e JOGA de verdade.
//!
//!   1. o app abre direto no jogo (Zen) e continua aberto;
//!   2. Zen: 5 vezes, toca em "Dica", arrasta o dedo de uma pedra da dica até a outra, espera a
//!      animação terminar e confere que a jogada valeu e os pontos subiram;
//!   3. Clássico: troca de aba e faz 5 jogadas tocando numa pedra e depois na vizinha;
//!   4. Novo jogo (com a confirmação) no Clássico zera os pontos;
//!   5. volta ao Zen e confere que os pontos do Zen continuam lá.
//!
//! Guarda o registro de erros (logcat) e uma foto da tela de cada etapa numa pasta. Foi escrito
//! primeiro em shell (`tools/teste-emulador.sh`); aqui faz a mesma coisa, mas o controle de fluxo
//! (esperas, conferências, mensagens de erro) tem testes que rodam contra um celular falso.
//!
//! O app escreve no registro do Android linhas que começam com `PRISMA_M1` (veja `App::registro`
//! em `game/src/app.rs`): a posição do tabuleiro e dos botões, cada toque que recebe e o que
//! acontece no jogo. O módulo `registro` lê essas linhas.

use crate::base64;
use crate::registro::{centro_da_casa, deslocamento, Registro};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Quanto tempo cada chamada do `adb` pode levar antes de ser interrompida.
const LIMITE_DO_ADB: Duration = Duration::from_secs(60);

/// O que um comando do `adb` respondeu.
#[derive(Debug)]
pub struct Resposta {
    /// O comando terminou com código de saída 0.
    pub sucesso: bool,
    /// O que ele escreveu na saída normal (bytes, porque a foto da tela é um PNG).
    pub texto: Vec<u8>,
    /// O que ele escreveu na saída de erros.
    pub erro: String,
}

impl Resposta {
    fn texto_lossy(&self) -> String {
        String::from_utf8_lossy(&self.texto).into_owned()
    }
}

/// O celular (ou o de mentira, nos testes). É por esta porta que o teste enxerga o mundo.
pub trait Celular {
    /// Roda `adb <args>`. `Err` só quando o comando nem rodou ou passou do limite de tempo;
    /// um comando que rodou e saiu com erro volta como `Ok` com `sucesso == false`.
    fn adb(&mut self, args: &[&str]) -> Result<Resposta, String>;
    /// Espera `ms` milissegundos (o celular falso só adianta um relógio de mentira).
    fn dormir(&mut self, ms: u64);
    /// Reduz a foto para caber no registro: `convert <origem> -resize 270x -colors 48 PNG8:<destino>`.
    /// `Ok(true)` = reduziu; `Ok(false)` = o `convert` (ImageMagick) não existe e a foto é pulada.
    fn reduzir_foto(&mut self, origem: &Path, destino: &Path) -> Result<bool, String>;
}

/// O celular de verdade: chama o programa `adb` (ou o que a variável `ADB` indicar).
pub struct AdbReal {
    programa: String,
    convert: String,
    limite: Duration,
}

impl AdbReal {
    pub fn novo() -> AdbReal {
        let programa = std::env::var("ADB").ok().filter(|p| !p.is_empty()).unwrap_or_else(|| "adb".to_string());
        AdbReal::com_programa(&programa, LIMITE_DO_ADB)
    }

    pub fn com_programa(programa: &str, limite: Duration) -> AdbReal {
        AdbReal { programa: programa.to_string(), convert: "convert".to_string(), limite }
    }
}

/// Um arquivo temporário que se apaga sozinho ao sair de cena.
struct ArquivoTemporario(PathBuf);

impl ArquivoTemporario {
    fn novo(rotulo: &str) -> ArquivoTemporario {
        static CONTADOR: AtomicUsize = AtomicUsize::new(0);
        let numero = CONTADOR.fetch_add(1, Ordering::Relaxed);
        ArquivoTemporario(std::env::temp_dir().join(format!("prisma_tools_adb_{}_{numero}_{rotulo}", std::process::id())))
    }
}

impl Drop for ArquivoTemporario {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

impl Celular for AdbReal {
    fn adb(&mut self, args: &[&str]) -> Result<Resposta, String> {
        let comando = format!("{} {}", self.programa, args.join(" "));
        // A saída vai para arquivos, não para "pipes": o primeiro `adb` liga o servidor dele, que
        // herda a saída e a mantém aberta; esperar o fim de um pipe assim nunca acabaria.
        let (saida, erro) = (ArquivoTemporario::novo("saida"), ArquivoTemporario::novo("erro"));
        let abrir = |arquivo: &ArquivoTemporario| {
            fs::File::create(&arquivo.0).map_err(|e| format!("não consegui criar {}: {e}", arquivo.0.display()))
        };
        let mut filho = Command::new(&self.programa)
            .args(args)
            .stdin(Stdio::null())
            .stdout(abrir(&saida)?)
            .stderr(abrir(&erro)?)
            .spawn()
            .map_err(|e| format!("não consegui rodar '{}': {e}", self.programa))?;
        let prazo = Instant::now() + self.limite;
        let status = loop {
            match filho.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= prazo => {
                    let _ = filho.kill();
                    let _ = filho.wait();
                    return Err(format!("o comando '{comando}' passou de {:.1} s e foi interrompido", self.limite.as_secs_f32()));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(e) => return Err(format!("não consegui esperar o comando '{comando}': {e}")),
            }
        };
        Ok(Resposta {
            sucesso: status.success(),
            texto: fs::read(&saida.0).unwrap_or_default(),
            erro: String::from_utf8_lossy(&fs::read(&erro.0).unwrap_or_default()).into_owned(),
        })
    }

    fn dormir(&mut self, ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    fn reduzir_foto(&mut self, origem: &Path, destino: &Path) -> Result<bool, String> {
        let resultado = Command::new(&self.convert)
            .arg(origem)
            .args(["-resize", "270x", "-colors", "48"])
            .arg(format!("PNG8:{}", destino.display()))
            .stdin(Stdio::null())
            .output();
        match resultado {
            Ok(saida) if saida.status.success() => Ok(true),
            Ok(saida) => Err(format!("o convert falhou: {}", String::from_utf8_lossy(&saida.stderr).trim())),
            // Sem ImageMagick neste computador: a foto é pulada, o teste continua.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(format!("não consegui rodar o convert: {e}")),
        }
    }
}

/// O identificador do app: a linha `package_name = "..."` do `game/Cargo.toml` (no começo da linha).
pub fn pacote_do_cargo_toml(texto: &str) -> Option<String> {
    texto
        .lines()
        .find_map(|linha| {
            let resto = linha.strip_prefix("package_name = \"")?;
            Some(resto[..resto.rfind('"')?].to_string())
        })
        .filter(|pacote| !pacote.is_empty())
}

fn ler_pacote(caminho: &Path) -> Result<String, String> {
    let texto = fs::read_to_string(caminho)
        .map_err(|e| format!("não consegui ler {} (rode o programa da raiz do projeto): {e}", caminho.display()))?;
    pacote_do_cargo_toml(&texto)
        .ok_or_else(|| format!("Não achei package_name em {} (rode o programa da raiz do projeto).", caminho.display()))
}

/// O primeiro `.apk` da pasta, em ordem alfabética.
fn achar_apk(pasta: &Path) -> Result<PathBuf, String> {
    let itens = fs::read_dir(pasta).map_err(|e| format!("não consegui ler a pasta {}: {e}", pasta.display()))?;
    let mut apks: Vec<PathBuf> =
        itens.filter_map(|item| item.ok()).map(|item| item.path()).filter(|c| c.extension().is_some_and(|e| e == "apk")).collect();
    apks.sort();
    apks.into_iter().next().ok_or_else(|| format!("não achei nenhum .apk em {}", pasta.display()))
}

/// As linhas do registro que mais ajudam a achar um erro (sem ligar para maiúsculas), no máximo 200.
pub fn resumo(logcat: &str) -> String {
    const PADROES: [&str; 12] = [
        "prisma_m1",
        "fatal",
        "androidruntime",
        "panicked",
        "sigsegv",
        "sigabrt",
        "abort message",
        "unsatisfiedlink",
        "dlopen",
        "libprisma",
        "miniquad",
        "debug  ",
    ];
    let linhas: Vec<&str> = logcat
        .lines()
        .filter(|linha| {
            let minuscula = linha.to_lowercase();
            PADROES.iter().any(|padrao| minuscula.contains(padrao))
        })
        .take(200)
        .collect();
    format!("===== logcat (resumo) =====\n{}\n===== fim do resumo =====", linhas.join("\n"))
}

/// A foto reduzida em texto (base64), entre dois marcadores, para ler direto no registro do GitHub.
fn bloco_da_foto(nome: &str, png: &[u8]) -> String {
    format!("TELA_BASE64_INICIO {nome}\n{}\nTELA_BASE64_FIM", base64::codificar(png))
}

/// Reduz e imprime uma foto que já está guardada em `arquivo`. Sem ImageMagick, ou se a redução
/// falhar, a foto só não aparece no registro: isso nunca reprova o teste.
fn imprimir_foto(celular: &mut dyn Celular, saida: &Path, nome: &str, arquivo: &Path) {
    if fs::metadata(arquivo).map(|m| m.len()).unwrap_or(0) == 0 {
        return;
    }
    let pequena = saida.join(format!("pequena-{nome}.png"));
    match celular.reduzir_foto(arquivo, &pequena) {
        Ok(true) => match fs::read(&pequena) {
            Ok(bytes) => println!("{}", bloco_da_foto(nome, &bytes)),
            Err(e) => println!("AVISO: não consegui ler a foto reduzida {nome}: {e}"),
        },
        Ok(false) => {}
        Err(e) => println!("AVISO: a foto {nome} não foi impressa: {e}"),
    }
}

/// O que sai quando o teste falha: o motivo, o registro guardado, o resumo dele e a foto da tela.
/// Cada parte é tentada sozinha: se o celular nem responde, as outras ainda saem.
fn relatar_falha(celular: &mut dyn Celular, saida: &Path, motivo: &str) {
    println!("ERRO: {motivo}");
    let logcat = match celular.adb(&["logcat", "-d"]) {
        Ok(resposta) => {
            let _ = fs::write(saida.join("logcat.txt"), &resposta.texto);
            resposta.texto_lossy()
        }
        Err(e) => {
            println!("AVISO: não consegui ler o registro do celular: {e}");
            String::new()
        }
    };
    let tela = saida.join("tela-do-erro.png");
    if let Ok(resposta) = celular.adb(&["exec-out", "screencap", "-p"]) {
        let _ = fs::write(&tela, &resposta.texto);
    }
    println!("{}", resumo(&logcat));
    imprimir_foto(celular, saida, "do-erro", &tela);
}

/// O teste em andamento: o celular, o último registro lido e o que já se descobriu dele.
struct Teste<'a> {
    celular: &'a mut dyn Celular,
    pacote: &'a str,
    saida: &'a Path,
    /// O último despejo do logcat (texto) e as linhas `PRISMA_M1` lidas dele.
    logcat: String,
    registro: Registro,
    /// Onde o tabuleiro está, em pixels do app: `(x, y, lado)`.
    tabuleiro: (i32, i32, i32),
    /// A diferença entre a tela e a janela do app (achada na calibração).
    deslocamento: (i32, i32),
    pontos_zen: u64,
}

impl Teste<'_> {
    /// Lê o registro de novo (e guarda em `logcat.txt`, que vira artefato do GitHub).
    fn ler_registro(&mut self) -> Result<(), String> {
        let resposta = self.celular.adb(&["logcat", "-d"])?;
        if let Err(e) = fs::write(self.saida.join("logcat.txt"), &resposta.texto) {
            println!("AVISO: não consegui guardar o logcat.txt: {e}");
        }
        self.logcat = resposta.texto_lossy();
        self.registro = Registro::de(&self.logcat);
        Ok(())
    }

    /// Confere que o app continua aberto. `quando` diz o que acabou de acontecer.
    fn continua_aberto(&mut self, quando: &str) -> Result<(), String> {
        let resposta = self.celular.adb(&["shell", "pidof", self.pacote])?;
        if resposta.sucesso && resposta.texto.iter().any(|b| !b.is_ascii_whitespace()) {
            Ok(())
        } else {
            Err(format!("o app FECHOU sozinho {quando}."))
        }
    }

    fn centro_do_botao(&self, nome: &str) -> Result<(i32, i32), String> {
        self.registro.ultimo_botao(nome).ok_or_else(|| format!("o registro não tem a posição do botão '{nome}'"))
    }

    fn ler_tabuleiro(&mut self) -> Result<(), String> {
        match self.registro.ultimo_tabuleiro() {
            Some((x, y, lado)) if lado > 0 => {
                self.tabuleiro = (x, y, lado);
                Ok(())
            }
            _ => Err("o registro não tem a posição do tabuleiro".to_string()),
        }
    }

    /// O centro de uma casa (0 a 63) em coordenadas da TELA, já com o deslocamento da janela.
    fn centro_da_casa(&self, casa: usize) -> (i32, i32) {
        centro_da_casa(self.tabuleiro, self.deslocamento, casa)
    }

    /// Toca num ponto da TELA.
    fn tocar_na_tela(&mut self, x: i32, y: i32) -> Result<(), String> {
        self.celular.adb(&["shell", "input", "tap", &x.to_string(), &y.to_string()])?;
        Ok(())
    }

    fn arrastar(&mut self, de: (i32, i32), ate: (i32, i32)) -> Result<(), String> {
        let (x1, y1, x2, y2) = (de.0.to_string(), de.1.to_string(), ate.0.to_string(), ate.1.to_string());
        self.celular.adb(&["shell", "input", "swipe", &x1, &y1, &x2, &y2, "200"])?;
        Ok(())
    }

    /// Toca num botão (pela posição do registro) e espera um pouco o app responder.
    fn tocar_botao(&mut self, nome: &str) -> Result<(), String> {
        let (x, y) = self.centro_do_botao(nome)?;
        let (tela_x, tela_y) = (x.saturating_add(self.deslocamento.0), y.saturating_add(self.deslocamento.1));
        println!("Tocando em '{nome}' (app: {x} {y}; tela: {tela_x} {tela_y})");
        self.tocar_na_tela(tela_x, tela_y)?;
        self.celular.dormir(1000);
        self.ler_registro()
    }

    /// Toca em "Dica" e devolve as duas casas. Exige uma linha NOVA no registro, para nunca usar a
    /// dica de uma rodada anterior caso o toque não tenha pegado.
    fn pedir_dica(&mut self) -> Result<(usize, usize), String> {
        let antes = self.registro.contar_dicas();
        self.tocar_botao("dica")?;
        if self.registro.contar_dicas() <= antes {
            return Err("o toque em 'Dica' não gerou uma dica nova".to_string());
        }
        self.registro.ultima_dica().ok_or_else(|| "a linha da dica veio sem as duas casas".to_string())
    }

    /// O app mede os toques a partir do canto da PRÓPRIA janela, mas o `adb input` usa coordenadas
    /// da TELA; no emulador a janela começa abaixo da barra de status (uns 24 a 28 px). Para
    /// descobrir o deslocamento sem depender da versão do Android, toca num ponto da tela que não
    /// faz nada no jogo (a margem de cima, acima das abas) e compara com a linha `toque` do app.
    fn calibrar_toque(&mut self) -> Result<(), String> {
        let (tx, _, lado) = self.tabuleiro;
        let antes = self.registro.contar_toques();
        let tela = (tx.saturating_add(lado / 2), 60);
        println!("Calibrando: tocando na margem de cima, na tela em ({},{})", tela.0, tela.1);
        self.tocar_na_tela(tela.0, tela.1)?;
        self.celular.dormir(1000);
        self.ler_registro()?;
        if self.registro.contar_toques() <= antes {
            return Err("o app não registrou o toque de calibração".to_string());
        }
        let visto = self.registro.ultimo_toque().ok_or_else(|| "a linha do toque veio sem x e y".to_string())?;
        let (dx, dy) = deslocamento(tela, visto);
        println!("O app viu o toque em ({},{}): a janela está deslocada em ({dx},{dy}) px da tela", visto.0, visto.1);
        // Um deslocamento absurdo quer dizer que o toque caiu em outro lugar.
        if !(0..=100).contains(&dx) || !(0..=200).contains(&dy) {
            return Err(format!("deslocamento da janela fora do esperado: ({dx},{dy})"));
        }
        self.deslocamento = (dx, dy);
        Ok(())
    }

    /// Espera (até 30 s, de meio em meio segundo) o registro ter mais linhas `animacao_terminou`
    /// do que `antes`.
    fn esperar_animacao(&mut self, antes: usize) -> Result<(), String> {
        for _ in 0..60 {
            self.celular.dormir(500);
            self.ler_registro()?;
            if self.registro.contar_animacoes() > antes {
                return Ok(());
            }
        }
        Err("a animação não terminou em 30 s".to_string())
    }

    /// Confere a última jogada (válida e com mais pontos que `anteriores`) e devolve os pontos novos.
    fn conferir_jogada(&self, modo: &str, rodada: usize, anteriores: u64) -> Result<u64, String> {
        let Some((valida, pontos, nivel)) = self.registro.ultima_jogada() else {
            return Err(format!("a jogada {rodada} do {modo} não foi válida: o registro não tem nenhuma linha 'jogada'"));
        };
        let linha = format!("PRISMA_M1 jogada valida={valida} pontos={pontos} nivel={nivel}");
        println!("{linha}");
        if !valida {
            return Err(format!("a jogada {rodada} do {modo} não foi válida: {linha}"));
        }
        if pontos <= anteriores {
            return Err(format!("os pontos do {modo} não subiram na jogada {rodada} (antes {anteriores}, agora '{pontos}')"));
        }
        Ok(pontos)
    }

    /// Tira a foto da tela, guarda e imprime reduzida. Uma foto que falha não reprova o teste.
    fn foto(&mut self, nome: &str) {
        let arquivo = self.saida.join(format!("tela-{nome}.png"));
        match self.celular.adb(&["exec-out", "screencap", "-p"]) {
            Ok(resposta) => {
                if let Err(e) = fs::write(&arquivo, &resposta.texto) {
                    println!("AVISO: não consegui guardar a foto {nome}: {e}");
                    return;
                }
                imprimir_foto(self.celular, self.saida, nome, &arquivo);
            }
            Err(e) => println!("AVISO: não consegui tirar a foto {nome}: {e}"),
        }
    }

    fn mostrar_versao(&mut self) -> Result<(), String> {
        for propriedade in ["ro.build.version.release", "ro.product.cpu.abilist"] {
            let resposta = self.celular.adb(&["shell", "getprop", propriedade])?;
            println!("{}", resposta.texto_lossy().trim_end());
        }
        Ok(())
    }

    fn instalar(&mut self, apk: &Path) -> Result<(), String> {
        let resposta = self.celular.adb(&["install", "-r", &apk.to_string_lossy()])?;
        let texto = format!("{}{}", resposta.texto_lossy(), resposta.erro);
        let _ = fs::write(self.saida.join("instalacao.txt"), &texto);
        if !resposta.sucesso {
            return Err(format!("A instalação FALHOU: {}", texto.trim()));
        }
        println!("{}", texto.trim_end());
        Ok(())
    }

    /// 1. Abertura: o app abre direto no jogo.
    fn abrir(&mut self) -> Result<(), String> {
        self.celular.adb(&["logcat", "-c"])?;
        let resposta =
            self.celular.adb(&["shell", "monkey", "-p", self.pacote, "-c", "android.intent.category.LAUNCHER", "1"])?;
        let abertura = format!("{}{}", resposta.texto_lossy(), resposta.erro);
        let _ = fs::write(self.saida.join("abertura.txt"), &abertura);
        println!("{}", abertura.trim_end());
        self.celular.dormir(15_000);
        self.ler_registro()?;
        self.continua_aberto("depois de abrir")?;
        if !self.registro.tem_pronto("zen") {
            return Err("faltou a linha 'PRISMA_M1 pronto modo=zen' (o jogo não abriu?).".to_string());
        }
        println!("OK: o jogo abriu no modo Zen.");
        self.ler_tabuleiro()?;
        let (x, y, lado) = self.tabuleiro;
        println!("Tabuleiro (coordenadas do app): x={x} y={y} lado={lado}");
        for botao in ["zen", "classico", "embaralhar", "dica", "novo_jogo", "confirmar"] {
            self.centro_do_botao(botao)?;
        }
        self.foto("1-zen-inicio");
        self.calibrar_toque()
    }

    /// 2. Zen: 5 jogadas arrastando o dedo.
    fn zen(&mut self) -> Result<(), String> {
        let mut pontos = 0;
        for rodada in 1..=5 {
            let (a, b) = self.pedir_dica()?;
            println!("Rodada {rodada} (Zen): a dica pede trocar a casa {a} com a {b}");
            if rodada == 1 {
                self.foto("2-zen-dica");
            }
            // Não precisa esperar a dica sumir: o arrasto abaixo apaga a dica sozinho.
            let animacoes = self.registro.contar_animacoes();
            let (de, ate) = (self.centro_da_casa(a), self.centro_da_casa(b));
            println!("Arrastando de ({},{}) até ({},{})", de.0, de.1, ate.0, ate.1);
            self.arrastar(de, ate)?;
            self.esperar_animacao(animacoes)?;
            self.continua_aberto(&format!("durante a jogada {rodada} do Zen"))?;
            pontos = self.conferir_jogada("Zen", rodada, pontos)?;
            if rodada == 3 {
                self.foto("3-zen-depois-da-jogada");
            }
            self.celular.dormir(1000);
        }
        println!("OK: 5 jogadas no Zen, pontos em {pontos}.");
        self.pontos_zen = pontos;
        self.foto("4-zen-fim");
        Ok(())
    }

    /// 3. Clássico: 5 jogadas tocando numa pedra e depois na vizinha.
    fn classico(&mut self) -> Result<(), String> {
        self.tocar_botao("classico")?;
        self.celular.dormir(1000);
        self.ler_registro()?;
        if self.registro.ultimo_modo() != Some(("classico".to_string(), 0)) {
            return Err("faltou 'PRISMA_M1 modo=classico pontos=0' depois de tocar na aba Clássico".to_string());
        }
        println!("OK: mudou para o Clássico (começa com 0 pontos).");
        self.ler_tabuleiro()?; // o tabuleiro muda de lugar quando some o botão Embaralhar
        let mut pontos = 0;
        for rodada in 1..=5 {
            let (a, b) = self.pedir_dica()?;
            println!("Rodada {rodada} (Clássico): a dica pede trocar a casa {a} com a {b}");
            let animacoes = self.registro.contar_animacoes();
            let (primeiro, segundo) = (self.centro_da_casa(a), self.centro_da_casa(b));
            println!("Tocando em ({},{}) e depois em ({},{})", primeiro.0, primeiro.1, segundo.0, segundo.1);
            self.tocar_na_tela(primeiro.0, primeiro.1)?;
            self.celular.dormir(500);
            self.tocar_na_tela(segundo.0, segundo.1)?;
            self.esperar_animacao(animacoes)?;
            self.continua_aberto(&format!("durante a jogada {rodada} do Clássico"))?;
            pontos = self.conferir_jogada("Clássico", rodada, pontos)?;
            self.celular.dormir(1000);
        }
        println!("OK: 5 jogadas no Clássico, pontos em {pontos}.");
        self.foto("5-classico");
        Ok(())
    }

    /// 4. Novo jogo no Clássico (com a confirmação).
    fn novo_jogo(&mut self) -> Result<(), String> {
        self.tocar_botao("novo_jogo")?;
        self.foto("6-classico-confirmando");
        self.tocar_botao("confirmar")?;
        if !self.registro.tem_novo_jogo("classico", 0) {
            return Err("faltou 'PRISMA_M1 novo_jogo modo=classico pontos=0' depois de confirmar".to_string());
        }
        println!("OK: Novo jogo no Clássico zerou os pontos.");
        self.continua_aberto("depois do Novo jogo")
    }

    /// 5. Volta ao Zen: os pontos do Zen continuam.
    fn volta_ao_zen(&mut self) -> Result<(), String> {
        self.tocar_botao("zen")?;
        self.celular.dormir(1000);
        self.ler_registro()?;
        let pontos = self.pontos_zen;
        match self.registro.ultimo_modo() {
            Some((modo, p)) if modo == "zen" && p == pontos => println!("PRISMA_M1 modo=zen pontos={p}"),
            outra => {
                let linha = outra.map(|(modo, p)| format!("PRISMA_M1 modo={modo} pontos={p}")).unwrap_or_default();
                return Err(format!("ao voltar para o Zen os pontos não eram {pontos}: '{linha}'"));
            }
        }
        println!("OK: de volta ao Zen, os {pontos} pontos continuam.");
        self.continua_aberto("ao voltar para o Zen")?;
        self.foto("7-zen-de-volta");
        Ok(())
    }
}

fn executar(celular: &mut dyn Celular, pacote: &str, pasta_apk: &Path, saida: &Path) -> Result<(), String> {
    println!("Pacote: {pacote}");
    fs::create_dir_all(saida).map_err(|e| format!("não consegui criar a pasta {}: {e}", saida.display()))?;
    let apk = achar_apk(pasta_apk)?;
    println!("APK: {}", apk.display());
    let mut teste = Teste {
        celular,
        pacote,
        saida,
        logcat: String::new(),
        registro: Registro::de(""),
        tabuleiro: (0, 0, 0),
        deslocamento: (0, 0),
        pontos_zen: 0,
    };
    teste.mostrar_versao()?;
    teste.instalar(&apk)?;
    teste.abrir()?;
    teste.zen()?;
    teste.classico()?;
    teste.novo_jogo()?;
    teste.volta_ao_zen()?;
    println!("{}", resumo(&teste.logcat));
    println!("OK: o jogo rodou no celular virtual (Zen e Clássico, toque e arrasto).");
    Ok(())
}

/// Roda o teste inteiro. Se algo falhar, imprime o motivo, guarda o registro e a foto da tela
/// (`relatar_falha`) e devolve o motivo como erro.
pub fn rodar(celular: &mut dyn Celular, pacote: &str, pasta_apk: &Path, saida: &Path) -> Result<(), String> {
    let resultado = executar(&mut *celular, pacote, pasta_apk, saida);
    if let Err(motivo) = &resultado {
        relatar_falha(celular, saida, motivo);
    }
    resultado
}

/// O que o `main` chama: lê o pacote do `game/Cargo.toml` (a partir da pasta atual, como o script
/// antigo) e roda contra o celular de verdade. Devolve o código de saída do programa (0 ou 1).
pub fn rodar_do_programa(pasta_apk: &Path, saida: &Path) -> i32 {
    let pacote = match ler_pacote(Path::new("game/Cargo.toml")) {
        Ok(pacote) => pacote,
        Err(erro) => {
            println!("{erro}");
            return 1;
        }
    };
    match rodar(&mut AdbReal::novo(), &pacote, pasta_apk, saida) {
        Ok(()) => 0,
        Err(_) => 1, // o motivo e o resumo já foram impressos por `rodar`
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    const PACOTE: &str = "com.exemplo.prisma_teste";

    /// Uma pasta temporária que se apaga sozinha quando o teste acaba.
    struct Pasta(PathBuf);

    impl Pasta {
        fn nova(nome: &str) -> Pasta {
            let caminho = std::env::temp_dir().join(format!("prisma_tools_{}_{nome}", std::process::id()));
            let _ = fs::remove_dir_all(&caminho);
            fs::create_dir_all(&caminho).unwrap();
            Pasta(caminho)
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ModoFalso {
        Zen,
        Classico,
    }

    /// Um celular de mentira. Obedece aos mesmos comandos do `adb` que o teste usa, guarda um
    /// "registro" (logcat) e roda um mini-jogo: toques e arrastos chegam em coordenadas da TELA,
    /// o jogo falso desconta o deslocamento da janela (como o Android de verdade) e só reage se o
    /// teste acertou a casa ou o botão. O tempo é um relógio de mentira: `dormir` só soma.
    struct CelularFalso {
        // ---- o que o teste liga para provocar cada falha ----
        deslocamento: (i32, i32),
        sem_pronto: bool,
        sem_botao: Option<&'static str>,
        app_fechado: bool,
        fecha_depois_de_jogadas: Option<usize>,
        ignora_toques: bool,
        sem_dica: bool,
        animacao_nunca_termina: bool,
        jogada_invalida: bool,
        pontos_param: bool,
        instalacao_falha: bool,
        sem_convert: bool,
        sem_tabuleiro: bool,
        lado_zero_no_registro: bool,
        classico_comeca_com_pontos: bool,
        novo_jogo_nao_zera: bool,
        zen_perde_os_pontos: bool,
        // ---- o que ele guarda ----
        relogio_ms: u64,
        logcat: String,
        instalado: bool,
        aberto: bool,
        modo: ModoFalso,
        pontos: [u64; 2], // [Zen, Clássico]
        selecionada: Option<usize>,
        confirmando: bool,
        dica: Option<(usize, usize)>,
        dicas_dadas: usize,
        jogadas: usize,
        toques: usize,
        arrastos: usize,
        fotos_reduzidas: usize,
        relogio_no_ultimo_arrasto: u64,
    }

    impl CelularFalso {
        fn novo() -> CelularFalso {
            CelularFalso {
                deslocamento: (0, 24),
                sem_pronto: false,
                sem_botao: None,
                app_fechado: false,
                fecha_depois_de_jogadas: None,
                ignora_toques: false,
                sem_dica: false,
                animacao_nunca_termina: false,
                jogada_invalida: false,
                pontos_param: false,
                instalacao_falha: false,
                sem_convert: false,
                sem_tabuleiro: false,
                lado_zero_no_registro: false,
                classico_comeca_com_pontos: false,
                novo_jogo_nao_zera: false,
                zen_perde_os_pontos: false,
                relogio_ms: 0,
                logcat: String::new(),
                instalado: false,
                aberto: false,
                modo: ModoFalso::Zen,
                pontos: [0, 0],
                selecionada: None,
                confirmando: false,
                dica: None,
                dicas_dadas: 0,
                jogadas: 0,
                toques: 0,
                arrastos: 0,
                fotos_reduzidas: 0,
                relogio_no_ultimo_arrasto: 0,
            }
        }

        /// Escreve uma linha no registro, no formato do logcat (hora, processo, nível, etiqueta).
        fn escrever(&mut self, mensagem: &str) {
            let ms = self.relogio_ms;
            self.logcat.push_str(&format!(
                "11-21 20:{:02}:{:02}.{:03}  3597  3633 I SAPP    : {mensagem}\n",
                (ms / 60_000) % 60,
                (ms / 1000) % 60,
                ms % 1000
            ));
        }

        /// Cada comando `input` do `adb` também deixa linhas de outros programas no registro.
        fn ruido(&mut self) {
            self.logcat.push_str("11-21 20:36:14.229  5069  5069 D AndroidRuntime: Calling main entry com.android.commands.input.Input\n");
        }

        fn indice(&self) -> usize {
            match self.modo {
                ModoFalso::Zen => 0,
                ModoFalso::Classico => 1,
            }
        }

        /// O tabuleiro muda de lugar no Clássico (some o botão Embaralhar): quem não ler de novo erra a casa.
        fn tabuleiro_atual(&self) -> (i32, i32, i32) {
            match self.modo {
                ModoFalso::Zen => (10, 194, 304),
                ModoFalso::Classico => (10, 170, 304),
            }
        }

        fn botoes_normais(&self) -> Vec<(&'static str, i32, i32)> {
            match self.modo {
                ModoFalso::Zen => vec![
                    ("zen", 83, 96),
                    ("classico", 237, 96),
                    ("embaralhar", 58, 517),
                    ("dica", 160, 517),
                    ("novo_jogo", 262, 517),
                ],
                ModoFalso::Classico => vec![("zen", 83, 96), ("classico", 237, 96), ("dica", 109, 517), ("novo_jogo", 211, 517)],
            }
        }

        fn botoes_da_confirmacao() -> [(&'static str, i32, i32); 3] {
            [("jogar_novamente", 160, 397), ("cancelar", 99, 346), ("confirmar", 221, 346)]
        }

        fn escrever_geometria(&mut self) {
            let (x, y, lado) = self.tabuleiro_atual();
            if !self.sem_tabuleiro {
                let lado_escrito = if self.lado_zero_no_registro { 0 } else { lado };
                self.escrever(&format!("PRISMA_M1 tabuleiro x={x} y={y} lado={lado_escrito}"));
            }
            let mut todos = self.botoes_normais();
            todos.extend(Self::botoes_da_confirmacao());
            for (nome, bx, by) in todos {
                if self.sem_botao != Some(nome) {
                    self.escrever(&format!("PRISMA_M1 botao={nome} x={bx} y={by}"));
                }
            }
        }

        fn botao_em(&self, x: i32, y: i32) -> Option<&'static str> {
            let candidatos = if self.confirmando { Self::botoes_da_confirmacao().to_vec() } else { self.botoes_normais() };
            candidatos
                .into_iter()
                .find(|(_, bx, by)| (x - bx).abs() <= 25 && (y - by).abs() <= 25)
                .map(|(nome, _, _)| nome)
        }

        fn casa_em(&self, x: i32, y: i32) -> Option<usize> {
            let (bx, by, lado) = self.tabuleiro_atual();
            if x < bx || y < by || x >= bx + lado || y >= by + lado {
                return None;
            }
            Some((((y - by) * 8 / lado) * 8 + (x - bx) * 8 / lado) as usize)
        }

        fn tentar_jogada(&mut self, a: usize, b: usize) {
            let indice = self.indice();
            let certa = self.dica.is_some_and(|(x, y)| (a, b) == (x, y) || (a, b) == (y, x));
            let valida = certa && !self.jogada_invalida;
            if valida && !self.pontos_param {
                self.pontos[indice] += 75;
            }
            let pontos = self.pontos[indice];
            self.escrever(&format!("PRISMA_M1 jogada valida={valida} pontos={pontos} nivel=1"));
            if !self.animacao_nunca_termina {
                self.escrever("PRISMA_M1 animacao_terminou");
            }
            self.dica = None;
            self.selecionada = None;
            self.jogadas += 1;
            if self.fecha_depois_de_jogadas.is_some_and(|n| self.jogadas >= n) {
                self.aberto = false;
            }
        }

        fn tocar(&mut self, tela_x: i32, tela_y: i32) {
            self.toques += 1;
            self.ruido();
            if self.ignora_toques || !self.aberto {
                return;
            }
            let (x, y) = (tela_x - self.deslocamento.0, tela_y - self.deslocamento.1);
            self.escrever(&format!("PRISMA_M1 toque x={x} y={y}"));
            if self.confirmando {
                match self.botao_em(x, y) {
                    Some("confirmar") => {
                        self.confirmando = false;
                        self.pontos[1] = if self.novo_jogo_nao_zera { 7 } else { 0 };
                        let pontos = self.pontos[1];
                        self.escrever(&format!("PRISMA_M1 novo_jogo modo=classico pontos={pontos}"));
                    }
                    Some("cancelar") => self.confirmando = false,
                    _ => {}
                }
                return;
            }
            if let Some(nome) = self.botao_em(x, y) {
                match nome {
                    "zen" if self.modo != ModoFalso::Zen => {
                        self.modo = ModoFalso::Zen;
                        self.selecionada = None;
                        if self.zen_perde_os_pontos {
                            self.pontos[0] = 0;
                        }
                        self.escrever_geometria();
                        let pontos = self.pontos[0];
                        self.escrever(&format!("PRISMA_M1 modo=zen pontos={pontos}"));
                    }
                    "classico" if self.modo != ModoFalso::Classico => {
                        self.modo = ModoFalso::Classico;
                        self.selecionada = None;
                        if self.classico_comeca_com_pontos {
                            self.pontos[1] = 10;
                        }
                        self.escrever_geometria();
                        let pontos = self.pontos[1];
                        self.escrever(&format!("PRISMA_M1 modo=classico pontos={pontos}"));
                    }
                    "dica" if !self.sem_dica => {
                        let a = (self.dicas_dadas % 7) * 9;
                        self.dicas_dadas += 1;
                        self.dica = Some((a, a + 1));
                        self.escrever(&format!("PRISMA_M1 dica a={a} b={}", a + 1));
                    }
                    "novo_jogo" if self.modo == ModoFalso::Classico => self.confirmando = true,
                    _ => {}
                }
                return;
            }
            if let Some(casa) = self.casa_em(x, y) {
                match self.selecionada.take() {
                    None => self.selecionada = Some(casa),
                    Some(primeira) => self.tentar_jogada(primeira, casa),
                }
            }
        }

        fn arrastar(&mut self, de: (i32, i32), ate: (i32, i32)) {
            self.arrastos += 1;
            self.relogio_no_ultimo_arrasto = self.relogio_ms;
            self.ruido();
            if self.ignora_toques || !self.aberto {
                return;
            }
            let (dx, dy) = self.deslocamento;
            self.escrever(&format!("PRISMA_M1 toque x={} y={}", de.0 - dx, de.1 - dy));
            if let (Some(a), Some(b)) = (self.casa_em(de.0 - dx, de.1 - dy), self.casa_em(ate.0 - dx, ate.1 - dy)) {
                self.tentar_jogada(a, b);
            }
        }
    }

    fn resposta_ok(texto: &str) -> Result<Resposta, String> {
        Ok(Resposta { sucesso: true, texto: texto.as_bytes().to_vec(), erro: String::new() })
    }

    fn numero(texto: &str) -> Result<i32, String> {
        texto.parse().map_err(|_| format!("número inválido no celular falso: {texto}"))
    }

    impl Celular for CelularFalso {
        fn adb(&mut self, args: &[&str]) -> Result<Resposta, String> {
            match args {
                ["shell", "getprop", "ro.build.version.release"] => resposta_ok("11\n"),
                ["shell", "getprop", "ro.product.cpu.abilist"] => resposta_ok("x86,armeabi-v7a\n"),
                ["install", "-r", _apk] => {
                    if self.instalacao_falha {
                        return Ok(Resposta {
                            sucesso: false,
                            texto: Vec::new(),
                            erro: "adb: failed to install x.apk: Failure [INSTALL_FAILED_OLDER_SDK]\n".to_string(),
                        });
                    }
                    self.instalado = true;
                    resposta_ok("Performing Streamed Install\nSuccess\n")
                }
                ["logcat", "-c"] => {
                    self.logcat.clear();
                    resposta_ok("")
                }
                ["logcat", "-d"] => Ok(Resposta { sucesso: true, texto: self.logcat.clone().into_bytes(), erro: String::new() }),
                ["shell", "monkey", "-p", pacote, "-c", "android.intent.category.LAUNCHER", "1"] => {
                    if *pacote == PACOTE && self.instalado {
                        self.aberto = !self.app_fechado;
                        if self.aberto {
                            self.escrever_geometria();
                            if !self.sem_pronto {
                                self.escrever("PRISMA_M1 pronto modo=zen");
                            }
                        }
                    }
                    resposta_ok("Events injected: 1\n")
                }
                ["shell", "pidof", pacote] => {
                    if *pacote == PACOTE && self.aberto {
                        resposta_ok("4321\n")
                    } else {
                        // Como o `adb` de verdade: sem processo, não imprime nada e sai com código 1.
                        Ok(Resposta { sucesso: false, texto: Vec::new(), erro: String::new() })
                    }
                }
                ["shell", "input", "tap", x, y] => {
                    self.tocar(numero(x)?, numero(y)?);
                    resposta_ok("")
                }
                ["shell", "input", "swipe", x1, y1, x2, y2, "200"] => {
                    self.arrastar((numero(x1)?, numero(y1)?), (numero(x2)?, numero(y2)?));
                    resposta_ok("")
                }
                ["exec-out", "screencap", "-p"] => Ok(Resposta { sucesso: true, texto: vec![0x89, b'P', b'N', b'G', 1, 2, 3], erro: String::new() }),
                outro => Err(format!("comando inesperado no celular falso: {outro:?}")),
            }
        }

        fn dormir(&mut self, ms: u64) {
            self.relogio_ms += ms;
        }

        fn reduzir_foto(&mut self, origem: &Path, destino: &Path) -> Result<bool, String> {
            if self.sem_convert {
                return Ok(false);
            }
            assert!(origem.exists(), "a foto grande devia existir antes de reduzir: {}", origem.display());
            self.fotos_reduzidas += 1;
            fs::write(destino, b"foto pequena").map_err(|e| e.to_string())?;
            Ok(true)
        }
    }

    /// Roda o teste inteiro contra o celular falso, numa pasta temporária só deste teste.
    fn rodar_com(celular: &mut CelularFalso, nome: &str) -> (Result<(), String>, Pasta) {
        let pasta = Pasta::nova(nome);
        let apks = pasta.0.join("apk");
        fs::create_dir_all(&apks).unwrap();
        fs::write(apks.join("prisma.apk"), b"apk de mentira").unwrap();
        let resultado = rodar(celular, PACOTE, &apks, &pasta.0.join("resultado"));
        (resultado, pasta)
    }

    fn erro_de(celular: &mut CelularFalso, nome: &str) -> String {
        let (resultado, _pasta) = rodar_com(celular, nome);
        resultado.expect_err("o teste devia ter falhado")
    }

    #[test]
    fn fluxo_completo_passa_com_o_celular_falso() {
        let mut celular = CelularFalso::novo();
        let (resultado, pasta) = rodar_com(&mut celular, "fluxo_completo");
        assert_eq!(resultado, Ok(()));
        assert_eq!(celular.arrastos, 5, "5 jogadas arrastando no Zen");
        // 1 calibração + 5 dicas do Zen + 1 aba + 15 no Clássico (dica e dois toques, 5 vezes)
        // + Novo jogo + Confirmar + voltar ao Zen
        assert_eq!(celular.toques, 25);
        assert_eq!(celular.pontos, [375, 0], "Zen guardou os pontos; o Novo jogo zerou o Clássico");
        assert_eq!(celular.fotos_reduzidas, 7);
        for nome in [
            "1-zen-inicio",
            "2-zen-dica",
            "3-zen-depois-da-jogada",
            "4-zen-fim",
            "5-classico",
            "6-classico-confirmando",
            "7-zen-de-volta",
        ] {
            assert!(pasta.0.join(format!("resultado/tela-{nome}.png")).exists(), "faltou a foto {nome}");
            assert!(pasta.0.join(format!("resultado/pequena-{nome}.png")).exists(), "faltou a foto pequena {nome}");
        }
        assert!(pasta.0.join("resultado/logcat.txt").exists());
        assert!(pasta.0.join("resultado/instalacao.txt").exists());
        assert!(!pasta.0.join("resultado/tela-do-erro.png").exists());
    }

    #[test]
    fn calibracao_acha_o_deslocamento_mesmo_diferente() {
        let mut celular = CelularFalso::novo();
        celular.deslocamento = (7, 40);
        let (resultado, _pasta) = rodar_com(&mut celular, "calibracao");
        assert_eq!(resultado, Ok(()));
    }

    #[test]
    fn falha_quando_o_deslocamento_e_absurdo() {
        let mut celular = CelularFalso::novo();
        celular.deslocamento = (0, 300);
        let erro = erro_de(&mut celular, "deslocamento_absurdo");
        assert!(erro.contains("deslocamento da janela fora do esperado: (0,300)"), "{erro}");
    }

    #[test]
    fn falha_quando_o_app_nao_registra_o_toque_de_calibracao() {
        let mut celular = CelularFalso::novo();
        celular.ignora_toques = true;
        let erro = erro_de(&mut celular, "sem_toque");
        assert!(erro.contains("o app não registrou o toque de calibração"), "{erro}");
    }

    #[test]
    fn falha_com_mensagem_quando_o_toque_na_dica_nao_gera_dica() {
        let mut celular = CelularFalso::novo();
        celular.sem_dica = true;
        let erro = erro_de(&mut celular, "sem_dica");
        assert!(erro.contains("o toque em 'Dica' não gerou uma dica nova"), "{erro}");
    }

    #[test]
    fn falha_quando_o_app_fecha_logo_depois_de_abrir() {
        let mut celular = CelularFalso::novo();
        celular.app_fechado = true;
        let erro = erro_de(&mut celular, "fecha_ao_abrir");
        assert!(erro.contains("o app FECHOU sozinho depois de abrir"), "{erro}");
    }

    #[test]
    fn falha_quando_o_app_fecha_durante_a_jogada() {
        let mut celular = CelularFalso::novo();
        celular.fecha_depois_de_jogadas = Some(2);
        let erro = erro_de(&mut celular, "fecha_na_jogada");
        assert!(erro.contains("o app FECHOU sozinho durante a jogada 2 do Zen"), "{erro}");
    }

    #[test]
    fn falha_quando_a_animacao_nao_termina_depois_de_exatos_30_s() {
        let mut celular = CelularFalso::novo();
        celular.animacao_nunca_termina = true;
        let erro = erro_de(&mut celular, "sem_animacao");
        assert!(erro.contains("a animação não terminou em 30 s"), "{erro}");
        let esperou = celular.relogio_ms - celular.relogio_no_ultimo_arrasto;
        assert!((30_000..=31_000).contains(&esperou), "esperou {esperou} ms: nem menos de 30 s, nem para sempre");
    }

    #[test]
    fn falha_quando_os_pontos_nao_sobem() {
        let mut celular = CelularFalso::novo();
        celular.pontos_param = true;
        let erro = erro_de(&mut celular, "pontos_param");
        assert!(erro.contains("os pontos do Zen não subiram na jogada 1 (antes 0, agora '0')"), "{erro}");
    }

    #[test]
    fn falha_quando_a_jogada_nao_e_valida() {
        let mut celular = CelularFalso::novo();
        celular.jogada_invalida = true;
        let erro = erro_de(&mut celular, "jogada_invalida");
        assert!(erro.contains("a jogada 1 do Zen não foi válida: PRISMA_M1 jogada valida=false pontos=0 nivel=1"), "{erro}");
    }

    #[test]
    fn falha_quando_falta_a_linha_pronto() {
        let mut celular = CelularFalso::novo();
        celular.sem_pronto = true;
        let erro = erro_de(&mut celular, "sem_pronto");
        assert!(erro.contains("faltou a linha 'PRISMA_M1 pronto modo=zen'"), "{erro}");
    }

    #[test]
    fn falha_quando_falta_a_posicao_de_um_botao() {
        let mut celular = CelularFalso::novo();
        celular.sem_botao = Some("embaralhar");
        let erro = erro_de(&mut celular, "sem_botao");
        assert!(erro.contains("o registro não tem a posição do botão 'embaralhar'"), "{erro}");
    }

    #[test]
    fn na_falha_o_registro_e_a_foto_ainda_saem() {
        let mut celular = CelularFalso::novo();
        celular.sem_botao = Some("embaralhar");
        let (resultado, pasta) = rodar_com(&mut celular, "falha_com_foto");
        assert!(resultado.is_err());
        let registro = fs::read_to_string(pasta.0.join("resultado/logcat.txt")).unwrap();
        assert!(registro.contains("PRISMA_M1 pronto modo=zen"), "o logcat.txt devia ter o registro até a falha");
        assert!(pasta.0.join("resultado/tela-do-erro.png").exists());
        assert!(pasta.0.join("resultado/pequena-do-erro.png").exists());
        assert_eq!(celular.fotos_reduzidas, 1, "só a foto do erro");
    }

    #[test]
    fn falha_quando_falta_a_posicao_do_tabuleiro() {
        let mut celular = CelularFalso::novo();
        celular.sem_tabuleiro = true;
        let erro = erro_de(&mut celular, "sem_tabuleiro");
        assert!(erro.contains("o registro não tem a posição do tabuleiro"), "{erro}");
    }

    #[test]
    fn falha_quando_o_lado_do_tabuleiro_no_registro_e_zero() {
        let mut celular = CelularFalso::novo();
        celular.lado_zero_no_registro = true;
        let erro = erro_de(&mut celular, "lado_zero");
        assert!(erro.contains("o registro não tem a posição do tabuleiro"), "{erro}");
    }

    #[test]
    fn falha_quando_o_classico_nao_comeca_com_zero_pontos() {
        let mut celular = CelularFalso::novo();
        celular.classico_comeca_com_pontos = true;
        let erro = erro_de(&mut celular, "classico_com_pontos");
        assert!(erro.contains("faltou 'PRISMA_M1 modo=classico pontos=0' depois de tocar na aba Clássico"), "{erro}");
    }

    #[test]
    fn falha_quando_o_novo_jogo_nao_zera_os_pontos() {
        let mut celular = CelularFalso::novo();
        celular.novo_jogo_nao_zera = true;
        let erro = erro_de(&mut celular, "novo_jogo_nao_zera");
        assert!(erro.contains("faltou 'PRISMA_M1 novo_jogo modo=classico pontos=0' depois de confirmar"), "{erro}");
    }

    #[test]
    fn falha_quando_o_zen_perde_os_pontos_ao_voltar() {
        let mut celular = CelularFalso::novo();
        celular.zen_perde_os_pontos = true;
        let erro = erro_de(&mut celular, "zen_perde_pontos");
        assert!(erro.contains("ao voltar para o Zen os pontos não eram 375: 'PRISMA_M1 modo=zen pontos=0'"), "{erro}");
    }

    #[test]
    fn foto_e_pulada_sem_imagemagick_e_o_teste_continua() {
        let mut celular = CelularFalso::novo();
        celular.sem_convert = true;
        let (resultado, pasta) = rodar_com(&mut celular, "sem_convert");
        assert_eq!(resultado, Ok(()));
        assert_eq!(celular.fotos_reduzidas, 0);
        assert!(pasta.0.join("resultado/tela-1-zen-inicio.png").exists(), "a foto grande ainda é guardada");
        assert!(!pasta.0.join("resultado/pequena-1-zen-inicio.png").exists());
    }

    #[test]
    fn falha_quando_a_instalacao_falha() {
        let mut celular = CelularFalso::novo();
        celular.instalacao_falha = true;
        let erro = erro_de(&mut celular, "instalacao");
        assert!(erro.contains("A instalação FALHOU"), "{erro}");
        assert!(erro.contains("INSTALL_FAILED_OLDER_SDK"), "o motivo do adb tem de aparecer: {erro}");
    }

    #[test]
    fn falha_quando_nao_ha_apk_na_pasta() {
        let pasta = Pasta::nova("sem_apk");
        let vazia = pasta.0.join("apk");
        fs::create_dir_all(&vazia).unwrap();
        fs::write(vazia.join("leia-me.txt"), b"nao e apk").unwrap();
        let mut celular = CelularFalso::novo();
        let erro = rodar(&mut celular, PACOTE, &vazia, &pasta.0.join("resultado")).unwrap_err();
        assert!(erro.contains("não achei nenhum .apk"), "{erro}");
    }

    #[test]
    fn usa_o_primeiro_apk_em_ordem_alfabetica() {
        let pasta = Pasta::nova("ordem_apk");
        for nome in ["b.apk", "a.apk", "c.txt"] {
            fs::write(pasta.0.join(nome), b"x").unwrap();
        }
        assert_eq!(achar_apk(&pasta.0).unwrap(), pasta.0.join("a.apk"));
    }

    #[test]
    fn o_pacote_vem_da_linha_package_name_do_cargo_toml() {
        assert_eq!(pacote_do_cargo_toml("package_name = \"com.exemplo.jogo\"\n"), Some("com.exemplo.jogo".to_string()));
        // comentário, outras chaves e linhas com recuo não valem (o shell usava `^package_name = "..."`)
        let texto = "# o package_name = \"nao.vale\"\n[package.metadata.android]\n  package_name = \"nem.este\"\npackage_name = \"com.vale\"\n";
        assert_eq!(pacote_do_cargo_toml(texto), Some("com.vale".to_string()));
        assert_eq!(pacote_do_cargo_toml("name = \"x\"\n"), None);
        assert_eq!(pacote_do_cargo_toml("package_name = \"\"\n"), None);
        assert_eq!(pacote_do_cargo_toml(""), None);
    }

    #[test]
    fn le_o_pacote_do_game_cargo_toml_de_verdade() {
        // Os testes rodam de dentro de `tools/`; o `game/` fica ao lado.
        let pacote = ler_pacote(Path::new("../game/Cargo.toml")).unwrap();
        assert!(pacote.contains('.') && !pacote.contains('"'), "pacote estranho: {pacote}");
        let erro = ler_pacote(Path::new("../game/nao-existe.toml")).unwrap_err();
        assert!(erro.contains("nao-existe.toml") && erro.contains("raiz do projeto"), "{erro}");
    }

    #[test]
    fn resumo_pega_as_linhas_que_ajudam_a_achar_erro_sem_ligar_para_maiuscula() {
        let logcat = "I Outro: nada\nE AndroidRuntime: FATAL EXCEPTION\nI SAPP : PRISMA_M1 toque x=1 y=2\nthread panicked at x\nF libc: Fatal signal 11 (SIGSEGV)\nD  DEBUG  : backtrace\nI miniquad: algo\ndlopen failed\nI Banal: de novo nada\n";
        let texto = resumo(logcat);
        assert!(texto.starts_with("===== logcat (resumo) =====\n"));
        assert!(texto.ends_with("\n===== fim do resumo ====="));
        for deve in ["FATAL EXCEPTION", "PRISMA_M1 toque", "panicked", "SIGSEGV", "backtrace", "miniquad", "dlopen"] {
            assert!(texto.contains(deve), "o resumo devia ter '{deve}': {texto}");
        }
        assert!(!texto.contains("nada"), "linhas banais ficam de fora: {texto}");
        // Sem maiúsculas iguais, também vale (grep -i):
        assert!(resumo("i sapp: prisma_m1 pronto").contains("prisma_m1 pronto"));
    }

    #[test]
    fn resumo_mostra_no_maximo_200_linhas() {
        let logcat: String = (0..500).map(|i| format!("PRISMA_M1 toque x={i} y=0\n")).collect();
        let texto = resumo(&logcat);
        let uteis = texto.lines().filter(|l| l.contains("PRISMA_M1")).count();
        assert_eq!(uteis, 200);
        assert!(texto.contains("x=199 ") && !texto.contains("x=200 "), "vale o começo do registro, como o `head -200`");
    }

    #[test]
    fn bloco_da_foto_tem_marcadores_e_base64_numa_linha_so() {
        let bloco = bloco_da_foto("1-zen-inicio", b"foobar");
        assert_eq!(bloco, "TELA_BASE64_INICIO 1-zen-inicio\nZm9vYmFy\nTELA_BASE64_FIM");
    }

    #[cfg(unix)]
    #[test]
    fn adb_real_devolve_saida_erro_e_codigo_de_saida() {
        let mut adb = AdbReal::com_programa("sh", Duration::from_secs(10));
        let r = adb.adb(&["-c", "echo oi; echo erro 1>&2; exit 3"]).unwrap();
        assert!(!r.sucesso);
        assert_eq!(r.texto, b"oi\n");
        assert_eq!(r.erro, "erro\n");
        let r = adb.adb(&["-c", "printf abc"]).unwrap();
        assert!(r.sucesso);
        assert_eq!(r.texto, b"abc");
    }

    #[cfg(unix)]
    #[test]
    fn adb_que_passa_do_limite_vira_erro_sem_esperar_o_tempo_todo() {
        let mut adb = AdbReal::com_programa("sleep", Duration::from_millis(200));
        let inicio = Instant::now();
        let erro = adb.adb(&["5"]).unwrap_err();
        assert!(inicio.elapsed() < Duration::from_secs(3), "devia desistir logo, não esperar os 5 s");
        assert!(erro.contains("passou de") && erro.contains("interrompido"), "{erro}");
    }

    #[cfg(unix)]
    #[test]
    fn adb_real_nao_trava_quando_um_filho_segura_a_saida() {
        // O servidor do `adb` (que o primeiro comando liga) herda a saída; com pipes isso travaria.
        let mut adb = AdbReal::com_programa("sh", Duration::from_secs(10));
        let inicio = Instant::now();
        let r = adb.adb(&["-c", "sleep 3 & echo oi"]).unwrap();
        assert_eq!(r.texto, b"oi\n");
        assert!(inicio.elapsed() < Duration::from_secs(2), "devia voltar assim que o `sh` terminou");
    }

    #[test]
    fn adb_real_com_programa_que_nao_existe_da_erro_claro() {
        let mut adb = AdbReal::com_programa("programa-que-nao-existe-xyz", Duration::from_secs(1));
        let erro = adb.adb(&["version"]).unwrap_err();
        assert!(erro.contains("não consegui rodar 'programa-que-nao-existe-xyz'"), "{erro}");
    }

    #[test]
    fn reduzir_foto_sem_o_convert_da_ok_false_em_vez_de_erro() {
        let mut adb = AdbReal::com_programa("adb", Duration::from_secs(1));
        adb.convert = "programa-que-nao-existe-xyz".to_string();
        let pasta = Pasta::nova("sem_convert_real");
        assert_eq!(adb.reduzir_foto(&pasta.0.join("a.png"), &pasta.0.join("b.png")), Ok(false));
    }

    #[test]
    fn reduzir_foto_de_verdade_quando_o_convert_existe() {
        let mut adb = AdbReal::com_programa("adb", Duration::from_secs(1));
        let pasta = Pasta::nova("convert_real");
        let destino = pasta.0.join("pequena.png");
        match adb.reduzir_foto(Path::new("../game/assets/gem-0.png"), &destino) {
            Ok(false) => {} // sem ImageMagick neste computador: nada a conferir
            Ok(true) => assert!(fs::metadata(&destino).unwrap().len() > 0),
            Err(e) => panic!("o convert falhou: {e}"),
        }
    }
}
