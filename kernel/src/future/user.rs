use super::yield_once;
use crate::{
    arch::{
        self,
        trap::{Resume, Trap},
    },
    config,
};
use core::time::Duration;

/// Thread exit status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// Normal termination with exit code
    Terminate(i32),

    /// Termination due to a fault
    Fault,
}

/// The thread execution loop future. This future runs the given thread
/// until it terminates, either normally or due to a fault.
pub async fn thread_loop(mut thread: arch::thread::Thread) {
    let max_run_duration = Duration::from_millis(config::THREAD_MAX_RUN_DURATION);
    let mut remaining = max_run_duration;

    let exit = loop {
        // Set the next timer event
        arch::timer::next_event(remaining);

        // Execute the thread until it traps, and measure the elapsed time
        // to update the remaining quantum of continuous user execution.
        let start = arch::timer::since_boot();
        let trap = arch::thread::execute(&mut thread);
        let elapsed = arch::timer::since_boot().saturating_sub(start);
        remaining = remaining.saturating_sub(elapsed);

        // Handle the trap and determine whether to continue executing
        // the thread or terminate it.
        let start = arch::timer::since_boot();
        let mut resume = match trap {
            Trap::Exception => arch::trap::handle_exception(&mut thread),
            Trap::Interrupt => arch::trap::handle_interrupt(&mut thread),
            Trap::Syscall => arch::trap::handle_syscall(&mut thread).await,
        };

        // If the quantum has expired, yield to the scheduler and reset
        // the quantum. This ensures that threads are preempted after
        // their maximum allowed continuous execution time without yielding.
        if remaining.is_zero() {
            log::trace!("Thread continuous execution runtime expired, yielding");
            resume = Resume::Yield;
        }

        // Measures the time spent handling the trap to update the remaining
        // quantum, to avoid giving extra time to threads that trap frequently.
        // WARNING : Do not add too much code after this point that could take
        // a long time to execute, as it will not be accounted for in the
        // remaining quantum, potentially causing threads to exceed their
        // allowed continuous execution time.
        let elapsed = arch::timer::since_boot().saturating_sub(start);
        remaining = remaining.saturating_sub(elapsed);

        match resume {
            Resume::Terminate(code) => break Exit::Terminate(code),
            Resume::Yield => {
                // Reset the quantum and yield to the scheduler. We reset
                // the quantum because the thread voluntarily yielded, so
                // we reset its continuous execution time in order to give
                // it a full quantum when it is rescheduled.
                remaining = max_run_duration;
                yield_once().await;
            }
            Resume::Fault => break Exit::Fault,
            Resume::Continue => (),
        }
    };

    log::info!("Thread terminated with {:?}", exit);
}
