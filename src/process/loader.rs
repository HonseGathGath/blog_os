//! ELF parsing using elf_rs crate
//! No_std compatible implementation
use crate::print;
use crate::println;
use alloc::vec::Vec;
use core::convert::TryFrom;
use elf_rs::ElfHeader32;
use elf_rs::{
    Elf, Elf64, ElfFile, ElfHeaderRaw, ElfMachine, ElfType, ProgramHeader64, ProgramHeaderFlags,
    ProgramHeaderRaw, ProgramType,
};
/// ELF parsing errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    InvalidMagic,
    Not64Bit,
    NotExecutable,
    NotX86_64,
    InvalidEntryPoint,
    InvalidSegment,
    SegmentOverlap,
    UnsupportedSegment,
    BoundsError,
    AlignmentError,
    ZeroSizeSegment,
}

/// Information about a segment to load
#[derive(Debug, Clone)]
pub struct SegmentInfo {
    pub virtual_addr: u64,
    pub size_in_memory: u64,
    pub size_in_file: u64,
    pub file_offset: u64,
    pub flags: u32, // PF_R, PF_W, PF_X
    pub is_loadable: bool,
    pub align: u64,
}

impl SegmentInfo {
    /// Parse from elf_rs ProgramHeader
    pub fn from_program_header(ph: &dyn ProgramHeaderRaw) -> Result<Self, ElfError> {
        let is_loadable = ph.ph_type() == ProgramType::LOAD;

        // Validate segment has valid size
        if ph.memsz() == 0 && is_loadable {
            return Err(ElfError::ZeroSizeSegment);
        }

        // Convert segment flags to u32
        let flags = {
            let mut f = 0;
            if ph.flags().contains(ProgramHeaderFlags::EXECUTE) {
                f |= 0x1
            }
            if ph.flags().contains(ProgramHeaderFlags::WRITE) {
                f |= 0x2
            }
            if ph.flags().contains(ProgramHeaderFlags::READ) {
                f |= 0x4
            }
            f
        };

        Ok(SegmentInfo {
            virtual_addr: ph.vaddr(),
            size_in_memory: ph.memsz(),
            size_in_file: ph.filesz(),
            file_offset: ph.offset(),
            flags,
            is_loadable,
            align: ph.align(),
        })
    }

    /// Get BSS size (zero-initialized data after file content)
    pub fn bss_size(&self) -> u64 {
        self.size_in_memory.saturating_sub(self.size_in_file)
    }
}

/// Parse ELF file and extract loadable segments
pub fn parse_elf(elf_data: &[u8]) -> Result<Vec<SegmentInfo>, ElfError> {
    println!("=== ELF STRUCTURE DEBUG ===");

    // Manual check of ELF fields
    // if elf_data.len() >= 64 {
    //     // e_phoff - program header table offset (8 bytes at offset 0x20)
    //     let phoff = u64::from_le_bytes([
    //         elf_data[0x20],
    //         elf_data[0x21],
    //         elf_data[0x22],
    //         elf_data[0x23],
    //         elf_data[0x24],
    //         elf_data[0x25],
    //         elf_data[0x26],
    //         elf_data[0x27],
    //     ]);
    //
    //     // e_phnum - number of program headers (2 bytes at offset 0x38)
    //     let phnum = u16::from_le_bytes([elf_data[0x38], elf_data[0x39]]);
    //
    //     // e_phentsize - size of each program header (2 bytes at offset 0x36)
    //     let phentsize = u16::from_le_bytes([elf_data[0x36], elf_data[0x37]]);
    //
    //     println!("Program header table:");
    //     println!("  Offset (e_phoff): 0x{:x} ({} dec)", phoff, phoff);
    //     println!("  Count (e_phnum): {}", phnum);
    //     println!("  Size each (e_phentsize): {} bytes", phentsize);
    //
    //     // Check if program header is within bounds
    //     let ph_table_end = phoff as usize + (phnum as usize * phentsize as usize);
    //     println!("  Table spans: bytes {}-{}", phoff, ph_table_end - 1);
    //     println!("  File size: {} bytes", elf_data.len());
    //
    //     if ph_table_end > elf_data.len() {
    //         println!("❌ Program header table exceeds file bounds!");
    //     }
    //
    //     // Check alignment
    //     println!("  phoff alignment: {} mod 8 = {}", phoff, phoff % 8);
    //     if phoff % 8 != 0 {
    //         println!("❌ Program header not 8-byte aligned!");
    //     }
    // }

    // Parse ELF using elf_rs
    let elf = Elf64::from_bytes(elf_data).map_err(|_| ElfError::InvalidMagic)?;

    // Validate ELF properties
    validate_elf(&elf)?;

    // Extract loadable segments
    let mut segments = Vec::new();

    for program_header_entry in elf.program_header_iter() {
        let phdr_raw: &dyn ProgramHeaderRaw = &*program_header_entry;

        match SegmentInfo::from_program_header(phdr_raw) {
            Ok(segment_info) if segment_info.is_loadable => {
                // Validate segment
                validate_segment(&segment_info, elf_data.len() as u64)?;
                segments.push(segment_info);
            }
            Ok(_) => {} // Skip non-loadable segments
            Err(_) => {
                println!("uwu ghalet");
            }
        }
    }

    // Sort segments by virtual address for overlap checking
    segments.sort_by_key(|s| s.virtual_addr);

    // Check for overlaps
    check_segment_overlaps(&segments)?;

    Ok(segments)
}

/// Validate ELF file properties
fn validate_elf(elf: &Elf64) -> Result<(), ElfError> {
    // Must be executable type
    match elf.elf_header().elftype() {
        elf_rs::ElfType::ET_EXEC => (),
        _ => return Err(ElfError::NotExecutable),
    }

    // Must be x86_64 architecture
    match elf.elf_header().machine() {
        elf_rs::ElfMachine::x86_64 => (),
        _ => return Err(ElfError::NotX86_64),
    }

    Ok(())
}

/// Validate segment properties
fn validate_segment(segment: &SegmentInfo, file_size: u64) -> Result<(), ElfError> {
    // Check bounds in file
    if segment.size_in_file > 0 {
        let file_end = segment.file_offset.saturating_add(segment.size_in_file);
        if file_end > file_size {
            return Err(ElfError::BoundsError);
        }
    }

    // Check alignment (must be 0 or a power of 2)
    if segment.align != 0 {
        if !segment.align.is_power_of_two() {
            return Err(ElfError::AlignmentError);
        }
        if segment.virtual_addr % segment.align != 0 {
            return Err(ElfError::AlignmentError);
        }
    }

    Ok(())
}

/// Check that segments don't overlap in memory
fn check_segment_overlaps(segments: &[SegmentInfo]) -> Result<(), ElfError> {
    for i in 0..segments.len() {
        let a = &segments[i];
        if a.size_in_memory == 0 {
            continue;
        }

        let a_end = a.virtual_addr.saturating_add(a.size_in_memory);

        for j in (i + 1)..segments.len() {
            let b = &segments[j];
            if b.size_in_memory == 0 {
                continue;
            }

            let b_end = b.virtual_addr.saturating_add(b.size_in_memory);

            // Check if segments overlap
            if a.virtual_addr < b_end && b.virtual_addr < a_end {
                return Err(ElfError::SegmentOverlap);
            }
        }
    }
    Ok(())
}

/// Parse ELF entry point
pub fn get_entry_point(elf_data: &[u8]) -> Result<u64, ElfError> {
    let elf = Elf64::from_bytes(elf_data).map_err(|_| ElfError::InvalidMagic)?;

    Ok(elf.entry_point())
}
