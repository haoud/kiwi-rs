use super::{PageAligned, Physical};
use crate::utils::align::Aligned;

/// A frame of memory. On riscv64, this can be a 4Kib, 2Mib, or 1Gib frame,
/// assuming a Sv39 paging scheme. Greater frame sizes may be available with
/// other paging schemes, but are not currently supported.
pub enum Frame {
    Frame4Kib(Frame4Kib),
    Frame2Mib(Frame2Mib),
    Frame1Gib(Frame1Gib),
}

/// A 4Kib frame
pub type Frame4Kib = PageAligned<Physical>;

impl Frame4Kib {
    /// The size of a 4Kib frame in bytes.
    pub const SIZE: usize = 4096;
}

impl From<Physical> for Frame4Kib {
    fn from(value: Physical) -> Self {
        Frame4Kib::new(value)
    }
}

impl From<Frame2Mib> for Frame4Kib {
    fn from(frame: Frame2Mib) -> Self {
        Frame4Kib::new(*frame)
    }
}

impl From<Frame1Gib> for Frame4Kib {
    fn from(frame: Frame1Gib) -> Self {
        Frame4Kib::new(*frame)
    }
}

impl From<Frame4Kib> for Physical {
    fn from(frame: Frame4Kib) -> Self {
        frame.into_inner()
    }
}

/// A 2Mib frame
pub type Frame2Mib = Aligned<Physical, { 4096 * 512 }>;

impl Frame2Mib {
    /// The size of a 2Mib frame in bytes.
    pub const SIZE: usize = 4096 * 512;
}

impl From<Physical> for Frame2Mib {
    fn from(value: Physical) -> Self {
        Frame2Mib::new(value)
    }
}

impl From<Frame4Kib> for Frame2Mib {
    fn from(frame: Frame4Kib) -> Self {
        Frame2Mib::new(*frame)
    }
}

impl From<Frame1Gib> for Frame2Mib {
    fn from(frame: Frame1Gib) -> Self {
        Frame2Mib::new(*frame)
    }
}

impl From<Frame2Mib> for Physical {
    fn from(frame: Frame2Mib) -> Self {
        frame.into_inner()
    }
}

/// A 1 Gib frame
pub type Frame1Gib = Aligned<Physical, { 4096 * 512 * 512 }>;

impl Frame1Gib {
    /// The size of a 1 Gib frame in bytes.
    pub const SIZE: usize = 4096 * 512 * 512;
}

impl From<Physical> for Frame1Gib {
    fn from(value: Physical) -> Self {
        Frame1Gib::new(value)
    }
}

impl From<Frame4Kib> for Frame1Gib {
    fn from(frame: Frame4Kib) -> Self {
        Frame1Gib::new(*frame)
    }
}

impl From<Frame2Mib> for Frame1Gib {
    fn from(frame: Frame2Mib) -> Self {
        Frame1Gib::new(*frame)
    }
}

impl From<Frame1Gib> for Physical {
    fn from(frame: Frame1Gib) -> Self {
        frame.into_inner()
    }
}
