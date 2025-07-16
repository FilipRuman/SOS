#[cfg(not(test))]
use core::panic::PanicInfo;

use crate::hlt_loop;
use log::*;
//  run on panic
#[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}\n", info);
    error!("{}", info);

    hlt_loop();
}

// our panic handler in test mode

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::qemu::{QemuExitCode, exit_qemu};

    error!("[failed]\n");
    error!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}
