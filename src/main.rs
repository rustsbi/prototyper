#![feature(naked_functions, asm_const)]
#![no_std]
#![no_main]

#[macro_use]
extern crate log;
#[macro_use]
mod macros;

mod board;
mod console;
mod dt;
mod dynamic;
mod fail;
mod reset;
mod trap;

use panic_halt as _;
use riscv::register::mstatus;

extern "C" fn main(hart_id: usize, opaque: usize, nonstandard_a2: usize) -> usize {
    let _ = (hart_id, opaque);
    console::init();

    info!("RustSBI version {}", rustsbi::VERSION);
    rustsbi::LOGO.lines().for_each(|line| info!("{}", line));
    info!("Initializing RustSBI machine-mode environment.");

    let dtb = dt::parse_device_tree(opaque).unwrap_or_else(fail::device_tree_format);
    let dtb = dtb.share();
    let tree = serde_device_tree::from_raw_mut(&dtb).unwrap_or_else(fail::device_tree_deserialize);
    if let Some(model) = tree.model {
        info!("Model: {}", model.iter().next().unwrap_or("<unspecified>"));
    }
    info!("Chosen stdout item: {}", tree.chosen.stdout_path.iter().next().unwrap_or("<unspecified>"));
    // TODO handle unspecified by parsing into &'a str

    let info = dynamic::read_paddr(nonstandard_a2).unwrap_or_else(fail::no_dynamic_info_available);

    let (mpp, next_addr) = dynamic::mpp_next_addr(&info).unwrap_or_else(fail::invalid_dynamic_data);

    info!("Redirecting harts to 0x{:x} in {:?} mode.", next_addr, mpp);

    trap::init();
    unsafe { mstatus::set_mpp(mpp) };
    next_addr
}

const LEN_STACK_PER_HART: usize = 16 * 1024;
pub(crate) const NUM_HART_MAX: usize = 8;
const LEN_STACK: usize = LEN_STACK_PER_HART * NUM_HART_MAX;

// TODO contribute `Stack` struct into the crate `riscv`
#[repr(C, align(128))]
struct Stack<const N: usize>([u8; N]);

#[link_section = ".bss.uninit"]
static STACK: Stack<LEN_STACK> = Stack([0; LEN_STACK]);

#[naked]
#[link_section = ".text.entry"]
#[export_name = "_start"]
unsafe extern "C" fn start() -> ! {
    core::arch::asm!(
        // 1. Turn off interrupt
        "   csrw    mie, zero",
        // 2. Initialize programming langauge runtime
        // only clear bss if hartid matches preferred boot hart id
        "   csrr    t0, mhartid",
        "   ld      t1, 0(a2)",
        "   li      t2, {magic}",
        "   bne     t1, t2, 3f",
        "   ld      t2, 40(a2)",
        "   bne     t0, t2, 2f",
        "   j       4f",
        "3:",
        "   j       3b", // TODO multi hart preempt for runtime init
        "4:",
        // clear bss segment
        "   la      t0, sbss
            la      t1, ebss
        1:  bgeu    t0, t1, 2f
            sd      zero, 0(t0)
            addi    t0, t0, 8
            j       1b",
        // prepare data segment
        "   la      t3, sidata
            la      t4, sdata
            la      t5, edata
        1:  bgeu    t4, t5, 2f
            ld      t6, 0(t3)
            sd      t6, 0(t4)
            addi    t3, t3, 8
            addi    t4, t4, 8
            j       1b",
        "2: ",
        // TODO wait before boot-hart initializes runtime
        // 3. Prepare stack for each hart
        "   la      sp, {stack}",
        "   li      t0, {stack_size_per_hart}",
        "   csrr    t1, mhartid",
        "   addi    t1, t1, 1",
        "1: ",
        "   add     sp, sp, t0",
        "   addi    t1, t1, -1",
        "   bnez    t1, 1b",
        // 4. Run Rust main function
        "   call    {main}",
        // 5. Jump to following boot sequences
        "   csrw    mepc, a0",
        "   mret",
        magic = const dynamic::MAGIC,
        stack_size_per_hart = const LEN_STACK_PER_HART,
        stack = sym STACK,
        main = sym main,
        options(noreturn)
    )
}
