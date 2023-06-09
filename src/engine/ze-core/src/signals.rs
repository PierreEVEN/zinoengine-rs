﻿use crate::sparse_vec::SparseVec;
use parking_lot::Mutex;

/// An object storing functions to be called when signaled

type Slot<Args> = Box<dyn FnMut(Args)>;
type SyncSlot<Args> = Box<dyn FnMut(Args) + Send>;

pub struct Handle(usize);

pub struct Signal<Args> {
    slots: SparseVec<Slot<Args>>,
}

impl<Args> Signal<Args>
where
    Args: Clone + 'static,
{
    pub fn connect<F>(&mut self, func: F) -> Handle
    where
        F: FnMut(Args) + 'static,
    {
        Handle(self.slots.push(Box::new(func)))
    }

    /// Disconnect `handle`
    ///
    /// # Panics
    ///
    /// Panics if `handle` is not a valid handle
    pub fn disconnect(&mut self, handle: Handle) {
        let _ = self.slots.remove(handle.0);
    }

    pub fn emit(&mut self, args: Args) {
        for slot in self.slots.iter_mut() {
            (slot)(args.clone())
        }
    }
}

impl<Args> Default for Signal<Args> {
    fn default() -> Self {
        Self {
            slots: Default::default(),
        }
    }
}

/// Sync version of `Signal`
pub struct SyncSignal<Args> {
    slots: Mutex<SparseVec<SyncSlot<Args>>>,
}

impl<Args> SyncSignal<Args>
where
    Args: Clone + 'static,
{
    pub fn connect<F>(&mut self, func: F) -> Handle
    where
        F: FnMut(Args) + Send + Sync + 'static,
    {
        let mut slots = self.slots.lock();
        Handle(slots.push(Box::new(func)))
    }

    /// Disconnect `handle`
    ///
    /// # Panics
    ///
    /// Panics if `handle` is not a valid handle
    pub fn disconnect(&mut self, handle: Handle) {
        let mut slots = self.slots.lock();
        let _ = slots.remove(handle.0);
    }

    pub fn emit(&self, args: Args) {
        let mut slots = self.slots.lock();
        for slot in slots.iter_mut() {
            (slot)(args.clone())
        }
    }
}

impl<Args> Default for SyncSignal<Args> {
    fn default() -> Self {
        Self {
            slots: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::signals::SyncSignal;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn connect_emit() {
        let received = Arc::new(AtomicBool::new(false));
        let mut signal: SyncSignal<()> = SyncSignal::default();
        {
            let received = received.clone();
            signal.connect(move |_| received.store(true, Ordering::SeqCst));
        }

        signal.emit(());

        assert!(received.load(Ordering::SeqCst));
    }

    #[test]
    fn connect_disconnect_and_emit() {
        let received = Arc::new(AtomicBool::new(false));
        let mut signal: SyncSignal<()> = SyncSignal::default();
        {
            let received = received.clone();
            let handle = signal.connect(move |_| received.store(true, Ordering::SeqCst));
            signal.disconnect(handle);
        }

        signal.emit(());

        assert!(!received.load(Ordering::SeqCst));
    }
}
