#![no_std]
#![no_main]

use arch::mmu::Virtual;

pub mod task;

/// The `kiwi` function is called after the architecture-specific initialization
/// was completed. It is responsible for setting up the kernel and starting the
/// first user-space process.
///
/// # Safety
/// This function should only be called once during the kernel boot process. Once
/// the boot process is completed, the function will be wiped from memory to free
/// up memory space.
#[macros::init]
#[no_mangle]
pub fn kiwi(mut memory: arch::memory::UsableMemory) -> ! {
    arch::log::write("Hello, world!\n");

    let init = include_bytes!("../../user/init/target/riscv64gc-unknown-none-elf/release/init");
    let data = elf::ElfBytes::<elf::endian::LittleEndian>::minimal_parse(init)
        .expect("Failed to parse ELF");

    let entry = data.ehdr.e_entry as usize;
    let mut thread = arch::thread::create(entry, 0);
    let table = thread.table_mut();

    unsafe {
        table.set_current();
    }

    for segment in data
        .segments()
        .expect("Failed to get segments")
        .iter()
        .filter(|phdr| phdr.p_type == elf::abi::PT_LOAD)
    {
        let misalign = (segment.p_vaddr % 4096) as usize;
        let start = segment.p_vaddr as usize - misalign;

        let memory_end = start + segment.p_memsz as usize;
        let file_end = start + segment.p_filesz as usize;

        for addr in (start..memory_end).step_by(4096) {
            let section_offset = addr.saturating_sub(start);
            let file_offset = segment.p_offset as usize - misalign + addr - start;
            let mut frame = memory
                .allocate_zeroed_page()
                .expect("Failed to allocate page");
            let addr = Virtual::new(addr);

            loop {
                match arch::mmu::map(
                    table,
                    addr,
                    frame,
                    arch::mmu::Rights::RWX | arch::mmu::Rights::USER,
                    arch::mmu::Flags::empty(),
                ) {
                    Ok(_) => break,
                    Err(arch::mmu::MapError::FrameConsumed) => {
                        frame = memory
                            .allocate_zeroed_page()
                            .expect("Failed to allocate page");
                    }
                    Err(_) => {
                        panic!("Failed to map page")
                    }
                }
            }

            // Copy the data
            // TODO: Disable write protection for the duration of the copy :
            // the kernel cannot access user-space memory, it probably needs
            // a flags to allow this.
            let len = core::cmp::min(4096, file_end.saturating_sub(usize::from(addr)));
            core::ptr::copy_nonoverlapping(
                init.as_ptr().wrapping_add(file_offset + section_offset),
                arch::mmu::translate_physical(frame)
                    .unwrap()
                    .as_mut_ptr::<u8>(),
                len,
            )
        }
    }

    arch::thread::execute(&mut thread);
    log::debug!("Thread trapped back to kernel");
    loop {}
}
