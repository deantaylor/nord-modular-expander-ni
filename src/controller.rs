//! Device-specific names end here; the G1 client uses the same controls for either panel.
use encdr::{DeviceId, Event};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Controller {
    Maschine,
    #[default]
    Komplete,
}
impl Controller {
    pub fn product(self) -> u16 {
        match self {
            Self::Maschine => 0x1600,
            Self::Komplete => 0x1610,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Maschine => "Maschine MK3",
            Self::Komplete => "Komplete Kontrol S49 Mk2",
        }
    }
    pub fn accepts(self, id: DeviceId) -> bool {
        id.vendor_id() == 0x17cc && id.product_id() == self.product()
    }
    pub fn input(self, event: Event) -> Option<Event> {
        if self == Self::Maschine {
            return Some(match event {
                Event::Button { device, name, pressed } => Event::Button {
                    device,
                    name: Self::maschine_button(name)?,
                    pressed,
                },
                event => event,
            });
        }
        match event {
            Event::Encoder {
                device,
                name: "encoder_main",
                delta,
            } => Some(Event::Encoder {
                device,
                name: "main_encoder",
                delta,
            }),
            Event::EncoderFine {
                device,
                name,
                delta,
            } => Self::knob(name).map(|name| Event::EncoderFine {
                device,
                name,
                delta,
            }),
            Event::Button {
                device,
                name,
                pressed,
            } => Self::button(name).map(|name| Event::Button {
                device,
                name,
                pressed,
            }),
            event @ (Event::DeviceConnected { .. } | Event::DeviceDisconnected { .. }) => {
                Some(event)
            }
            _ => None,
        }
    }
    fn knob(name: &'static str) -> Option<&'static str> {
        Some(match name {
            "encoder_1" => "screen_encoder_1",
            "encoder_2" => "screen_encoder_2",
            "encoder_3" => "screen_encoder_3",
            "encoder_4" => "screen_encoder_4",
            "encoder_5" => "screen_encoder_5",
            "encoder_6" => "screen_encoder_6",
            "encoder_7" => "screen_encoder_7",
            "encoder_8" => "screen_encoder_8",
            _ => return None,
        })
    }
    fn button(name: &'static str) -> Option<&'static str> {
        Some(match name {
            "pattern" | "clear" | "shift" | "track" | "browser" | "mixer" | "top_1" | "top_2" | "top_3" | "top_4" | "top_5" | "top_7" | "top_8" => name,
            "plugin" => "module_edit",
            "preset_up" => "arrow_left",
            "preset_down" => "arrow_right",
            "page_left" => "knob_page_previous",
            "page_right" => "knob_page_next",
            "encoder_main_press" => "browser_select",
            "encoder_main_left" => "encoder_left",
            "encoder_main_right" => "encoder_right",
            "encoder_main_up" => "encoder_up",
            "encoder_main_down" => "encoder_down",
            _ => return None,
        })
    }
    fn maschine_button(name: &str) -> Option<&str> {
        Some(match name {
            "channel" => "track",
            "plugin" => "module_edit",
            "erase" => "clear",
            "encoder_press" => "browser_select",
            "arranger" => "pattern",
            "pattern" | "macro" => return None,
            "arrow_left" => "page_or_preset_previous",
            "arrow_right" => "page_or_preset_next",
            _ => name,
        })
    }
    pub fn owns_led(self, name: &str) -> bool {
        self == Self::Maschine
            || matches!(
                name,
                "pattern" | "clear" | "shift" | "track" | "browser" | "mixer" | "top_1"
                    | "top_2"
                    | "top_3"
                    | "top_4"
                    | "top_5"
                    | "top_7"
                    | "top_8"
                    | "plugin"
                    | "page_left"
                    | "page_right"
                    | "preset_up"
                    | "preset_down"
            )
    }
    /// Indexed Mk2 display lamps use NI's hardware palette, not RGB screen colours.
    pub fn slot_light(self, slot:usize, level:u8)->encdr::LedValue {
        if level==0 || slot>=4{return encdr::LedValue::Off;}
        encdr::LedValue::Single(([1,7,3,5][slot]<<2)|if level<80{0}else{2})
    }
    pub fn button_light(self, name: &str, level: u8) -> encdr::LedValue {
        if self == Self::Maschine && name.starts_with("group_") && level > 0 {
            let green = if level < 80 { 64 } else { 255 };
            return encdr::LedValue::Rgb { r: 0, g: green, b: green / 3 };
        }
        if self==Self::Komplete && name.starts_with("top_") && level>0 {
            return encdr::LedValue::Single((17<<2)|if level<80{0}else{2});
        }
        if level == 0 { encdr::LedValue::Off }
        else if level < 80 { encdr::LedValue::Dim }
        else { encdr::LedValue::Bright }
    }
    pub fn led_source(self, name: &str) -> Option<&str> {
        if self == Self::Maschine {
            return Self::maschine_button(name);
        }
        Some(match name {
            "page_left" => "knob_page_previous",
            "page_right" => "knob_page_next",
            "preset_up" => "arrow_left",
            "preset_down" => "arrow_right",
            "plugin" => "module_edit",
            _ => name,
        })
    }
}

/// Local NI menus need an explicit handover back before our surface can resume.
#[derive(Default)]
pub struct NativeModeGate {
    paused: std::sync::atomic::AtomicBool,
    saw_handover: std::sync::atomic::AtomicBool,
    shift_held: std::sync::atomic::AtomicBool,
    plugin_held: std::sync::atomic::AtomicBool,
}
impl NativeModeGate {
    pub fn maschine_input_allowed(&self, event: &Event) -> bool {
        use std::sync::atomic::Ordering::SeqCst;
        match event {
            Event::Button { name: "shift", pressed, .. } => self.shift_held.store(*pressed, SeqCst),
            Event::DeviceDisconnected { .. } => self.shift_held.store(false, SeqCst),
            Event::Button { name: "channel" | "plugin", pressed: true, .. } => {
                // These shifted buttons open NI's MIDI and instance menus.
                return !self.shift_held.load(SeqCst);
            }
            _ => {}
        }
        true
    }
    pub fn note_input(&self, event: &Event) {
        use std::sync::atomic::Ordering::SeqCst;
        match event {
            Event::Button{name:"shift",pressed,..}=>self.shift_held.store(*pressed,SeqCst),
            Event::Button{name:"plugin",pressed,..}=>self.plugin_held.store(*pressed,SeqCst),
            _=>{}
        }
        let native_midi_chord=self.shift_held.load(SeqCst)&&self.plugin_held.load(SeqCst);
        if native_midi_chord || matches!(
            event,
            Event::Button {
                name: "setup" | "midi" | "instance",
                pressed: true,
                ..
            }
        ) {
            self.saw_handover.store(false, SeqCst);
            self.paused.store(true, SeqCst);
        } else if matches!(event, Event::DeviceDisconnected { .. }) {
            self.paused.store(false, SeqCst);
            self.saw_handover.store(false, SeqCst);
            self.shift_held.store(false,SeqCst);self.plugin_held.store(false,SeqCst);
        }
    }
    pub fn paused(&self) -> bool {
        self.paused.load(std::sync::atomic::Ordering::SeqCst)
    }
    pub fn note_control(&self, owned: bool) -> bool {
        use std::sync::atomic::Ordering::SeqCst;
        if !owned{self.shift_held.store(false,SeqCst);self.plugin_held.store(false,SeqCst);}
        if self.paused() {
            if !owned {
                self.saw_handover.store(true, SeqCst);
            } else if self.saw_handover.swap(false, SeqCst) {
                self.paused.store(false, SeqCst);
            }
        }
        owned && !self.paused()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maschine_native_chords_leave_plain_channel_and_plugin_available() {
        let gate = NativeModeGate::default();
        let button = |name, pressed| Event::Button { device: DeviceId::default(), name, pressed };
        for name in ["channel", "plugin"] {
            assert!(gate.maschine_input_allowed(&button(name, true)));
            assert!(gate.maschine_input_allowed(&button("shift", true)));
            assert!(!gate.maschine_input_allowed(&button(name, true)));
            assert!(gate.maschine_input_allowed(&button(name, false)));
            assert!(!gate.note_control(false));
            assert!(gate.maschine_input_allowed(&button(name, true)));
        }
    }
    #[test]
    fn maschine_buttons_follow_the_pinned_descriptor_and_drive_matching_lamps() {
        use encdr::device::{loader::DescriptorRegistry, parser::PacketParser};
        let mut registry = DescriptorRegistry::new();
        registry.load_builtins().unwrap();
        let descriptor = registry.find(0x17cc, 0x1600).unwrap().clone();
        let device = DeviceId::default();
        let mut parser = PacketParser::new(device, &descriptor, registry.intern_descriptor_names(&descriptor));
        let mut report = [0; 42];
        report[0] = 1;
        let mut events = Vec::new();
        parser.parse_from("control", &report, &mut events);

        for (physical, action, byte, mask) in [
            ("channel", "track", 8, 0x01),
            ("plugin", "module_edit", 7, 0x20),
            ("erase", "clear", 6, 0x04),
            ("encoder_press", "browser_select", 1, 0x01),
            ("arranger", "pattern", 8, 0x02),
            ("arrow_left", "page_or_preset_previous", 8, 0x08),
            ("arrow_right", "page_or_preset_next", 7, 0x04),
        ] {
            for pressed in [true, false] {
                report[byte] = if pressed { mask } else { 0 };
                events.clear();
                parser.parse_from("control", &report, &mut events);
                assert!(events.drain(..).filter_map(|event| Controller::Maschine.input(event))
                    .any(|event| matches!(event, Event::Button {name, pressed: state, ..} if name == action && state == pressed)), "{physical} {pressed}");
            }
            if physical != "encoder_press" {
                assert_eq!(Controller::Maschine.led_source(physical), Some(action));
            }
        }
        for (inactive, byte, mask) in [("pattern", 4, 0x40), ("macro", 7, 0x01)] {
            report[byte] = mask;
            events.clear();
            parser.parse_from("control", &report, &mut events);
            let press = events.drain(..).find(|event|
                matches!(event, Event::Button { name, pressed: true, .. } if *name == inactive)
            ).unwrap();
            assert!(Controller::Maschine.input(press).is_none());
            assert_eq!(Controller::Maschine.led_source(inactive), None);
            report[byte] = 0;
            parser.parse_from("control", &report, &mut events);
        }
    }
    #[test]
    fn maschine_leds_use_nhl2_brightness_and_indexed_group_intensity() {
        use encdr::device::{led_builder::LedBuilder, loader::DescriptorRegistry};
        let mut registry = DescriptorRegistry::new();
        registry.load_builtins().unwrap();
        let descriptor = registry.find(0x17cc, 0x1600).unwrap();
        let mut leds = LedBuilder::new(&descriptor.leds[0], &descriptor.interfaces[0]);

        for (level, button_byte, group_byte) in [(0, 0, 0), (30, 228, 33), (50, 228, 33), (80, 158, 35)] {
            assert!(leds.set("arranger", Controller::Maschine.button_light("arranger", level)));
            assert!(leds.set("group_a", Controller::Maschine.button_light("group_a", level)));
            let report = leds.flush().unwrap();
            assert_eq!(report[3], button_byte);
            assert_eq!(report[30], group_byte);
        }
    }
    #[test]
    fn display_settings_button_and_slot_palette(){
        assert!(matches!(Controller::Komplete.input(Event::Button{device:DeviceId::default(),name:"top_8",pressed:true}),Some(Event::Button{name:"top_8",..})));
        assert!(Controller::Komplete.owns_led("top_8"));
        for (slot,palette) in [1,7,3,5].into_iter().enumerate(){
            assert_eq!(Controller::Komplete.slot_light(slot,30),encdr::LedValue::Single(palette<<2|0));
            assert_eq!(Controller::Komplete.slot_light(slot,80),encdr::LedValue::Single(palette<<2|2));
            assert_eq!(Controller::Komplete.slot_light(slot,0),encdr::LedValue::Off);
        }
    }
    #[test]
    fn clear_is_a_mapped_available_action(){
        assert!(matches!(Controller::Komplete.input(Event::Button{device:DeviceId::default(),name:"clear",pressed:true}),Some(Event::Button{name:"clear",pressed:true,..})));
        assert!(Controller::Komplete.owns_led("clear"));
    }
    #[test]
    fn native_and_unassigned_controls_cannot_trigger_synth_actions() {
        for name in [
            "setup", "midi", "instance", "play", "stop", "rec",
            "restart", "metro", "scale", "arp", "mute", "solo", "scene",
            "top_6",
        ] {
            assert!(
                Controller::Komplete
                    .input(Event::Button {
                        device: DeviceId::default(),
                        name,
                        pressed: true
                    })
                    .is_none(),
                "{name}"
            );
            assert!(!Controller::Komplete.owns_led(name), "{name}");
        }
        assert!(!Controller::Komplete.owns_led("light_guide"));
    }
    #[test]
    fn local_setup_and_midi_pause_until_explicit_ni_handover() {
        let gate = super::NativeModeGate::default();
        assert!(gate.note_control(true));
        for name in ["setup", "midi"] {
            gate.note_input(&Event::Button {
                device: DeviceId::default(),
                name,
                pressed: true,
            });
            assert!(gate.paused());
            assert!(!gate.note_control(true));
            assert!(!gate.note_control(false));
            assert!(gate.note_control(true));
            assert!(!gate.paused());
        }
    }
    #[test]
    fn native_shift_plugin_chord_is_reserved_in_both_press_orders(){
        for order in [["shift","plugin"],["plugin","shift"]]{
            let gate=NativeModeGate::default();let device=DeviceId::default();
            gate.note_input(&Event::Button{device,name:order[0],pressed:true});assert!(!gate.paused());
            gate.note_input(&Event::Button{device,name:order[1],pressed:true});assert!(gate.paused());assert!(!gate.note_control(true));
            gate.note_input(&Event::Button{device,name:order[0],pressed:false});assert!(gate.paused(),"release must not steal native mode back");
            assert!(!gate.note_control(false));assert!(gate.note_control(true));
            gate.note_input(&Event::Button{device,name:"plugin",pressed:true});assert!(!gate.paused(),"stale Shift must be cleared on handover");
        }
    }
    #[test]
    fn former_dial_bit_produces_no_function_event() {
        use encdr::device::{loader::DescriptorRegistry, parser::PacketParser};
        let mut registry = DescriptorRegistry::new();
        registry.load_builtins().unwrap();
        let desc = registry.find(0x17cc, 0x1610).unwrap().clone();
        let mut parser = PacketParser::new(
            DeviceId::default(),
            &desc,
            registry.intern_descriptor_names(&desc),
        );
        let mut report = [0u8; 32];
        report[0] = 1;
        let mut events = Vec::new();
        parser.parse_from("control", &report, &mut events);
        report[6] = 4;
        parser.parse_from("control", &report, &mut events);
        assert!(events.is_empty(), "{events:?}");
    }
    #[test]
    fn captured_mixer_button_and_calibrated_lamp_are_mixer_not_plugin(){
        use encdr::device::{loader::DescriptorRegistry,parser::PacketParser};let mut r=DescriptorRegistry::new();r.load_builtins().unwrap();let d=r.find(0x17cc,0x1610).unwrap().clone();let mut p=PacketParser::new(DeviceId::default(),&d,r.intern_descriptor_names(&d));let mut report=[0;32];report[0]=1;let mut e=vec![];p.parse_from("control",&report,&mut e);
        report[5]=1;p.parse_from("control",&report,&mut e);assert!(e.iter().any(|e|matches!(Controller::Komplete.input(e.clone()),Some(Event::Button{name:"mixer",pressed:true,..}))));assert!(Controller::Komplete.owns_led("mixer"));assert_eq!(Controller::Komplete.led_source("mixer"),Some("mixer"));
    }
    #[test]
    fn corrected_s49_lamps_use_indexed_display_colours_and_verified_panel_positions(){
        use encdr::device::{loader::DescriptorRegistry,led_builder::LedBuilder};
        let mut registry=DescriptorRegistry::new();registry.load_builtins().unwrap();let desc=registry.find(0x17cc,0x1610).unwrap();
        let mut leds=LedBuilder::new(&desc.leds[0],&desc.interfaces[0]);
        for (name,offset) in [("shift",14),("track",23),("page_left",32),("page_right",33),("browser",35),("plugin",36)]{
            leds.clear();assert!(leds.set(name,encdr::LedValue::Bright));let wire=leds.flush().unwrap();assert_eq!(wire.len(),80);assert_eq!(wire[0],0x80);assert_eq!(wire[offset+1],158);if name!="track"{assert_eq!(wire[24],0,"other controls must not drive TRACK");}assert_eq!(wire[29],0,"navigation must not drive KEY MODE");
        }
        for i in 1..=8 {
            leds.clear();let name=format!("top_{}",i);assert!(leds.set(&name,encdr::LedValue::Bright));let wire=leds.flush().unwrap();assert_eq!(wire[i+2],71,"display buttons use WHITE palette, not monochrome158/GREEN");
            leds.set(&name,encdr::LedValue::Dim);assert_eq!(leds.flush().unwrap()[i+2],69);
        }
        assert_eq!(Controller::Komplete.led_source("plugin"),Some("module_edit"));assert!(Controller::Komplete.owns_led("top_5"));
    }
    #[test]
    fn captured_shift_and_track_are_separate_press_release_controls(){
        use encdr::device::{loader::DescriptorRegistry,parser::PacketParser};
        let mut registry=DescriptorRegistry::new();registry.load_builtins().unwrap();let desc=registry.find(0x17cc,0x1610).unwrap().clone();
        let mut parser=PacketParser::new(DeviceId::default(),&desc,registry.intern_descriptor_names(&desc));let mut report=[0u8;32];report[0]=1;let mut events=vec![];parser.parse_from("control",&report,&mut events);
        for (byte,mask,expected) in [(2,0x80,"shift"),(4,0x10,"track")] {
            for pressed in [true,false] {events.clear();report[byte]=if pressed{mask}else{0};parser.parse_from("control",&report,&mut events);
                assert_eq!(events.len(),1,"{events:?}");assert!(matches!(&events[0],Event::Button{name,pressed:p,..} if *name==expected&&*p==pressed));assert!(Controller::Komplete.input(events[0].clone()).is_some());
            }
        }
    }
    #[test]
    fn komplete_mk2_captured_native_report_decodes_all_knobs_and_resyncs() {
        use encdr::device::{loader::DescriptorRegistry, parser::PacketParser};
        let mut registry = DescriptorRegistry::new();
        registry.load_builtins().unwrap();
        let descriptor = registry.find(0x17cc, 0x1610).unwrap().clone();
        let names = registry.intern_descriptor_names(&descriptor);
        let mut parser = PacketParser::new(DeviceId::default(), &descriptor, names);
        // Captured from this user's S49 Mk2 in native/NI-service mode.
        let mut report = [
            0x01, 0, 0, 0, 0, 0, 0, 0x08, 0, 0, 0x0e, 0x01, 0xdf, 0x03, 0x49, 0x03, 0xfd, 0, 0xa7,
            0x03, 0x5d, 0x01, 0xc2, 0x02, 0x3a, 0x01, 0, 0, 0, 0, 0x03, 0x24,
        ];
        let mut events = Vec::new();
        parser.parse_from("control", &report, &mut events);
        assert!(events.is_empty());
        // Next actual report: only encoder 5 moves, 935 -> 926.
        report[18] = 0x9e;
        parser.parse_from("control", &report, &mut events);
        assert!(
            matches!(events.as_slice(),[Event::EncoderFine {name:"encoder_5",delta,..}] if (*delta+0.009).abs()<1e-6)
        );
        for i in 0..8 {
            events.clear();
            let byte = 10 + i * 2;
            let raw = u16::from_le_bytes([report[byte], report[byte + 1]]);
            report[byte..byte + 2].copy_from_slice(&((raw + 20) % 1000).to_le_bytes());
            parser.parse_from("control", &report, &mut events);
            assert!(
                matches!(events.as_slice(),[Event::EncoderFine {name,delta,..}] if *name==format!("encoder_{}",i+1) && (*delta-0.020).abs()<1e-6)
            );
        }
        events.clear();
        parser.set_active(false, &mut events);
        report[10..12].copy_from_slice(&999u16.to_le_bytes());
        parser.parse_from("control", &report, &mut events);
        assert!(events.is_empty());
        parser.set_active(true, &mut events);
        report[10..12].copy_from_slice(&0u16.to_le_bytes());
        parser.parse_from("control", &report, &mut events);
        assert!(
            matches!(events.as_slice(),[Event::EncoderFine {name:"encoder_1",delta,..}] if (*delta-0.001).abs()<1e-6)
        );
        events.clear();
        report[3] = 1;
        parser.parse_from("control", &report, &mut events);
        assert!(matches!(
            events.as_slice(),
            [Event::Button {
                name: "stop",
                pressed: true,
                ..
            }]
        ));
    }
    #[test]
    fn komplete_browser_presets_and_page_arrows_decode_to_their_actions() {
        use encdr::device::{loader::DescriptorRegistry, parser::PacketParser};
        let mut registry = DescriptorRegistry::new();
        registry.load_builtins().unwrap();
        let desc = registry.find(0x17cc, 0x1610).unwrap().clone();
        let mut parser = PacketParser::new(
            DeviceId::default(),
            &desc,
            registry.intern_descriptor_names(&desc),
        );
        let mut report = [0u8; 32];
        report[0] = 1;
        let mut events = Vec::new();
        parser.parse_from("control", &report, &mut events);
        for (byte, mask, expected) in [
            (5, 0x04, "browser"),
            (3, 0x10, "arrow_left"),
            (3, 0x40, "arrow_right"),
            (3, 0x80, "knob_page_previous"),
            (3, 0x20, "knob_page_next"),
        ] {
            events.clear();
            report[byte] = mask;
            parser.parse_from("control", &report, &mut events);
            assert!(
                matches!(events.as_slice(), [Event::Button { pressed: true, .. }]),
                "{events:?}"
            );
            assert!(
                matches!(Controller::Komplete.input(events[0].clone()),Some(Event::Button {name,pressed:true,..}) if name==expected)
            );
            events.clear();
            report[byte] = 0;
            parser.parse_from("control", &report, &mut events);
            assert!(
                matches!(Controller::Komplete.input(events[0].clone()),Some(Event::Button {name,pressed:false,..}) if name==expected)
            );
        }
    }
    #[test]
    fn komplete_keeps_encoder_order_direction_and_release_edges() {
        let device = DeviceId::default();
        for (name, expected) in [
            ("encoder_1", "screen_encoder_1"),
            ("encoder_8", "screen_encoder_8"),
        ] {
            assert!(
                matches!(Controller::Komplete.input(Event::EncoderFine {device,name,delta:-0.02}), Some(Event::EncoderFine {name,delta,..}) if name==expected && delta == -0.02)
            );
        }
        assert!(matches!(Controller::Komplete.input(Event::Button {device,name:"plugin",pressed:false}),Some(Event::Button{name:"module_edit",pressed:false,..})));
        assert_eq!(Controller::Komplete.button_light("top_2",30),encdr::LedValue::Single(68));
        assert_eq!(Controller::Komplete.button_light("top_1",80),encdr::LedValue::Single(70));
        assert_eq!(Controller::Komplete.button_light("top_4",0),encdr::LedValue::Off);
        assert_eq!(Controller::Komplete.button_light("page_left",80),encdr::LedValue::Bright);
        assert!(matches!(
            Controller::Maschine.input(Event::Encoder {
                device,
                name: "main_encoder",
                delta: 1
            }),
            Some(Event::Encoder {
                name: "main_encoder",
                delta: 1,
                ..
            })
        ));
    }
}
