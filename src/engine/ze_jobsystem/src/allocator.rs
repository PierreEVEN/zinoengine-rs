use crate::MAX_JOB_COUNT_PER_THREAD;
use std::cell::Cell;
use std::marker::PhantomData;
use std::mem;
use thread_local::ThreadLocal;

/// A very simple allocator that works by pre-allocating N instance of T on each threads
/// (so there is no atomic operations)
///
/// This allocator takes advantage of the fact that a game engine is frame-based and so N jobs will always rseult in N jobs destroyed at some point
/// So we can use a simple counter into our array
pub struct Allocator<T> {
    max_elements: usize,
    elements: ThreadLocal<Vec<u8>>,
    num_allocated: ThreadLocal<Cell<usize>>,
    _phantom: PhantomData<T>,
}

impl<T> Allocator<T> {
    pub fn new(max_elements: usize) -> Self {
        Self {
            max_elements,
            elements: ThreadLocal::new(),
            num_allocated: ThreadLocal::new(),
            _phantom: Default::default(),
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn allocate(&self, element: T) -> &mut T {
        let elements = self.elements.get_or(|| {
            let length = self.max_elements * mem::size_of::<T>();
            vec![0u8; length]
        });
        let num_allocated_cell = self.num_allocated.get_or_default();
        let num_allocated = num_allocated_cell.get();
        debug_assert!(num_allocated < self.max_elements);

        let index = num_allocated & (MAX_JOB_COUNT_PER_THREAD - 1);

        let elem_head = unsafe {
            let elem_head = elements.as_ptr().add(index * mem::size_of::<T>()) as *mut T;
            elem_head.write(element);
            elem_head
        };

        num_allocated_cell.set(num_allocated + 1);

        unsafe { <*mut T>::as_mut(elem_head).unwrap() }
    }
}
