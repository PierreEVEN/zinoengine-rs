use crate::iter::producer::{connect_iter_to_consumer, Producer, UnindexedConsumer};
use crate::iter::{IndexedParallelIterator, ParallelIterator};
use std::ops::Range;

/// An iterator that yields the current count and the element during iteration like [`std::iter::Enumerate`]
pub struct Enumerate<I: IndexedParallelIterator> {
    iter: I,
}

impl<I: IndexedParallelIterator> Enumerate<I> {
    pub(super) fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<I: IndexedParallelIterator> ParallelIterator for Enumerate<I> {
    type Item = (usize, I::Item);

    fn produce_unindexed<C: UnindexedConsumer<Self::Item>>(self, consumer: C) -> C::Result {
        connect_iter_to_consumer(self, consumer)
    }
}

impl<I: IndexedParallelIterator> IndexedParallelIterator for Enumerate<I> {
    type Producer = EnumerateProducer<I::Producer>;

    fn len(&self) -> usize {
        self.iter.len()
    }

    fn producer(self) -> Self::Producer {
        EnumerateProducer {
            producer: self.iter.producer(),
            offset: 0,
        }
    }
}

pub struct EnumerateProducer<P: Producer> {
    producer: P,
    offset: usize,
}

impl<P: Producer> Producer for EnumerateProducer<P> {
    type Item = (usize, P::Item);
    type IntoIter = std::iter::Zip<Range<usize>, P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        let iter = self.producer.into_iter();
        let end = self.offset + iter.len();
        (self.offset..end).zip(iter)
    }

    fn len(&self) -> usize {
        self.producer.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.producer.split_at(index);
        (
            EnumerateProducer {
                producer: left,
                offset: self.offset,
            },
            EnumerateProducer {
                producer: right,
                offset: self.offset + index,
            },
        )
    }
}
