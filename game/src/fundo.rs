//! O som de fundo do Zen (música e ambiente): "o que deveria estar tocando" contra "o que está
//! carregado", e a lista de comandos que leva de um ao outro.
//!
//! Tudo aqui é conta pura (sem Macroquad, sem arquivo): quem executa os comandos é o `audio.rs`.
//! Assim dá para provar, com testes, que cada canal tem no máximo UMA faixa na memória e que
//! nada é carregado de novo à toa.

use crate::preferencias::{Ambiente, Canal, Musica};

/// Uma faixa de áudio de fundo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Faixa {
    Musica(Musica),
    Ambiente(Ambiente),
}

impl Faixa {
    /// Onde o arquivo fica, dentro da pasta dos sons (a pasta `sons` no PC, os assets no Android).
    pub fn caminho(&self) -> &'static str {
        match self {
            Faixa::Musica(Musica::MagicPuzzle) => "musica/magic-puzzle.ogg",
            Faixa::Musica(Musica::CozyPuzzle) => "musica/cozy-puzzle.ogg",
            Faixa::Musica(Musica::SpaceCity) => "musica/space-city.ogg",
            Faixa::Ambiente(Ambiente::Riacho) => "ambiente/stream.ogg",
            Faixa::Ambiente(Ambiente::Chuva) => "ambiente/rain-soft.ogg",
            Faixa::Ambiente(Ambiente::Floresta) => "ambiente/forest-cicadas.ogg",
            Faixa::Ambiente(Ambiente::FlorestaAposChuva) => "ambiente/rainforest.ogg",
            Faixa::Ambiente(Ambiente::Trovoada) => "ambiente/rain-thunder.ogg",
            Faixa::Ambiente(Ambiente::RuidoSuave) => "ambiente/soft-noise.ogg",
        }
    }

    /// O nome da faixa, sem pasta nem extensão (é o que o registro mostra).
    pub fn nome(&self) -> &'static str {
        let caminho = self.caminho();
        let sem_pasta = caminho.rsplit('/').next().unwrap_or(caminho);
        sem_pasta.strip_suffix(".ogg").unwrap_or(sem_pasta)
    }

    /// O canal em que a faixa toca.
    pub fn canal(&self) -> Canal {
        match self {
            Faixa::Musica(_) => Canal::Musica,
            Faixa::Ambiente(_) => Canal::Ambiente,
        }
    }

    /// "musica" ou "ambiente": o `tipo=` das linhas do registro.
    pub fn tipo(&self) -> &'static str {
        match self {
            Faixa::Musica(_) => "musica",
            Faixa::Ambiente(_) => "ambiente",
        }
    }
}

/// O que o jogador quer ouvir agora: por canal, a faixa e o volume (de 0,0 a 1,0), ou `None`
/// se aquele canal deve ficar em silêncio.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Desejado {
    pub musica: Option<(Musica, f32)>,
    pub ambiente: Option<(Ambiente, f32)>,
}

/// O que está carregado agora, na mesma forma. Guarda a faixa que foi TENTADA, mesmo se o
/// carregamento falhou: assim ela não é tentada de novo a cada quadro (só quando o jogador
/// desliga e liga, ou escolhe outra).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Atual {
    pub musica: Option<(Musica, f32)>,
    pub ambiente: Option<(Ambiente, f32)>,
}

/// Uma ordem para a camada de áudio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Comando {
    /// Para o canal e solta a faixa da memória.
    Parar(Canal),
    /// Carrega a faixa e toca em laço, com este volume.
    Carregar(Faixa, f32),
    /// Só muda o volume da faixa que já está tocando.
    Volume(Canal, f32),
}

/// A diferença de volume que ainda vale um comando (abaixo disso, é a mesma coisa).
const VOLUME_MINIMO: f32 = 0.001;

/// Os comandos que levam o `atual` até o `desejado`, canal por canal (primeiro a música, depois
/// o ambiente). Faixa diferente: `Parar` da antiga ANTES do `Carregar` da nova (nunca duas
/// faixas na memória no mesmo canal). Mesma faixa: só `Volume`, e só se mudou. Igual: nada.
pub fn planejar(atual: &Atual, desejado: &Desejado) -> Vec<Comando> {
    let mut comandos = Vec::new();
    planejar_canal(
        Canal::Musica,
        atual.musica.map(|(faixa, volume)| (Faixa::Musica(faixa), volume)),
        desejado.musica.map(|(faixa, volume)| (Faixa::Musica(faixa), volume)),
        &mut comandos,
    );
    planejar_canal(
        Canal::Ambiente,
        atual.ambiente.map(|(faixa, volume)| (Faixa::Ambiente(faixa), volume)),
        desejado.ambiente.map(|(faixa, volume)| (Faixa::Ambiente(faixa), volume)),
        &mut comandos,
    );
    comandos
}

fn planejar_canal(canal: Canal, atual: Option<(Faixa, f32)>, desejado: Option<(Faixa, f32)>, comandos: &mut Vec<Comando>) {
    match (atual, desejado) {
        (None, None) => {}
        (Some(_), None) => comandos.push(Comando::Parar(canal)),
        (None, Some((faixa, volume))) => comandos.push(Comando::Carregar(faixa, volume)),
        (Some((faixa_atual, volume_atual)), Some((faixa, volume))) => {
            if faixa_atual != faixa {
                comandos.push(Comando::Parar(canal));
                comandos.push(Comando::Carregar(faixa, volume));
            } else if (volume_atual - volume).abs() > VOLUME_MINIMO {
                comandos.push(Comando::Volume(canal, volume));
            }
        }
    }
}

/// O que o laço faz com a lista de comandos deste quadro (ver [`decidir`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decisao {
    /// Não há nada a fazer.
    Nada,
    /// Executa os comandos agora.
    Aplicar,
    /// Vai carregar uma faixa nova (isso trava o quadro por um instante): primeiro avisa na tela
    /// "Carregando…" deste canal e só aplica no quadro seguinte.
    Avisar(Canal),
}

/// Decide o que fazer com os `comandos` do quadro. Carregar uma faixa demora e trava a tela;
/// por isso, antes de carregar, o laço mostra um quadro com o aviso "Carregando…" e só no
/// seguinte aplica. `ja_avisado` é o canal cujo aviso já está na tela (veja `App::carregando`).
///
/// - sem comandos: `Nada`;
/// - tem `Carregar` (vale o PRIMEIRO da lista) de um canal ainda não avisado: `Avisar` esse canal;
/// - o resto (só `Parar`/`Volume`, ou o aviso já está na tela): `Aplicar`.
pub fn decidir(comandos: &[Comando], ja_avisado: Option<Canal>) -> Decisao {
    if comandos.is_empty() {
        return Decisao::Nada;
    }
    let primeiro_a_carregar = comandos.iter().find_map(|comando| match comando {
        Comando::Carregar(faixa, _) => Some(faixa.canal()),
        _ => None,
    });
    match primeiro_a_carregar {
        Some(canal) if ja_avisado != Some(canal) => Decisao::Avisar(canal),
        _ => Decisao::Aplicar,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prisma_engine::lcg_random;
    use std::collections::HashSet;

    fn musica(faixa: Musica, volume: f32) -> Option<(Musica, f32)> {
        Some((faixa, volume))
    }

    fn ambiente(faixa: Ambiente, volume: f32) -> Option<(Ambiente, f32)> {
        Some((faixa, volume))
    }

    #[test]
    fn nada_desejado_e_nada_atual_nao_faz_nada() {
        assert_eq!(planejar(&Atual::default(), &Desejado::default()), vec![]);
    }

    #[test]
    fn ligar_musica_carrega_uma_vez() {
        let desejado = Desejado { musica: musica(Musica::CozyPuzzle, 0.7), ambiente: None };
        let comandos = planejar(&Atual::default(), &desejado);
        assert_eq!(comandos, vec![Comando::Carregar(Faixa::Musica(Musica::CozyPuzzle), 0.7)]);
        // Depois de carregada, pedir a mesma coisa não carrega de novo.
        let atual = Atual { musica: musica(Musica::CozyPuzzle, 0.7), ambiente: None };
        assert_eq!(planejar(&atual, &desejado), vec![]);
    }

    #[test]
    fn mesmo_estado_nao_repete_comando() {
        let estado = (musica(Musica::SpaceCity, 0.4), ambiente(Ambiente::Trovoada, 1.0));
        let atual = Atual { musica: estado.0, ambiente: estado.1 };
        let desejado = Desejado { musica: estado.0, ambiente: estado.1 };
        for _ in 0..3 {
            assert_eq!(planejar(&atual, &desejado), vec![]);
        }
        // Uma diferença minúscula de volume (ruído de conta) também não vale um comando.
        let quase = Desejado { musica: musica(Musica::SpaceCity, 0.4005), ambiente: estado.1 };
        assert_eq!(planejar(&atual, &quase), vec![]);
    }

    #[test]
    fn mudar_so_o_volume_so_manda_volume() {
        let atual = Atual { musica: musica(Musica::CozyPuzzle, 0.7), ambiente: ambiente(Ambiente::Chuva, 0.65) };
        let desejado = Desejado { musica: musica(Musica::CozyPuzzle, 0.5), ambiente: ambiente(Ambiente::Chuva, 0.65) };
        assert_eq!(planejar(&atual, &desejado), vec![Comando::Volume(Canal::Musica, 0.5)]);
        // Até silenciar é só volume: a faixa continua carregada.
        let mudo = Desejado { musica: musica(Musica::CozyPuzzle, 0.0), ..desejado };
        assert_eq!(planejar(&atual, &mudo), vec![Comando::Volume(Canal::Musica, 0.0)]);
    }

    #[test]
    fn trocar_de_faixa_para_antes_de_carregar() {
        let atual = Atual { musica: musica(Musica::CozyPuzzle, 0.7), ambiente: ambiente(Ambiente::Chuva, 0.65) };
        let desejado = Desejado { musica: musica(Musica::SpaceCity, 0.7), ambiente: ambiente(Ambiente::Floresta, 0.3) };
        assert_eq!(
            planejar(&atual, &desejado),
            vec![
                Comando::Parar(Canal::Musica),
                Comando::Carregar(Faixa::Musica(Musica::SpaceCity), 0.7),
                Comando::Parar(Canal::Ambiente),
                Comando::Carregar(Faixa::Ambiente(Ambiente::Floresta), 0.3),
            ]
        );
    }

    #[test]
    fn desligar_para() {
        let atual = Atual { musica: musica(Musica::MagicPuzzle, 0.7), ambiente: ambiente(Ambiente::RuidoSuave, 0.2) };
        assert_eq!(planejar(&atual, &Desejado::default()), vec![Comando::Parar(Canal::Musica), Comando::Parar(Canal::Ambiente)]);
    }

    #[test]
    fn musica_e_ambiente_sao_independentes() {
        // Só o ambiente muda: a música nem é mencionada.
        let atual = Atual { musica: musica(Musica::CozyPuzzle, 0.7), ambiente: None };
        let desejado = Desejado { musica: musica(Musica::CozyPuzzle, 0.7), ambiente: ambiente(Ambiente::Riacho, 0.65) };
        assert_eq!(planejar(&atual, &desejado), vec![Comando::Carregar(Faixa::Ambiente(Ambiente::Riacho), 0.65)]);
        // Só a música desliga: o ambiente continua.
        let sem_musica = Desejado { musica: None, ..desejado };
        let atual = Atual { musica: musica(Musica::CozyPuzzle, 0.7), ambiente: ambiente(Ambiente::Riacho, 0.65) };
        assert_eq!(planejar(&atual, &sem_musica), vec![Comando::Parar(Canal::Musica)]);
    }

    /// Um "áudio de mentira": aplica os comandos a um `Atual` e confere a regra de ouro: nunca
    /// carregar numa vaga ocupada (seria uma segunda faixa na memória do mesmo canal).
    fn aplicar_de_mentira(atual: &mut Atual, comandos: &[Comando], cargas: &mut usize) {
        for comando in comandos {
            match *comando {
                Comando::Parar(Canal::Musica) => atual.musica = None,
                Comando::Parar(Canal::Ambiente) => atual.ambiente = None,
                Comando::Parar(Canal::Efeitos) => panic!("efeitos não têm faixa de fundo"),
                Comando::Carregar(Faixa::Musica(faixa), volume) => {
                    assert!(atual.musica.is_none(), "carregou música com outra ainda na memória");
                    atual.musica = Some((faixa, volume));
                    *cargas += 1;
                }
                Comando::Carregar(Faixa::Ambiente(faixa), volume) => {
                    assert!(atual.ambiente.is_none(), "carregou ambiente com outro ainda na memória");
                    atual.ambiente = Some((faixa, volume));
                    *cargas += 1;
                }
                Comando::Volume(Canal::Musica, volume) => {
                    let (faixa, _) = atual.musica.expect("volume sem música carregada");
                    atual.musica = Some((faixa, volume));
                }
                Comando::Volume(Canal::Ambiente, volume) => {
                    let (faixa, _) = atual.ambiente.expect("volume sem ambiente carregado");
                    atual.ambiente = Some((faixa, volume));
                }
                Comando::Volume(Canal::Efeitos, _) => panic!("efeitos não têm faixa de fundo"),
            }
        }
    }

    #[test]
    fn ligar_e_desligar_varias_vezes_nunca_deixa_duas_faixas_no_mesmo_canal() {
        let mut sorte = lcg_random(2024);
        let mut sortear = |limite: usize| ((sorte() * limite as f64) as usize).min(limite - 1);
        let volumes = [0.0, 0.25, 0.5, 0.65, 0.7, 1.0];
        let mut atual = Atual::default();
        let mut cargas = 0;
        let mut pedidos_de_faixa = 0;
        let mut ultimo = Desejado::default();
        for passo in 0..200 {
            let desejado = Desejado {
                musica: (sortear(3) != 0).then(|| (Musica::TODAS[sortear(3)], volumes[sortear(6)])),
                ambiente: (sortear(3) != 0).then(|| (Ambiente::TODOS[sortear(6)], volumes[sortear(6)])),
            };
            pedidos_de_faixa += usize::from(desejado.musica.is_some()) + usize::from(desejado.ambiente.is_some());
            let comandos = planejar(&atual, &desejado);
            aplicar_de_mentira(&mut atual, &comandos, &mut cargas);
            assert_eq!((atual.musica, atual.ambiente), (desejado.musica, desejado.ambiente), "passo {passo}: depois dos comandos o atual é o desejado");
            // E pedir o mesmo de novo não faz mais nada.
            assert_eq!(planejar(&atual, &desejado), vec![], "passo {passo}");
            ultimo = desejado;
        }
        assert!(cargas > 20, "a simulação carregou de verdade ({cargas} cargas)");
        assert!(cargas <= pedidos_de_faixa, "nunca carrega mais do que foi pedido");
        // Desligar tudo no fim solta as duas.
        let comandos = planejar(&atual, &Desejado::default());
        aplicar_de_mentira(&mut atual, &comandos, &mut cargas);
        assert_eq!(atual, Atual::default(), "depois de {ultimo:?}");
    }

    #[test]
    fn faixa_que_falhou_nao_e_tentada_de_novo_ate_desligar_e_ligar() {
        let desejado = Desejado { musica: musica(Musica::CozyPuzzle, 0.7), ambiente: None };
        let mut atual = Atual::default();
        let mut cargas = 0;
        // Primeira tentativa: o planejador manda carregar. O áudio falha, mas `Atual` guarda a faixa.
        let comandos = planejar(&atual, &desejado);
        assert_eq!(comandos.len(), 1);
        aplicar_de_mentira(&mut atual, &comandos, &mut cargas);
        // Quadro após quadro, nada de tentar outra vez.
        for _ in 0..100 {
            assert_eq!(planejar(&atual, &desejado), vec![]);
        }
        assert_eq!(cargas, 1);
        // O jogador desliga e liga: agora tenta de novo.
        let comandos = planejar(&atual, &Desejado::default());
        assert_eq!(comandos, vec![Comando::Parar(Canal::Musica)]);
        aplicar_de_mentira(&mut atual, &comandos, &mut cargas);
        let comandos = planejar(&atual, &desejado);
        assert_eq!(comandos, vec![Comando::Carregar(Faixa::Musica(Musica::CozyPuzzle), 0.7)]);
        // Escolher outra faixa também é uma nova tentativa.
        aplicar_de_mentira(&mut atual, &comandos, &mut cargas);
        let outra = Desejado { musica: musica(Musica::SpaceCity, 0.7), ambiente: None };
        assert_eq!(planejar(&atual, &outra), vec![Comando::Parar(Canal::Musica), Comando::Carregar(Faixa::Musica(Musica::SpaceCity), 0.7)]);
    }

    #[test]
    fn caminhos_sao_relativos_ogg_e_sem_repeticao() {
        let todas: Vec<Faixa> =
            Musica::TODAS.iter().map(|&m| Faixa::Musica(m)).chain(Ambiente::TODOS.iter().map(|&a| Faixa::Ambiente(a))).collect();
        assert_eq!(todas.len(), 9, "3 músicas e 6 ambientes");
        let mut vistos = HashSet::new();
        let mut nomes = HashSet::new();
        for faixa in &todas {
            let caminho = faixa.caminho();
            assert!(!caminho.starts_with('/') && !caminho.contains(".."), "{caminho} não é relativo");
            assert!(caminho.ends_with(".ogg"), "{caminho}");
            let pasta = match faixa {
                Faixa::Musica(_) => "musica/",
                Faixa::Ambiente(_) => "ambiente/",
            };
            assert!(caminho.starts_with(pasta), "{caminho} fora da pasta {pasta}");
            assert_eq!(caminho, format!("{pasta}{}.ogg", faixa.nome()), "o nome é o caminho sem pasta e sem extensão");
            assert!(vistos.insert(caminho), "{caminho} repetido");
            assert!(nomes.insert(faixa.nome()), "nome repetido: {}", faixa.nome());
            assert_eq!(faixa.tipo(), pasta.trim_end_matches('/'));
            assert_eq!(faixa.canal(), if faixa.tipo() == "musica" { Canal::Musica } else { Canal::Ambiente });
        }
        // Os nomes combinados com a preparação dos sons (`prisma_tools audio`).
        let esperados: HashSet<&str> = [
            "musica/magic-puzzle.ogg",
            "musica/cozy-puzzle.ogg",
            "musica/space-city.ogg",
            "ambiente/stream.ogg",
            "ambiente/rain-soft.ogg",
            "ambiente/forest-cicadas.ogg",
            "ambiente/rainforest.ogg",
            "ambiente/rain-thunder.ogg",
            "ambiente/soft-noise.ogg",
        ]
        .into_iter()
        .collect();
        assert_eq!(vistos, esperados);
        // E o mapa de cada opção da folha (a música padrão é a Cozy Puzzle; o ambiente, a chuva suave).
        assert_eq!(Faixa::Musica(Musica::CozyPuzzle).caminho(), "musica/cozy-puzzle.ogg");
        assert_eq!(Faixa::Ambiente(Ambiente::Chuva).caminho(), "ambiente/rain-soft.ogg");
        assert_eq!(Faixa::Ambiente(Ambiente::Floresta).caminho(), "ambiente/forest-cicadas.ogg");
        assert_eq!(Faixa::Ambiente(Ambiente::FlorestaAposChuva).caminho(), "ambiente/rainforest.ogg");
    }

    // ---- M4: decidir se avisa "Carregando…" antes de aplicar ----

    fn carregar_musica() -> Comando {
        Comando::Carregar(Faixa::Musica(Musica::CozyPuzzle), 0.7)
    }

    fn carregar_ambiente() -> Comando {
        Comando::Carregar(Faixa::Ambiente(Ambiente::Chuva), 0.65)
    }

    #[test]
    fn decidir_sem_comandos_e_nada() {
        assert_eq!(decidir(&[], None), Decisao::Nada);
        assert_eq!(decidir(&[], Some(Canal::Musica)), Decisao::Nada);
    }

    #[test]
    fn decidir_com_carregar_novo_manda_avisar() {
        assert_eq!(decidir(&[carregar_musica()], None), Decisao::Avisar(Canal::Musica));
        assert_eq!(decidir(&[carregar_ambiente()], None), Decisao::Avisar(Canal::Ambiente));
        // Trocar de faixa (parar a antiga e carregar a nova) também avisa.
        let troca = [Comando::Parar(Canal::Musica), carregar_musica()];
        assert_eq!(decidir(&troca, None), Decisao::Avisar(Canal::Musica));
    }

    #[test]
    fn decidir_depois_do_aviso_manda_aplicar() {
        assert_eq!(decidir(&[carregar_musica()], Some(Canal::Musica)), Decisao::Aplicar);
        assert_eq!(decidir(&[carregar_ambiente()], Some(Canal::Ambiente)), Decisao::Aplicar);
        // O aviso era de outro canal: o canal que vai carregar agora ainda não foi avisado.
        assert_eq!(decidir(&[carregar_musica()], Some(Canal::Ambiente)), Decisao::Avisar(Canal::Musica));
    }

    #[test]
    fn decidir_so_com_volume_ou_parar_aplica_sem_avisar() {
        let comandos = [Comando::Parar(Canal::Musica), Comando::Volume(Canal::Ambiente, 0.5)];
        assert_eq!(decidir(&comandos, None), Decisao::Aplicar);
        assert_eq!(decidir(&[Comando::Volume(Canal::Musica, 0.0)], Some(Canal::Musica)), Decisao::Aplicar);
    }

    #[test]
    fn decidir_com_dois_carregar_avisa_o_primeiro_canal() {
        // A ordem natural do planejador: a música vem antes do ambiente.
        let desejado = Desejado { musica: musica(Musica::CozyPuzzle, 0.7), ambiente: ambiente(Ambiente::Chuva, 0.65) };
        let comandos = planejar(&Atual::default(), &desejado);
        assert_eq!(decidir(&comandos, None), Decisao::Avisar(Canal::Musica));
        assert_eq!(decidir(&comandos, Some(Canal::Musica)), Decisao::Aplicar, "o aviso de música já está na tela");
        // Vale o primeiro `Carregar` da lista, qualquer que seja o canal.
        assert_eq!(decidir(&[carregar_ambiente(), carregar_musica()], None), Decisao::Avisar(Canal::Ambiente));
    }
}
