use std::{sync::Arc, thread, time::Duration};

use crate::{PlatformDispatcher, TaskLabel};
use async_task::Runnable;
use parking::{Parker, Unparker};
use parking_lot::Mutex;

pub(crate) struct LinuxDispatcher {
    parker: Arc<Mutex<Parker>>,
}

impl Default for LinuxDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxDispatcher {
    pub fn new() -> Self {
        LinuxDispatcher {
            parker: Arc::new(Mutex::new(Parker::new())),
        }
    }
}

impl PlatformDispatcher for LinuxDispatcher {
    fn is_main_thread(&self) -> bool {
        rustix::thread::gettid() == rustix::process::getpid()
    }
    fn dispatch(&self, runnable: Runnable, _: Option<TaskLabel>) {
        std::thread::spawn(move || runnable.run());
    }

    fn dispatch_on_main_thread(&self, runnable: Runnable) {
        std::thread::spawn(move || runnable.run());
    }

    fn dispatch_after(&self, duration: Duration, runnable: Runnable) {
        std::thread::spawn(move || {
            thread::sleep(duration);
            runnable.run();
        });
    }

    fn tick(&self, _background_only: bool) -> bool {
        false
    }

    fn park(&self) {
        self.parker.lock().park()
    }

    fn unparker(&self) -> Unparker {
        self.parker.lock().unparker()
    }
}
