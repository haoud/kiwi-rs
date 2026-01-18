use super::yield_once;
use crate::{
    arch::{
        self,
        trap::{Resume, Trap},
    },
    config::THREAD_MAX_RUN_DURATION,
    future,
    time::Instant,
};

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
    let mut poll_generation = future::executor::poll_generation();
    let mut deadline = Instant::now() + THREAD_MAX_RUN_DURATION;

    let exit = loop {
        // Set the next timer event
        arch::timer::next_event(Instant::now().duration_until(deadline));

        // Execute the thread until it traps, and measure the elapsed time
        // to update the remaining quantum of continuous user execution.
        let trap = arch::thread::execute(&mut thread);

        // Handle the trap and determine whether to continue executing
        // the thread or terminate it.
        let mut resume = match trap {
            Trap::Exception => arch::trap::handle_exception(&mut thread),
            Trap::Interrupt => arch::trap::handle_interrupt(&mut thread),
            Trap::Syscall => arch::trap::handle_syscall(&mut thread).await,
        };

        if future::executor::has_yielded(&poll_generation) {
            // The executor has polled other tasks since we last checked,
            // indicating that this task has yielded. Update the poll generation
            // to the current one and reset the continuous execution quantum
            // to the maximum.
            poll_generation = future::executor::poll_generation();
            deadline = Instant::now() + THREAD_MAX_RUN_DURATION;
        } else if deadline.has_passed() {
            // If the quantum has expired, yield to the scheduler and reset
            // the quantum. This ensures that threads are preempted after
            // their maximum allowed continuous execution time without yielding.
            resume = Resume::Yield;
        }

        match resume {
            Resume::Terminate(code) => break Exit::Terminate(code),
            Resume::Yield => {
                // Reset the quantum and yield to the scheduler. We reset
                // the quantum because the thread voluntarily yielded, so
                // we reset its continuous execution time in order to give
                // it a full quantum when it is rescheduled.
                yield_once().await;
                deadline = Instant::now() + THREAD_MAX_RUN_DURATION;
            }
            Resume::Fault => break Exit::Fault,
            Resume::Continue => (),
        }
    };

    log::info!("Thread terminated with {:?}", exit);
}
