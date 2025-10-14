
//qemu-system-x86_64 -drive format=raw,file=target/x86_64-blog_os/debug/bootimage-blog_os.bin


#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::boxed::Box;
use core::panic::PanicInfo;
use blog_os::println;
use bootloader::{BootInfo, entry_point};

entry_point!(kernel_main);

 

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    

    use blog_os::allocator;
    use blog_os::memory::{self, BootInfoFrameAllocator};
   // add code here
    use x86_64::{VirtAddr, structures::paging::Page};

    println!("Hello World{}", "!");
    blog_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe {
        memory::init(phys_mem_offset)
    };

     let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    // map an unused page
    let page = Page::containing_address(VirtAddr::new(0));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);
    
    allocator::init_heap(&mut mapper, &mut frame_allocator)
    .expect("heap init failed");

    let x = Box::new(41);
        
    // as before*/
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


//static HELLO: &[u8] = b"Hello World!";

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

