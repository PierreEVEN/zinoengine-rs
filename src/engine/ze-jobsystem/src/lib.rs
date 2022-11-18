use crate::job::{JobHandle, MAX_CONTINUATIONS, MAX_USERDATA_SIZE};
use crate::job_allocator::JobAllocator;
use crate::worker_thread::WorkerThread;
use crossbeam::deque::{Injector, Stealer, Worker};
use once_cell::sync::OnceCell;
use parking_lot::{Condvar, Mutex};
use std::fmt::Debug;
use std::mem;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ze_core::ze_info;

/// Maximum amount of jobs allocated per thread
const JOB_CAPACITY_PER_THREAD: usize = 2048;

#[derive(Debug)]
struct SharedWorkerData {
    injector: Injector<JobHandle>,
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

    #[inline]
    fn schedule_job(&self, job: JobHandle) {
        job.unfinished_jobs.fetch_add(1, Ordering::SeqCst);
        self.injector.push(job);
        self.sleep_condvar.notify_all();
    }

    #[inline]
    fn has_any_jobs(&self) -> bool {
        !self.injector.is_empty() || self.stealers.iter().any(|stealer| !stealer.is_empty())
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

/// A stealing jobsystem
#[derive(Debug)]
pub struct JobSystem {
    job_allocator: JobAllocator,
    worker_threads: Vec<WorkerThread>,
    shared_worker_data: Arc<SharedWorkerData>,
}

impl JobSystem {
    pub fn new(worker_count: usize) -> Arc<Self> {
        ze_info!("Creating job system with {} workers", worker_count);

        let mut queues = Vec::with_capacity(worker_count);
        let mut stealers = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            let queue = Worker::new_fifo();
            stealers.push(queue.stealer());
            queues.push(queue);
        }

        let shared_worker_data = Arc::new(SharedWorkerData::new(stealers));
        let mut worker_threads = Vec::with_capacity(worker_count);
        for (i, queue) in queues.drain(..).enumerate() {
            worker_threads.push(WorkerThread::new(i, queue, shared_worker_data.clone()));
        }

        let jobsystem = Arc::new(Self {
            worker_threads,
            job_allocator: JobAllocator::with_capacity(JOB_CAPACITY_PER_THREAD),
            shared_worker_data,
        });

        jobsystem
    }

    pub fn spawn<F>(&self, f: F) -> JobBuilder
    where
        F: FnOnce(&JobSystem, JobHandle),
        F: Send + 'static,
    {
        // SAFETY: Lifetime is statically checked thanks to the 'static lifetime bound
        unsafe { self.spawn_unchecked(f) }
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
        let (left, right) = unsafe {
            let left = self.spawn_unchecked(|_, _| {
                left_result.write(f1());
            });

            let right = self.spawn_unchecked(|_, _| {
                right_result.write(f2());
            });

            (left.schedule(), right.schedule())
        };

        self.wait_for(&[left, right]);

        // SAFETY: Jobs are finished, results are initialized
        unsafe { (left_result.assume_init(), right_result.assume_init()) }
    }

    /// Spawn a job, without any lifetime constraints
    ///
    /// # Safety
    ///
    /// The function or caller must guarantee correct data lifetime management
    pub unsafe fn spawn_unchecked<F>(&self, f: F) -> JobBuilder
    where
        F: FnOnce(&JobSystem, JobHandle),
        F: Send,
    {
        struct PackedUserdata<F> {
            jobsystem: *const JobSystem,
            f: F,
        }

        assert!(
            mem::size_of::<PackedUserdata<F>>() <= MAX_USERDATA_SIZE,
            "Userdata max size exceeded! {} out of {} bytes max.",
            mem::size_of::<PackedUserdata<F>>(),
            MAX_USERDATA_SIZE
        );

        let mut job = self
            .job_allocator
            .allocate()
            .expect("Job allocator is full! TODO: Wait for jobs");

        let userdata_ptr = job.userdata.as_mut_ptr() as *mut PackedUserdata<F>;
        unsafe {
            userdata_ptr.write(PackedUserdata {
                jobsystem: self as *const JobSystem,
                f,
            });
        }

        job.function = MaybeUninit::new(|mut job| {
            let ptr = job.userdata.as_mut_ptr() as *mut PackedUserdata<F>;
            let userdata = unsafe { ptr.read() };

            (userdata.f)(&*userdata.jobsystem, job);
        });

        JobBuilder::new(self, job)
    }

    /// Add a continuation to the job. Can be added while the job is running.
    /// A job must have less than [`MAX_CONTINUATIONS`] continuations
    pub fn add_continuation(&self, job: &mut JobHandle, continuation: JobHandle) {
        let index = job.continuation_count.fetch_add(1, Ordering::Relaxed) as usize;
        assert!(index + 1 < MAX_CONTINUATIONS);
        job.continuations[index] = MaybeUninit::new(continuation);
    }

    /// Wait for the given jobs to finish
    pub fn wait_for(&self, jobs: &[JobHandle]) {
        loop {
            if jobs.iter().all(|job| job.is_finished()) {
                break;
            }

            self.shared_worker_data.sleep_condvar().notify_one();

            if let Some(job) = std::iter::repeat_with(|| {
                self.shared_worker_data.injector().steal().or_else(|| {
                    std::thread::yield_now();

                    self.shared_worker_data
                        .stealers()
                        .iter()
                        .map(|stealer| stealer.steal())
                        .collect()
                })
            })
            .find(|stealer| !stealer.is_retry())
            .and_then(|stealer| stealer.success())
            {
                job::execute(job, &self.shared_worker_data);
            }
        }
    }

    pub fn wait_until_idle(&self) {
        while self.shared_worker_data.has_any_jobs() {
            self.shared_worker_data.sleep_condvar().notify_one();

            if let Some(job) = std::iter::repeat_with(|| {
                self.shared_worker_data.injector().steal().or_else(|| {
                    std::thread::yield_now();

                    self.shared_worker_data
                        .stealers()
                        .iter()
                        .map(|stealer| stealer.steal())
                        .collect()
                })
            })
            .find(|stealer| !stealer.is_retry())
            .and_then(|stealer| stealer.success())
            {
                job::execute(job, &self.shared_worker_data);
            }
        }
    }

    pub fn schedule(&self, job: JobHandle) {
        self.shared_worker_data.schedule_job(job);
    }

    pub fn cpu_thread_count() -> usize {
        num_cpus::get()
    }
}

impl Drop for JobSystem {
    fn drop(&mut self) {
        let worker_threads = mem::take(&mut self.worker_threads);

        self.shared_worker_data
            .jobsystem_dropped
            .store(true, Ordering::SeqCst);

        self.shared_worker_data.sleep_condvar.notify_all();

        for worker in &worker_threads {
            while !worker.is_finished() {
                self.shared_worker_data.sleep_condvar.notify_all();
            }
        }
    }
}

unsafe impl Sync for JobSystem {}

pub struct JobBuilder<'a> {
    jobsystem: &'a JobSystem,
    handle: JobHandle,
}

impl<'a> JobBuilder<'a> {
    fn new(jobsystem: &'a JobSystem, handle: JobHandle) -> Self {
        Self { jobsystem, handle }
    }

    pub fn with_parent(mut self, parent: &JobHandle) -> Self {
        self.handle.parent = Some(*parent);
        parent.unfinished_jobs.fetch_add(1, Ordering::SeqCst);
        self
    }

    pub fn with_continuation(mut self, continuation: impl IntoContinuation) -> Self {
        self.jobsystem
            .add_continuation(&mut self.handle, continuation.into_continuation());
        self
    }

    pub fn schedule(self) -> JobHandle {
        self.jobsystem.schedule(self.handle);
        self.handle
    }
}

pub trait IntoContinuation {
    fn into_continuation(self) -> JobHandle;
}

impl<'a> IntoContinuation for JobBuilder<'a> {
    fn into_continuation(self) -> JobHandle {
        self.handle
    }
}

static GLOBAL_JOBSYSTEM: OnceCell<Arc<JobSystem>> = OnceCell::new();

/// Get the global jobsystem
/// Panic if it's not initialized
#[inline]
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

pub mod iter;
mod job;
mod job_allocator;
pub mod prelude;
#[cfg(test)]
mod tests;
mod worker_thread;
