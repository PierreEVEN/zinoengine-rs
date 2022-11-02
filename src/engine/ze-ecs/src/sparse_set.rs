use crate::erased_vec::{TypeErasedVec, TypeInfo};
use std::ops::Index;
use std::ptr::Unique;
use ze_core::sparse_vec::SparseVec;

/// A data structure relying on two arrays:
/// - A tightly packed dense array containing the actual elements
/// - A sparse array, mapping elements to their actual index in the dense array
#[derive(Debug)]
pub struct SparseSet<T> {
    sparse: SparseVec<usize>,
    dense: Vec<T>,

    // Store sparse indices for each dense element
    dense_sparse_indices: Vec<usize>,
}

impl<T> SparseSet<T> {
    pub fn insert(&mut self, index: usize, value: T) {
        self.sparse.resize(index + 1);
        self.sparse.insert(index, self.dense.len());
        self.dense.push(value);
        self.dense_sparse_indices.push(index);
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.sparse.get(index).map(|index| {
            // SAFETY: sparse always store valid dense indices
            unsafe { self.dense.get_unchecked(*index) }
        })
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.sparse.get(index).map(|index| {
            // SAFETY: sparse always store valid dense indices
            unsafe { self.dense.get_unchecked_mut(*index) }
        })
    }
}

impl<T> Default for SparseSet<T> {
    fn default() -> Self {
        Self {
            sparse: Default::default(),
            dense: vec![],
            dense_sparse_indices: vec![],
        }
    }
}

impl<T> Index<usize> for SparseSet<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("Invalid index")
    }
}

/// Like `SparseSet` but use a `TypeErasedVec`
pub(crate) struct TypeErasedSparseSet {
    sparse: SparseVec<usize>,
    dense: TypeErasedVec,

    // Store sparse indices for each dense element
    dense_sparse_indices: Vec<usize>,
}

impl TypeErasedSparseSet {
    pub fn new(type_info: TypeInfo) -> Self {
        Self {
            sparse: Default::default(),
            dense: TypeErasedVec::new(type_info),
            dense_sparse_indices: vec![],
        }
    }

    /// # Safety
    ///
    /// `value` must point to a valid value with a correct layout
    pub unsafe fn insert_unchecked(&mut self, index: usize, value: Unique<u8>) -> usize {
        let dense_index = self.dense.len();
        self.sparse.resize(index + 1);
        self.sparse.insert(index, dense_index);
        self.dense.push(value);
        self.dense_sparse_indices.push(index);
        dense_index
    }

    pub fn remove(&mut self, index: usize) {
        // SAFETY: We don't use the contained value
        assert!(unsafe { self.get_unchecked::<u8>(index).is_some() });
        let last_sparse_index = self.dense_sparse_indices[self.dense.len() - 1];
        self.sparse[last_sparse_index] = index;
        unsafe { self.dense.swap_remove_unchecked(index) }
        self.sparse.remove(index);
    }

    /// Same as `remove` but will forget the value instead of dropping it
    pub fn remove_forget(&mut self, index: usize) {
        // SAFETY: We don't use the contained value
        assert!(unsafe { self.get_unchecked::<u8>(index).is_some() });
        let last_sparse_index = self.dense_sparse_indices[self.dense.len() - 1];
        self.sparse[last_sparse_index] = index;
        unsafe { self.dense.swap_remove_forget_unchecked(index) }
        self.sparse.remove(index);
    }

    /// # Safety
    ///
    /// `T` must be the same as the contained type
    pub unsafe fn get_unchecked<T>(&self, index: usize) -> Option<&T> {
        self.sparse.get(index).map(|index| {
            // SAFETY: sparse always store valid dense indices
            (self.dense.get_unchecked(*index) as *const T)
                .as_ref()
                .unwrap_unchecked()
        })
    }

    /// # Safety
    ///
    /// `T` must be the same as the contained type
    pub unsafe fn get_unchecked_mut<T>(&mut self, index: usize) -> Option<&mut T> {
        self.sparse.get(index).map(|index| {
            // SAFETY: sparse always store valid dense indices
            (self.dense.get_unchecked_mut(*index) as *mut T)
                .as_mut()
                .unwrap_unchecked()
        })
    }

    pub fn get<T: 'static>(&self, index: usize) -> Option<&T> {
        assert_eq!(TypeInfo::new::<T>(), self.dense.type_info());
        // SAFETY: Assert if T is not correct
        unsafe { self.get_unchecked(index) }
    }

    pub fn get_mut<T: 'static>(&mut self, index: usize) -> Option<&mut T> {
        assert_eq!(TypeInfo::new::<T>(), self.dense.type_info());
        // SAFETY: Assert if T is not correct
        unsafe { self.get_unchecked_mut(index) }
    }
}

#[cfg(test)]
mod tests {
    use crate::sparse_set::SparseSet;

    #[test]
    fn insert() {
        let mut set = SparseSet::default();
        set.insert(1000, 120);
        set.insert(0, 10);
        set.insert(1, 20);
        set.insert(2, 40);
        assert_eq!(set.dense.len(), 4);
        assert_eq!(set.dense.capacity(), 4);
        assert_eq!(set[1000], 120);
        assert_eq!(set[0], 10);
        assert_eq!(set[1], 20);
        assert_eq!(set[2], 40);
    }

    mod type_erased {
        use crate::erased_vec::TypeInfo;
        use crate::sparse_set::TypeErasedSparseSet;
        use std::mem::forget;
        use std::ptr::Unique;

        fn insert_typed<T: 'static>(
            sparse_set: &mut TypeErasedSparseSet,
            index: usize,
            value: T,
        ) -> &mut T {
            assert_eq!(TypeInfo::new::<T>(), sparse_set.dense.type_info());
            let dense_index = unsafe {
                // SAFETY: we assert if layout is invalid
                sparse_set.insert_unchecked(
                    index,
                    Unique::new(&value as *const T as *mut T as *mut u8).expect("value was null!"),
                )
            };
            forget(value);

            // SAFETY: insert_unchecked always insert at dense_index
            unsafe { &mut *(sparse_set.dense.get_unchecked_mut(dense_index) as *mut T) }
        }

        #[test]
        fn insert() {
            let mut set = TypeErasedSparseSet::new(TypeInfo::new::<u128>());
            insert_typed(&mut set, 1000, 10u128);
            insert_typed(&mut set, 0, 20u128);
            insert_typed(&mut set, 1, 30u128);
            insert_typed(&mut set, 2, 40u128);
            assert_eq!(set.dense.len(), 4);
            assert_eq!(*set.get::<u128>(1000).unwrap(), 10);
            assert_eq!(*set.get::<u128>(0).unwrap(), 20);
            assert_eq!(*set.get::<u128>(1).unwrap(), 30);
            assert_eq!(*set.get::<u128>(2).unwrap(), 40);
        }

        #[test]
        fn remove() {
            let mut set = TypeErasedSparseSet::new(TypeInfo::new::<u128>());
            insert_typed(&mut set, 0, 10u128);
            insert_typed(&mut set, 1, 20u128);
            insert_typed(&mut set, 2, 40u128);
            insert_typed(&mut set, 3, 60u128);
            assert_eq!(set.dense.len(), 4);
            set.remove(2);
            assert_eq!(set.dense.len(), 3);
            assert_eq!(*set.get::<u128>(0).unwrap(), 10);
            assert_eq!(*set.get::<u128>(1).unwrap(), 20);
            assert!(set.get::<u128>(2).is_none());
            assert_eq!(*set.get::<u128>(3).unwrap(), 60);
        }
    }
}
