use crate::{logger, serial::SerialPort};
use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use core::fmt::Write;
use crossbeam_queue::ArrayQueue;
use log::{LevelFilter, warn};
use spin::Mutex;
use spinning_top::Spinlock;

pub struct LockedLogger {
    serial: Spinlock<SerialPort>,
}

static LOGGER: OnceCell<LockedLogger> = OnceCell::uninit();

pub struct LogStream {
    _private: (),
}
impl LogStream {
    pub fn new() -> Self {
        LOG_QUE
            .try_init_once(|| ArrayQueue::new(32))
            .expect("LogStream::new should only be called once");
        LogStream { _private: () }
    }
}
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures_util::stream::Stream;

use futures_util::task::AtomicWaker;

static WAKER: AtomicWaker = AtomicWaker::new();

pub const MAX_LOG_SIZE: usize = 80;

impl Stream for LogStream {
    type Item = [u8; MAX_LOG_SIZE];

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<[u8; MAX_LOG_SIZE]>> {
        let queue = LOG_QUE.try_get().expect("log queue not initialized");

        // fast path
        if let Some(log) = queue.pop() {
            return Poll::Ready(Some(log));
        }
        WAKER.register(cx.waker());
        match queue.pop() {
            Some(log) => {
                WAKER.take();
                Poll::Ready(Some(log))
            }
            None => Poll::Pending,
        }
    }
}

use futures_util::stream::StreamExt;

pub(crate) async fn handel_log_que() {
    let mut stream = LogStream::new();
    while let Some(log) = stream.next().await {
        for listener in ON_LOG_LISTENERS.lock().iter() {
            listener(&log);
        }
    }
}
pub type OnLogFunction = fn(&[u8; MAX_LOG_SIZE]);
pub static ON_LOG_LISTENERS: Mutex<Vec<OnLogFunction>> = Mutex::new(Vec::new());

impl LockedLogger {
    pub fn new() -> Self {
        LockedLogger {
            serial: Spinlock::new(unsafe { SerialPort::init() }),
        }
    }
}

pub fn init_logger(log_level: LevelFilter) {
    let logger = logger::LOGGER.get_or_init(LockedLogger::new);

    log::set_logger(logger).expect("setting logger did not succeed");
    log::set_max_level(convert_level(log_level));
    LogStream::new();
    log::info!("initialized logs");
}
fn convert_level(level: LevelFilter) -> log::LevelFilter {
    match level {
        LevelFilter::Off => log::LevelFilter::Off,
        LevelFilter::Error => log::LevelFilter::Error,
        LevelFilter::Warn => log::LevelFilter::Warn,
        LevelFilter::Info => log::LevelFilter::Info,
        LevelFilter::Debug => log::LevelFilter::Debug,
        LevelFilter::Trace => log::LevelFilter::Trace,
    }
}

static LOG_QUE: OnceCell<ArrayQueue<[u8; 80]>> = OnceCell::uninit();
impl log::Log for LockedLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut serial = self.serial.lock();
        writeln!(serial, "{:5}: {}", record.level(), record.args()).unwrap();
        if let Ok(queue) = LOG_QUE.try_get() {
            let mut buffer = [0u8; 80];
            let mut buffer_writer = BufferWriter::new(&mut buffer);
            match writeln!(buffer_writer, "{:5}: {}", record.level(), record.args()) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("err while writing log to buffer:{e}")
                }
            };

            if queue.push(buffer).is_err() {
                writeln!(serial, "WARN: log queue full; dropping log").unwrap();
            }

            WAKER.wake();
        } else {
            warn!("log queue uninitialized");
        }
    }

    fn flush(&self) {}
}

struct BufferWriter<'a> {
    buffer: &'a mut [u8],
    pos: usize,
}

impl<'a> BufferWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buffer: buf,
            pos: 0,
        }
    }
}

impl<'a> Write for BufferWriter<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let available = self.buffer.len().saturating_sub(self.pos);
        let to_copy = available.min(bytes.len());
        self.buffer[self.pos..self.pos + to_copy].copy_from_slice(&bytes[..to_copy]);
        self.pos += to_copy;
        Ok(())
    }
}
