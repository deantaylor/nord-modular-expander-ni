//! Real G1-Emu controller transport. Audio and patches stay in the emulator process.
use crate::patch_browser::Browser;
use encdr::Event;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
fn wall_ms()->i64{std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64}
const FRAME_LIMIT: usize = 1024 * 1024;
const ENCODER_SENSITIVITY: f64 = 0.5;
const ENCODER_WRITE_INTERVAL: Duration = Duration::from_millis(16);
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Knob {
    pub index: usize,
    pub assigned: bool,
    pub slot: u8,
    pub binding: String,
    pub module: String,
    pub parameter: String,
    pub value: u8,
    pub max: u8,
    pub display: String,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ModuleEntry {pub section:u8,pub index:u8,pub name:String,pub type_name:String,pub parameter_count:usize}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ModuleParameter {pub index:usize,pub binding:String,pub name:String,pub value:u8,pub min:u8,pub max:u8,pub display:String}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ModuleView {#[serde(default)] pub playhead:Option<u8>,pub section:u8,pub index:u8,pub name:String,pub type_name:String,pub parameters:Vec<ModuleParameter>}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    #[serde(default)] pub midi_activity:Value,
    #[serde(default)] pub restore:Value,
    #[serde(default)] pub patch_binding:String,
    #[serde(default)] pub edit:Option<ModuleView>,
    pub running: bool,
    #[serde(default)]
    pub problem: String,
    pub active_slot: u8,
    #[serde(default)]
    pub slot_known: bool,
    #[serde(default)]
    pub pad_midi: bool,
    #[serde(default)]
    pub pad_midi_held: Vec<u8>,
    pub lcd: [String; 2],
    pub knobs: Vec<Knob>,
    #[serde(default)]
    pub allocated_voices: Option<u8>,
    #[serde(default)]
    pub requested_voices: Option<u8>,
    #[serde(default)]
    pub upload: Option<Upload>,
    #[serde(default)]
    pub bank_load: Option<Upload>,
    #[serde(default)]
    pub configuration: Option<Configuration>,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Configuration {
    pub audio: String,
    pub rom: String,
    pub model: String,
    pub direct_link: bool,
    pub function: bool,
    pub settings_pending: bool,
    pub controls: Vec<Control>,
    pub selected_slots: Vec<bool>,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Control {
    pub key: String,
    pub label: String,
    pub value: i32,
    pub min: i32,
    pub max: i32,
    pub display: String,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Upload {
    pub serial: u64,
    pub busy: bool,
    pub ok: bool,
    pub message: String,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Preset {
    pub bank: u8,
    pub position: u8,
    pub name: String,
}
#[derive(Clone,Debug,PartialEq,Deserialize)]
struct ValuePreview {binding:String,value:u8,display:String}
#[derive(Clone, Debug, Default, PartialEq)]
struct Observed {
    connected: bool,
    instance: String,
    status: Status,
    presets: Arc<Vec<Preset>>,
    complete: bool,
    presets_revision: u64,
    error: String,
    generation: u64,
    load_response: Option<(u64, String)>,
    modules: Arc<Vec<ModuleEntry>>,
    module_binding:String,
    previews:Arc<BTreeMap<String,ValuePreview>>,
}
#[derive(Clone, Debug)]
enum Command {
    Restore(u8,String),
    Home,
    ModuleMode(bool),
    MixerMode(bool),
    SelectModule(ModuleEntry,String),
    Parameter {input_seq:u64,section:u8,module:u8,index:usize,value:u8,binding:String},
    Set {
        input_seq: u64,
        index: usize,
        value: u8,
        binding: String,
    },
    Slot(u8),
    Load(Preset, u8, u64),
    File(String, u8, u64),
    Refresh,
    Control(String, i32),
    Function(bool),
    SettingsUi,
    Pad {
        epoch: u64,
        pad: u8,
        slot: u8,
        note: u8,
        velocity: u8,
    },
}
struct Rpc {
    stream: TcpStream,
    buffer: Vec<u8>,
    id: u64,
}
impl Rpc {
    fn connect(address: SocketAddr, stop: &AtomicBool) -> Result<(Self, String), String> {
        let stream = TcpStream::connect_timeout(&address, Duration::from_millis(300))
            .map_err(|e| e.to_string())?;
        stream.set_nonblocking(true).map_err(|e| e.to_string())?;
        stream.set_nodelay(true).map_err(|e| e.to_string())?;
        let mut rpc = Self {
            stream,
            buffer: Vec::new(),
            id: 0,
        };
        let hello = rpc.read(stop)?;
        if !matches!(
            hello["service"].as_str(),
            Some("nord-modular-ni-interface-g1" | "modular-machine-g1")
        ) || hello["protocol"] != 1
        {
            return Err(
                "Not a compatible G1 controller bridge (stock DirectLink is a different protocol)"
                    .into(),
            );
        }
        let instance = hello["instance"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 64)
            .ok_or("Invalid bridge instance")?
            .to_string();
        Ok((rpc, instance))
    }
    fn read(&mut self, stop: &AtomicBool) -> Result<Value, String> {
        let deadline = Instant::now() + Duration::from_millis(800);
        loop {
            if stop.load(Ordering::Relaxed) {
                return Err("Controller stopping".into());
            }
            if let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
                let frame: Vec<_> = self.buffer.drain(..=end).collect();
                return serde_json::from_slice(&frame)
                    .map_err(|e| format!("Invalid bridge JSON: {e}"));
            }
            if self.buffer.len() >= FRAME_LIMIT {
                return Err("Bridge frame exceeds limit".into());
            }
            if Instant::now() >= deadline {
                return Err("G1 bridge timed out".into());
            }
            let mut bytes = [0; 8192];
            match self.stream.read(&mut bytes) {
                Ok(0) => return Err("G1 bridge disconnected".into()),
                Ok(count) => {
                    if self.buffer.len() + count > FRAME_LIMIT {
                        return Err("Bridge frame exceeds limit".into());
                    }
                    self.buffer.extend_from_slice(&bytes[..count]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2))
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e.to_string()),
            }
        }
    }
    fn call(&mut self, mut request: Value, stop: &AtomicBool) -> Result<Value, String> {
        self.id = self.id.checked_add(1).ok_or("RPC ID exhausted")?;
        request["id"] = self.id.into();
        let mut bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        let deadline = Instant::now() + Duration::from_millis(800);
        let mut offset = 0;
        while offset < bytes.len() {
            if stop.load(Ordering::Relaxed) || Instant::now() >= deadline {
                return Err("G1 bridge write stopped/timed out".into());
            }
            match self.stream.write(&bytes[offset..]) {
                Ok(0) => return Err("G1 bridge closed while writing".into()),
                Ok(count) => offset += count,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2))
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        let response = self.read(stop)?;
        if response["id"] != self.id {
            return Err("G1 bridge response ID mismatch".into());
        }
        if let Some(error) = response["error"].as_str() {
            return Ok(json!({"rejected":error}));
        }
        response
            .get("result")
            .cloned()
            .ok_or("Missing bridge result".into())
    }
}
fn checked_status(value: Value) -> Result<Status, String> {
    if let Some(error) = value.get("rejected").and_then(Value::as_str) {
        return Err(format!("G1 rejected status: {error}"));
    }
    let status: Status = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if status.pad_midi_held.len() > 16
        || status.pad_midi_held.iter().any(|n| *n >= 16)
        || status.active_slot > 3
        || (status.running && status.knobs.len() != 18)
        || (!status.running && !status.knobs.is_empty())
        || status.lcd.iter().any(|s| s.len() > 64)
        || status
            .upload
            .as_ref()
            .is_some_and(|u| u.message.len() > 1024)
    {
        return Err("Invalid G1 status".into());
    }
    if let Some(config) = &status.configuration {
        let mut keys = std::collections::BTreeSet::new();
        if config.controls.len() > 32
            || config.audio.len() > 512
            || config.rom.len() > 4096
            || config.selected_slots.len() > 4
        {
            return Err("Invalid emulator settings metadata".into());
        }
        for c in &config.controls {
            if !keys.insert(&c.key)
                || c.key.len() > 256
                || c.label.len() > 64
                || c.display.len() > 128
                || c.min < -127
                || c.max > 255
                || c.min > c.max
                || c.value < c.min
                || c.value > c.max
            {
                return Err("Invalid emulator control range".into());
            }
        }
    }
    if let Some(edit)=&status.edit {
        let mut ids=std::collections::BTreeSet::new();
        if edit.section>1 || edit.index>127 || edit.parameters.len()>128 || edit.name.len()>128 || edit.type_name.len()>128 {return Err("Invalid module view".into());}
        for p in &edit.parameters {if p.index>127 || !ids.insert(p.index) || p.min>p.max || p.max>127 || p.value<p.min || p.value>p.max || p.binding.len()>256 || p.name.len()>128 || p.display.len()>128 {return Err("Invalid module parameter".into());}}
    }
    for (i, k) in status.knobs.iter().enumerate() {
        if k.index != i
            || k.slot > 3
            || k.value > k.max && k.assigned
            || k.binding.len() > 256
            || k.module.len() > 64
            || k.parameter.len() > 64
            || k.display.len() > 128
        {
            return Err("Invalid G1 knob metadata".into());
        }
    }
    Ok(status)
}
fn publish(shared: &Mutex<Observed>, mut next: Observed) {
    let mut state = shared.lock().unwrap();
    next.generation = state.generation;
    if *state != next {
        next.generation = next.generation.wrapping_add(1);
        *state = next;
    }
}
fn fetch_modules(rpc:&mut Rpc,state:&mut Observed,stop:&AtomicBool,select_first:bool)->Result<(),String>{
    let result=rpc.call(json!({"method":"modules"}),stop)?;
    if result.get("entries").is_none()&&!select_first{state.modules=Default::default();state.module_binding=state.status.patch_binding.clone();return Ok(());}
    if let Some(error)=result["rejected"].as_str(){state.error=error.into();return Ok(());}
    let key=result["patchBinding"].as_str().ok_or("Missing module identity")?.to_string();
    let entries:Vec<ModuleEntry>=serde_json::from_value(result["entries"].clone()).map_err(|e|e.to_string())?;
    let mut ids=std::collections::BTreeSet::new();
    if key.len()>128 || entries.len()>256 || entries.iter().any(|m|m.section>1 || m.index>127 || !ids.insert((m.section,m.index)) || m.parameter_count>128 || m.name.len()>128 || m.type_name.len()>128){return Err("Invalid module catalogue".into());}
    if let Some(first)=entries.first().filter(|_|select_first){command_call(rpc,json!({"method":"module_select","patchBinding":key,"section":first.section,"index":first.index}),stop,state)?;}
    state.modules=Arc::new(entries);state.module_binding=key;Ok(())
}
fn command_call(
    rpc: &mut Rpc,
    request: Value,
    stop: &AtomicBool,
    state: &mut Observed,
) -> Result<(), String> {
    let edits=matches!(request["method"].as_str(),Some("set_knobs"|"set_parameters")) || (request["method"]=="set_control" && request["key"].as_str().is_some_and(|k|k.starts_with("slotLevel:")));
    let response = rpc.call(request, stop)?;
    state.error = response["rejected"].as_str().unwrap_or("").to_string();
    if !edits || !state.error.is_empty(){state.previews=Default::default();}
    if edits && state.error.is_empty(){
        if let Some(raw)=response.get("previews"){
            let previews:Vec<ValuePreview>=serde_json::from_value(raw.clone()).map_err(|e|e.to_string())?;
            if previews.len()>128 || previews.iter().any(|p|p.binding.len()>256||p.value>127||p.display.len()>128){return Err("Invalid accepted-value preview".into());}
            let mut cached=(*state.previews).clone();for preview in previews{cached.insert(preview.binding.clone(),preview);}state.previews=Arc::new(cached);
        }
    }
    Ok(())
}
fn worker(
    address: SocketAddr,
    rx: mpsc::Receiver<Command>,
    shared: Arc<Mutex<Observed>>,
    stop: Arc<AtomicBool>,
    pad_panic: Arc<AtomicBool>,
    pad_epoch: Arc<AtomicU64>,
) {
    let mut banks_requested = false;
    while !stop.load(Ordering::Relaxed) {
        // Preserve read intent, never replay queued mutations against a new patch.
        while let Ok(command) = rx.try_recv() {
            if matches!(command, Command::Refresh) { banks_requested = true; }
        }
        let connection = Rpc::connect(address, &stop);
        match connection {
            Err(error) => publish(
                &shared,
                Observed {
                    error,
                    ..Default::default()
                },
            ),
            Ok((mut rpc, instance)) => {
                let mut state = Observed {
                    connected: true,
                    instance,
                    ..Default::default()
                };
                let mut next_presets = Instant::now();
                let mut next_refresh = Instant::now();
                let mut next_status = Instant::now();
                let mut queued = VecDeque::new();
                let mut targets = BTreeMap::new();
                let mut controls = BTreeMap::new();
                let mut parameter_targets=BTreeMap::new();
                let mut wants_modules=false;
                let mut next_encoder_write = Instant::now();
                let run = (|| -> Result<(), String> {
                    loop {
                        if stop.load(Ordering::Relaxed) {
                            return Ok(());
                        }
                        let mut commands: Vec<_> =
                            queued.drain(..).chain(rx.try_iter()).take(128).collect();
                        if pad_panic.swap(false, Ordering::AcqRel) {
                            commands.retain(|c| !matches!(c, Command::Pad { epoch, .. } if *epoch != pad_epoch.load(Ordering::Acquire)));
                            if state.status.pad_midi {
                                command_call(
                                    &mut rpc,
                                    json!({"method":"pad_panic"}),
                                    &stop,
                                    &mut state,
                                )?;
                            }
                        }
                        let mut pending_module=None;
                        for command in commands {
                            match command {
                                Command::Parameter {input_seq,section,module,index,value,binding}=>{
                                    parameter_targets.insert(binding.clone(),json!({"inputSeq":input_seq,"section":section,"module":module,"index":index,"value":value,"binding":binding}));continue;
                                }
                                Command::SelectModule(entry,key)=>{targets.clear();parameter_targets.clear();pending_module=Some((entry,key));continue;}
                                Command::Restore(slot,binding)=>{targets.clear();parameter_targets.clear();controls.clear();pending_module=None;command_call(&mut rpc,json!({"method":"restore_slot","slot":slot,"patchBinding":binding}),&stop,&mut state)?;next_status=Instant::now();continue;}
                                Command::Home=>{targets.clear();parameter_targets.clear();controls.clear();pending_module=None;wants_modules=false;command_call(&mut rpc,json!({"method":"module_exit"}),&stop,&mut state)?;command_call(&mut rpc,json!({"method":"mixer_mode","enabled":false}),&stop,&mut state)?;next_status=Instant::now();continue;}
                                Command::MixerMode(enabled)=>{controls.clear();command_call(&mut rpc,json!({"method":"mixer_mode","enabled":enabled}),&stop,&mut state)?;next_status=Instant::now();continue;}
                                Command::ModuleMode(enabled)=>{
                                    wants_modules=enabled;targets.clear();parameter_targets.clear();pending_module=None;
                                    if enabled && state.status.running {fetch_modules(&mut rpc,&mut state,&stop,true)?;}
                                    else if !enabled {command_call(&mut rpc,json!({"method":"module_exit"}),&stop,&mut state)?;}
                                    next_status=Instant::now();continue;
                                }
                                _=>{}
                            }
                            if matches!(&command, Command::Pad { epoch, .. } if *epoch != pad_epoch.load(Ordering::Acquire))
                            {
                                continue;
                            }
                            if let Command::Set {
                                input_seq,
                                index,
                                value,
                                binding,
                            } = command
                            {
                                targets.insert(
                                    index,
                                    json!({"index":index,"value":value,"binding":binding,"inputSeq":input_seq}),
                                );
                                continue;
                            }
                            if let Command::Control(key, value) = command {
                                controls.insert(key, value);
                                continue;
                            }
                            if matches!(
                                command,
                                Command::Slot(_)
                                    | Command::Load(..)
                                    | Command::File(..)
                                    | Command::Function(true)
                            ) {
                                targets.clear();parameter_targets.clear();pending_module=None;
                            }
                            let load_request = match &command {
                                Command::File(_, _, id) | Command::Load(_, _, id) => Some(*id),
                                _ => None,
                            };
                            let request = match command {
                                Command::Slot(slot) => json!({"method":"select_slot","slot":slot}),
                                Command::Load(preset, slot, _) => {
                                    json!({"method":"load_patch","slot":slot,"bank":preset.bank,"position":preset.position})
                                }
                                Command::File(path, slot, _) => {
                                    json!({"method":"load_file","slot":slot,"path":path})
                                }
                                Command::Refresh => {
                                    banks_requested = true;
                                    next_refresh = Instant::now();
                                    next_presets = Instant::now();
                                    continue;
                                }
                                Command::Control(key, value) => {
                                    json!({"method":"set_control","key":key,"value":value})
                                }
                                Command::Function(pressed) => {
                                    json!({"method":"function","pressed":pressed})
                                }
                                Command::SettingsUi => json!({"method":"show_settings"}),
                                Command::Pad {
                                    epoch: _,
                                    pad,
                                    slot,
                                    note,
                                    velocity,
                                } => {
                                    json!({"method":"pad_midi","pad":pad,"slot":slot,"note":note,"velocity":velocity})
                                }
                                Command::Restore(..) | Command::Home | Command::Set { .. } | Command::Parameter {..} | Command::MixerMode(_) | Command::ModuleMode(_) | Command::SelectModule(..) => unreachable!(),
                            };
                            command_call(&mut rpc, request, &stop, &mut state)?;
                            if let Some(id) = load_request {
                                state.load_response = Some((id, state.error.clone()));
                            }
                        }
                        if let Some((entry,key))=pending_module {
                            command_call(&mut rpc,json!({"method":"module_select","patchBinding":key,"section":entry.section,"index":entry.index}),&stop,&mut state)?;
                            next_status=Instant::now();
                        }
                        if (!targets.is_empty() || !controls.is_empty() || !parameter_targets.is_empty())
                            && Instant::now() >= next_encoder_write
                        {
                            if !targets.is_empty() {
                                if crate::input_dispatch::pipeline_trace_enabled(){tracing::info!("PIPELINE stage=rpc_send targets={:?}",targets);}
                                command_call(
                                    &mut rpc,
                                    json!({"method":"set_knobs","targets":targets.values().collect::<Vec<_>>()}),
                                    &stop,
                                    &mut state,
                                )?;
                                targets.clear();
                            }
                            if !parameter_targets.is_empty(){
                                if crate::input_dispatch::pipeline_trace_enabled(){tracing::info!("PIPELINE stage=rpc_send module_targets={:?}",parameter_targets);}
                                command_call(&mut rpc,json!({"method":"set_parameters","targets":parameter_targets.values().collect::<Vec<_>>()}),&stop,&mut state)?;
                                parameter_targets.clear();
                            }
                            for (key, value) in std::mem::take(&mut controls) {
                                command_call(
                                    &mut rpc,
                                    json!({"method":"set_control","key":key,"value":value}),
                                    &stop,
                                    &mut state,
                                )?;
                            }
                            next_encoder_write = Instant::now() + ENCODER_WRITE_INTERVAL;
                        }
                        if Instant::now() >= next_status {
                            let next=checked_status(rpc.call(json!({"method":"status"}), &stop)?)?;
                            if crate::input_dispatch::pipeline_trace_enabled() && next.knobs!=state.status.knobs {
                                tracing::info!("PIPELINE stage=readback values={:?}",next.knobs.iter().map(|k|(k.index,k.binding.as_str(),k.value,k.display.as_str())).collect::<Vec<_>>());
                            }
                            state.status=next;
                            if state.status.running && state.status.patch_binding!=state.module_binding {
                                parameter_targets.clear();fetch_modules(&mut rpc,&mut state,&stop,wants_modules)?;
                            }
                            next_status = Instant::now() + ENCODER_WRITE_INTERVAL;
                        }
                        if banks_requested && state.status.running && Instant::now() >= next_presets {
                            let refresh = Instant::now() >= next_refresh;
                            let result =
                                rpc.call(json!({"method":"presets","refresh":refresh}), &stop)?;
                            if let Some(error) = result["rejected"].as_str() {
                                state.error = error.into();
                            } else {
                                let presets: Vec<Preset> =
                                    serde_json::from_value(result["entries"].clone())
                                        .map_err(|e| e.to_string())?;
                                if presets.len() > 891
                                    || presets
                                        .iter()
                                        .any(|p| p.bank > 8 || p.position > 98 || p.name.len() > 64)
                                {
                                    return Err("Invalid preset list".into());
                                }
                                if state.presets.as_ref() != &presets {
                                    state.presets_revision = state.presets_revision.wrapping_add(1);
                                    state.presets = Arc::new(presets);
                                }
                                state.complete = result["complete"]
                                    .as_bool()
                                    .ok_or("Missing preset-list state")?;
                            }
                            next_presets = Instant::now()
                                + if state.complete {
                                    Duration::from_secs(10)
                                } else {
                                    Duration::from_millis(500)
                                };
                            if refresh {
                                next_refresh = Instant::now() + Duration::from_secs(10);
                            }
                        }
                        publish(&shared, state.clone());
                        match rx.recv_timeout(Duration::from_millis(4)) {
                            Ok(command) => queued.push_back(command),
                            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                        }
                    }
                })();
                if let Err(error) = run {
                    tracing::warn!(
                        "G1 bridge connection lost: {error}; reconnecting without queued commands"
                    );
                    publish(
                        &shared,
                        Observed {
                            error,
                            ..Default::default()
                        },
                    );
                }
            }
        }
        for _ in 0..50 {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
#[derive(Clone)]
struct Drive {
    binding: String,
    remainder: f64,
    pending: Option<(u8, Instant)>,
}
#[derive(Default)]
struct ControlDrive {
    remainder: f64,
    pending: Option<(i32, Instant)>,
}
pub struct Client {
    shared: Arc<Mutex<Observed>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
    tx: mpsc::SyncSender<Command>,
    observed: Observed,
    page: usize,
    cursor: usize,
    drives: [Option<Drive>; 18],
    pending_slot: Option<(u8, Instant)>,
    files: Browser,
    browser_open: bool,
    file_source: bool,
    pending_file: Option<(u64, String, Instant, u64)>,
    pending_bank: Option<(u64, String, Instant, u64)>,
    load_request: u64,
    browser_state: Option<PathBuf>,
    library_root: PathBuf,
    restore_bank: Option<(u8, u8)>,
    notice: String,
    view: &'static str,
    settings_page: usize,
    ui_colour_intensity: f64,
    ui_theme:f64,
    blobs_enabled:bool,
    glitch_intensity:f64,
    last_activity:Instant,
    last_midi_activity:i64,
    screensaver:bool,
    screensaver_since_ms:i64,
    screen_fade_until:Option<Instant>,
    activity_dirty:bool,
    restore_wait:Option<u64>,
    sequence_edit:bool,sequence_step_focus:bool,sequence_step:usize,sequence_row:usize,sequence_target:String,
    module_edit:bool,
    module_cursor:usize,
    module_page:usize,
    module_drives:BTreeMap<String,Drive>,
    control_drives: BTreeMap<String, ControlDrive>,
    pending_home_reset: bool,
    shift_pressed: bool,
    track_pressed: bool,
    held_buttons: std::collections::BTreeSet<String>,
    function_release_pending: bool,
    pads: crate::pad_midi::Pads,
    pad_panic: Arc<AtomicBool>,
    pad_epoch: Arc<AtomicU64>,
}
impl Client {
    pub fn new(address: SocketAddr, patch_dir: PathBuf) -> Self {
        let shared = Arc::new(Mutex::new(Observed::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::sync_channel(128);
        let pad_panic = Arc::new(AtomicBool::new(false));
        let pad_epoch = Arc::new(AtomicU64::new(0));
        let thread = {
            let pad_panic = pad_panic.clone();
            let pad_epoch = pad_epoch.clone();
            let shared = shared.clone();
            let stop = stop.clone();
            thread::spawn(move || worker(address, rx, shared, stop, pad_panic, pad_epoch))
        };
        let mut files = Browser::new(patch_dir.clone());
        let _ = files.refresh();
        let mut browser_state = std::env::var_os("MK3_BROWSER_STATE").map(PathBuf::from);
        let mut preference_error = false;
        let preferences = browser_state.as_ref().and_then(|path| {
            match crate::browser_state::Preferences::load(path, &patch_dir, &mut files) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("Browser preferences preserved without loading: {error}");
                    preference_error = true;
                    None
                }
            }
        });
        if preference_error {
            browser_state = None;
        }
        let file_source = preferences.as_ref().is_none_or(|p| p.files);
        let ui_colour_intensity=preferences.as_ref().map_or(0.,|p|f64::from(p.ui_colour_intensity.min(100))/100.);
        let ui_theme=preferences.as_ref().map_or(0.,|p|f64::from(p.ui_theme.min(2)));
        let blobs_enabled=preferences.as_ref().is_none_or(|p|p.blobs_enabled);
        let glitch_intensity=preferences.as_ref().map_or(0.5,|p|f64::from(p.glitch_level())/100.);
        let restore_bank = preferences.and_then(|p| p.bank);
        let client = Self {
            shared,
            stop,
            thread: Some(thread),
            tx,
            observed: Observed::default(),
            page: 0,
            cursor: 0,
            drives: std::array::from_fn(|_| None),
            pending_slot: None,
            files,
            browser_open: false,
            file_source,
            pending_file: None,
            pending_bank: None,
            load_request: 0,
            browser_state,
            library_root: patch_dir,
            restore_bank,
            notice: "Waiting for G1-Emu controller bridge".into(),
            view: "parameters",
            settings_page: 0,
            ui_colour_intensity,
            ui_theme,
            blobs_enabled,glitch_intensity,
            last_activity:Instant::now(),last_midi_activity:0,screensaver:false,screensaver_since_ms:0,screen_fade_until:None,activity_dirty:false,restore_wait:None,sequence_edit:false,sequence_step_focus:false,sequence_step:0,sequence_row:0,sequence_target:String::new(),
            module_edit:false,module_cursor:0,module_page:0,module_drives:BTreeMap::new(),
            control_drives: BTreeMap::new(),
            pending_home_reset:false,shift_pressed: false, track_pressed:false, held_buttons:Default::default(),
            function_release_pending: false,
            pads: crate::pad_midi::Pads::default(),
            pad_panic,
            pad_epoch,
        };
        if !client.file_source { let _ = client.tx.try_send(Command::Refresh); }
        client
    }
    pub fn note_activity(&mut self){
        self.last_activity=Instant::now();
        if self.screensaver{self.screensaver=false;self.screensaver_since_ms=0;self.screen_fade_until=Some(Instant::now()+Duration::from_millis(800));self.activity_dirty=true;}
    }
    fn update_idle(&mut self)->bool{
        let now=Instant::now();
        if self.screen_fade_until.is_some_and(|until|now>=until){self.screen_fade_until=None;self.activity_dirty=true;}
        let idle=now.duration_since(self.last_activity);
        let should=self.observed.connected && idle>=Duration::from_secs(300);
        if should!=self.screensaver{
            self.screensaver=should;self.activity_dirty=true;self.screen_fade_until=Some(now+Duration::from_millis(800));
            self.screensaver_since_ms=if should{wall_ms()-(idle.as_millis().saturating_sub(300000) as i64)}else{0};
        }
        std::mem::take(&mut self.activity_dirty)
    }
    pub fn refresh_input(&mut self) -> bool {
        let changed = self.shared.lock().unwrap().generation != self.observed.generation;
        changed && self.tick()
    }
    pub fn tick(&mut self) -> bool {
        let latest = self.shared.lock().unwrap().clone();
        let changed = latest != self.observed;
        let old_notice = self.notice.clone();
        if latest.connected
            && latest.status.running
            && (!self.observed.connected || !self.observed.status.running)
        {
            self.notice = "G1 panel assignments; BROWSER opens patches".into();
        }
        if !latest.connected && self.observed.connected {
            self.restore_bank = self
                .observed
                .presets
                .get(self.cursor)
                .map(|p| (p.bank, p.position));
        }
        let reset = latest.instance != self.observed.instance || !latest.connected;
        if !latest.error.is_empty() {
            self.drives = std::array::from_fn(|_| None);
            self.control_drives.clear();self.module_drives.clear();
        }
        let module_changed=latest.module_binding!=self.observed.module_binding;
        let patch_changed = latest.status.patch_binding!=self.observed.status.patch_binding || latest.status.active_slot != self.observed.status.active_slot
            || latest.status.lcd[0] != self.observed.status.lcd[0];
        if self.restore_wait.is_some_and(|serial|latest.status.restore["serial"].as_u64().unwrap_or(0)>serial)||reset||patch_changed||!latest.error.is_empty(){self.restore_wait=None;}
        if latest.status.restore!=self.observed.status.restore && latest.status.restore["slot"].as_u64()==Some(u64::from(latest.status.active_slot)){
            if let Some(message)=latest.status.restore["message"].as_str().filter(|s|!s.is_empty()){self.notice=message.into();}
        }
        self.observed = latest;
        if module_changed || patch_changed || reset {self.module_cursor=0;self.module_page=0;self.module_drives.clear();}
        if reset {self.module_edit=false;if self.view=="sequencer"{self.view="parameters";}}
        if module_changed||patch_changed||reset{self.sequence_edit=false;self.sequence_step_focus=false;self.sequence_step=0;self.sequence_row=0;self.sequence_target.clear();}
        if self.view=="sequencer"{if self.observed.status.running&&self.observed.module_binding==self.observed.status.patch_binding&&self.sequence_indices().is_empty(){let _=self.go_home();}else{self.sync_sequence();}}
        if patch_changed {
            self.drives = std::array::from_fn(|_| None);
            self.control_drives.clear();
        }
        if reset {
            self.pending_home_reset=false;self.track_pressed=false;self.held_buttons.clear();
            self.pads.clear();
            self.drives = std::array::from_fn(|_| None);
            self.pending_slot = None;
            self.pending_file = None;
            self.pending_bank = None;
            self.control_drives.clear();
            self.shift_pressed = false;
            if !self.observed.connected {
                self.function_release_pending = false;
            }
        }
        if let Some(config) = &self.observed.status.configuration {
            self.control_drives.retain(|key,_|config.controls.iter().any(|c|c.key==*key));
            for (key, drive) in &mut self.control_drives {
                if drive.pending.is_some_and(|(value, deadline)| {
                    Instant::now() >= deadline
                        || config
                            .controls
                            .iter()
                            .any(|c| c.key == *key && c.value == value)
                }) {
                    drive.pending = None;
                }
            }
        }
        if let Some((serial, name, deadline, request)) = self.pending_file.clone() {
            if let Some((id, error)) = &self.observed.load_response {
                if *id == request && !error.is_empty() {
                    self.pending_file = None;
                    self.notice = format!("Load failed: {error}");
                }
            }
            if let Some(upload) = &self.observed.status.upload {
                if upload.serial > serial && !upload.busy {
                    self.notice = if upload.ok {
                        self.browser_open = false;
                        format!("Loaded {name}")
                    } else {
                        format!("Load failed: {}", upload.message)
                    };
                    self.pending_file = None;
                }
            }
            if self.pending_file.is_some() && Instant::now() >= deadline {
                self.pending_file = None;
                self.notice = "Patch upload was not confirmed; check G1-Emu".into();
            }
        }
        if let Some((serial, name, deadline, request)) = self.pending_bank.clone() {
            if let Some((id, error)) = &self.observed.load_response {
                if *id == request && !error.is_empty() {
                    self.pending_bank = None;
                    self.notice = format!("Load failed: {error}");
                }
            }
            if let Some(load) = &self.observed.status.bank_load {
                if load.serial > serial && !load.busy {
                    self.pending_bank = None;
                    self.notice = if load.ok {
                        self.browser_open = false;
                        format!("Loaded {name}")
                    } else {
                        format!("Load failed: {}", load.message)
                    };
                }
            }
            if self.pending_bank.is_some() && Instant::now() >= deadline {
                self.pending_bank = None;
                self.notice = "Bank load was not confirmed; check G1-Emu".into();
            }
        }
        let mut knob_timed_out = false;
        for (i, drive) in self.drives.iter_mut().enumerate() {
            if let Some(d) = drive {
                if self
                    .observed
                    .status
                    .knobs
                    .get(i)
                    .is_none_or(|k| k.binding != d.binding)
                {
                    *drive = None;
                    continue;
                }
                let knob = &self.observed.status.knobs[i];
                knob_timed_out |= d.pending.is_some_and(|(value, deadline)|
                    knob.value != value && Instant::now() >= deadline);
                if d.pending.is_some_and(|(value, deadline)| {
                    knob.value == value || Instant::now() >= deadline
                }) {
                    d.pending = None;
                }
            }
        }
        self.module_drives.retain(|key,_|self.observed.status.edit.as_ref().is_some_and(|m|m.parameters.iter().any(|p|&p.binding==key)));
        if let Some(edit)=&self.observed.status.edit {
            for p in &edit.parameters {
                if let Some(d)=self.module_drives.get_mut(&p.binding) {
                    knob_timed_out |= d.pending.is_some_and(|(value,until)|p.value!=value && Instant::now()>=until);
                    if d.pending.is_some_and(|(value,until)|p.value==value || Instant::now()>=until){d.pending=None;}
                }
            }
        }
        if self.notice.ends_with("(awaiting G1)") {
            if knob_timed_out {
                self.notice = "Parameter update not confirmed".into();
            } else if self.drives.iter().flatten().chain(self.module_drives.values()).all(|drive| drive.pending.is_none()) {
                self.notice.clear();
            }
        }
        if let Some((slot, deadline)) = self.pending_slot {
            if self.observed.status.active_slot == slot {
                self.pending_slot = None;
                self.notice=format!("Slot {} selected",char::from(b'A'+slot));
            } else if Instant::now() >= deadline {
                self.pending_slot = None;
                self.notice = "Slot selection was not confirmed".into();
            }
        }
        if let Some((bank, position)) = self.restore_bank {
            if let Some(index) = self
                .observed
                .presets
                .iter()
                .position(|p| p.bank == bank && p.position == position)
            {
                self.cursor = index;
                self.restore_bank = None;
            } else if self.observed.complete {
                self.restore_bank = None;
                self.cursor = 0;
            }
        }
        if self.observed.connected
            && self.observed.complete
            && self.cursor >= self.observed.presets.len()
        {
            self.cursor = 0;
        }
        if self.pending_home_reset && self.observed.connected && self.send(Command::Home).is_ok(){self.pending_home_reset=false;self.notice.clear();}
        if self.function_release_pending {
            let _ = self.function(false);
        }
        let midi=&self.observed.status.midi_activity;
        let activity=midi["activityTimestampMs"].as_i64().unwrap_or(0);
        let held=midi["activeNotes"].as_u64().unwrap_or(0)>0 && midi["connected"]!=false;
        if activity>self.last_midi_activity{self.last_midi_activity=activity;if wall_ms()-activity<300000{self.note_activity();}}
        if held{self.note_activity();}
        let idle_changed=self.update_idle();
        changed || idle_changed || self.notice != old_notice
    }
    fn send(&mut self, command: Command) -> Result<(), String> {
        let slot_control=(matches!(&command,Command::Slot(_)|Command::MixerMode(_)|Command::Function(false)) || matches!(&command,Command::Control(key,_) if key.starts_with("slotLevel:"))) && self.observed.status.slot_known;
        if !self.observed.connected || (!self.observed.status.running && !slot_control && !matches!(&command,Command::Home)) {
            return Err("G1-Emu is not connected/running".into());
        }
        self.tx
            .try_send(command)
            .map_err(|_| "G1 command queue is busy; retry".to_string())
    }
    fn selected_module(&self)->Option<&ModuleView>{
        if self.observed.module_binding!=self.observed.status.patch_binding{return None;}
        let selected=self.observed.modules.get(self.module_cursor)?;
        self.observed.status.edit.as_ref().filter(|m|m.section==selected.section && m.index==selected.index)
    }
    fn sequence_indices(&self)->Vec<usize>{if self.observed.module_binding!=self.observed.status.patch_binding{return Vec::new();}self.observed.modules.iter().enumerate().filter(|(_,m)|crate::sequencer::Kind::from_type(&m.type_name).is_some()).map(|(i,_)|i).collect()}
    fn sync_sequence(&mut self){
        if !self.observed.status.running||self.observed.module_binding!=self.observed.status.patch_binding{return;}
        let indices=self.sequence_indices();if indices.is_empty(){return;}
        if !indices.contains(&self.module_cursor){self.module_cursor=indices[0];}
        let entry=self.observed.modules[self.module_cursor].clone();let key=format!("{}:{}:{}",self.observed.module_binding,entry.section,entry.index);
        if key!=self.sequence_target{
            if self.send(Command::SelectModule(entry,self.observed.module_binding.clone())).is_ok(){self.sequence_target=key;}
        }
    }
    fn step_sequence(&mut self,delta:i32)->Result<bool,String>{
        let indices=self.sequence_indices();if indices.is_empty()||self.loading(){return Ok(false);}
        let current=indices.iter().position(|i|*i==self.module_cursor).unwrap_or(0);
        let next=(current as i64+i64::from(delta)).rem_euclid(indices.len() as i64) as usize;
        if indices[next]==self.module_cursor{return Ok(false);}
        self.module_cursor=indices[next];self.sequence_step=0;self.sequence_row=0;self.module_drives.clear();self.sync_sequence();Ok(true)
    }
    fn change_sequence_value(&mut self,delta:i32,toggle:bool)->Result<bool,String>{
        if !self.sequence_edit||self.restoring()||self.shift_pressed||self.loading(){return Ok(false);}
        let module=self.selected_module().ok_or("Waiting for sequencer parameters")?;
        let kind=crate::sequencer::Kind::from_type(&module.type_name).ok_or("Not a supported sequencer")?;
        if toggle!=(kind==crate::sequencer::Kind::Event){return Ok(false);}
        let index=kind.parameter(self.sequence_row,self.sequence_step).ok_or("Invalid sequencer step")?;
        let p=module.parameters.iter().find(|p|p.index==index).ok_or("Step unavailable")?.clone();let section=module.section;let module=module.index;
        let base=self.module_drives.get(&p.binding).and_then(|d|d.pending).filter(|(_,until)|Instant::now()<*until).map_or(p.value,|(value,_)|value);
        let target=if toggle{if base==0{1}else{0}}else{(i64::from(base)+i64::from(delta)).clamp(i64::from(p.min),i64::from(p.max)) as u8};
        if target==base{return Ok(false);}
        self.send(Command::Parameter{input_seq:crate::input_dispatch::input_sequence(),section,module,index,value:target,binding:p.binding.clone()})?;
        self.module_drives.insert(p.binding.clone(),Drive{binding:p.binding,remainder:0.,pending:Some((target,Instant::now()+Duration::from_millis(500)))});Ok(true)
    }
    fn sequence_snapshot(&self)->Value{
        let indices=self.sequence_indices();let selected=self.selected_module();let kind=selected.and_then(|m|crate::sequencer::Kind::from_type(&m.type_name));
        let list=indices.iter().map(|i|{let m=&self.observed.modules[*i];json!({"name":m.name,"type":m.type_name,"section":m.section,"selected":*i==self.module_cursor})}).collect::<Vec<_>>();
        let mut rows=Vec::new();let mut length=16;
        if let (Some(m),Some(kind))=(selected,kind){
            if let Some(p)=m.parameters.iter().find(|p|p.index==kind.length_parameter()){length=usize::from(p.value)+1;}
            for row in 0..kind.rows(){let mut steps=Vec::new();for step in 0..16{
                let p=kind.parameter(row,step).and_then(|i|m.parameters.iter().find(|p|p.index==i));
                steps.push(if let Some(p)=p{let (value,_,pending)=self.feedback(&p.binding,p.value,&p.display,self.module_drives.get(&p.binding));json!({"value":value,"display":kind.display(value),"pending":pending,"available":true})}else{json!({"available":false})});
            }rows.push(steps);}
        }
        json!({"stepFocus":self.sequence_step_focus&&self.sequence_edit,"editing":self.sequence_edit,"step":self.sequence_step,"row":self.sequence_row,"ready":kind.is_some(),"playhead":selected.and_then(|m|m.playhead).filter(|step|*step<16),"kind":kind.map(|k|k.name()),"name":selected.map(|m|m.name.as_str()).unwrap_or(""),"length":length,"list":list,"rows":rows})
    }
    fn module_pages(&self)->usize {self.selected_module().map_or(1,|m|m.parameters.len().div_ceil(8).max(1))}
    fn step_module(&mut self,delta:i32)->Result<bool,String>{
        if self.loading(){return Err("Wait for the patch load".into());}
        let count=self.observed.modules.len();if count==0{return Ok(false);}
        let next=(self.module_cursor as i64+i64::from(delta)).rem_euclid(count as i64) as usize;
        if next==self.module_cursor{return Ok(false);}
        let entry=self.observed.modules[next].clone();self.send(Command::SelectModule(entry,self.observed.module_binding.clone()))?;
        self.module_cursor=next;self.module_page=0;self.module_drives.clear();Ok(true)
    }
    fn turn_module(&mut self,index:usize,delta:f64)->Result<bool,String>{
        if self.loading(){return Err("Wait for the patch load".into());}
        let module=self.selected_module().ok_or("Waiting for module parameters")?;
        let p=module.parameters.get(self.module_page*8+index).ok_or("No parameter on this encoder")?.clone();let section=module.section;let module_index=module.index;
        let d=self.module_drives.entry(p.binding.clone()).or_insert_with(||Drive{binding:p.binding.clone(),remainder:0.,pending:None});
        let base=d.pending.filter(|(_,until)|Instant::now()<*until).map_or(p.value,|(value,_)|value);
        let movement=d.remainder+delta.clamp(-1.,1.)*ENCODER_SENSITIVITY*f64::from((p.max-p.min).max(32));
        let target=(f64::from(base)+movement.trunc()).clamp(f64::from(p.min),f64::from(p.max)) as u8;
        d.remainder=if (target==p.min && movement<0.) || (target==p.max && movement>0.){0.}else{movement-movement.trunc()};
        if target==base{return Ok(false);}
        let input_seq=crate::input_dispatch::input_sequence();
        if crate::input_dispatch::pipeline_trace_enabled(){tracing::info!("PIPELINE stage=target input_seq={} module={} param={} binding={} value={}",input_seq,module_index,p.index,p.binding,target);}
        self.send(Command::Parameter{input_seq,section,module:module_index,index:p.index,value:target,binding:p.binding.clone()})?;
        self.module_drives.get_mut(&p.binding).unwrap().pending=Some((target,Instant::now()+Duration::from_millis(500)));
        self.notice=format!("{} (awaiting G1)",p.name);Ok(true)
    }
    fn restoring(&self)->bool{self.restore_wait.is_some()||self.observed.status.restore["busy"]==true}
    fn turn(&mut self, index: usize, delta: f64) -> Result<bool, String> {
        self.note_activity();
        if self.view=="sequencer"||self.restoring(){return Ok(false);}
        if index >= 8 || !delta.is_finite() {
            return Err("Invalid encoder movement".into());
        }
        if self.view=="display" {
            if index==1{
                if delta==0.{return Ok(false);}
                let next=delta>0.;let changed=self.blobs_enabled!=next;self.blobs_enabled=next;return Ok(changed);
            }
            if index==2 && self.ui_theme.round()!=2.{let next=(self.glitch_intensity+delta*2.).clamp(0.,1.);let changed=next!=self.glitch_intensity;self.glitch_intensity=next;return Ok(changed);}
            if index==0 {
                let previous=self.ui_theme.round();self.ui_theme=(self.ui_theme+delta*4.).clamp(0.,2.);return Ok(previous!=self.ui_theme.round());
            }
            if index!=3 || self.ui_theme.round()!=2.{return Ok(false);}
            let next=(self.ui_colour_intensity+delta).clamp(0.,1.);
            let changed=next!=self.ui_colour_intensity;self.ui_colour_intensity=next;return Ok(changed);
        }
        if self.shift_pressed {return Ok(false);}
        if self.module_edit && !self.browser_open && self.view=="parameters" {return self.turn_module(index,delta);}
        if self.view != "parameters" {
            return self.turn_control(index, delta);
        }
        if self.shift_pressed { return Ok(false); }
        if self.pending_slot.is_some() {
            return Err("Waiting for G1 slot selection".into());
        }
        if self.loading() {
            return Err("Wait for the patch upload to finish".into());
        }
        let physical = self.page * 8 + index;
        let k = self
            .observed
            .status
            .knobs
            .get(physical)
            .filter(|k| k.assigned)
            .ok_or("No G1 parameter assigned to this panel knob")?
            .clone();
        let drive = self.drives[physical].get_or_insert_with(|| Drive {
            binding: k.binding.clone(),
            remainder: 0.,
            pending: None,
        });
        if drive.binding != k.binding {
            *drive = Drive {
                binding: k.binding.clone(),
                remainder: 0.,
                pending: None,
            };
        }
        let base = drive
            .pending
            .filter(|(_, deadline)| Instant::now() < *deadline)
            .map_or(k.value, |(value, _)| value);
        let movement =
            drive.remainder + delta.clamp(-1., 1.) * ENCODER_SENSITIVITY * f64::from(k.max.max(32));
        let steps = movement.trunc();
        let target = (f64::from(base) + steps).clamp(0., f64::from(k.max)) as u8;
        drive.remainder = if target == 0 && movement < 0. || target == k.max && movement > 0. {
            0.
        } else {
            movement - steps
        };
        if target == base {
            return Ok(false);
        }
        let input_seq=crate::input_dispatch::input_sequence();
        if crate::input_dispatch::pipeline_trace_enabled(){tracing::info!("PIPELINE stage=target input_seq={} knob={} binding={} value={}",input_seq,physical,k.binding,target);}
        self.send(Command::Set {
            input_seq,
            index: physical,
            value: target,
            binding: k.binding.clone(),
        })?;
        self.drives[physical].as_mut().unwrap().pending =
            Some((target, Instant::now() + Duration::from_millis(500)));
        self.notice = format!("{} · {} (awaiting G1)", k.module, k.parameter);
        Ok(true)
    }
    fn loading(&self) -> bool {
        self.pending_file.is_some()
            || self.pending_bank.is_some()
            || self.observed.status.upload.as_ref().is_some_and(|u| u.busy)
            || self
                .observed
                .status
                .bank_load
                .as_ref()
                .is_some_and(|u| u.busy)
    }
    fn control_keys(&self) -> Vec<&str> {
        match self.view {
            "mixer" => {
                let controls:Vec<_>=self.observed.status.configuration.iter().flat_map(|c|c.controls.iter()).collect();
                let levels:Vec<_>=controls.iter().filter(|c|c.key.starts_with("slotLevel:")).map(|c|if c.max>c.min{c.key.as_str()}else{""}).collect();
                if levels.len()==4{vec![levels[0],levels[1],levels[2],levels[3],controls.iter().find(|c|c.key=="master").map_or("",|c|c.key.as_str()),"","",""]}else if controls.iter().any(|c|c.key=="master"){vec!["master","gain"]}else{vec!["";8]}
            },
            "settings" => match self.settings_page {
                0 => vec![
                    "gain", "bpm", "tune", "sync", "midi0", "midi1", "midi2", "midi3",
                ],
                1 => vec![
                    "clock",
                    "local",
                    "knobMode",
                    "velMin",
                    "velMax",
                    "pcReceive",
                    "pcSend",
                    "keyboard",
                ],
                _ => vec!["pedal", "leds"],
            },
            _ => vec![],
        }
    }
    fn turn_control(&mut self, index: usize, delta: f64) -> Result<bool, String> {
        if self.loading() {
            return Err("Wait for patch loading".into());
        }
        let key = self.control_keys().get(index).ok_or("No setting assigned to this encoder")?.to_string();
        if key.is_empty(){return Ok(false);}
        let c = self
            .observed
            .status
            .configuration
            .as_ref()
            .and_then(|config| config.controls.iter().find(|c| c.key == key))
            .ok_or("Restart updated G1-Emu for real Settings/Mixer controls")?
            .clone();
        let drive = self.control_drives.entry(key.clone()).or_default();
        let base = drive
            .pending
            .filter(|(_, deadline)| Instant::now() < *deadline)
            .map_or(c.value, |(v, _)| v);
        let movement = drive.remainder
            + delta.clamp(-1., 1.) * ENCODER_SENSITIVITY * f64::from((c.max - c.min).max(16));
        let steps = movement.trunc();
        let value = (f64::from(base) + steps).clamp(f64::from(c.min), f64::from(c.max)) as i32;
        drive.remainder = movement - steps;
        if value == base {
            return Ok(false);
        }
        self.send(Command::Control(key.clone(), value))?;
        self.control_drives.get_mut(&key).unwrap().pending =
            Some((value, Instant::now() + Duration::from_secs(5)));
        self.notice = if key.starts_with("slotLevel:"){String::new()}else{format!("{} (awaiting emulator)", c.label)};
        Ok(true)
    }
    fn function(&mut self, pressed: bool) -> Result<bool, String> {
        if self.observed.status.configuration.is_none() {
            return Err("Restart updated G1-Emu for Function/Shift".into());
        }
        if let Err(error) = self.send(Command::Function(pressed)) {
            if !pressed {
                self.function_release_pending = true;
            }
            return Err(error);
        }
        self.shift_pressed = pressed;
        if pressed { for drive in self.drives.iter_mut().flatten() { drive.pending = None; drive.remainder = 0.; } self.module_drives.clear(); }
        if !pressed {
            self.function_release_pending = false;
        }
        Ok(true)
    }
    pub fn release_function(&mut self) {
        self.track_pressed=false;self.held_buttons.clear();
        if self.shift_pressed
            || self
                .observed
                .status
                .configuration
                .as_ref()
                .is_some_and(|c| c.function)
        {
            let _ = self.function(false);
        }
    }
    fn step_preset(&mut self, delta: i32) -> Result<bool, String> {
        if self.loading() || self.pending_slot.is_some() {
            return Err("Wait for the current load/slot change".into());
        }
        if self.file_source {
            if self.files.entries.is_empty() {
                self.files.refresh()?;
            }
            let candidate = if delta > 0 {
                (self.files.cursor + 1..self.files.entries.len())
                    .find(|&i| !self.files.entries[i].directory)
            } else {
                (0..self.files.cursor)
                    .rev()
                    .find(|&i| !self.files.entries[i].directory)
            };
            let Some(index) = candidate else {
                return Ok(false);
            };
            self.files.cursor = index;
            let path = self.files.enter()?.ok_or("Select a patch file")?;
            self.load_file(path)
        } else {
            let before = self.cursor;
            self.scroll(delta);
            if before == self.cursor {
                return Ok(false);
            }
            self.load_selected()
        }
    }
    fn scroll(&mut self, delta: i32) -> bool {
        if self.browser_open && self.file_source {
            self.files.move_by(delta);
        } else {
            self.restore_bank = None;
            self.cursor = (self.cursor as i64 + i64::from(delta))
                .clamp(0, self.observed.presets.len().saturating_sub(1) as i64)
                as usize;
        }
        true
    }
    fn load_file(&mut self, path: PathBuf) -> Result<bool, String> {
        if self.pending_slot.is_some() {
            return Err("Waiting for G1 slot selection".into());
        }
        if self.loading() {
            return Err("Wait for the patch upload to finish".into());
        }
        let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        if !path.is_file()
            || !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pch"))
        {
            return Err("Select a local .pch file for G1-Emu to load".into());
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let serial = self
            .observed
            .status
            .upload
            .as_ref()
            .ok_or("Restart the updated G1-Emu to enable Browser file loading")?
            .serial;
        self.load_request = self.load_request.wrapping_add(1);
        self.send(Command::File(
            path.to_str().ok_or("Patch path is not UTF-8")?.into(),
            self.observed.status.active_slot,
            self.load_request,
        ))?;
        self.pending_file = Some((
            serial,
            name.clone(),
            Instant::now() + Duration::from_secs(30),
            self.load_request,
        ));
        self.notice = format!("Loading {name}…");
        Ok(true)
    }
    fn load_selected(&mut self) -> Result<bool, String> {
        if self.pending_slot.is_some() {
            return Err("Waiting for G1 slot selection".into());
        }
        if self.loading() {
            return Err("Wait for the patch upload to finish".into());
        }
        if self.browser_open && self.file_source {
            return match self.files.enter()? {
                Some(path) => self.load_file(path),
                None => Ok(true),
            };
        }
        let preset = self
            .observed
            .presets
            .get(self.cursor)
            .ok_or("No G1 bank patch selected")?
            .clone();
        let serial = self
            .observed
            .status
            .bank_load
            .as_ref()
            .ok_or("Restart G1-Emu for confirmed bank loading")?
            .serial;
        self.load_request = self.load_request.wrapping_add(1);
        let name = preset.name.clone();
        self.send(Command::Load(
            preset,
            self.observed.status.active_slot,
            self.load_request,
        ))?;
        self.pending_bank = Some((
            serial,
            name.clone(),
            Instant::now() + Duration::from_secs(10),
            self.load_request,
        ));
        self.notice = format!("Loading {name}…");
        Ok(true)
    }
    fn save_preferences(&self){
        if let Some(path) = &self.browser_state {
            let bank = self.restore_bank.or_else(|| {
                self.observed
                    .presets
                    .get(self.cursor)
                    .map(|p| (p.bank, p.position))
            });
            if let Err(error) = crate::browser_state::Preferences::save(
                path,
                &self.library_root,
                &self.files,
                self.file_source,
                bank,
                (self.ui_colour_intensity*100.).round() as u8,
                self.ui_theme.round() as u8,
                self.blobs_enabled,(self.glitch_intensity*100.).round() as u8,
            ) {
                eprintln!("Browser position was not saved: {error}");
            }
        }
    }
    fn go_home(&mut self)->Result<bool,String>{
        if self.view=="display"{self.save_preferences();}
        let reset=self.pending_home_reset || self.module_edit || self.view!="parameters";
        self.sequence_edit=false;self.sequence_step_focus=false;self.sequence_step=0;self.sequence_row=0;self.sequence_target.clear();
        self.module_edit=false;self.module_cursor=0;self.module_page=0;self.browser_open=false;self.view="parameters";
        self.module_drives.clear();self.control_drives.clear();self.drives=std::array::from_fn(|_|None);self.notice.clear();
        if reset && self.observed.connected{self.pending_home_reset=true;self.send(Command::Home)?;self.pending_home_reset=false;}
        Ok(true)
    }
    fn button(&mut self, name: &str) -> Result<bool, String> {
        self.note_activity();
        if name=="clear" {
            if self.shift_pressed||self.restoring()||self.loading()||self.pending_slot.is_some()||self.observed.status.restore["available"]!=true{return Ok(false);}
            self.send(Command::Restore(self.observed.status.active_slot,self.observed.status.patch_binding.clone()))?;
            self.restore_wait=Some(self.observed.status.restore["serial"].as_u64().unwrap_or(0));
            self.drives=std::array::from_fn(|_|None);self.module_drives.clear();self.control_drives.clear();self.notice="Restoring original slot values…".into();return Ok(true);
        }
        if matches!(name, "pad_mode" | "keyboard" | "stop") && !self.observed.status.pad_midi {
            return Ok(false);
        }
        if matches!(name, "pad_mode" | "keyboard") {
            self.release_pads();
            self.pads.mode = if name == "keyboard" {
                crate::pad_midi::Mode::Keyboard
            } else {
                crate::pad_midi::Mode::Drums
            };
            self.notice = format!(
                "MIDI {} · notes {}–{}",
                self.pads.mode.name(),
                self.pads.mode.base(),
                self.pads.mode.base() + 15
            );
            return Ok(true);
        }
        if name == "stop" {
            self.release_pads();
            return Ok(true);
        }

        if self.track_pressed && name=="top_7" {
            self.go_home()?;self.view="settings";return Ok(true);
        }
        if self.track_pressed && name=="top_8" {
            self.go_home()?;self.view="display";return Ok(true);
        }
        if self.track_pressed && matches!(name,"top_1"|"top_2"|"top_3"|"top_4") {
            let slot=name.as_bytes()[4]-b'1';
            self.module_drives.clear();self.drives=std::array::from_fn(|_|None);
            return self.button(["group_a","group_b","group_c","group_d"][usize::from(slot)]);
        }
        if name=="pattern"{
            if self.shift_pressed{return Ok(false);}
            let enter=self.view!="sequencer";if enter&&self.sequence_indices().is_empty(){return Ok(false);}self.go_home()?;
            if enter{self.send(Command::ModuleMode(true))?;self.module_edit=true;self.view="sequencer";self.sync_sequence();}return Ok(true);
        }
        if self.view=="sequencer"&&!self.track_pressed{
            match name{
                "top_5"=>{if self.selected_module().and_then(|m|crate::sequencer::Kind::from_type(&m.type_name)).is_none(){return Ok(false);}self.sequence_edit=!self.sequence_edit;self.sequence_step_focus=self.sequence_edit;return Ok(true);},
                "browser_select"=>return self.change_sequence_value(0,true),
                "encoder_left"|"encoder_right"=>{
                    if !self.sequence_edit{return Ok(false);}
                    self.sequence_step=(self.sequence_step+if name=="encoder_left"{15}else{1})%16;return Ok(true);
                },
                "encoder_up"|"encoder_down"=>{if !self.sequence_step_focus{return self.step_sequence(if name=="encoder_up"{-1}else{1});}if self.selected_module().is_some_and(|m|m.type_name=="EventSeq"){self.sequence_row=if name=="encoder_up"{0}else{1};return Ok(true);}return Ok(false);},
                _=>{}
            }
        }
        match name {
            "module_edit"=>{
                if self.shift_pressed{return Ok(false);}
                let enter=!(self.module_edit&&self.view=="parameters"&&!self.browser_open);
                self.go_home()?;
                if enter{self.send(Command::ModuleMode(true))?;self.module_edit=true;}
            }
            "top_1" | "encoder_up" if self.module_edit && !self.browser_open && self.view=="parameters"=>return self.step_module(-1),
            "top_2" | "encoder_down" if self.module_edit && !self.browser_open && self.view=="parameters"=>return self.step_module(1),
            "browser_select" if self.module_edit && !self.browser_open && self.view=="parameters"=>return Ok(false),
            "encoder_left" | "encoder_right" if self.module_edit && !self.browser_open && self.view=="parameters"=>{
                let pages=self.module_pages();self.module_page=(self.module_page+if name=="encoder_left"{pages-1}else{1})%pages;
            }
            "browser_select" if self.browser_open => return self.load_selected(),
            "browser_select" => return self.button("browser"),
            "page_or_preset_previous" | "page_or_preset_next" => {
                let previous = name == "page_or_preset_previous";
                if self.held_buttons.contains("shift") {
                    return self.step_preset(if previous { -1 } else { 1 });
                }
                return self.button(if previous { "knob_page_previous" } else { "knob_page_next" });
            }
            "knob_page_previous" | "knob_page_next" => {
                if self.view!="parameters"||self.browser_open{self.go_home()?;}
                if self.module_edit {let pages=self.module_pages();self.module_page=(self.module_page+if name=="knob_page_previous"{pages-1}else{1})%pages;}
                else {self.page = (self.page + if name == "knob_page_previous" { 2 } else { 1 }) % 3;}
                self.view = "parameters";
                self.browser_open = false;
            }
            "shift" => return self.function(true),
            "settings" | "mixer" => {
                if self.shift_pressed{return Ok(false);}
                let enter=self.view!=name;
                self.go_home()?;
                if enter{if name=="mixer"{self.send(Command::MixerMode(true))?;}self.view=if name=="mixer"{"mixer"}else{"settings"};}
            }
            // Legacy terminal fixture command. The Maschine profile drops physical MACRO input.
            "macro" => {
                self.go_home()?;
                self.page = (self.page + 1) % 3;
            }
            "arrow_left" => return self.step_preset(-1),
            "arrow_right" => return self.step_preset(1),
            "top_1" | "top_2" | "top_3" if self.view == "settings" => {
                self.settings_page = name[4..].parse::<usize>().unwrap() - 1;
            }
            "top_4" if self.view == "settings" => {
                self.send(Command::SettingsUi)?;
            }
            "browser" => {
                let open=!self.browser_open;self.go_home()?;self.browser_open=open;
                if open{if self.file_source{self.files.refresh()?;}else{self.send(Command::Refresh)?;}}
            }
            "top_1" if self.browser_open => {
                self.file_source = true;
                self.files.refresh()?;
            }
            "top_2" if self.browser_open => {
                self.file_source = false;
                self.send(Command::Refresh)?;
            }
            "top_3" if self.browser_open => {
                if self.file_source {
                    self.files.refresh()?;
                } else {
                    self.send(Command::Refresh)?;
                }
            }
            "group_a" | "group_b" | "group_c" | "group_d" => {
                if self.loading() {
                    return Err("Wait for the patch upload to finish".into());
                }
                let slot = name.as_bytes()[6] - b'a';
                if slot == self.observed.status.active_slot
                    && self.observed.connected
                    && self.pending_slot.is_none()
                    && !self.shift_pressed
                {
                    return Ok(false);
                }
                self.send(Command::Slot(slot))?;
                if !self.shift_pressed {
                    self.pending_slot = Some((slot, Instant::now() + Duration::from_secs(2)));
                    self.notice=format!("Selecting slot {}…",char::from(b'A'+slot));
                }
            }
            "encoder_left" | "encoder_right" if !self.browser_open && self.view == "parameters" => {
                return self.button(if name == "encoder_left" { "knob_page_previous" } else { "knob_page_next" });
            }
            "encoder_left" if self.browser_open && self.file_source => {
                self.files.parent()?;
            }
            "encoder_up" | "encoder_left" => {
                self.scroll(-1);
            }
            "encoder_down" | "encoder_right" => {
                self.scroll(1);
            }
            "top_4" | "encoder_press" if self.browser_open => return self.load_selected(),
            _ => return Ok(false),
        }
        Ok(true)
    }
    pub fn release_pads(&mut self) {
        self.pads.clear();
        self.pad_epoch.fetch_add(1, Ordering::AcqRel);
        self.pad_panic.store(true, Ordering::Release);
    }
    fn pad(&mut self, name: &str, pressure: f32) -> Result<bool, String> {
        if !self.observed.status.pad_midi {
            return Ok(false);
        }
        if pressure > 0. && (self.loading() || self.pending_slot.is_some()) {
            return Ok(false);
        }
        let Some((pad, note, velocity)) = self.pads.change(name, pressure) else {
            return Ok(false);
        };
        if let Err(error) = self.send(Command::Pad {
            epoch: self.pad_epoch.load(Ordering::Acquire),
            pad,
            slot: self.observed.status.active_slot,
            note,
            velocity,
        }) {
            // A release must survive a full ordinary command queue.
            self.release_pads();
            return Err(error);
        }
        Ok(true)
    }
    pub fn event(&mut self, event: &Event) -> bool {
        if !matches!(event,Event::DeviceConnected{..}|Event::DeviceDisconnected{..}){self.note_activity();}
        let held_changed=if let Event::Button{name,pressed,..}=event {
            if *pressed {self.held_buttons.insert((*name).into())}else{self.held_buttons.remove(*name)}
        }else{false};
        let result = match event {
            Event::Button{name:"track",pressed,..}=>{self.track_pressed=*pressed;if *pressed{self.go_home()}else{Ok(true)}},
            Event::Grid { name, pressure, .. } => self.pad(name, *pressure),
            Event::Button {
                name,
                pressed: false,
                ..
            } if crate::pad_midi::Pads::index(name).is_some() => self.pad(name, 0.),
            Event::Button {
                name: "shift",
                pressed,
                ..
            } => self.function(*pressed),
            Event::Encoder{name:"main_encoder",delta,..} if self.view=="sequencer"=>{
                if self.sequence_edit&&self.selected_module().is_some_and(|m|m.type_name!="EventSeq"){self.change_sequence_value(*delta,false)}
                else if self.sequence_step_focus{let next=(self.sequence_step as i64+i64::from(*delta)).rem_euclid(16) as usize;let changed=next!=self.sequence_step;self.sequence_step=next;Ok(changed)}
                else{self.step_sequence(*delta)}
            },
            Event::Encoder{name:"main_encoder",delta,..} if self.module_edit && !self.browser_open && self.view=="parameters"=>self.step_module(*delta),
            Event::Encoder {
                name: "main_encoder",
                delta,
                ..
            } if self.browser_open => Ok(self.scroll(*delta)),
            Event::EncoderFine { name, delta, .. } => name
                .strip_prefix("screen_encoder_")
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| (1..=8).contains(n))
                .map_or(Ok(false), |n| self.turn(n - 1, f64::from(*delta))),
            Event::Button {
                name,
                pressed: true,
                ..
            } => self.button(name),
            _ => Ok(false),
        };
        match result {
            Ok(changed) => changed || held_changed,
            Err(error) => {
                self.notice = error;
                true
            }
        }
    }
    pub fn command(&mut self, command: &str) -> Result<bool, String> {
        if command == "refresh-presets" {
            self.send(Command::Refresh)?;
            return Ok(true);
        }
        if let Some(path) = command.strip_prefix("load-patch ") {
            return self.load_file(PathBuf::from(path.trim()));
        }
        let parts: Vec<_> = command.split_whitespace().collect();
        match parts.as_slice() {
            ["idle-for",seconds]=>{let seconds=seconds.parse::<u64>().map_err(|_|"Invalid idle age")?;if seconds>86400{return Err("Idle fixture limit is one day".into());}self.last_activity=Instant::now().checked_sub(Duration::from_secs(seconds)).ok_or("Invalid idle age")?;if self.screensaver&&seconds>=300{self.screensaver_since_ms=wall_ms()-((seconds-300)*1000) as i64;self.activity_dirty=true;}Ok(self.update_idle())},
            [action @ ("hold"|"release"), "track"] => Ok(self.event(&Event::Button{device:encdr::DeviceId::default(),name:"track",pressed:*action=="hold"})),
            ["turn", index, delta] => {
                let index = index.parse::<usize>().map_err(|_| "Invalid encoder")?;
                if !(1..=8).contains(&index) {
                    return Err("Encoder must be 1–8".into());
                }
                self.turn(index - 1, delta.parse().map_err(|_| "Invalid delta")?)
            }
            ["wheel",delta]=>{let delta=delta.parse::<i32>().map_err(|_|"Invalid wheel movement")?;Ok(self.event(&Event::Encoder{device:encdr::DeviceId::default(),name:"main_encoder",delta}))},
            ["button", name] => self.button(name),
            ["pad", number, pressure] => {
                let name = format!("pad_{number}");
                if crate::pad_midi::Pads::index(&name).is_none() {
                    return Err("Pad must be 1–16".into());
                }
                self.pad(&name, pressure.parse().map_err(|_| "Invalid pad pressure")?)
            }
            ["panic"] => {
                self.release_pads();
                Ok(true)
            }
            ["release", "shift"] => self.function(false),
            ["scroll", delta] if self.browser_open => {
                Ok(self.scroll(delta.parse().map_err(|_| "Invalid scroll steps")?))
            }
            _ => Err("G1 mode: use turn, button, load-patch PATH.pch, or refresh-presets".into()),
        }
    }
    fn feedback(&self,binding:&str,actual:u8,display:&str,drive:Option<&Drive>)->(u8,String,bool){
        let target=drive.filter(|d|d.binding==binding).and_then(|d|d.pending).filter(|(_,until)|Instant::now()<*until).map(|(value,_)|value);
        if let Some(target)=target.filter(|value|*value!=actual){
            let formatted=self.observed.previews.get(binding).filter(|p|p.value==target).map_or_else(||display.to_string(),|p|p.display.clone());
            (target,formatted,true)
        }else{(actual,display.to_string(),false)}
    }
    /// Button state without allocating the browser, parameter and sequence payloads.
    pub fn leds(&self) -> Value {
        let o = &self.observed;
        let s = &o.status;
        let running = o.connected && s.running;
        let slot_known = o.connected && (s.slot_known || s.running);
        let mut leds = BTreeMap::<String, u8>::new();
        if slot_known {
            for slot in 0..4 {
                let active = usize::from(s.active_slot) == slot;
                if active || !self.loading() {
                    leds.insert(format!("group_{}", char::from(b'a' + slot as u8)), if active { 80 } else { 30 });
                }
            }
        }
        if self.track_pressed {leds.insert("top_7".into(),if self.view=="settings"{80}else{30});leds.insert("top_8".into(),if self.view=="display"{80}else{30});}
        if running {
            leds.insert("module_edit".into(),if self.module_edit&&self.view=="parameters"&&!self.browser_open{80}else{30});
            leds.insert("mixer".into(),if self.view=="mixer"{80}else{30});
            leds.insert("settings".into(), if self.view == "settings" { 80 } else { 30 });
            if self.module_edit && !self.browser_open && self.view=="parameters" {
                if self.observed.modules.len()>1{leds.insert("top_1".into(),30);leds.insert("top_2".into(),30);}
            }
            if s.pad_midi {
                leds.insert(
                    if self.pads.mode == crate::pad_midi::Mode::Drums {
                        "pad_mode"
                    } else {
                        "keyboard"
                    }
                    .into(),
                    80,
                );
                if !s.pad_midi_held.is_empty() {
                    leds.insert("stop".into(), 50);
                }
                for pad in &s.pad_midi_held {
                    if *pad < 16 {
                        leds.insert(format!("pad_{}", pad + 1), 80);
                    }
                }
            }
            if s.configuration.as_ref().is_some_and(|c| c.function) {
                leds.insert("shift".into(), 80);
            }
            if self.browser_open {
                leds.insert("top_1".into(), 30);
                leds.insert("top_2".into(), 30);
                leds.insert(if self.file_source { "top_1" } else { "top_2" }.into(), 80);
                leds.insert("top_3".into(), 50);
                let selected = if self.file_source {
                    self.files.entries.get(self.files.cursor).is_some()
                } else {
                    self.observed.presets.get(self.cursor).is_some()
                };
                if !self.loading() && self.pending_slot.is_none() && selected {
                    leds.insert("top_4".into(), 50);
                }
            }
            if self.view == "settings" {
                for button in ["top_1", "top_2", "top_3"] {
                    leds.insert(button.into(), 30);
                }
                leds.insert(format!("top_{}", self.settings_page + 1), 80);
                leds.insert("top_4".into(), 50);
            }
            if !self.loading() && self.pending_slot.is_none() {
                let (previous, next) = if self.file_source {
                    (
                        self.files
                            .entries
                            .iter()
                            .take(self.files.cursor)
                            .any(|e| !e.directory),
                        self.files
                            .entries
                            .iter()
                            .skip(self.files.cursor + 1)
                            .any(|e| !e.directory),
                    )
                } else {
                    (self.cursor > 0, self.cursor + 1 < o.presets.len())
                };
                if previous {
                    leds.insert("arrow_left".into(), 50);
                }
                if next {
                    leds.insert("arrow_right".into(), 50);
                }
            }
        }
        if running {
            leds.insert("browser".into(),if self.browser_open{80}else{30});
            if s.configuration.is_some(){leds.insert("shift".into(),if self.shift_pressed{80}else{30});}
            leds.insert("track".into(),if self.track_pressed{80}else if !self.loading(){30}else{0});
            let pages=if self.module_edit{self.module_pages()}else{3};
            if pages>1 {leds.insert("knob_page_previous".into(),30);leds.insert("knob_page_next".into(),30);}
            let (previous, next) = if self.held_buttons.contains("shift") {
                ("arrow_left", "arrow_right")
            } else {
                ("knob_page_previous", "knob_page_next")
            };
            for (button, source) in [("page_or_preset_previous", previous), ("page_or_preset_next", next)] {
                if let Some(&level) = leds.get(source) {
                    leds.insert(button.into(), level);
                }
            }
            if self.track_pressed {
                for slot in 0..4 {leds.insert(format!("top_{}",slot+1),if self.loading(){0}else if slot==usize::from(s.active_slot){80}else{30});}
            }
        }
        if slot_known && !running {
            leds.insert("mixer".into(),if self.view=="mixer"{80}else{30});
            leds.insert("track".into(),if self.track_pressed{80}else{30});
            if self.track_pressed{for slot in 0..4{leds.insert(format!("top_{}",slot+1),if slot==usize::from(s.active_slot){80}else{30});}}
        }
        if running&&!self.sequence_indices().is_empty(){leds.insert("pattern".into(),if self.view=="sequencer"{80}else{30});}
        if running&&self.view=="sequencer"&&!self.track_pressed&&self.selected_module().and_then(|m|crate::sequencer::Kind::from_type(&m.type_name)).is_some(){leds.insert("top_5".into(),if self.sequence_edit{80}else{30});}
        if running&&!self.shift_pressed&&s.restore["available"]==true&&!self.loading()&&self.pending_slot.is_none(){leds.insert("clear".into(),if self.restoring(){80}else{30});}
        for name in &self.held_buttons {
            if self.track_pressed && matches!(name.as_str(), "top_1" | "top_2" | "top_3" | "top_4") {
                continue;
            }
            if let Some(level) = leds.get_mut(name) {
                *level = 80;
            }
        }
        json!(leds)
    }
    pub fn snapshot(&self) -> Value {
        let o = &self.observed;
        let s = &o.status;
        let running = o.connected && s.running;
        let slot_known=o.connected && (s.slot_known || s.running);
        let mut labels = vec!["—".to_string(); 8];
        let mut values = vec![0.; 8];
        let mut display = vec!["—".to_string(); 8];
        let mut confirmed_display=vec!["—".to_string();8];let mut pending_values=vec![false;8];
        if running {
            for (i, k) in s.knobs.iter().skip(self.page * 8).take(8).enumerate() {
                labels[i] = if k.assigned {
                    format!("{} · {}", k.module, k.parameter)
                } else {
                    "UNASSIGNED".into()
                };
                confirmed_display[i]=if k.assigned{k.display.clone()}else{"—".into()};
                let (value,formatted,pending)=self.feedback(&k.binding,k.value,&k.display,self.drives[self.page*8+i].as_ref());
                values[i]=if k.assigned && k.max>0{f64::from(value)/f64::from(k.max)}else{0.};
                display[i]=if k.assigned{formatted}else{"—".into()};pending_values[i]=k.assigned&&pending;
            }
        }
        if self.module_edit {
            labels.fill("UNASSIGNED".into());values.fill(0.);display.fill("—".into());confirmed_display.fill("—".into());pending_values.fill(false);
            if let Some(module)=self.selected_module(){
                for (i,p) in module.parameters.iter().skip(self.module_page*8).take(8).enumerate(){
                    let (value,formatted,pending)=self.feedback(&p.binding,p.value,&p.display,self.module_drives.get(&p.binding));
                    labels[i]=format!("{} · {}",module.name,p.name);display[i]=formatted;confirmed_display[i]=p.display.clone();pending_values[i]=pending;values[i]=if p.max>p.min{f64::from(value-p.min)/f64::from(p.max-p.min)}else{0.};
                }
            }
        }
        if self.view != "parameters" {
            labels.fill("UNASSIGNED".into());
            values.fill(0.);
            display.fill("—".into());confirmed_display.fill("—".into());pending_values.fill(false);
            if let Some(config) = &s.configuration {
                for (i, key) in self.control_keys().iter().enumerate() {
                    if let Some(c) = config.controls.iter().find(|c| &c.key == key) {
                        labels[i] = c.label.clone();
                        display[i] = c.display.clone();confirmed_display[i]=c.display.clone();
                        let target=if c.key.starts_with("slotLevel:"){self.control_drives.get(&c.key).and_then(|d|d.pending).filter(|(_,until)|Instant::now()<*until).map(|(v,_)|v).filter(|v|*v!=c.value)}else{None};
                        let value=target.unwrap_or(c.value);pending_values[i]=target.is_some();
                        if let Some(v)=target{if let Some(p)=o.previews.get(&c.key).filter(|p|i32::from(p.value)==v){display[i]=p.display.clone();}}
                        values[i] = if c.max > c.min {f64::from(value-c.min)/f64::from(c.max-c.min)}else{0.};
                    }
                }
            }
        }
        if self.view=="display" {
            labels.fill("UNASSIGNED".into());values.fill(0.);display.fill("—".into());pending_values.fill(false);confirmed_display.fill("—".into());
            if self.ui_theme.round()==2.{labels[3]="Colour intensity".into();values[3]=self.ui_colour_intensity;display[3]=format!("{:.0}%",self.ui_colour_intensity*100.);confirmed_display[3]=display[3].clone();}
            labels[1]="Blobs".into();values[1]=if self.blobs_enabled{1.}else{0.};display[1]=if self.blobs_enabled{"On"}else{"Off"}.into();confirmed_display[1]=display[1].clone();
            if self.ui_theme.round()!=2.{labels[2]="Glitch intensity".into();values[2]=self.glitch_intensity;display[2]=format!("{:.0}%",self.glitch_intensity*100.);confirmed_display[2]=display[2].clone();}
            labels[0]="Theme".into();values[0]=self.ui_theme.round()/2.;display[0]=["Light","Dark","Colour"][self.ui_theme.round() as usize].into();confirmed_display[0]=display[0].clone();
        }
        let leds = self.leds();
        let status = if !s.problem.is_empty() {
            format!("G1 audio error: {}", s.problem)
        } else if !o.error.is_empty() {
            o.error.clone()
        } else if slot_known && !running {
            format!("Reading slot {} patch…",char::from(b'A'+s.active_slot))
        } else if !running {
            "Waiting for running G1-Emu".into()
        } else {
            self.notice.clone()
        };
        let (cursor, count) = if self.file_source {
            (self.files.cursor, self.files.entries.len())
        } else {
            (self.cursor, o.presets.len())
        };
        let start = cursor.saturating_sub(2);
        let browser: Vec<_> = if self.file_source {
            self.files
                .entries
                .iter()
                .enumerate()
                .skip(start)
                .take(10)
                .map(|(i, p)| json!({"index":i,"selected":i==cursor,"name":p.name,"directory":p.directory}))
                .collect()
        } else {
            o.presets.iter().enumerate().skip(start).take(10).map(|(i,p)|json!({"index":i,"selected":i==cursor,"name":p.name,"bank":p.bank+1,"position":p.position+1})).collect()
        };
        let mut snapshot=json!({"backend":"g1","engine":if running{"G1-EMU / LIVE"}else{"G1-EMU / NOT READY"},"slot":if slot_known{char::from(b'A'+s.active_slot).to_string()}else{"—".into()},
            "patch":if running{s.lcd[0].trim()}else{"Waiting for emulator"},"lcd":s.lcd,"page":if self.module_edit{self.module_page}else{self.page},"pageCount":if self.module_edit{self.module_pages()}else{3},
            "moduleEdit":self.module_edit,"moduleCursor":self.module_cursor,"moduleCount":o.modules.len(),"moduleReady":self.selected_module().is_some(),
            "moduleName":o.modules.get(self.module_cursor).map(|m|m.name.as_str()).unwrap_or("Waiting for modules"),
            "moduleType":o.modules.get(self.module_cursor).map(|m|m.type_name.as_str()).unwrap_or(""),
            "moduleSection":o.modules.get(self.module_cursor).map(|m|m.section),
            "moduleList":o.modules.iter().enumerate().filter(|(i,_)|i.abs_diff(self.module_cursor)<=1).map(|(i,m)|json!({"index":i,"name":m.name,"selected":i==self.module_cursor})).collect::<Vec<_>>(),"pageName":format!("G1 PANEL KNOBS {}–{}",self.page*8+1,(self.page*8+8).min(18)),
            "labels":labels,"values":values,"displayValues":display,"status":status,"view":self.view,"settingsPage":self.settings_page,"configuration":s.configuration,"allocatedVoices":s.allocated_voices,"requestedVoices":s.requested_voices,"leds":leds,"browser":browser,"browserOpen":self.browser_open,"padMode":self.pads.mode.name(),"padBase":self.pads.mode.base(),"heldPads":s.pad_midi_held,"browserSource":if self.file_source{"files"}else{"banks"},"browserFolder":self.files.folder(),"browserRevision":if self.file_source {self.files.revision}else{o.presets_revision},"browserCursor":cursor,"browserCount":count,"loading":self.loading(),"banksComplete":o.complete,"presetCount":o.presets.len(),"source":"g1-emu","revision":o.generation});
        snapshot["moduleParameters"]=json!(self.selected_module().map(|m|m.parameters.iter().map(|p|{
            let (value,display,pending)=self.feedback(&p.binding,p.value,&p.display,self.module_drives.get(&p.binding));
            json!({"index":p.index,"value":value,"confirmedValue":p.value,"min":p.min,"max":p.max,"display":display,"confirmedDisplay":p.display,"pending":pending})
        }).collect::<Vec<_>>()).unwrap_or_default());
        snapshot["sequence"]=self.sequence_snapshot();
        snapshot["restore"]=s.restore.clone();
        snapshot["screensaver"]=json!(self.screensaver);snapshot["screensaverSinceMs"]=json!(self.screensaver_since_ms);snapshot["screenFade"]=json!(self.screen_fade_until.is_some_and(|until|Instant::now()<until));snapshot["blobsEnabled"]=json!(self.blobs_enabled);snapshot["glitchIntensity"]=json!((self.glitch_intensity*100.).round()/100.);snapshot["glitchesEnabled"]=json!(self.glitch_intensity>0.);snapshot["midiActivity"]=s.midi_activity.clone();snapshot["uiTheme"]=json!(["light","dark","colour"][self.ui_theme.round() as usize]);snapshot["uiColourIntensity"]=json!(self.ui_colour_intensity);snapshot["trackHeld"]=json!(self.track_pressed);snapshot["pendingValues"]=json!(pending_values);snapshot["confirmedDisplayValues"]=json!(confirmed_display);
        snapshot
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.save_preferences();

        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Observed {
        Observed {
            connected: true,
            instance: "test-only".into(),
            status: Status {
                restore:Value::Null,midi_activity:Value::Null,patch_binding:"patch-test".into(),edit:None,
                running: true,
                problem: String::new(),
                active_slot: 0,slot_known:true,
                pad_midi: true,
                pad_midi_held: vec![],
                lcd: ["Unit test".into(), "".into()],
                allocated_voices: None,
                requested_voices: None,
                upload: Some(Upload::default()),
                bank_load: Some(Upload::default()),
                configuration: Some(Configuration::default()),
                knobs: (0..18)
                    .map(|i| Knob {
                        index: i,
                        assigned: i < 2,
                        slot: 0,
                        binding: format!("binding-{i}"),
                        module: "Fixture".into(),
                        parameter: format!("P{i}"),
                        value: 64,
                        max: 127,
                        display: "64".into(),
                    })
                    .collect(),
            },
            ..Default::default()
        }
    }
    fn client() -> (Client, mpsc::Receiver<Command>) {
        let observed = fixture();
        let shared = Arc::new(Mutex::new(observed.clone()));
        let (tx, rx) = mpsc::sync_channel(128);
        (
            Client {
                shared,
                stop: Arc::new(AtomicBool::new(false)),
                thread: None,
                tx,
                observed,
                page: 0,
                cursor: 0,
                drives: std::array::from_fn(|_| None),
                pending_slot: None,
                files: Browser::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("patches")),
                browser_open: false,
                file_source: true,
                pending_file: None,
                pending_bank: None,
                load_request: 0,
                browser_state: None,
                library_root: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("patches"),
                restore_bank: None,
                notice: "".into(),
                view: "parameters",
                settings_page: 0,
                ui_colour_intensity:0.,
                ui_theme:0.,
                blobs_enabled:true,glitch_intensity:0.5,
                last_activity:Instant::now(),last_midi_activity:0,screensaver:false,screensaver_since_ms:0,screen_fade_until:None,activity_dirty:false,restore_wait:None,sequence_edit:false,sequence_step_focus:false,sequence_step:0,sequence_row:0,sequence_target:String::new(),
                module_edit:false,module_cursor:0,module_page:0,module_drives:BTreeMap::new(),
                control_drives: BTreeMap::new(),
                pending_home_reset:false,shift_pressed: false, track_pressed:false, held_buttons:Default::default(),
                function_release_pending: false,
                pads: crate::pad_midi::Pads::default(),
                pad_panic: Arc::new(AtomicBool::new(false)),
                pad_epoch: Arc::new(AtomicU64::new(0)),
            },
            rx,
        )
    }
    fn module_client()->(Client,mpsc::Receiver<Command>){
        let (mut c,rx)=client();
        c.observed.modules=Arc::new(vec![
            ModuleEntry{section:1,index:1,name:"Oscillator".into(),type_name:"OscA".into(),parameter_count:10},
            ModuleEntry{section:1,index:2,name:"Mixer".into(),type_name:"Mixer".into(),parameter_count:3},
            ModuleEntry{section:0,index:3,name:"Output".into(),type_name:"2Output".into(),parameter_count:0},
        ]);
        c.observed.module_binding="patch-test".into();
        c.observed.status.edit=Some(ModuleView{playhead:None,section:1,index:1,name:"Oscillator".into(),type_name:"OscA".into(),parameters:(0..10).map(|index|ModuleParameter{index,binding:format!("module-param-{index}"),name:format!("Parameter {index}"),value:64,min:0,max:127,display:"64".into()}).collect()});
        *c.shared.lock().unwrap()=c.observed.clone();(c,rx)
    }
    fn sequencer_client(kind:&str)->(Client,mpsc::Receiver<Command>){
        let(mut c,rx)=module_client();let mut modules=(*c.observed.modules).clone();modules.push(ModuleEntry{section:1,index:4,name:"Sequence".into(),type_name:kind.into(),parameter_count:if kind=="EventSeq"{36}else{21}});c.observed.modules=Arc::new(modules);
        c.observed.status.edit=Some(ModuleView{playhead:None,section:1,index:4,name:"Sequence".into(),type_name:kind.into(),parameters:(0..if kind=="EventSeq"{36}else{21}).map(|index|ModuleParameter{index,binding:format!("step-{index}"),name:format!("Parameter {index}"),value:if kind=="EventSeq"{if index==0{15}else{0}}else if index==16{15}else{60},min:0,max:if kind=="EventSeq"&&index>0{1}else{127},display:"60".into()}).collect()});
        *c.shared.lock().unwrap()=c.observed.clone();(c,rx)
    }
    #[test]
    fn available_leds_brighten_on_press_and_dim_on_release() {
        let (mut c, _commands) = sequencer_client("NoteSeqA");
        let event = |name, pressed| Event::Button { device: encdr::DeviceId::default(), name, pressed };

        assert_eq!(c.leds()["pattern"], 30);
        c.event(&event("pattern", true));
        c.event(&event("pattern", false));
        assert_eq!(c.leds()["pattern"], 80);
        c.event(&event("pattern", true));
        assert_eq!(c.leds()["pattern"], 80);
        c.event(&event("pattern", false));
        assert_eq!(c.leds()["pattern"], 30);

        assert_eq!(c.leds()["page_or_preset_next"], 30);
        c.event(&event("page_or_preset_next", true));
        assert_eq!(c.leds()["page_or_preset_next"], 80);
        c.event(&event("page_or_preset_next", false));
        assert_eq!(c.leds()["page_or_preset_next"], 30);
        assert!(c.leds().get("macro").is_none());
    }
    #[test]
    fn maschine_arrows_switch_pages_until_shift_is_held() {
        use crate::controller::Controller;
        let press = |name, pressed| Controller::Maschine.input(Event::Button {
            device: encdr::DeviceId::default(), name, pressed,
        }).unwrap();
        for (physical, logical, page, position) in [
            ("arrow_left", "page_or_preset_previous", 2, 0),
            ("arrow_right", "page_or_preset_next", 1, 2),
        ] {
            let (mut c, rx) = client();
            c.file_source = false;
            c.cursor = 1;
            c.observed.presets = (0..3)
                .map(|position| Preset { bank: 0, position, name: format!("Preset {position}") })
                .collect::<Vec<_>>()
                .into();
            assert_eq!(c.leds()[logical], 30);
            assert!(c.event(&press(physical, true)));
            c.event(&press(physical, false));
            assert_eq!(c.page, page);
            assert!(rx.try_recv().is_err());

            assert!(c.event(&press("shift", true)));
            assert_eq!(c.leds()[logical], 50);
            assert!(matches!(rx.try_recv().unwrap(), Command::Function(true)));
            assert!(c.event(&press(physical, true)));
            assert_eq!(c.page, page);
            assert!(matches!(rx.try_recv().unwrap(), Command::Load(preset, 0, _) if preset.position == position));
            c.event(&press("shift", false));
            c.event(&press(physical, false));
            assert_eq!(c.leds()[logical], 30);
            assert!(matches!(rx.try_recv().unwrap(), Command::Function(false)));
            assert!(rx.try_recv().is_err());
        }
    }
    #[test]
    fn pattern_filters_sequencers_and_wheel_edits_without_page_arrow_conflict(){
        let(mut c,rx)=sequencer_client("NoteSeqB");c.button("pattern").unwrap();assert_eq!(c.view,"sequencer");assert_eq!(c.module_cursor,3);
        assert!(matches!(rx.try_recv().unwrap(),Command::ModuleMode(true)));assert!(matches!(rx.try_recv().unwrap(),Command::SelectModule(m,_) if m.index==4));
        assert_eq!(c.snapshot()["sequence"]["list"].as_array().unwrap().len(),1);c.button("top_5").unwrap();assert!(c.sequence_edit);
        c.command("wheel 1").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Parameter{module:4,index:0,value:61,..}));
        c.button("encoder_right").unwrap();assert_eq!(c.sequence_step,1);c.command("wheel -2").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Parameter{index:1,value:58,..}));
        assert!(!c.button("browser_select").unwrap());assert!(!c.turn(0,0.2).unwrap());
        c.button("top_5").unwrap();assert!(!c.sequence_edit);c.button("top_5").unwrap();
        c.button("knob_page_next").unwrap();assert_eq!(c.view,"parameters");assert!(!c.sequence_edit&&!c.module_edit);assert_eq!(c.page,1);assert!(matches!(rx.try_recv().unwrap(),Command::Home));
    }
    #[test]
    fn pattern_event_rows_toggle_only_on_wheel_press_and_exit_cleanly(){
        let(mut c,rx)=sequencer_client("EventSeq");c.button("pattern").unwrap();rx.try_iter().for_each(drop);c.button("top_5").unwrap();
        c.button("encoder_left").unwrap();assert_eq!(c.sequence_step,15);c.button("encoder_down").unwrap();assert_eq!(c.sequence_row,1);
        assert!(!c.command("wheel 0").unwrap());assert!(rx.try_recv().is_err());c.button("browser_select").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Parameter{index:35,value:1,..}));
        c.button("browser_select").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Parameter{index:35,value:0,..}));
        c.button("pattern").unwrap();assert_eq!(c.view,"parameters");assert!(!c.sequence_edit);rx.try_iter().for_each(drop);
        c.button("pattern").unwrap();rx.try_iter().for_each(drop);c.button("browser").unwrap();assert!(c.browser_open);assert!(!c.sequence_edit&&!c.module_edit);
    }
    #[test]
    fn pattern_wheel_requires_edit_lock_for_right_side_navigation_or_writes(){
        let(mut c,rx)=sequencer_client("NoteSeqB");c.button("pattern").unwrap();rx.try_iter().for_each(drop);
        assert!(!c.button("encoder_right").unwrap());assert!(!c.sequence_step_focus&&!c.sequence_edit);
        c.command("wheel 3").unwrap();assert_eq!(c.sequence_step,0);assert!(rx.try_recv().is_err());
        c.button("top_5").unwrap();c.button("encoder_right").unwrap();c.command("wheel 2").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Parameter{index:1,value:62,..}));
        c.button("top_5").unwrap();assert!(!c.sequence_step_focus&&!c.sequence_edit);c.command("wheel -1").unwrap();assert_eq!(c.sequence_step,1);assert!(rx.try_recv().is_err());
        c.button("encoder_left").unwrap();assert_eq!(c.sequence_step,1);assert!(rx.try_recv().is_err());
        let(mut c,rx)=sequencer_client("EventSeq");c.button("pattern").unwrap();rx.try_iter().for_each(drop);c.button("top_5").unwrap();c.command("wheel -1").unwrap();assert_eq!(c.sequence_step,15);assert!(rx.try_recv().is_err());
        c.button("browser_select").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Parameter{index:19,value:1,..}));
        c.button("top_5").unwrap();assert!(!c.button("browser_select").unwrap());assert!(rx.try_recv().is_err());
    }
    #[test]
    fn pattern_live_playhead_is_distinct_from_edit_selection_and_stored_position(){
        let(mut c,rx)=sequencer_client("NoteSeqB");c.button("pattern").unwrap();rx.try_iter().for_each(drop);
        c.observed.status.edit.as_mut().unwrap().parameters[17].value=5;assert!(c.snapshot()["sequence"]["playhead"].is_null());
        c.observed.status.edit.as_mut().unwrap().playhead=Some(7);c.button("top_5").unwrap();c.button("encoder_right").unwrap();c.button("encoder_right").unwrap();
        assert_eq!(c.sequence_step,2);assert_eq!(c.snapshot()["sequence"]["playhead"],7);assert!(rx.try_recv().is_err());
        c.observed.status.edit.as_mut().unwrap().playhead=Some(16);assert!(c.snapshot()["sequence"]["playhead"].is_null());
    }
    #[test]
    fn pattern_lamp_requires_a_sequencer_in_the_confirmed_patch(){
        let(c,_)=module_client();assert!(c.snapshot()["leds"].get("pattern").is_none());
        let(mut c,_)=sequencer_client("NoteSeqA");assert_eq!(c.snapshot()["leds"]["pattern"],30);
        c.observed.status.patch_binding="replacement".into();assert!(c.snapshot()["leds"].get("pattern").is_none());
    }
    #[test]
    fn pattern_empty_and_stale_catalogues_never_edit_other_modules(){
        let(mut c,rx)=module_client();assert!(!c.button("pattern").unwrap());assert_eq!(c.view,"parameters");assert!(c.snapshot()["leds"].get("pattern").is_none());assert!(rx.try_recv().is_err());
        c.observed.module_binding="old-patch".into();assert!(c.sequence_indices().is_empty());
    }
    #[test]
    fn module_edit_pages_actual_parameters_and_restores_assigned_page(){
        let (mut c,rx)=module_client();c.page=2;c.button("module_edit").unwrap();
        assert!(matches!(rx.try_recv().unwrap(),Command::ModuleMode(true)));
        assert_eq!(c.snapshot()["pageCount"],2);assert_eq!(c.snapshot()["labels"][0],"Oscillator · Parameter 0");
        c.turn(0,0.1).unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Parameter{section:1,module:1,index:0,value:70,..}));
        assert_eq!(c.snapshot()["displayValues"][0],"64","pending target must not be shown as engine-confirmed");
        c.button("knob_page_next").unwrap();assert_eq!(c.snapshot()["page"],1);assert_eq!(c.snapshot()["labels"][0],"Oscillator · Parameter 8");assert_eq!(c.snapshot()["moduleParameters"].as_array().unwrap().len(),10);assert_eq!(c.snapshot()["displayValues"][2],"—");
        c.turn(1,0.1).unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Parameter{index:9,..}));
        c.button("module_edit").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Home));assert_eq!(c.snapshot()["page"],2);assert_eq!(c.snapshot()["pageCount"],3);
    }
    #[test]
    fn track_seven_restores_settings_without_writing_synth_configuration(){
        let(mut c,rx)=module_client();let device=encdr::DeviceId::default();
        c.button("top_7").unwrap();assert_eq!(c.view,"parameters");
        c.event(&Event::Button{device,name:"track",pressed:true});assert_eq!(c.snapshot()["leds"]["top_7"],30);
        c.button("top_7").unwrap();assert_eq!(c.view,"settings");assert_eq!(c.snapshot()["leds"]["top_7"],80);assert!(rx.try_recv().is_err());
        c.event(&Event::Button{device,name:"track",pressed:false});assert_eq!(c.view,"settings");
        c.event(&Event::Button{device,name:"track",pressed:true});assert_eq!(c.view,"parameters");assert!(matches!(rx.try_recv().unwrap(),Command::Home));assert!(rx.try_recv().is_err());
    }
    #[test]
    fn clear_restores_only_confirmed_slot_and_blocks_interleaved_turns(){
        let(mut c,rx)=module_client();
        c.observed.status.restore=json!({"baselineAvailable":true,"available":false,"busy":false,"serial":2,"slot":0});
        assert!(c.snapshot()["leds"].get("clear").is_none());assert!(!c.button("clear").unwrap());assert!(rx.try_recv().is_err());
        c.observed.status.restore=json!({"available":true,"busy":false,"serial":2,"slot":0});
        assert_eq!(c.snapshot()["leds"]["clear"],30);assert!(c.button("clear").unwrap());
        assert!(matches!(rx.try_recv().unwrap(),Command::Restore(0,key) if key=="patch-test"));
        assert_eq!(c.snapshot()["leds"]["clear"],80);assert!(!c.turn(0,0.1).unwrap());assert!(!c.button("clear").unwrap());assert!(rx.try_recv().is_err());
        c.observed.status.restore["busy"]=json!(true);c.restore_wait=None;assert!(!c.turn(4,0.2).unwrap());
        c.observed.status.restore=json!({"baselineAvailable":true,"available":false,"busy":false,"serial":3,"slot":0});
        assert!(c.snapshot()["leds"].get("clear").is_none());assert!(!c.button("clear").unwrap());assert!(rx.try_recv().is_err());
    }
    #[test]
    fn screensaver_idle_and_wake_preserve_settings_and_page(){
        let(mut c,_)=module_client();c.view="mixer";c.blobs_enabled=false;c.glitch_intensity=0.42;c.ui_theme=2.;
        c.last_activity=Instant::now()-Duration::from_secs(299);c.update_idle();assert!(!c.screensaver);
        c.last_activity=Instant::now()-Duration::from_secs(2100);assert!(c.update_idle());assert!(c.screensaver);
        assert!((wall_ms()-c.screensaver_since_ms-1800000).abs()<100);
        assert_eq!(c.snapshot()["screensaver"],true);c.note_activity();assert!(c.update_idle());
        assert!(!c.screensaver);assert!(c.screen_fade_until.is_some());assert_eq!(c.view,"mixer");
        assert!(!c.blobs_enabled);assert_eq!(c.glitch_intensity,0.42);assert_eq!(c.ui_theme,2.);
    }
    #[test]
    fn colour_mode_has_no_glitch_control(){
        let(mut c,_)=module_client();c.view="display";c.ui_theme=2.;
        assert!(!c.turn(2,0.2).unwrap());assert_eq!(c.glitch_intensity,0.5);
        assert_eq!(c.snapshot()["displayValues"][2],"—");
    }
    #[test]
    fn display_blob_switches_default_on_and_are_local(){
        let(mut c,rx)=module_client();assert!(c.blobs_enabled);assert_eq!(c.glitch_intensity,0.5);c.view="display";
        assert_eq!(c.snapshot()["displayValues"][1],"On");assert_eq!(c.snapshot()["displayValues"][2],"50%");
        assert!(c.turn(1,-0.005).unwrap());assert!(c.turn(2,0.005).unwrap());assert!(!c.blobs_enabled);assert_eq!(c.snapshot()["displayValues"][2],"51%");
        c.turn(2,-1.).unwrap();assert_eq!(c.glitch_intensity,0.);c.turn(2,1.).unwrap();assert_eq!(c.glitch_intensity,1.);
        assert!(rx.try_recv().is_err());assert!(c.turn(1,0.005).unwrap());assert!(c.blobs_enabled);
    }

    #[test]
    fn display_settings_chord_is_local_bounded_and_home_preserves_colour_intensity(){
        let(mut c,rx)=module_client();let device=encdr::DeviceId::default();
        c.button("top_8").unwrap();assert_eq!(c.view,"parameters");assert!(rx.try_recv().is_err());
        c.event(&Event::Button{device,name:"track",pressed:true});assert_eq!(c.snapshot()["leds"]["top_8"],30);
        c.button("top_8").unwrap();assert_eq!(c.view,"display");assert_eq!(c.snapshot()["leds"]["top_8"],80);
        c.event(&Event::Button{device,name:"track",pressed:false});assert_eq!(c.view,"display");
        assert_eq!(c.snapshot()["uiTheme"],"light");assert!(!c.turn(3,0.4).unwrap());assert!(c.turn(0,0.5).unwrap());assert_eq!(c.snapshot()["uiTheme"],"colour");
        assert!(c.turn(3,0.4).unwrap());assert!((c.ui_colour_intensity-0.4).abs()<1e-6);
        assert_eq!(c.snapshot()["displayValues"][3],"40%");assert!(rx.try_recv().is_err());
        assert!(!c.turn(4,0.3).unwrap());c.turn(3,-10.).unwrap();assert_eq!(c.ui_colour_intensity,0.);
        c.turn(3,10.).unwrap();assert_eq!(c.ui_colour_intensity,1.);c.turn(3,-0.6).unwrap();
        c.event(&Event::Button{device,name:"track",pressed:true});assert_eq!(c.view,"parameters");assert!((c.ui_colour_intensity-0.4).abs()<1e-6);
        assert_eq!(c.snapshot()["uiColourIntensity"],0.4);
    }
    #[test]
    fn track_only_modifies_screen_slots_and_lamps_follow_available_actions(){
        let(mut c,rx)=module_client();let device=encdr::DeviceId::default();
        assert!(c.snapshot()["leds"].get("top_1").is_none());
        assert_eq!(c.snapshot()["leds"]["knob_page_previous"],30);
        c.event(&Event::Button{device,name:"track",pressed:true});assert_eq!(c.snapshot()["leds"]["top_1"],80);assert_eq!(c.snapshot()["leds"]["top_4"],30);
        for slot in 1..4 {c.button(["top_1","top_2","top_3","top_4"][slot]).unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Slot(s) if usize::from(s)==slot));}
        assert!(!c.shift_pressed);assert!(rx.try_recv().is_err());
        c.event(&Event::Button{device,name:"track",pressed:false});assert!(c.snapshot()["leds"].get("top_1").is_none());
        c.button("module_edit").unwrap();rx.try_recv().unwrap();assert_eq!(c.snapshot()["leds"]["module_edit"],80);assert_eq!(c.snapshot()["leds"]["top_1"],30);assert!(c.snapshot()["leds"].get("top_3").is_none());
        c.function(true).unwrap();rx.try_recv().unwrap();assert!(!c.turn(0,0.2).unwrap());assert!(rx.try_recv().is_err());
        c.event(&Event::Button{device,name:"track",pressed:true});assert!(matches!(rx.try_recv().unwrap(),Command::Home));c.button("top_2").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Slot(1)));assert!(c.shift_pressed);assert!(!c.module_edit);
        c.release_function();assert!(!c.track_pressed);assert!(c.held_buttons.is_empty());
    }
    #[test]
    fn browser_is_exclusive_and_track_returns_home_from_every_page(){
        let(mut c,rx)=module_client();let device=encdr::DeviceId::default();
        c.button("module_edit").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::ModuleMode(true)));
        c.button("browser").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Home));assert!(c.browser_open&&!c.module_edit&&c.view=="parameters");
        c.button("browser").unwrap();assert!(!c.browser_open&&!c.module_edit);assert!(rx.try_recv().is_err());
        for page in ["module_edit","mixer","settings","browser"]{
            c.button(page).unwrap();rx.try_iter().for_each(drop);
            c.event(&Event::Button{device,name:"track",pressed:true});assert!(c.track_pressed&&!c.module_edit&&!c.browser_open&&c.view=="parameters","{page}");rx.try_iter().for_each(drop);
            c.button("top_2").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Slot(1)));c.pending_slot=None;
            c.event(&Event::Button{device,name:"track",pressed:false});
        }
    }
    #[test]
    fn home_remains_visible_and_backend_cleanup_retries_when_queue_is_full(){
        let(mut c,rx)=module_client();for _ in 0..128{c.tx.try_send(Command::Refresh).unwrap();}
        assert!(c.button("module_edit").is_err());assert!(!c.module_edit);assert!(c.button("mixer").is_err());assert_eq!(c.view,"parameters");c.module_edit=true;
        c.event(&Event::Button{device:encdr::DeviceId::default(),name:"track",pressed:true});assert!(c.track_pressed&&!c.module_edit&&!c.browser_open&&c.view=="parameters");assert!(c.pending_home_reset);
        rx.try_iter().for_each(drop);c.tick();assert!(!c.pending_home_reset);assert!(matches!(rx.try_recv().unwrap(),Command::Home));
    }
    #[test]
    fn module_switch_and_patch_change_block_stale_parameter_edits(){
        let (mut c,rx)=module_client();c.button("module_edit").unwrap();rx.try_recv().unwrap();
        c.step_module(1).unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::SelectModule(ModuleEntry{index:2,..},_)));
        assert!(!c.snapshot()["moduleReady"].as_bool().unwrap());assert!(c.turn(0,0.2).is_err());assert!(rx.try_recv().is_err());
        c.shared.lock().unwrap().status.patch_binding="new-patch".into();c.tick();assert!(c.turn(0,0.2).is_err());
    }
    #[test]
    fn modules_with_no_parameters_have_one_inert_page(){
        let (mut c,rx)=module_client();c.button("module_edit").unwrap();rx.try_recv().unwrap();c.step_module(2).unwrap();rx.try_recv().unwrap();
        c.shared.lock().unwrap().status.edit=Some(ModuleView{playhead:None,section:0,index:3,name:"Output".into(),type_name:"2Output".into(),parameters:vec![]});c.tick();
        assert_eq!(c.snapshot()["pageCount"],1);assert!(c.turn(0,0.1).is_err());assert!(c.leds().get("macro").is_none());
        c.button("knob_page_next").unwrap();assert_eq!(c.snapshot()["page"],0);
    }
    #[test]
    fn hardware_events_dispatch_without_the_view_loop() {
        let (client, commands) = client();
        let client = Arc::new(Mutex::new(client));
        let (tx, rx) = mpsc::channel();
        let (dispatch, _lifecycle) = crate::input_dispatch::InputDispatch::start(
            move || rx.recv_timeout(Duration::from_millis(5)).ok(),
            Some(client.clone()),
        );
        let device = encdr::DeviceId::default();
        dispatch.set_device(Some(device));
        let start = Instant::now();
        tx.send(Event::Grid {
            device,
            name: "pad_1",
            index: 12,
            pressure: 0.25,
        })
        .unwrap();
        assert!(matches!(
            commands.recv_timeout(Duration::from_millis(100)).unwrap(),
            Command::Pad {
                pad: 0,
                note: 36,
                velocity: 64,
                ..
            }
        ));
        assert!(start.elapsed() < Duration::from_millis(100));
        // No main-thread tick or render call was needed to deliver either event.
        tx.send(Event::EncoderFine {
            device,
            name: "screen_encoder_1",
            delta: 0.04,
        })
        .unwrap();
        assert!(matches!(
            commands.recv_timeout(Duration::from_millis(100)).unwrap(),
            Command::Set { index: 0, .. }
        ));
        dispatch.set_device(None);
        tx.send(Event::Grid {
            device,
            name: "pad_2",
            index: 13,
            pressure: 1.,
        })
        .unwrap();
        thread::sleep(Duration::from_millis(15));
        assert!(commands.try_recv().is_err());
        drop(dispatch);
        assert!(client.lock().unwrap().pad_panic.load(Ordering::Acquire));
    }
    #[test]
    #[ignore = "uses isolated localhost sockets"]
    fn encoder_storm_is_paced_and_latest_target_survives() {
        use std::io::{BufRead, BufReader};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let samples = Arc::new(Mutex::new(Vec::new()));
        let recorded = samples.clone();
        let bank_requests=Arc::new(AtomicU64::new(0));let bank_counter=bank_requests.clone();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            writeln!(
                stream,
                "{}",
                json!({"service":"nord-modular-ni-interface-g1","protocol":1,"instance":"storm-test"})
            )
            .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut status = fixture().status;
            loop {
                let mut line = String::new();
                if !matches!(reader.read_line(&mut line),Ok(n) if n > 0) {
                    break;
                }
                let request: Value = serde_json::from_str(&line).unwrap();
                let result = match request["method"].as_str().unwrap() {
                    "status" => serde_json::to_value(&status).unwrap(),
                    "presets" => {bank_counter.fetch_add(1,Ordering::Relaxed);json!({"entries":[],"complete":true})},
                    "set_knobs" => {
                        recorded.lock().unwrap().push(Instant::now());
                        for target in request["targets"].as_array().unwrap() {
                            let k = &mut status.knobs[target["index"].as_u64().unwrap() as usize];
                            k.value = target["value"].as_u64().unwrap() as u8;
                            k.display = k.value.to_string();
                        }
                        json!({"accepted":true})
                    }
                    _ => json!({"accepted":true}),
                };
                if writeln!(stream, "{}", json!({"id":request["id"],"result":result})).is_err() {
                    break;
                }
            }
        });
        let shared = Arc::new(Mutex::new(Observed::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::sync_channel(128);
        let worker_thread = {
            let shared = shared.clone();
            let stop = stop.clone();
            thread::spawn(move || {
                worker(
                    address,
                    rx,
                    shared,
                    stop,
                    Arc::new(AtomicBool::new(false)),
                    Arc::new(AtomicU64::new(0)),
                )
            })
        };
        let deadline = Instant::now() + Duration::from_secs(3);
        while !shared.lock().unwrap().status.running && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(2));
        }
        assert!(shared.lock().unwrap().status.running);
        for n in 0..10000 {
            tx.send(Command::Set {
                input_seq: 0,
                index: 0,
                value: (n % 127) as u8,
                binding: "binding-0".into(),
            })
            .unwrap();
        }
        tx.send(Command::Set {
            input_seq: 0,
            index: 0,
            value: 127,
            binding: "binding-0".into(),
        })
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while shared.lock().unwrap().status.knobs[0].value != 127 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(2));
        }
        let final_value = shared.lock().unwrap().status.knobs[0].value;
        assert_eq!(bank_requests.load(Ordering::Relaxed),0,"Files mode must not start a bank scan");
        tx.send(Command::Refresh).unwrap();
        let deadline=Instant::now()+Duration::from_secs(2);
        while bank_requests.load(Ordering::Relaxed)==0 && Instant::now()<deadline {thread::sleep(Duration::from_millis(2));}
        assert!(bank_requests.load(Ordering::Relaxed)>0);
        stop.store(true, Ordering::Relaxed);
        worker_thread.join().unwrap();
        server.join().unwrap();
        assert_eq!(final_value, 127);
        let times = samples.lock().unwrap();
        assert!(
            !times.is_empty() && times.len() < 200,
            "{} native batches",
            times.len()
        );
        assert!(times
            .windows(2)
            .all(|t| t[1].duration_since(t[0]) >= Duration::from_millis(15)));
        println!(
            "10001 encoder targets -> {} paced native batches; final target confirmed",
            times.len()
        );
    }
    #[test]
    fn komplete_navigation_reuses_real_pages_browser_and_slots() {
        let (mut c, rx) = client();
        c.button("knob_page_previous").unwrap();
        assert_eq!(c.page, 2);
        c.button("knob_page_next").unwrap();
        assert_eq!(c.page, 0);
        c.button("browser_select").unwrap();
        assert!(c.browser_open);
        // Empty test library cannot manufacture a successful patch activation.
        c.files.entries.clear();
        assert!(c.button("browser_select").is_err());
        c.button("macro").unwrap();
        assert!(!c.browser_open);
        c.button("group_d").unwrap();
        assert!(matches!(rx.try_recv().unwrap(), Command::Slot(3)));
    }
    #[test]
    fn held_function_blocks_knob_writes_and_slot_pinning_does_not_wait_for_focus() {
        let (mut c,rx)=client();
        c.function(true).unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Function(true)));
        assert!(!c.turn(0,0.5).unwrap());assert!(rx.try_recv().is_err());
        c.button("group_b").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Slot(1)));
        assert!(c.pending_slot.is_none());
        c.function(false).unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Function(false)));
        c.button("encoder_right").unwrap();assert_eq!(c.page,1);
        c.button("encoder_left").unwrap();assert_eq!(c.page,0);
        c.button("browser").unwrap();assert!(c.browser_open);
        c.button("browser").unwrap();assert!(!c.browser_open);
    }
    #[test]
    fn half_sensitivity_accumulates_and_mode_switch_releases_pads() {
        let (mut c, rx) = client();
        c.turn(0, 0.2).unwrap();
        assert!(matches!(
            rx.try_recv().unwrap(),
            Command::Set { value: 76, .. }
        ));
        c.turn(0, 0.2).unwrap();
        assert!(matches!(
            rx.try_recv().unwrap(),
            Command::Set { value: 89, .. }
        ));
        c.pad("pad_1", 0.25).unwrap();
        assert!(matches!(
            rx.try_recv().unwrap(),
            Command::Pad {
                note: 36,
                velocity: 64,
                ..
            }
        ));
        assert!(!c.pad("pad_1", 0.8).unwrap());
        c.button("keyboard").unwrap();
        assert!(c.pad_panic.load(Ordering::Acquire));
        c.pad("pad_1", 1.).unwrap();
        assert!(matches!(
            rx.try_recv().unwrap(),
            Command::Pad {
                epoch: 1,
                note: 48,
                velocity: 127,
                ..
            }
        ));
        c.pad("pad_1", 0.).unwrap();
        assert!(matches!(
            rx.try_recv().unwrap(),
            Command::Pad { velocity: 0, .. }
        ));
    }
    #[test]
    fn saturated_queue_uses_out_of_band_pad_release() {
        let (mut c, rx) = client();
        c.pad("pad_1", 1.).unwrap();
        rx.try_recv().unwrap();
        for _ in 0..128 {
            c.send(Command::Refresh).unwrap();
        }
        assert!(c.pad("pad_1", 0.).is_err());
        assert!(c.pad_panic.load(Ordering::Acquire));
        assert!(!c.pads.held.iter().any(|v| *v));
    }
    #[test]
    fn native_engine_problem_is_visible_instead_of_old_success() {
        let (mut c, _rx) = client();
        c.notice = "Loaded previous.pch".into();
        c.shared.lock().unwrap().status.problem = "DSP 0 JIT stopped".into();
        c.tick();
        assert!(c.snapshot()["status"]
            .as_str()
            .unwrap()
            .contains("DSP 0 JIT stopped"));
    }

    #[test]
    fn low_range_parameters_use_encoder_steps_not_full_turn_switches() {
        let (mut c, rx) = client();
        c.observed.status.knobs[0].max = 1;
        c.observed.status.knobs[0].value = 0;
        c.turn(0, 0.08).unwrap();
        assert!(matches!(
            rx.try_recv().unwrap(),
            Command::Set { value: 1, .. }
        ));
        c.turn(0, -0.08).unwrap();
        assert!(matches!(
            rx.try_recv().unwrap(),
            Command::Set { value: 0, .. }
        ));
    }

    #[test]
    fn pending_encoder_display_tracks_latest_confirmed_state_during_reversals() {
        let (mut c, _rx) = client();
        c.turn(0, 0.4).unwrap(); // Half sensitivity: 64 -> 89
        for reported in [80, 72, 89, 61, 79, 58] {
            // Extend the pending edit on every turn, as in continuous use.
            c.turn(0, if reported % 2 == 0 { -0.2 } else { 0.2 }).unwrap();
            {
                let mut observed = c.shared.lock().unwrap();
                observed.status.knobs[0].value = reported;
                observed.status.knobs[0].display = reported.to_string();
            }
            c.tick();
            assert_eq!(c.snapshot()["displayValues"][0], reported.to_string());
        }
        // Once confirmed, an external change remains authoritative.
        {
            let mut observed = c.shared.lock().unwrap();
            observed.status.knobs[0].value = 50;
            observed.status.knobs[0].display = "50".into();
        }
        c.tick();
        assert_eq!(c.snapshot()["displayValues"][0], "50");
    }

    #[test]
    fn pending_feedback_precedes_readback_and_uses_native_formatting() {
        let (mut c,rx)=client();c.turn(0,0.1).unwrap();
        let Command::Set{value,binding,..}=rx.try_recv().unwrap() else {panic!("missing target")};
        let first=c.snapshot();assert_eq!(first["pendingValues"][0],true);
        assert_eq!(first["confirmedDisplayValues"][0],"64");
        assert_eq!(first["values"][0],f64::from(value)/127.);
        c.shared.lock().unwrap().previews=Arc::new(BTreeMap::from([(binding.clone(),ValuePreview{binding:binding.clone(),value,display:"1.25 kHz".into()})]));c.tick();
        assert_eq!(c.snapshot()["displayValues"][0],"1.25 kHz");
        assert_eq!(c.observed.status.knobs[0].value,64);
        c.turn(0,-0.2).unwrap();
        assert_ne!(c.snapshot()["displayValues"][0],"1.25 kHz","older acceptance cannot label a newer target");
        c.shared.lock().unwrap().error="rejected".into();c.tick();
        assert_eq!(c.snapshot()["pendingValues"][0],false);
        assert_eq!(c.snapshot()["displayValues"][0],"64");
    }
    #[test]
    fn module_preview_is_distinct_and_clears_on_confirmation_or_patch_change(){
        let(mut c,rx)=module_client();c.button("module_edit").unwrap();rx.try_recv().unwrap();c.turn(0,0.1).unwrap();
        let Command::Parameter{value,binding,..}=rx.try_recv().unwrap() else{panic!("missing module target")};
        let p=c.snapshot()["moduleParameters"][0].clone();assert_eq!(p["value"],value);assert_eq!(p["confirmedValue"],64);assert_eq!(p["pending"],true);
        {let mut o=c.shared.lock().unwrap();let p=&mut o.status.edit.as_mut().unwrap().parameters[0];p.value=value;p.display=value.to_string();}c.tick();
        assert_eq!(c.snapshot()["moduleParameters"][0]["pending"],false);
        c.turn(0,0.1).unwrap();assert!(c.module_drives.contains_key(&binding));
        c.shared.lock().unwrap().status.patch_binding="another-patch".into();c.tick();assert!(c.module_drives.is_empty());
    }
    #[test]
    fn rejects_inconsistent_metadata_and_preserves_no_engine_state() {
        let valid = fixture().status;
        checked_status(serde_json::to_value(&valid).unwrap()).unwrap();
        let mut invalid = valid.clone();
        invalid.knobs[0].value = 128;
        assert!(checked_status(serde_json::to_value(invalid).unwrap()).is_err());
        let mut invalid = valid;
        invalid.knobs[3].index = 2;
        assert!(checked_status(serde_json::to_value(invalid).unwrap()).is_err());
        let (mut c, _rx) = client();
        c.observed = Observed::default();
        let snapshot = c.snapshot();
        assert_eq!(snapshot["engine"], "G1-EMU / NOT READY");
        assert_eq!(snapshot["slot"], "—");
        assert!(snapshot["displayValues"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v == "—"));
    }
    #[test]
    fn fine_turns_accumulate_and_confirmation_clears_pending_feedback() {
        let (mut c, rx) = client();
        for _ in 0..20 {
            c.turn(0, 0.001).unwrap();
        }
        let commands: Vec<_> = rx.try_iter().collect();
        assert_eq!(commands.len(), 1);
        assert!(
            matches!(&commands[0],Command::Set{index:0,value:65,binding,..} if binding=="binding-0")
        );
        assert_eq!(c.snapshot()["displayValues"][0], "64");
        assert_eq!(c.snapshot()["pendingValues"][0],true);
        {
            let mut shared = c.shared.lock().unwrap();
            shared.status.knobs[0].value = 65;
            shared.status.knobs[0].display = "65".into();
            shared.generation = 1;
        }
        assert!(c.tick());
        assert_eq!(c.snapshot()["displayValues"][0], "65");
        assert_eq!(c.snapshot()["pendingValues"][0],false);
        assert!(!c.notice.contains("awaiting G1"));
    }
    #[test]
    fn expired_knob_confirmation_reports_failure_instead_of_waiting_forever() {
        let (mut c, _rx) = client();
        c.turn(0,0.2).unwrap();
        c.drives[0].as_mut().unwrap().pending.as_mut().unwrap().1 = Instant::now()-Duration::from_secs(1);
        c.tick();
        assert_eq!(c.notice,"Parameter update not confirmed");
        assert_eq!(c.snapshot()["pendingValues"][0],false);
        assert_eq!(c.snapshot()["values"][0],64./127.);
    }
    #[test]
    fn track_lamps_and_screen_follow_focus_not_enabled_pins_even_before_patch_ready(){
        let(mut c,rx)=client();let device=encdr::DeviceId::default();
        c.observed.status.active_slot=2;c.observed.status.configuration=Some(Configuration{selected_slots:vec![true,false,true,false],..Default::default()});
        *c.shared.lock().unwrap()=c.observed.clone();c.event(&Event::Button{device,name:"track",pressed:true});
        for ready in [true,false]{
            c.observed.status.running=ready;c.observed.status.slot_known=true;*c.shared.lock().unwrap()=c.observed.clone();
            let s=c.snapshot();assert_eq!(s["slot"],"C");assert_eq!(s["leds"]["top_1"],30);assert_eq!(s["leds"]["top_3"],80);
        }
        c.event(&Event::Button{device,name:"top_1",pressed:true});assert!(matches!(rx.try_recv().unwrap(),Command::Slot(0)),"can leave a slot whose patch is still loading");
        assert_eq!(c.snapshot()["leds"]["top_1"],30,"requested or pinned slot is not confirmed focus");assert_eq!(c.snapshot()["leds"]["top_3"],80);
    }
    #[test]
    fn slot_a_reversal_is_sent_while_another_slot_is_pending(){
        let(mut c,rx)=client();c.button("group_b").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Slot(1)));
        c.button("group_a").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Slot(0)));assert_eq!(c.pending_slot.unwrap().0,0);
        c.tick();assert!(c.pending_slot.is_none());assert_eq!(c.notice,"Slot A selected");
        c.function(true).unwrap();rx.try_recv().unwrap();assert!(!c.button("module_edit").unwrap());assert!(!c.module_edit);assert!(rx.try_recv().is_err());
    }
    #[test]
    fn slot_confirmation_does_not_deadlock_on_noop_or_timeout() {
        let (mut c, _rx) = client();
        assert!(!c.button("group_a").unwrap());
        assert!(c.pending_slot.is_none());
        c.pending_slot = Some((1, Instant::now() - Duration::from_secs(1)));
        assert!(c.tick());
        assert!(c.pending_slot.is_none());
        assert!(c.notice.contains("not confirmed"));
    }
    #[test]
    fn assignment_change_discards_pending_target_and_rejects_unused_knobs() {
        let (mut c, rx) = client();
        c.turn(0, 0.1).unwrap();
        let _ = rx.try_recv().unwrap();
        {
            let mut shared = c.shared.lock().unwrap();
            shared.status.knobs[0].binding = "new-patch".into();
            shared.generation = 1;
        }
        c.tick();
        assert!(c.drives[0].is_none());
        c.turn(0, 0.1).unwrap();
        assert!(matches!(rx.try_recv().unwrap(),Command::Set{binding,..} if binding=="new-patch"));
        assert!(c.turn(7, 0.1).is_err());
        assert!(c.turn(0, f64::NAN).is_err());
    }
    #[test]
    fn browser_button_selects_files_and_loads_into_confirmed_slot() {
        let root = std::env::temp_dir().join(format!("mk3-load-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("A.pch"), "fixture").unwrap();
        std::fs::write(root.join("B.pch"), "fixture").unwrap();
        let (mut c, rx) = client();
        c.files = Browser::new(root.clone());
        c.observed.status.active_slot = 2;
        c.shared.lock().unwrap().status.active_slot = 2;
        c.page = 2;
        c.button("browser").unwrap();
        assert_eq!(c.snapshot()["browserCount"], 2);
        assert!(c.event(&Event::Encoder {
            device: encdr::DeviceId::default(),
            name: "main_encoder",
            delta: 1
        }));
        assert_eq!(c.snapshot()["browser"][1]["name"], "B.pch");
        c.button("top_1").unwrap();
        assert_eq!(c.page, 2);
        c.button("encoder_press").unwrap();
        assert!(
            matches!(rx.try_recv().unwrap(), Command::File(path,2,_) if std::path::Path::new(&path)==root.join("B.pch").canonicalize().unwrap())
        );
        assert!(c.snapshot()["loading"].as_bool().unwrap());
        assert!(c.browser_open, "browser stays open until confirmed");
        assert!(c.button("encoder_press").is_err());
        assert!(c.button("group_b").is_err());
        assert!(!c.notice.starts_with("Loaded"));
        c.shared.lock().unwrap().status.upload = Some(Upload {
            serial: 2,
            busy: false,
            ok: true,
            message: "done".into(),
        });
        c.tick();
        assert_eq!(c.notice, "Loaded B.pch");
        assert!(!c.browser_open, "confirmed file load returns to parameters");
        assert!(c.pending_file.is_none());
        c.button("browser").unwrap();
        c.button("top_1").unwrap();
        assert_eq!(c.page, 2);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn bank_load_waits_for_firmware_outcome_and_releases_on_failure() {
        let (mut c, rx) = client();
        c.file_source = false;
        c.browser_open = true;
        c.observed.presets = vec![Preset {
            bank: 0,
            position: 0,
            name: "Stored".into(),
        }]
        .into();
        c.shared.lock().unwrap().presets = c.observed.presets.clone();
        c.load_selected().unwrap();
        assert!(c.loading());
        assert!(matches!(rx.try_recv().unwrap(), Command::Load(_, 0, _)));
        {
            let mut shared = c.shared.lock().unwrap();
            shared.status.bank_load = Some(Upload {
                serial: 2,
                busy: false,
                ok: false,
                message: "No acknowledgement".into(),
            });
        }
        c.tick();
        assert!(!c.loading());
        assert!(c.notice.contains("No acknowledgement"));
        assert!(c.browser_open, "failed load retains browser for retry");
        c.load_selected().unwrap();
        {
            let mut shared = c.shared.lock().unwrap();
            shared.status.bank_load = Some(Upload {
                serial: 4,
                busy: false,
                ok: true,
                message: String::new(),
            });
        }
        c.tick();
        assert!(!c.loading());
        assert_eq!(c.notice, "Loaded Stored");
        assert!(!c.browser_open, "confirmed bank load closes browser");
    }

    #[test]
    fn disconnect_and_external_patch_change_clear_targets_keep_navigation() {
        let (mut c, _rx) = client();
        c.page = 2;
        c.browser_open = true;
        c.file_source = false;
        c.cursor = 1;
        c.observed.presets = vec![
            Preset {
                bank: 0,
                position: 0,
                name: "A".into(),
            },
            Preset {
                bank: 0,
                position: 1,
                name: "B".into(),
            },
        ]
        .into();
        c.shared.lock().unwrap().presets = c.observed.presets.clone();
        c.page = 0;
        c.turn(0, 0.1).unwrap();
        assert!(c.drives[0].as_ref().unwrap().pending.is_some());
        c.shared.lock().unwrap().status.lcd[0] = "Different patch".into();
        c.tick();
        assert!(c.drives[0].is_none());
        c.page = 2;
        *c.shared.lock().unwrap() = Observed::default();
        c.tick();
        assert_eq!(c.page, 2);
        assert!(c.browser_open);
        assert_eq!(c.restore_bank, Some((0, 1)));
        assert_eq!(c.snapshot()["engine"], "G1-EMU / NOT READY");
        let mut next = fixture();
        next.instance = "new engine".into();
        next.complete = true;
        next.presets = vec![
            Preset {
                bank: 0,
                position: 1,
                name: "B".into(),
            },
            Preset {
                bank: 0,
                position: 0,
                name: "A".into(),
            },
        ]
        .into();
        *c.shared.lock().unwrap() = next;
        c.tick();
        assert_eq!(c.cursor, 0);
        assert_eq!(c.page, 2);
    }

    #[test]
    fn repeated_file_rejection_and_successful_retry_release_pending_state() {
        let (mut c, _rx) = client();
        {
            let mut shared = c.shared.lock().unwrap();
            shared.error = "Invalid patch".into();
            shared.load_response = Some((1, "Invalid patch".into()));
        }
        c.tick();
        c.pending_file = Some((
            10,
            "bad.pch".into(),
            Instant::now() + Duration::from_secs(30),
            2,
        ));
        c.shared.lock().unwrap().load_response = Some((2, "Invalid patch".into()));
        c.tick();
        assert!(c.pending_file.is_none());
        assert!(c.notice.contains("Invalid patch"));
        c.pending_file = Some((
            10,
            "good.pch".into(),
            Instant::now() + Duration::from_secs(30),
            3,
        ));
        {
            let mut shared = c.shared.lock().unwrap();
            shared.error.clear();
            shared.load_response = Some((3, String::new()));
            shared.status.upload = Some(Upload {
                serial: 11,
                busy: true,
                ok: false,
                message: String::new(),
            });
        }
        c.tick();
        assert!(
            c.pending_file.is_some(),
            "accepting a retry must retain its confirmation barrier"
        );
        {
            let mut shared = c.shared.lock().unwrap();
            shared.status.upload.as_mut().unwrap().busy = false;
            shared.status.upload.as_mut().unwrap().ok = true;
        }
        c.tick();
        assert!(c.pending_file.is_none());
        assert_eq!(c.notice, "Loaded good.pch");
    }

    #[test]
    fn firmware_rejection_and_timeout_do_not_claim_patch_loaded() {
        let (mut c, _rx) = client();
        c.pending_file = Some((
            10,
            "bad.pch".into(),
            Instant::now() + Duration::from_secs(5),
            1,
        ));
        c.shared.lock().unwrap().status.upload = Some(Upload {
            serial: 11,
            busy: false,
            ok: false,
            message: "Synth rejected packet".into(),
        });
        c.tick();
        assert_eq!(c.notice, "Load failed: Synth rejected packet");
        assert!(c.pending_file.is_none());
        c.pending_file = Some((
            11,
            "late.pch".into(),
            Instant::now() - Duration::from_secs(1),
            1,
        ));
        c.tick();
        assert!(c.notice.contains("not confirmed"));
        assert!(c.pending_file.is_none());
    }
    #[test]
    fn user_assignments_and_leds_follow_confirmed_slot_and_view() {
        let (mut c, rx) = client();
        assert_eq!(c.snapshot()["leds"]["group_a"], 80);
        assert_eq!(c.leds()["group_d"], 30);
        assert_eq!(c.leds()["settings"], 30);
        assert!(!c.button("top_6").unwrap());
        assert!(rx.try_recv().is_err());
        c.button("group_d").unwrap();
        assert!(matches!(rx.try_recv().unwrap(), Command::Slot(3)));
        assert_eq!(c.snapshot()["slot"], "A");
        c.shared.lock().unwrap().status.active_slot = 3;
        c.tick();
        assert_eq!(c.snapshot()["leds"]["group_d"], 80);
        assert_eq!(c.leds()["group_a"], 30);
        for page in [1, 2, 0] {
            c.button("macro").unwrap();
            assert_eq!(c.page, page);
        }
        c.button("settings").unwrap();
        assert_eq!(c.snapshot()["leds"]["settings"], 80);
        assert_eq!(c.leds()["top_2"], 30);
        assert!(c.snapshot()["leds"].get("macro").is_none());
        c.button("mixer").unwrap();
        assert_eq!(c.snapshot()["view"], "mixer");
        c.button("mixer").unwrap();
        assert_eq!(c.snapshot()["view"], "parameters");
        c.observed = Observed::default();
        assert_eq!(c.snapshot()["leds"], json!({}));
    }
    #[test]
    fn mixer_maps_four_bound_output_controls_without_changing_module_targets(){
        let(mut c,rx)=module_client();c.module_edit=true;
        c.observed.status.configuration=Some(Configuration{controls:(0..4).map(|slot|Control{key:format!("slotLevel:{slot}:instance:1:1:2"),label:format!("Slot {} · Patch",char::from(b'A'+slot)),value:100,min:0,max:127,display:"100".into()}).collect(),..Default::default()});*c.shared.lock().unwrap()=c.observed.clone();
        c.button("mixer").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Home));assert!(matches!(rx.try_recv().unwrap(),Command::MixerMode(true)));assert!(!c.module_edit);assert_eq!(c.snapshot()["leds"]["mixer"],80);assert_eq!(c.snapshot()["leds"]["module_edit"],30);
        for (index,slot) in [(0,0),(1,1),(2,2),(3,3)]{c.turn(index,-0.1).unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Control(key,94) if key.starts_with(&format!("slotLevel:{slot}:"))));assert_eq!(c.snapshot()["pendingValues"][index],true);}
        assert!(!c.turn(5,0.1).unwrap());assert!(rx.try_recv().is_err());
        c.observed.status.running=false;c.observed.status.slot_known=true;*c.shared.lock().unwrap()=c.observed.clone();c.turn(0,-0.1).unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Control(_,88)));assert_eq!(c.snapshot()["leds"]["mixer"],80);
        c.observed.status.running=true;*c.shared.lock().unwrap()=c.observed.clone();
        c.event(&Event::Button{device:encdr::DeviceId::default(),name:"track",pressed:true});assert!(matches!(rx.try_recv().unwrap(),Command::Home));c.button("top_2").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Slot(1)));c.pending_slot=None;c.track_pressed=false;
        assert!(c.view=="parameters"&&!c.module_edit&&!c.browser_open);
        c.turn(1,0.1).unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Set{..}),"assigned knobs restored by TRACK Home");
        c.button("mixer").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::MixerMode(true)));c.button("module_edit").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::Home));assert!(matches!(rx.try_recv().unwrap(),Command::ModuleMode(true)));assert!(c.module_edit&&c.view=="parameters");
    }
    #[test]
    fn real_controls_and_function_release_are_not_optimistic() {
        let (mut c, rx) = client();
        c.observed.status.configuration.as_mut().unwrap().controls = vec![Control {
            key: "master".into(),
            label: "Master volume".into(),
            value: 200,
            min: 0,
            max: 255,
            display: "78%".into(),
        }];
        c.button("mixer").unwrap();assert!(matches!(rx.try_recv().unwrap(),Command::MixerMode(true)));
        c.turn(0, -0.1).unwrap();
        assert!(matches!(rx.try_recv().unwrap(),Command::Control(key,188) if key=="master"));
        assert_eq!(c.snapshot()["displayValues"][0], "78%");
        let dev = encdr::DeviceId::default();
        c.event(&Event::Button {
            device: dev,
            name: "shift",
            pressed: true,
        });
        assert!(matches!(rx.try_recv().unwrap(), Command::Function(true)));
        c.event(&Event::Button {
            device: dev,
            name: "shift",
            pressed: false,
        });
        assert!(matches!(rx.try_recv().unwrap(), Command::Function(false)));
        c.function(true).unwrap();
        let _ = rx.try_recv();
        c.release_function();
        assert!(matches!(rx.try_recv().unwrap(), Command::Function(false)));
        assert_eq!(c.snapshot()["leds"]["shift"],30);
    }
    #[test]
    fn function_release_retries_after_queue_pressure() {
        let (mut c, rx) = client();
        c.function(true).unwrap();
        let _ = rx.try_recv();
        for _ in 0..128 {
            c.tx.try_send(Command::Refresh).unwrap();
        }
        assert!(c.function(false).is_err());
        assert!(c.function_release_pending);
        while rx.try_recv().is_ok() {}
        c.tick();
        assert!(!c.function_release_pending);
        assert!(!c.shift_pressed);
        assert!(matches!(rx.try_recv().unwrap(), Command::Function(false)));
    }
}
