use crate::access::Access;
use crate::archetype::Archetype;
use crate::component::{Component, ComponentId};
use crate::world::query::Query;
use crate::world::World;
use std::marker::PhantomData;

pub struct With<T: Component>(PhantomData<T>);

impl<T: Component> Query for With<T> {
    type Item<'world> = ();
    type Fetch<'world> = ();
    type State = ComponentId;

    fn initialize_state(_: &World) -> Self::State {
        T::component_id()
    }

    fn initialize_fetch<'world>(_: &'world World, _: &Self::State) -> Self::Fetch<'world> {}

    fn prepare_fetch<'world>(_: &mut Self::Fetch<'world>, _: &Self::State, _: &'world Archetype) {}

    unsafe fn fetch<'world>(_: &Self::Fetch<'world>, _: usize) -> Self::Item<'world> {}

    fn archetype_contains_component<F: Fn(&ComponentId) -> bool>(
        state: &Self::State,
        f: F,
    ) -> bool {
        f(state)
    }

    fn update_archetype_access(_: &Self::State, _: &Archetype, _: &mut Access) {}
}

pub struct Without<T: Component>(PhantomData<T>);

impl<T: Component> Query for Without<T> {
    type Item<'world> = ();
    type Fetch<'world> = ();
    type State = ComponentId;

    fn initialize_state(_: &World) -> Self::State {
        T::component_id()
    }

    fn initialize_fetch<'world>(_: &'world World, _: &Self::State) -> Self::Fetch<'world> {}

    fn prepare_fetch<'world>(_: &mut Self::Fetch<'world>, _: &Self::State, _: &'world Archetype) {}

    unsafe fn fetch<'world>(_: &Self::Fetch<'world>, _: usize) -> Self::Item<'world> {}

    fn archetype_contains_component<F: Fn(&ComponentId) -> bool>(
        state: &Self::State,
        f: F,
    ) -> bool {
        !f(state)
    }

    fn update_archetype_access(_: &Self::State, _: &Archetype, _: &mut Access) {}
}
