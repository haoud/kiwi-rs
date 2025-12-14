use crate::{arch::mmu, utils::align::Aligned};

pub mod frame;
pub mod phys;
pub mod virt;

pub use frame::{Frame, Frame1Gib, Frame2Mib, Frame4Kib};
pub use phys::Physical;
pub use virt::Virtual;

/// A value that is page aligned
pub type PageAligned<T> = Aligned<T, { mmu::PAGE_SIZE }>;
