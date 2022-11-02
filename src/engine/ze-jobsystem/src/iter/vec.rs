use crate::iter::slice::{Iter, ParallelSlice};
use crate::iter::IntoParallelIterator;

impl<T: Sync> ParallelSlice<T> for Vec<T> {
    fn par_iter(&self) -> Iter<T> {
        self.into_par_iter()
    }
}

impl<'v, T: Sync> IntoParallelIterator for &'v Vec<T> {
    type Iter = Iter<'v, T>;
    type Item = &'v T;

    fn into_par_iter(self) -> Self::Iter {
        Iter::new(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::iter::slice::ParallelSlice;
    use crate::iter::ParallelIterator;
    use crate::{try_initialize_global, JobSystem};
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    #[test]
    fn for_each() {
        let _ = try_initialize_global(JobSystem::new(JobSystem::cpu_thread_count() - 1));

        let v = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let sum = Arc::new(AtomicI32::new(0));
        v.par_iter().for_each(|x| {
            sum.fetch_add(*x, Ordering::SeqCst);
        });

        assert_eq!(sum.load(Ordering::SeqCst), v.iter().sum());
    }
}
