//! Base64 padrão (RFC 4648), escrito à mão: são ~20 linhas e evita uma dependência.
//! Serve para o teste do celular virtual mandar a foto da tela para o registro do GitHub.

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
}
