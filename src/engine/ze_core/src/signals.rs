use crate::sparse_array::SparseArray;
use parking_lot::Mutex;

/// An object storing functions to be called when signaled

type Slot<Args> = Box<dyn FnMut(Args) + Send>;

pub struct Handle(usize);

pub struct Signal<Args> {
    slots: Mutex<SparseArray<Slot<Args>>>,
}

impl<Args> Signal<Args>
where
    Args: Clone + 'static,
{
    pub fn connect<F>(&mut self, func: F) -> Handle
    where
        F: FnMut(Args) + Send + Sync + 'static,
    {
        let mut slots = self.slots.lock();
        Handle(slots.add(Box::new(func)))
    }

    pub fn disconnect(&mut self, handle: Handle) -> bool {
        let mut slots = self.slots.lock();
        slots.remove(handle.0)
    }

    pub fn emit(&self, args: Args) {
        let mut slots = self.slots.lock();
        for slot in slots.iter_mut() {
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

#[cfg(test)]
mod tests {
    use crate::signals::Signal;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn connect_emit() {
        let mut received = Arc::new(AtomicBool::new(false));
        let mut signal: Signal<()> = Signal::default();
        {
            let received = received.clone();
            signal.connect(move |_| received.store(true, Ordering::SeqCst));
        }

        signal.emit(());

        assert_eq!(received.load(Ordering::SeqCst), true);
    }

    #[test]
    fn connect_disconnect_and_emit() {
        let mut received = Arc::new(AtomicBool::new(false));
        let mut signal: Signal<()> = Signal::default();
        {
            let received = received.clone();
            let handle = signal.connect(move |_| received.store(true, Ordering::SeqCst));
            signal.disconnect(handle);
        }

        signal.emit(());

        assert_eq!(received.load(Ordering::SeqCst), false);
    }
}
