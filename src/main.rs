mod audio_manager;
mod audio_sys;
mod autostart;
mod config;
mod controller;
mod device_watcher;
mod tray_ui;
mod ui_notifier;

use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::unbounded;

use crate::config::ConfigStore;
use crate::controller::{run_enforcement_worker, Controller};
use crate::device_watcher::DeviceWatcher;

fn main() {
    let config = Arc::new(ConfigStore::load());
    let cfg = config.get();

    let controller = Arc::new(Controller::new(cfg.lock_enabled, cfg.locked_uid.clone()));

    let (tx, rx) = unbounded();
    let watcher = DeviceWatcher::start(tx).expect("audio watcher");

    let (app, ui) = tray_ui::init_app(controller.clone(), config.clone());

    run_enforcement_worker(rx, controller.clone(), ui.clone());

    let initial_controller = controller.clone();
    let initial_ui = ui.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        let _ = initial_controller.enforce();
        initial_ui.request_refresh();
    });

    app.run();

    drop(watcher);
}
