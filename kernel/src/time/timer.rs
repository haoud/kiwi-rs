use core::sync::atomic::{AtomicBool, Ordering};

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use macros::init;

use crate::{
    library::lock::spin::Spinlock,
    time::{duration::Duration, instant::Instant},
};

/// The callback type for timers. It is a boxed closure that takes a mutable
/// reference to the timer that is being executed, allowing the callback to
/// modify the timer's state (e.g. rearm it with a new deadline) if needed.
///
/// The `Send` bound is required to allow the callback to be executed in an
/// interrupt context, which may be on a different thread than the one that
/// registered the timer.
pub type TimerCallback = Box<dyn FnMut(&mut Timer) + Send>;

/// The mode of a timer, which determines whether it is a one-shot timer or a
/// periodic timer. This will affect the behavior of the timer after its callback
/// is executed: A one-shot timer will be dropped, while a periodic timer will
/// be rearmed with a new deadline and registered again with the timer manager to
/// execute its callback again when it expires.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TimerMode {
    /// A one-shot timer that will execute its callback once when it expires and
    /// then be removed from the active timers list. This is the default mode
    /// for timers.
    #[default]
    OneShot,

    /// A periodic timer that will execute its callback every time it expires,
    /// and then be rearmed with a new deadline based on the timer's original
    /// deadline and the current time. This allows the timer to maintain a
    /// consistent period even if there are delays in executing the callback or
    /// in handling expired timers.
    Periodic(Duration),
}

/// A timer that can be registered with the timer manager to execute a callback
/// at a specific deadline.
///
/// The timer callback will never be executed before the timer's deadline, but
/// a small delay after the deadline is possible due to the kernel's timer
/// resolution and scheduling.
///
/// The timer can be cancelled by dropping its guard, which will prevent the
/// callback from being executed even if the timer has expired.
pub struct Timer {
    /// The deadline of this timer, represented as an `Instant`. When
    /// the current time reaches or exceeds this deadline, the timer is
    /// considered expired and its associated callback should be executed.
    pub deadline: Instant,

    /// The guard of this timer, which will cancel the timer when dropped. If
    /// the guard is dropped, the timer will be cancelled and its callback will
    /// not be executed even if the timer has expired.
    guard: TimerGuard,

    /// The mode of this timer, which determines whether it is a one-shot timer
    /// or a periodic timer.
    mode: TimerMode,

    /// The callback function to execute when this timer expires.
    callback: Option<TimerCallback>,
}

impl Timer {
    /// Creates a new timer with the given deadline and callback. The timer is
    /// active upon creation, but must be registered with the timer manager to
    /// be executed when it expires.
    #[must_use]
    pub fn new(deadline: Instant, mode: TimerMode, callback: TimerCallback) -> Self {
        Self {
            guard: TimerGuard::default(),
            mode,
            deadline,
            callback: Some(callback),
        }
    }

    /// Consumes this timer and executes its callback, without checking whether
    /// the timer has expired or is still active.
    pub fn run_callback(mut self) {
        if let Some(mut callback) = self.callback.take() {
            if self.guard.is_active() {
                self.guard.cancel();
                (callback)(&mut self);

                if let TimerMode::Periodic(period) = self.mode {
                    self.guard.rearm();
                    self.deadline += period;
                    self.callback = Some(callback);
                    register(self).ignore();
                } else if self.guard.is_active() {
                    // If the timer guard is still active after running the
                    // callback, it means that the timer was rearmed by the
                    // callback, so we need to register it again with the timer
                    // manager to ensure that it will be executed again when it
                    // expires
                    self.callback = Some(callback);
                    register(self).ignore();
                }
            }
        } else {
            log::warn!("Called run_callback on a timer with no callback !");
        }
    }

    /// Rearms the timer. This can be used to reactivate a timer that has been
    /// cancelled or to update the timer's deadline in the timer callback.
    /// This does not change the timer's deadline nor insert the timer back
    /// into the active timers list, it is the caller's responsibility to do
    /// this.
    pub fn rearm(&mut self) {
        self.guard.rearm();
    }

    /// Verifies whether this timer has expired by comparing the current time
    /// with the timer's deadline. If the current time is greater than or equal
    /// to the deadline, this function returns `true`, indicating that the
    /// timer has expired and its callback should be executed, otherwise it
    /// returns `false`.
    #[must_use]
    pub fn expired(&self) -> bool {
        Instant::now() >= self.deadline
    }

    /// Verifies whether this timer is active by checking the state of its
    /// guard. If a guard has been dropped, the timer is considered inactive
    /// and should not be executed even if it has expired.
    #[must_use]
    pub fn active(&self) -> bool {
        self.guard.is_active()
    }
}

/// A guard that will cancel the timer when dropped. It can be cloned to create
/// multiple guards that will all cancel the timer when dropped. If one guard
/// is dropped, the corresponding timer will be cancelled even if other guards
/// are still alive.
#[derive(Debug, Clone)]
#[must_use = "Dropping a TimerGuard will cancel the associated timer, which is likely \
not what you want. Use TimerGuard::ignore() to prevent this if you want to drop the \
guard without cancelling the timer."]
pub struct TimerGuard {
    /// The atomic boolean that will be set to false when the timer is cancelled. It is
    /// shared with the timer and with all the guards that have been cloned from the
    /// original guard.
    active: Option<Arc<AtomicBool>>,

    /// Set to true if the guard should not cancel the timer when dropped,
    /// allowing to drop the guard without cancelling the timer. This is not
    /// shared between clones, so it only affects this guard and not any
    /// clones of it.
    ignore_drop: bool,
}

impl TimerGuard {
    /// Creates a new dummy guard that is not active and will not cancel any
    /// timer when dropped.
    pub fn dummy() -> Self {
        Self {
            ignore_drop: false,
            active: None,
        }
    }

    /// Returns true if the timer is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        if let Some(active) = &self.active {
            active.load(Ordering::Acquire)
        } else {
            false
        }
    }

    /// Prevents this guard from cancelling the timer when dropped.
    pub fn ignore(mut self) {
        self.ignore_drop = true;
    }

    /// Rearms the timer by setting its active state to true. This can be used
    /// to reactivate a timer that has been cancelled by dropping one of its
    /// guards. However, the state of the associated timer is not guaranteed to
    /// be consistent: The timer may have already been deleted.
    pub fn rearm(&mut self) {
        if let Some(active) = &self.active {
            active.store(true, Ordering::Release);
        }
    }

    /// Cancels the timer.
    pub fn cancel(&self) {
        if let Some(active) = &self.active {
            active.store(false, Ordering::Release);
        }
    }
}

impl Default for TimerGuard {
    /// Creates a new active guard that will share an atomic boolean with the
    /// timer and with all its clones, and will cancel the timer when one of
    /// the guards is dropped.
    fn default() -> Self {
        Self {
            active: Some(Arc::new(AtomicBool::new(true))),
            ignore_drop: false,
        }
    }
}

impl Drop for TimerGuard {
    fn drop(&mut self) {
        if !self.ignore_drop {
            self.cancel();
        }
    }
}

/// The timer manager is responsible for managing the active timers and
/// executing their callbacks when they expire.
pub struct TimerManager {
    /// A list of active timers, protected by a spinlock to allow concurrent
    /// access. The timers in this list are not sorted, since I don't expect
    /// to have a large number of active timers at the same time until I have
    /// a more complete kernel. If performance becomes an issue, I can switch
    /// to a more efficient data structure without changing the public API of
    /// the timer manager.
    timers: Spinlock<Vec<Timer>>,
}

impl TimerManager {
    /// Creates a new `TimerManager` instance with empty timer queues.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            timers: Spinlock::new(Vec::new()),
        }
    }

    /// Registers a new timer with this timer manager and return a guard that
    /// can be dropped to cancel the timer. If the timer is already expired, it
    /// will be executed immediately and the returned guard will be inactive.
    pub fn register_timer(&self, timer: Timer) -> TimerGuard {
        if timer.expired() {
            let guard = match &timer.mode {
                TimerMode::Periodic(_) => timer.guard.clone(),
                TimerMode::OneShot => TimerGuard::dummy(),
            };

            if timer.active() {
                timer.run_callback();
            }
            guard
        } else {
            let guard = timer.guard.clone();
            self.timers.lock_irq_safe().push(timer);
            guard
        }
    }

    /// Handles expired timers by executing their callbacks and removing them
    /// from the active timers list. Also removes timers that have been
    /// cancelled by dropping their guards.
    pub fn handle_expired_timers(&self) {
        // Drain all expired and inactive timers and collect expired timers.
        // PERF: Instead of collecting all expired timers in a vector allocated
        // on the heap, we could instead allocate a fixed-size array on the
        // stack and fill it with expired timers, and repeat this process until
        // there are no more expired timers.
        let expired: Vec<Timer> = self
            .timers
            .lock_irq_safe()
            .extract_if(.., |timer| timer.expired() || !timer.active())
            .filter(Timer::active)
            .collect();

        // Execute all expired timers. We need to do this outside of the lock
        // on the active timers list to allow callbacks to modify the active
        // timers list. Without this, a callback could deadlock the system by
        // trying to acquire the active timers list lock
        expired.into_iter().for_each(Timer::run_callback);
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// The global timer manager instance that will be used to manage all timers in
/// the kernel.
static TIMER_MANAGER: TimerManager = TimerManager::new();

/// Initializes the timer subsystem.
///
/// # Safety
/// This function should only be called once during the kernel initialization
/// process, after the memory management subsystem has been set up, and before
/// any timers are registered or any timer-related functions are called.
#[init]
pub unsafe fn setup() {
    // Nothing to do for now, but this function exists to allow for future
    // initialization of the timer subsystem if needed without having to modify
    // the main function.
}

/// Registers a new timer with the global timer manager.
///
/// See [`TimerManager::register_timer`] for more details since this function
/// is just a wrapper around it.
pub fn register(timer: Timer) -> TimerGuard {
    TIMER_MANAGER.register_timer(timer)
}

/// Schedules a new timer to execute the given callback at the specified
/// instant using the specified timer mode, which determines whether the
/// timer is a one-shot or a periodic timer.
///
/// The returned guard can be dropped to cancel the timer before it
/// expires. If the specified instant is in the past, the callback will
/// be executed immediately and the returned guard will be inactive.
pub fn schedule<F>(mode: TimerMode, at: Instant, callback: F) -> TimerGuard
where
    F: FnMut(&mut Timer) + Send + 'static,
{
    register(Timer::new(at, mode, Box::new(callback)))
}

/// Schedules a new timer to execute the given callback after the specified
/// duration has elapsed from the current time. The returned guard can be
/// dropped to cancel the timer before it expires.
pub fn schedule_in<F>(duration: Duration, callback: F) -> TimerGuard
where
    F: FnMut(&mut Timer) + Send + 'static,
{
    register(Timer::new(
        Instant::now() + duration,
        TimerMode::OneShot,
        Box::new(callback),
    ))
}

/// Schedules a new timer to execute the given callback at the specified
/// deadline. The returned guard can be dropped to cancel the timer before it
/// expires. If the deadline is in the past, the callback will be executed
/// immediately and the returned guard will be inactive.
pub fn schedule_at<F>(deadline: Instant, callback: F) -> TimerGuard
where
    F: FnMut(&mut Timer) + Send + 'static,
{
    register(Timer::new(deadline, TimerMode::OneShot, Box::new(callback)))
}

/// Schedules a new periodic timer to execute the given callback every time the
/// specified period elapses. The returned guard can be dropped to cancel the
/// timer. The first execution of the callback will be scheduled to occur after
/// the specified period elapses from the current time.
pub fn schedule_periodic<F>(period: Duration, callback: F) -> TimerGuard
where
    F: FnMut(&mut Timer) + Send + 'static,
{
    register(Timer::new(
        Instant::now() + period,
        TimerMode::Periodic(period),
        Box::new(callback),
    ))
}

/// Handles expired timers by executing their callbacks and removing them from
/// the active timers list. Also removes timers that have been cancelled by
/// dropping their guards.
///
/// This function should be called periodically by the architecture-specific
/// timer interrupt handler to ensure that expired timers are executed in a
/// timely manner.
pub fn handle_expired_timers() {
    TIMER_MANAGER.handle_expired_timers();
}
