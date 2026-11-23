//! A vibração do celular: quais "toques" existem e quando cada um é pedido. Tudo "puro" (só
//! escolhe e descreve); quem faz o celular vibrar de verdade é a camada `plataforma`.
//!
//! Os padrões são os do jogo em JS (`app.js`, funções `vibrate` e `play`): uma lista de
//! milissegundos no formato do Android, em que o primeiro número é a espera antes de começar e os
//! seguintes alternam "vibra" e "para".

/// Cada tipo de vibração que o jogo pede.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toque {
    /// Uma jogada válida comum.
    Curto,
    /// Uma jogada em que alguma pedra especial (Pulso, Raio, Espectro, Supernova) foi ativada.
    ComEspecial,
    /// Uma jogada que fez o jogador subir de nível.
    NivelAcima,
    /// O aviso de que a vibração acabou de ser ligada.
    Ligou,
}

impl Toque {
    /// O padrão em milissegundos (espera, vibra, para, vibra...).
    pub fn padrao_ms(self) -> &'static [u32] {
        match self {
            Toque::Curto => &[0, 28],
            Toque::ComEspecial => &[0, 45],
            Toque::NivelAcima => &[0, 28, 55, 35],
            Toque::Ligou => &[0, 55],
        }
    }

    /// O nome nas linhas do registro (`PRISMA_M4 vibracao toque=...`).
    pub fn nome(self) -> &'static str {
        match self {
            Toque::Curto => "curto",
            Toque::ComEspecial => "especial",
            Toque::NivelAcima => "nivel",
            Toque::Ligou => "ligou",
        }
    }
}

/// Que toque uma jogada pede? Jogada inválida não vibra. Nas válidas, subir de nível vence ter
/// ativado um especial, que vence a jogada comum (a mesma ordem do JS).
pub fn toque_da_jogada(valida: bool, ativou_especial: bool, niveis_ganhos: u32) -> Option<Toque> {
    if !valida {
        return None;
    }
    Some(if niveis_ganhos > 0 {
        Toque::NivelAcima
    } else if ativou_especial {
        Toque::ComEspecial
    } else {
        Toque::Curto
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padroes_sao_os_do_js() {
        // Em milissegundos, no formato do Android: o primeiro número é a espera, os seguintes
        // alternam "vibra" e "para".
        assert_eq!(Toque::Curto.padrao_ms(), &[0, 28]);
        assert_eq!(Toque::ComEspecial.padrao_ms(), &[0, 45]);
        assert_eq!(Toque::NivelAcima.padrao_ms(), &[0, 28, 55, 35]);
        assert_eq!(Toque::Ligou.padrao_ms(), &[0, 55]);
    }

    #[test]
    fn nomes_do_registro() {
        assert_eq!(Toque::Curto.nome(), "curto");
        assert_eq!(Toque::ComEspecial.nome(), "especial");
        assert_eq!(Toque::NivelAcima.nome(), "nivel");
        assert_eq!(Toque::Ligou.nome(), "ligou");
    }

    #[test]
    fn jogada_invalida_nao_vibra() {
        assert_eq!(toque_da_jogada(false, true, 1), None);
        assert_eq!(toque_da_jogada(false, false, 0), None);
    }

    #[test]
    fn nivel_vence_especial_que_vence_comum() {
        assert_eq!(toque_da_jogada(true, true, 2), Some(Toque::NivelAcima));
        assert_eq!(toque_da_jogada(true, false, 1), Some(Toque::NivelAcima));
        assert_eq!(toque_da_jogada(true, true, 0), Some(Toque::ComEspecial));
        assert_eq!(toque_da_jogada(true, false, 0), Some(Toque::Curto));
    }
}
