use std::{ffi::c_void, mem, ptr};

use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};

use crate::audio_sys::*;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: AudioDeviceID,
    pub uid: String,
    pub name: String,
    pub input_channels: u32,
}

#[derive(Debug)]
pub enum AudioError {
    OsStatus(OSStatus),
    NotFound,
}

fn ok(status: OSStatus) -> Result<(), AudioError> {
    if status == 0 {
        Ok(())
    } else {
        Err(AudioError::OsStatus(status))
    }
}

fn get_cfstring_property(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Result<String, AudioError> {
    unsafe {
        let address = AudioObjectPropertyAddress {
            mSelector: selector,
            mScope: scope,
            mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        let mut data_size: u32 = 0;
        ok(AudioObjectGetPropertyDataSize(
            object_id,
            &address,
            0,
            ptr::null(),
            &mut data_size,
        ))?;

        if data_size as usize == mem::size_of::<CFStringRef>() {
            let mut cf: CFStringRef = ptr::null();
            let mut size = data_size;
            ok(AudioObjectGetPropertyData(
                object_id,
                &address,
                0,
                ptr::null(),
                &mut size,
                (&mut cf as *mut CFStringRef).cast::<c_void>(),
            ))?;

            if cf.is_null() {
                return Err(AudioError::NotFound);
            }

            // CoreAudio can return tagged-pointer NSStrings; avoid CFRetain/CFRelease.
            let cf = std::mem::ManuallyDrop::new(CFString::wrap_under_create_rule(cf));
            return Ok(cf.to_string());
        }

        if data_size == 0 {
            return Err(AudioError::NotFound);
        }

        let mut buf = vec![0u8; data_size as usize];
        let mut size = data_size;
        ok(AudioObjectGetPropertyData(
            object_id,
            &address,
            0,
            ptr::null(),
            &mut size,
            buf.as_mut_ptr().cast::<c_void>(),
        ))?;

        if let Some(pos) = buf.iter().position(|&b| b == 0) {
            buf.truncate(pos);
        }

        Ok(String::from_utf8_lossy(&buf).to_string())
    }
}

fn get_device_ids() -> Result<Vec<AudioDeviceID>, AudioError> {
    unsafe {
        let address = AudioObjectPropertyAddress {
            mSelector: K_AUDIO_HARDWARE_PROPERTY_DEVICES,
            mScope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        let mut size: u32 = 0;
        ok(AudioObjectGetPropertyDataSize(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            ptr::null(),
            &mut size,
        ))?;

        let count = (size as usize) / mem::size_of::<AudioDeviceID>();
        let mut ids = vec![0u32; count];

        ok(AudioObjectGetPropertyData(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            ptr::null(),
            &mut size,
            ids.as_mut_ptr().cast::<c_void>(),
        ))?;

        Ok(ids)
    }
}

fn get_input_channel_count(device_id: AudioDeviceID) -> Result<u32, AudioError> {
    unsafe {
        let address = AudioObjectPropertyAddress {
            mSelector: K_AUDIO_DEVICE_PROPERTY_STREAM_CONFIGURATION,
            mScope: K_AUDIO_DEVICE_PROPERTY_SCOPE_INPUT,
            mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        let mut size: u32 = 0;
        ok(AudioObjectGetPropertyDataSize(
            device_id,
            &address,
            0,
            ptr::null(),
            &mut size,
        ))?;

        if size == 0 {
            return Ok(0);
        }

        let word_count = (size as usize + 7) / 8;
        let mut buffer = vec![0u64; word_count.max(1)];
        ok(AudioObjectGetPropertyData(
            device_id,
            &address,
            0,
            ptr::null(),
            &mut size,
            buffer.as_mut_ptr().cast::<c_void>(),
        ))?;

        let abl = buffer.as_ptr() as *const AudioBufferList;
        let abl = &*abl;
        let buffers = std::slice::from_raw_parts(abl.mBuffers.as_ptr(), abl.mNumberBuffers as usize);

        let mut channels = 0u32;
        for b in buffers {
            channels = channels.saturating_add(b.mNumberChannels);
        }

        Ok(channels)
    }
}

fn has_input_streams(device_id: AudioDeviceID) -> bool {
    unsafe {
        let address = AudioObjectPropertyAddress {
            mSelector: K_AUDIO_DEVICE_PROPERTY_STREAMS,
            mScope: K_AUDIO_DEVICE_PROPERTY_SCOPE_INPUT,
            mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        let mut size: u32 = 0;
        let status = AudioObjectGetPropertyDataSize(
            device_id,
            &address,
            0,
            ptr::null(),
            &mut size,
        );
        if status != 0 {
            return false;
        }
        let count = (size as usize) / mem::size_of::<AudioStreamID>();
        count > 0
    }
}

pub fn list_input_devices() -> Result<Vec<DeviceInfo>, AudioError> {
    let mut out = Vec::new();
    for id in get_device_ids()? {
        let mut input_channels = 0u32;
        let mut is_input = false;

        match get_input_channel_count(id) {
            Ok(count) => {
                input_channels = count;
                if count > 0 {
                    is_input = true;
                }
            }
            Err(_) => {}
        }

        if !is_input && has_input_streams(id) {
            is_input = true;
        }

        if !is_input {
            continue;
        }

        let name = get_cfstring_property(id, K_AUDIO_OBJECT_PROPERTY_NAME, K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL)
            .unwrap_or_else(|_| "<unknown>".to_string());
        let uid = get_cfstring_property(id, K_AUDIO_DEVICE_PROPERTY_DEVICE_UID, K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL)
            .unwrap_or_else(|_| "<no-uid>".to_string());

        out.push(DeviceInfo {
            id,
            uid,
            name,
            input_channels,
        });
    }

    Ok(out)
}

pub fn get_default_input_device() -> Result<AudioDeviceID, AudioError> {
    unsafe {
        let address = AudioObjectPropertyAddress {
            mSelector: K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE,
            mScope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        let mut device_id: AudioDeviceID = 0;
        let mut size = mem::size_of::<AudioDeviceID>() as u32;

        ok(AudioObjectGetPropertyData(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            ptr::null(),
            &mut size,
            (&mut device_id as *mut AudioDeviceID).cast::<c_void>(),
        ))?;

        Ok(device_id)
    }
}

pub fn set_default_input_device(device_id: AudioDeviceID) -> Result<(), AudioError> {
    unsafe {
        let address = AudioObjectPropertyAddress {
            mSelector: K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE,
            mScope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        let size = mem::size_of::<AudioDeviceID>() as u32;
        ok(AudioObjectSetPropertyData(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            ptr::null(),
            size,
            (&device_id as *const AudioDeviceID).cast::<c_void>(),
        ))
    }
}

pub fn device_name_by_id(device_id: AudioDeviceID) -> Result<String, AudioError> {
    get_cfstring_property(
        device_id,
        K_AUDIO_OBJECT_PROPERTY_NAME,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
}

pub fn device_uid_by_id(device_id: AudioDeviceID) -> Result<String, AudioError> {
    get_cfstring_property(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_DEVICE_UID,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
}

pub fn device_id_for_uid(uid: &str) -> Result<AudioDeviceID, AudioError> {
    // Prefer the HAL translation API when available, fall back to enumeration.
    if let Ok(id) = device_id_for_uid_via_translation(uid) {
        return Ok(id);
    }

    for device in list_input_devices()? {
        if device.uid == uid {
            return Ok(device.id);
        }
    }

    Err(AudioError::NotFound)
}

fn device_id_for_uid_via_translation(uid: &str) -> Result<AudioDeviceID, AudioError> {
    unsafe {
        let address = AudioObjectPropertyAddress {
            mSelector: K_AUDIO_HARDWARE_PROPERTY_DEVICE_FOR_UID,
            mScope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            mElement: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        let cf_uid = CFString::new(uid);
        let uid_ref: CFStringRef = cf_uid.as_concrete_TypeRef();
        let mut out_device: AudioDeviceID = 0;
        let mut translation = AudioValueTranslation {
            mInputData: (&uid_ref as *const CFStringRef).cast::<c_void>(),
            mInputDataSize: mem::size_of::<CFStringRef>() as u32,
            mOutputData: (&mut out_device as *mut AudioDeviceID).cast::<c_void>(),
            mOutputDataSize: mem::size_of::<AudioDeviceID>() as u32,
        };

        let mut size = mem::size_of::<AudioValueTranslation>() as u32;
        ok(AudioObjectGetPropertyData(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            ptr::null(),
            &mut size,
            (&mut translation as *mut AudioValueTranslation).cast::<c_void>(),
        ))?;

        if out_device == 0 {
            return Err(AudioError::NotFound);
        }

        Ok(out_device)
    }
}

pub fn device_name_for_uid(uid: &str) -> Result<String, AudioError> {
    if let Ok(id) = device_id_for_uid(uid) {
        if let Ok(name) = device_name_by_id(id) {
            return Ok(name);
        }
    }

    for device in list_input_devices()? {
        if device.uid == uid {
            return Ok(device.name);
        }
    }

    Err(AudioError::NotFound)
}
