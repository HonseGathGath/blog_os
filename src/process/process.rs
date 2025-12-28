//! Process Control Block (PCB) implementation with complete register context
//! Integrates with your allocator.rs and memory.rs

use super::elf_loader::{LoadedProgram, MemoryRegion};
use alloc::vec::Vec;
use core::arch::asm;
use x86_64::{structures::paging::Size4KiB, VirtAddr};
/// Process identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProcessId(u64);

impl ProcessId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Process states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    New,
    Ready,
    Running,
    Blocked,
    Sleeping,
    Zombie,
    Dead,
}

/// Process priority for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProcessPriority {
    Idle = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    RealTime = 4,
}

/// Complete CPU context for process switching
/// Based on x86_64 System V ABI register preservation rules
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessContext {
    // Callee-saved registers (must be preserved)
    pub rbx: u64,
    pub rsp: u64,
    pub rbp: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    // Caller-saved registers (can be clobbered, saved for context switch)
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,

    // Instruction pointer and flags
    pub rip: u64,
    pub rflags: u64,

    // Segment registers
    pub cs: u64,
    pub ss: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,

    // Control registers (for kernel threads)
    pub cr3: u64,
    pub cr2: u64, // For page fault handling

    // Floating point/SSE state (if needed)
    pub mxcsr: u32,
    pub fpu_cw: u16,
}

impl ProcessContext {
    /// Create initial context for new user process
    pub fn new_user(entry_point: u64, stack_pointer: u64) -> Self {
        ProcessContext {
            // Callee-saved
            rbx: 0,
            rsp: stack_pointer,
            rbp: stack_pointer,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,

            // Caller-saved
            rax: 0,
            rcx: 0,
            rdx: 0,
            rdi: 0,
            rsi: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,

            // Control
            rip: entry_point,
            rflags: 0x202, // IF=1 (interrupts enabled), bit 1 always 1

            // Segment registers (user mode values)
            cs: 0x1b, // User code segment (ring 3)
            ss: 0x23, // User stack segment (ring 3)
            ds: 0x23,
            es: 0x23,
            fs: 0x23,
            gs: 0x23,

            // Control registers
            cr3: 0, // Will be set when page table is loaded
            cr2: 0,

            // FPU/SSE
            mxcsr: 0x1F80,  // Default MXCSR value
            fpu_cw: 0x037F, // Default FPU control word
        }
    }

    /// Create kernel context (for kernel threads)
    pub fn new_kernel(entry_point: u64, stack_pointer: u64) -> Self {
        let mut ctx = Self::new_user(entry_point, stack_pointer);

        // Kernel mode segments (ring 0)
        ctx.cs = 0x08; // Kernel code segment
        ctx.ss = 0x10; // Kernel stack segment
        ctx.ds = 0x10;
        ctx.es = 0x10;
        ctx.fs = 0x10;
        ctx.gs = 0x10;

        ctx
    }

    /// Save complete CPU state
    pub unsafe fn save(&mut self) {
        unsafe {
            asm!(
                // Save callee-saved registers
                "mov [rdi + 0x00], rbx",
                "mov [rdi + 0x08], rsp",
                "mov [rdi + 0x10], rbp",
                "mov [rdi + 0x18], r12",
                "mov [rdi + 0x20], r13",
                "mov [rdi + 0x28], r14",
                "mov [rdi + 0x30], r15",

                // Save caller-saved registers
                "mov [rdi + 0x38], rax",
                "mov [rdi + 0x40], rcx",
                "mov [rdi + 0x48], rdx",
                "mov [rdi + 0x50], rdi",
                "mov [rdi + 0x58], rsi",
                "mov [rdi + 0x60], r8",
                "mov [rdi + 0x68], r9",
                "mov [rdi + 0x70], r10",
                "mov [rdi + 0x78], r11",

                // Save rflags
                "pushfq",
                "pop qword ptr [rdi + 0x88]",

                // Save segment registers
                "mov rax, cs",
                "mov [rdi + 0x90], rax",
                "mov rax, ss",
                "mov [rdi + 0x98], rax",
                "mov rax, ds",
                "mov [rdi + 0xA0], rax",
                "mov rax, es",
                "mov [rdi + 0xA8], rax",
                "mov rax, fs",
                "mov [rdi + 0xB0], rax",
                "mov rax, gs",
                "mov [rdi + 0xB8], rax",

                // Save control registers
                "mov rax, cr3",
                "mov [rdi + 0xC0], rax",
                "mov rax, cr2",
                "mov [rdi + 0xC8], rax",

                // Save FPU/SSE state
                "stmxcsr [rdi + 0xD0]",
                "fnstcw [rdi + 0xD4]",

                // Note: rip is saved by the caller (call instruction)
                in("rdi") self,
                options(preserves_flags, nostack)
            );
        }
    }

    /// Restore complete CPU state
    pub unsafe fn restore(&self) {
        unsafe {
            asm!(
                // Restore segment registers first
                "mov rax, [rdi + 0xB8]",
                "mov gs, ax",
                "mov rax, [rdi + 0xB0]",
                "mov fs, ax",
                "mov rax, [rdi + 0xA8]",
                "mov es, ax",
                "mov rax, [rdi + 0xA0]",
                "mov ds, ax",
                "mov rax, [rdi + 0x98]",
                "mov ss, ax",

                // Restore stack pointer
                "mov rsp, [rdi + 0x08]",

                // Restore control registers
                "mov rax, [rdi + 0xC0]",
                "mov cr3, rax",
                "mov rax, [rdi + 0xC8]",
                "mov cr2, rax",

                // Restore callee-saved registers
                "mov r15, [rdi + 0x30]",
                "mov r14, [rdi + 0x28]",
                "mov r13, [rdi + 0x20]",
                "mov r12, [rdi + 0x18]",
                "mov rbp, [rdi + 0x10]",
                "mov rbx, [rdi + 0x00]",

                // Restore caller-saved registers
                "mov r11, [rdi + 0x78]",
                "mov r10, [rdi + 0x70]",
                "mov r9, [rdi + 0x68]",
                "mov r8, [rdi + 0x60]",
                "mov rsi, [rdi + 0x58]",
                "mov rcx, [rdi + 0x40]",
                "mov rdx, [rdi + 0x48]",
                "mov rax, [rdi + 0x38]",

                // Restore rflags
                "push qword ptr [rdi + 0x88]",
                "popfq",

                // Restore FPU/SSE state
                "ldmxcsr [rdi + 0xD0]",
                "fldcw [rdi + 0xD4]",

                // Prepare rdi for restore (must be last)
                "mov rdi, [rdi + 0x50]",

                // Note: rip is restored by the ret instruction
                in("rdi") self,
                options(nostack)
            );
        }
    }

    /// Save minimal context (for syscall entry/exit)
    pub unsafe fn save_minimal(&mut self) {
        unsafe {
            asm!(
                // Save only what's needed for syscall
                "mov [rdi + 0x08], rsp",     // rsp
                "mov [rdi + 0x10], rbp",     // rbp
                "mov [rdi + 0x38], rax",     // rax (syscall return)
                "mov [rdi + 0x40], rcx",     // rcx (syscall return addr)
                "mov [rdi + 0x78], r11",     // r11 (syscall flags)

                // Save rip from return address
                "pop qword ptr [rdi + 0x80]", // rip

                // Save rflags
                "pushfq",
                "pop qword ptr [rdi + 0x88]",
                in("rdi") self,
                options(nostack)
            );
        }
    }

    /// Restore minimal context (for syscall return)
    pub unsafe fn restore_minimal(&self) {
        unsafe {
            asm!(
                // Restore minimal state for sysret
                "mov rsp, [rdi + 0x08]",
                "mov rbp, [rdi + 0x10]",

                // For sysret: rcx = rip, r11 = rflags
                "mov rcx, [rdi + 0x80]",
                "mov r11, [rdi + 0x88]",

                // Restore rax (syscall return value)
                "mov rax, [rdi + 0x38]",

                // Jump to sysret instruction (caller handles this)
                in("rdi") self,
                options(nostack)
            );
        }
    }

    /// Check if this is a user mode context
    pub fn is_user_mode(&self) -> bool {
        // Check if CS segment selector indicates ring 3
        (self.cs & 0x03) == 0x03
    }
}

/// Process statistics
#[derive(Debug, Clone)]
pub struct ProcessStats {
    pub creation_time: u64,
    pub cpu_time: u64,
    pub user_time: u64,
    pub kernel_time: u64,
    pub page_faults: u32,
    pub context_switches: u32,
    pub syscalls_made: u32,
    pub memory_usage: u64,
    pub heap_usage: u64,
    pub last_context_save: Option<u64>,
}

/// Process Control Block (PCB)
pub struct Process {
    pub pid: ProcessId,
    pub state: ProcessState,
    pub context: ProcessContext,
    pub priority: ProcessPriority,
    pub entry_point: VirtAddr,
    pub stack_pointer: VirtAddr,
    pub exit_code: Option<u32>,
    pub parent_pid: Option<ProcessId>,
    pub children: Vec<ProcessId>,
    pub pgid: ProcessId,
    pub memory_regions: Vec<MemoryRegion>,
    pub name: alloc::string::String,
    pub stats: ProcessStats,
    pub signal_mask: u64,
    pub pending_signals: u64,
    pub working_dir: alloc::string::String,
    pub environment: Vec<(alloc::string::String, alloc::string::String)>,
    pub arguments: Vec<alloc::string::String>,
    pub kernel_stack: Option<VirtAddr>,
    pub heap_break: VirtAddr,
    pub heap_start: VirtAddr,
    pub heap_end: VirtAddr,
    pub page_table_base: Option<u64>, // CR3 value for this process
    pub timeslice: u32,               // Remaining time slice
    pub wakeup_time: Option<u64>,     // For sleeping processes
}

/// Process ID counter
static NEXT_PID: spin::Mutex<u64> = spin::Mutex::new(1);

impl Process {
    /// Create a new process from a loaded ELF program
    pub fn new(
        loaded_program: LoadedProgram,
        name: &str,
        arguments: Vec<alloc::string::String>,
        environment: Vec<(alloc::string::String, alloc::string::String)>,
    ) -> Self {
        let pid = allocate_pid();

        // Find heap region
        let (heap_start, heap_end) = loaded_program
            .regions
            .iter()
            .find(|r| r.is_heap)
            .map(|r| (r.start, r.end))
            .unwrap_or((VirtAddr::new(0), VirtAddr::new(0)));

        Process {
            pid: ProcessId::new(pid),
            state: ProcessState::New,
            context: ProcessContext::new_user(
                loaded_program.entry_point,
                loaded_program.stack_top.as_u64(),
            ),
            priority: ProcessPriority::Normal,
            entry_point: VirtAddr::new(loaded_program.entry_point),
            stack_pointer: loaded_program.stack_top,
            exit_code: None,
            parent_pid: None,
            children: Vec::new(),
            pgid: ProcessId::new(pid),
            memory_regions: loaded_program.regions,
            name: alloc::string::String::from(name),
            stats: ProcessStats {
                creation_time: get_current_time(),
                cpu_time: 0,
                user_time: 0,
                kernel_time: 0,
                page_faults: 0,
                context_switches: 0,
                syscalls_made: 0,
                memory_usage: loaded_program.total_memory,
                heap_usage: 0,
                last_context_save: None,
            },
            signal_mask: 0,
            pending_signals: 0,
            working_dir: alloc::string::String::from("/"),
            environment,
            arguments,
            kernel_stack: None,
            heap_break: heap_start,
            heap_start,
            heap_end,
            page_table_base: None,
            timeslice: 100, // Default timeslice (e.g., 100ms)
            wakeup_time: None,
        }
    }

    pub fn new_kernel_thread(entry_point: VirtAddr, stack_top: VirtAddr, name: &str) -> Self {
        let pid = allocate_pid();

        Process {
            pid: ProcessId::new(pid),
            state: ProcessState::New,

            // CRITICAL: Kernel mode context (ring 0)
            context: ProcessContext::new_kernel(entry_point.as_u64(), stack_top.as_u64()),

            priority: ProcessPriority::Normal,
            entry_point,
            stack_pointer: stack_top,
            exit_code: None,
            parent_pid: None,
            children: Vec::new(),
            pgid: ProcessId::new(pid),

            // Kernel threads have NO user memory regions
            memory_regions: Vec::new(),

            name: alloc::string::String::from(name),

            stats: ProcessStats {
                creation_time: get_current_time(),
                cpu_time: 0,
                user_time: 0,
                kernel_time: 0,
                page_faults: 0,
                context_switches: 0,
                syscalls_made: 0,
                memory_usage: 0, // No user memory
                heap_usage: 0,
                last_context_save: None,
            },

            signal_mask: 0,
            pending_signals: 0,
            working_dir: alloc::string::String::from("/"),

            // Kernel threads typically don't have environment/arguments
            environment: Vec::new(),
            arguments: Vec::new(),

            // Kernel thread uses this as its kernel stack
            kernel_stack: Some(stack_top),

            // No user heap for kernel threads
            heap_break: VirtAddr::new(0),
            heap_start: VirtAddr::new(0),
            heap_end: VirtAddr::new(0),

            // Kernel threads share kernel page tables (maybe None or kernel CR3)
            page_table_base: None,

            timeslice: 100,
            wakeup_time: None,
        }
    }

    /// Switch to this process (context switch)
    pub unsafe fn switch_to(&mut self, from: &mut Process) {
        // Save old context
        from.context.save();
        from.stats.context_switches += 1;
        from.stats.last_context_save = Some(get_current_time());

        // Update process states
        from.state = ProcessState::Ready;
        self.state = ProcessState::Running;

        // Load page table if different
        if let Some(cr3) = self.page_table_base {
            if from.page_table_base != Some(cr3) {
                asm!("mov cr3, {}", in(reg) cr3, options(nostack, preserves_flags));
            }
        }

        // Restore new context
        self.context.restore();

        // Note: Execution continues in the new process
    }

    /// Handle syscall entry
    pub unsafe fn enter_syscall(&mut self) {
        // Save minimal context for syscall
        self.context.save_minimal();

        // Switch to kernel stack if in user mode
        if self.context.is_user_mode() && self.kernel_stack.is_some() {
            self.context.rsp = self.kernel_stack.unwrap().as_u64();
        }

        // Update segment registers for kernel mode
        self.context.cs = 0x08; // Kernel code segment
        self.context.ss = 0x10; // Kernel stack segment
    }

    pub fn is_runnable(&self) -> bool {
        matches!(self.state, ProcessState::Ready | ProcessState::Running)
    }

    /// Check if process is alive
    pub fn is_alive(&self) -> bool {
        !matches!(self.state, ProcessState::Zombie | ProcessState::Dead)
    }

    /// Check if address is in process memory space
    pub fn is_valid_address(&self, addr: VirtAddr) -> bool {
        self.memory_regions
            .iter()
            .any(|r| r.start <= addr && addr < r.end)
    }

    /// Handle syscall exit
    pub unsafe fn exit_syscall(&mut self) {
        // Restore minimal context for sysret
        self.context.restore_minimal();
    }

    /// Save FPU/SSE state (for lazy FPU context switching)
    pub unsafe fn save_fpu_state(&mut self) {
        // Note: This would need an extended context structure
        // with space for XMM/YMM/ZMM registers
        self.context.mxcsr = get_mxcsr();
        self.context.fpu_cw = get_fpu_cw();
    }

    /// Restore FPU/SSE state
    pub unsafe fn restore_fpu_state(&self) {
        set_mxcsr(self.context.mxcsr);
        set_fpu_cw(self.context.fpu_cw);
    }
}

/// Get current time (placeholder - implement with your timer)
fn get_current_time() -> u64 {
    // You'll need to implement this with your timer subsystem
    // For now, return 0 or use x86_64's time stamp counter
    unsafe {
        let low: u32;
        let high: u32;
        asm!("rdtsc", out("eax") low, out("edx") high, options(nomem, nostack));
        ((high as u64) << 32) | (low as u64)
    }
}

/// Get MXCSR register
unsafe fn get_mxcsr() -> u32 {
    let mut mxcsr: u32 = 0;
    asm!("stmxcsr [{}]", in(reg) &mut mxcsr, options(nostack));
    mxcsr
}

/// Set MXCSR register
unsafe fn set_mxcsr(mxcsr: u32) {
    asm!("ldmxcsr [{}]", in(reg) &mxcsr, options(nostack));
}

/// Get FPU control word
unsafe fn get_fpu_cw() -> u16 {
    let mut cw: u16 = 0;
    asm!("fnstcw [{}]", in(reg) &mut cw, options(nostack));
    cw
}

/// Set FPU control word
unsafe fn set_fpu_cw(cw: u16) {
    asm!("fldcw [{}]", in(reg) &cw, options(nostack));
}

/// Allocate a new process ID
pub fn allocate_pid() -> u64 {
    let mut next_pid = NEXT_PID.lock();
    let pid = *next_pid;
    *next_pid += 1;
    pid
}
