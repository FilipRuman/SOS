use x86::apic::ioapic;

use crate::interrupts::apic::{self, xapic};
/// # SAFETY
/// Shouldn't cause any deadlocks
pub async fn wait_ms(length_ms: u64) {
    let time_waker = TimeWaker::new(length_ms, AtomicWaker::new());
    let waker_arc: Arc<TimeWaker> = Arc::new(time_waker);
    TIME_WAKERS.push(waker_arc.clone());
    let mut time_waiter = TimeWaiter::new(waker_arc);
    time_waiter.await;
}

static TIMER_FIRED: AtomicBool = AtomicBool::new(false);
pub static TIME_MS: AtomicU64 = AtomicU64::new(0);

static WAKER: AtomicWaker = AtomicWaker::new();
// WARN: this is called by interrupt controller so don't do anything that could cause deadlock
pub fn on_1ms_timer_interrupt() {
    TIME_MS.fetch_add(1, Ordering::Relaxed);
    TIMER_FIRED.store(true, Ordering::Relaxed);
    WAKER.wake();
}

use alloc::{string::String, sync::Arc, vec::Vec};
use conquer_once::spin::OnceCell;
use crossbeam_queue::{ArrayQueue, SegQueue};

use log::*;
use spin::Mutex;

use core::{
    pin::Pin,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    task::{Context, Poll, Waker},
};
use futures_util::task::AtomicWaker;
pub struct WaitForInterrupt;
impl Future for WaitForInterrupt {
    type Output = ();

    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if TIMER_FIRED.swap(false, Ordering::Relaxed) {
            WAKER.take();
            Poll::Ready(())
        } else {
            WAKER.register(&cx.waker());
            Poll::Pending
        }
    }
}
pub fn wait_for_next_interrupt() -> WaitForInterrupt {
    WaitForInterrupt
}

pub static TIME_WAKERS: SegQueue<Arc<TimeWaker>> = SegQueue::new();
pub(crate) async fn run_timer_loop() {
    debug!("run_timer_loop");

    loop {
        wait_for_next_interrupt().await;
        let time = TIME_MS.load(Ordering::Relaxed);

        update_time_wakers(time);
    }
}

fn update_time_wakers(current_time: u64) {
    let mut retained = Vec::new();
    while let Some(waker) = TIME_WAKERS.pop() {
        if !waker.update(current_time) {
            retained.push(waker);
        }
    }
    for w in retained {
        TIME_WAKERS.push(w);
    }
}
pub struct TimeWaiter {
    time_waker: Arc<TimeWaker>,
}
pub struct TimeWaker {
    end_time: u64,

    wake: AtomicBool,
    waker: AtomicWaker,
}

impl TimeWaker {
    pub fn new(length_ms: u64, waker: AtomicWaker) -> Self {
        TimeWaker {
            end_time: TIME_MS.load(Ordering::Relaxed) + length_ms,
            wake: AtomicBool::new(false),
            waker,
        }
    }
    // checks current time and tells weather it used its waker
    pub fn update(&self, current_time: u64) -> bool {
        if current_time < self.end_time {
            return false;
        }
        self.wake.store(true, Ordering::Relaxed);
        self.waker.wake();
        true
    }
}
impl TimeWaiter {
    pub fn new(waker: Arc<TimeWaker>) -> Self {
        TimeWaiter { time_waker: waker }
    }
}
impl Future for TimeWaiter {
    type Output = ();

    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.time_waker.wake.load(Ordering::Relaxed) {
            self.time_waker.waker.take();
            Poll::Ready(())
        } else {
            self.time_waker.waker.register(cx.waker());
            Poll::Pending
        }
    }
}
