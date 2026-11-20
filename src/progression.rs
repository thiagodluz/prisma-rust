//! Metas de pontos e níveis.
//!
//! A meta do nível 1 é 1800 pontos e cresce 150 por nível, até um teto (6000 a partir do nível 29).

/// Meta de pontos para passar do nível `level` para o seguinte.
pub fn level_goal(level: u32) -> u64 {
    goal_for(u64::from(level), 6000)
}

/// Meta do nível com um teto escolhido. Saves antigos usavam tetos menores (4650 e 5000).
///
/// `saturating_*` são contas que, em vez de dar erro quando o número fica grande demais,
/// "param no máximo". Assim um nível absurdo nunca derruba o programa.
pub(crate) fn goal_for(level: u64, cap: u64) -> u64 {
    let growth = level.saturating_sub(1).saturating_mul(150);
    cap.min(1800u64.saturating_add(growth))
}

/// Pontos acumulados no início do nível `level`: a soma das metas de todos os níveis anteriores.
///
/// Devolve `None` se a conta estourar o tamanho do número (`checked_*` avisa em vez de dar erro).
pub(crate) fn level_start_at(level: u64, cap: u64) -> Option<u64> {
    // Quantos níveis anteriores ainda estavam "subindo" (meta crescente) e quantos já estavam no teto.
    let rising_levels = level.saturating_sub(1).min(cap.saturating_sub(1800).div_ceil(150));
    let capped_levels = level.saturating_sub(rising_levels).saturating_sub(1);
    let rising = rising_levels
        .checked_mul(1800)?
        .checked_add(rising_levels.checked_mul(rising_levels.saturating_sub(1))?.checked_mul(75)?)?;
    rising.checked_add(capped_levels.checked_mul(cap)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goals_grow_150_per_level_up_to_the_cap() {
        assert_eq!(goal_for(1, 6000), 1800);
        assert_eq!(goal_for(2, 6000), 1950);
        assert_eq!(goal_for(29, 6000), 6000);
        assert_eq!(goal_for(u64::MAX, 6000), 6000, "nível enorme não pode estourar a conta");
    }

    #[test]
    fn level_start_adds_up_the_goals_of_the_previous_levels() {
        let mut sum = 0;
        for level in 1..=40u64 {
            assert_eq!(level_start_at(level, 6000), Some(sum), "nível {level}");
            sum += goal_for(level, 6000);
        }
    }

    #[test]
    fn level_start_reports_overflow_instead_of_panicking() {
        assert_eq!(level_start_at(u64::MAX, 6000), None);
        assert_eq!(level_start_at(u64::MAX, u64::MAX), None);
    }
}
