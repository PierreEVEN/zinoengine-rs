use crate::{job, JobHandle, SharedWorkerData};
use crossbeam::deque::Worker;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug)]
pub(crate) struct WorkerThread {
    thread: JoinHandle<()>,
}

impl WorkerThread {
    pub fn new(
        index: usize,
        job_queue: Worker<JobHandle>,
        shared_worker_data: Arc<SharedWorkerData>,
    ) -> Self {
        Self {
            thread: thread::Builder::new()
                .name(format!("Worker Thread {}", index))
                .spawn(move || {
                    WorkerThread::thread_main(job_queue, shared_worker_data);
                })
                .unwrap(),
        }
    }

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
            if let Some(job) = job_queue.pop().or_else(|| {
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
                job::execute(job, &shared_worker_data);
            } else {
                // Nothing :( so sleep until another job is here!
                let mut guard = shared_worker_data.sleep_mutex().lock();
                shared_worker_data.sleep_condvar().wait(&mut guard);
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.thread.is_finished()
    }
}
