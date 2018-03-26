use kernel::RsOsErr;
use kernel::scheduler::*;



#[derive(Default, Copy, Clone)]
pub struct Task {
    /// Task associated function
    pub func            : Option<extern "C" fn(arg1 : u32)>,
    /// Task function argument
    pub fn_arg          : u32,
    /// Offset in the statically allocated stack array
    pub stack_pointer   : u32,
    /// Stack size dedicated to this task, may vary between tasks...
    pub stack_size      : u32,
    /// Task priority, already incorporated in the task structure but not currently used
    pub priority        : u32,
    /// Status flags
    pub st_flags        : u32,
}

impl Task {
    pub fn new(
        fp : extern "C" fn(arg : u32),
        arg : u32,
        stack_size : u32,
        prio : u32,
    ) -> Task {
        Task {
            func : Some(fp),
            fn_arg : arg,
            stack_pointer : 0,
            stack_size,
            priority : prio,
            st_flags : 1,
        }
    }
}
