/// For now, use the spinlock implementation from the `spin` crate. In the
/// future, we may want to implement our own spinlock that can disable
/// interrupts while holding the lock, but for now this will suffice.
pub type Spinlock<T> = spin::Mutex<T>;
