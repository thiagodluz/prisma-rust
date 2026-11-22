//! `prisma_tools audio`: prepara as músicas e os ambientes do jogo.
//!
//! Os sons não entram no git (a spec pede isso). Este comando os recria do zero:
//! 1. baixa o HTML da release v1.4.2 do Prisma em JS, que traz os 9 áudios embutidos;
//! 2. tira cada áudio de dentro dele e confere o SHA-256 (o mesmo do `download-audio.mjs` do JS);
//! 3. grava as músicas como vieram e prepara os ambientes com o `ffmpeg`.
//!
//! Por que preparar os ambientes? O Macroquad carrega o arquivo inteiro descompactado na memória
//! (uns 21 MB por minuto). Os ambientes originais chegam a 12 minutos, o que derrubaria o app.
//! Então cada ambiente passa a 44 100 Hz e, se for longo, vira um "laço" de exatamente 60 s:
//! um trecho do meio com 3 s de transição suave entre o fim e o começo, para repetir sem estalo.

use crate::base64::decodificar;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// De onde vêm os áudios (o mesmo endereço do `scripts/download-audio.mjs` do Prisma em JS).
pub const ENDERECO: &str = "https://github.com/thiagodluz/Prisma/releases/download/v1.4.2/Prisma-jogar-offline.html";

/// Acima disto (em segundos) o ambiente vira laço; até aqui só é reconvertido.
const LIMITE_DO_LACO_S: f64 = 63.0;
/// Duração do laço e da transição suave (segundos).
const DURACAO_DO_LACO_S: u32 = 60;
const TRANSICAO_S: u32 = 3;
/// A conferência aceita um ambiente de até isto (margem sobre os 63 s do limite do laço).
const LIMITE_DA_CONFERENCIA_S: f64 = 63.5;

/// Música (arquivo como veio) ou ambiente (reconvertido).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    Musica,
    Ambiente,
}

/// Um dos 9 áudios: onde está no Prisma em JS, o SHA-256 do original e o nome do arquivo
/// que o jogo vai ler (dentro da pasta de sons).
pub struct Origem {
    pub caminho_js: &'static str,
    pub sha256: &'static str,
    pub tipo: Tipo,
    pub saida: &'static str,
}

/// Os 9 áudios. Os SHA-256 são os do `download-audio.mjs` (o teste `tabela_igual_ao_script_do_js`
/// confere isso quando o Prisma em JS está na máquina).
pub const ORIGENS: [Origem; 9] = [
    Origem { caminho_js: "audio/music/magic-puzzle.ogg", sha256: "f94f4a6f895f22669dd5ced4061f932e1b5335e40e36d1e94fe7cfc222a4c2b8", tipo: Tipo::Musica, saida: "musica/magic-puzzle.ogg" },
    Origem { caminho_js: "audio/music/cozy-puzzle.ogg", sha256: "56551121c11db983564106fbc199ed647a555e3e798501f3c6847c29278ebe37", tipo: Tipo::Musica, saida: "musica/cozy-puzzle.ogg" },
    Origem { caminho_js: "audio/music/space-city.ogg", sha256: "6ecb8408cd84b1031425e8f1b941d7a89ab8a850cea2833ac2e005f76544d7a7", tipo: Tipo::Musica, saida: "musica/space-city.ogg" },
    Origem { caminho_js: "audio/ambience/stream.mp3", sha256: "3bb152c08d3646faf3f09c2bcfd0f17a31ad962699258b55c56ccf4735fee7d8", tipo: Tipo::Ambiente, saida: "ambiente/stream.ogg" },
    Origem { caminho_js: "audio/ambience/rain-soft.ogg", sha256: "d80e1d2bdfb2d9956bea305f1529fc0f5ba84417f5e8dbb4b6edc753d1a1a2e4", tipo: Tipo::Ambiente, saida: "ambiente/rain-soft.ogg" },
    Origem { caminho_js: "audio/ambience/forest-cicadas.ogg", sha256: "64b1c8391fc8ecb091f03ad485982c0a051fece12e95534a06ffd85ca309f8e2", tipo: Tipo::Ambiente, saida: "ambiente/forest-cicadas.ogg" },
    Origem { caminho_js: "audio/ambience/rainforest.mp3", sha256: "3fb52816724ddab1b4af2c6c2106965595e228af38133b4eef4a3a6552980e10", tipo: Tipo::Ambiente, saida: "ambiente/rainforest.ogg" },
    Origem { caminho_js: "audio/ambience/rain-thunder.ogg", sha256: "c2955d257e1e8a9decd115869f28b94f09e864a1098e81f3e05c54d31e933aef", tipo: Tipo::Ambiente, saida: "ambiente/rain-thunder.ogg" },
    Origem { caminho_js: "audio/ambience/soft-noise.ogg", sha256: "1ce3d836d0b9299058ea711d76b15cc244256ff5accbf3b894c1de134b50b011", tipo: Tipo::Ambiente, saida: "ambiente/soft-noise.ogg" },
];

// ---- Partes puras (só texto e números: dá para testar sem rede e sem ffmpeg) ---------------

/// Tira de dentro do HTML todos os áudios embutidos como `data:audio/ogg;base64,...` ou
/// `data:audio/mpeg;base64,...` e devolve os bytes de cada um. Pedaço com base64 estragado ou
/// vazio é ignorado.
pub fn extrair_audios(html: &str) -> Vec<Vec<u8>> {
    const MARCA: &str = "data:audio/";
    let mut achados = Vec::new();
    let mut resto = html;
    while let Some(posicao) = resto.find(MARCA) {
        resto = &resto[posicao + MARCA.len()..];
        let Some(dados) = resto.strip_prefix("ogg;base64,").or_else(|| resto.strip_prefix("mpeg;base64,")) else {
            continue;
        };
        // O base64 vai até a primeira letra que não pertence a ele (a aspa que fecha o texto).
        let fim = dados.find(|c: char| !(c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')).unwrap_or(dados.len());
        if let Ok(bytes) = decodificar(&dados[..fim]) {
            if !bytes.is_empty() {
                achados.push(bytes);
            }
        }
        resto = &dados[fim..];
    }
    achados
}

/// O SHA-256 (64 letras minúsculas) dos bytes.
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Confere se os bytes têm o SHA-256 esperado. O erro nomeia o arquivo.
pub fn conferir_sha256(bytes: &[u8], esperado: &str, nome: &str) -> Result<(), String> {
    let obtido = sha256_hex(bytes);
    if obtido == esperado {
        Ok(())
    } else {
        Err(format!("{nome}: o SHA-256 não bate (esperado {esperado}, obtido {obtido})"))
    }
}

/// O que fazer com um ambiente.
#[derive(Clone, Debug, PartialEq)]
pub enum Plano {
    /// Só reconverter (44 100 Hz, OGG Vorbis).
    Reencodar,
    /// Cortar um laço de 60 s começando em `inicio_s` segundos do original.
    Laco { inicio_s: f64 },
}

/// Ambiente de até 63 s só é reconvertido; mais longo vira laço de 60 s tirado do meio.
pub fn plano_do_ambiente(duracao_s: f64) -> Plano {
    if duracao_s.is_finite() && duracao_s > LIMITE_DO_LACO_S {
        Plano::Laco { inicio_s: (duracao_s - LIMITE_DO_LACO_S) / 2.0 }
    } else {
        Plano::Reencodar
    }
}

/// O filtro do laço: o começo (entrada 0) entra aos poucos, o "rabo" (entrada 1, os 3 s depois
/// do fim do laço) sai aos poucos, os dois se somam (`amix`) e o corpo (entrada 2) vem depois.
/// Assim o fim do laço emenda no começo sem estalo.
const FILTRO_DO_LACO: &str = "[0:a]afade=t=in:d=3:curve=qsin[h];[1:a]afade=t=out:d=3:curve=qsin[t];\
[h][t]amix=inputs=2:duration=longest:normalize=0[x];[x][2:a]concat=n=2:v=0:a=1[o]";

/// Os argumentos do `ffmpeg` para o plano.
pub fn argumentos_ffmpeg(entrada: &Path, saida: &Path, plano: &Plano) -> Vec<String> {
    let entrada = entrada.display().to_string();
    let mut argumentos: Vec<String> = ["-v", "error", "-y"].iter().map(|a| a.to_string()).collect();
    match plano {
        Plano::Reencodar => argumentos.extend(["-i".to_string(), entrada]),
        Plano::Laco { inicio_s } => {
            let corpo_s = DURACAO_DO_LACO_S - TRANSICAO_S;
            let pedacos = [
                (*inicio_s, TRANSICAO_S),
                (inicio_s + f64::from(DURACAO_DO_LACO_S), TRANSICAO_S),
                (inicio_s + f64::from(TRANSICAO_S), corpo_s),
            ];
            for (inicio, duracao) in pedacos {
                argumentos.extend([
                    "-ss".to_string(),
                    format!("{inicio:.3}"),
                    "-t".to_string(),
                    duracao.to_string(),
                    "-i".to_string(),
                    entrada.clone(),
                ]);
            }
            argumentos.extend(["-filter_complex", FILTRO_DO_LACO, "-map", "[o]"].iter().map(|a| a.to_string()));
        }
    }
    argumentos.extend(["-ar", "44100", "-c:a", "libvorbis", "-q:a", "3"].iter().map(|a| a.to_string()));
    argumentos.push(saida.display().to_string());
    argumentos
}

/// Confere a resposta do `ffprobe` (linhas `chave=valor`): o áudio tem de ser Vorbis; o ambiente
/// também tem de ter 44 100 Hz e no máximo 63,5 s.
pub fn conferir_ffprobe(texto: &str, tipo: Tipo) -> Result<(), String> {
    let valor = |chave: &str| texto.lines().find_map(|linha| linha.strip_prefix(chave).and_then(|resto| resto.strip_prefix('=')));
    let codec = valor("codec_name").ok_or("a sonda não trouxe `codec_name`")?;
    if codec != "vorbis" {
        return Err(format!("o codec é `{codec}`, esperava vorbis"));
    }
    if tipo == Tipo::Musica {
        return Ok(());
    }
    let taxa = valor("sample_rate").ok_or("a sonda não trouxe `sample_rate`")?;
    if taxa.trim() != "44100" {
        return Err(format!("a taxa é {taxa} Hz, esperava 44100"));
    }
    let duracao: f64 = valor("duration")
        .ok_or("a sonda não trouxe `duration`")?
        .trim()
        .parse()
        .map_err(|_| "a `duration` da sonda não é um número".to_string())?;
    if duracao > LIMITE_DA_CONFERENCIA_S {
        return Err(format!("a duração é {duracao:.1} s, passa do limite de {LIMITE_DA_CONFERENCIA_S} s"));
    }
    Ok(())
}

// ---- Partes que mexem no computador (programas externos e arquivos) ------------------------

/// Roda um programa e devolve o que ele escreveu na saída normal. Se o programa não existir ou
/// falhar, o erro diz qual foi e o que ele reclamou.
pub fn executar(programa: &str, argumentos: &[&str]) -> Result<String, String> {
    let saida = Command::new(programa).args(argumentos).output().map_err(|erro| {
        if erro.kind() == std::io::ErrorKind::NotFound {
            format!("o programa `{programa}` não foi encontrado: instale-o e tente de novo (`{programa}` precisa estar no PATH)")
        } else {
            format!("não consegui rodar `{programa}`: {erro}")
        }
    })?;
    if !saida.status.success() {
        return Err(format!("`{programa}` falhou ({}): {}", saida.status, String::from_utf8_lossy(&saida.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&saida.stdout).into_owned())
}

/// A duração (segundos) de um arquivo de áudio, pelo `ffprobe`.
pub fn duracao_do_arquivo(arquivo: &Path) -> Result<f64, String> {
    let arquivo = arquivo.display().to_string();
    let texto = executar("ffprobe", &["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", &arquivo])?;
    texto.trim().parse().map_err(|_| format!("o ffprobe devolveu uma duração estranha para {arquivo}: {texto:?}"))
}

/// Pergunta ao `ffprobe` o codec, a taxa e a duração do arquivo (linhas `chave=valor`).
fn sondar(arquivo: &Path) -> Result<String, String> {
    let nome = arquivo.display().to_string();
    executar(
        "ffprobe",
        &["-v", "error", "-show_entries", "stream=codec_name,sample_rate:format=duration", "-of", "default=noprint_wrappers=1", &nome],
    )
}

/// Confere um arquivo já gravado.
fn conferir_arquivo(arquivo: &Path, tipo: Tipo) -> Result<(), String> {
    conferir_ffprobe(&sondar(arquivo)?, tipo).map_err(|erro| format!("{}: {erro}", arquivo.display()))
}

/// Todos os 9 arquivos já existem em `destino` e passam na conferência?
fn todos_prontos(destino: &Path, conferidor: &dyn Fn(&Path, Tipo) -> Result<(), String>) -> bool {
    ORIGENS.iter().all(|origem| {
        let arquivo = destino.join(origem.saida);
        arquivo.is_file() && conferidor(&arquivo, origem.tipo).is_ok()
    })
}

/// Os nomes dos 9 arquivos, na ordem da tabela.
fn nomes_de_saida() -> Vec<String> {
    ORIGENS.iter().map(|origem| origem.saida.to_string()).collect()
}

/// Baixa o HTML da release (com o `curl`, que já vem no sistema).
fn baixar_html() -> Result<String, String> {
    let arquivo = std::env::temp_dir().join(format!("prisma-audio-{}.html", std::process::id()));
    let nome = arquivo.display().to_string();
    let resultado = executar("curl", &["-fsSL", "--retry", "3", "-o", &nome, ENDERECO]);
    let lido = resultado.and_then(|_| std::fs::read(&arquivo).map_err(|erro| format!("não consegui ler o HTML baixado: {erro}")));
    let _ = std::fs::remove_file(&arquivo);
    // O HTML é texto, mas lido como "texto com defeitos permitidos" para um byte estranho não derrubar tudo.
    lido.map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// Grava `bytes` em `arquivo` por um arquivo temporário e uma troca de nome, para nunca ficar
/// um arquivo pela metade com o nome final.
fn gravar_por_etapas(arquivo: &Path, bytes: &[u8]) -> Result<(), String> {
    let parcial = arquivo.with_extension("parcial");
    std::fs::write(&parcial, bytes).map_err(|erro| format!("não consegui gravar {}: {erro}", parcial.display()))?;
    std::fs::rename(&parcial, arquivo).map_err(|erro| format!("não consegui renomear {}: {erro}", parcial.display()))
}

/// Prepara um ambiente: grava o original numa pasta temporária, converte com o `ffmpeg` para um
/// arquivo parcial, confere e só então troca pelo nome final.
fn preparar_ambiente(origem: &Origem, bytes: &[u8], destino: &Path, temporaria: &Path) -> Result<(), String> {
    let extensao = Path::new(origem.caminho_js).extension().and_then(|e| e.to_str()).unwrap_or("ogg");
    let original = temporaria.join(format!("original.{extensao}"));
    std::fs::write(&original, bytes).map_err(|erro| format!("não consegui gravar {}: {erro}", original.display()))?;
    let plano = plano_do_ambiente(duracao_do_arquivo(&original)?);

    let final_ = destino.join(origem.saida);
    let parcial = final_.with_extension("parcial.ogg");
    let argumentos = argumentos_ffmpeg(&original, &parcial, &plano);
    let resultado = executar("ffmpeg", &argumentos.iter().map(String::as_str).collect::<Vec<_>>())
        .and_then(|_| conferir_arquivo(&parcial, Tipo::Ambiente))
        .and_then(|()| std::fs::rename(&parcial, &final_).map_err(|erro| format!("não consegui renomear {}: {erro}", parcial.display())));
    if resultado.is_err() {
        let _ = std::fs::remove_file(&parcial);
    }
    resultado
}

/// Prepara o que falta em `destino` a partir do texto do HTML. Se algum dos áudios que faltam não
/// estiver no HTML (ou estiver com outro conteúdo), recusa **antes** de gravar qualquer coisa.
/// Devolve os nomes dos 9 arquivos.
pub fn preparar_com(html: &str, destino: &Path) -> Result<Vec<String>, String> {
    let faltando: Vec<&Origem> = ORIGENS
        .iter()
        .filter(|origem| {
            let arquivo = destino.join(origem.saida);
            !(arquivo.is_file() && conferir_arquivo(&arquivo, origem.tipo).is_ok())
        })
        .collect();
    if faltando.is_empty() {
        return Ok(nomes_de_saida());
    }

    // Guarda só os áudios do HTML que a gente precisa, achados pelo SHA-256 do conteúdo.
    let mut achados: HashMap<String, Vec<u8>> = HashMap::new();
    for bytes in extrair_audios(html) {
        let hash = sha256_hex(&bytes);
        if faltando.iter().any(|origem| origem.sha256 == hash) {
            achados.insert(hash, bytes);
        }
    }
    let ausentes: Vec<&str> = faltando.iter().filter(|origem| !achados.contains_key(origem.sha256)).map(|origem| origem.saida).collect();
    if !ausentes.is_empty() {
        return Err(format!("Áudios ausentes ou divergentes: {}", ausentes.join(", ")));
    }

    let temporaria = destino.join(format!(".preparo-{}", std::process::id()));
    std::fs::create_dir_all(&temporaria).map_err(|erro| format!("não consegui criar {}: {erro}", temporaria.display()))?;
    let resultado = faltando.iter().try_for_each(|origem| {
        let bytes = &achados[origem.sha256];
        conferir_sha256(bytes, origem.sha256, origem.saida)?;
        let arquivo = destino.join(origem.saida);
        if let Some(pasta) = arquivo.parent() {
            std::fs::create_dir_all(pasta).map_err(|erro| format!("não consegui criar {}: {erro}", pasta.display()))?;
        }
        match origem.tipo {
            Tipo::Musica => {
                gravar_por_etapas(&arquivo, bytes)?;
                conferir_arquivo(&arquivo, Tipo::Musica)
            }
            Tipo::Ambiente => preparar_ambiente(origem, bytes, destino, &temporaria),
        }
    });
    let _ = std::fs::remove_dir_all(&temporaria);
    resultado?;
    Ok(nomes_de_saida())
}

/// O comando: se os 9 arquivos já estão prontos e passam na conferência, não faz nada; senão
/// baixa o HTML e prepara o que faltar. Devolve os nomes dos 9 arquivos.
pub fn preparar(destino: &Path) -> Result<Vec<String>, String> {
    if todos_prontos(destino, &conferir_arquivo) {
        return Ok(nomes_de_saida());
    }
    let html = baixar_html()?;
    preparar_com(&html, destino)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base64::codificar;
    use std::path::Path;

    /// Um "HTML" de mentira com áudios embutidos como no arquivo da release do Prisma em JS.
    fn html_com(partes: &[(&str, &[u8])]) -> String {
        let mut html = String::from("<html><body>lixo ");
        for (tipo, bytes) in partes {
            html.push_str(&format!("<audio src=\"data:audio/{tipo};base64,{}\"></audio> mais lixo ", codificar(bytes)));
        }
        html.push_str("</body></html>");
        html
    }

    #[test]
    fn extrair_audios_acha_as_duas_formas() {
        let html = html_com(&[("ogg", b"OggS um"), ("mpeg", b"ID3 dois")]);
        let achados = extrair_audios(&html);
        assert_eq!(achados, vec![b"OggS um".to_vec(), b"ID3 dois".to_vec()]);
    }

    #[test]
    fn extrair_audios_ignora_lixo_e_base64_estragado() {
        let html = format!(
            "<a src=\"data:image/png;base64,{}\"> <b src=\"data:audio/ogg;base64,@@@@\"> <c src=\"data:audio/wav;base64,{}\"> {}",
            codificar(b"png"),
            codificar(b"wav"),
            html_com(&[("ogg", b"bom")])
        );
        assert_eq!(extrair_audios(&html), vec![b"bom".to_vec()]);
        assert!(extrair_audios("").is_empty());
        assert!(extrair_audios("data:audio/ogg;base64,").is_empty());
    }

    #[test]
    fn conferir_sha256_aceita_o_certo_e_recusa_o_diferente() {
        // SHA-256 de "abc" (vetor de teste do próprio padrão).
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(conferir_sha256(b"abc", abc, "cozy-puzzle.ogg"), Ok(()));
        let erro = conferir_sha256(b"abd", abc, "cozy-puzzle.ogg").unwrap_err();
        assert!(erro.contains("cozy-puzzle.ogg"), "o erro precisa nomear o arquivo: {erro}");
        assert!(erro.contains(abc), "o erro precisa mostrar o esperado: {erro}");
    }

    #[test]
    fn tabela_de_origens_tem_9_itens_e_nomes_unicos() {
        assert_eq!(ORIGENS.len(), 9);
        assert_eq!(ORIGENS.iter().filter(|o| o.tipo == Tipo::Musica).count(), 3);
        assert_eq!(ORIGENS.iter().filter(|o| o.tipo == Tipo::Ambiente).count(), 6);
        let mut saidas: Vec<&str> = ORIGENS.iter().map(|o| o.saida).collect();
        let mut hashes: Vec<&str> = ORIGENS.iter().map(|o| o.sha256).collect();
        saidas.sort_unstable();
        saidas.dedup();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(saidas.len(), 9, "saídas repetidas");
        assert_eq!(hashes.len(), 9, "SHA-256 repetidos");
        for origem in &ORIGENS {
            assert_eq!(origem.sha256.len(), 64, "{}", origem.caminho_js);
            assert!(origem.sha256.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()), "{}", origem.caminho_js);
            let pasta = if origem.tipo == Tipo::Musica { "musica/" } else { "ambiente/" };
            assert!(origem.saida.starts_with(pasta) && origem.saida.ends_with(".ogg"), "{}", origem.saida);
        }
    }

    #[test]
    fn tabela_igual_ao_script_do_js() {
        // Só confere se o Prisma em JS estiver aqui ao lado; sem ele, o teste não tem o que comparar.
        let Ok(script) = std::fs::read_to_string("/home/claude/thiagodluz/prisma/scripts/download-audio.mjs") else {
            return;
        };
        for origem in &ORIGENS {
            let par = format!("'{}': '{}'", origem.caminho_js, origem.sha256);
            assert!(script.contains(&par), "o script do JS não tem este par: {par}");
        }
        assert!(script.contains(ENDERECO), "o endereço do script do JS é outro");
    }

    #[test]
    fn plano_do_ambiente_curto_so_reencoda() {
        assert_eq!(plano_do_ambiente(20.0), Plano::Reencodar);
        assert_eq!(plano_do_ambiente(42.192), Plano::Reencodar);
        // Até 63 s inclusive não vira laço; um pouco mais, sim.
        assert_eq!(plano_do_ambiente(63.0), Plano::Reencodar);
        assert!(matches!(plano_do_ambiente(63.1), Plano::Laco { .. }));
    }

    #[test]
    fn plano_do_ambiente_longo_vira_laco_de_60s() {
        let Plano::Laco { inicio_s } = plano_do_ambiente(732.0) else { panic!("devia ser laço") };
        // O trecho de 63 s fica no meio da gravação.
        assert!((inicio_s - (732.0 - 63.0) / 2.0).abs() < 1e-9, "início {inicio_s}");
        let argumentos = argumentos_ffmpeg(Path::new("entrada.mp3"), Path::new("saida.ogg"), &Plano::Laco { inicio_s: 100.0 });
        let texto = argumentos.join(" ");
        // Três entradas: o começo (3 s em +0), o "rabo" (3 s em +60) e o corpo (57 s em +3).
        assert_eq!(argumentos.iter().filter(|a| *a == "-i").count(), 3, "{texto}");
        assert!(texto.contains("-ss 100.000 -t 3 -i entrada.mp3"), "{texto}");
        assert!(texto.contains("-ss 160.000 -t 3 -i entrada.mp3"), "{texto}");
        assert!(texto.contains("-ss 103.000 -t 57 -i entrada.mp3"), "{texto}");
        // As duas pontas se misturam com uma transição suave e o corpo vem depois.
        assert!(texto.contains("afade=t=in:d=3:curve=qsin"), "{texto}");
        assert!(texto.contains("afade=t=out:d=3:curve=qsin"), "{texto}");
        assert!(texto.contains("amix=inputs=2:duration=longest:normalize=0"), "{texto}");
        assert!(texto.contains("concat=n=2:v=0:a=1"), "{texto}");
        assert!(texto.ends_with("-ar 44100 -c:a libvorbis -q:a 3 saida.ogg"), "{texto}");
    }

    #[test]
    fn argumentos_do_reencode_simples() {
        let argumentos = argumentos_ffmpeg(Path::new("a.ogg"), Path::new("b.ogg"), &Plano::Reencodar);
        assert_eq!(
            argumentos.join(" "),
            "-v error -y -i a.ogg -ar 44100 -c:a libvorbis -q:a 3 b.ogg"
        );
    }

    #[test]
    fn conferir_ffprobe_aceita_e_recusa() {
        let bom = "codec_name=vorbis\nsample_rate=44100\nduration=60.000000\n";
        assert_eq!(conferir_ffprobe(bom, Tipo::Ambiente), Ok(()));
        assert_eq!(conferir_ffprobe(bom, Tipo::Musica), Ok(()));
        // Música é copiada como veio: qualquer taxa serve, desde que seja Vorbis.
        assert_eq!(conferir_ffprobe("codec_name=vorbis\nsample_rate=48000\nduration=120.0\n", Tipo::Musica), Ok(()));
        for (texto, tipo, parte) in [
            ("codec_name=mp3\nsample_rate=44100\nduration=60.0\n", Tipo::Ambiente, "vorbis"),
            ("codec_name=vorbis\nsample_rate=48000\nduration=60.0\n", Tipo::Ambiente, "44100"),
            ("codec_name=vorbis\nsample_rate=44100\nduration=120.0\n", Tipo::Ambiente, "duração"),
            ("codec_name=vorbis\nsample_rate=44100\n", Tipo::Ambiente, "duration"),
            ("", Tipo::Musica, "codec_name"),
            ("codec_name=mp3\n", Tipo::Musica, "vorbis"),
        ] {
            let erro = conferir_ffprobe(texto, tipo).unwrap_err();
            assert!(erro.contains(parte), "{texto:?} → {erro}");
        }
    }

    #[test]
    fn executar_diz_quando_o_programa_nao_existe() {
        let erro = executar("programa-que-nao-existe-prisma", &[]).unwrap_err();
        assert!(erro.contains("programa-que-nao-existe-prisma"), "{erro}");
        assert!(erro.contains("instal"), "o erro precisa dizer para instalar: {erro}");
    }

    /// Uma pasta nova só deste teste.
    fn pasta_nova(nome: &str) -> std::path::PathBuf {
        let pasta = std::env::temp_dir().join(format!("prisma-audio-teste-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).unwrap();
        pasta
    }

    #[test]
    fn html_sem_os_audios_recusa_e_nao_grava_nada() {
        let destino = pasta_nova("vazio");
        let erro = preparar_com(&html_com(&[("ogg", b"nao e nenhum dos 9")]), &destino).unwrap_err();
        assert!(erro.contains("ausentes ou divergentes"), "{erro}");
        // Os 9 estão faltando e todos aparecem no erro.
        for origem in &ORIGENS {
            assert!(erro.contains(origem.saida), "o erro não cita {}: {erro}", origem.saida);
        }
        let gravados: Vec<_> = std::fs::read_dir(&destino).unwrap().collect();
        assert!(gravados.is_empty(), "não devia gravar nada: {gravados:?}");
        let _ = std::fs::remove_dir_all(&destino);
    }

    #[test]
    fn ja_preparado_e_valido_nao_precisa_de_html() {
        // Se todos os 9 já estão lá e passam na conferência, o html não é nem olhado.
        let destino = pasta_nova("pronto");
        for origem in &ORIGENS {
            let arquivo = destino.join(origem.saida);
            std::fs::create_dir_all(arquivo.parent().unwrap()).unwrap();
            std::fs::write(&arquivo, b"x").unwrap();
        }
        let conferidor = |_: &Path, _: Tipo| -> Result<(), String> { Ok(()) };
        assert!(todos_prontos(&destino, &conferidor));
        let reprova = |_: &Path, _: Tipo| -> Result<(), String> { Err("estragado".into()) };
        assert!(!todos_prontos(&destino, &reprova));
        std::fs::remove_file(destino.join(ORIGENS[0].saida)).unwrap();
        assert!(!todos_prontos(&destino, &conferidor), "um arquivo faltando");
        let _ = std::fs::remove_dir_all(&destino);
    }

    // ---- Testes com o ffmpeg de verdade (rodam com `cargo test -- --ignored`) -----------------

    /// Gera um ruído de `segundos` s a 48 kHz mono em OGG, para servir de "gravação original".
    fn gravacao_de_mentira(pasta: &Path, segundos: u32) -> std::path::PathBuf {
        let arquivo = pasta.join("original.ogg");
        executar(
            "ffmpeg",
            &["-v", "error", "-y", "-f", "lavfi", "-i", &format!("anoisesrc=d={segundos}:c=pink:r=48000"), "-c:a", "libvorbis", arquivo.to_str().unwrap()],
        )
        .unwrap();
        arquivo
    }

    fn sonda(arquivo: &Path) -> String {
        executar(
            "ffprobe",
            &["-v", "error", "-show_entries", "stream=codec_name,sample_rate:format=duration", "-of", "default=noprint_wrappers=1", arquivo.to_str().unwrap()],
        )
        .unwrap()
    }

    fn duracao(texto: &str) -> f64 {
        texto.lines().find_map(|l| l.strip_prefix("duration=")).unwrap().trim().parse().unwrap()
    }

    #[test]
    #[ignore = "precisa do ffmpeg instalado"]
    fn laco_real_tem_60_segundos_a_44100hz() {
        let pasta = pasta_nova("laco");
        let original = gravacao_de_mentira(&pasta, 100);
        let saida = pasta.join("laco.ogg");
        let plano = plano_do_ambiente(duracao_do_arquivo(&original).unwrap());
        assert!(matches!(plano, Plano::Laco { .. }));
        executar("ffmpeg", &argumentos_ffmpeg(&original, &saida, &plano).iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
        let texto = sonda(&saida);
        assert!((duracao(&texto) - 60.0).abs() < 0.2, "duração {texto}");
        assert_eq!(conferir_ffprobe(&texto, Tipo::Ambiente), Ok(()));
        assert!(texto.contains("sample_rate=44100"), "{texto}");
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    #[ignore = "precisa do ffmpeg instalado"]
    fn ambiente_curto_real_mantem_a_duracao() {
        let pasta = pasta_nova("curto");
        let original = gravacao_de_mentira(&pasta, 20);
        let saida = pasta.join("curto.ogg");
        let plano = plano_do_ambiente(duracao_do_arquivo(&original).unwrap());
        assert_eq!(plano, Plano::Reencodar);
        executar("ffmpeg", &argumentos_ffmpeg(&original, &saida, &plano).iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
        let texto = sonda(&saida);
        assert!((duracao(&texto) - 20.0).abs() < 0.2, "duração {texto}");
        assert!(texto.contains("sample_rate=44100"), "{texto}");
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
