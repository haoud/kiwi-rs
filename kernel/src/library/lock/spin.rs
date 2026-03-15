use core::{
    cell::UnsafeCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::arch;

/// A simple spinlock implementation that provides interior mutability and can
/// be safely shared across thread boundaries as long as `T` is `Send`.
pub struct Spinlock<T: ?Sized> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

/// SAFETY: A `Spinlock<T>` provides interior mutability and can be safely
/// shared across thread boundaries as long as `T` is `Send` since the spinlock
/// ensures that only one thread can access the data at a time, preventing any
/// data races or undefined behavior if multiple threads access the same
/// `Spinlock<T>` concurrently.
unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}

/// SAFETY: A `Spinlock<T>` can be sent across thread boundaries as long as `T`
/// is `Send` since a spinlock can be safely transferred to another thread
/// without issues if the data it protects can also be safely transferred.
unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}

impl<T> Spinlock<T> {
    /// Create a new unlocked `Spinlock` containing the given data.
    #[must_use]
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> Spinlock<T> {
    /// Check if the lock is currently held or not. Due to the relaxed memory
    /// ordering, the result of this function should be treated as outdated
    /// immediately after it is returned, and should not be used for
    /// synchronization purposes, but rather as a hint for optimization.
    #[must_use]
    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    /// Force unlock this [`Spinlock`].
    ///
    /// # Safety
    ///
    /// This is *extremely* unsafe if the lock is not held by the current
    /// thread, but can be useful in some instances for exposing the lock
    /// to FFI that doesn't know how to deal with RAII.
    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }

    /// Try to acquire the lock and return a guard that provides access to the
    /// protected data if successful. If the lock is already held, this method
    /// returns `None` immediately without blocking.
    pub fn try_lock(&self) -> Option<SpinlockGuard<'_, T>> {
        // Attempt to acquire the lock by updating atomically from `false` to
        // `true` if the current value is `false`. A weak compare-and-exchange
        // is used here to allow for spurious failures even if the lock is not
        // currently held, but can improve performance on some platforms.
        if self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinlockGuard {
                // SAFETY: We have successfully acquired the lock meaning that
                // we have exclusive access to the data: it is safe to create a
                // mutable reference to the inner data.
                data: unsafe { self.data.as_mut_unchecked() },
                lock: &self.lock,
            })
        } else {
            None
        }
    }

    /// Try to acquire the lock while also disabling interrupts to prevent
    /// deadlocks if the lock can be acquired by an interrupt handler. If the
    /// lock is already held, this method returns `None` immediately without
    /// blocking and restores the previous interrupt state.
    pub fn try_lock_irq_safe(&self) -> Option<SpinlockGuardIrqSafe<'_, T>> {
        // Attempt to acquire the lock by updating atomically from `false` to
        // `true` if the current value is `false`. A weak compare-and-exchange
        // is used here to allow for spurious failures even if the lock is not
        // currently held, but can improve performance on some platforms.
        let irq_guard = arch::irq::IrqGuard::new();
        if self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinlockGuardIrqSafe {
                // SAFETY: We have successfully acquired the lock meaning that
                // we have exclusive access to the data: it is safe to create a
                // mutable reference to the inner data.
                data: unsafe { self.data.as_mut_unchecked() },
                lock: &self.lock,
                irq_guard,
            })
        } else {
            None
        }
    }

    /// Locks the [`Spinlock`] and returns a guard that permits an exclusive
    /// access to the inner data. The lock will be automatically released when
    /// the guard will fall out of scope.
    ///
    /// If the lock is currently held by another thread, this method will loop
    /// until the lock becomes available, wasting precious CPU cycles in the
    /// meantime.
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_lock() {
                return guard;
            }

            while self.lock.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
    }

    /// Locks the [`Spinlock`] and returns a guard that permits an exclusive
    /// access to the inner data and can be safely used in interrupt handlers
    /// by disabling interrupts during the duration of the lock. In order to be
    /// effective, the `*_irq_safe` methods should be used in all code paths, even
    /// those that are not in interrupt context.
    ///
    /// The lock will be automatically released and the previous interrupt
    /// state will be restored when the guard will fall out of scope.
    ///
    /// If the lock is currently held by another thread, this method will loop
    /// until the lock becomes available, wasting precious CPU cycles in the
    /// meantime.
    pub fn lock_irq_safe(&self) -> SpinlockGuardIrqSafe<'_, T> {
        loop {
            if let Some(guard) = self.try_lock_irq_safe() {
                return guard;
            }

            while self.lock.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
    }

    /// Locks the [`Spinlock`] and executes the given closure with a mutable
    /// reference to the inner data. The lock will be automatically released
    /// when the closure returns.
    ///
    /// This method is a convenient wrapper around `lock` that allows for a
    /// more precise control over the scope of the lock, and can help to avoid
    /// accidentally holding the lock for longer than necessary.
    pub fn with_lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut guard = self.lock();
        f(&mut *guard)
    }

    /// Locks the [`Spinlock`] and executes the given closure with a mutable
    /// reference to the inner data while also disabling interrupts to prevent
    /// deadlocks if the lock can be acquired by an interrupt handler. The lock
    /// will be automatically released and the previous interrupt state will be
    /// restored when the closure returns.
    ///
    /// This method is a convenient wrapper around `lock_irq_safe` that allows
    /// for a more precise control over the scope of the lock, and can help to
    /// avoid accidentally holding the lock for longer than necessary.
    pub fn with_lock_irq_safe<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut guard = self.lock_irq_safe();
        f(&mut *guard)
    }
}

impl<T: ?Sized + Debug> Debug for Spinlock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Spinlock {{ data: ")
                .and_then(|()| guard.fmt(f))
                .and_then(|()| write!(f, " }}")),
            None => write!(f, "Spinlock {{ <locked> }}"),
        }
    }
}

impl<T: Default> Default for Spinlock<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: ?Sized> Drop for Spinlock<T> {
    fn drop(&mut self) {}
}

#[must_use = "Locking a spinlock without using the guard is likely a mistake since the \
    lock will be immediately released when the guard is dropped"]
pub struct SpinlockGuard<'a, T: ?Sized> {
    lock: &'a AtomicBool,
    data: &'a mut T,
}

impl<T: ?Sized> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T: ?Sized> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<T: ?Sized> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

#[must_use = "Locking a spinlock without using the guard is likely a mistake since the \
    lock will be immediately released when the guard is dropped"]
pub struct SpinlockGuardIrqSafe<'a, T: ?Sized> {
    lock: &'a AtomicBool,
    data: &'a mut T,

    #[allow(unused)]
    irq_guard: arch::irq::IrqGuard,
}

impl<T: ?Sized> Deref for SpinlockGuardIrqSafe<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T: ?Sized> DerefMut for SpinlockGuardIrqSafe<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<T: ?Sized> Drop for SpinlockGuardIrqSafe<'_, T> {
    fn drop(&mut self) {
        // Release the lock and restore the previous interrupt state by
        // dropping the `IrqGuard` after the lock is released.
        self.lock.store(false, Ordering::Release);
    }
}
