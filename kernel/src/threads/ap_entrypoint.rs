use core::sync::atomic::AtomicBool;

use x86::task::tr;

pub static test: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn ap_entrypoint() -> ! {
    // Setup per-core GDT/IDT/etc.
    log::info!("AP core online!");
    test.store(true, core::sync::atomic::Ordering::Relaxed);
    loop {
        x86_64::instructions::hlt();
    }
}
