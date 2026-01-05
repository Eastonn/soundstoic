use std::cell::OnceCell;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, ClassType, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSControlStateValueOff, NSControlStateValueOn,
    NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
    NSApplicationDelegate,
};
use objc2_foundation::{ns_string, MainThreadMarker, NSObject, NSObjectProtocol, NSNotification, NSString};

use crate::audio_manager;
use crate::autostart;
use crate::config::ConfigStore;
use crate::controller::{Controller, LockSnapshot};
use crate::ui_notifier::UiNotifier;

fn load_status_image() -> Option<Retained<NSImage>> {
    let symbol = ns_string!("mic");
    let image: Option<Retained<NSImage>> = unsafe {
        msg_send![
            NSImage::class(),
            imageWithSystemSymbolName: symbol,
            accessibilityDescription: Option::<&NSString>::None
        ]
    };
    if let Some(image) = image.as_ref() {
        image.setTemplate(true);
    }
    image
}

#[derive(Default)]
struct Ivars {
    status_item: OnceCell<Retained<NSStatusItem>>,
    status_image: OnceCell<Retained<NSImage>>,
    menu: OnceCell<Retained<NSMenu>>,
    devices_menu: OnceCell<Retained<NSMenu>>,
    toggle_lock_item: OnceCell<Retained<NSMenuItem>>,
    start_login_item: OnceCell<Retained<NSMenuItem>>,
    current_item: OnceCell<Retained<NSMenuItem>>,
    locked_item: OnceCell<Retained<NSMenuItem>>,
    controller: OnceCell<Arc<Controller>>,
    config: OnceCell<Arc<ConfigStore>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, notification: &NSNotification) {
            let mtm = self.mtm();
            let app = notification.object()
                .and_then(|obj| obj.downcast::<NSApplication>().ok())
                .unwrap_or_else(|| NSApplication::sharedApplication(mtm));

            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

            let status_bar = NSStatusBar::systemStatusBar();
            let status_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);

            if let Some(image) = load_status_image() {
                status_item.setImage(Some(&image));
                status_item.setTitle(None);
                self.ivars().status_image.set(image).ok();
            } else {
                #[allow(deprecated)]
                status_item.setTitle(Some(ns_string!("MicLock")));
            }

            let menu = NSMenu::new(mtm);
            menu.setAutoenablesItems(false);

            let toggle = NSMenuItem::alloc(mtm);
            let toggle = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    toggle,
                    ns_string!("Input Lock"),
                    Some(sel!(toggleInputLock:)),
                    ns_string!(""),
                )
            };
            unsafe { toggle.setTarget(Some(self)) };
            menu.addItem(&toggle);

            let select_item = NSMenuItem::alloc(mtm);
            let select_item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    select_item,
                    ns_string!("Select Locked Mic..."),
                    None,
                    ns_string!(""),
                )
            };
            let devices_menu = NSMenu::new(mtm);
            select_item.setSubmenu(Some(&devices_menu));
            menu.addItem(&select_item);

            menu.addItem(&NSMenuItem::separatorItem(mtm));

            let current = NSMenuItem::alloc(mtm);
            let current = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    current,
                    ns_string!("Current Input: ..."),
                    None,
                    ns_string!(""),
                )
            };
            current.setEnabled(false);
            menu.addItem(&current);

            let locked = NSMenuItem::alloc(mtm);
            let locked = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    locked,
                    ns_string!("Locked Input: ..."),
                    None,
                    ns_string!(""),
                )
            };
            locked.setEnabled(false);
            menu.addItem(&locked);

            menu.addItem(&NSMenuItem::separatorItem(mtm));

            let start_login = NSMenuItem::alloc(mtm);
            let start_login = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    start_login,
                    ns_string!("Start at Login"),
                    Some(sel!(toggleStartAtLogin:)),
                    ns_string!(""),
                )
            };
            unsafe { start_login.setTarget(Some(self)) };
            menu.addItem(&start_login);

            menu.addItem(&NSMenuItem::separatorItem(mtm));

            let quit = NSMenuItem::alloc(mtm);
            let quit = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    quit,
                    ns_string!("Quit"),
                    Some(sel!(terminate:)),
                    ns_string!("q"),
                )
            };
            menu.addItem(&quit);

            status_item.setMenu(Some(&menu));

            self.ivars().status_item.set(status_item).ok();
            self.ivars().menu.set(menu).ok();
            self.ivars().devices_menu.set(devices_menu).ok();
            self.ivars().toggle_lock_item.set(toggle).ok();
            self.ivars().start_login_item.set(start_login).ok();
            self.ivars().current_item.set(current).ok();
            self.ivars().locked_item.set(locked).ok();

            self.refresh_menu_state_impl();
        }
    }

    impl AppDelegate {
        #[unsafe(method(refreshMenuState:))]
        fn refresh_menu_state(&self, _sender: Option<&NSObject>) {
            self.refresh_menu_state_impl();
        }

        #[unsafe(method(toggleInputLock:))]
        fn toggle_input_lock(&self, _sender: Option<&NSMenuItem>) {
            let mut cfg = self.config().get();
            cfg.lock_enabled = !cfg.lock_enabled;
            self.config().update(|c| c.lock_enabled = cfg.lock_enabled);
            self.controller().set_enabled(cfg.lock_enabled);
            let _ = self.controller().enforce();
            self.refresh_menu_state_impl();
        }

        #[unsafe(method(selectLockedMic:))]
        fn select_locked_mic(&self, sender: Option<&NSMenuItem>) {
            let Some(item) = sender else { return; };
            let uid_obj = item.representedObject();
            let uid = uid_obj
                .and_then(|obj| obj.downcast::<NSString>().ok())
                .map(|s| s.to_string());

            if let Some(uid) = uid {
                self.config().update(|c| c.locked_uid = Some(uid.clone()));
                self.controller().set_locked_uid(Some(uid));
                let _ = self.controller().enforce();
            }

            self.refresh_menu_state_impl();
        }

        #[unsafe(method(toggleStartAtLogin:))]
        fn toggle_start_at_login(&self, _sender: Option<&NSMenuItem>) {
            let current = autostart::is_enabled();
            if autostart::set_enabled(!current).is_ok() {
                self.config().update(|c| c.start_at_login = !current);
            }
            self.refresh_menu_state_impl();
        }
    }
);

impl AppDelegate {
    pub fn new(mtm: MainThreadMarker, controller: Arc<Controller>, config: Arc<ConfigStore>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars::default());
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };
        this.ivars().controller.set(controller).ok();
        this.ivars().config.set(config).ok();
        this
    }

    fn controller(&self) -> &Controller {
        self.ivars().controller.get().expect("controller")
    }

    fn config(&self) -> &ConfigStore {
        self.ivars().config.get().expect("config")
    }

    fn lock_snapshot(&self) -> LockSnapshot {
        self.controller().snapshot()
    }

    fn refresh_menu_state_impl(&self) {
        let snapshot = self.lock_snapshot();
        self.update_status_title(&snapshot);
        self.update_menu_items(&snapshot);
        self.rebuild_devices_menu(&snapshot);
    }

    fn update_status_title(&self, snapshot: &LockSnapshot) {
        if let Some(item) = self.ivars().status_item.get() {
            if self.ivars().status_image.get().is_some() {
                item.setTitle(None);
                return;
            }
            #[allow(deprecated)]
            if snapshot.enabled && snapshot.locked_missing {
                item.setTitle(Some(ns_string!("MicLock!")));
            } else {
                item.setTitle(Some(ns_string!("MicLock")));
            }
        }
    }

    fn update_menu_items(&self, snapshot: &LockSnapshot) {
        if let Some(toggle) = self.ivars().toggle_lock_item.get() {
            toggle.setState(if snapshot.enabled {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }

        if let Some(start_login) = self.ivars().start_login_item.get() {
            let enabled = autostart::is_enabled();
            start_login.setState(if enabled {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }

        if let Some(current_item) = self.ivars().current_item.get() {
            let current = match audio_manager::get_default_input_device()
                .ok()
                .and_then(|id| audio_manager::device_name_by_id(id).ok())
            {
                Some(name) => name,
                None => "<unknown>".to_string(),
            };
            let title = format!("Current Input: {}", current);
            let title = NSString::from_str(&title);
            current_item.setTitle(&title);
        }

        if let Some(locked_item) = self.ivars().locked_item.get() {
            let title = match snapshot.locked_uid.as_deref() {
                Some(uid) => match audio_manager::device_name_for_uid(uid) {
                    Ok(name) => {
                        if snapshot.locked_missing {
                            format!("Locked Input: {} (missing)", name)
                        } else {
                            format!("Locked Input: {}", name)
                        }
                    }
                    Err(_) => {
                        if snapshot.locked_missing {
                            format!("Locked Input: {} (missing)", uid)
                        } else {
                            format!("Locked Input: {}", uid)
                        }
                    }
                },
                None => "Locked Input: (not set)".to_string(),
            };
            let title = NSString::from_str(&title);
            locked_item.setTitle(&title);
        }
    }

    fn rebuild_devices_menu(&self, snapshot: &LockSnapshot) {
        let Some(menu) = self.ivars().devices_menu.get() else { return; };
        menu.removeAllItems();

        let devices = match audio_manager::list_input_devices() {
            Ok(list) => list,
            Err(_) => Vec::new(),
        };

        if devices.is_empty() {
            let mtm = self.mtm();
            let item = NSMenuItem::alloc(mtm);
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    item,
                    ns_string!("No input devices"),
                    None,
                    ns_string!(""),
                )
            };
            item.setEnabled(false);
            menu.addItem(&item);
            return;
        }

        let mtm = self.mtm();
        for device in devices {
            let title = NSString::from_str(&device.name);
            let item = NSMenuItem::alloc(mtm);
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    item,
                    &title,
                    Some(sel!(selectLockedMic:)),
                    ns_string!(""),
                )
            };
            unsafe { item.setTarget(Some(self)) };

            let uid = NSString::from_str(&device.uid);
            unsafe { item.setRepresentedObject(Some(&uid)) };
            if snapshot
                .locked_uid
                .as_deref()
                .map(|locked| locked == device.uid)
                .unwrap_or(false)
            {
                item.setState(NSControlStateValueOn);
            }

            menu.addItem(&item);
        }
    }

}

pub fn init_app(
    controller: Arc<Controller>,
    config: Arc<ConfigStore>,
) -> (Retained<NSApplication>, UiNotifier) {
    let mtm = MainThreadMarker::new().expect("main thread");
    let app = NSApplication::sharedApplication(mtm);

    let delegate = AppDelegate::new(mtm, controller, config);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    let delegate_ptr = &*delegate as *const AppDelegate as *const AnyObject;
    let notifier = UiNotifier::new(delegate_ptr);

    std::mem::forget(delegate);
    (app, notifier)
}
