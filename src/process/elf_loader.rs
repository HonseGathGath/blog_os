//! ELF memory loading using your memory manager
//! Integrates with your existing memory.rs and allocator.rs

use super::loader::{ElfError, SegmentInfo};
use alloc::vec::Vec;
use core::ptr;
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, Size4KiB},
    PhysAddr, VirtAddr,
};
/// Memory loading errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    MemoryAllocationFailed,
    InvalidSegment,
    PageTableError,
    NoEntryPoint,
    StackAllocationFailed,
    FrameAllocationFailed,
    CopyFailed,
    ZeroFailed,
    InvalidProgramHeader,
    AlignmentError,
    StackSizeError,
    BoundsError,
}

/// A loaded program in memory
pub struct LoadedProgram {
    /// Entry point virtual address
    pub entry_point: u64,
    /// Stack pointer (top of stack, grows downward)
    pub stack_top: VirtAddr,
    /// Heap start address
    pub heap_start: VirtAddr,
    /// Memory regions used by this program
    pub regions: Vec<MemoryRegion>,
    /// Total memory used (including stack)
    pub total_memory: u64,
}

/// Memory region with permissions
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start: VirtAddr,
    pub end: VirtAddr,
    pub flags: PageTableFlags,
    pub is_mapped: bool,
    pub is_heap: bool,
    pub is_stack: bool,
}

/// Load ELF segments into memory
pub fn load_elf_program(
    elf_data: &[u8],
    segments: &[SegmentInfo],
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    stack_size: Option<u64>,
    heap_size: Option<u64>,
) -> Result<LoadedProgram, LoadError> {
    let entry_point =
        super::loader::get_entry_point(elf_data).map_err(|_| LoadError::NoEntryPoint)?;

    // Calculate program memory layout
    let (stack_top, heap_start) = calculate_memory_layout(segments, stack_size, heap_size)?;

    // 1. Create empty program structure
    let mut program = LoadedProgram {
        entry_point,
        stack_top,
        heap_start,
        regions: Vec::new(),
        total_memory: 0,
    };

    // 2. Load each segment
    for segment in segments {
        if segment.is_loadable && segment.size_in_memory > 0 {
            load_segment(segment, elf_data, mapper, frame_allocator, &mut program)?;
        }
    }

    // 3. Allocate user stack
    let stack_size = stack_size.unwrap_or(8 * 1024 * 1024); // Default 8MB
    allocate_user_stack(mapper, frame_allocator, &mut program, stack_size)?;

    // 4. Reserve heap space (no pages allocated yet)
    let heap_size = heap_size.unwrap_or(2 * 1024 * 1024); // Default 2MB
    reserve_heap_region(&mut program, heap_size);

    // Calculate total memory usage
    program.total_memory = calculate_total_memory(&program.regions);

    Ok(program)
}

/// Calculate memory layout avoiding conflicts with existing segments
fn calculate_memory_layout(
    segments: &[SegmentInfo],
    stack_size: Option<u64>,
    heap_size: Option<u64>,
) -> Result<(VirtAddr, VirtAddr), LoadError> {
    // Default addresses
    let default_stack_top = VirtAddr::new(0x7FFF_FFFF_F000); // 8MB below 8GB
    let default_heap_start = VirtAddr::new(0x4000_0000); // 1GB mark

    // Find highest segment address
    let highest_segment = segments
        .iter()
        .filter(|s| s.is_loadable)
        .map(|s| s.virtual_addr + s.size_in_memory)
        .max()
        .unwrap_or(0);

    let stack_top = if highest_segment > default_stack_top.as_u64() {
        // Move stack above segments
        VirtAddr::new(highest_segment + stack_size.unwrap_or(8 * 1024 * 1024))
    } else {
        default_stack_top
    };

    let heap_start = if highest_segment > default_heap_start.as_u64() {
        // Move heap above segments
        VirtAddr::new(highest_segment + 0x1000) // 4KB alignment
    } else {
        default_heap_start
    };

    Ok((stack_top, heap_start))
}

/// Load a single ELF segment into memory
fn load_segment(
    segment: &SegmentInfo,
    elf_data: &[u8],
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    program: &mut LoadedProgram,
) -> Result<(), LoadError> {
    // Convert segment flags to page table flags
    let flags = segment_flags_to_page_flags(segment.flags);

    // Calculate page-aligned range
    let virt_start = VirtAddr::new(segment.virtual_addr);
    let virt_end = virt_start + segment.size_in_memory;

    let page_start = Page::containing_address(virt_start);
    let page_end = Page::containing_address(virt_end - 1u64) + 1;

    // Map each page in the segment
    for page in Page::range(page_start, page_end) {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(LoadError::FrameAllocationFailed)?;

        unsafe {
            mapper
                .map_to(page, frame, flags, frame_allocator)
                .map_err(|_| LoadError::MemoryAllocationFailed)?
                .ignore();
        }
    }

    // Copy segment data from ELF file
    if segment.size_in_file > 0 {
        copy_segment_data(
            virt_start,
            elf_data,
            segment.file_offset,
            segment.size_in_file,
        )?;
    }

    // Zero BSS section
    if segment.size_in_memory > segment.size_in_file {
        let bss_start = virt_start + segment.size_in_file;
        let bss_size = segment.size_in_memory - segment.size_in_file;

        zero_memory(bss_start, bss_size)?;
    }

    // Record memory region
    program.regions.push(MemoryRegion {
        start: virt_start,
        end: virt_end,
        flags,
        is_mapped: true,
        is_heap: false,
        is_stack: false,
    });

    Ok(())
}

/// Allocate user stack pages
fn allocate_user_stack(
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    program: &mut LoadedProgram,
    stack_size: u64,
) -> Result<(), LoadError> {
    if stack_size == 0 {
        return Err(LoadError::StackSizeError);
    }

    let stack_bottom = program.stack_top - stack_size;

    // Check stack alignment
    if stack_bottom.as_u64() % 4096 != 0 {
        return Err(LoadError::AlignmentError);
    }

    let stack_start_page = Page::containing_address(stack_bottom);
    let stack_end_page = Page::containing_address(program.stack_top - 1u64) + 1;

    let flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::NO_EXECUTE;

    for page in Page::range(stack_start_page, stack_end_page) {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(LoadError::StackAllocationFailed)?;

        unsafe {
            mapper
                .map_to(page, frame, flags, frame_allocator)
                .map_err(|_| LoadError::StackAllocationFailed)?
                .ignore();
        }
    }

    // Record stack region
    program.regions.push(MemoryRegion {
        start: stack_bottom,
        end: program.stack_top,
        flags,
        is_mapped: true,
        is_heap: false,
        is_stack: true,
    });

    Ok(())
}

/// Reserve heap region (no pages allocated yet)
fn reserve_heap_region(program: &mut LoadedProgram, heap_size: u64) {
    if heap_size == 0 {
        return;
    }

    let heap_end = program.heap_start + heap_size;

    program.regions.push(MemoryRegion {
        start: program.heap_start,
        end: heap_end,
        flags: PageTableFlags::PRESENT
            | PageTableFlags::WRITABLE
            | PageTableFlags::USER_ACCESSIBLE
            | PageTableFlags::NO_EXECUTE,
        is_mapped: false, // Heap pages allocated on demand
        is_heap: true,
        is_stack: false,
    });
}

/// Copy segment data from ELF file to mapped memory
fn copy_segment_data(
    virt_start: VirtAddr,
    elf_data: &[u8],
    file_offset: u64,
    size_in_file: u64,
) -> Result<(), LoadError> {
    if size_in_file == 0 {
        return Ok(());
    }

    // Get data slice from ELF file
    let start = file_offset as usize;
    let end = start
        .checked_add(size_in_file as usize)
        .ok_or(LoadError::BoundsError)?;

    if end > elf_data.len() {
        return Err(LoadError::BoundsError);
    }

    let segment_data = &elf_data[start..end];

    // Convert virtual address to pointer and copy
    let dest_ptr = virt_start.as_u64() as *mut u8;

    unsafe {
        ptr::copy_nonoverlapping(segment_data.as_ptr(), dest_ptr, segment_data.len());
    }

    Ok(())
}

/// Zero memory region
fn zero_memory(start: VirtAddr, size: u64) -> Result<(), LoadError> {
    if size == 0 {
        return Ok(());
    }

    let start_ptr = start.as_u64() as *mut u8;

    unsafe {
        ptr::write_bytes(start_ptr, 0, size as usize);
    }

    Ok(())
}

/// Convert ELF segment flags to page table flags
fn segment_flags_to_page_flags(elf_flags: u32) -> PageTableFlags {
    let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;

    if (elf_flags & 0x2) != 0 {
        // PF_W - Writable
        flags |= PageTableFlags::WRITABLE;
    }
    if (elf_flags & 0x1) == 0 {
        // PF_X - NOT Executable
        flags |= PageTableFlags::NO_EXECUTE;
    }
    // Note: READ flag is always implied by PRESENT flag

    flags
}

/// Calculate total memory used by regions
fn calculate_total_memory(regions: &[MemoryRegion]) -> u64 {
    regions
        .iter()
        .filter(|r| r.is_mapped)
        .map(|r| (r.end - r.start))
        .sum()
}
