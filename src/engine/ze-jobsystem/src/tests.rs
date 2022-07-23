use crate::JobSystem;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn spawn_one_job() {
    let jobsystem = JobSystem::new(JobSystem::get_cpu_thread_count() - 1);
    let simple_bool = Arc::new(AtomicBool::new(false));
    {
        let simple_bool = simple_bool.clone();
        let mut job = jobsystem.spawn(move |_, _| {
            simple_bool.store(true, Ordering::SeqCst);
        });
        job.schedule();
        job.wait();
    }
    assert!(simple_bool.load(Ordering::SeqCst));
}

#[test]
fn spawn_one_job_five_childs() {
    let jobsystem = JobSystem::new(JobSystem::get_cpu_thread_count() - 1);
    let counter = Arc::new(AtomicU32::new(0));
    {
        let counter = counter.clone();
        let mut parent = jobsystem.spawn(move |jobsystem, job| {
            counter.fetch_add(1, Ordering::SeqCst);

            for _ in 0..5 {
                let counter = counter.clone();
                let mut child = jobsystem.spawn_child(job, move |_, _| {
                    counter.fetch_add(1, Ordering::SeqCst);
                });
                child.schedule();
            }
        });
        parent.schedule();
        parent.wait();
    }
    assert_eq!(counter.load(Ordering::SeqCst), 6);
}

#[test]
fn spawn_three_jobs_one_continuation_per_job() {
    let jobsystem = JobSystem::new(JobSystem::get_cpu_thread_count() - 1);
    let counter = Arc::new(AtomicU32::new(0));

    for _ in 0..3 {
        let counter = counter.clone();
        let counter2 = counter.clone();
        let mut ancestor = jobsystem.spawn(move |_, _| {
            counter.fetch_add(1, Ordering::SeqCst);
        });

        ancestor.add_continuation(&jobsystem.spawn(move |_, _| {
            counter2.fetch_add(1, Ordering::SeqCst);
        }));

        ancestor.schedule();
    }

    sleep(Duration::from_millis(250));
    assert_eq!(counter.load(Ordering::SeqCst), 6);
}
