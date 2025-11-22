use crate::println;

/// Fork current process
/// Returns child PID to parent, 0 to child, or negative error code
pub fn sys_fork() -> i64 {
    // TODO: implement actual fork
    println!("sys_fork called");
    -1 // ENOSYS - not implemented yet
}

/// Execute a program
/// Does not return on success, returns negative error code on failure
pub fn sys_exec(path: usize, argv: usize) -> i64 {
    // TODO: implement actual exec
    println!("sys_exec: path={:#x}, argv={:#x}", path, argv);
    -1 // ENOSYS - not implemented yet
}

/// Exit current process
/// Does not return
pub fn sys_exit(code: i32) -> i64 {
    println!("sys_exit: code={}", code);
    // TODO: implement actual process exit
    // For now, just halt
    crate::hlt_loop();
}

/// Wait for child process
/// Returns child PID or negative error code
pub fn sys_wait(status: usize) -> i64 {
    // TODO: implement actual wait
    println!("sys_wait: status={:#x}", status);
    -1 // ENOSYS - not implemented yet
}

/// Get current process ID
pub fn sys_getpid() -> i64 {
    // TODO: return actual PID
    1 // Temporary: always return PID 1
}

/// Get parent process ID
pub fn sys_getppid() -> i64 {
    // TODO: return actual parent PID
    0 // Temporary: always return 0
}
