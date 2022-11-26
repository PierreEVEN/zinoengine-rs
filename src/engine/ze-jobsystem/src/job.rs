use crate::SharedWorkerData;
use std::cell::UnsafeCell;
use std::fmt::{Debug, Formatter};
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::sync::atomic::{AtomicU8, Ordering};

#[repr(transparent)]
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct JobHandle(pub(crate) *const UnsafeCell<Job>);

impl JobHandle {
    pub fn is_finished(&self) -> bool {
        self.unfinished_jobs.load(Ordering::SeqCst) == 0
    }
}

impl Deref for JobHandle {
    type Target = Job;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(*self.0).get() }
    }
}

impl DerefMut for JobHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(*self.0).get() }
    }
}

unsafe impl Send for JobHandle {}

pub const MAX_CONTINUATIONS: usize = 16;
pub const MAX_USERDATA_SIZE: usize = 128;

#[repr(align(64))]
pub struct Job {
    pub(crate) parent: Option<JobHandle>,
    pub(crate) function: MaybeUninit<fn(JobHandle)>,
    pub(crate) unfinished_jobs: AtomicU8,
    pub(crate) continuation_count: AtomicU8,
    pub(crate) continuations: [MaybeUninit<JobHandle>; MAX_CONTINUATIONS],
    pub(crate) userdata: [u8; MAX_USERDATA_SIZE],
}

impl Debug for Job {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("unfinished_jobs", &self.unfinished_jobs)
            .field("continuation_count", &self.continuation_count)
            .field("continuations", &self.continuations)
            .finish()
    }
}

impl Default for Job {
    fn default() -> Self {
        Self {
            parent: None,
            function: MaybeUninit::uninit(),
            unfinished_jobs: AtomicU8::new(0),
            continuation_count: Default::default(),
            continuations: [MaybeUninit::uninit(); MAX_CONTINUATIONS],
            userdata: [Default::default(); MAX_USERDATA_SIZE],
        }
    }
}

#[inline]
pub(crate) fn execute(job: JobHandle, shared_worker_data: &SharedWorkerData) {
    {
        let func = unsafe { job.function.assume_init() };
        func(job);
    }

    finish(job, shared_worker_data);
}

#[inline]
pub(crate) fn finish(job: JobHandle, shared_worker_data: &SharedWorkerData) {
    let old = job.unfinished_jobs.fetch_sub(1, Ordering::SeqCst);
    if old == 1 {
        if let Some(parent) = job.parent {
            finish(parent, shared_worker_data);
        }

        for i in 0..job.continuation_count.load(Ordering::SeqCst) {
            let continuation = unsafe { job.continuations[i as usize].assume_init() };
            shared_worker_data.schedule_job(continuation);
        }

        unsafe {
            ptr::drop_in_place((*job.0).get());
        }
    }
}
