use super::yield_once;
use arch::trap::{Resume, Trap};

pub async fn thread_loop(mut thread: arch::thread::Thread) {
    loop {
        // TODO: Setup thread execution timeout
        let trap = arch::thread::execute(&mut thread);
        let resume = match trap {
            Trap::Exception => Resume::Fault,
            Trap::Interrupt => Resume::Yield,
            Trap::Syscall => Resume::Continue,
        };

        match resume {
            Resume::Terminate(_) => break,
            Resume::Continue => continue,
            Resume::Yield => yield_once().await,
            Resume::Fault => break,
        }
    }

    log::info!("Thread terminated");
}
