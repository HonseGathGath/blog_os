# Blog OS

## Overview

A hobby operating system kernel for the x86_64 architecture, developed by following the Philipp Oppermann Blog OS series. Implements core kernel components: VGA text buffer driver, interrupt descriptor table (IDT), global descriptor table (GDT), memory paging, heap allocators (bump, linked-list, fixed-size block), an async task executor, keyboard input handling, and syscall stubs. Includes integration tests run in QEMU.

## Prerequisites

- Rust toolchain (edition 2021) with `no_std` target support
- `cargo bootimage` for building bootable disk images
- QEMU for running the kernel
- `x86_64-unknown-none` target

### Installing Dependencies

```bash
rustup target add x86_64-unknown-none
cargo install bootimage
```

Ensure QEMU is installed on your system (e.g., `apt install qemu-system-x86_64` on Debian/Ubuntu).

## Build & Run

```bash
# Build the kernel
cargo build --target x86_64-blog_os.json -Z json-target-spec

# Create a bootable disk image
cargo bootimage

# Run in QEMU
qemu-system-x86_64 -drive format=raw,file=target/x86_64-blog_os/debug/bootimage-blog_os.bin
```

Or use the provided script:

```bash
./run.sh
```

### Running Tests

```bash
cargo test --target x86_64-blog_os.json -Z json-target-spec
```

## Project Structure

```
blog_os/
  Cargo.toml                — Kernel dependencies
  x86_64-blog_os.json       — Target specification
  run.sh                    — Build and run script
  src/
    main.rs                 — Kernel entry point
    lib.rs                  — Test framework, init, common traits
    vga_buffer.rs           — VGA text mode buffer driver
    gdt.rs                  — Global descriptor table
    interrupts.rs           — Interrupt descriptor table and handlers
    memory.rs               — Memory management and page table mapping
    allocator.rs            — Heap allocator interface
    allocator/              — Allocator implementations
    serial.rs               — Serial port driver
    task/                   — Async task system and executor
    syscall/                — Syscall stubs (in progress)
  tests/                    — Integration tests (heap, stack overflow)
```

## Current State

Components implemented:
- VGA text buffer with `println!` macro
- GDT segmentation
- IDT with interrupt handlers (keyboard, timer, exceptions)
- Page table mapping with example mapping
- Heap allocation (multiple allocator strategies)
- Async task executor with keyboard task
- Serial port output
- Integration test framework

Syscalls (`fork`, `exec`, `exit`, `read`, `write`) and filesystem operations are stubbed and pending implementation.

## Contributing

This is an open-source hobby OS project. Contributions, bug reports, and pull requests are welcome.

## License

Open-source software. Available under the MIT License.
