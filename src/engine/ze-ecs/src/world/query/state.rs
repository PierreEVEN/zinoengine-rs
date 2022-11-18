use crate::access::Access;
use crate::archetype::{Archetype, ArchetypeId};
use crate::entity::Entity;
use crate::system::param::{SystemParamFetch, SystemParamState};
use crate::system::query::SystemQuery;
use crate::world::query::Query;
use crate::world::World;
use ze_core::sync::SyncUnsafeCell;
use ze_jobsystem::prelude::*;

pub struct QueryState<Q: Query> {
    state: Q::State,
    archetype_generation: u64,
    archetypes: Vec<ArchetypeId>,
    archetype_access: Access,
}

impl<'world, Q: Query> QueryState<Q> {
    pub fn new(world: &'world World) -> Self {
        let mut state = Self {
            state: Q::initialize_state(world),
            archetype_generation: 0,
            archetypes: vec![],
            archetype_access: Default::default(),
        };
        state.collect_archetypes(world);
        state
    }

    pub fn for_each<F: FnMut(Q::Item<'world>)>(&mut self, world: &'world World, mut f: F) {
        self.collect_archetypes(world);

        let mut fetch = Q::initialize_fetch(world, &self.state);
        for archetype_id in &self.archetypes {
            let archetype = world.archetype_registry.get(*archetype_id);
            Q::prepare_fetch(&mut fetch, &self.state, archetype);
            for entity in archetype.entities() {
                // SAFETY: set_archetype is called before fetch
                let item = unsafe { Q::fetch(&fetch, entity.id() as usize) };
                f(item);
            }
        }
    }

    pub fn par_for_each<F>(&mut self, world: &'world World, f: F)
    where
        F: FnMut(Q::Item<'world>) + Send + Sync,
        Q::Fetch<'world>: Send + Sync,
    {
        self.collect_archetypes(world);

        // We use UnsafeCells so we can mutate inside the parallel iterator
        let func = SyncUnsafeCell::new(f);
        let fetch = SyncUnsafeCell::new(Q::initialize_fetch(world, &self.state));
        for archetype_id in &self.archetypes {
            let fetch = unsafe { &mut *fetch.get() };
            let archetype = world.archetype_registry.get(*archetype_id);
            Q::prepare_fetch(fetch, &self.state, archetype);
            archetype.entities().par_iter().for_each(|entity| {
                // SAFETY: set_archetype is called before fetch
                let item = unsafe { Q::fetch(fetch, entity.id() as usize) };

                // SAFETY: Function is Send + Sync
                let func = unsafe { &mut *func.get() };
                func(item);
            });
        }
    }

    /// Get query result for a specific entity
    #[inline]
    pub fn get(&mut self, world: &'world World, entity: Entity) -> Option<Q::Item<'world>> {
        self.collect_archetypes(world);

        let archetype_id = world.entity_registry.archetype_id(entity).0;
        if !self.archetypes.contains(&archetype_id) {
            return None;
        }

        let archetype = world.archetype_registry.get(archetype_id);
        let mut fetch = Q::initialize_fetch(world, &self.state);
        Q::prepare_fetch(&mut fetch, &self.state, archetype);
        let item = unsafe { Q::fetch(&fetch, entity.id() as usize) };
        Some(item)
    }

    fn collect_archetypes(&mut self, world: &'world World) {
        if self.archetype_generation != world.archetype_registry.generation() {
            let old_generation = self.archetype_generation;
            self.archetype_generation = world.archetype_registry.generation();

            // Using the generation number we can compute what archetypes were added
            for i in old_generation..self.archetype_generation {
                let archetype_id = i as ArchetypeId;
                let archetype = world.archetype_registry.get(archetype_id);
                self.process_new_archetype(archetype);
            }
        }
    }

    /// Called whenever a new archetype is added to the world and needs to be processed
    pub(crate) fn process_new_archetype(&mut self, archetype: &Archetype) {
        if Q::archetype_contains_component(&self.state, |id| archetype.components().contains(id)) {
            Q::update_archetype_access(&self.state, archetype, &mut self.archetype_access);
            self.archetypes.push(archetype.id());
        }
    }
}

impl<Q: Query + 'static> SystemParamState for QueryState<Q> {
    fn initialize(world: &mut World, archetype_access: &mut Access) -> Self {
        let query = QueryState::new(world);
        archetype_access.union(&query.archetype_access);
        query
    }

    fn update_archetype_access(&mut self, archetype: &Archetype, archetype_access: &mut Access) {
        QueryState::process_new_archetype(self, archetype);
        archetype_access.union(&self.archetype_access);
    }
}

impl<'world, 'state, Q: Query + 'static> SystemParamFetch<'world, 'state> for QueryState<Q> {
    type Item = SystemQuery<'world, 'state, Q>;

    unsafe fn param(state: &'state mut Self, world: &'world SyncUnsafeCell<World>) -> Self::Item {
        let world = world.get().as_ref().unwrap_unchecked();
        SystemQuery::new(world, state)
    }
}
