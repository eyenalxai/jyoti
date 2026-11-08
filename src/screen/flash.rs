use std::time::{Duration, Instant};

const FLASH_WINDOW: usize = 3;
const FLASH_HOLD: Duration = Duration::from_secs(2);
const FLASH_PEAK_TO_PEAK: u8 = 25;

#[derive(Clone, Debug)]
pub struct FlashGuard {
    targets: Vec<u8>,
    held_until: Option<Instant>,
    armed: bool,
}

impl Default for FlashGuard {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            held_until: None,
            armed: true,
        }
    }
}

impl FlashGuard {
    pub fn is_flashing(&self, now: Instant) -> bool {
        self.held_until.is_some_and(|until| now < until)
    }

    pub fn guard(&mut self, target: u8, now: Instant) -> bool {
        self.targets.push(target);
        if self.targets.len() > FLASH_WINDOW {
            self.targets.remove(0);
        }

        if self.is_flashing(now) {
            return true;
        }

        let mut armed = if self.held_until.is_none() {
            self.armed
        } else {
            false
        };
        if !armed
            && (same_direction(&self.targets) || peak_to_peak(&self.targets) <= FLASH_PEAK_TO_PEAK)
        {
            armed = true;
        }
        if armed
            && flips_direction(&self.targets)
            && reversal_swing(&self.targets) > FLASH_PEAK_TO_PEAK
        {
            self.held_until = Some(now + FLASH_HOLD);
            self.armed = false;
            return true;
        }

        self.held_until = None;
        self.armed = armed;
        false
    }
}

fn deltas(targets: &[u8]) -> Option<(i8, i8)> {
    if targets.len() < FLASH_WINDOW {
        return None;
    }
    let first_value = *targets.first()?;
    let second_value = *targets.get(1)?;
    let third_value = *targets.get(2)?;
    let first = sign(i16::from(second_value) - i16::from(first_value));
    let second = sign(i16::from(third_value) - i16::from(second_value));
    Some((first, second))
}

fn sign(value: i16) -> i8 {
    match value.cmp(&0) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn same_direction(targets: &[u8]) -> bool {
    deltas(targets).is_some_and(|(first, second)| first != 0 && first == second)
}

fn flips_direction(targets: &[u8]) -> bool {
    deltas(targets).is_some_and(|(first, second)| first != 0 && second != 0 && first != second)
}

fn peak_to_peak(targets: &[u8]) -> u8 {
    let Some(max) = targets.iter().max() else {
        return 0;
    };
    let Some(min) = targets.iter().min() else {
        return 0;
    };
    max.saturating_sub(*min)
}

fn reversal_swing(targets: &[u8]) -> u8 {
    let Some(second) = targets.get(1) else {
        return 0;
    };
    let Some(third) = targets.get(2) else {
        return 0;
    };
    third.abs_diff(*second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_large_reversal_holds_the_brightness() {
        let now = Instant::now();
        let mut guard = FlashGuard::default();
        assert!(!guard.guard(100, now));
        assert!(!guard.guard(50, now));
        assert!(guard.guard(100, now));
        assert!(guard.is_flashing(now + Duration::from_millis(500)));
        assert!(!guard.is_flashing(now + Duration::from_secs(3)));
    }
}
