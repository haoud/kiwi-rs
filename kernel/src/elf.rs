use crate::{arch, pmm};

/// Load an ELF file into memory and return a thread that can be executed.
///
/// # Safety
/// This function should only be called once to initialize thread during
/// the boot process. After the boot process, the memory used by this
/// function will be reclaimed by the kernel to reuse it for other purposes.
#[macros::init]
pub fn load(file: &[u8]) -> arch::thread::Thread {
    let header =
        elf::ElfBytes::<elf::endian::LittleEndian>::minimal_parse(file)
            .expect("Failed to parse ELF file");
    let entry = header.ehdr.e_entry as usize;

    let mut thread = arch::thread::create(entry, 0);
    for segment in header
        .segments()
        .unwrap()
        .iter()
        .filter(|phdr| phdr.p_type == elf::abi::PT_LOAD)
    {
        let segment_mem_size = segment.p_memsz as usize;
        let segment_mem_start = segment.p_vaddr as usize;
        let segment_file_size = segment.p_filesz as usize;
        let segment_mem_end = segment_mem_start + segment_mem_size;

        // Compute the aligned memory start address and the misalignment of the
        // segment in memory
        let mut misalign = segment_mem_start % arch::mmu::PAGE_SIZE;
        let segment_aligned_mem_start = segment_mem_start - misalign;

        log::trace!(
            "Loading elf segment 0x{:x} - 0x{:x} (misalign: 0x{:x})",
            segment_mem_start,
            segment_mem_end,
            misalign
        );

        // Map each page in the segment into the thread's page table. If the
        // start address of the segment is not page aligned, the first page
        // will be partially filled with data from the ELF file and the rest
        // of the page will handled normally.
        for page in (segment_aligned_mem_start..segment_mem_end)
            .step_by(arch::mmu::PAGE_SIZE)
        {
            let section_offset = page + misalign - segment_mem_start;
            let file_offset = segment.p_offset as usize + section_offset;
            log::trace!(
                "Mapping page 0x{:x} with offset 0x{:x}",
                page,
                file_offset
            );
            let addr = arch::mmu::Virtual::new(page);

            let frame = pmm::allocate_frame(pmm::AllocationFlags::ZEROED)
                .expect("Failed to allocate zeroed page");

            // Map the page into the thread's page table
            arch::mmu::map(
                thread.table_mut(),
                addr,
                frame,
                arch::mmu::Rights::RWX | arch::mmu::Rights::USER,
                arch::mmu::Flags::empty(),
            )
            .expect("Failed to map page");

            // Compute the size of the data to copy into the physical
            // page and compute the source and destination pointers
            let remaning = segment_file_size.saturating_sub(section_offset);
            let size =
                core::cmp::min(arch::mmu::PAGE_SIZE - misalign, remaning);
            let src = file.as_ptr().wrapping_add(file_offset);
            let dst = arch::mmu::translate_physical(frame)
                .unwrap()
                .as_mut_ptr::<u8>()
                .wrapping_add(misalign);

            // Copy the data into the physical page
            unsafe {
                core::ptr::copy_nonoverlapping(src, dst, size);
            }

            misalign = 0;
        }
    }

    log::debug!("Loaded ELF file at 0x{:x}", entry);
    thread
}
