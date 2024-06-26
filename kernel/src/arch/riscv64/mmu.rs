//! RISC-V64 Memory Management Unit implementation. Currently, this
//! implementation only handle SV39 paging, which should be supported by all
//! RISC-V64 systems and should be enough for most use cases. However, it is
//! possible to add support for other paging modes in the future.
use super::addr::{
    self, virt::Kernel, Frame1Gib, Frame4Kib, Physical, Virtual,
};
use crate::{
    arch::mmu::{Flags, MapError, Rights, UnmapError},
    pmm::{self, AllocationFlags},
};
use bitflags::bitflags;
use core::ops::{Index, IndexMut};
use usize_cast::IntoUsize;

/// The kernel's page table. This table is used by the kernel to identity
/// map the physical memory of the system, allowing the kernel to easily
/// access the physical memory of the system.
static KERNEL_TABLE: spin::Mutex<Table> = spin::Mutex::new(Table::empty());

/// The virtual address where the kernel address space starts. Since we are
/// developing a micro-kernel, allowing the kernel to access only the first
/// 1 GiB of physical memory in the last 1 GiB of virtual memory should be
/// enough and have the nice side effect of making the kernel's address space
/// independent of the pagination mode used by the processor (SV39, SV48, etc).
pub const KERNEL_VIRTUAL_BASE: Virtual<Kernel> =
    Virtual::<Kernel>::new(0xFFFF_FFFF_C000_0000);

/// The physical address where the RAM starts. This address will be mapped
/// to the kernel's address space at the address defined by
/// `KERNEL_VIRTUAL_BASE`.
pub const KERNEL_PHYSICAL_BASE: Frame1Gib =
    unsafe { Frame1Gib::new_unchecked(Physical::new(0x8000_0000)) };

/// The start of ther kernel's address space. This corresponds to the first
/// address after the 'canonical hole' in the virtual address space and goes
/// up to the last address of the virtual address space.
pub const KERNEL_START: Virtual<Kernel> =
    Virtual::<Kernel>::new(0xFFFF_FFC0_0000_0000);

/// The size of a page in bytes.
pub const PAGE_SIZE: usize = 4096;

/// The shift required to convert a byte address to a page address.
pub const PAGE_SHIFT: usize = 12;

/// Represents a page table. A page table is a data structure used by the
/// processor to translate virtual addresses to physical addresses. The page
/// table is composed of multiple levels, each level containing a number of
/// entries that point to the next level of the page table or to a physical
/// address. The number of levels and the number of entries per level depend
/// on the processor and the paging mode used.
///
/// Currently, Kiwi only supports the SV39 paging mode, which uses 3 levels
/// of page tables and 512 entries per level.
///
/// To determine if a level of the page table is a leaf level or an
/// intermediate level, the processor checks if any of the read, write
/// or execute bits are set in the entry. If any of these bits are set,
/// the level is a leaf level.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(align(4096))]
pub struct Table([Entry; 512]);

impl Table {
    /// Create a new empty page table. An empty page table is a table where
    /// all entries are missing, meaning that they do not point to a physical
    /// address and are not present in the page table.
    ///
    /// # Warning
    /// This function should only be used to initialize static variables !
    /// Trying to create a new empty page table at runtime will result in a
    /// stack overflow, as the stack is 4 KiB large by default.
    #[must_use]
    pub const fn empty() -> Self {
        Self([Entry::missing(); 512])
    }

    /// Set all the user-accessible entries of the table to zero and copy the
    /// kernel address space into the table.
    ///
    /// If any of the user-accessible entries was pointer to a valid table or
    /// a valid page, this will lead to the memory leak of the entire table
    /// and its sub-tables or pages.
    pub fn setup_from_kernel_space(&mut self) {
        let table = KERNEL_TABLE.lock();
        for i in 0..256 {
            self[i] = Entry::missing();
        }
        for i in 256..512 {
            self[i] = table[i];
        }
    }

    /// Set the current page table to this table.
    ///
    /// # Safety
    /// This function is unsafe because it can cause undefined behavior if
    /// the table is not properly initialized. The caller must ensure that
    /// the table given will not cause an instant page fault when set as
    /// the current page table, and must ensure that the table will remain
    /// in memory while it is set as the current page table.
    pub unsafe fn set_current(&self) {
        let ppn = translate_kernel_ptr(self).as_usize() >> 12;
        riscv::register::satp::set(riscv::register::satp::Mode::Sv39, 0, ppn);
        riscv::asm::sfence_vma_all();
    }
}

impl Index<usize> for Table {
    type Output = Entry;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Table {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

/// A page table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Entry(u64);

impl Entry {
    /// Create a new entry that is missing, meaning that it does not point to a
    /// physical address and is not present in the page table.
    #[must_use]
    pub const fn missing() -> Self {
        Self(0)
    }

    /// Create a new entry that points to a physical address. However, this
    /// entry will not have any flags set, meaning that it is not present in
    /// the page table and trying to access it will raise an exception.
    ///
    /// Furthermore, the physical address must be properly aligned, depending
    /// on the level of the page table that this entry is part of. For example,
    /// if this entry is part of the first level of the page table, the
    /// physical address must be aligned to 0x1000 (4 KiB). If this entry is
    /// part of the second level of the page table, the physical address must
    /// be aligned to 0x200000 (2 MiB). And if this entry is part of the third
    /// level of the page table, the physical address must be aligned to
    /// 0x40000000 (1 GiB).
    #[must_use]
    pub const fn new(frame: Frame4Kib) -> Self {
        Self((frame.inner().as_u64() & !0x3FF) >> 2)
    }

    /// Set the access rights of the entry.
    pub fn set_rights(&mut self, rights: Rights) {
        self.set_user(rights.contains(Rights::USER));
        self.set_readable(rights.contains(Rights::READ));
        self.set_writable(rights.contains(Rights::WRITE));
        self.set_executable(rights.contains(Rights::EXECUTE));
    }

    /// Set the flags of the entry.
    pub fn set_flags(&mut self, flags: Flags) {
        self.set_global(flags.contains(Flags::GLOBAL));
    }

    /// Set or clear the present bit of the entry. If this bit is set, the
    /// page is mapped to a physical address. If this bit is not set, the page
    /// is not mapped to a physical address and trying to access it will raise
    /// an exception.
    pub fn set_present(&mut self, present: bool) {
        if present {
            self.0 |= EntryFlags::PRESENT.bits();
        } else {
            self.0 &= !EntryFlags::PRESENT.bits();
        }
    }

    /// Set or clear the readable bit of the entry. If this bit is set, the
    /// page can be read from by the processor. If this bit is not set, the
    /// page cannot be read from by the processor. Trying to read from a page
    /// that is not readable will raise an exception.
    pub fn set_readable(&mut self, readable: bool) {
        if readable {
            self.0 |= EntryFlags::READABLE.bits();
        } else {
            self.0 &= !EntryFlags::READABLE.bits();
        }
    }

    /// Set or clear the writable bit of the entry. If this bit is set, the
    /// page can be written to by the processor. If this bit is not set, the
    /// page cannot be written to by the processor. Trying to write to a page
    /// that is not writable will raise an exception.
    pub fn set_writable(&mut self, writable: bool) {
        if writable {
            self.0 |= EntryFlags::WRITABLE.bits();
        } else {
            self.0 &= !EntryFlags::WRITABLE.bits();
        }
    }

    /// Set or clear the executable bit of the entry. If this bit is set, the
    /// page can be executed by the processor. If this bit is not set, the page
    /// cannot be executed by the processor. Trying to execute a page that is
    /// not executable will raise an exception.
    pub fn set_executable(&mut self, executable: bool) {
        if executable {
            self.0 |= EntryFlags::EXECUTABLE.bits();
        } else {
            self.0 &= !EntryFlags::EXECUTABLE.bits();
        }
    }

    /// Set or clear the user bit of the entry. If this bit is set, the entry
    /// can be accessed by the user mode of the processor. If this bit is not
    /// set, the entry can only be accessed by the supervisor or machine mode
    /// of the processor.
    pub fn set_user(&mut self, user: bool) {
        if user {
            self.0 |= EntryFlags::USER.bits();
        } else {
            self.0 &= !EntryFlags::USER.bits();
        }
    }

    /// Set or clear the global bit of the entry. If this bit is set, the
    /// entry will not be flushed from the TLB when changing the address
    /// space. This bit should only be set if the page is shared between all
    /// address spaces, otherwise it may lead to security issues or strange
    /// bugs that will be very, very hard to debug.
    pub fn set_global(&mut self, global: bool) {
        if global {
            self.0 |= EntryFlags::GLOBAL.bits();
        } else {
            self.0 &= !EntryFlags::GLOBAL.bits();
        }
    }

    /// Set or clear the accessed bit of the entry.
    pub fn set_accessed(&mut self, accessed: bool) {
        if accessed {
            self.0 |= EntryFlags::ACCESSED.bits();
        } else {
            self.0 &= !EntryFlags::ACCESSED.bits();
        }
    }

    /// Set or clear the dirty bit of the entry.
    pub fn set_dirty(&mut self, dirty: bool) {
        if dirty {
            self.0 |= EntryFlags::DIRTY.bits();
        } else {
            self.0 &= !EntryFlags::DIRTY.bits();
        }
    }

    /// Set the physical address that the entry points to. The physical address
    /// must be properly aligned, depending on the level of the page table that
    /// this entry is part of.
    pub fn set_address<T: Into<Frame4Kib>>(&mut self, frame: T) {
        self.0 &= 0x3FF;
        self.0 |= (frame.into().inner().as_u64() & !0xFFF) >> 2;
    }

    /// Check if the entry is present, meaning that the page is mapped to a
    /// physical address. If this bit is not set, the page is not mapped to
    /// a physical address and trying to access it will raise an exception.
    #[must_use]
    pub fn present(&self) -> bool {
        self.0 & EntryFlags::PRESENT.bits() != 0
    }

    /// Check if the entry is readable, meaning that the page can be read from
    /// by the processor. If this bit is not set, the page cannot be read from
    /// by the processor. Trying to read from a page that is not readable will
    /// raise an exception.
    #[must_use]
    pub fn readable(&self) -> bool {
        self.0 & EntryFlags::READABLE.bits() != 0
    }

    /// Check if the entry is writable, meaning that the page can be written to
    /// by the processor. If this bit is not set, the page cannot be written to
    /// by the processor. Trying to write to a page that is not writable will
    /// raise an exception.
    #[must_use]
    pub fn writable(&self) -> bool {
        self.0 & EntryFlags::WRITABLE.bits() != 0
    }

    /// Check if the entry is executable, meaning that the page can be executed
    /// by the processor. If this bit is not set, the page cannot be executed
    /// by the processor. Trying to execute a page that is not executable will
    /// raise an exception.
    #[must_use]
    pub fn executable(&self) -> bool {
        self.0 & EntryFlags::EXECUTABLE.bits() != 0
    }

    /// Check if the entry is accessible by the user, meaning that it can be
    /// accessed by the user mode of the processor. If this bit is not set,
    /// the entry can only be accessed by the supervisor or machine mode of
    /// the processor.
    #[must_use]
    pub fn user(&self) -> bool {
        self.0 & EntryFlags::USER.bits() != 0
    }

    /// Check if the entry is global, meaning that it is not flushed from the
    /// TLB when changing the address space.
    #[must_use]
    pub fn global(&self) -> bool {
        self.0 & EntryFlags::GLOBAL.bits() != 0
    }

    /// Check if the entry was accessed, meaning that the page was read from
    /// or written to. This bit is set by the processor when a read access is
    /// made to the page, but is never cleared by the processor: it must be
    /// cleared by the OS.
    #[must_use]
    pub fn accessed(&self) -> bool {
        self.0 & EntryFlags::ACCESSED.bits() != 0
    }

    /// Check if the entry is dirty, meaning that the page was written to. This
    /// bit is set by the processor when a write access is made to the page,
    /// but is never cleared by the processor: it must be cleared by the OS.
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.0 & EntryFlags::DIRTY.bits() != 0
    }

    /// Return the physical address that the entry points to. This method does
    /// not check if the entry is present, and calling this method on an entry
    /// that is not present will return incorrect results.
    #[must_use]
    pub fn address(&self) -> Physical {
        // SAFETY: This is safe because the table entry cannot physically
        // contains an invalid physical address (not enough bits to have
        // an address greater than [`Physical::MAX`])
        unsafe { Physical::new_unchecked((self.0.into_usize() & !0x3FF) << 2) }
    }

    /// Check if the entry is a leaf entry, meaning that it points to a
    /// physical address and not to another table.
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        self.readable() | self.writable() | self.executable()
    }

    /// Clear the entry, meaning that it does not point to a physical address
    /// and does not have any flags set.
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Get the next table from the entry. If the entry is a leaf entry or is
    /// not present, this method will return `None`.
    ///
    /// # Safety
    /// This function assume that the entry points to a valid physical address
    /// that will be translated to a valid virtual address that contains a
    /// valid table.
    ///
    /// If the address is not valid or points to another object, the behavior
    /// is undefined and may lead to memory corruption or data loss.
    ///
    /// # Panics
    /// Panics if the physical address in the table cannot be translated to a
    /// virtual address. This should not happens even in the SV39 paging mode,
    /// as this would require a machine with more than 128 GiB of RAM, which
    /// is not supported by Kiwi.
    #[must_use]
    pub unsafe fn next_table_mut(&self) -> Option<&mut Table> {
        if self.is_leaf() || !self.present() {
            None
        } else {
            let table = translate_physical(self.address())
                .expect("Failed to translate table physical address")
                .as_mut_ptr::<Table>();
            Some(&mut *(table))
        }
    }
}

bitflags! {
    /// A set of flags that can be used to control the behavior of a virtual
    /// memory region.
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EntryFlags: u64 {
        /// The entry point to an valid physical address.
        const PRESENT = 1 << 0;

        /// The entry is readable.
        const READABLE = 1 << 1;

        /// The entry is writable.
        const WRITABLE = 1 << 2;

        /// The entry is executable.
        const EXECUTABLE = 1 << 3;

        /// The entry is accessible by the user mode.
        const USER = 1 << 4;

        /// The entry is global and should not be flushed from the TLB.
        /// This must be only used if a page is shared between all address
        /// spaces, otherwise it may lead to security issues or strange bugs
        /// that will be very, very hard to debug.
        const GLOBAL = 1 << 5;

        /// The entry was accessed. This bit is set by the processor when a
        /// read access is made to the page, but is never cleared by the
        /// processor: it must be cleared by the OS.
        const ACCESSED = 1 << 6;

        /// The entry was written to. This bit is set by the processor when a
        /// write access is made to the page, but is never cleared by the
        /// processor: it must be cleared by the OS.
        const DIRTY = 1 << 7;
    }
}

/// Setup the MMU. This will create a kernel page table that identity maps the
/// first 256 GiB of physical memory to the first 256 GiB of virtual memory.
/// This will allow the kernel to access the physical memory of the system
/// without having to manually map each page.
pub fn setup() {
    log::info!("Initializing the MMU and remapping the kernel");
    log::debug!("Using SV39 paging mode (3 levels of page tables)");
    log::debug!(
        "User address space :   0x0000000000000000 - 0x00007FFFFFFFFFFF"
    );
    log::debug!(
        "Kernel address space : 0xFFFFFFFFC0000000 - 0xFFFFFFFFFFFFFFFF"
    );
    let mut table = KERNEL_TABLE.lock();

    // Map the first 255 GiB of physical memory to the first 255 GiB
    // of virtual memory in the kernel's address space. This will allow
    // the kernel to access any physical address easily without having
    // to manually map each page.
    for i in 256..511 {
        let entry = &mut table[i];
        entry.set_address(Frame1Gib::new(Physical::new(
            (i - 256) * 0x4000_0000,
        )));
        entry.set_executable(true);
        entry.set_writable(true);
        entry.set_readable(true);
        entry.set_present(true);
    }

    // Map the kernel to the last 1 GiB of virtual memory.
    let entry = &mut table[511];
    entry.set_address(KERNEL_PHYSICAL_BASE);
    entry.set_executable(true);
    entry.set_writable(true);
    entry.set_readable(true);
    entry.set_present(true);

    // SAFETY: The kernel table was properly initialized and will not cause
    // a page fault when set as the current page table.
    unsafe {
        table.set_current();
    }
}

/// Map a physical address to a virtual address.
///
/// # Errors
/// This function will return an error if any of the following conditions
/// are met:
/// - The virtual address is already mapped to a physical address.
/// - An intermediate table is missing and the kernel is unable to
///   allocate a new table.
///
/// # Panics
/// Panics if an error occurs while traversing the page table. This should
/// never happen, as the page table should be properly initialized.
pub fn map<T: addr::virt::Type>(
    root: &mut Table,
    virt: Virtual<T>,
    frame: Frame4Kib,
    rights: Rights,
    flags: Flags,
) -> Result<(), MapError> {
    // TODO: Huge pages support

    // Extract the VPNs from the virtual address.
    let vpn = [
        (virt.as_usize() >> 12) & 0x1FF,
        (virt.as_usize() >> 21) & 0x1FF,
        (virt.as_usize() >> 30) & 0x1FF,
    ];

    let mut entry = &mut root[vpn[2]];
    for i in (0..2).rev() {
        if !entry.present() {
            // Allocate a new zeroed frame with the kernel flag set and
            // create a new leaf entry.
            let flags = AllocationFlags::KERNEL | AllocationFlags::ZEROED;
            let frame =
                pmm::allocate_frame(flags).ok_or(MapError::OutOfMemory)?;

            entry.clear();
            entry.set_address(frame);
            entry.set_present(true);
        }

        // Get the next table from the current entry and continue the traversal
        let table = unsafe { entry.next_table_mut().unwrap() };
        entry = &mut table[vpn[i]];
    }

    if entry.present() {
        return Err(MapError::AlreadyMapped);
    }

    // Update the entry with the physical address, rights and flags given.
    entry.set_address(frame);
    entry.set_present(true);
    entry.set_rights(rights);
    entry.set_flags(flags);
    Ok(())
}

/// Unmap a virtual address, returning the physical address that was previously
/// mapped to it.
///
/// # Errors
/// This function will return an error if the virtual address is not mapped to
/// a physical address.
///
/// # Panics
/// Panics if an error occurs while traversing the page table. This should
/// never happen, as the page table should be properly initialized.
pub fn unmap<T: addr::virt::Type>(
    root: &mut Table,
    virt: Virtual<T>,
) -> Result<Physical, UnmapError> {
    // Extract the VPNs from the virtual address.
    let vpn = [
        (virt.as_usize() >> 12) & 0x1FF,
        (virt.as_usize() >> 21) & 0x1FF,
        (virt.as_usize() >> 30) & 0x1FF,
    ];

    let mut entry = &mut root[vpn[2]];
    for i in (0..2).rev() {
        if !entry.present() {
            return Err(UnmapError::NotMapped);
        }

        if entry.is_leaf() {
            break;
        }

        if i == 0 {
            return Err(UnmapError::NotMapped);
        }

        let table = unsafe { entry.next_table_mut().unwrap() };
        entry = &mut table[vpn[i]];
    }

    // Get the physical address that was previously mapped to the given virtual
    // address, unmap it, flush the TLB and return the physical address.
    let address = entry.address();
    entry.clear();
    // TODO: Flush the TLB
    Ok(address)
}

/// Translate a physical address to a virtual address. If the translation
/// cannot be done because the physical address is greater than the maximum
/// virtual address representable by the system, this function will return
/// `None`.
#[must_use]
pub fn translate_physical(
    phys: impl Into<Physical>,
) -> Option<Virtual<Kernel>> {
    Some(Virtual::<Kernel>::new(
        usize::from(KERNEL_START) + usize::from(phys.into()),
    ))
}

/// Translate a virtual address in the kernel's address space to a physical
/// address.
///
/// # Panics
/// Panics if the virtual address is not located in the kernel's address space,
/// i.e. if it is not greater than or equal to `KERNEL_START`.
#[must_use]
pub fn translate_virtual_kernel(virt: Virtual<Kernel>) -> Physical {
    if virt >= KERNEL_VIRTUAL_BASE {
        Physical::new(
            usize::from(virt) - usize::from(KERNEL_VIRTUAL_BASE)
                + KERNEL_PHYSICAL_BASE.as_usize(),
        )
    } else {
        Physical::new(usize::from(virt) - usize::from(KERNEL_START))
    }
}

/// Translate a kernel pointer to a physical address.
///
/// # Panics
/// Panics if the pointer is not located in the kernel's address space,
/// i.e. if it is not greater than or equal to `KERNEL_VIRTUAL_BASE`.
#[must_use]
pub fn translate_kernel_ptr<T>(ptr: *const T) -> Physical {
    translate_virtual_kernel(Virtual::<Kernel>::new(ptr as usize))
}
