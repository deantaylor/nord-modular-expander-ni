//! Project-owned mock patch format. This is not a Nord .pch/.prf parser.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Slot {
    A,
    B,
    C,
    D,
}
impl Slot {
    pub const ALL: [Self; 4] = [Self::A, Self::B, Self::C, Self::D];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModuleId(pub String);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParameterId(pub String);
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    Linear,
    Log,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Kind {
    Continuous {
        min: f64,
        max: f64,
        scale: Scale,
        unit: String,
    },
    Integer {
        min: i32,
        max: i32,
        unit: String,
    },
    Enum {
        choices: Vec<String>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub id: ParameterId,
    pub name: String,
    pub kind: Kind,
    pub value: f64,
}
impl Parameter {
    pub fn bounds(&self) -> (f64, f64) {
        match &self.kind {
            Kind::Continuous { min, max, .. } => (*min, *max),
            Kind::Integer { min, max, .. } => (f64::from(*min), f64::from(*max)),
            Kind::Enum { choices } => (0.0, choices.len().saturating_sub(1) as f64),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        identifier(&self.id.0)?;
        text(&self.name, 32)?;
        match &self.kind {
            Kind::Continuous {
                min,
                max,
                scale,
                unit,
            } => {
                text(unit, 12)?;
                if !min.is_finite()
                    || !max.is_finite()
                    || min >= max
                    || !(max - min).is_finite()
                    || (*scale == Scale::Log
                        && (*min <= 0.0 || !(max / min).is_finite() || (max / min).ln() <= 0.0))
                {
                    return Err(format!("Invalid continuous bounds: {}", self.id.0));
                }
            }
            Kind::Integer { min, max, unit } => {
                text(unit, 12)?;
                if min >= max {
                    return Err("Invalid integer bounds".into());
                }
            }
            Kind::Enum { choices } => {
                if choices.is_empty() || choices.len() > 64 {
                    return Err("Enum needs 1–64 choices".into());
                }
                for choice in choices {
                    text(choice, 32)?;
                }
            }
        }
        self.check_value(self.value)
    }
    pub fn check_value(&self, value: f64) -> Result<(), String> {
        let (min, max) = self.bounds();
        if !value.is_finite() || value < min || value > max {
            return Err(format!("{} must be in {min}..={max}", self.id.0));
        }
        if !matches!(self.kind, Kind::Continuous { .. }) && value.fract() != 0.0 {
            return Err(format!("{} needs a whole-number value", self.id.0));
        }
        Ok(())
    }
    pub fn normalized(&self) -> f64 {
        let (min, max) = self.bounds();
        if min == max {
            return 0.0;
        }
        match self.kind {
            Kind::Continuous {
                scale: Scale::Log, ..
            } => (self.value / min).ln() / (max / min).ln(),
            _ => (self.value - min) / (max - min),
        }
    }
    pub fn value_at(&self, normalized: f64) -> f64 {
        let n = normalized.clamp(0.0, 1.0);
        let (min, max) = self.bounds();
        match self.kind {
            Kind::Continuous {
                scale: Scale::Log, ..
            } => min * (max / min).powf(n),
            Kind::Continuous { .. } => min + n * (max - min),
            _ => (min + n * (max - min)).round(),
        }
        .clamp(min, max)
    }
    pub fn display(&self) -> String {
        match &self.kind {
            Kind::Enum { choices } => choices[self.value as usize].clone(),
            Kind::Integer { unit, .. } => format!("{:.0}{}", self.value, unit),
            Kind::Continuous { unit, .. } if unit == "Hz" && self.value >= 1000.0 => {
                format!("{:.2} kHz", self.value / 1000.0)
            }
            Kind::Continuous { unit, .. } if unit == "s" && self.value < 1.0 => {
                format!("{:.0} ms", self.value * 1000.0)
            }
            Kind::Continuous { unit, .. } => format!("{:.1}{}", self.value, unit),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub parameters: Vec<Parameter>,
}
impl Module {
    pub fn pages(&self) -> usize {
        self.parameters.len().div_ceil(8)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub id: String,
    pub name: String,
    pub modules: Vec<Module>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotState {
    pub patch: Patch,
    pub module: ModuleId,
    pub page: usize,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub selected_slot: Slot,
    pub slots: [SlotState; 4],
}
impl Project {
    pub fn initial() -> Self {
        let slot = SlotState {
            patch: preset("init").unwrap(),
            module: ModuleId("filter".into()),
            page: 0,
        };
        Self {
            selected_slot: Slot::A,
            slots: std::array::from_fn(|_| slot.clone()),
        }
    }
    pub fn active(&self) -> (&SlotState, &Module) {
        let slot = &self.slots[self.selected_slot.index()];
        (
            slot,
            slot.patch
                .modules
                .iter()
                .find(|m| m.id == slot.module)
                .unwrap(),
        )
    }
    pub fn validate(&self) -> Result<(), String> {
        for slot in &self.slots {
            identifier(&slot.patch.id)?;
            text(&slot.patch.name, 64)?;
            if slot.patch.modules.is_empty() || slot.patch.modules.len() > 64 {
                return Err("Patch needs 1–64 modules".into());
            }
            let mut modules = HashSet::new();
            for module in &slot.patch.modules {
                identifier(&module.id.0)?;
                text(&module.name, 32)?;
                if !modules.insert(&module.id.0) {
                    return Err("Duplicate module ID".into());
                }
                if module.parameters.is_empty() || module.parameters.len() > 128 {
                    return Err("Module needs 1–128 parameters".into());
                }
                let mut parameters = HashSet::new();
                for parameter in &module.parameters {
                    parameter.validate()?;
                    if !parameters.insert(&parameter.id.0) {
                        return Err("Duplicate parameter ID".into());
                    }
                }
            }
            let module = slot
                .patch
                .modules
                .iter()
                .find(|m| m.id == slot.module)
                .ok_or("Selected module is missing")?;
            if slot.page >= module.pages() {
                return Err("Selected page does not exist".into());
            }
        }
        Ok(())
    }
}
fn text(s: &str, max: usize) -> Result<(), String> {
    if s.len() > max || s.chars().any(char::is_control) {
        Err("Invalid/oversized text field".into())
    } else {
        Ok(())
    }
}
fn identifier(s: &str) -> Result<(), String> {
    if s.is_empty() || s.len() > 40 || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        Err("Invalid identifier".into())
    } else {
        Ok(())
    }
}
pub const PRESETS: [(&str, &str); 3] = [
    ("init", "Init Mock"),
    ("warm_pad", "Warm Pad Mock"),
    ("step_bass", "Step Bass Mock"),
];
fn continuous(
    id: &str,
    name: &str,
    value: f64,
    min: f64,
    max: f64,
    scale: Scale,
    unit: &str,
) -> Parameter {
    Parameter {
        id: ParameterId(id.into()),
        name: name.into(),
        value,
        kind: Kind::Continuous {
            min,
            max,
            scale,
            unit: unit.into(),
        },
    }
}
fn integer(id: &str, name: &str, value: i32, min: i32, max: i32, unit: &str) -> Parameter {
    Parameter {
        id: ParameterId(id.into()),
        name: name.into(),
        value: f64::from(value),
        kind: Kind::Integer {
            min,
            max,
            unit: unit.into(),
        },
    }
}
fn choice(id: &str, name: &str, value: usize, choices: &[&str]) -> Parameter {
    Parameter {
        id: ParameterId(id.into()),
        name: name.into(),
        value: value as f64,
        kind: Kind::Enum {
            choices: choices.iter().map(|s| s.to_string()).collect(),
        },
    }
}
pub fn preset(id: &str) -> Result<Patch, String> {
    let name = PRESETS
        .iter()
        .find(|(key, _)| *key == id)
        .ok_or("Unknown mock preset")?
        .1;
    use Scale::{Linear as L, Log as G};
    let mut modules = vec![
        Module {
            id: ModuleId("osc".into()),
            name: "Oscillator".into(),
            parameters: vec![
                continuous("frequency", "FREQUENCY", 220., 20., 20000., G, "Hz"),
                choice("wave", "WAVE", 2, &["Sine", "Triangle", "Saw", "Pulse"]),
                continuous("detune", "DETUNE", 0., -24., 24., L, "st"),
                continuous("pulse_width", "PULSE WIDTH", 50., 1., 99., L, "%"),
                integer("voices", "VOICES", 1, 1, 8, ""),
                continuous("fm", "FM AMOUNT", 0., 0., 100., L, "%"),
                continuous("glide", "GLIDE", 0., 0., 2., L, "s"),
                continuous("level", "LEVEL", -6., -60., 0., L, "dB"),
                continuous("phase", "PHASE", 0., 0., 360., L, "°"),
                integer("octave", "OCTAVE", 0, -2, 2, ""),
            ],
        },
        Module {
            id: ModuleId("filter".into()),
            name: "Filter".into(),
            parameters: vec![
                continuous("cutoff", "CUTOFF", 1050., 20., 20000., G, "Hz"),
                continuous("resonance", "RESONANCE", 32., 0., 100., L, "%"),
                continuous("env_amount", "ENV AMOUNT", 40., -100., 100., L, "%"),
                continuous("keytrack", "KEY TRACK", 75., 0., 100., L, "%"),
                choice("mode", "MODE", 0, &["Low-pass", "Band-pass", "High-pass"]),
                choice("slope", "SLOPE", 1, &["12 dB", "24 dB"]),
                continuous("drive", "DRIVE", 0., 0., 24., L, "dB"),
                choice("bypass", "BYPASS", 0, &["Off", "On"]),
            ],
        },
        Module {
            id: ModuleId("envelope".into()),
            name: "Envelope".into(),
            parameters: vec![
                continuous("attack", "ATTACK", 0.005, 0.001, 10., G, "s"),
                continuous("decay", "DECAY", 0.28, 0.001, 10., G, "s"),
                continuous("sustain", "SUSTAIN", 72., 0., 100., L, "%"),
                continuous("release", "RELEASE", 0.37, 0.001, 20., G, "s"),
            ],
        },
        Module {
            id: ModuleId("mixer".into()),
            name: "Output Mixer".into(),
            parameters: vec![
                continuous("level", "LEVEL", -6., -60., 0., L, "dB"),
                continuous("pan", "PAN", 0., -100., 100., L, "%"),
                integer("polyphony", "POLYPHONY", 8, 1, 16, ""),
            ],
        },
    ];
    if id == "warm_pad" {
        modules[2].parameters[0].value = 0.8;
        modules[1].parameters[0].value = 3000.;
        modules[0].parameters[4].value = 4.;
    }
    if id == "step_bass" {
        modules[0].parameters[0].value = 55.;
        modules[0].parameters[9].value = -1.;
        modules[1].parameters[0].value = 350.;
        modules.pop();
    }
    Ok(Patch {
        id: id.into(),
        name: name.into(),
        modules,
    })
}
