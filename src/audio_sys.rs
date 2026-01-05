#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_void;

pub type AudioObjectID = u32;
pub type AudioDeviceID = u32;
pub type AudioObjectPropertySelector = u32;
pub type AudioObjectPropertyScope = u32;
pub type AudioObjectPropertyElement = u32;
pub type OSStatus = i32;
pub type AudioStreamID = u32;

pub const K_AUDIO_OBJECT_SYSTEM_OBJECT: AudioObjectID = 1;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AudioObjectPropertyAddress {
    pub mSelector: AudioObjectPropertySelector,
    pub mScope: AudioObjectPropertyScope,
    pub mElement: AudioObjectPropertyElement,
}

pub type AudioObjectPropertyListenerProc = Option<unsafe extern "C" fn(
    in_object_id: AudioObjectID,
    in_num_addresses: u32,
    in_addresses: *const AudioObjectPropertyAddress,
    in_client_data: *mut c_void,
) -> OSStatus>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AudioValueTranslation {
    pub mInputData: *const c_void,
    pub mInputDataSize: u32,
    pub mOutputData: *mut c_void,
    pub mOutputDataSize: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AudioBuffer {
    pub mNumberChannels: u32,
    pub mDataByteSize: u32,
    pub mData: *mut c_void,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AudioBufferList {
    pub mNumberBuffers: u32,
    pub mBuffers: [AudioBuffer; 1],
}

const fn fourcc(tag: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*tag)
}

pub const K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL: u32 = fourcc(b"glob");
pub const K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN: u32 = 0;

pub const K_AUDIO_DEVICE_PROPERTY_SCOPE_INPUT: u32 = fourcc(b"inpt");

pub const K_AUDIO_HARDWARE_PROPERTY_DEVICES: u32 = fourcc(b"dev#");
pub const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE: u32 = fourcc(b"dIn ");
pub const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE: u32 = fourcc(b"dOut");
pub const K_AUDIO_HARDWARE_PROPERTY_DEVICE_FOR_UID: u32 = fourcc(b"duid");
pub const K_AUDIO_HARDWARE_PROPERTY_SERVICE_RESTARTED: u32 = fourcc(b"srst");

pub const K_AUDIO_OBJECT_PROPERTY_NAME: u32 = fourcc(b"name");
pub const K_AUDIO_DEVICE_PROPERTY_DEVICE_UID: u32 = fourcc(b"uid ");
pub const K_AUDIO_DEVICE_PROPERTY_STREAM_CONFIGURATION: u32 = fourcc(b"scfg");
pub const K_AUDIO_DEVICE_PROPERTY_STREAMS: u32 = fourcc(b"stm#");
pub const K_AUDIO_DEVICE_PROPERTY_DEVICE_IS_ALIVE: u32 = fourcc(b"aliv");

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    pub fn AudioObjectGetPropertyDataSize(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        out_data_size: *mut u32,
    ) -> OSStatus;

    pub fn AudioObjectGetPropertyData(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        io_data_size: *mut u32,
        out_data: *mut c_void,
    ) -> OSStatus;

    pub fn AudioObjectSetPropertyData(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        in_data_size: u32,
        in_data: *const c_void,
    ) -> OSStatus;

    pub fn AudioObjectAddPropertyListener(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_listener: AudioObjectPropertyListenerProc,
        in_client_data: *mut c_void,
    ) -> OSStatus;

    pub fn AudioObjectRemovePropertyListener(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_listener: AudioObjectPropertyListenerProc,
        in_client_data: *mut c_void,
    ) -> OSStatus;
}
