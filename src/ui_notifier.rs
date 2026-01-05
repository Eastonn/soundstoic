use std::ptr;

use objc2::{msg_send, sel};
use objc2::runtime::AnyObject;

#[derive(Clone, Copy)]
pub struct UiNotifier {
    delegate: *const AnyObject,
}

unsafe impl Send for UiNotifier {}
unsafe impl Sync for UiNotifier {}

impl UiNotifier {
    pub fn new(delegate: *const AnyObject) -> Self {
        Self { delegate }
    }

    pub fn request_refresh(&self) {
        unsafe {
            if self.delegate.is_null() {
                return;
            }

            let _: () = msg_send![
                self.delegate,
                performSelectorOnMainThread: sel!(refreshMenuState:),
                withObject: ptr::null::<AnyObject>(),
                waitUntilDone: false
            ];
        }
    }
}
