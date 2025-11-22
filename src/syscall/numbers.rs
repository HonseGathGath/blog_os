/// Syscall numbers - following Linux convention for familiarity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallNumber {
    Read = 0,
    Write = 1,
    Open = 2,
    Close = 3,
    Fork = 57,
    Exec = 59,
    Exit = 60,
    Wait = 61,
    GetPid = 39,
    GetPPid = 110,
}

impl SyscallNumber {
    pub fn from_u64(num: u64) -> Option<Self> {
        match num {
            0 => Some(SyscallNumber::Read),
            1 => Some(SyscallNumber::Write),
            2 => Some(SyscallNumber::Open),
            3 => Some(SyscallNumber::Close),
            57 => Some(SyscallNumber::Fork),
            59 => Some(SyscallNumber::Exec),
            60 => Some(SyscallNumber::Exit),
            61 => Some(SyscallNumber::Wait),
            39 => Some(SyscallNumber::GetPid),
            110 => Some(SyscallNumber::GetPPid),
            _ => None,
        }
    }
}
