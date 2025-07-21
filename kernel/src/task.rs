pub mod executor;
pub mod keyboard;
pub mod simple_executor;

use alloc::boxed::Box;
use core::task::Context;
use core::{future::Future, pin::Pin};
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(u64);

use core::sync::atomic::{AtomicU64, Ordering};

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct StaticTask {
    id: TaskId, // new
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl StaticTask {
    pub fn new(future: impl Future<Output = ()> + 'static + Send) -> StaticTask {
        StaticTask {
            id: TaskId::new(), // new
            future: Mutex::new(Box::pin(future)),
        }
    }
    fn poll(&mut self, context: &mut Context) -> core::task::Poll<()> {
        self.future.lock().as_mut().poll(context)
    }
}
