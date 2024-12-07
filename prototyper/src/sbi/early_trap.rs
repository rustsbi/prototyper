use core::arch::asm;

/// When you expected some insts will cause trap, use this.
/// If trap happend, a0 will set to 1, otherwise will be 0.
///
/// This function will change a0 and a1 and will NOT change them back.
#[naked]
#[repr(align(16))]
pub(crate) unsafe extern "C" fn expected_trap() {
    asm!(
        "add a0, zero, zero",
        "add a1, zero, zero",
        "csrr a1, mepc",
        "addi a1, a1, 4",
        "csrw mepc, a1",
        "addi a0, zero, 1",
        "mret",
        options(noreturn)
    )
}