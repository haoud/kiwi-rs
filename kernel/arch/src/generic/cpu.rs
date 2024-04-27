/// Relaxes the CPU and wait for the next event to happen. On most architectures, this
/// wait for the next interrupt to happen. This allow the CPU to save power and reduce
/// heat.
///
/// Although this function seems nice, it should be used with caution because on
/// some architectures (like x86), it will enable interrupts in order to wait for the
/// next interrupt to happen and to not be stuck in an infinite loop. This can cause
/// some unexpected behavior if the caller is not prepared for it.
pub fn relax() {
    crate::target::cpu::relax();
}
