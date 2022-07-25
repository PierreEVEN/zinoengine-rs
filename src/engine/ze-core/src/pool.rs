use std::collections::VecDeque;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::slice::IterMut;

enum Slot<T> {
    Alive((u16, T)),
    Free(u16),
    Dead,
}

enum SlotRemoveResult {
    Freed,
    MarkedAsDead,
    Failed,
}

struct Page<T, const PAGE_SIZE_IN_ELEMENTS: usize, const DISABLE_SLOT_AFTER_OVERFLOW: bool> {
    index: usize,
    memory: [Slot<T>; PAGE_SIZE_IN_ELEMENTS],
}

impl<T, const PAGE_SIZE_IN_ELEMENTS: usize, const DISABLE_SLOT_AFTER_OVERFLOW: bool>
    Page<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>
{
    fn new(index: usize) -> Self {
        let memory: [Slot<T>; PAGE_SIZE_IN_ELEMENTS] = array_init::array_init(|_| Slot::Free(0));
        Self { index, memory }
    }

    fn insert(&mut self, index: u32, object: T) -> Handle<T> {
        let real_index = self.real_index(index);

        if let Slot::Free(current_generation) = self.memory[real_index] {
            self.memory[real_index] = Slot::Alive((current_generation, object));
            return Handle {
                generation: current_generation,
                index,
                _phantom: Default::default(),
            };
        }

        unreachable!("The slot must be freed")
    }

    /// Remove an object from the pool
    /// Returns `true` if the slot has been freed and `false` if it didn't or was marked dead
    fn remove(&mut self, handle: &Handle<T>) -> SlotRemoveResult {
        let real_index = self.real_index(handle.index);
        let slot = &mut self.memory[real_index];
        if let Slot::Alive((generation, _)) = slot {
            if handle.generation == *generation {
                // If on the next generation we overflow and we have DISABLE_SLOT_AFTER_OVERFLOW
                // mark this slot as dead
                if DISABLE_SLOT_AFTER_OVERFLOW && handle.generation == u16::MAX {
                    *slot = Slot::Dead;
                    SlotRemoveResult::MarkedAsDead
                } else {
                    *slot = Slot::Free(generation.wrapping_add(1));
                    SlotRemoveResult::Freed
                }
            } else {
                SlotRemoveResult::Failed
            }
        } else {
            SlotRemoveResult::Failed
        }
    }

    fn at(&self, index: u32) -> &Slot<T> {
        &self.memory[self.real_index(index)]
    }

    fn at_mut(&mut self, index: u32) -> &mut Slot<T> {
        &mut self.memory[self.real_index(index)]
    }

    fn iter_mut(&mut self) -> IterMut<'_, Slot<T>> {
        self.memory.iter_mut()
    }

    fn real_index(&self, index: u32) -> usize {
        (index as usize) - (self.index * PAGE_SIZE_IN_ELEMENTS)
    }
}

/// A pool allocating pages of objects
/// Each handle to a pool is unique and will never be reused so checking validity works
///
/// A slot is given a generation, and when the generation overflows (since it is a u16) we just make it unusable
/// (controlled by DISABLE_SLOT_AFTER_OVERFLOW)
pub struct Pool<
    T,
    const PAGE_SIZE_IN_ELEMENTS: usize = 4096,
    const DISABLE_SLOT_AFTER_OVERFLOW: bool = true,
> {
    pages: Vec<Page<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>>,
    free_indices: VecDeque<u32>,
    _phantom: PhantomData<T>,
}

impl<T, const PAGE_SIZE_IN_ELEMENTS: usize, const DISABLE_SLOT_AFTER_OVERFLOW: bool>
    Pool<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>
{
    pub fn new() -> Self {
        Self {
            pages: vec![],
            free_indices: Default::default(),
            _phantom: Default::default(),
        }
    }

    pub fn insert(&mut self, object: T) -> Handle<T> {
        if let Some(index) = self.free_indices.pop_front() {
            if let Some(page_index) = self.find_index_page(index) {
                return self.pages[page_index].insert(index, object);
            }
        }

        // No free indices, allocate a new page
        let page_index = self.pages.len();
        self.pages.push(Page::new(page_index));
        for (index, _) in self.pages[page_index].iter_mut().enumerate() {
            self.free_indices
                .push_back(((page_index * PAGE_SIZE_IN_ELEMENTS) + index) as u32);
        }

        self.insert(object)
    }

    pub fn get(&self, handle: &Handle<T>) -> &T {
        if let Some(Slot::Alive((_, object))) = self.at(handle.index) {
            object
        } else {
            panic!()
        }
    }

    pub fn remove(&mut self, handle: &Handle<T>) -> bool {
        if let Some(page_index) = self.find_index_page(handle.index) {
            match self.pages[page_index].remove(handle) {
                SlotRemoveResult::Freed => {
                    self.free_indices.push_back(handle.index);
                    true
                }
                SlotRemoveResult::MarkedAsDead => true,
                SlotRemoveResult::Failed => false,
            }
        } else {
            false
        }
    }

    pub fn is_valid(&self, handle: &Handle<T>) -> bool {
        self.is_slot_alive(handle.index)
    }

    pub fn iter(&self) -> PoolIterator<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW> {
        PoolIterator::new(self)
    }

    pub fn iter_mut(
        &mut self,
    ) -> PoolIteratorMut<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW> {
        PoolIteratorMut::new(self)
    }

    fn at(&self, index: u32) -> Option<&Slot<T>> {
        if let Some(page_index) = self.find_index_page(index) {
            Some(self.pages[page_index].at(index))
        } else {
            None
        }
    }

    fn at_mut(&mut self, index: u32) -> Option<&mut Slot<T>> {
        if let Some(page_index) = self.find_index_page(index) {
            Some(self.pages[page_index].at_mut(index))
        } else {
            None
        }
    }

    /// Find the page corresponding to the index
    fn find_index_page(&self, index: u32) -> Option<usize> {
        let page_index = (index as usize) / PAGE_SIZE_IN_ELEMENTS;
        if page_index < self.pages.len() {
            Some(page_index)
        } else {
            None
        }
    }

    fn is_slot_alive(&self, index: u32) -> bool {
        matches!(self.at(index), Some(Slot::Alive(_)))
    }
}

impl<T, const PAGE_SIZE_IN_ELEMENTS: usize, const DISABLE_SLOT_AFTER_OVERFLOW: bool> Default
    for Pool<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const PAGE_SIZE_IN_ELEMENTS: usize, const DISABLE_SLOT_AFTER_OVERFLOW: bool>
    Index<&Handle<T>> for Pool<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>
{
    type Output = T;

    fn index(&self, handle: &Handle<T>) -> &Self::Output {
        if let Some(Slot::Alive((_, object))) = self.at(handle.index) {
            object
        } else {
            panic!("Handle not valid")
        }
    }
}

impl<T, const PAGE_SIZE_IN_ELEMENTS: usize, const DISABLE_SLOT_AFTER_OVERFLOW: bool>
    IndexMut<&Handle<T>> for Pool<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>
{
    fn index_mut(&mut self, handle: &Handle<T>) -> &mut Self::Output {
        if let Some(Slot::Alive((_, object))) = self.at_mut(handle.index) {
            object
        } else {
            panic!("Handle not valid")
        }
    }
}

pub struct PoolIterator<
    'a,
    T,
    const PAGE_SIZE_IN_ELEMENTS: usize,
    const DISABLE_SLOT_AFTER_OVERFLOW: bool,
> {
    pool: &'a Pool<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>,
    current_index: u32,
}

impl<'a, T, const PAGE_SIZE_IN_ELEMENTS: usize, const DISABLE_SLOT_AFTER_OVERFLOW: bool>
    PoolIterator<'a, T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>
{
    pub fn new(pool: &'a Pool<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>) -> Self {
        Self {
            pool,
            current_index: 0,
        }
    }
}

impl<'a, T, const PAGE_SIZE_IN_ELEMENTS: usize, const DISABLE_SLOT_AFTER_OVERFLOW: bool> Iterator
    for PoolIterator<'a, T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(slot) = self.pool.at(self.current_index) {
            if let Slot::Alive((_, object)) = slot {
                self.current_index += 1;
                return Some(object);
            }

            self.current_index += 1;
        }

        None
    }
}

pub struct PoolIteratorMut<
    'a,
    T,
    const PAGE_SIZE_IN_ELEMENTS: usize,
    const DISABLE_SLOT_AFTER_OVERFLOW: bool,
> {
    pool: &'a mut Pool<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>,
    current_index: u32,
}

impl<'a, T, const PAGE_SIZE_IN_ELEMENTS: usize, const DISABLE_SLOT_AFTER_OVERFLOW: bool>
    PoolIteratorMut<'a, T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>
{
    pub fn new(pool: &'a mut Pool<T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>) -> Self {
        Self {
            pool,
            current_index: 0,
        }
    }
}

impl<'a, T, const PAGE_SIZE_IN_ELEMENTS: usize, const DISABLE_SLOT_AFTER_OVERFLOW: bool> Iterator
    for PoolIteratorMut<'a, T, PAGE_SIZE_IN_ELEMENTS, DISABLE_SLOT_AFTER_OVERFLOW>
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(slot) = self.pool.at_mut(self.current_index) {
            if let Slot::Alive((_, object)) = slot {
                self.current_index += 1;
                return Some(unsafe { &mut *(object as *mut _) });
            }

            self.current_index += 1;
        }

        None
    }
}

/// Unique handle to a pool
pub struct Handle<T> {
    generation: u16,
    index: u32,
    _phantom: PhantomData<T>,
}

impl<T> Default for Handle<T> {
    fn default() -> Self {
        Self {
            generation: 0,
            index: 0,
            _phantom: Default::default(),
        }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            generation: self.generation,
            index: self.index,
            _phantom: self._phantom,
        }
    }
}

impl<T> Copy for Handle<T> {}

#[cfg(test)]
mod tests {
    use crate::pool::{Handle, Pool};

    #[test]
    fn insert_500_elements_and_verify_memory_sanity() {
        let mut pool = Pool::<u32>::new();
        let mut handles = vec![];
        for i in 0..500 {
            handles.push(pool.insert(i));
        }

        let mut index = 0;
        for i in pool.iter() {
            assert_eq!(*i, index);
            index += 1;
        }

        // Ensure we visited all slots
        assert_eq!(index, 500);
    }

    #[test]
    fn insert_500_elements_remove_one_and_verify_memory_sanity() {
        let mut pool = Pool::<u32>::new();
        let mut handles = vec![];
        for i in 0..500 {
            handles.push(pool.insert(i));
        }

        pool.remove(&handles.pop().unwrap());

        let mut index = 0;
        for i in pool.iter() {
            assert_eq!(*i, index);
            index += 1;
        }

        assert_eq!(index, 499);
    }

    #[test]
    fn insert_one_element_until_overflow_check_slot_marked_as_dead() {
        let mut pool = Pool::<u32, 1, true>::new();

        let mut old_handle = Default::default();
        for i in 0..(u16::MAX as u32) + 1 {
            old_handle = pool.insert(i);
            pool.remove(&old_handle);
        }

        let handle = pool.insert(0);
        assert!(pool.is_valid(&handle));
        assert_eq!(handle.index, 1);
        assert!(!pool.is_valid(&old_handle));
    }

    #[test]
    fn insert_500_elements_check_iter_mut() {
        let mut pool = Pool::<u32>::new();
        let mut handles = vec![];
        for i in 0..500 {
            handles.push(pool.insert(i));
        }

        for i in pool.iter_mut() {
            *i = 20;
        }

        for i in pool.iter_mut() {
            assert_eq!(*i, 20);
        }
    }

    #[test]
    fn insert_one_element_until_overflow_check_remove_fail() {
        let mut pool = Pool::<u32, 1, true>::new();

        let mut old_handle = Default::default();
        for i in 0..(u16::MAX as u32) + 1 {
            old_handle = pool.insert(i);
            pool.remove(&old_handle);
        }

        assert!(!pool.remove(&old_handle));
    }

    #[test]
    fn remove_invalid_handle_fail() {
        let mut pool = Pool::<u32, 1, true>::new();
        assert!(!pool.remove(&Handle::default()));
    }
}
