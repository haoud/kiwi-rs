//! The keyboard subsystem. Keyboard drivers (e.g. [`ps2`]) decode what the
//! hardware sends into [`KeyEvent`]s identifying the physical keys, and push
//! them into a queue with [`push_event`], usually from an interrupt handler.
//! The events are then consumed asynchronously with [`next_event`], and can be
//! translated into characters with an [`EventDecoder`] configured with the
//! current [`layout`].
//!
//! The key events, the decoding of the scancodes and the keyboard layouts
//! come from the `pc-keyboard` crate.
use core::{
    future::poll_fn,
    task::{Context, Poll, Waker},
};

pub use pc_keyboard::{DecodedKey, EventDecoder, HandleControl, KeyCode, KeyEvent, KeyState};

use crate::library::lock::spin::Spinlock;

pub mod layout;

#[cfg(target_arch = "x86_64")]
pub mod ps2;

/// The number of events that can be queued before new events are dropped.
const QUEUE_CAPACITY: usize = 64;

/// The queue of key events, filled by the keyboard drivers and emptied by the
/// consumer of the events. It is a fixed-size ring buffer, so that pushing an
/// event from an interrupt handler never allocates memory.
struct Queue {
    events: [Option<KeyEvent>; QUEUE_CAPACITY],
    head: usize,
    len: usize,

    /// The waker of the task waiting for an event, if any.
    waker: Option<Waker>,
}

impl Queue {
    const fn new() -> Self {
        Self {
            events: [const { None }; QUEUE_CAPACITY],
            head: 0,
            len: 0,
            waker: None,
        }
    }

    fn push(&mut self, event: KeyEvent) -> Result<(), KeyEvent> {
        if self.len == QUEUE_CAPACITY {
            return Err(event);
        }
        self.events[(self.head + self.len) % QUEUE_CAPACITY] = Some(event);
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<KeyEvent> {
        if self.len == 0 {
            return None;
        }
        let event = self.events[self.head].take();
        self.head = (self.head + 1) % QUEUE_CAPACITY;
        self.len -= 1;
        event
    }
}

/// The key events not consumed yet. The lock must always be taken with
/// interrupts disabled, since events are pushed from interrupt handlers.
static QUEUE: Spinlock<Queue> = Spinlock::new(Queue::new());

/// Pushes a key event into the queue and wakes up the task waiting for it.
/// The event is dropped if the queue is full. This function can be called from
/// an interrupt handler.
pub fn push_event(event: KeyEvent) {
    let waker = {
        let mut queue = QUEUE.lock_irq_safe();
        // The queue is full when nobody consumes the events (e.g. when the
        // kernel is built without the debug console): drop the event.
        if let Err(event) = queue.push(event) {
            log::trace!("Keyboard: event queue full, dropping {:?}", event.code);
        }
        queue.waker.take()
    };

    // Wake up the waiting task after releasing the lock, so that the lock is
    // not held while the waker pushes the task into its executor.
    if let Some(waker) = waker {
        waker.wake();
    }
}

/// Attempts to get the next key event. If there is none, the task of the given
/// context is woken up when a key event is pushed.
///
/// The queue only remembers one waker: the keyboard is meant to have a single
/// consumer, which receives all the key events.
pub fn poll_event(cx: &mut Context<'_>) -> Poll<KeyEvent> {
    let mut queue = QUEUE.lock_irq_safe();
    if let Some(event) = queue.pop() {
        return Poll::Ready(event);
    }

    match &mut queue.waker {
        Some(waker) if waker.will_wake(cx.waker()) => {}
        waker => *waker = Some(cx.waker().clone()),
    }
    Poll::Pending
}

/// Waits for the next key event.
pub async fn next_event() -> KeyEvent {
    poll_fn(poll_event).await
}
