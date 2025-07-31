use alloc::{string::String, vec::Vec};
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

use log::*;
use spin::Mutex;
/// Called by the keyboard interrupt handler
///
/// Must not block or allocate.
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            warn!("scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake(); 
        }
    } else {
        warn!("scancode queue uninitialized");
    }
}
pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures_util::stream::Stream;

use futures_util::task::AtomicWaker;

static WAKER: AtomicWaker = AtomicWaker::new();

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // fast path
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

use futures_util::stream::StreamExt;
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts};

pub(crate) async fn print_keypresses() {
    debug!("print_keypresses");
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                //debug!("key pressed:{key:?}");
                // call all listeners
                for listener in ON_KEY_PRESSED_LISTENERS.lock().iter() {
                    listener(&key);
                }


            }
        }
    }

}
pub type OnKeyFunction = fn(&DecodedKey);
pub static ON_KEY_PRESSED_LISTENERS: Mutex<Vec<OnKeyFunction>> = Mutex::new(Vec::new());
