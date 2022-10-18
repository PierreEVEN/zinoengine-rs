use crate::allocator::Allocator;
use crossbeam::deque::{Injector, Stealer, Worker};
use once_cell::sync::OnceCell;
use parking_lot::{Condvar, Mutex};
use std::fmt::{Debug, Formatter};
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::panic::AssertUnwindSafe;
use std::process::abort;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{mem, ptr};
use std::{panic, thread};
use ze_core::ze_info;

const MAX_CONTINUATIONS: usize = 16;
const MAX_USERDATA_SIZE: usize = 128;

/// Maximum amount of jobs allocated at anytime per thread
const MAX_JOB_COUNT_PER_THREAD: usize = 4096;

/// A shared handle to a job, each job manage a refcount
/// This allows the user to store `JobHandle` with no problems of jobs being recycled
#[derive(PartialEq, Eq, Debug)]
pub struct JobHandle {
    ptr: *mut Job,
}

impl JobHandle {
    pub fn new(ptr: *mut Job) -> Self {
        Self { ptr }
    }
}

impl Clone for JobHandle {
    fn clone(&self) -> Self {
        unsafe {
            (*self.ptr).refcount.fetch_add(1, Ordering::SeqCst);
        }
        Self { ptr: self.ptr }
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        unsafe {
            let job = &mut (*self.ptr);
            job.refcount.fetch_sub(1, Ordering::SeqCst);
            if job.refcount.load(Ordering::SeqCst) == 0 {
                debug_assert!(job.is_finished());
                Job::free(self);
            }
        }
    }
}

impl From<&mut Job> for JobHandle {
    fn from(job: &mut Job) -> Self {
        job.refcount.fetch_add(1, Ordering::SeqCst);

        Self {
            ptr: job as *mut Job,
        }
    }
}

impl Deref for JobHandle {
    type Target = Job;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl DerefMut for JobHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

unsafe impl Send for JobHandle {}

#[repr(align(64))]
pub struct Job {
    jobsystem: *mut JobSystem,

    /// Used by `JobHandle`
    refcount: AtomicU32,
    parent: Option<JobHandle>,
    function: Option<fn(job: &mut JobHandle)>,

    unfinished_jobs: AtomicU8,
    continuation_count: AtomicU8,
    continuations: [Option<JobHandle>; MAX_CONTINUATIONS],
    userdata: [u8; MAX_USERDATA_SIZE],
}

impl Debug for Job {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("refcount", &self.refcount)
            .field("unfinished_jobs", &self.unfinished_jobs)
            .field("continuation_count", &self.continuation_count)
            .field("continuations", &self.continuations)
            .finish()
    }
}

impl Job {
    pub fn new_inner(jobsystem: &JobSystem) -> Self {
        const EMPTY_JOB: Option<JobHandle> = None;
        let continuations = [EMPTY_JOB; MAX_CONTINUATIONS];

        Self {
            jobsystem: (jobsystem as *const JobSystem) as *mut JobSystem,
            refcount: Default::default(),
            parent: None,
            function: None,
            unfinished_jobs: AtomicU8::new(0),
            continuation_count: Default::default(),
            continuations,
            userdata: [Default::default(); MAX_USERDATA_SIZE],
        }
    }

    pub fn free(job: &mut Job) {
        drop(unsafe { ptr::read(job) });
    }

    fn execute(job: &mut JobHandle) {
        let func = job.function.unwrap();
        func(job);
        job.finish();
    }

    pub fn schedule(&mut self) {
        self.unfinished_jobs.fetch_add(1, Ordering::SeqCst);
        let jobsystem = unsafe { self.jobsystem.as_mut().unwrap() };
        jobsystem
            .shared_worker_data
            .injector()
            .push(JobHandle::from(self));
        jobsystem.shared_worker_data.sleep_condvar().notify_all();
    }

    /// Add a continuation job that will be scheduled when this job finishes
    pub fn add_continuation(&mut self, job: &JobHandle) {
        debug_assert!(
            (self.continuation_count.load(Ordering::SeqCst) as usize) < MAX_CONTINUATIONS
        );
        self.continuations[self.continuation_count.load(Ordering::SeqCst) as usize] =
            Some(job.clone());
        self.continuation_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn wait(&mut self) {
        while !self.is_finished() {
            let jobsystem = unsafe { self.jobsystem.as_mut().unwrap() };
            jobsystem.shared_worker_data.sleep_condvar().notify_one();
        }
    }

    fn finish(&mut self) {
        self.unfinished_jobs.fetch_sub(1, Ordering::SeqCst);

        if self.is_finished() {
            if let Some(parent) = &mut self.parent {
                parent.finish();
            }

            // Schedule dependents
            for continuation in &mut self.continuations {
                if let Some(mut continuation) = continuation.take() {
                    continuation.schedule();
                }
            }

            if self.refcount.load(Ordering::SeqCst) == 0 {
                Job::free(self);
            }
        }
    }

    fn is_finished(&self) -> bool {
        self.unfinished_jobs.load(Ordering::SeqCst) == 0
    }
}

unsafe impl Send for Job {}

#[derive(Debug)]
struct SharedWorkerData {
    injector: Injector<JobHandle>,

    /// Stealer from each thread pool so a worker can steal jobs from another worker safely
    stealers: Vec<Stealer<JobHandle>>,

    sleep_condvar: Condvar,
    sleep_mutex: Mutex<()>,

    jobsystem_dropped: AtomicBool,
}

impl SharedWorkerData {
    fn new(stealers: Vec<Stealer<JobHandle>>) -> Self {
        Self {
            injector: Injector::new(),
            stealers,
            sleep_condvar: Condvar::new(),
            sleep_mutex: Mutex::new(()),
            jobsystem_dropped: AtomicBool::new(false),
        }
    }

    fn injector(&self) -> &Injector<JobHandle> {
        &self.injector
    }

    fn stealers(&self) -> &Vec<Stealer<JobHandle>> {
        &self.stealers
    }

    fn sleep_condvar(&self) -> &Condvar {
        &self.sleep_condvar
    }

    fn sleep_mutex(&self) -> &Mutex<()> {
        &self.sleep_mutex
    }
}

pub struct WorkerThread {
    thread: Option<JoinHandle<()>>,
}

impl WorkerThread {
    fn thread_main(job_queue: Worker<JobHandle>, shared_worker_data: Arc<SharedWorkerData>) {
        ze_core::thread::set_thread_name(
            thread::current().id(),
            thread::current().name().unwrap().to_string(),
        );

        loop {
            if shared_worker_data.jobsystem_dropped.load(Ordering::SeqCst) {
                return;
            }

            // Try to pop a job from our local queue
            // If it's empty, try to steal a batch of jobs of the global queue
            // If it's empty, steal from other workers
            if let Some(mut job) = job_queue.pop().or_else(|| {
                std::iter::repeat_with(|| {
                    let shared_worker_data = shared_worker_data.as_ref();
                    shared_worker_data
                        .injector()
                        .steal_batch_and_pop(&job_queue)
                        .or_else(|| {
                            shared_worker_data
                                .stealers()
                                .iter()
                                .map(|stealer| stealer.steal())
                                .collect()
                        })
                })
                .find(|stealer| !stealer.is_retry())
                .and_then(|stealer| stealer.success())
            }) {
                panic::catch_unwind(AssertUnwindSafe(|| {
                    Job::execute(&mut job);
                }))
                .unwrap_or_else(|_| {
                    abort();
                });
            } else {
                // Nothing :( so sleep until another job is here!
                let mut guard = shared_worker_data.sleep_mutex().lock();
                shared_worker_data.sleep_condvar().wait(&mut guard);
            }
        }
    }

    fn new(
        index: usize,
        job_queue: Worker<JobHandle>,
        shared_worker_data: Arc<SharedWorkerData>,
    ) -> Self {
        Self {
            thread: Some(
                thread::Builder::new()
                    .name(format!("Worker Thread {}", index))
                    .spawn(move || {
                        WorkerThread::thread_main(job_queue, shared_worker_data);
                    })
                    .unwrap(),
            ),
        }
    }
}

/// A stealing jobsystem
#[derive(Debug)]
pub struct JobSystem {
    worker_threads: MaybeUninit<Vec<WorkerThread>>,
    allocator: Allocator<Job>,
    shared_worker_data: Arc<SharedWorkerData>,
}

impl JobSystem {
    pub fn new(count: usize) -> Arc<Self> {
        ze_info!("Creating job system with {} workers", count);

        let mut queues = Vec::with_capacity(count);
        let mut stealers = Vec::with_capacity(count);
        for _ in 0..count {
            let queue = Worker::new_fifo();
            stealers.push(queue.stealer());
            queues.push(queue);
        }

        let shared_worker_data = Arc::new(SharedWorkerData::new(stealers));
        let mut worker_threads = Vec::with_capacity(count);

        for (i, queue) in queues.drain(..).enumerate() {
            worker_threads.push(WorkerThread::new(i, queue, shared_worker_data.clone()));
        }

        Arc::new(Self {
            worker_threads: MaybeUninit::new(worker_threads),
            allocator: Allocator::new(MAX_JOB_COUNT_PER_THREAD),
            shared_worker_data,
        })
    }

    pub fn spawn<F, R>(&self, f: F) -> JobHandle
    where
        F: FnOnce(&mut JobSystem, &JobHandle) -> R,
        F: Send + 'static,
    {
        // SAFETY: Lifetime is statically checked thanks to the 'static lifetime bound
        unsafe { self.spawn_unchecked(f) }
    }

    /// Create a new child job
    /// Childs jobs must be completed before the parent can finish
    pub fn spawn_child<F>(&self, parent: &JobHandle, f: F) -> JobHandle
    where
        F: FnOnce(&mut JobSystem, &JobHandle),
        F: Send + 'static,
    {
        let mut job = self.spawn(f);
        job.parent = Some(parent.clone());
        parent.unfinished_jobs.fetch_add(1, Ordering::SeqCst);
        job
    }

    /// Schedule two function to be executed in jobs, waiting for the result of both
    /// ```
    /// let jobsystem = ze_jobsystem::JobSystem::new(ze_jobsystem::JobSystem::cpu_thread_count() - 1);
    /// let mut a = 0;
    /// let mut b = 0;
    /// jobsystem.join(|| a = 20, || b = 30);
    /// assert_eq!(a, 20);
    /// assert_eq!(b, 30);
    /// ```
    pub fn join<F1, F2, R1, R2>(&self, f1: F1, f2: F2) -> (R1, R2)
    where
        F1: FnOnce() -> R1 + Send,
        F2: FnOnce() -> R2 + Send,
        R1: Send,
        R2: Send,
    {
        let mut left_result = MaybeUninit::uninit();
        let mut right_result = MaybeUninit::uninit();

        // SAFETY: Lifetimes are guaranteed by the fact that we wait for the jobs to finish after scheduling them
        let (mut left, mut right) = unsafe {
            let left = self.spawn_unchecked(|_, _| {
                left_result.write(f1());
            });

            let right = self.spawn_unchecked(|_, _| {
                right_result.write(f2());
            });

            (left, right)
        };

        left.schedule();
        right.schedule();
        left.wait();
        right.wait();

        // SAFETY: Jobs are finished, results are initialized
        unsafe { (left_result.assume_init(), right_result.assume_init()) }
    }

    /// Spawn a job, without any lifetime constraints
    /// # Safety
    /// The function or caller must guarantee correct data lifetime management
    pub unsafe fn spawn_unchecked<F, R>(&self, f: F) -> JobHandle
    where
        F: FnOnce(&mut JobSystem, &JobHandle) -> R,
        F: Send,
    {
        debug_assert!(
            mem::size_of_val(&f) <= MAX_USERDATA_SIZE,
            "Userdata max size exceeded! {} out of {} bytes max.",
            mem::size_of_val(&f),
            MAX_USERDATA_SIZE
        );

        let mut job = Job::new_inner(self);

        let userdata_ptr = job.userdata.as_mut_ptr() as *mut F;
        unsafe {
            userdata_ptr.write(f);
        }

        job.function = Some(|job| {
            let func = unsafe {
                let mut dst: MaybeUninit<F> = MaybeUninit::zeroed();
                let ptr = job.userdata.as_mut_ptr() as *mut F;
                dst.write(ptr.read());
                dst.assume_init()
            };
            unsafe {
                func(job.jobsystem.as_mut().unwrap(), job);
            }
        });

        JobHandle::from(self.allocator.allocate(job))
    }

    pub fn job_allocator(&mut self) -> &mut Allocator<Job> {
        &mut self.allocator
    }

    pub fn cpu_thread_count() -> usize {
        num_cpus::get()
    }
}

impl Drop for JobSystem {
    fn drop(&mut self) {
        let worker_threads =
            unsafe { mem::replace(&mut self.worker_threads, MaybeUninit::uninit()).assume_init() };

        self.shared_worker_data
            .jobsystem_dropped
            .store(true, Ordering::SeqCst);

        self.shared_worker_data.sleep_condvar.notify_all();

        for worker in &worker_threads {
            while !worker.thread.as_ref().unwrap().is_finished() {
                self.shared_worker_data.sleep_condvar.notify_all();
            }
        }
    }
}

unsafe impl Sync for JobSystem {}

static GLOBAL_JOBSYSTEM: OnceCell<Arc<JobSystem>> = OnceCell::new();

/// Get the global jobsystem
/// Panic if it's not initialized
pub fn global() -> &'static Arc<JobSystem> {
    GLOBAL_JOBSYSTEM
        .get()
        .expect("Global jobsystem was not initialized")
}

pub fn initialize_global(jobsystem: Arc<JobSystem>) {
    GLOBAL_JOBSYSTEM
        .set(jobsystem)
        .expect("Global jobsystem was already initialized");
}

pub fn try_initialize_global(jobsystem: Arc<JobSystem>) -> Result<(), Arc<JobSystem>> {
    GLOBAL_JOBSYSTEM.set(jobsystem)
}

pub mod allocator;
pub mod iter;

pub mod prelude;
#[cfg(test)]
mod tests;
