use crate::iter::enumerate::Enumerate;
use crate::iter::producer::{Producer, UnindexedConsumer};
use crate::iter::zip::Zip;

/// Parallel variant of [`std::iter::Iterator`]
pub trait ParallelIterator: Sized + Send {
    type Item: Send;

    /// Iterate over an iterator in parallel by splitting in jobs
    /// ```
    /// use std::mem::forget;
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicI32, Ordering};
    /// use ze_jobsystem::{JobSystem, try_initialize_global};
    /// use ze_jobsystem::prelude::*;
    /// forget(try_initialize_global(JobSystem::new(JobSystem::cpu_thread_count() - 1)));
    ///
    /// let v = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    /// let sum = Arc::new(AtomicI32::new(0));
    /// v.par_iter().for_each(|x| {
    ///     sum.fetch_add(*x, Ordering::SeqCst);
    /// });
    /// assert_eq!(sum.load(Ordering::SeqCst), v.iter().sum());
    /// ```
    fn for_each<F: Fn(Self::Item) + Send + Sync>(self, f: F) {
        for_each::for_each(self, &f)
    }

    fn produce_unindexed<C: UnindexedConsumer<Self::Item>>(self, consumer: C) -> C::Result;
}

/// Parallel variant of [`std::iter::IntoIterator`]
pub trait IntoParallelIterator {
    type Iter: ParallelIterator<Item = Self::Item>;
    type Item: Send;

    fn into_par_iter(self) -> Self::Iter;
}

/// [`ParallelIterator`] that supports indexing
pub trait IndexedParallelIterator: ParallelIterator {
    type Producer: Producer<Item = Self::Item>;

    /// Zip two iterators to form one. See [`std::iter::Iterator::zip`]
    /// ```
    /// use std::mem::forget;
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicI32, Ordering};
    /// use ze_jobsystem::{try_initialize_global, JobSystem};
    /// use ze_jobsystem::prelude::*;
    ///
    /// forget(try_initialize_global(JobSystem::new(JobSystem::cpu_thread_count() - 1)));
    ///
    /// let a = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    /// let b = [11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
    /// let sum = Arc::new(AtomicI32::new(0));
    /// a.par_iter().zip(&b).for_each(|(a, b)| {
    ///     sum.fetch_add(*a + *b, Ordering::SeqCst);
    /// });
    /// let par_sum = sum.load(Ordering::SeqCst);
    /// let sum = a.iter().zip(b.iter()).fold(0, |acc, (a, b)| acc + a + b);
    /// assert_eq!(par_sum, sum);
    /// ```
    fn zip<I>(self, other: I) -> Zip<Self, I::Iter>
    where
        I: IntoParallelIterator,
        I::Iter: IndexedParallelIterator,
    {
        Zip::new(self, other.into_par_iter())
    }

    /// An iterator that yields the current count and the element during iteration
    /// ```
    /// use std::mem::forget;
    /// use std::sync::{Arc, Mutex};
    /// use std::sync::atomic::{AtomicI32, Ordering};
    /// use ze_jobsystem::{try_initialize_global, JobSystem};
    /// use ze_jobsystem::prelude::*;
    ///
    /// forget(try_initialize_global(JobSystem::new(JobSystem::cpu_thread_count() - 1)));
    /// let v = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    /// let indices = Mutex::new(vec![0; v.len()]);
    /// v.par_iter().enumerate().for_each(|(i, _)| {
    ///     indices.lock().unwrap()[i] = i;
    /// });
    ///
    /// for i in indices.lock().unwrap().iter() {
    ///     assert_eq!(*i, v[*i]);
    /// }
    /// ```
    fn enumerate(self) -> Enumerate<Self> {
        Enumerate::new(self)
    }

    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn producer(self) -> Self::Producer;
}

mod enumerate;
mod for_each;
pub mod prelude;
mod producer;
pub mod slice;
pub mod vec;
mod zip;
