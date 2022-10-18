use crate::iter::producer::{connect_iter_to_consumer, Producer, UnindexedConsumer};
use crate::iter::{IndexedParallelIterator, ParallelIterator};
use std::cmp;

#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct Zip<A: IndexedParallelIterator, B: IndexedParallelIterator> {
    a: A,
    b: B,
}

impl<A: IndexedParallelIterator, B: IndexedParallelIterator> Zip<A, B> {
    pub(super) fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A: IndexedParallelIterator, B: IndexedParallelIterator> ParallelIterator for Zip<A, B> {
    type Item = (A::Item, B::Item);

    fn produce_unindexed<C: UnindexedConsumer<Self::Item>>(self, consumer: C) -> C::Result {
        connect_iter_to_consumer(self, consumer)
    }
}

impl<A: IndexedParallelIterator, B: IndexedParallelIterator> IndexedParallelIterator for Zip<A, B> {
    type Producer = ZipProducer<A::Producer, B::Producer>;

    fn len(&self) -> usize {
        cmp::min(self.a.len(), self.b.len())
    }

    fn producer(self) -> Self::Producer {
        ZipProducer {
            a: self.a.producer(),
            b: self.b.producer(),
        }
    }
}

pub struct ZipProducer<A: Producer, B: Producer> {
    a: A,
    b: B,
}

impl<A: Producer, B: Producer> Producer for ZipProducer<A, B> {
    type Item = (A::Item, B::Item);
    type IntoIter = std::iter::Zip<A::IntoIter, B::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.a.into_iter().zip(self.b.into_iter())
    }

    fn len(&self) -> usize {
        cmp::min(self.a.len(), self.b.len())
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (a_left, a_right) = self.a.split_at(index);
        let (b_left, b_right) = self.b.split_at(index);
        (
            ZipProducer {
                a: a_left,
                b: b_left,
            },
            ZipProducer {
                a: a_right,
                b: b_right,
            },
        )
    }
}
