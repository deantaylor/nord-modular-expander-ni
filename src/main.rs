//! Maschine front panel with an optional real G1-Emu bridge and developer fixtures.
mod browser_state;
mod connection;
mod controller;
mod engine;
mod g1;
mod hardware_test;
mod input_dispatch;
mod model;
mod options;
mod pad_midi;
mod patch_browser;
mod project;
mod session;
mod sequencer;
mod storage;
use connection::Link;
use controller::Controller;
use encdr::{DeviceId, Encdr, EncdrConfig, Event, LedValue};
use encdr_view::{ScreenContent, ScreenView};
use options::Options;
use session::Session;
use std::error::Error;
use std::io::{self, BufRead};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}
const HELP: &str = "Modular Machine Keyboard — G1-Emu controller / developer fixtures (Encdr 0.8.5)
Usage: cargo run -- [--mock | --diagnose] [--visible] [--led-test] [--hardware-test] [--state-file FILE | --no-persist] [--g1]
  --mock       Desktop front panel without MK3; --g1 can still control the real emulator
  --diagnose   Report USB enumeration, NI processes, and MK3 connection status
  --controller maschine|komplete  Select Maschine MK3 or Komplete S49 Mk2 (default)
  --visible    Also show hardware screen previews on desktop
  --g1         Connect to local G1-Emu control bridge (127.0.0.1:47410)
  --g1-address ADDRESS  Override loopback bridge address
  --patch-dir FOLDER  Local .pch browser library (default: project patches/)
  --led-test   Light 16 pads and play button, cleared on clean exit
  --state-file FILE  Optional developer fixture JSON; emulator patches will stay with the emulator
  --no-persist  Disable developer-fixture persistence
  --hardware-test  Screen patterns and input counters; includes LED test on hardware
Terminal commands (press Enter):
  turn 1 0.05       Simulate encoder 1 (+5%; encoders 1–8)
  button top_1     Select page (top_1–3); slot (top_5–8); top_4 cycles presets
  button group_a   Select module (groups A–H); 4-way encoder navigates modules/pages
  preset warm_pad / module osc / page 2   Mock fixture navigation
  load-patch PATH.pch / refresh-presets   G1-Emu patch commands
  G1 mode: BROWSER opens files; main encoder scrolls/press loads; left goes up
  Browser: TOP 1 files / TOP 2 banks / TOP 3 refresh / TOP 4 load
  Maschine: GROUP A–D select slots; CHANNEL returns Home
  Maschine: < > change knob pages; hold SHIFT for presets; ARRANGER opens Sequencers
  Maschine: SETTINGS opens emulator settings; ERASE restores changed slot parameters
  Komplete: TRACK returns Home; hold TRACK + TOP 1–4 for slots; CLEAR restores changes
  PLUG-IN opens Module Edit; MIXER edits patch output levels; SHIFT held = Function
  set filter cutoff 2000                 Developer fixture parameter update
  save FILE.json / load FILE.json        Export/import project-owned JSON
  inspect          Log mock WebView DOM and layout diagnostics
  report           Print input counters in hardware test mode
  pad 16           Simulate a diagnostic pad in --mock --hardware-test
  pad 1 0.7 / pad 1 0   G1 MIDI strike/release (mock mode only)
  G1 PAD MODE = drums; KEYBOARD = chromatic; STOP = release owned pad notes
  disconnect / reconnect / handover / takeover   Mock connection checks
  quit             Clean exit (Ctrl+C also supported)
Keep NIHardwareAgent / NIHostIntegrationAgent running on macOS MK3.
Use the controller instance button to hand control to Encdr.
Hardware mode waits for MK3 and reconnects automatically; quit stays available.";

fn backend(auto_hotplug: bool, controller: Controller) -> encdr::Result<Encdr> {
    Encdr::new(EncdrConfig {
        auto_hotplug,
        supported_controllers: Some((0x17cc, controller.product()).into()),
        ..Default::default()
    })
}
fn find_controller(encdr: &Encdr, controller: Controller) -> Option<DeviceId> {
    encdr
        .connected_devices()
        .into_iter()
        .filter(|id| controller.accepts(*id))
        .min_by_key(|id| id.to_string())
}
fn diagnose(controller: Controller) -> Result<(), Box<dyn Error>> {
    println!(
        "Encdr / encdr-view 0.8.5; host {} / {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    #[cfg(target_os = "macos")]
    {
        for (cmd, args) in [
            (
                "system_profiler",
                vec!["SPUSBDataType", "-detailLevel", "mini"],
            ),
            (
                "pgrep",
                vec!["-lf", "NIHardwareAgent|NIHostIntegrationAgent"],
            ),
        ] {
            match std::process::Command::new(cmd).args(args).output() {
                Ok(out) => println!(
                    "{cmd} ({}):\n{}{}",
                    out.status,
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                ),
                Err(e) => eprintln!("{cmd}: {e}"),
            }
        }
    }
    let mut encdr = backend(false, controller)?;
    encdr.scan()?;
    if let Some(id) = find_controller(&encdr, controller) {
        println!(
            "{} {id}: connected={}, control={}, screen backlog={}",
            controller.label(),
            encdr.is_connected(id),
            encdr.has_control(id),
            encdr.screen_backlog(id)
        );
        println!(
            "Connection is not a screen/encoder validation. Run with --visible to test displays."
        );
    } else {
        println!("No connected {} 17cc:{:04x}. Check USB/power, NI services and instance selection. No services were changed.", controller.label(), controller.product());
    }
    encdr.clean_exit();
    Ok(())
}

fn screen_html(side: &str, test: bool, live: bool) -> String {
    screen_html_with_role(side, side, test, live)
}

fn screen_html_with_role(side: &str, role: &str, test: bool, live: bool) -> String {
    if test {
        hardware_test::HTML.replace("__SIDE__", side)
    } else if live {
        use base64::Engine;
        static FONT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        let font = FONT.get_or_init(|| {
            base64::engine::general_purpose::STANDARD
                .encode(include_bytes!("../screens/fonts/Inter-Variable.ttf"))
        });
        include_str!("../screens/g1.html")
            .replace("__SIDE__", side)
            .replace("__PRIMARY__", role)
            .replace("__INTER_FONT__", font)
            .replace("__MODULE_VISUALS__",include_str!("../screens/module-visuals.js"))
            .replace("__MIDI_BLOOM__",include_str!("../screens/midi-bloom.js"))
    } else if side == "left" {
        include_str!("../screens/left.html").into()
    } else {
        include_str!("../screens/right.html").into()
    }
}

fn run_ui_ticks(
    mut tick: impl FnMut() -> Result<bool, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let mut error = None;
    ScreenView::run_event_loop(|| match tick() {
        Ok(keep_running) => keep_running,
        Err(e) => {
            error = Some(e);
            false
        }
    });
    STOP.store(true, Ordering::Relaxed);
    match error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
#[cfg(target_os = "macos")]
fn run_mock(input: mpsc::Receiver<String>, options: &Options) -> Result<(), Box<dyn Error>> {
    use encdr_view::{bridge::build_send_js, webview_macos::ManagedWebView};
    let test = options.test;
    let mut state = Session::new(
        test,
        true,
        options.state_file.clone(),
        options.g1,
        options.patch_dir.clone(),
    )?;
    let trace_frames = std::env::var_os("MK3_TRACE_FRAMES").is_some();
    let preview_visible = !trace_frames && std::env::var_os("MK3_HIDDEN_PREVIEW").is_none();
    let mut animation_refresh = Instant::now();
    let mut trace_captures: [encdr_view::capture_macos::AsyncCapture; 2] =
        std::array::from_fn(|_| Default::default());
    if options.g1.is_some() {
        trace_captures[0].require_ink = Some((58, 11, 362, 32));
        trace_captures[1].require_ink = Some((20, 11, 400, 32));
    }
    let left = ManagedWebView::new(
        480,
        272,
        &screen_html("left", test, options.g1.is_some()),
        preview_visible,
    )?;
    let right = ManagedWebView::new(
        480,
        272,
        &screen_html("right", test, options.g1.is_some()),
        preview_visible,
    )?;
    let mut link = Link::new();
    link.observe(Some(1_u8), true);
    state.set_connection(link.snapshot());
    struct PreviewCapture {
        folder: std::path::PathBuf,
        pending: [encdr_view::capture_macos::PendingSnapshot; 2],
        frames: [Option<Vec<u8>>; 2],
    }
    let mut preview_capture: Option<PreviewCapture> = None;
    let mut dirty = true;
    let mut refresh = Instant::now() - Duration::from_secs(1);
    let result = run_ui_ticks(|| {
        if STOP.load(Ordering::Relaxed) {
            return Ok(false);
        }
        if !left.is_page_ready() || !right.is_page_ready() {
            return Ok(true);
        }
        while let Ok(command) = input.try_recv() {
            if ["disconnect", "reconnect", "handover", "takeover"].contains(&command.trim()) {
                match command.trim() {
                    "disconnect" => {
                        link.observe(None, false);
                    }
                    "reconnect" => {
                        link.observe(Some(1), true);
                    }
                    "takeover" => {
                        link.observe(link.device(), true);
                    }
                    "handover" => {
                        link.observe(link.device(), false);
                    }
                    _ => unreachable!(),
                }
                state.set_connection(link.snapshot());
                dirty = true;
            } else if command.trim() == "simulate-script-error" {
                left.eval(
                    "window.encdr.onMessage=()=>{throw new Error('Injected renderer failure')}",
                )?;
                dirty = true;
            } else if command.trim() == "simulate-blank-ui" {
                left.eval("document.getElementById('browser').style.visibility='hidden';document.getElementById('title').style.visibility='hidden';window.encdr.requestCapture();")?;
            } else if command.trim() == "reload-ui" {
                left.recover()?;
                right.recover()?;
                println!("mock views reloading");
            } else if let Some(folder) = command.strip_prefix("capture-preview ") {
                let folder = std::path::Path::new(folder.trim());
                std::fs::create_dir_all(folder)?;
                if preview_capture.is_some() {
                    return Err("Preview capture already in progress".into());
                }
                preview_capture = Some(PreviewCapture {
                    folder: folder.to_path_buf(),
                    pending: [
                        encdr_view::capture_macos::PendingSnapshot::begin(&left.webview),
                        encdr_view::capture_macos::PendingSnapshot::begin(&right.webview),
                    ],
                    frames: [None, None],
                });
            } else if command.trim() == "bench-state" {
                let start=Instant::now();
                for _ in 0..2000 { std::hint::black_box(state.leds()); }
                println!("bench LEDs: {}",start.elapsed().as_micros());
            } else if command.trim() == "bench-sequence" {
                right.webview.evaluate_script_with_callback(
                    "(()=>{const original=data,sample=JSON.parse(JSON.stringify(data));let replaced=0;const start=performance.now();for(let i=0;i<500;i++){const before=document.querySelector('.sequence-step');sample.sequence.playhead=i%16;window.encdr.onMessage('state',sample);if(before!==document.querySelector('.sequence-step'))replaced++;void document.body.offsetHeight;}const ms=performance.now()-start;window.encdr.onMessage('state',original);return {ms,replaced,updates:500};})()",
                    |result|println!("bench sequence: {result}"),
                )?;
            } else if command.trim() == "inspect" {
                for (side, view) in [("left", &left), ("right", &right)] {
                    view.webview.evaluate_script_with_callback(
                        "({editAction:{text:document.querySelector('#edit-action')?.textContent,active:document.querySelector('#edit-action')?.classList.contains('active'),x:document.querySelector('#edit-action')?.getBoundingClientRect().x,y:document.querySelector('#edit-action')?.getBoundingClientRect().y},supportingText:[...document.querySelectorAll('.sequence-help,.sequence-footer,#function,#notice')].filter(e=>getComputedStyle(e).display!=='none'&&Number(getComputedStyle(e).opacity)>0&&e.innerText).map(e=>e.innerText),sequenceVisible:!document.querySelector('#sequence-panel')?.classList.contains('hidden'),sequenceDots:[...document.querySelectorAll('.sequence-dot')].map(e=>({playhead:e.classList.contains('playhead'),on:e.classList.contains('on'),colour:getComputedStyle(e).backgroundColor})),sequenceCells:[...document.querySelectorAll('.sequence-step')].map(e=>({selected:e.classList.contains('selected'),bottom:e.getBoundingClientRect().bottom,text:e.innerText})),pageTitle:document.querySelector('#counter')?.innerText,saverUiOpacity:getComputedStyle(document.querySelector('header')).opacity,effectiveGlitch:typeof MidiBloom!=='undefined'?MidiBloom.mode(data,Date.now()).intensity:0,effectiveTheme:document.documentElement.dataset.effectiveTheme,bloomVisible:!document.querySelector('#midi-bloom')?.classList.contains('hidden'),bloomFrame:Number(document.querySelector('#midi-bloom')?.dataset.frame||0),bloomAlertSequence:document.querySelector('#midi-bloom')?.dataset.alertSequence||'',bloomAlertPhase:Number(document.querySelector('#midi-bloom')?.dataset.alertPhase||0),bloomAlertCount:Number(document.querySelector('#midi-bloom')?.dataset.alertCount||0),bloomAlert:document.querySelector('#midi-bloom')?.dataset.alert==='true',bloomEnergy:Number(document.querySelector('#midi-bloom')?.dataset.energy||0),selectedNameCount:document.querySelectorAll('.selected-name').length,moduleVisual:document.querySelector('#upper svg')?.getAttribute('aria-label'),visualPaths:[...document.querySelectorAll('#upper .module-curve')].map(p=>p.getAttribute('d')),visualBox:document.querySelector('#upper')?.getBoundingClientRect().toJSON(),text:document.body.innerText,width:document.body.getBoundingClientRect().width,height:document.body.getBoundingClientRect().height,scrollWidth:document.body.scrollWidth,scrollHeight:document.body.scrollHeight,bridge:typeof window.encdr.requestCapture,font:document.fonts.check('24px Inter'),browserOpen:!!document.querySelector('#browser:not(.hidden)'),knobsHidden:!!document.querySelector('#knobs')?.classList.contains('hidden'),scrollPosition:typeof scrollPosition==='number'?scrollPosition:0,scrollTarget:typeof scrollTarget==='number'?scrollTarget:0,fontFaces:[...document.fonts].map(f=>({family:f.family,status:f.status})),family:getComputedStyle(document.body).fontFamily,background:getComputedStyle(document.body).backgroundColor,animationUntil:typeof animationUntil==='number'?animationUntil:0,now:performance.now(),rows:[...document.querySelectorAll('.patch-row')].map(e=>({bottom:e.getBoundingClientRect().bottom,top:e.getBoundingClientRect().top,text:e.innerText,name:e.dataset.name,index:e.dataset.index,transform:getComputedStyle(e).transform,opacity:getComputedStyle(e).opacity})),cells:[...document.querySelectorAll('.cell')].map(e=>({bottom:e.getBoundingClientRect().bottom,text:e.innerText})),knobs:[...document.querySelectorAll('.knob')].map(e=>({bottom:e.getBoundingClientRect().bottom,top:e.getBoundingClientRect().top,left:e.getBoundingClientRect().left,encoder:e.dataset.encoder,numberVisible:getComputedStyle(e.querySelector('.knob-number')).display!=='none',moduleVisible:getComputedStyle(e.querySelector('.kmodule')).display!=='none',active:e.classList.contains('active'),value:e.innerText,display:e.querySelector('.kvalue').dataset.display,weight:getComputedStyle(e.querySelector('.kvalue')).fontWeight}))})",
                        move |result| println!("mock DOM {side}: {result}"),
                    )?;
                }
            } else if !link.accepts_input(1)
                && ["turn", "button", "pad"]
                    .contains(&command.split_whitespace().next().unwrap_or(""))
            {
                eprintln!("Mock input ignored while disconnected or another app has control");
            } else {
                dirty |= apply_command(&mut state, &command);
            }
        }
        dirty |= state.tick();
        left.replay_loaded();
        right.replay_loaded();
        if dirty && refresh.elapsed() >= Duration::from_millis(16) {
            let payload = state.snapshot();
            let js = build_send_js("state", &payload);
            left.eval(&js)?;
            right.eval(&js)?;
            println!("state: {payload}");
            dirty = false;
            refresh = Instant::now();
        }
        if !preview_visible
            && !trace_frames
            && animation_refresh.elapsed() >= Duration::from_millis(16)
        {
            for view in [&left, &right] {
                if view
                    .animation_until
                    .lock()
                    .is_ok_and(|until| Instant::now() < *until)
                {
                    view.eval(encdr_view::bridge::ADVANCE_JS)?;
                }
            }
            animation_refresh = Instant::now();
        }
        if trace_frames {
            for (index, (side, view)) in [("left", &left), ("right", &right)].iter().enumerate() {
                if view
                    .animation_until
                    .lock()
                    .is_ok_and(|until| Instant::now() < *until)
                {
                    *view.frame_ready.lock().unwrap() = true;
                }
                if let Some(result) =
                    trace_captures[index].poll(&view.webview, 480, 272, &view.frame_ready)
                {
                    let (_, _, rgba) = match result {
                        Ok(frame) => frame,
                        Err(error) => {
                            println!("mock capture recovery {side}: {error}");
                            view.recover()?;
                            continue;
                        }
                    };
                    let bands: Vec<_> = (0..7)
                        .map(|band| {
                            let mut ink = 0;
                            for y in (11 + band * 38)..(43 + band * 38).min(272) {
                                for x in 58..420 {
                                    let offset = (y * 480 + x) * 4;
                                    if rgba[offset] > 29
                                        && rgba[offset + 1] > 29
                                        && rgba[offset + 2] > 29
                                    {
                                        ink += 1;
                                    }
                                }
                            }
                            ink
                        })
                        .collect();
                    println!(
                        "mock frame {side}: {}",
                        serde_json::json!({"bands":bands,"frames":trace_captures[index].stats.frames,"captureMs":trace_captures[index].stats.last_ms,"cursor":state.snapshot()["browserCursor"]})
                    );
                }
            }
        }
        if let Some(capture) = &mut preview_capture {
            for index in 0..2 {
                if capture.frames[index].is_none() {
                    if let Some(result) = capture.pending[index].poll(480, 272) {
                        capture.frames[index] = Some(result.map_err(std::io::Error::other)?.2);
                    }
                }
            }
            if capture.frames.iter().all(Option::is_some) {
                for (index, side) in ["left", "right"].iter().enumerate() {
                    std::fs::write(
                        capture.folder.join(format!("{side}.rgba")),
                        capture.frames[index].as_ref().unwrap(),
                    )?;
                }
                println!("preview captured: {}", capture.folder.display());
                preview_capture = None;
            }
        }
        Ok(true)
    });
    let save = state.finish().map_err(Into::into);
    result.and(save)
}
#[cfg(not(target_os = "macos"))]
fn run_mock(_: mpsc::Receiver<String>, _: &Options) -> Result<(), Box<dyn Error>> {
    Err("Desktop mock currently supports macOS; portable state tests use cargo test".into())
}
fn apply_command(state: &mut Session, command: &str) -> bool {
    if command.trim() == "quit" {
        STOP.store(true, Ordering::Relaxed);
        return false;
    }
    if command.trim() == "report" {
        println!("report: {}", state.snapshot());
        return false;
    }
    match state.command(command) {
        Ok(changed) => changed,
        Err(error) => {
            eprintln!("{error}");
            false
        }
    }
}
fn set_test_leds(encdr: &Encdr, dev: DeviceId) {
    encdr.set_led(dev, "play", LedValue::Single(80));
    for i in 1..=16 {
        encdr.set_led(
            dev,
            &format!("pad_{i}"),
            match (i - 1) / 4 {
                0 => LedValue::Rgb { r: 80, g: 0, b: 0 },
                1 => LedValue::Rgb { r: 0, g: 80, b: 0 },
                2 => LedValue::Rgb { r: 0, g: 0, b: 80 },
                _ => LedValue::Rgb {
                    r: 60,
                    g: 60,
                    b: 60,
                },
            },
        );
    }
}
// Diagnostic only: one LED payload byte, through Encdr's existing strip writer.
fn s49_probe_frame(raw:&[u8])->Option<Vec<u8>>{
    if raw.len()>1024{return None;}
    let probe:serde_json::Value=serde_json::from_slice(raw).ok()?;let mut frame=vec![0;79];
    if probe["off"]==true{return Some(frame);}
    let offset=probe["offset"].as_u64()?;let value=probe["value"].as_u64()?;
    if offset>=79||value>255{return None;}frame[offset as usize]=value as u8;Some(frame)
}
fn set_g1_leds(encdr: &Encdr, dev: DeviceId, state: &serde_json::Value, controller: Controller) {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let names = NAMES.get_or_init(|| {
        let descriptor: serde_json::Value = serde_json::from_str(match controller {
            Controller::Maschine => include_str!("../vendor/encdr/descriptors/ni_maschine_mk3.json"),
            Controller::Komplete => include_str!("../vendor/encdr/descriptors/ni_komplete_kontrol_s49_mk2.json"),
        })
        .expect("bundled MK3 descriptor");
        descriptor["leds"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|group| group["items"].as_array().unwrap())
            .filter_map(|item| item["name"].as_str().map(str::to_owned))
            .collect()
    });
    for name in names {
        if controller == Controller::Komplete && matches!(name.as_str(), "top_6" | "key_mode") {
            encdr.set_led(dev, name, LedValue::Off);
            continue;
        }
        if !controller.owns_led(name) { continue; }
        let level = controller.led_source(name).and_then(|source| state[source].as_u64()).unwrap_or(0).min(100) as u8;
        let value = if controller == Controller::Komplete
            && state["track"].as_u64() == Some(80)
            && matches!(name.as_str(), "top_1" | "top_2" | "top_3" | "top_4")
        {
            controller.slot_light(usize::from(name.as_bytes()[4] - b'1'), level)
        } else {
            controller.button_light(name, level)
        };
        encdr.set_led(dev, name, value);
    }
}
fn run_hardware(input: mpsc::Receiver<String>, options: &Options) -> Result<(), Box<dyn Error>> {
    let controller = options.controller;
    let visible = options.visible;
    let led_test = options.led_test;
    let test = options.test;
    let mut state = Session::new(
        test,
        false,
        options.state_file.clone(),
        options.g1,
        options.patch_dir.clone(),
    )?;
    let mut encdr = backend(true, options.controller)?;
    let mut link: Link<DeviceId> = Link::new();
    state.set_connection(link.snapshot());
    // Keep views across disconnects; set_device_id retargets them after replug.
    let mut views: Option<(ScreenView, ScreenView)> = {
        let left = ScreenView::new_unbound(
            "left",
            (480, 272),
            ScreenContent::Html(screen_html_with_role(
                "left",
                "left",
                test,
                options.g1.is_some(),
            )),
            visible,
        )
        .map_err(|e| format!("Left screen open failed: {e}"))?;
        let right = ScreenView::new_unbound(
            "right",
            (480, 272),
            ScreenContent::Html(screen_html_with_role(
                "right",
                "right",
                test,
                options.g1.is_some(),
            )),
            visible,
        )
        .map_err(|e| format!("Right screen open failed: {e}"))?;
        #[cfg(target_os = "macos")]
        if options.g1.is_some() {
            left.set_capture_ink_guard(Some((20, 11, 400, 32)));
            right.set_capture_ink_guard(Some((6, 11, 468, 245)));
        }
        #[cfg(target_os="macos")]
        if controller == Controller::Komplete {
            left.set_capture_interval_ms(33);
            right.set_capture_interval_ms(33);
        }
        Some((left, right))
    };
    let mut last_leds: Option<serde_json::Value> = None;
    println!("Waiting for {} 17cc:{:04x}. Connect/reconnect it, or type quit to exit.", controller.label(), controller.product());
    let source = encdr.events().clone();
    let native_mode = std::sync::Arc::new(controller::NativeModeGate::default());
    let input_mode = native_mode.clone();
    let capture_buttons_only=controller==Controller::Komplete && std::env::var_os("MK3_S49_CAPTURE_BUTTONS_ONLY").is_some();
    if capture_buttons_only{println!("S49 button capture only: control actions suppressed; raw reports remain logged");}
    let led_probe_path=if capture_buttons_only{std::env::var_os("MK3_S49_LED_PROBE_FILE").map(std::path::PathBuf::from)}else{None};
    let mut led_probe_previous:Option<Vec<u8>>=None;let mut led_probe_poll=Instant::now()-Duration::from_secs(1);
    let activity_client=if test{None}else{state.hardware_input()};
    let (input_dispatch, events) = input_dispatch::InputDispatch::start(
        move || source.recv_timeout(Duration::from_millis(5)).ok().and_then(|event| {
            if input_dispatch::pipeline_trace_enabled(){tracing::info!("PIPELINE stage=decoded event={:?}",event);}
            let touched=match &event {Event::Encoder{delta,..}=>*delta!=0,Event::EncoderFine{delta,..}=>delta.is_finite()&&*delta!=0.,Event::Button{..}|Event::Touch{..}|Event::Slider{..}|Event::Grid{..}=>true,_=>false};
            if touched&&controller.accepts(event.device_id()){if let Some(client)=&activity_client{client.lock().unwrap().note_activity();}}
            if controller == Controller::Komplete {
                input_mode.note_input(&event);
                if input_mode.paused() && !matches!(event, Event::DeviceConnected { .. } | Event::DeviceDisconnected { .. }) { return None; }
            } else if !input_mode.maschine_input_allowed(&event) {
                return None;
            }
            if capture_buttons_only && !matches!(event,Event::DeviceConnected{..}|Event::DeviceDisconnected{..}){return None;}
            controller.input(event)
        }),
        if test { None } else { state.hardware_input() },
    );
    let mut dirty = true;
    let mut refresh = Instant::now() - Duration::from_secs(1);
    let mut trace_state_seq=0u64;
    #[cfg(target_os = "macos")]
    let mut timing_report = Instant::now();
    #[cfg(target_os = "macos")]
    let mut health_report = Instant::now();
    let result = run_ui_ticks(|| {
        if STOP.load(Ordering::Relaxed) {
            return Ok(false);
        }
        let detected = link
            .device()
            .filter(|id| encdr.is_connected(*id))
            .or_else(|| find_controller(&encdr, controller));
        let ni_owned = detected.is_some_and(|id| encdr.has_control(id));
        let owns = native_mode.note_control(ni_owned);
        input_dispatch.set_device(if owns { detected } else { None });
        dirty |= input_dispatch.take_dirty();
        if let Some(change) = link.observe(detected, owns) {
            last_leds = None;
            state.set_connection(link.snapshot());
            println!("{} connection: {}", controller.label(), link.snapshot());
            dirty = true;
            if let Some(dev) = detected {
                if change.retarget {
                    if let Some((left, right)) = &mut views {
                        left.set_device_id(dev);
                        right.set_device_id(dev);
                    }
                }
                if change.resume_output && (led_test || test) {
                    set_test_leds(&encdr, dev);
                }
            }
        }
        while let Ok(event) = events.try_recv() {
            if let Event::DeviceDisconnected { id } = &event {
                if link.disconnected(*id) {
                    state.set_connection(link.snapshot());
                    dirty = true;
                    println!("{} unplugged; waiting for reconnect. Counters/state preserved; quit remains available.", controller.label());
                }
                continue;
            }
            if !link.accepts_input(event.device_id()) {
                continue;
            }
            tracing::debug!("input: {event:?}");
            if test
                && matches!(
                    event,
                    Event::Button {
                        name: "stop",
                        pressed: true,
                        ..
                    }
                )
            {
                STOP.store(true, Ordering::Relaxed);
            }
            dirty |= state.event(&event);
        }
        while let Ok(command) = input.try_recv() {
            dirty |= apply_command(&mut state, &command);
        }
        dirty |= state.tick();
        if let Some(path)=&led_probe_path {
            if !owns{led_probe_previous=None;}
            else if led_probe_poll.elapsed()>=Duration::from_millis(150){
                led_probe_poll=Instant::now();
                let frame=std::fs::read(path).ok().and_then(|raw|s49_probe_frame(&raw)).unwrap_or_else(||vec![0;79]);
                if led_probe_previous.as_ref()!=Some(&frame){
                    if let Some(dev)=link.device(){encdr.set_led_strip_in_group(dev,"button_leds","button_probe",&frame);println!("S49 LED PROBE payload={:?}",frame);led_probe_previous=Some(frame);}
                }
            }
        }
        if led_probe_path.is_none() && options.g1.is_some() && owns && (dirty || last_leds.is_none()) {
            let leds = state.leds();
            if last_leds.as_ref() != Some(&leds) {
                if let Some(dev) = link.device() {
                    set_g1_leds(&encdr, dev, &leds, controller);
                }
                last_leds = Some(leds);
            }
        }
        if dirty && refresh.elapsed() >= Duration::from_millis(16) {
            let mut payload = state.snapshot();
            if input_dispatch::pipeline_trace_enabled(){
                trace_state_seq+=1;
                payload["traceStateSeq"]=trace_state_seq.into();
                payload["traceInputSeq"]=input_dispatch::input_sequence().into();
                tracing::info!("PIPELINE stage=state state_seq={} latest_input_seq={} values={} browser={} cursor={}",trace_state_seq,input_dispatch::input_sequence(),payload["displayValues"],payload["browserOpen"],payload["browserCursor"]);
            }
            if let Some((left, right)) = &views {
                // Mixer intentionally clears screen 2. Do not let the blank-
                // snapshot recovery guard retain the previous screen instead.
                #[cfg(target_os="macos")]
                right.set_capture_ink_guard(if payload["view"]=="mixer"{None}else{Some((6,11,468,245))});
                // Full-screen ALERT can legitimately replace the header with dark pixels.
                #[cfg(target_os="macos")]
                left.set_capture_ink_guard(Some(if payload["screensaver"]==true || payload["screenFade"]==true || payload["glitchIntensity"].as_f64().is_some_and(|v|v>=0.999999) && payload["blobsEnabled"]!=false {(8,8,464,256)}else{(20,11,400,32)}));
                #[cfg(target_os="macos")]
                right.set_capture_interval_ms(if payload["screensaver"]==true || payload["screenFade"]==true || payload["blobsEnabled"]!=false && (payload["view"]=="display" || payload["view"]=="parameters" && payload["moduleEdit"]!=true) && payload["browserOpen"]!=true {50}else{33});
                #[cfg(target_os="macos")]
                left.set_capture_interval_ms(if payload["screensaver"]==true || payload["screenFade"]==true || payload["blobsEnabled"]!=false && (payload["view"]=="display" || payload["view"]=="parameters" && payload["moduleEdit"]!=true) && payload["browserOpen"]!=true {50}else{33});
                left.send("state", payload.clone());
                right.send("state", payload.clone());
            }
            dirty = false;
            refresh = Instant::now();
        }
        if let Some((left, right)) = &views {
            if link.device().is_some_and(|id| link.accepts_input(id)) {
                left.poll(&encdr);
                right.poll(&encdr);
            }
            #[cfg(target_os = "macos")]
            if std::env::var_os("MK3_SCREEN_TIMING").is_some()
                && timing_report.elapsed() >= Duration::from_secs(1)
            {
                let backlog = link.device().map_or(0, |id| encdr.screen_backlog(id));
                println!(
                    "Screen timing: left={:?} right={:?} backlog={backlog}",
                    left.capture_stats(),
                    right.capture_stats()
                );
                timing_report = Instant::now();
            }
        }
        #[cfg(target_os = "macos")]
        if health_report.elapsed() >= Duration::from_secs(5) {
            let snapshot = state.snapshot();
            let screens=views.as_ref().map(|(left,right)|serde_json::json!({"leftFrames":left.capture_stats().frames,"leftMs":left.capture_stats().last_ms,"leftMaxMs":left.capture_stats().max_ms,"rightFrames":right.capture_stats().frames,"rightMs":right.capture_stats().last_ms,"rightMaxMs":right.capture_stats().max_ms,"backlog":link.device().map_or(0, |id| encdr.screen_backlog(id))}));
            println!(
                "health: {}",
                serde_json::json!({"engine":snapshot["engine"],"revision":snapshot["revision"],"view":snapshot["view"],"browser":snapshot["browserOpen"],"cursor":snapshot["browserCursor"],"loading":snapshot["loading"],"status":snapshot["status"],"connection":snapshot["connection"],"screens":screens})
            );
            health_report = Instant::now();
        }
        Ok(true)
    });
    drop(input_dispatch);
    if test {
        let report = serde_json::json!({
            "observations": state.snapshot(),
            "exit": if result.is_ok() { "requested" } else { "error" },
            "error": result.as_ref().err().map(|e|e.to_string()),
            "cleanup": "requested; verify LEDs off and terminal returned",
        });
        println!("Hardware test report: {report}");
        match serde_json::to_vec_pretty(&report)
            .map_err(io::Error::other)
            .and_then(|bytes| std::fs::write("hardware-test-session.json", bytes))
        {
            Ok(()) => println!("Saved hardware-test-session.json"),
            Err(e) => eprintln!("Could not save report: {e}"),
        }
    }
    let save = state.finish().map_err(Into::into);
    println!("Releasing MK3 and clearing LEDs…");
    encdr.clean_exit();
    println!("MK3 disconnected after output cleanup; check LEDs and exit screens on hardware.");
    result.and(save)
}
fn main() -> Result<(), Box<dyn Error>> {
    // Configure before creating any threads. The MK3 service screen route was
    // confirmed on the user's Mac; direct USB screen claiming conflicted with NI.
    #[cfg(target_os = "macos")]
    std::env::set_var("ENCDR_FORCE_SERVICE", "1");
    let options = Options::parse(std::env::args().skip(1))?;
    if options.help {
        println!("{HELP}");
        return Ok(());
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    if options.diagnose {
        return diagnose(options.controller);
    }
    // Handler only sets an atomic flag; cleanup runs on the main thread.
    unsafe {
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
    }
    println!("{HELP}");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            match line {
                Ok(s) => {
                    if tx.send(s).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    if options.mock {
        run_mock(rx, &options)
    } else {
        run_hardware(rx, &options)
    }
}

#[cfg(test)]
mod led_probe_tests{
    #[test]fn one_bounded_byte_or_all_off(){
        let frame=super::s49_probe_frame(br#"{"offset":37,"value":158}"#).unwrap();assert_eq!(frame.len(),79);assert_eq!(frame.iter().filter(|&&v|v!=0).count(),1);assert_eq!(frame[37],158);
        for bad in [br#"{"offset":79,"value":158}"#.as_slice(),br#"{"offset":0,"value":256}"#,br#"{"offset":-1,"value":0}"#,b"bad"]{assert!(super::s49_probe_frame(bad).is_none());}
        assert!(super::s49_probe_frame(br#"{"off":true}"#).unwrap().iter().all(|&v|v==0));
    }
}
