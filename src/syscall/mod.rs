use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::println;

pub mod numbers;
pub mod process;
pub mod fs;
pub mod io;

use numbers::SyscallNumber;

/// Syscall handler - called from interrupt handler
/// Arguments are passed in registers following System V ABI:
/// rax = syscall number
/// rdi = arg1, rsi = arg2, rdx = arg3, r10 = arg4, r8 = arg5, r9 = arg6
pub fn syscall_handler(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> i64 {
    let syscall = SyscallNumber::from_u64(syscall_num);
    
    match syscall {
        Some(SyscallNumber::Read) => io::sys_read(arg1 as i32, arg2 as usize, arg3 as usize),
        Some(SyscallNumber::Write) => io::sys_write(arg1 as i32, arg2 as usize, arg3 as usize),
        Some(SyscallNumber::Open) => fs::sys_open(arg1 as usize, arg2 as i32, arg3 as u32),
        Some(SyscallNumber::Close) => fs::sys_close(arg1 as i32),
        Some(SyscallNumber::Fork) => process::sys_fork(),
        Some(SyscallNumber::Exec) => process::sys_exec(arg1 as usize, arg2 as usize),
        Some(SyscallNumber::Exit) => process::sys_exit(arg1 as i32),
        Some(SyscallNumber::Wait) => process::sys_wait(arg1 as usize),
        Some(SyscallNumber::GetPid) => process::sys_getpid(),
        Some(SyscallNumber::GetPPid) => process::sys_getppid(),
        None => {
            println!("Unknown syscall: {}", syscall_num);
            -1 // ENOSYS
        }
    }
}

/// Initialize syscall support
pub fn init() {
    // We'll use int 0x80 for now (simpler than syscall instruction)
    // The syscall instruction requires MSR setup which is more complex
    println!("Syscall interface initialized");
}
