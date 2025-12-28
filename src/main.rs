//qemu-system-x86_64 -drive format=raw,file=target/x86_64-blog_os/debug/bootimage-blog_os.bin

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

//use alloc::boxed::Box;
use alloc::vec::Vec;
use blog_os::task::executor::Executor;
use blog_os::task::{keyboard, Task};
use blog_os::{print, println};
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use blog_os::allocator;
    use blog_os::memory::{self, BootInfoFrameAllocator};
    // add code here
    use blog_os::process::*;
    use x86_64::{structures::paging::Page, VirtAddr};

    println!("Hello World{}", "!");
    blog_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };

    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // map an unused page
    let page = Page::containing_address(VirtAddr::new(0));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap init failed");

    /*let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();*/

    {
        let mut process_system = PROCESS_SYSTEM.lock();
        process_system
            .initialize(phys_mem_offset, &boot_info.memory_map)
            .expect("Process system initialization failed");

        println!("Process system initialized");

        // Create a test kernel thread to test scheduler
        let test_thread = blog_os::process::Process::new_kernel_thread(
            VirtAddr::new(0x1000),
            VirtAddr::new(0x8000),
            "test_thread",
        );

        let pid = process_system.manager().add_process(test_thread);
        println!("Created test thread with PID: {}", pid.as_u64());
    }

    println!("\n=== ELF CREATION TEST ===");

    // Use your create_minimal_elf function from the tests module
    use alloc::vec::Vec;

    // Manually create ELF like in your test
    let ELF: [u8; 258] = create_correct_elf();
    // Try to parse it with your loader
    match loader::parse_elf(&ELF) {
        Ok(segments) => {
            println!("✓ Successfully parsed {} segments", segments.len());

            if !segments.is_empty() {
                let segment = &segments[0];
                println!(
                    "✓ Segment 0: vaddr=0x{:x}, size=0x{:x}, flags={}",
                    segment.virtual_addr, segment.size_in_memory, segment.flags
                );
            }
        }
        Err(e) => println!("✗ Failed to parse ELF: {:?}", e),
    }

    // Test your process system if you have the global instance
    {
        let mut process_system = PROCESS_SYSTEM.lock();
        process_system
            .initialize(phys_mem_offset, &boot_info.memory_map)
            .expect("Process system initialization failed");

        println!("✓ Process system initialized");

        // Add a test kernel thread
        let test_thread = Process::new_kernel_thread(
            VirtAddr::new(0x2000),
            VirtAddr::new(0x9000),
            "system_test_thread",
        );

        let system_pid = process_system.manager().add_process(test_thread);
        println!("✓ Added thread to system with PID: {}", system_pid.as_u64());

        // Schedule it
        if let Some(proc) = process_system.manager().schedule() {
            println!(
                "✓ System scheduled: {} (PID: {})",
                proc.name,
                proc.pid.as_u64()
            );
        }
    }

    println!("=== ALL TESTS COMPLETE ===\n");

    // 5. Create and run the async executor
    let mut executor = Executor::new();

    // Spawn keyboard task
    executor.spawn(Task::new(keyboard::print_keypresses()));

    // Spawn process scheduler task

    // Spawn idle task (keeps CPU from busy-waiting)

    println!("Executor ready. Starting main loop...");

    // 6. Run the executor (never returns)
    executor.run();

    //let x = Box::new(41);

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    blog_os::hlt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info);
}

#[cfg(feature = "init")]
fn load_init_process(process_system: &mut blog_os::process::ProcessSystem) {
    // Try to load a simple init program
    #[cfg(feature = "test_program")]
    {
        // Embedded test program (simple assembly that just returns)
        let init_program = [
            0x48, 0xc7, 0xc0, 0x3c, 0x00, 0x00, 0x00, // mov rax, 60 (exit syscall)
            0x48, 0xc7, 0xc7, 0x00, 0x00, 0x00, 0x00, // mov rdi, 0 (exit code)
            0x0f, 0x05, // syscall
        ];

        let config = blog_os::process::ProcessConfig {
            name: "init".to_string(),
            arguments: vec!["init".to_string()],
            environment: vec![
                ("PATH".to_string(), "/bin".to_string()),
                ("HOME".to_string(), "/root".to_string()),
            ],
            stack_size: Some(8 * 1024 * 1024),
            heap_size: Some(2 * 1024 * 1024),
        };

        match process_system.load_process(&init_program, config) {
            Ok(pid) => println!("Loaded init process with PID: {}", pid.as_u64()),
            Err(e) => println!("Failed to load init process: {:?}", e),
        }
    }
} //static HELLO: &[u8] = b"Hello World!";

/*pub extern "C" fn _start() -> ! {
    /*let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }*/

    blog_os::init();

    // trigger a page fault: invalid virtual memory
    /*unsafe {
        *(0xdeadbeef as *mut u8) = 42;
    };*/
    println!("Hello World{}", "!");
    //x86_64::instructions::interrupts::int3();


     #[cfg(test)]
    test_main();

     println!("maher is a {}", "!");

    loop{}
}*/

fn create_correct_elf() -> [u8; 258] {
    let mut elf = [0u8; 258];

    // Header (64 bytes)
    let header: [u8; 64] = [
        0x7F, b'E', b'L', b'F', 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x02, 0x00, 0x3E, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x38, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];

    // Program header (56 bytes)
    let phdr: [u8; 56] = [
        0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    // Copy sections
    elf[0..64].copy_from_slice(&header);
    elf[64..120].copy_from_slice(&phdr);
    // Padding stays zeros (bytes 120-255 = 136 bytes)
    elf[256] = 0x90; // NOP at byte 256
    elf[257] = 0xC3; // RET at byte 257

    elf
}
