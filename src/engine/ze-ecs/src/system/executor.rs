use crate::access::Access;
use crate::system::schedule::Schedule;
use crate::world::World;
use bitvec::prelude::BitVec;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::cell::RefCell;
use ze_core::sync::SyncUnsafeCell;
use ze_jobsystem::global;

pub trait Executor {
    fn initialize(&mut self, world: &World, schedule: &Schedule);
    fn run(&mut self, world: &mut World, schedule: &mut Schedule);
}

#[derive(Default)]
pub struct SequentialExecutor {
    /// Systems that were evaluated and/or run
    processed_systems: BitVec,

    /// System sets that were evaluated for execution
    evaluated_sets: BitVec,
}

impl Executor for SequentialExecutor {
    fn initialize(&mut self, _: &World, schedule: &Schedule) {
        self.processed_systems.resize(schedule.systems.len(), false);
        self.evaluated_sets
            .resize(schedule.set_conditions.len(), false);
        self.processed_systems.fill(false);
        self.evaluated_sets.fill(false);
    }

    fn run(&mut self, world: &mut World, schedule: &mut Schedule) {
        for (system_idx, system) in schedule.systems.iter().enumerate() {
            if self.processed_systems[system_idx] {
                continue;
            } else {
                self.processed_systems.set(system_idx, true);
            }

            let world = unsafe {
                (world as *const _ as *const SyncUnsafeCell<World>)
                    .as_ref()
                    .unwrap_unchecked()
            };

            if should_system_run(
                world,
                &mut self.evaluated_sets,
                &mut self.processed_systems,
                schedule,
                system_idx,
            ) {
                let mut system = RefCell::borrow_mut(system);
                // SAFETY: We run one system at any given time, there is no collisions between systems
                // possible
                unsafe {
                    system.run((), world);
                }
            }
        }
    }
}

struct SystemData {
    dependencies_count: usize,
    dependencies_remaining: usize,
    dependents: Vec<usize>,
    archetype_access: Access,
}

pub struct ParallelExecutor {
    system_datas: Vec<SystemData>,
    completed_sender: Sender<usize>,
    completed_receiver: Receiver<usize>,

    /// Systems that are ready to be processed (not waiting for any dependencies)
    ready_systems: BitVec,
    processed_systems: BitVec,
    evaluated_sets: BitVec,
    running_systems: BitVec,

    current_archetype_access: Access,
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        let (completed_sender, completed_receiver) = unbounded();
        Self {
            system_datas: vec![],
            completed_sender,
            completed_receiver,
            ready_systems: Default::default(),
            processed_systems: Default::default(),
            evaluated_sets: Default::default(),
            running_systems: Default::default(),
            current_archetype_access: Default::default(),
        }
    }
}

impl Executor for ParallelExecutor {
    fn initialize(&mut self, world: &World, schedule: &Schedule) {
        self.system_datas.clear();
        self.processed_systems.fill(false);
        self.ready_systems.fill(false);
        self.evaluated_sets.fill(false);
        self.running_systems.fill(false);

        self.processed_systems.resize(schedule.systems.len(), false);
        self.ready_systems.resize(schedule.systems.len(), false);
        self.running_systems.resize(schedule.systems.len(), false);
        self.evaluated_sets
            .resize(schedule.set_conditions.len(), false);

        for i in 0..self.processed_systems.len() {
            let (dependencies_count, dependents) = &schedule.systems_dependencies[i];

            let system = unsafe { schedule.systems[i].as_ptr().as_mut().unwrap() };
            system.update_archetype_access(world);

            let mut archetype_access = system.archetype_access().clone();

            for set_idx in schedule.system_is_in_set_bitset[i]
                .iter_ones()
                .filter(|i| !self.evaluated_sets[*i])
            {
                let mut set_conditions = RefCell::borrow_mut(&schedule.set_conditions[set_idx]);
                for condition in set_conditions.iter_mut() {
                    condition.update_archetype_access(world);
                    archetype_access.union(condition.archetype_access());
                }
            }

            let mut system_conditions = RefCell::borrow_mut(&schedule.systems_conditions[i]);
            for condition in system_conditions.iter_mut() {
                condition.update_archetype_access(world);
                archetype_access.union(condition.archetype_access());
            }

            self.system_datas.push(SystemData {
                dependencies_count: *dependencies_count,
                dependencies_remaining: *dependencies_count,
                dependents: dependents.clone(),
                archetype_access,
            });
        }
    }

    fn run(&mut self, world: &mut World, schedule: &mut Schedule) {
        let world = unsafe {
            (world as *const _ as *const SyncUnsafeCell<World>)
                .as_ref()
                .unwrap_unchecked()
        };

        // Systems without any dependencies are ready to run
        for (i, (dependencies_count, _)) in schedule.systems_dependencies.iter().enumerate() {
            if *dependencies_count == 0 {
                self.ready_systems.set(i, true);
            }
        }

        while !self.processed_systems.all() {
            // Spawn some tasks to execute systems
            let ready_sets = self
                .ready_systems
                .iter()
                .by_vals()
                .enumerate()
                .filter_map(|(idx, bit)| if bit { Some(idx) } else { None })
                .collect::<Vec<_>>();

            for system_idx in ready_sets {
                debug_assert!(!self.running_systems[system_idx]);

                // Can't determine execution yet
                let system_archetype_access = &self.system_datas[system_idx].archetype_access;
                if !system_archetype_access.is_disjoint(&self.current_archetype_access) {
                    continue;
                }

                if !should_system_run(
                    world,
                    &mut self.evaluated_sets,
                    &mut self.processed_systems,
                    schedule,
                    system_idx,
                ) {
                    self.ready_systems.set(system_idx, false);
                    self.mark_system_as_processed(system_idx);
                    continue;
                }

                // SAFETY: There is no other mutable references to the system while running a schedule
                let system = unsafe { schedule.systems[system_idx].as_ptr().as_mut().unwrap() };
                let completed_sender = self.completed_sender.clone();

                // SAFETY: Jobs are guaranteed to complete before leaving the executor
                unsafe {
                    global()
                        .spawn_unchecked(move |_, _| {
                            system.run((), world);
                            completed_sender.send(system_idx).unwrap();
                        })
                        .schedule();
                }

                self.current_archetype_access.union(system_archetype_access);
                self.ready_systems.set(system_idx, false);
                self.running_systems.set(system_idx, true);
            }

            // Check if some scheduled systems have completed
            let mut has_systems_completed = false;
            while let Ok(system_idx) = self.completed_receiver.try_recv() {
                has_systems_completed = true;
                self.running_systems.set(system_idx, false);
                self.mark_system_as_processed(system_idx);
            }

            // Remove completed systems from the current archetype access
            if has_systems_completed {
                self.current_archetype_access.clear();
                for system_idx in self.running_systems.iter_ones() {
                    let system_archetype_access = &self.system_datas[system_idx].archetype_access;
                    self.current_archetype_access.union(system_archetype_access);
                }
            }
        }
    }
}

impl ParallelExecutor {
    /// Mark system as processed, and update its dependents
    fn mark_system_as_processed(&mut self, system_idx: usize) {
        self.running_systems.set(system_idx, false);
        self.processed_systems.set(system_idx, true);

        let dependents = self.system_datas[system_idx].dependents.clone();
        for dependent in dependents {
            self.system_datas[dependent].dependencies_remaining -= 1;
            if self.system_datas[dependent].dependencies_count == 0
                && !self.processed_systems[dependent]
            {
                self.ready_systems.set(dependent, true);
            }
        }
    }
}

fn should_system_run(
    world: &SyncUnsafeCell<World>,
    evaluated_sets: &mut BitVec,
    processed_systems: &mut BitVec,
    schedule: &Schedule,
    system_idx: usize,
) -> bool {
    let mut should_run = true;

    // Evaluate the conditions of the system's set if needed
    for set_idx in schedule.system_is_in_set_bitset[system_idx].iter_ones() {
        if evaluated_sets[set_idx] {
            continue;
        } else {
            evaluated_sets.set(set_idx, true);
        }

        let mut set_conditions = RefCell::borrow_mut(&schedule.set_conditions[set_idx]);
        if !set_conditions.iter_mut().all(|condition| {
            // SAFETY: Conditions are run one at a time, there is no collisions between conditions
            unsafe { condition.run((), world) }
        }) {
            should_run = false;
            *processed_systems |= &schedule.set_has_system_bitset[set_idx];
        }
    }

    if !should_run {
        return false;
    }

    // Check systems conditions
    let mut system_conditions = RefCell::borrow_mut(&schedule.systems_conditions[system_idx]);
    if !system_conditions.iter_mut().all(|condition| {
        // SAFETY: Conditions are run one at a time, there is no collisions between conditions
        unsafe { condition.run((), world) }
    }) {
        should_run = false;
        processed_systems.set(system_idx, true);
    }

    if !should_run {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::component::{Component, ComponentId};
    use crate::system::executor::{Executor, ParallelExecutor, SequentialExecutor};
    use crate::system::query::*;
    use crate::system::registry::SystemRegistry;
    use crate::system::set::IntoSystemSetDesc;
    use crate::system::IntoSystemDesc;
    use crate::world::World;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use ze_jobsystem::{try_initialize_global, JobSystem};

    #[test]
    fn sequential_test_conditions() {
        let mut registry = SystemRegistry::default();
        let mut world = World::default();
        registry.add_system_set("X");
        registry.add_system_set("Y".in_set("X").condition(|| false));
        registry.add_system((|| {}).id("A").in_set("X"));
        registry.add_system((|| unreachable!()).id("B").in_set("Y"));
        registry.update(&mut world);

        let mut executor = SequentialExecutor::default();
        let schedule = registry.schedule_mut("X");
        executor.initialize(&world, schedule);
        executor.run(&mut world, schedule);
    }

    #[test]
    fn parallel_test_conditions() {
        let _ = try_initialize_global(JobSystem::new(JobSystem::cpu_thread_count() - 1));

        let mut registry = SystemRegistry::default();
        let mut world = World::default();
        registry.add_system_set("X");
        registry.add_system_set("Y".in_set("X").condition(|| false));
        registry.add_system((|| {}).id("A").in_set("X"));
        registry.add_system((|| unreachable!()).id("B").in_set("Y"));
        registry.update(&mut world);

        let mut executor = ParallelExecutor::default();
        let schedule = registry.schedule_mut("X");
        executor.initialize(&world, schedule);
        executor.run(&mut world, schedule);
    }

    #[test]
    fn parallel_test_exclusive_archetype_access() {
        let _ = try_initialize_global(JobSystem::new(JobSystem::cpu_thread_count() - 1));

        let mut registry = SystemRegistry::default();
        let mut world = World::default();

        struct MyComponent {}
        impl Component for MyComponent {
            fn component_id() -> ComponentId {
                0
            }
        }

        let entity = world.spawn();
        world.add(entity, MyComponent {});

        let accesses = Arc::new(AtomicU8::new(0));

        registry.add_system_set("X");

        {
            let accesses = accesses.clone();
            registry.add_system(
                (move |_: SystemQuery<&mut MyComponent>| {
                    let old = accesses.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(old, 0);
                    thread::sleep(Duration::from_millis(100));
                    accesses.fetch_sub(1, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(100));
                })
                .id("A")
                .in_set("X"),
            );
        }
        {
            registry.add_system(
                (move |_: SystemQuery<&mut MyComponent>| {
                    let old = accesses.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(old, 0);
                    thread::sleep(Duration::from_millis(100));
                    accesses.fetch_sub(1, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(100));
                })
                .id("B")
                .in_set("X"),
            );
        }

        registry.update(&mut world);

        let mut executor = ParallelExecutor::default();
        let schedule = registry.schedule_mut("X");
        executor.initialize(&world, schedule);
        executor.run(&mut world, schedule);
    }
}
