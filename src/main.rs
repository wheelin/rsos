#![feature(used)]
#![no_std]
#![feature(asm)]

extern crate cortex_m;
#[macro_use(exception)]
extern crate cortex_m_rt;
extern crate cortex_m_semihosting;

extern crate vcell;

use core::fmt::Write;

use cortex_m::asm;
use cortex_m_semihosting::hio;

mod kernel;
use kernel::scheduler;
use kernel::task;

extern "C" fn task1(arg : u32) {
    loop {
        asm::nop();
    }
}

extern "C" fn task2(arg : u32) {
    loop {
        asm::nop();
    }
}

static mut stack_storage : [u32; 256] = [0; 256];

fn main() {
    let mut stdout = hio::hstdout().unwrap();
    unsafe {
        if scheduler::init(&mut stack_storage, 5000).is_err() {
            writeln!(stdout, "Problem while initializing scheduler");
        } else {
            scheduler::add_task(task::Task::new(task1, 0, 128, 0));
            scheduler::add_task(task::Task::new(task2, 0, 128, 0));
            scheduler::start();
        }
    }
}

exception!(SYS_TICK, kernel::scheduler::systick_handler);
exception!(PENDSV, kernel::scheduler::pensv_handler);

// As we are not using interrupts, we just register a dummy catch all handler
#[link_section = ".vector_table.interrupts"]
#[used]
static INTERRUPTS: [extern "C" fn(); 240] = [default_handler; 240];

extern "C" fn default_handler() {
    asm::bkpt();
}
