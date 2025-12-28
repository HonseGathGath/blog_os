//! Unit tests for process management
#![no_std]

use super::*;
use crate::{print, println};
use alloc::string::ToString;
use alloc::vec;
use x86_64::VirtAddr;

macro_rules! test {
    ($name:ident, $body:block) => {
        fn $name() -> Result<(), &'static str> {
            $body
        }
    };
}

/// Helper to run a test and print results
fn run_test<F>(name: &str, test_func: F)
where
    F: FnOnce() -> Result<(), &'static str>,
{
    print!("Testing {}... ", name);
    match test_func() {
        Ok(_) => println!("OK"),
        Err(e) => println!("FAILED: {}", e),
    }
}

/// Test helper: Create a minimal valid ELF binary
pub fn create_minimal_elf() -> Result<Vec<u8>, &'static str> {
    let mut elf = Vec::new();

    // ELF Header (64 bytes)
    // Magic
    elf.extend_from_slice(&[0x7F, b'E', b'L', b'F']);
    // 64-bit, little endian
    elf.push(2); // ELF_CLASS_64
    elf.push(1); // ELF_DATA_LITTLE
    elf.push(1); // Version 1
    elf.push(0); // OS ABI
    elf.extend_from_slice(&[0; 7]); // Padding

    // Type: Executable
    elf.extend_from_slice(&[0x02, 0x00]); // ET_EXEC

    // Machine: x86_64
    elf.extend_from_slice(&[0x3E, 0x00]); // EM_X86_64

    // Version again
    elf.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

    // Entry point
    elf.extend_from_slice(&[0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00]); // 0x400000

    // Program header offset
    elf.extend_from_slice(&[0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // After header

    // Section header offset (none)
    elf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // Flags
    elf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    // ELF header size
    elf.extend_from_slice(&[0x40, 0x00]); // 64 bytes

    // Program header entry size
    elf.extend_from_slice(&[0x38, 0x00]); // 56 bytes

    // Number of program headers: 1
    elf.extend_from_slice(&[0x01, 0x00]);

    // Section header entry size, count, string table index
    elf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // Ensure we have at least 64 bytes for header
    if elf.len() < 64 {
        elf.resize(64, 0);
    }

    // Add a program header (LOAD segment)
    // p_type: PT_LOAD (1)
    elf.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

    // p_flags: R + X (5)
    elf.extend_from_slice(&[0x05, 0x00, 0x00, 0x00]);

    // p_offset: 0x100
    elf.extend_from_slice(&[0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // p_vaddr: 0x400000
    elf.extend_from_slice(&[0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // p_paddr: same
    elf.extend_from_slice(&[0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // p_filesz: 0x100 bytes
    elf.extend_from_slice(&[0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // p_memsz: 0x1000 bytes
    elf.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // p_align: 0x1000
    elf.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // Fill with some executable code (nop + ret)
    let code_start = 0x100;
    if elf.len() < code_start {
        elf.resize(code_start, 0);
    }
    elf[code_start] = 0x90; // NOP
    elf[code_start + 1] = 0xC3; // RET

    Ok(elf)
}

/// Test helper: Create an invalid ELF
fn create_invalid_elf() -> Vec<u8> {
    // Just random bytes, not a valid ELF
    vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05]
}

/// Test: Process creation
test!(process_creation, {
    let mut manager = ProcessManager::new();

    // Create a kernel thread
    let process =
        Process::new_kernel_thread(VirtAddr::new(0x1000), VirtAddr::new(0x8000), "test_thread");

    let pid = manager.add_process(process);

    assert!(pid.as_u64() > 0, "PID should be assigned");
    assert_eq!(manager.process_count(), 1, "Should have 1 process");

    Ok(())
});

/// Test: Process scheduling
test!(process_scheduling, {
    let mut manager = ProcessManager::new();

    // Add multiple processes
    for i in 0..3 {
        let process = Process::new_kernel_thread(
            VirtAddr::new(0x1000 + i * 0x1000),
            VirtAddr::new(0x8000 + i * 0x1000),
            &alloc::format!("thread_{}", i),
        );
        manager.add_process(process);
    }

    assert_eq!(manager.process_count(), 3, "Should have 3 processes");

    // First schedule
    let scheduled = manager.schedule();
    assert!(scheduled.is_some(), "Should schedule a process");
    assert_eq!(scheduled.unwrap().pid.as_u64(), 1, "First PID should be 1");

    // Yield and schedule next
    manager.yield_current();
    let scheduled2 = manager.schedule();
    assert!(scheduled2.is_some(), "Should schedule another process");
    assert_eq!(
        scheduled2.unwrap().pid.as_u64(),
        2,
        "Second PID should be 2"
    );

    Ok(())
});

/// Test: ELF parsing valid file
test!(elf_parsing_valid, {
    let elf_data = create_minimal_elf()?;

    let segments = loader::parse_elf(&elf_data).map_err(|_| "Failed to parse valid ELF")?;

    assert!(segments.len() > 0, "Should have at least one segment");

    let segment = &segments[0];
    assert!(segment.is_loadable, "Segment should be loadable");
    assert_eq!(
        segment.virtual_addr, 0x400000,
        "Virtual address should be 0x400000"
    );
    assert_eq!(
        segment.size_in_memory, 0x1000,
        "Memory size should be 0x1000"
    );

    Ok(())
});

/// Test: ELF parsing invalid file
test!(elf_parsing_invalid, {
    let invalid_elf = create_invalid_elf();

    let result = loader::parse_elf(&invalid_elf);
    assert!(result.is_err(), "Should fail on invalid ELF");

    Ok(())
});

/// Test: Process context creation
test!(process_context, {
    // Test user context
    let user_context = ProcessContext::new_user(0x400000, 0x7FFFFFFF);

    assert!(user_context.is_user_mode(), "Should be user mode");

    // Test kernel context
    let kernel_context = ProcessContext::new_kernel(0x1000, 0x8000);
    assert!(!kernel_context.is_user_mode(), "Should not be user mode");

    Ok(())
});

/// Test: Memory region validation
test!(memory_regions, {
    let mut process =
        Process::new_kernel_thread(VirtAddr::new(0x1000), VirtAddr::new(0x8000), "test");

    // Add a memory region
    let region = MemoryRegion {
        start: VirtAddr::new(0x400000),
        end: VirtAddr::new(0x401000),
        flags: x86_64::structures::paging::PageTableFlags::PRESENT
            | x86_64::structures::paging::PageTableFlags::WRITABLE,
        is_mapped: true,
        is_heap: false,
        is_stack: false,
    };

    process.memory_regions.push(region);

    // Test address validation
    assert!(
        process.is_valid_address(VirtAddr::new(0x400000)),
        "Start address should be valid"
    );
    assert!(
        process.is_valid_address(VirtAddr::new(0x400FFF)),
        "Address inside region should be valid"
    );
    assert!(
        !process.is_valid_address(VirtAddr::new(0x401000)),
        "End address should not be valid (exclusive)"
    );
    assert!(
        !process.is_valid_address(VirtAddr::new(0x500000)),
        "Address outside region should not be valid"
    );

    Ok(())
});

/// Test: Process state transitions
test!(process_state_transitions, {
    let mut process =
        Process::new_kernel_thread(VirtAddr::new(0x1000), VirtAddr::new(0x8000), "state_test");

    // Initial state should be New
    assert!(
        matches!(process.state, ProcessState::New),
        "Initial state should be New"
    );

    // Test runnable check
    process.state = ProcessState::Ready;
    assert!(process.is_runnable(), "Ready state should be runnable");

    process.state = ProcessState::Running;
    assert!(process.is_runnable(), "Running state should be runnable");

    process.state = ProcessState::Blocked;
    assert!(
        !process.is_runnable(),
        "Blocked state should not be runnable"
    );

    process.state = ProcessState::Zombie;
    assert!(
        !process.is_runnable(),
        "Zombie state should not be runnable"
    );

    // Test alive check
    process.state = ProcessState::Ready;
    assert!(process.is_alive(), "Ready process should be alive");

    process.state = ProcessState::Zombie;
    assert!(!process.is_alive(), "Zombie process should not be alive");

    Ok(())
});

/// Test: Process priority
test!(process_priority, {
    let process = Process::new_kernel_thread(
        VirtAddr::new(0x1000),
        VirtAddr::new(0x8000),
        "priority_test",
    );

    // Default priority should be Normal
    assert!(
        matches!(process.priority, ProcessPriority::Normal),
        "Default priority should be Normal"
    );

    // Test priority ordering
    assert!(
        ProcessPriority::RealTime > ProcessPriority::Normal,
        "RealTime should be higher than Normal"
    );
    assert!(
        ProcessPriority::Low < ProcessPriority::High,
        "Low should be lower than High"
    );
    assert!(
        ProcessPriority::Idle < ProcessPriority::RealTime,
        "Idle should be lowest"
    );

    Ok(())
});

/// Test: Segment info methods
test!(segment_info_methods, {
    let segment = SegmentInfo {
        virtual_addr: 0x400000,
        size_in_memory: 0x2000,
        size_in_file: 0x1000,
        file_offset: 0x100,
        flags: 0x5, // R + X
        is_loadable: true,
        align: 0x1000,
    };

    // Test BSS calculation
    let bss_size = segment.bss_size();
    assert_eq!(
        bss_size, 0x1000,
        "BSS size should be memory_size - file_size"
    );

    // Test zero file size
    let zero_file_segment = SegmentInfo {
        size_in_memory: 0x1000,
        size_in_file: 0,
        ..segment
    };
    assert_eq!(
        zero_file_segment.bss_size(),
        0x1000,
        "BSS should equal memory size when file size is 0"
    );

    // Test memory smaller than file (shouldn't happen but test anyway)
    let invalid_segment = SegmentInfo {
        size_in_memory: 0x500,
        size_in_file: 0x1000,
        ..segment
    };
    assert_eq!(
        invalid_segment.bss_size(),
        0,
        "BSS should be 0 when memory < file"
    );

    Ok(())
});

/// Test: Error types
test!(error_types, {
    // Test ElfError debug
    let error = ElfError::InvalidMagic;
    let debug_output = alloc::format!("{:?}", error);
    assert!(!debug_output.is_empty(), "Should format debug output");

    // Test LoadError debug
    let load_error = LoadError::MemoryAllocationFailed;
    let load_debug = alloc::format!("{:?}", load_error);
    assert!(!load_debug.is_empty(), "Should format LoadError debug");

    Ok(())
});

/// Test: Process ID generation
test!(process_id_generation, {
    use super::process::allocate_pid;

    let pid1 = allocate_pid();
    let pid2 = allocate_pid();
    let pid3 = allocate_pid();

    assert!(pid2 > pid1, "PIDs should increment");
    assert!(pid3 > pid2, "PIDs should continue incrementing");

    // Test ProcessId methods
    let process_id = ProcessId::new(42);
    assert_eq!(process_id.as_u64(), 42, "as_u64 should return value");

    // Test equality
    let id1 = ProcessId::new(100);
    let id2 = ProcessId::new(100);
    let id3 = ProcessId::new(200);

    assert!(id1 == id2, "Same values should be equal");
    assert!(id1 != id3, "Different values should not be equal");

    Ok(())
});

/// Run all process tests
pub fn run_process_tests() {
    println!("\n=== Process Module Tests ===");

    // Run each test with our runner
    run_test("process_creation", process_creation);
    run_test("process_scheduling", process_scheduling);
    run_test("elf_parsing_valid", elf_parsing_valid);
    run_test("elf_parsing_invalid", elf_parsing_invalid);
    run_test("process_context", process_context);
    run_test("memory_regions", memory_regions);
    run_test("process_state_transitions", process_state_transitions);
    run_test("process_priority", process_priority);
    run_test("segment_info_methods", segment_info_methods);
    run_test("error_types", error_types);
    run_test("process_id_generation", process_id_generation);

    println!("=== End Tests ===");
}
