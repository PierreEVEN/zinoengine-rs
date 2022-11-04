use crate::job::{Job, JobHandle};
use std::cell::{Cell, UnsafeCell};
use std::sync::atomic::Ordering;
use thread_local::ThreadLocal;

/// Thread-local job allocator
#[derive(Debug)]
pub(crate) struct JobAllocator {
    capacity: usize,
    elements: ThreadLocal<Vec<UnsafeCell<Job>>>,
    num_allocated: ThreadLocal<Cell<usize>>,
}

#[derive(Debug)]
pub(crate) enum Error {
    /// Capacity exhausted, flush some jobs to retry
    Exhausted,
}

impl JobAllocator {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            elements: ThreadLocal::new(),
            num_allocated: ThreadLocal::new(),
        }
    }

    pub fn allocate(&self) -> Result<JobHandle, Error> {
        let elements = self.elements.get_or(|| {
            let mut vec = Vec::with_capacity(self.capacity);
            vec.resize_with(self.capacity, || UnsafeCell::new(Job::default()));
            vec
        });

        let num_allocated_cell = self.num_allocated.get_or_default();
        let num_allocated = num_allocated_cell.get();

        let index = num_allocated & (self.capacity - 1);

        // SAFETY: We only read the atomic finished state
        let job = unsafe { &*elements[index].get() };
        if job.unfinished_jobs.load(Ordering::SeqCst) == 0 {
            num_allocated_cell.set(num_allocated + 1);
            Ok(JobHandle(&elements[index]))
        } else {
            Err(Error::Exhausted)
        }
    }
}
