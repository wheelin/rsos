use kernel::task::*;
use kernel::*;
use cortex_m::peripheral::{SYST, NVIC, SCB};
use cortex_m_rt;
use core::mem;

const MAIN_RETURN   : u32 = 0xFFFFFFF9;
const THREAD_RETURN : u32 = 0xFFFFFFFD;

pub const READY_MSK     : u32 = 0x00000001;
pub const BLOCKED_MSK   : u32 = 0x00000002;
pub const IN_USE_FLAG   : u32 = 0x00000004;



static mut task_table : [Task; MAX_TASK_NUM as usize] = [Task {
    func : None,
    fn_arg : 0,
    stack_pointer : 0,
    stack_size : 0,
    priority : 0,
    st_flags : 0,
}; MAX_TASK_NUM as usize];

static mut stack : *mut u32 = 0 as *mut u32;
static mut current_task : u32 = 0;
static mut tasks_stack_storage_array : &mut [u32] = &mut [0];
static mut tasks_cntr : u32 = 0;
static mut sched_started : bool = false;

enum Syscall {
    Start,
    ContextSwitching,
}
static mut syscall_num : Syscall = Syscall::Start;

#[repr(C)]
struct HwStackFrame {
    r0  : u32,
    r1  : u32,
    r2  : u32,
    r3  : u32,
    r12 : u32,
    lr  : u32,
    pc  : u32,
    psr : u32,
}

#[repr(C)]
struct SwStackFrame {
    r4  : u32,
    r5  : u32,
    r6  : u32,
    r7  : u32,
    r8  : u32,
    r9  : u32,
    r10 : u32,
    r11 : u32,
}

pub fn init(stack_storage : &'static mut [u32], tick : u32) -> Result<(), RsOsErr>{
    if stack_storage.len() < (MAX_TASK_NUM * MIN_STACK_SIZE_PER_TASK) as usize {
        return Err(RsOsErr::NoEnoughStack);
    }

    unsafe {
        tasks_stack_storage_array = stack_storage;

        (*SYST::ptr()).rvr.write(tick & 0x00FFFFFF);
    }

    Ok(())
}

pub fn add_task(mut t : Task) -> Result<u32, RsOsErr> {
    unsafe {
        if sched_started {
            enter_critical_section();
        }

        if tasks_cntr >= MAX_TASK_NUM {
            return Err(RsOsErr::ToMuchTasks);
        }

        if tasks_cntr != 0 {
            let last_task = task_table[(tasks_cntr - 1) as usize];
            t.stack_pointer = last_task.stack_pointer + last_task.stack_size + t.stack_size;
        } else {
            t.stack_pointer = tasks_stack_storage_array.as_ptr() as u32 + t.stack_size;
        }

        task_table[tasks_cntr as usize] = t;
        tasks_cntr += 1;

        t.stack_pointer -= mem::size_of::<HwStackFrame>() as u32;
        let mut hw_frame_ptr = t.stack_pointer as *mut HwStackFrame;
        (*hw_frame_ptr).r0 = t.fn_arg;
        (*hw_frame_ptr).r1 = 1;
        (*hw_frame_ptr).r2 = 2;
        (*hw_frame_ptr).r3 = 3;
        (*hw_frame_ptr).r12 = 12;
        (*hw_frame_ptr).lr = 0; // reset when exiting task, further investigations needed.
        (*hw_frame_ptr).pc = t.func.unwrap() as *const u32 as u32;
        (*hw_frame_ptr).psr = 0x21000000;
        t.st_flags = READY_MSK | IN_USE_FLAG;
        t.stack_pointer -= mem::size_of::<SwStackFrame>() as u32;

        if sched_started {
            leave_critical_section();
        }
        Ok(tasks_cntr - 1)
    }
}

pub fn start() {
    unsafe {
        syscall_num = Syscall::Start;
    }
    fire_pendsv();
}

pub fn stop() {
    unsafe {
        (*SYST::ptr()).csr.modify(|v| v & !3);
        sched_started = false;
    }
}

pub fn enter_critical_section() {
    unsafe {
        (*SYST::ptr()).csr.modify(|v| v | 2);
    }
}

pub fn leave_critical_section() {
    unsafe {
        (*SYST::ptr()).csr.modify(|v| v & 0xFFFFFFFD);
    }
}

pub fn fire_pendsv() {
    unsafe {
        (*SCB::ptr()).icsr.modify(|v| v | (1 << 28));
    }
}

pub fn read_stack_ptr() -> u32 {
    let mut res: u32 = 0;
    unsafe {
        let mut ptr : *mut u32 = 0 as *mut u32;
        asm!("MRS $0, msp\n\t"
            : "=r"(ptr)
        );
        res = ptr as u32;
    }
    res
}

pub fn read_thread_stack_ptr() -> u32 {
    let mut res: u32 = 0;
    unsafe {
        let mut ptr : *mut u32 = 0 as *mut u32;
        asm!("MRS $0, psp\n\t"
            : "=r"(ptr)
        );
        res = ptr as u32;
    }
    res
}

pub fn write_thread_stack_ptr(ptr : u32) {
    unsafe{
        asm!("MSR psp, $0\n\t" : : "r" (ptr as *const u32))
    }
}

pub fn save_context() {
    unsafe {
        let mut scratch : u32 = 0;
        asm!("MRS $0, psp\n\t
              STMDB $0!, {r4-r11}\n\t
              MSR psp, $0\n\t"  : "=r" (scratch)
        );
    }
}

pub fn load_context() {
    unsafe {
        let mut scratch : u32 = 0;
        asm!("MRS $0, psp\n\t
              LDMFD $0!, {r4-r11}\n\t
              MSR psp, $0\n\t"  : "=r" (scratch)
        );
    }
}

pub fn schedule_next_task() {
    unsafe {
        loop {
            current_task += 1;
            if current_task == MAX_TASK_NUM {
                current_task = 0;
            }
            let mut nt = task_table[current_task as usize];
            if nt.st_flags & READY_MSK != 0 {
                return;
            }
        }
    }
}


pub fn systick_handler() {
    unsafe {
        if (*SYST::ptr()).csr.read() & 0x00000002 != 0 {
            schedule_next_task();
            syscall_num = Syscall::ContextSwitching;
            fire_pendsv();
        }
    }
}

pub fn pensv_handler() {

    unsafe {
        match syscall_num {
            Syscall::Start => {
                if task_table[0].st_flags == 0 {
                    return;
                } else {
                    write_thread_stack_ptr(task_table[0].stack_pointer);
                    load_context();
                    (*SYST::ptr()).csr.modify(|v| v | 3);
                    asm!("mov lr, #0xFFFFFFFD");
                    return;
                }
            },
            Syscall::ContextSwitching => {
                let mut old_sp : u32 = if current_task == 0 {
                    &(task_table[(MAX_TASK_NUM - 1) as usize].stack_pointer) as u32
                } else {
                    &(task_table[current_task as usize].stack_pointer) as u32
                };
                let cur_sp = &task_table[current_task as usize].stack_pointer as u32;

                // disable interrupt
                asm!("cpsid     i\n\t
                      mrs       r0, psp\n\t
                      subs      r0, #16\n\t
                      stmia     r0!,{r4-r7}\n\t
                      mov       r4, r8\n\t
                      mov       r5, r9\n\t
                      mov       r6, r10\n\t
                      mov       r7, r11\n\t
                      subs      r0, #32\n\t
                      stmia     r0!,{r4-r7}\n\t
                      subs      r0,#16\n\t

                      ldr       r2,[$0]\n\t
                      ldr       r1,[r2]\n\t
                      str       r0,[r1]\n\t

                      ldr       r2,[$1]\n\t
                      ldr       r1,[r2]\n\t
                      ldr       r0,[r1]\n\t

                      ldmia	    r0!,{r4-r7}\n\t
                      mov	    r8, r4\n\t
                      mov	    r9, r5\n\t
                      mov	    r10, r6\n\t
                      mov	    r11, r7\n\t
                      ldmia	    r0!,{r4-r7}\n\t
                      msr	    psp, r0\n\t

                      ldr       r0, =0xFFFFFFFD\n\t
                      cpsie     i\n\t
                      bx        r0\n\t
                      "
                    :
                    :"r"(old_sp),"r"(cur_sp)
                );

            },
        }
    }
}
