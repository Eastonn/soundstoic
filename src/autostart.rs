use std::ffi::CStr;
use std::ptr;

use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject, Bool};

#[link(name = "ServiceManagement", kind = "framework")]
extern "C" {}

#[derive(Debug)]
pub enum AutostartError {
    Unavailable,
    Failed,
}

fn main_app_service() -> Option<*mut AnyObject> {
    unsafe {
        let cls = AnyClass::get(CStr::from_bytes_with_nul_unchecked(b"SMAppService\0"))?;
        let service: *mut AnyObject = msg_send![cls, mainAppService];
        if service.is_null() {
            None
        } else {
            Some(service)
        }
    }
}

pub fn is_enabled() -> bool {
    unsafe {
        let Some(service) = main_app_service() else {
            return false;
        };
        let status: i64 = msg_send![service, status];
        status == 1
    }
}

pub fn set_enabled(enabled: bool) -> Result<(), AutostartError> {
    unsafe {
        let Some(service) = main_app_service() else {
            return Err(AutostartError::Unavailable);
        };

        let mut error: *mut AnyObject = ptr::null_mut();
        let ok: Bool = if enabled {
            msg_send![service, registerAndReturnError: &mut error]
        } else {
            msg_send![service, unregisterAndReturnError: &mut error]
        };

        if ok.as_bool() {
            Ok(())
        } else {
            Err(AutostartError::Failed)
        }
    }
}
