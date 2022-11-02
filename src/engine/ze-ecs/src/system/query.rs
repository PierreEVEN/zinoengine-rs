use crate::system::param::SystemParam;
use crate::world::query::state::QueryState;
use crate::world::query::Query;
use crate::world::World;

/// A system parameter that query the world
pub struct SystemQuery<'world, 'state, Q: Query> {
    world: &'world World,
    state: &'state mut QueryState<Q>,
}

impl<'world, 'state, Q: Query> SystemQuery<'world, 'state, Q> {
    pub fn new(world: &'world World, state: &'state mut QueryState<Q>) -> Self {
        Self { world, state }
    }

    pub fn for_each<F: FnMut(Q::Item<'world>)>(&mut self, f: F) {
        self.state.for_each(self.world, f);
    }

    pub fn par_for_each<F: FnMut(Q::Item<'world>) + Send + Sync>(&mut self, f: F)
    where
        Q::Fetch<'world>: Send + Sync,
    {
        self.state.par_for_each(self.world, f);
    }
}

impl<'world, 'state, Q: Query + 'static> SystemParam for SystemQuery<'world, 'state, Q> {
    type Fetch = QueryState<Q>;
}
