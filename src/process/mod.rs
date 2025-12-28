// src/process/mod.rs
#![no_std]

use alloc::vec::Vec;

pub mod elf_loader;
pub mod loader;
pub mod manager;
pub mod process;

// Add process manager
//

pub mod tests;

// Re-export main types
pub use elf_loader::{LoadError, LoadedProgram, MemoryRegion};
pub use loader::{ElfError, SegmentInfo};
pub use manager::ProcessManager;
pub use process::{Process, ProcessContext, ProcessId, ProcessPriority, ProcessState};

#[cfg(test)]
pub use tests::run_process_tests;

/// Configuration for process creation
#[derive(Debug, Clone)]
pub struct ProcessConfig<'a> {
    pub name: &'a str,
    pub arguments: Vec<alloc::string::String>,
    pub environment: Vec<(alloc::string::String, alloc::string::String)>,
    pub stack_size: Option<u64>,
    pub heap_size: Option<u64>,
}

impl<'a> Default for ProcessConfig<'a> {
    fn default() -> Self {
        ProcessConfig {
            name: "unnamed",
            arguments: Vec::new(),
            environment: Vec::new(),
            stack_size: None,
            heap_size: None,
        }
    }
}

/// High-level API for process management
pub struct ProcessSystem {
    manager: ProcessManager,
    initialized: bool,
}

impl ProcessSystem {
    pub fn new() -> Self {
        ProcessSystem {
            manager: ProcessManager::new(),
            initialized: false,
        }
    }

    /// Initialize the process subsystem with memory manager
    pub fn initialize(
        &mut self,
        physical_memory_offset: x86_64::VirtAddr,
        memory_map: &'static bootloader::bootinfo::MemoryMap,
    ) -> Result<(), &'static str> {
        use crate::memory;

        // Initialize memory system
        let _mapper = unsafe { memory::init(physical_memory_offset) };
        let _frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(memory_map) };

        self.initialized = true;
        Ok(())
    }

    /// Load and create a process from ELF binary
    pub fn load_process(
        &mut self,
        elf_data: &[u8],
        config: ProcessConfig,
    ) -> Result<ProcessId, LoadError> {
        if !self.initialized {
            return Err(LoadError::MemoryAllocationFailed);
        }

        // For now, return dummy implementation
        // You'll need to pass mapper and frame_allocator here
        Ok(ProcessId::new(1))
    }

    /// Get process manager reference
    pub fn manager(&mut self) -> &mut ProcessManager {
        &mut self.manager
    }
}

// Global process system instance
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref PROCESS_SYSTEM: Mutex<ProcessSystem> = Mutex::new(ProcessSystem::new());
}
