//! Development session and optional project-fixture persistence.
use crate::{g1, hardware_test::Panel, storage};
use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
struct Persistence {
    path: PathBuf,
    saved: Option<u64>,
    due: Option<Instant>,
}
pub struct Session {
    panel: Panel,
    g1: Option<Arc<Mutex<g1::Client>>>,
    persistence: Option<Persistence>,
}
impl Deref for Session {
    type Target = Panel;
    fn deref(&self) -> &Panel {
        &self.panel
    }
}
impl DerefMut for Session {
    fn deref_mut(&mut self) -> &mut Panel {
        &mut self.panel
    }
}
impl Session {
    pub fn new(
        test: bool,
        simulated: bool,
        path: Option<PathBuf>,
        address: Option<std::net::SocketAddr>,
        patch_dir: Option<PathBuf>,
    ) -> Result<Self, String> {
        let mut panel = Panel::new(test, simulated);
        let persistence = if test {
            None
        } else if let Some(path) = path {
            let loaded = storage::load(&path)?;
            let existed = loaded.is_some();
            if let Some(project) = loaded {
                panel.restore(project)?;
                println!("Restored mock project: {}", path.display());
            }
            Some(Persistence {
                path,
                saved: existed.then(|| panel.revision()),
                due: None,
            })
        } else {
            None
        };
        Ok(Self {
            panel,
            persistence,
            g1: address.map(|address| {
                Arc::new(Mutex::new(g1::Client::new(
                    address,
                    patch_dir.unwrap_or_else(|| {
                        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("patches")
                    }),
                )))
            }),
        })
    }
    pub fn hardware_input(&self) -> Option<Arc<Mutex<g1::Client>>> {
        self.g1.clone()
    }
    pub fn leds(&self) -> serde_json::Value {
        if let Some(g1) = &self.g1 { g1.lock().unwrap().leds() }
        else { self.panel.snapshot()["leds"].clone() }
    }
    pub fn snapshot(&self) -> serde_json::Value {
        if let Some(g1) = &self.g1 {
            let mut snapshot = g1.lock().unwrap().snapshot();
            snapshot["connection"] = self.panel.connection().clone();
            snapshot
        } else {
            self.panel.snapshot()
        }
    }
    pub fn event(&mut self, event: &encdr::Event) -> bool {
        if let Some(g1) = &mut self.g1 {
            g1.lock().unwrap().event(event)
        } else {
            self.panel.event(event)
        }
    }
    pub fn set_connection(&mut self, connection: serde_json::Value) {
        if connection["hasControl"] != true {
            if let Some(g1) = &mut self.g1 {
                let mut g1 = g1.lock().unwrap();
                g1.release_function();
                g1.release_pads();
            }
        }
        self.panel.set_connection(connection);
    }
    pub fn tick(&mut self) -> bool {
        let changed = self.g1.as_ref().is_some_and(|g1| g1.lock().unwrap().tick());
        if let Some(store) = &mut self.persistence {
            if store.saved != Some(self.panel.revision()) {
                let due = *store
                    .due
                    .get_or_insert_with(|| Instant::now() + Duration::from_millis(500));
                if Instant::now() >= due {
                    match storage::save(&store.path, self.panel.project()) {
                        Ok(()) => {
                            store.saved = Some(self.panel.revision());
                            store.due = None;
                        }
                        Err(error) => {
                            eprintln!("Autosave failed ({}): {error}", store.path.display());
                            store.due = Some(Instant::now() + Duration::from_secs(2));
                        }
                    }
                }
            }
        }
        changed
    }
    pub fn finish(&mut self) -> Result<(), String> {
        if let Some(store) = &mut self.persistence {
            if store.saved != Some(self.panel.revision()) {
                storage::save(&store.path, self.panel.project())?;
                store.saved = Some(self.panel.revision());
            }
        }
        Ok(())
    }
    pub fn command(&mut self, command: &str) -> Result<bool, String> {
        let command = command.trim();
        if let Some(g1) = &mut self.g1 {
            return g1.lock().unwrap().command(command);
        }
        if let Some(path) = command.strip_prefix("save ") {
            storage::save(std::path::Path::new(path.trim()), self.panel.project())?;
            println!("Saved mock project: {}", path.trim());
            return Ok(false);
        }
        if let Some(path) = command.strip_prefix("load ") {
            let project = storage::load(std::path::Path::new(path.trim()))?
                .ok_or("Project file not found")?;
            return self.panel.restore(project);
        }
        if self.panel.command(command) {
            Ok(true)
        } else {
            Err(format!(
                "Unknown command or invalid value: {command}; use --help"
            ))
        }
    }
}
