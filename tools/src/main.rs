//! `prisma_tools`: as ferramentas do Prisma que não fazem parte do jogo.
//!
//! - `prisma_tools emulador <pasta-do-apk> <pasta-de-saida>`: joga no celular virtual e confere.
//! - `prisma_tools imagens <pasta-do-prisma-js> <pasta-do-jogo>`: gera as imagens do jogo.

// A Task 3 (comando `emulador`) passa a usar o base64; até lá ele só é testado.
#[allow(dead_code)]
mod base64;

use std::path::PathBuf;
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
        // Provisório: os dois comandos entram nas Tasks 3 e 5.
        Ok(comando) => {
            eprintln!("ERRO: ainda não implementado: {comando:?}");
            ExitCode::from(1)
        }
    }
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
