pub mod task;
pub mod scheduler;

pub const MAX_TASK_NUM : u32 = 2;
pub const MIN_STACK_SIZE_PER_TASK : u32 = 128;

pub enum RsOsErr {
    NoEnoughStack,
    ToMuchTasks,
}
