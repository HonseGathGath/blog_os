
//qemu-system-x86_64 -drive format=raw,file=target/x86_64-blog_os/debug/bootimage-blog_os.bin


#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]


use core::panic::PanicInfo;
use blog_os::println;
use bootloader::{BootInfo, entry_point};

entry_point!(kernel_main);

const PHYS_MEM_OFFSET: u64 = 0xffff_8000_0000_0000;

fn kernel_main(_boot_info: &'static BootInfo) -> ! {

    use blog_os::memory::translate_addr;
   // add code here
    use x86_64::VirtAddr;

    println!("Hello World{}", "!");
    blog_os::init();

    let phys_mem_offset = VirtAddr::new(PHYS_MEM_OFFSET);
    let addresses = [
        0xb8000,
        0x201008,
        0x0100_0020_1a10,
        PHYS_MEM_OFFSET
    ];

    for &address in &addresses {
        let virt = VirtAddr::new(address);
        let phys = unsafe { translate_addr(virt, phys_mem_offset) };
        println!("{:?} -> {:?}", virt, phys);
    }

    loop{}


 /*   let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let l4_table = unsafe { active_level_4_table(phys_mem_offset) };

    for (i, entry) in l4_table.iter().enumerate() {
       
use x86_64::structures::paging::PageTable;

if !entry.is_unused() {
    println!("L4 Entry {}: {:?}", i, entry);

    // get the physical address from the entry and convert it
    let phys = entry.frame().unwrap().start_address();
    let virt = phys.as_u64() + PHYS_MEM_OFFSET;
    let ptr = VirtAddr::new(virt).as_mut_ptr();
    let l3_table: &PageTable = unsafe { &*ptr };

    // print non-empty entries of the level 3 table
    for (i, entry) in l3_table.iter().enumerate() {
        if !entry.is_unused() {
            println!("  L3 Entry {}: {:?}", i, entry);
        }
    }
}    }

    // as before
    #[cfg(test)]
16H   test_main();

    println!("It did not crash!");
16H   blog_os::hlt_loop();


*/
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

