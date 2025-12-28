#[cfg(test)]
mod host_tests {
    use std::fs;
    use std::process::Command;

    #[test]
    fn test_elf_parser_on_host() {
        // Create a minimal ELF file using host tools
        let elf_data = create_test_elf();

        // Parse using your loader (must be no_std compatible)
        let segments = your_os::process::loader::parse_elf(&elf_data);
        assert!(segments.is_ok());
    }

    #[test]
    fn test_mock_process_creation() {
        // Test process structures without actual execution
        let process = your_os::process::process::Process::new_kernel_thread(
            your_os::process::process::VirtAddr::new(0x1000),
            your_os::process::process::VirtAddr::new(0x8000),
            "test",
        );

        assert_eq!(process.pid.as_u64(), 0); // Not allocated yet
        assert_eq!(process.name, "test");
    }
}
