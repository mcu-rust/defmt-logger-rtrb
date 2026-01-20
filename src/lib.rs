#![doc = include_str!("../README.md")]
#![no_std]

use core::{
    cell::{Cell, UnsafeCell},
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};
use rtrb::{Consumer, CopyToUninit, Producer, RingBuffer, chunks::ChunkError};

/// Initialize global logger
///
/// - size: size of buffer in logger
pub fn init(size: usize) -> Consumer<u8> {
    let (p, c) = RingBuffer::new(size);
    LOGGER.init_buf(p);
    c
}

static LOGGER: LoggerRtrb = LoggerRtrb::new();

struct LoggerRtrb {
    /// A boolean lock
    ///
    /// Is `true` when `acquire` has been called and we have exclusive access to
    /// the rest of this structure.
    taken: AtomicBool,
    /// We need to remember this to exit a critical section
    cs_restore: Cell<critical_section::RestoreState>,
    /// A defmt::Encoder for encoding frames
    encoder: UnsafeCell<defmt::Encoder>,
    buf: UnsafeCell<MaybeUninit<Producer<u8>>>,
}

impl LoggerRtrb {
    /// Create a new logger based on rtrb
    const fn new() -> LoggerRtrb {
        LoggerRtrb {
            taken: AtomicBool::new(false),
            cs_restore: Cell::new(critical_section::RestoreState::invalid()),
            encoder: UnsafeCell::new(defmt::Encoder::new()),
            buf: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    fn init_buf(&self, buf: Producer<u8>) {
        unsafe { &mut *self.buf.get() }.write(buf);
    }

    /// Acquire the defmt encoder.
    #[inline]
    fn acquire(&self) {
        // safety: Must be paired with corresponding call to release(), see below
        let restore = unsafe { critical_section::acquire() };

        // NB: You can re-enter critical sections but we need to make sure
        // no-one does that.
        if self.taken.load(Ordering::Relaxed) {
            panic!("logger taken reentrantly")
        }

        // no need for CAS because we are in a critical section
        self.taken.store(true, Ordering::Relaxed);

        // safety: accessing the cell is OK because we have acquired a critical
        // section.
        unsafe {
            self.cs_restore.set(restore);
            let encoder = &mut *self.encoder.get();
            encoder.start_frame(|b| do_write(&self.buf, b));
        }
    }

    /// Write bytes to the defmt encoder.
    ///
    /// # Safety
    ///
    /// Do not call unless you have called `acquire`.
    #[inline]
    unsafe fn write(&self, bytes: &[u8]) {
        // safety: accessing the cell is OK because we have acquired a critical
        // section.
        let encoder = unsafe { &mut *self.encoder.get() };
        encoder.write(bytes, |b| do_write(&self.buf, b));
    }

    /// Flush the encoder
    ///
    /// # Safety
    ///
    /// Do not call unless you have called `acquire`.
    #[inline]
    unsafe fn flush(&self) {}

    /// Release the defmt encoder.
    ///
    /// # Safety
    ///
    /// Do not call unless you have called `acquire`. This will release
    /// your lock - do not call `flush` and `write` until you have done another
    /// `acquire`.
    #[inline]
    unsafe fn release(&self) {
        // safety: accessing the cell is OK because we have acquired a critical
        // section.
        unsafe {
            let encoder = &mut *self.encoder.get();
            encoder.end_frame(|b| do_write(&self.buf, b));
            self.taken.store(false, Ordering::Relaxed);
            // paired with exactly one acquire call
            critical_section::release(self.cs_restore.get());
        }
    }
}

unsafe impl Sync for LoggerRtrb {}

fn do_write(buf: &UnsafeCell<MaybeUninit<Producer<u8>>>, bytes: &[u8]) {
    use ChunkError::TooFewSlots;
    let buf = unsafe { (&mut *buf.get()).assume_init_mut() };
    let mut chunk = match buf.write_chunk_uninit(bytes.len()) {
        Ok(chunk) => chunk,
        Err(TooFewSlots(0)) => return,
        Err(TooFewSlots(n)) => match buf.write_chunk_uninit(n) {
            Ok(chunk) => chunk,
            _ => return,
        },
    };

    let end = chunk.len();
    let (first, second) = chunk.as_mut_slices();
    let mid = first.len();
    bytes[..mid].copy_to_uninit(first);
    bytes[mid..end].copy_to_uninit(second);
    unsafe { chunk.commit_all() };
}

#[defmt::global_logger]
struct Logger;

unsafe impl defmt::Logger for Logger {
    fn acquire() {
        LOGGER.acquire();
    }

    unsafe fn write(bytes: &[u8]) {
        unsafe {
            LOGGER.write(bytes);
        }
    }

    unsafe fn flush() {
        unsafe {
            LOGGER.flush();
        }
    }

    unsafe fn release() {
        unsafe {
            LOGGER.release();
        }
    }
}
