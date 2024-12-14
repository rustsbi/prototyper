use core::arch::asm;
use core::sync::atomic::{AtomicBool, Ordering};
use riscv::register::mstatus;

use super::{BootHart, BootInfo};

/// Determine whether the current hart is boot hart.
///
/// Return true if the current hart is boot hart.
pub fn is_boot_hart(_nonstandard_a2: usize) -> bool {
    static GENESIS: AtomicBool = AtomicBool::new(true);
    GENESIS.swap(false, Ordering::AcqRel)
}

pub fn get_boot_info(_nonstandard_a2: usize) -> BootInfo {
    BootInfo {
        next_address: get_image_address(),
        mpp: mstatus::MPP::Supervisor,
    }
}

#[naked]
#[link_section = ".payload"]
pub unsafe extern "C" fn payload_image() {
    asm!(
        concat!(".incbin \"", env!("PROTOTYPER_PAYLOAD_PATH"), "\""),
        options(noreturn)
    );
}

#[inline]
fn get_image_address() -> usize {
    payload_image as usize
}
