use super::yield_once;
use arch::trap::{Resume, Trap};
use config::THREAD_QUANTUM;
use core::time::Duration;

/// Thread exit status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Terminate(i32),
    Fault,
}

pub async fn thread_loop(mut thread: arch::thread::Thread) {
    let exit = loop {
        // Set the next timer event
        arch::timer::next_event(Duration::from_millis(THREAD_QUANTUM));
        let trap = arch::thread::execute(&mut thread);
        let resume = match trap {
            Trap::Exception => arch::trap::handle_exception(&mut thread),
            Trap::Interrupt => arch::trap::handle_interrupt(&mut thread),
            Trap::Syscall => Resume::Terminate(0),
        };

        // TODO: Proper quantum management: if a thread yields, its quantum
        // sohuld be reset to the full value.
        match resume {
            Resume::Terminate(code) => break Exit::Terminate(code),
            Resume::Continue => continue,
            Resume::Yield => yield_once().await,
            Resume::Fault => break Exit::Fault,
        }
    };

    log::info!("Thread terminated with {:?}", exit);
}
