use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;

use crate::audio_manager::{self, AudioError};
use crate::ui_notifier::UiNotifier;

#[derive(Debug, Clone)]
pub enum AudioEvent {
    DefaultInputChanged,
    DevicesChanged,
    ServiceRestarted,
}

#[derive(Debug, Clone)]
pub struct LockSnapshot {
    pub enabled: bool,
    pub locked_uid: Option<String>,
    pub locked_missing: bool,
}

#[derive(Debug)]
struct LockState {
    enabled: bool,
    locked_uid: Option<String>,
    locked_missing: bool,
    last_self_set: Option<(crate::audio_sys::AudioDeviceID, Instant)>,
}

#[derive(Debug, Default)]
pub struct EnforceResult {
    pub changed: bool,
    pub locked_missing: bool,
}

pub struct Controller {
    state: Mutex<LockState>,
}

impl Controller {
    pub fn new(enabled: bool, locked_uid: Option<String>) -> Self {
        Self {
            state: Mutex::new(LockState {
                enabled,
                locked_uid,
                locked_missing: false,
                last_self_set: None,
            }),
        }
    }

    pub fn snapshot(&self) -> LockSnapshot {
        let state = self.state.lock().expect("lock state");
        LockSnapshot {
            enabled: state.enabled,
            locked_uid: state.locked_uid.clone(),
            locked_missing: state.locked_missing,
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut state = self.state.lock().expect("lock state");
        state.enabled = enabled;
    }

    pub fn set_locked_uid(&self, uid: Option<String>) {
        let mut state = self.state.lock().expect("lock state");
        state.locked_uid = uid;
        state.locked_missing = false;
    }

    pub fn enforce(&self) -> Result<EnforceResult, AudioError> {
        let (enabled, locked_uid, last_self_set) = {
            let state = self.state.lock().expect("lock state");
            (state.enabled, state.locked_uid.clone(), state.last_self_set)
        };

        let mut result = EnforceResult::default();

        if !enabled {
            return Ok(result);
        }

        let Some(locked_uid) = locked_uid else {
            return Ok(result);
        };

        let locked_id = match audio_manager::device_id_for_uid(&locked_uid) {
            Ok(id) => id,
            Err(_) => {
                let mut state = self.state.lock().expect("lock state");
                state.locked_missing = true;
                result.locked_missing = true;
                return Ok(result);
            }
        };

        if let Some((id, when)) = last_self_set {
            if id == locked_id && when.elapsed() < Duration::from_millis(350) {
                return Ok(result);
            }
        }

        let current = audio_manager::get_default_input_device()?;
        if current != locked_id {
            audio_manager::set_default_input_device(locked_id)?;
            let mut state = self.state.lock().expect("lock state");
            state.last_self_set = Some((locked_id, Instant::now()));
            state.locked_missing = false;
            result.changed = true;
        }

        Ok(result)
    }
}

pub fn run_enforcement_worker(
    rx: Receiver<AudioEvent>,
    controller: Arc<Controller>,
    ui: UiNotifier,
) {
    std::thread::spawn(move || loop {
        if rx.recv().is_err() {
            break;
        }

        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(180) {
            if rx.try_recv().is_err() {
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        let _ = controller.enforce();
        ui.request_refresh();
    });
}
