pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;

pub const APP_BASE_ADDRESS: usize = 0x4020_0000;
pub const APP_SIZE_LIMIT: usize = 0x20000;