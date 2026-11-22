//! Base64 padrão (RFC 4648), escrito à mão: são poucas linhas e evita uma dependência.
//! `codificar` serve para o teste do celular virtual mandar a foto da tela para o registro do
//! GitHub; `decodificar` serve para o preparo dos sons tirar os áudios de dentro do HTML.

/// As 64 letras do alfabeto do base64 padrão, na ordem dos valores de 0 a 63.
const ALFABETO: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Codifica `bytes` em base64 padrão: termina com `=` quando faltam bytes no último grupo
/// e não tem quebras de linha (igual ao `base64 -w0` do sistema).
pub fn codificar(bytes: &[u8]) -> String {
    let mut saida = String::with_capacity(bytes.len().div_ceil(3) * 4);
    // Cada grupo de 3 bytes (24 bits) vira 4 letras de 6 bits.
    for grupo in bytes.chunks(3) {
        let b0 = u32::from(grupo[0]);
        let b1 = u32::from(*grupo.get(1).unwrap_or(&0));
        let b2 = u32::from(*grupo.get(2).unwrap_or(&0));
        let junto = (b0 << 16) | (b1 << 8) | b2;
        let letra = |deslocamento: u32| ALFABETO[((junto >> deslocamento) & 63) as usize] as char;
        saida.push(letra(18));
        saida.push(letra(12));
        saida.push(if grupo.len() > 1 { letra(6) } else { '=' });
        saida.push(if grupo.len() > 2 { letra(0) } else { '=' });
    }
    saida
}

/// O valor (0 a 63) de uma letra do alfabeto do base64, ou `None` se não for uma letra dele.
#[cfg(feature = "audio")]
fn valor_da_letra(letra: u8) -> Option<u32> {
    match letra {
        b'A'..=b'Z' => Some(u32::from(letra - b'A')),
        b'a'..=b'z' => Some(u32::from(letra - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(letra - b'0') + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// O contrário de `codificar`: devolve os bytes do texto em base64 padrão. Recusa letra fora do
/// alfabeto, espaço, quebra de linha, tamanho que não seja múltiplo de 4 e `=` fora do fim.
#[cfg(feature = "audio")]
pub fn decodificar(texto: &str) -> Result<Vec<u8>, String> {
    let letras = texto.as_bytes();
    if !letras.len().is_multiple_of(4) {
        return Err(format!("base64 com {} letras (o tamanho precisa ser múltiplo de 4)", letras.len()));
    }
    let grupos = letras.len() / 4;
    let mut saida = Vec::with_capacity(grupos * 3);
    for (numero, grupo) in letras.chunks(4).enumerate() {
        let ultimo = numero + 1 == grupos;
        let mut valores = [0u32; 4];
        let mut preenchimento = 0;
        for (posicao, &letra) in grupo.iter().enumerate() {
            if letra == b'=' {
                // O `=` só vale nas duas últimas letras do último grupo.
                if !ultimo || posicao < 2 {
                    return Err("base64 com `=` fora do fim".to_string());
                }
                preenchimento += 1;
            } else if preenchimento > 0 {
                return Err("base64 com letra depois do `=`".to_string());
            } else {
                valores[posicao] = valor_da_letra(letra).ok_or_else(|| format!("letra {:?} não existe no base64", letra as char))?;
            }
        }
        let junto = (valores[0] << 18) | (valores[1] << 12) | (valores[2] << 6) | valores[3];
        let tres = [(junto >> 16) as u8, (junto >> 8) as u8, junto as u8];
        saida.extend_from_slice(&tres[..3 - preenchimento]);
    }
    Ok(saida)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_bate_com_os_vetores_da_rfc_4648() {
        let casos = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (entrada, esperado) in casos {
            assert_eq!(codificar(entrada.as_bytes()), esperado, "entrada {entrada:?}");
        }
    }

    #[test]
    fn base64_dos_256_bytes_bate_com_o_comando_base64_do_sistema() {
        // Conferido uma vez com: python3 -c "...bytes(range(256))..." | base64 -w0
        let esperado = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/w==";
        let bytes: Vec<u8> = (0..=255).collect();
        assert_eq!(codificar(&bytes), esperado);
    }
    #[cfg(feature = "audio")]
    #[test]
    fn decodificar_e_o_inverso_de_codificar() {
        for tamanho in [0usize, 1, 2, 3, 4, 5, 6, 7, 255, 256, 1000] {
            let bytes: Vec<u8> = (0..tamanho).map(|i| (i * 7 + 3) as u8).collect();
            assert_eq!(decodificar(&codificar(&bytes)), Ok(bytes), "tamanho {tamanho}");
        }
        assert_eq!(decodificar("Zm9vYmFy"), Ok(b"foobar".to_vec()));
        assert_eq!(decodificar("Zm9vYg=="), Ok(b"foob".to_vec()));
        assert_eq!(decodificar("Zm9vYmE="), Ok(b"fooba".to_vec()));
    }

    #[cfg(feature = "audio")]
    #[test]
    fn decodificar_recusa_caractere_invalido_e_tamanho_errado() {
        for ruim in ["@@@@", "Zm9v YmFy", "Zm9vY", "Zm9vYg=", "Zg==Zg==", "=Zm9", "Zm9v\n"] {
            assert!(decodificar(ruim).is_err(), "{ruim:?} devia ser recusado");
        }
    }
}
