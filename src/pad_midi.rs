//! Physical pad numbers, not HID report indices, determine the note layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Mode {
    #[default]
    Drums,
    Keyboard,
}
impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Drums => "drums",
            Self::Keyboard => "keyboard",
        }
    }
    pub fn base(self) -> u8 {
        match self {
            Self::Drums => 36,
            Self::Keyboard => 48,
        }
    }
}
#[derive(Default)]
pub struct Pads {
    pub mode: Mode,
    pub held: [bool; 16],
}
impl Pads {
    pub fn index(name: &str) -> Option<usize> {
        name.strip_prefix("pad_")?
            .parse::<usize>()
            .ok()?
            .checked_sub(1)
            .filter(|n| *n < 16)
    }
    pub fn change(&mut self, name: &str, pressure: f32) -> Option<(u8, u8, u8)> {
        if !pressure.is_finite() {
            return None;
        }
        let index = Self::index(name)?;
        let pressed = pressure > 0.;
        if self.held[index] == pressed {
            return None;
        }
        self.held[index] = pressed;
        let velocity = if pressed {
            (pressure.clamp(0., 1.).sqrt() * 127.)
                .round()
                .clamp(1., 127.) as u8
        } else {
            0
        };
        Some((index as u8, self.mode.base() + index as u8, velocity))
    }
    pub fn clear(&mut self) {
        self.held.fill(false);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_numbers_modes_velocity_and_one_note_per_strike() {
        let mut p = Pads::default();
        assert_eq!(p.change("pad_1", 0.25), Some((0, 36, 64)));
        assert_eq!(p.change("pad_1", 0.8), None);
        assert_eq!(p.change("pad_1", 0.), Some((0, 36, 0)));
        assert_eq!(p.change("pad_1", 0.), None);
        p.mode = Mode::Keyboard;
        assert_eq!(p.change("pad_13", 1.), Some((12, 60, 127)));
        assert_eq!(p.change("pad_16", 0.01), Some((15, 63, 13)));
        assert_eq!(p.change("pad_0", 1.), None);
        assert_eq!(p.change("pad_17", 1.), None);
        assert_eq!(p.change("pad_2", f32::NAN), None);
        p.clear();
        assert!(!p.held.iter().any(|v| *v));
    }
}
