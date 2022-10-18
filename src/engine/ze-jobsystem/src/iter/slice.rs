use crate::iter::producer::{connect_iter_to_consumer, Producer, UnindexedConsumer};
use crate::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

pub struct Iter<'s, T: Sync> {
    slice: &'s [T],
}

impl<'s, T: Sync> Iter<'s, T> {
    pub fn new(slice: &'s [T]) -> Self {
        Self { slice }
    }
}

impl<'s, T: Sync> ParallelIterator for Iter<'s, T> {
    type Item = &'s T;

    fn produce_unindexed<C: UnindexedConsumer<Self::Item>>(self, consumer: C) -> C::Result {
        connect_iter_to_consumer(self, consumer)
    }
}

impl<'s, T: Sync> IndexedParallelIterator for Iter<'s, T> {
    type Producer = IterProducer<'s, T>;

    fn len(&self) -> usize {
        self.slice.len()
    }

    fn producer(self) -> Self::Producer {
        IterProducer { slice: self.slice }
    }
}

pub struct IterProducer<'s, T: Sync> {
    slice: &'s [T],
}

impl<'s, T: Sync> Producer for IterProducer<'s, T> {
    type Item = &'s T;
    type IntoIter = std::slice::Iter<'s, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.slice.iter()
    }

    fn len(&self) -> usize {
        self.slice.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.slice.split_at(index);
        (IterProducer { slice: left }, IterProducer { slice: right })
    }
}

pub trait ParallelSlice<T: Sync> {
    fn par_iter(&self) -> Iter<T>;
}

impl<T: Sync> ParallelSlice<T> for [T] {
    fn par_iter(&self) -> Iter<T> {
        self.into_par_iter()
    }
}

impl<'s, T: Sync> IntoParallelIterator for &'s [T] {
    type Iter = Iter<'s, T>;
    type Item = &'s T;

    fn into_par_iter(self) -> Self::Iter {
        Iter::new(self)
    }
}

impl<'s, const N: usize, T: Sync> IntoParallelIterator for &'s [T; N] {
    type Iter = Iter<'s, T>;
    type Item = &'s T;

    fn into_par_iter(self) -> Self::Iter {
        Iter::new(self)
    }
}
