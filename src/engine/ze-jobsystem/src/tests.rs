use crate::JobSystem;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

#[test]
fn spawn_one_job_and_wait() {
    let jobsystem = JobSystem::new(JobSystem::cpu_thread_count() - 1);
    let simple_bool = Arc::new(AtomicBool::new(false));
    {
        let simple_bool = simple_bool.clone();
        let job = jobsystem
            .spawn(move |_, _| {
                simple_bool.store(true, Ordering::SeqCst);
            })
            .schedule();
        jobsystem.wait_for(&[job]);
    }
    assert!(simple_bool.load(Ordering::SeqCst));
}

#[test]
fn drop() {
    let jobsystem = JobSystem::new(JobSystem::cpu_thread_count() - 1);

    struct TestDrop {
        dropped: AtomicBool,
    }

    let drop_test = Arc::new(TestDrop {
        dropped: Default::default(),
    });

    {
        let drop_test = drop_test.clone();
        let job = jobsystem
            .spawn(move |_, _| {
                assert_eq!(Arc::strong_count(&drop_test), 2);
            })
            .schedule();
        jobsystem.wait_for(&[job]);
    }

    assert_eq!(Arc::strong_count(&drop_test), 1);
}

#[test]
fn spawn_one_job_five_childs() {
    let jobsystem = JobSystem::new(JobSystem::cpu_thread_count() - 1);
    let counter = Arc::new(AtomicU32::new(0));
    {
        let counter = counter.clone();
        let parent = jobsystem
            .spawn(move |jobsystem, job| {
                counter.fetch_add(1, Ordering::SeqCst);

                for _ in 0..5 {
                    let counter = counter.clone();
                    jobsystem
                        .spawn(move |_, _| {
                            counter.fetch_add(1, Ordering::SeqCst);
                        })
                        .with_parent(&job)
                        .schedule();
                }
            })
            .schedule();

        jobsystem.wait_for(&[parent]);
    }
    assert_eq!(counter.load(Ordering::SeqCst), 6);
}

#[test]
fn spawn_three_jobs_one_continuation_per_job() {
    let jobsystem = JobSystem::new(JobSystem::cpu_thread_count() - 1);
    let counter = Arc::new(AtomicU32::new(0));

    for _ in 0..3 {
        let counter = counter.clone();
        let counter2 = counter.clone();
        jobsystem
            .spawn(move |_, _| {
                counter.fetch_add(1, Ordering::SeqCst);
            })
            .with_continuation(jobsystem.spawn(move |_, _| {
                counter2.fetch_add(1, Ordering::SeqCst);
            }))
            .schedule();
    }

    jobsystem.wait_until_idle();
    assert_eq!(counter.load(Ordering::SeqCst), 6);
}
