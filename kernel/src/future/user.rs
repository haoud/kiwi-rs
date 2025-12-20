use super::yield_once;
use crate::arch::{
    self,
    trap::{Resume, Trap},
};
use config::THREAD_QUANTUM;
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
    let exit = loop {
        // Set the next timer event
        arch::timer::next_event(Duration::from_millis(THREAD_QUANTUM));
        let trap = arch::thread::execute(&mut thread);
        let resume = match trap {
            Trap::Exception => arch::trap::handle_exception(&mut thread),
            Trap::Interrupt => arch::trap::handle_interrupt(&mut thread),
            Trap::Syscall => arch::trap::handle_syscall(&mut thread),
        };

        // TODO: Proper quantum management: if a thread yields, it should not
        // consume its entire quantum.
        match resume {
            Resume::Terminate(code) => break Exit::Terminate(code),
            Resume::Continue => (),
            Resume::Yield => yield_once().await,
            Resume::Fault => break Exit::Fault,
        }
    };

    log::info!("Thread terminated with {:?}", exit);
}
