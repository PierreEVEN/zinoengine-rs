//! Producer-consumer pattern as seen in the `rayon` crate
use crate::global;
use crate::iter::IndexedParallelIterator;

/// Produce a value that can be converted into a Iterator and also can be split up
pub trait Producer: Send + Sized {
    type Item: Send;
    type IntoIter: Iterator<Item = Self::Item> + DoubleEndedIterator + ExactSizeIterator;

    fn into_iter(self) -> Self::IntoIter;

    /// Number of item that the producer will produce sequentially
    fn len(&self) -> usize;

    /// Split the producer into two producers
    fn split_at(self, index: usize) -> (Self, Self);

    /// Feed the folder until it is full
    fn fold_with<F: Folder<Self::Item>>(self, folder: F) -> F {
        folder.consume_iter(self.into_iter())
    }
}

/// Consume items, can be split up and converted into a `Folder`
pub trait Consumer<Item>: Send + Sized {
    type Folder: Folder<Item, Result = Self::Result>;
    type Reducer: Reducer<Self::Result>;
    type Result: Send;

    /// Spit in half the consumer, returning also a reducer to combine it later on
    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer);
    fn into_folder(self) -> Self::Folder;
    fn full(&self) -> bool;
}

/// Consumer that can be split up without specifying an index
pub trait UnindexedConsumer<Item>: Consumer<Item> {
    /// Split the consumer, returning an half ot it
    fn split(&self) -> Self;
    fn to_reducer(&self) -> Self::Reducer;
}

/// Consume items sequentially
pub trait Folder<Item>: Sized {
    type Result;

    /// Consume the item, returning a updated Folder
    fn consume(self, item: Item) -> Self;

    /// Consume until full
    fn consume_iter(mut self, iter: impl IntoIterator<Item = Item>) -> Self {
        for item in iter {
            self = self.consume(item);
            if self.full() {
                break;
            }
        }
        self
    }

    /// Finish consuming items
    fn complete(self) -> Self::Result;
    fn full(&self) -> bool;
}

/// Reduce combine two results (e.g two consumers) into one
pub trait Reducer<Result> {
    fn reduce(self, left: Result, right: Result) -> Result;
}

pub struct EmptyReducer;

impl Reducer<()> for EmptyReducer {
    fn reduce(self, _: (), _: ()) {}
}

#[derive(Copy, Clone)]
struct Splitter {
    splits: usize,
    min_splits: usize,
}

impl Default for Splitter {
    fn default() -> Self {
        Self {
            splits: num_cpus::get(),
            min_splits: 1,
        }
    }
}

impl Splitter {
    fn try_split(&mut self) -> bool {
        if self.splits > self.min_splits {
            self.splits /= 2;
            true
        } else {
            false
        }
    }
}

/// Connect a [`IndexedParallelIterator`] to a [`Consumer`]
pub(crate) fn connect_iter_to_consumer<I: IndexedParallelIterator, C: Consumer<I::Item>>(
    iter: I,
    consumer: C,
) -> C::Result {
    connect_producer_to_consumer(iter.len(), iter.producer(), consumer)
}

/// Connect a [`Producer`] to a [`Consumer`], effectively spawning jobs
pub(crate) fn connect_producer_to_consumer<P: Producer, C: Consumer<P::Item>>(
    len: usize,
    producer: P,
    consumer: C,
) -> C::Result {
    fn splitter_impl<P: Producer, C: Consumer<P::Item>>(
        len: usize,
        mut splitter: Splitter,
        producer: P,
        consumer: C,
    ) -> C::Result {
        if consumer.full() {
            consumer.into_folder().complete()
        } else if splitter.try_split() {
            let mid = len / 2;
            let (left_producer, right_producer) = producer.split_at(mid);
            let (left_consumer, right_consumer, reducer) = consumer.split_at(mid);
            let (left, right) = global().join(
                || splitter_impl(mid, splitter, left_producer, left_consumer),
                || splitter_impl(len - mid, splitter, right_producer, right_consumer),
            );
            reducer.reduce(left, right)
        } else {
            producer.fold_with(consumer.into_folder()).complete()
        }
    }

    let splitter = Splitter::default();
    splitter_impl(len, splitter, producer, consumer)
}
