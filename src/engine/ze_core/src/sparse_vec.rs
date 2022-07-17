use bit_vec::BitVec;
use std::ops::{Index, IndexMut};

#[derive(Debug)]
pub struct SparseVec<T> {
    data: Vec<Option<T>>,
    allocated_bitset: BitVec<u32>,
    len: usize,
}

impl<T> SparseVec<T> {
    pub fn add(&mut self, elem: T) -> usize {
        let index = self.get_or_insert_free_index();
        debug_assert!(self.data[index].is_none());
        self.data[index] = Some(elem);
        self.allocated_bitset.set(index, true);
        self.len += 1;
        index
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if self.allocated_bitset.get(index).unwrap() {
            self.data[index] = None;
            self.allocated_bitset.set(index, false);
            self.len -= 1;
            true
        } else {
            false
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if let Some(object) = &self.data[index] {
            Some(object)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if let Some(object) = &mut self.data[index] {
            Some(object)
        } else {
            None
        }
    }

    pub fn is_valid(&self, index: usize) -> bool {
        index < self.len && self.data[index].is_some()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> SparseArrayIterator<T> {
        SparseArrayIterator::new(self)
    }

    pub fn iter_mut(&mut self) -> SparseArrayIteratorMut<T> {
        SparseArrayIteratorMut::new(self)
    }

    fn find_free_index(&self) -> Option<usize> {
        for (i, bit) in self.allocated_bitset.iter().enumerate() {
            if !bit {
                return Some(i);
            }
        }

        None
    }

    fn get_or_insert_free_index(&mut self) -> usize {
        if let Some(index) = self.find_free_index() {
            index
        } else {
            let index = self.data.len();
            self.data.push(None);
            self.allocated_bitset.push(false);
            index
        }
    }
}

impl<T> Default for SparseVec<T> {
    fn default() -> Self {
        Self {
            data: vec![],
            allocated_bitset: Default::default(),
            len: 0,
        }
    }
}

impl<T> Index<usize> for SparseVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<T> IndexMut<usize> for SparseVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

pub struct SparseArrayIterator<'a, T> {
    array: &'a SparseVec<T>,
    current_index: usize,
}

impl<'a, T> SparseArrayIterator<'a, T> {
    pub fn new(array: &'a SparseVec<T>) -> Self {
        Self {
            array,
            current_index: 0,
        }
    }
}

impl<'a, T> Iterator for SparseArrayIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_index < self.array.len() {
            if let Some(object) = self.array.get(self.current_index) {
                self.current_index += 1;
                return Some(object);
            }
            self.current_index += 1;
        }

        None
    }
}

pub struct SparseArrayIteratorMut<'a, T> {
    array: &'a mut SparseVec<T>,
    current_index: usize,
}

impl<'a, T> SparseArrayIteratorMut<'a, T> {
    pub fn new(array: &'a mut SparseVec<T>) -> Self {
        Self {
            array,
            current_index: 0,
        }
    }
}

impl<'a, T> Iterator for SparseArrayIteratorMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_index < self.array.len() {
            if let Some(object) = self.array.get_mut(self.current_index) {
                self.current_index += 1;
                return Some(unsafe { &mut *(object as *mut _) });
            }
            self.current_index += 1;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::sparse_vec::SparseVec;

    #[test]
    fn insert_500_remove_index_350_insert_index_350() {
        let mut array = SparseVec::default();
        for i in 0..500 {
            array.add(i);
        }
        assert_eq!(array.len(), 500);

        assert!(array.remove(350));
        assert_eq!(array.len(), 499);

        assert_eq!(array.add(20), 350);
        assert_eq!(array.len(), 500);
    }

    #[test]
    fn insert_500_iterate_validate() {
        let mut array = SparseVec::default();
        for i in 0..500 {
            array.add(i);
        }

        for (i, e) in array.iter().enumerate() {
            assert_eq!(*e, i);
        }
    }

    #[test]
    fn insert_500_iterate_mutate_to_200_validate() {
        let mut array = SparseVec::default();
        for i in 0..500 {
            array.add(i);
        }

        for element in array.iter_mut() {
            *element = 200;
        }

        for element in array.iter() {
            assert_eq!(*element, 200);
        }
    }
}
