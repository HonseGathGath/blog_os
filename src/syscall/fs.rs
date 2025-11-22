use crate::println;

/// Open a file
/// Returns file descriptor or negative error code
pub fn sys_open(path: usize, flags: i32, mode: u32) -> i64 {
    // TODO: implement actual file opening
    println!("sys_open: path={:#x}, flags={}, mode={}", path, flags, mode);
    -1 // ENOSYS - not implemented yet
}

/// Close a file descriptor
/// Returns 0 on success or negative error code
pub fn sys_close(fd: i32) -> i64 {
    // TODO: implement actual file closing
    println!("sys_close: fd={}", fd);
    -1 // ENOSYS - not implemented yet
}
