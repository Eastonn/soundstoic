use std::ffi::c_void;

use crossbeam_channel::Sender;

use crate::audio_sys::*;
use crate::controller::AudioEvent;
use crate::audio_manager::AudioError;

struct ListenerContext {
    tx: Sender<AudioEvent>,
}

unsafe extern "C" fn audio_object_listener(
    _in_object_id: AudioObjectID,
    in_num_addresses: u32,
    in_addresses: *const AudioObjectPropertyAddress,
    in_client_data: *mut c_void,
) -> OSStatus {
    let ctx = &*(in_client_data as *const ListenerContext);
    if in_addresses.is_null() || in_num_addresses == 0 {
        let _ = ctx.tx.send(AudioEvent::DefaultInputChanged);
        return 0;
    }

    let addresses = std::slice::from_raw_parts(in_addresses, in_num_addresses as usize);
    for addr in addresses {
        match addr.mSelector {
            K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE => {
                let _ = ctx.tx.send(AudioEvent::DefaultInputChanged);
            }
            K_AUDIO_HARDWARE_PROPERTY_DEVICES => {
                let _ = ctx.tx.send(AudioEvent::DevicesChanged);
            }
            K_AUDIO_HARDWARE_PROPERTY_SERVICE_RESTARTED => {
                let _ = ctx.tx.send(AudioEvent::ServiceRestarted);
            }
            _ => {}
        }
    }

    0
}

pub struct DeviceWatcher {
    ctx_raw: *mut ListenerContext,
}

impl DeviceWatcher {
    pub fn start(tx: Sender<AudioEvent>) -> Result<Self, AudioError> {
        unsafe {
            let ctx = Box::new(ListenerContext { tx });
            let ctx_raw = Box::into_raw(ctx);

            let default_addr = AudioObjectPropertyAddress {
                mSelector: K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE,
                mScope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
                mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
            };
            let devices_addr = AudioObjectPropertyAddress {
                mSelector: K_AUDIO_HARDWARE_PROPERTY_DEVICES,
                mScope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
                mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
            };
            let service_addr = AudioObjectPropertyAddress {
                mSelector: K_AUDIO_HARDWARE_PROPERTY_SERVICE_RESTARTED,
                mScope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
                mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
            };

            let status = AudioObjectAddPropertyListener(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &default_addr,
                Some(audio_object_listener),
                ctx_raw.cast::<c_void>(),
            );
            if status != 0 {
                return Err(AudioError::OsStatus(status));
            }

            let status = AudioObjectAddPropertyListener(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &devices_addr,
                Some(audio_object_listener),
                ctx_raw.cast::<c_void>(),
            );
            if status != 0 {
                return Err(AudioError::OsStatus(status));
            }

            let _ = AudioObjectAddPropertyListener(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &service_addr,
                Some(audio_object_listener),
                ctx_raw.cast::<c_void>(),
            );

            Ok(Self { ctx_raw })
        }
    }
}

impl Drop for DeviceWatcher {
    fn drop(&mut self) {
        unsafe {
            if self.ctx_raw.is_null() {
                return;
            }

            let default_addr = AudioObjectPropertyAddress {
                mSelector: K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE,
                mScope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
                mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
            };
            let devices_addr = AudioObjectPropertyAddress {
                mSelector: K_AUDIO_HARDWARE_PROPERTY_DEVICES,
                mScope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
                mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
            };
            let service_addr = AudioObjectPropertyAddress {
                mSelector: K_AUDIO_HARDWARE_PROPERTY_SERVICE_RESTARTED,
                mScope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
                mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
            };

            let _ = AudioObjectRemovePropertyListener(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &default_addr,
                Some(audio_object_listener),
                self.ctx_raw.cast::<c_void>(),
            );
            let _ = AudioObjectRemovePropertyListener(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &devices_addr,
                Some(audio_object_listener),
                self.ctx_raw.cast::<c_void>(),
            );
            let _ = AudioObjectRemovePropertyListener(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &service_addr,
                Some(audio_object_listener),
                self.ctx_raw.cast::<c_void>(),
            );

            let _ = Box::from_raw(self.ctx_raw);
        }
    }
}
