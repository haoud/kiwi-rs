//! The memory displayed by the memory dump panel of the console.

unsafe extern "C" {
    /// The start of the read-only data of the kernel, defined by the linker
    /// script.
    static __rodata_start: u8;

    /// The end of the read-only data of the kernel, defined by the linker
    /// script.
    static __rodata_end: u8;
}

/// Returns the offset of the first text found in the given memory, to start a
/// dump on something more interesting than tables of pointers: the first
/// block of 256 bytes made of at least 75% of printable characters.
pub fn first_text(memory: &[u8]) -> usize {
    const BLOCK: usize = 256;
    memory
        .chunks(BLOCK)
        .position(|block| {
            let printable = block.iter().filter(|b| b.is_ascii_graphic() || **b == b' ');
            printable.count() * 4 >= block.len() * 3
        })
        .map_or(0, |index| index * BLOCK)
}

/// Returns the read-only data section of the kernel. It contains varied data
/// (strings, lookup tables, the embedded fonts...) and is a safe region to
/// dump: it is always mapped, and it is never modified.
pub fn kernel_rodata() -> &'static [u8] {
    let start = &raw const __rodata_start;
    let end = &raw const __rodata_end;
    let len = (end as usize).saturating_sub(start as usize);

    // SAFETY: The linker script defines both symbols around the `.rodata`
    // section, which is mapped for the whole lifetime of the kernel and never
    // written to, so it can be shared as an immutable slice.
    unsafe { core::slice::from_raw_parts(start, len) }
}
