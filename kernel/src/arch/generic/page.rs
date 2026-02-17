/// Translate a physical address to a virtual address.
#[must_use]
pub fn translate(physical: usize) -> usize {
    crate::arch::target::page::translate(physical)
}
