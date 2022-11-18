use std::alloc::Layout;
use std::any::TypeId;
use std::ptr;
use std::ptr::NonNull;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct TypeInfo {
    pub(crate) id: TypeId,
    pub(crate) layout: Layout,
    pub(crate) drop_fn: unsafe fn(*mut u8),
}

impl TypeInfo {
    pub fn new<T: 'static>() -> Self {
        TypeInfo {
            id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop_fn: |ptr| unsafe { ptr::drop_in_place(ptr as *mut T) },
        }
    }
}

pub(crate) struct TypeErasedVec {
    type_info: TypeInfo,
    len: usize,
    capacity: usize,
    data: NonNull<u8>,
}

impl TypeErasedVec {
    pub fn new(type_info: TypeInfo) -> Self {
        // If type is a ZST, just put an infinite capacity to never alloc
        if type_info.layout.size() == 0 {
            Self {
                type_info,
                len: 0,
                capacity: usize::MAX,
                data: NonNull::dangling(),
            }
        } else {
            Self {
                type_info,
                len: 0,
                capacity: 0,
                data: NonNull::dangling(),
            }
        }
    }

    /// Push `value` by copying it to the vector
    ///
    /// # Safety
    ///
    /// `value` must point to a valid value with the same type
    pub unsafe fn push(&mut self, value: NonNull<u8>) {
        let index = self.len;
        self.reserve_exact(1);
        self.len += 1;
        self.initialize_unchecked(index, value);
    }

    /// Remove the element at `index` by swapping it with the last element
    /// # Safety
    ///
    /// `index` must be a valid index
    pub unsafe fn swap_remove_unchecked(&mut self, index: usize) {
        debug_assert!(index < self.len);

        (self.type_info.drop_fn)(self.get_unchecked_mut(index));

        ptr::copy(
            self.get_unchecked(self.len - 1),
            self.get_unchecked_mut(index),
            self.type_info.layout.size(),
        );

        self.len -= 1;
    }

    /// Remove the element at `index` by swapping it with the last element, but doesn't call its drop function
    /// # Safety
    ///
    /// `index` must be a valid index
    pub unsafe fn swap_remove_forget_unchecked(&mut self, index: usize) {
        debug_assert!(index < self.len);

        ptr::copy(
            self.get_unchecked(self.len - 1),
            self.get_unchecked_mut(index),
            self.type_info.layout.size(),
        );

        self.len -= 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn type_info(&self) -> TypeInfo {
        self.type_info
    }

    /// # Safety
    ///
    /// - `index` must be a valid index
    /// - `value` must be a valid pointer to a object with a correct type
    unsafe fn initialize_unchecked(&mut self, index: usize, value: NonNull<u8>) {
        ptr::copy_nonoverlapping(
            value.as_ptr(),
            self.get_unchecked_mut(index),
            self.type_info.layout.size(),
        );
    }

    /// # Safety
    ///
    /// Index must be a valid index
    pub unsafe fn get_unchecked(&self, index: usize) -> *const u8 {
        debug_assert!(index < self.len);
        self.data.as_ptr().add(index * self.type_info.layout.size())
    }

    /// # Safety
    ///
    /// Index must be a valid index
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> *mut u8 {
        debug_assert!(index < self.len);
        self.data.as_ptr().add(index * self.type_info.layout.size())
    }

    /// Clear the vector without resizing the internal buffer
    pub fn clear(&mut self) {
        for i in 0..self.len {
            unsafe {
                let element = self.get_unchecked_mut(i);
                (self.type_info.drop_fn)(element)
            }
        }
        self.len = 0;
    }

    fn reserve_exact(&mut self, additional: usize) {
        let available = self.capacity - self.len;
        if available < additional {
            let grow = additional - available;
            self.grow_exact(grow);
        }
    }

    fn grow_exact(&mut self, additional: usize) {
        assert_ne!(
            self.type_info.layout.size(),
            0,
            "Can't grow a zero sized type"
        );

        let new_capacity = self.capacity + additional;
        let new_layout = Layout::from_size_align(
            self.type_info.layout.size() * new_capacity,
            self.type_info.layout.align(),
        )
        .unwrap();

        let new_data = if self.capacity == 0 {
            unsafe { std::alloc::alloc(new_layout) }
        } else {
            unsafe {
                let layout =
                    Layout::from_size_align(self.capacity, self.type_info.layout.align()).unwrap();
                std::alloc::realloc(self.data.as_ptr(), layout, new_layout.size())
            }
        };

        self.data = NonNull::new(new_data).expect("Failed to allocate data");
        self.capacity = new_capacity;
    }
}

impl Drop for TypeErasedVec {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::erased_vec::{TypeErasedVec, TypeInfo};
    use std::mem::forget;
    use std::ptr::NonNull;

    #[test]
    fn push() {
        let mut numbers: Vec<i32> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut vec = TypeErasedVec::new(TypeInfo::new::<i32>());
        unsafe {
            vec.push(NonNull::new(numbers.as_mut_ptr().add(0).cast::<u8>()).unwrap());
            vec.push(NonNull::new(numbers.as_mut_ptr().add(1).cast::<u8>()).unwrap());
            vec.push(NonNull::new(numbers.as_mut_ptr().add(2).cast::<u8>()).unwrap());

            let at = |index| {
                *(vec.get_unchecked(index) as *const i32)
                    .as_ref()
                    .unwrap_unchecked()
            };

            assert_eq!(vec.len(), 3);
            assert_eq!(at(0), numbers[0]);
            assert_eq!(at(1), numbers[1]);
            assert_eq!(at(2), numbers[2]);
        }

        vec.clear();
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn drop() {
        use std::cell::Cell;
        use std::rc::Rc;

        struct DropTest {
            counter: Rc<Cell<u8>>,
        }

        impl DropTest {
            fn new(counter: Rc<Cell<u8>>) -> Self {
                Self { counter }
            }
        }

        impl Drop for DropTest {
            fn drop(&mut self) {
                self.counter.set(0);
            }
        }

        let mut vec = TypeErasedVec::new(TypeInfo::new::<DropTest>());
        let counter = Rc::new(Cell::new(1));
        let mut test = DropTest::new(counter.clone());
        unsafe {
            vec.push(NonNull::new(&mut test as *mut DropTest as *mut u8).unwrap());
        }
        forget(test);

        unsafe {
            vec.swap_remove_unchecked(0);
        }

        assert_eq!(counter.get(), 0);
        assert_eq!(vec.len(), 0);
    }
}
