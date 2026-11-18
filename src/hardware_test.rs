//! Hardware observations count input events; they do not certify display/LED quality.
use crate::model::State;
use encdr::Event;
use serde_json::{json, Value};

pub const HTML: &str = include_str!("../screens/hardware-test.html");

pub struct Panel {
    demo: State,
    connection: Value,
    test: Option<Controls>,
}
impl Panel {
    pub fn new(test: bool, simulated: bool) -> Self {
        Self {
            demo: State::new(),
            connection: json!({"status": if simulated {"SIMULATED"} else {"WAITING FOR MK3"}, "reconnects": 0}),
            test: test.then(|| Controls::new(simulated)),
        }
    }
    pub fn snapshot(&self) -> Value {
        let mut snapshot = self
            .test
            .as_ref()
            .map_or_else(|| self.demo.as_json(), Controls::snapshot);
        snapshot["connection"] = self.connection.clone();
        snapshot
    }
    pub fn connection(&self) -> &Value { &self.connection }
    pub fn set_connection(&mut self, connection: Value) {
        self.connection = connection;
    }
    pub fn revision(&self) -> u64 {
        self.demo.revision()
    }
    pub fn project(&self) -> &crate::project::Project {
        self.demo.project()
    }
    pub fn restore(&mut self, project: crate::project::Project) -> Result<bool, String> {
        if self.test.is_some() {
            return Err("Project load is unavailable in hardware-test mode".into());
        }
        self.demo.restore(project)
    }
    pub fn command(&mut self, command: &str) -> bool {
        if let Some(test) = &mut self.test {
            // Terminal events are only permitted in a simulated test session.
            if !test.simulated {
                return false;
            }
            let parts: Vec<_> = command.split_whitespace().collect();
            match parts.as_slice() {
                ["turn", index, delta] => match (index.parse::<usize>(), delta.parse::<f64>()) {
                    (Ok(i), Ok(d)) => test.encoder(&format!("screen_encoder_{i}"), d),
                    _ => false,
                },
                ["button", name] => test.button(name),
                ["pad", index] => index
                    .parse::<usize>()
                    .ok()
                    .is_some_and(|i| test.pad(&format!("pad_{i}"))),
                _ => false,
            }
        } else {
            self.demo.command(command)
        }
    }
    pub fn event(&mut self, event: &Event) -> bool {
        if let Some(test) = &mut self.test {
            match event {
                Event::EncoderFine { name, delta, .. } => test.encoder(name, f64::from(*delta)),
                Event::Button {
                    name,
                    pressed: true,
                    ..
                } => test.button(name),
                // Some descriptors expose pressure pads as Grid events instead.
                Event::Grid { name, pressure, .. } if pressure.is_finite() && *pressure > 0.0 => {
                    test.pad(name)
                }
                _ => false,
            }
        } else {
            match event {
                Event::EncoderFine { name, delta, .. } => self.demo.turn(name, f64::from(*delta)),
                Event::Button {
                    name,
                    pressed: true,
                    ..
                } => self.demo.button(name),
                _ => false,
            }
        }
    }
}
fn index(name: &str, prefix: &str, count: usize) -> Option<usize> {
    name.strip_prefix(prefix)?
        .parse::<usize>()
        .ok()
        .filter(|i| (1..=count).contains(i))
        .map(|i| i - 1)
}
struct Controls {
    simulated: bool,
    encoders: [u64; 8],
    positive: [bool; 8],
    negative: [bool; 8],
    totals: [f64; 8],
    top: [u64; 8],
    groups: [u64; 8],
    pads: [u64; 16],
    last: String,
}
impl Controls {
    fn new(simulated: bool) -> Self {
        Self {
            simulated,
            encoders: [0; 8],
            positive: [false; 8],
            negative: [false; 8],
            totals: [0.0; 8],
            top: [0; 8],
            groups: [0; 8],
            pads: [0; 16],
            last: "Turn every encoder both ways; press top/group buttons and all pads".into(),
        }
    }
    fn encoder(&mut self, name: &str, delta: f64) -> bool {
        let Some(i) = index(name, "screen_encoder_", 8) else {
            return false;
        };
        if !delta.is_finite() || delta == 0.0 {
            return false;
        }
        self.encoders[i] = self.encoders[i].saturating_add(1);
        self.positive[i] |= delta > 0.0;
        self.negative[i] |= delta < 0.0;
        self.totals[i] = (self.totals[i] + delta).clamp(-1e6, 1e6);
        self.last = format!("ENCODER {} {delta:+.4}", i + 1);
        true
    }
    fn button(&mut self, name: &str) -> bool {
        if let Some(i) = index(name, "top_", 8) {
            self.top[i] = self.top[i].saturating_add(1);
        } else if let Some(i) = name.strip_prefix("group_").and_then(|s| {
            ["a", "b", "c", "d", "e", "f", "g", "h"]
                .iter()
                .position(|n| *n == s)
        }) {
            self.groups[i] = self.groups[i].saturating_add(1);
        } else {
            return self.pad(name);
        }
        self.last = format!("BUTTON {name}");
        true
    }
    fn pad(&mut self, name: &str) -> bool {
        let Some(i) = index(name, "pad_", 16) else {
            return false;
        };
        self.pads[i] = self.pads[i].saturating_add(1);
        self.last = format!("PAD {} PRESSURE/PRESS EVENT", i + 1);
        true
    }
    fn snapshot(&self) -> Value {
        json!({"mode":if self.simulated {"SIMULATED INPUT"} else {"REAL MK3 INPUT"},"encoders":self.encoders,"positive":self.positive,"negative":self.negative,"totals":self.totals,"top":self.top,"groups":self.groups,"pads":self.pads,"last":self.last,"visualChecks":"User must verify colors, orientation, clipping, LEDs and cleanup","engine":"NO ENGINE"})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use encdr::DeviceId;
    #[test]
    fn real_session_rejects_simulation_and_ignores_release() {
        let mut p = Panel::new(true, false);
        assert!(!p.command("turn 1 1"));
        assert!(!p.command("pad 1"));
        assert!(!p.event(&Event::Button {
            device: DeviceId::default(),
            name: "pad_1",
            pressed: false
        }));
        assert_eq!(p.snapshot()["pads"][0], 0);
        assert!(p.event(&Event::Button {
            device: DeviceId::default(),
            name: "pad_16",
            pressed: true
        }));
        assert_eq!(p.snapshot()["pads"][15], 1);
    }
    #[test]
    fn every_control_and_direction_is_tracked_independently() {
        let mut p = Panel::new(true, true);
        for i in 1..=8 {
            assert!(p.command(&format!("turn {i} 0.01")));
            assert!(p.command(&format!("turn {i} -0.01")));
            assert!(p.command(&format!("button top_{i}")));
        }
        for i in 1..=16 {
            assert!(p.command(&format!("pad {i}")));
        }
        for name in ["a", "b", "c", "d", "e", "f", "g", "h"] {
            assert!(p.command(&format!("button group_{name}")));
        }
        let s = p.snapshot();
        for i in 0..8 {
            assert_eq!(s["encoders"][i], 2);
            assert_eq!(s["positive"][i], true);
            assert_eq!(s["negative"][i], true);
            assert_eq!(s["top"][i], 1);
            assert_eq!(s["groups"][i], 1);
        }
        assert!(s["pads"].as_array().unwrap().iter().all(|n| n == 1));
    }
    #[test]
    fn invalid_controls_do_not_count() {
        let mut p = Panel::new(true, true);
        let before = p.snapshot();
        for c in [
            "turn 0 1",
            "turn 9 1",
            "turn 1 NaN",
            "turn 1 0",
            "pad 17",
            "pad 0",
            "button group_z",
            "button top_9",
        ] {
            assert!(!p.command(c));
        }
        assert_eq!(p.snapshot(), before);
    }
}
