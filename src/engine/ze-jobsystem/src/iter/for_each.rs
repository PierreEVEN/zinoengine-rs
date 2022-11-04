use crate::iter::producer::{Consumer, EmptyReducer, Folder, UnindexedConsumer};
use crate::iter::ParallelIterator;

pub(super) fn for_each<T, I, F>(iter: I, f: &F)
where
    T: Send,
    I: ParallelIterator<Item = T>,
    F: Fn(T) + Sync,
{
    iter.produce_unindexed(ForEachConsumer { f })
}

struct ForEachConsumer<'a, F> {
    f: &'a F,
}

impl<'a, T, F: Fn(T) + Sync> Consumer<T> for ForEachConsumer<'a, F> {
    type Folder = ForEachConsumer<'a, F>;
    type Reducer = EmptyReducer;
    type Result = ();

    fn split_at(self, _: usize) -> (Self, Self, Self::Reducer) {
        (self.split(), self, EmptyReducer)
    }

    fn into_folder(self) -> Self::Folder {
        self
    }

    fn full(&self) -> bool {
        false
    }
}

impl<'a, T, F: Fn(T) + Sync> UnindexedConsumer<T> for ForEachConsumer<'a, F> {
    fn split(&self) -> Self {
        ForEachConsumer { f: self.f }
    }

    fn to_reducer(&self) -> Self::Reducer {
        EmptyReducer
    }
}

impl<'a, T, F: Fn(T) + Sync> Folder<T> for ForEachConsumer<'a, F> {
    type Result = ();

    #[inline]
    fn consume(self, item: T) -> Self {
        (self.f)(item);
        self
    }

    fn complete(self) -> Self::Result {}

    fn full(&self) -> bool {
        false
    }
}
