//! `prisma_tools`: as ferramentas do Prisma que não fazem parte do jogo.
//!
//! - `prisma_tools emulador <pasta-do-apk> <pasta-de-saida>`: joga no celular virtual e confere.
//! - `prisma_tools imagens <pasta-do-prisma-js> <pasta-do-jogo>`: gera as imagens do jogo.

mod base64;
mod emulador;
#[cfg(feature = "imagens")]
mod imagens;
mod registro;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// O que a pessoa pediu na linha de comando, já separado e conferido.
#[derive(Debug, PartialEq, Eq)]
pub enum Comando {
    /// Joga no celular virtual (via `adb`) e confere o registro.
    Emulador { apk: PathBuf, saida: PathBuf },
    /// Gera as imagens do jogo a partir das imagens do Prisma original (JS).
    Imagens { origem: PathBuf, jogo: PathBuf },
}

const USO: &str = "Uso:
  prisma_tools emulador <pasta-do-apk> <pasta-de-saida>
  prisma_tools imagens <pasta-do-prisma-js> <pasta-do-jogo>";

/// Lê os argumentos (sem o nome do programa). Se não entender, devolve o texto de uso.
pub fn ler_comando(args: &[String]) -> Result<Comando, String> {
    // `[a, b, c]` casa só com listas de exatamente três itens: falta ou sobra de caminho cai no `_`.
    match args {
        [sub, apk, saida] if sub == "emulador" => Ok(Comando::Emulador {
            apk: PathBuf::from(apk),
            saida: PathBuf::from(saida),
        }),
        [sub, origem, jogo] if sub == "imagens" => Ok(Comando::Imagens {
            origem: PathBuf::from(origem),
            jogo: PathBuf::from(jogo),
        }),
        _ => Err(USO.to_string()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match ler_comando(&args) {
        Err(uso) => {
            eprintln!("{uso}");
            ExitCode::from(2)
        }
        Ok(Comando::Emulador { apk, saida }) => {
            ExitCode::from(u8::try_from(emulador::rodar_do_programa(&apk, &saida)).unwrap_or(1))
        }
        Ok(Comando::Imagens { origem, jogo }) => rodar_imagens(&origem, &jogo),
    }
}

/// Gera as imagens do jogo e lista cada arquivo gravado. Sai com 1 se alguma imagem de origem
/// estiver faltando ou errada (e, nesse caso, não grava nada).
#[cfg(feature = "imagens")]
fn rodar_imagens(origem: &Path, jogo: &Path) -> ExitCode {
    match imagens::preparar(origem, jogo) {
        Ok(arquivos) => {
            for arquivo in &arquivos {
                println!("gerado {}", arquivo.display());
            }
            ExitCode::SUCCESS
        }
        Err(erro) => {
            eprintln!("ERRO: {erro}");
            ExitCode::from(1)
        }
    }
}

/// O programa do celular virtual é compilado sem a biblioteca de imagens (`--no-default-features`).
#[cfg(not(feature = "imagens"))]
fn rodar_imagens(_origem: &Path, _jogo: &Path) -> ExitCode {
    eprintln!("ERRO: este programa foi compilado sem o comando `imagens` (use o padrão, sem --no-default-features)");
    ExitCode::from(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn texto(itens: &[&str]) -> Vec<String> {
        itens.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn ler_comando_entende_os_dois_subcomandos() {
        assert_eq!(
            ler_comando(&texto(&["emulador", "apk", "resultado"])),
            Ok(Comando::Emulador {
                apk: PathBuf::from("apk"),
                saida: PathBuf::from("resultado")
            })
        );
        assert_eq!(
            ler_comando(&texto(&["imagens", "/js", "game"])),
            Ok(Comando::Imagens {
                origem: PathBuf::from("/js"),
                jogo: PathBuf::from("game")
            })
        );
    }

    #[test]
    fn ler_comando_recusa_o_resto_com_o_uso() {
        let casos: [&[&str]; 8] = [
            &[],
            &["inventado"],
            &["emulador"],
            &["emulador", "apk"],
            &["emulador", "apk", "saida", "sobra"],
            &["imagens"],
            &["imagens", "/js"],
            &["imagens", "/js", "game", "sobra"],
        ];
        for caso in casos {
            let erro = ler_comando(&texto(caso)).expect_err(&format!("{caso:?} devia ser recusado"));
            assert!(erro.contains("emulador"), "uso sem 'emulador': {erro}");
            assert!(erro.contains("imagens"), "uso sem 'imagens': {erro}");
        }
    }
}
