use crate::archetype::{ArchetypeId, ArchetypeRegistry};
use crate::component::{Component, ComponentId, ComponentInfo, ComponentRegistry};
use crate::entity::{Entity, EntityRegistry};
use crate::erased_vec::TypeInfo;
use crate::system::executor::{Executor, ParallelExecutor};
use crate::system::registry::SystemRegistry;
use crate::system::set::IntoSystemSetDesc;
use crate::system::{IntoSystemDesc, IntoSystemId};
use crate::world::query::state::QueryState;
use crate::world::query::Query;
use std::cell::Cell;
use std::mem::forget;
use std::ptr::Unique;

/// Id of the empty archetype
pub const EMPTY_ARCHETYPE_ID: ArchetypeId = 0;

pub struct World {
    entity_registry: EntityRegistry,
    pub(crate) archetype_registry: ArchetypeRegistry,
    component_registry: ComponentRegistry,
    system_registry: Cell<SystemRegistry>,
}

impl Default for World {
    fn default() -> Self {
        let mut world = Self {
            entity_registry: Default::default(),
            archetype_registry: Default::default(),
            component_registry: Default::default(),
            system_registry: Default::default(),
        };

        // Register default empty archetype
        // SAFETY: Empty archetype doesn't have any components
        unsafe {
            world
                .archetype_registry
                .register(&world.component_registry, &[]);
        }

        world
    }
}

impl World {
    pub fn spawn(&mut self) -> Entity {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        let entity = self.entity_registry.alloc();
        let archetype = self.archetype_registry.get_mut(EMPTY_ARCHETYPE_ID);
        // SAFETY: Empty archetype doesn't have any components
        let idx = unsafe { archetype.insert_row(entity, vec![]) };
        self.entity_registry
            .set_archetype_id(entity, EMPTY_ARCHETYPE_ID, idx);
        entity
    }

    pub fn destroy(&mut self, entity: Entity) {
        let (archetype_id, archetype_idx) = self.entity_registry.archetype_id(entity);
        self.archetype_registry
            .get_mut(archetype_id)
            .remove_row(archetype_idx, entity, |_| true);
        self.entity_registry.free(entity);
    }

    pub fn add<T: Component>(&mut self, entity: Entity, mut component: T) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
        if self.component_registry.get(&T::component_id()).is_none() {
            self.component_registry.register(
                T::component_id(),
                ComponentInfo {
                    type_info: TypeInfo::new::<T>(),
                },
            );
        }

        unsafe {
            self.add_component(
                entity,
                T::component_id(),
                Unique::new_unchecked(&mut component as *mut T as *mut u8),
            );
        }

        forget(component)
    }

    /// Remove component `T` from the entity
    pub fn remove<T: Component>(&mut self, entity: Entity) {
        self.remove_component(entity, T::component_id());
    }

    /// # Safety
    ///
    /// `value` must be a valid instance of `component`
    pub unsafe fn add_component(
        &mut self,
        entity: Entity,
        component: ComponentId,
        value: Unique<u8>,
    ) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
        assert!(self.component_registry.get(&component).is_some());

        let (archetype_id, _) = self.entity_registry.archetype_id(entity);
        let mut new_archetype_id = None;
        {
            let archetype = self.archetype_registry.get_mut(archetype_id);
            // Search for the new archetype using the cached edges
            if let Some(archetype_id) = archetype.edge(&component).add {
                new_archetype_id = Some(archetype_id);
            }
        }

        if new_archetype_id.is_none() {
            // No edge found, add one
            let mut components = {
                let archetype = self.archetype_registry.get(archetype_id);
                archetype.components().to_vec()
            };

            components.push(component);
            new_archetype_id = Some(
                self.archetype_registry
                    .get_or_register_mut(&self.component_registry, &components)
                    .id(),
            );

            let archetype = self.archetype_registry.get_mut(archetype_id);
            let mut edge = archetype.edge(&component);
            edge.add = Some(new_archetype_id.unwrap());
        }

        self.move_entity_to_archetype(
            entity,
            &archetype_id,
            &new_archetype_id.unwrap(),
            &[],
            vec![(component, value)],
        );
    }

    pub fn remove_component(&mut self, entity: Entity, component: ComponentId) {
        assert!(self.component_registry.get(&component).is_some());
        let archetype_id = self.entity_registry.archetype_id(entity).0;
        assert!(self
            .archetype_registry
            .get(archetype_id)
            .components()
            .contains(&component));

        // Find new archetype to target
        let mut new_archetype_id = None;
        {
            let archetype = self.archetype_registry.get_mut(archetype_id);
            // Search for the new archetype using the cached edges
            if let Some(archetype_id) = archetype.edge(&component).remove {
                new_archetype_id = Some(archetype_id);
            }
        }

        if new_archetype_id.is_none() {
            // No edge found, add one
            let mut components = {
                let archetype = self.archetype_registry.get(archetype_id);
                archetype.components().to_vec()
            };

            components.remove(component);

            // SAFETY: Components stored inside an archetype are always valid
            new_archetype_id = unsafe {
                Some(
                    self.archetype_registry
                        .get_or_register_mut(&self.component_registry, &components)
                        .id(),
                )
            };

            let archetype = self.archetype_registry.get_mut(archetype_id);
            let mut edge = archetype.edge(&component);
            edge.remove = Some(new_archetype_id.unwrap());
        }

        self.move_entity_to_archetype(
            entity,
            &archetype_id,
            &new_archetype_id.unwrap(),
            &[component],
            vec![],
        );
    }

    pub fn update(&mut self) {
        let mut system_registry = self.system_registry.take();
        system_registry.update(self);
        self.system_registry.set(system_registry);
    }

    pub fn run(&mut self, root_system_id: impl IntoSystemId) {
        let mut system_registry = self.system_registry.take();
        let schedule = system_registry.schedule_mut(root_system_id);
        let mut executor = ParallelExecutor::default();
        executor.initialize(self, schedule);
        executor.run(self, schedule);
        self.system_registry.set(system_registry);
    }

    pub fn add_system_set(&mut self, system: impl IntoSystemSetDesc) {
        self.system_registry.get_mut().add_system_set(system);
    }

    pub fn add_system<Params>(&mut self, system: impl IntoSystemDesc<Params>) {
        self.system_registry.get_mut().add_system(system);
    }

    pub fn query<Q: Query>(&self) -> QueryState<Q> {
        QueryState::new(self)
    }

    pub fn is_valid(&self, entity: Entity) -> bool {
        self.entity_registry.is_valid(entity)
    }

    fn move_entity_to_archetype(
        &mut self,
        entity: Entity,
        src: &ArchetypeId,
        dst: &ArchetypeId,
        components_to_drop: &[ComponentId],
        mut new_values: Vec<(ComponentId, Unique<u8>)>,
    ) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
        let src_archetype = self.archetype_registry.get_mut(*src);

        // Collect src archetype components pointers to move them
        let src_archetype_component_ids_to_move = src_archetype
            .components()
            .iter()
            .filter(|component_id| !components_to_drop.contains(component_id))
            .cloned()
            .collect::<Vec<_>>();

        let mut components = src_archetype_component_ids_to_move
            .iter()
            .map(|component_id| unsafe {
                (
                    *component_id,
                    Unique::new_unchecked(
                        src_archetype
                            .columns_mut()
                            .get_mut(*component_id)
                            .unwrap_unchecked()
                            .get_mut()
                            .components_mut()
                            .get_unchecked_mut::<u8>(entity.id() as usize)
                            .unwrap_unchecked() as *mut u8,
                    ),
                )
            })
            .collect::<Vec<_>>();

        components.append(&mut new_values);

        let dst_archetype = self.archetype_registry.get_mut(*dst);

        // SAFETY: We are moving the entity to a new archetype, so we can safely assume that the
        // previous components are still valid
        let new_archetype_index = unsafe { dst_archetype.insert_row(entity, components) };

        // We can remove the row from the src archetype, forgetting every value instead of dropping them if needed
        let src_archetype_index = self.entity_registry.archetype_id(entity).1;
        let src_archetype = self.archetype_registry.get_mut(*src);
        src_archetype.remove_row(src_archetype_index, entity, |id| {
            components_to_drop.contains(id)
        });
        self.entity_registry
            .set_archetype_id(entity, *dst, new_archetype_index);
    }
}

unsafe impl Send for World {}
unsafe impl Sync for World {}

pub mod query;

#[cfg(test)]
mod tests {
    use crate::component::ComponentId;
    use crate::world::Component;
    use crate::world::World;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    #[test]
    fn add_remove_query() {
        struct Counter {
            x: i32,
        }

        impl Default for Counter {
            fn default() -> Self {
                Self { x: 2201 }
            }
        }

        impl Component for Counter {
            fn component_id() -> ComponentId {
                0
            }
        }

        struct F32Counter {
            x: f32,
        }

        impl Default for F32Counter {
            fn default() -> Self {
                Self { x: 22.01 }
            }
        }

        impl Component for F32Counter {
            fn component_id() -> ComponentId {
                1
            }
        }

        let mut world = World::default();
        let entity = world.spawn();
        world.add(entity, Counter::default());
        world.add(entity, F32Counter::default());

        let entity2 = world.spawn();
        world.add(entity2, Counter::default());

        let mut query = world.query::<&Counter>();
        query.for_each(&world, |counter| {
            assert_eq!(counter.x, 2201);
        });

        let mut query = world.query::<&mut F32Counter>();
        query.for_each(&world, |mut counter| {
            assert_eq!(counter.x, 22.01);
            counter.x = 69.01;
        });

        let mut query = world.query::<(&Counter, &F32Counter)>();
        query.for_each(&world, |(counter, fcounter)| {
            assert_eq!(counter.x, 2201);
            assert_eq!(fcounter.x, 69.01);
        });
    }

    #[test]
    fn destroy() {
        struct Counter {
            c: Arc<AtomicI32>,
        }

        impl Component for Counter {
            fn component_id() -> ComponentId {
                0
            }
        }

        impl Drop for Counter {
            fn drop(&mut self) {
                self.c.fetch_sub(1, Ordering::SeqCst);
            }
        }

        struct Empty {}

        impl Component for Empty {
            fn component_id() -> ComponentId {
                1
            }
        }

        let c = Arc::new(AtomicI32::new(1));

        let mut world = World::default();
        let entity = world.spawn();
        world.add(entity, Counter { c: c.clone() });
        assert_eq!(c.load(Ordering::SeqCst), 1);
        world.add(entity, Empty {});
        assert_eq!(c.load(Ordering::SeqCst), 1);
        world.remove::<Empty>(entity);
        assert_eq!(c.load(Ordering::SeqCst), 1);
        world.destroy(entity);
        assert_eq!(c.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn recycling() {
        let mut world = World::default();
        let entity = world.spawn();
        let id = entity.id();
        world.destroy(entity);
        assert!(!world.is_valid(entity));

        let entity2 = world.spawn();
        assert_eq!(entity2.id(), id);
        assert_eq!(entity2.generation(), 1);
        assert!(!world.is_valid(entity));
        assert!(world.is_valid(entity2));
    }
}

#[cfg(test)]
mod benches {
    use crate::component::{Component, ComponentId};
    use crate::system::query::SystemQuery;
    use crate::system::IntoSystemDesc;
    use crate::world::World;
    use std::hint::black_box;
    use test::Bencher;
    use ze_core::maths::Matrix4x4;
    use ze_jobsystem::{try_initialize_global, JobSystem};

    #[bench]
    fn add_entity_with_component(b: &mut Bencher) {
        let _ = try_initialize_global(JobSystem::new(JobSystem::cpu_thread_count() - 1));

        struct Counter {
            _c: Matrix4x4<f64>,
        }

        impl Component for Counter {
            fn component_id() -> ComponentId {
                0
            }
        }

        let mut world = World::default();
        let mut c = 0u64;
        b.iter(|| {
            c += 1;
            let entity = world.spawn();
            world.add(
                entity,
                Counter {
                    _c: Matrix4x4::default(),
                },
            );
        });
        black_box(world);
    }

    #[bench]
    fn query_1_000_000_entities_mutate(b: &mut Bencher) {
        let _ = try_initialize_global(JobSystem::new(JobSystem::cpu_thread_count() - 1));

        struct Counter {
            m: Matrix4x4<f64>,
        }

        impl Component for Counter {
            fn component_id() -> ComponentId {
                0
            }
        }

        let mut world = World::default();
        for _ in 0..1_000_000 {
            let entity = world.spawn();
            world.add(
                entity,
                Counter {
                    m: Default::default(),
                },
            );
        }

        world.add_system_set("main_set");
        world.add_system(
            (|mut query: SystemQuery<&mut Counter>| {
                query.for_each(|mut counter| {
                    counter.m = counter.m * Matrix4x4::default();
                });
            })
            .in_set("main_set")
            .id("x"),
        );
        world.update();

        b.iter(|| {
            world.run("main_set");
        });
    }

    #[bench]
    fn query_1_000_000_entities_mutate_par_iter(b: &mut Bencher) {
        let _ = try_initialize_global(JobSystem::new(JobSystem::cpu_thread_count() - 1));

        struct Counter {
            m: Matrix4x4<f64>,
        }

        impl Component for Counter {
            fn component_id() -> ComponentId {
                0
            }
        }

        let mut world = World::default();
        for _ in 0..1_000_000 {
            let entity = world.spawn();
            world.add(
                entity,
                Counter {
                    m: Default::default(),
                },
            );
        }

        world.add_system_set("main_set");
        world.add_system(
            (|mut query: SystemQuery<&mut Counter>| {
                query.par_for_each(|mut counter| {
                    counter.m = counter.m * Matrix4x4::default();
                });
            })
            .in_set("main_set")
            .id("x"),
        );
        world.update();

        b.iter(|| {
            world.run("main_set");
        });
    }
}
