//! Hardware input must keep moving while the main thread services WebKit/windows.
use crate::g1::Client;
use encdr::{DeviceId, Event};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

static INPUT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn input_sequence() -> u64 { INPUT_SEQUENCE.load(Ordering::Relaxed) }
pub fn pipeline_trace_enabled() -> bool {
    if std::env::var_os("MK3_TRACE_PIPELINE").is_none(){return false;}
    let until=std::env::var("MK3_TRACE_UNTIL_MS").ok().and_then(|s|s.parse::<u128>().ok()).unwrap_or(u128::MAX);
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()<until
}

pub struct InputDispatch {
    device: Arc<Mutex<Option<DeviceId>>>,
    dirty: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl InputDispatch {
    pub fn start(
        mut receive: impl FnMut() -> Option<Event> + Send + 'static,
        client: Option<Arc<Mutex<Client>>>,
    ) -> (Self, mpsc::Receiver<Event>) {
        let device = Arc::new(Mutex::new(None));
        let dirty = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let handle = {
            let device = device.clone();
            let dirty = dirty.clone();
            let stop = stop.clone();
            thread::Builder::new()
                .name("mk3-input-dispatch".into())
                .spawn(move || {
                    while !stop.load(Ordering::Acquire) {
                        let Some(event) = receive() else {
                            thread::sleep(Duration::from_millis(1));
                            continue;
                        };
                        if let Event::DeviceDisconnected { id } = &event {
                            let mut active = device.lock().unwrap();
                            if *active == Some(*id) {
                                *active = None;
                                if let Some(client) = &client {
                                    let mut client = client.lock().unwrap();
                                    client.release_function();
                                    client.release_pads();
                                }
                                dirty.store(true, Ordering::Release);
                            }
                            if tx.send(event).is_err() {
                                break;
                            }
                            continue;
                        }
                        if matches!(event, Event::DeviceConnected { .. }) || client.is_none() {
                            if tx.send(event).is_err() {
                                break;
                            }
                            continue;
                        }
                        let active = device.lock().unwrap();
                        if *active != Some(event.device_id()) {
                            if pipeline_trace_enabled(){tracing::info!("PIPELINE stage=input_suppressed reason=device_or_ownership event={:?}",event);}
                            continue;
                        }
                        if let Some(client) = &client {
                            let sequence=INPUT_SEQUENCE.fetch_add(1,Ordering::Relaxed)+1;
                            let started=std::time::Instant::now();
                            if pipeline_trace_enabled(){tracing::info!("PIPELINE stage=input input_seq={} event={:?}",sequence,event);}
                            let mut client = client.lock().unwrap();
                            if pipeline_trace_enabled(){tracing::info!("PIPELINE stage=input_lock input_seq={} wait_ms={:.3}",sequence,started.elapsed().as_secs_f64()*1000.);}
                            let changed = client.refresh_input() | client.event(&event);
                            if changed {
                                dirty.store(true, Ordering::Release);
                            }
                        }
                    }
                    if let Some(client) = &client {
                        let mut client = client.lock().unwrap();
                        client.release_function();
                        client.release_pads();
                    }
                })
                .expect("MK3 input dispatch thread")
        };
        (
            Self {
                device,
                dirty,
                stop,
                thread: Some(handle),
            },
            rx,
        )
    }
    pub fn set_device(&self, device: Option<DeviceId>) {
        *self.device.lock().unwrap() = device;
    }
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }
}
impl Drop for InputDispatch {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
