#![no_std]

pub mod graphics;

use core::u8;

use alloc::{
    boxed::Box,
    vec::{self, Vec},
};
use conquer_once::spin::OnceCell;
use graphics::*;
use kernel::task::StaticTask;
use pc_keyboard::DecodedKey;
use spin::Mutex;
extern crate alloc;
/// should be called after kernel is initialized
pub fn init_os() {
    kernel::task::keyboard::ON_KEY_PRESSED_LISTENERS
        .lock()
        .push(on_key_pressed);

    kernel::logger::ON_LOG_LISTENERS.lock().push(on_log);

    SCREEN_SIZE_PIXELS.init_once(move || {
        let info = kernel::graphics::RENDERER
            .get()
            .expect("renderer was not yet initialized!")
            .lock()
            .info;
        Vec2 {
            x: info.width as u16,
            y: info.height as u16,
        }
    });
    log::debug!("os is initialized!");
}

// pub fn listen_to_logs(&mut) {
//     kernel::logger::ON_LOG_LISTENERS.lock().push(function);
// }
pub fn exec_async_task(future: impl Future<Output = ()> + 'static + Send) {
    kernel::task::executor::TASK_SPAWNER.spawn(StaticTask::new(future));
}
pub static SCREEN_SIZE_PIXELS: OnceCell<Vec2> = OnceCell::uninit();
pub fn run_app(mut app: AppType) {
    // later create func for creating new windows so they fit with other ones

    let window_settinggs =
        WindowSettings::new(*SCREEN_SIZE_PIXELS.get().unwrap(), Vec2 { x: 0, y: 0 });
    (*app).init(window_settinggs);

    let mut lock = FOCUSED_APP.lock();
    *lock = Some(app);
}

pub type AppType = Box<dyn App + Send>;

static FOCUSED_APP: Mutex<Option<AppType>> = Mutex::new(None);

pub static ON_LOG_LISTENERS: Mutex<Vec<Mutex<&mut Box<dyn App + Send>>>> = Mutex::new(Vec::new());

pub fn on_log(log: &[u8; MAX_LOG_SIZE]) {
    let listeners = ON_LOG_LISTENERS.lock();
    for app in listeners.iter() {
        app.lock().on_log(log);
    }
}

pub fn on_key_pressed(key: &DecodedKey) {
    let mut lock = FOCUSED_APP.lock();
    if let Some(ref mut app) = *lock {
        app.on_key_pressed(key);
    }
}

pub const MAX_LOG_SIZE: usize = kernel::logger::MAX_LOG_SIZE;
pub trait App {
    fn on_key_pressed(&mut self, key: &DecodedKey);
    fn on_time(&mut self);
    fn init(&mut self, graphics_data: WindowSettings);

    fn on_log(&mut self, log: &[u8; MAX_LOG_SIZE]);
}

pub fn shutdown() {
    kernel::qemu::exit_qemu(kernel::qemu::QemuExitCode::Success);
}
