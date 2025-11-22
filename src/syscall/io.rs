use crate::println;

/// Read from file descriptor
/// Returns number of bytes read or negative error code
pub fn sys_read(fd: i32, buf: usize, count: usize) -> i64 {
    // TODO: implement actual file descriptor reading
    // For now, stub implementation
    println!("sys_read: fd={}, buf={:#x}, count={}", fd, buf, count);
    -1 // ENOSYS - not implemented yet
}

/// Write to file descriptor
/// Returns number of bytes written or negative error code
pub fn sys_write(fd: i32, buf: usize, count: usize) -> i64 {
    // TODO: implement actual file descriptor writing
    // For now, basic console output for fd 1 (stdout)
    if fd == 1 || fd == 2 {
        // stdout or stderr - write to console
        // Safety: we trust the user buffer for now, will validate later
        if buf != 0 && count > 0 {
            let slice = unsafe {
                core::slice::from_raw_parts(buf as *const u8, count)
            };
            
            if let Ok(s) = core::str::from_utf8(slice) {
                print!("{}", s);
                return count as i64;
            }
        }
    }
    
    println!("sys_write: fd={}, buf={:#x}, count={}", fd, buf, count);
    -1 // Error
}
