use x86::apic::{self, ApicControl, ApicId};

use crate::interrupts;

pub mod ap_entrypoint;
mod trampoline;
pub fn init(
    ap_count: u8,
    level_4_table_phys_address: u64,
    gdt_base_phys_address: u64,
    gdt_size: usize,
) {
    trampoline::init(ap_count as usize);

    // 0-> bootstrap processor
    // init all aps
    for ap_index in 1..ap_count + 1 {
        trampoline::setup_trampoline_data(
            ap_index,
            level_4_table_phys_address,
            gdt_base_phys_address,
            gdt_size,
        );

        // unsafe { interrupts::apic::xapic().ipi_init(ApicId::XApic(ap_index)) };

        unsafe { interrupts::apic::xapic().ipi_init(ApicId::XApic(ap_index)) };
        unsafe {
            interrupts::apic::xapic()
                .ipi_startup(ApicId::XApic(ap_index), trampoline::TRAMPOLINE_ADDR as u8)
        };
    }
    log::debug!("initialized ap threads");
}
