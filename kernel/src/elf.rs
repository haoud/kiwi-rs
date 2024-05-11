use crate::pmm;

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
        let misalign = segment.p_vaddr as usize % arch::mmu::PAGE_SIZE;
        let start = segment.p_vaddr as usize - misalign;
        let filesize = segment.p_filesz as usize;
        let memsize = segment.p_memsz as usize;

        for page in (start..start + memsize).step_by(arch::mmu::PAGE_SIZE) {
            let section_offset = page - start;
            let file_offset =
                segment.p_offset as usize + section_offset - misalign;
            let addr = arch::mmu::Virtual::new(page);

            let mut frame = pmm::allocate_frame(pmm::AllocationFlags::ZEROED)
                .expect("Failed to allocate zeroed page");

            // Map the page into the thread's page table
            loop {
                match arch::mmu::map(
                    thread.table_mut(),
                    addr,
                    frame,
                    arch::mmu::Rights::RWX | arch::mmu::Rights::USER,
                    arch::mmu::Flags::empty(),
                ) {
                    Err(arch::mmu::MapError::FrameConsumed) => {
                        frame =
                            pmm::allocate_frame(pmm::AllocationFlags::ZEROED)
                                .expect("Failed to allocate zeroed page");
                    }
                    Err(e) => {
                        panic!("Failed to map page: {:?}", e);
                    }
                    Ok(()) => break,
                }
            }

            // Compute the size of the data to copy into the physical
            // page and compute the source and destination pointers
            let remaning = filesize.saturating_sub(section_offset);
            let size = core::cmp::min(arch::mmu::PAGE_SIZE, remaning);
            let src = file.as_ptr().wrapping_add(file_offset);
            let dst = arch::mmu::translate_physical(frame)
                .unwrap()
                .as_mut_ptr::<u8>();

            // Copy the data into the physical page
            unsafe {
                core::ptr::copy_nonoverlapping(src, dst, size);
            }
        }
    }

    log::debug!("Loaded ELF file at 0x{:x}", entry);
    thread
}
