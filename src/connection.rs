//! Pure connection lifecycle; independent of USB or rendering.
use serde_json::{json, Value};

#[derive(Debug, PartialEq)]
pub struct Change {
    pub retarget: bool,
    pub resume_output: bool,
}

pub struct Link<T> {
    device: Option<T>,
    owned: bool,
    connections: u64,
    disconnects: u64,
    handovers: u64,
}
impl<T: Copy + Eq> Link<T> {
    pub fn new() -> Self {
        Self {
            device: None,
            owned: false,
            connections: 0,
            disconnects: 0,
            handovers: 0,
        }
    }
    pub fn device(&self) -> Option<T> {
        self.device
    }
    pub fn accepts_input(&self, device: T) -> bool {
        self.device == Some(device) && self.owned
    }
    pub fn observe(&mut self, device: Option<T>, has_control: bool) -> Option<Change> {
        let owned = device.is_some() && has_control;
        if self.device == device && self.owned == owned {
            return None;
        }
        let retarget = self.device != device && device.is_some();
        let resume_output = owned && (!self.owned || retarget);
        if self.device != device {
            if self.device.is_some() {
                self.disconnects = self.disconnects.saturating_add(1);
            }
            if device.is_some() {
                self.connections = self.connections.saturating_add(1);
            }
        } else if self.owned && !owned {
            self.handovers = self.handovers.saturating_add(1);
        }
        self.device = device;
        self.owned = owned;
        Some(Change {
            retarget,
            resume_output,
        })
    }
    pub fn disconnected(&mut self, device: T) -> bool {
        self.device == Some(device) && self.observe(None, false).is_some()
    }
    pub fn snapshot(&self) -> Value {
        json!({
            "status": if self.device.is_none() { "WAITING FOR MK3" } else if self.owned { "ACTIVE" } else { "OTHER APP HAS CONTROL" },
            "connected": self.device.is_some(), "hasControl": self.owned,
            "connections": self.connections, "reconnects": self.connections.saturating_sub(1),
            "disconnects": self.disconnects, "handovers": self.handovers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unplug_and_same_address_replug_retargets_without_resetting_session() {
        let mut link = Link::new();
        assert!(!link.accepts_input(1));
        assert!(link.observe(Some(1), true).unwrap().resume_output);
        assert!(link.disconnected(1));
        assert!(!link.accepts_input(1));
        assert!(!link.disconnected(1));
        let change = link.observe(Some(1), true).unwrap();
        assert!(change.retarget && change.resume_output);
        assert_eq!(link.snapshot()["reconnects"], 1);
        assert_eq!(link.snapshot()["disconnects"], 1);
        assert!(link.observe(Some(1), true).is_none());
    }
    #[test]
    fn handover_suspends_input_and_resumes_output_without_a_reconnect() {
        let mut link = Link::new();
        link.observe(Some(1), true);
        let change = link.observe(Some(1), false).unwrap();
        assert!(!change.retarget && !change.resume_output);
        assert!(!link.accepts_input(1));
        let change = link.observe(Some(1), true).unwrap();
        assert!(!change.retarget && change.resume_output);
        assert_eq!(link.snapshot()["handovers"], 1);
        assert_eq!(link.snapshot()["reconnects"], 0);
    }
    #[test]
    fn stale_old_device_events_cannot_disconnect_or_edit_replacement() {
        let mut link = Link::new();
        link.observe(Some(1), true);
        link.observe(Some(2), true);
        assert!(!link.disconnected(1));
        assert!(!link.accepts_input(1));
        assert!(link.accepts_input(2));
        assert_eq!(link.snapshot()["connections"], 2);
    }
    #[test]
    fn absent_device_never_has_control() {
        let mut link: Link<u8> = Link::new();
        assert!(link.observe(None, true).is_none());
        assert_eq!(link.snapshot()["hasControl"], false);
    }
}
