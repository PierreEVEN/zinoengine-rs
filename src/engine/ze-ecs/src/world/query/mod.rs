use crate::access::Access;
use crate::archetype::{Archetype, Column};
use crate::component::{Component, ComponentId};
use crate::world::World;
use std::cell::UnsafeCell;
use ze_ecs_macros::repeat_tuples;

/// A object that fetch components from a world
pub trait Query {
    type Item<'world>;
    type Fetch<'world>: Send + Sync;

    /// State to construct `Self::Fetch`
    type State: Send + Sync + Sized;

    fn initialize_state(world: &World) -> Self::State;
    fn initialize_fetch<'world>(world: &'world World, state: &Self::State) -> Self::Fetch<'world>;

    /// Set the next archetype to fetch from
    fn set_archetype<'world>(
        fetch: &mut Self::Fetch<'world>,
        state: &Self::State,
        archetype: &'world Archetype,
    );

    /// Fetch from the currently set archetype the specified column
    /// # Safety
    ///
    /// - `prepare_fetch` must have been called
    /// - Caller must ensure that component accesses are respected
    unsafe fn fetch<'world>(
        fetch: &Self::Fetch<'world>,
        component_index: usize,
    ) -> Self::Item<'world>;

    fn archetype_contains_component<F: Fn(&ComponentId) -> bool>(state: &Self::State, f: F)
        -> bool;

    /// Update what archetype this query read/write
    fn update_archetype_access(state: &Self::State, archetype: &Archetype, access: &mut Access);
}

/// Fetch object meant to read a component
pub struct ReadFetch<'world> {
    column: Option<&'world UnsafeCell<Column>>,
}

// SAFETY: Executors guarantee we don't read columns while we are mutating them
unsafe impl<'world> Send for ReadFetch<'world> {}
unsafe impl<'world> Sync for ReadFetch<'world> {}

/// Fetch object meant to read and/or write a component
pub struct WriteFetch<'world> {
    column: Option<&'world UnsafeCell<Column>>,
}

// SAFETY: Executors guarantee we don't mutate columns while we are reading them
unsafe impl<'world> Send for WriteFetch<'world> {}
unsafe impl<'world> Sync for WriteFetch<'world> {}

impl<T: Component> Query for &T {
    type Item<'world> = &'world T;
    type Fetch<'world> = ReadFetch<'world>;
    type State = ComponentId;

    fn initialize_state(_: &World) -> Self::State {
        T::component_id()
    }

    fn initialize_fetch<'world>(_: &'world World, _: &Self::State) -> Self::Fetch<'world> {
        ReadFetch { column: None }
    }

    fn set_archetype<'world>(
        fetch: &mut Self::Fetch<'world>,
        state: &Self::State,
        archetype: &'world Archetype,
    ) {
        fetch.column = Some(&archetype.columns()[*state]);
    }

    unsafe fn fetch<'world>(fetch: &Self::Fetch<'world>, index: usize) -> Self::Item<'world> {
        // SAFETY: Caller guarantee that components accesses are respected and therefore
        // no other system is writing to this component
        let column = fetch
            .column
            .unwrap_unchecked()
            .get()
            .as_ref()
            .unwrap_unchecked();
        column.components().get(index).unwrap_unchecked()
    }

    fn archetype_contains_component<F: Fn(&ComponentId) -> bool>(
        state: &Self::State,
        f: F,
    ) -> bool {
        f(state)
    }

    fn update_archetype_access(state: &Self::State, archetype: &Archetype, access: &mut Access) {
        let position = archetype
            .components()
            .iter()
            .position(|id| id == state)
            .unwrap();

        access.add_read(archetype.components_archetype_ids()[position]);
    }
}

impl<T: Component> Query for &mut T {
    type Item<'world> = &'world mut T;
    type Fetch<'world> = WriteFetch<'world>;
    type State = ComponentId;

    fn initialize_state(_: &World) -> Self::State {
        T::component_id()
    }

    fn initialize_fetch<'world>(_: &'world World, _: &Self::State) -> Self::Fetch<'world> {
        WriteFetch { column: None }
    }

    fn set_archetype<'world>(
        fetch: &mut Self::Fetch<'world>,
        state: &Self::State,
        archetype: &'world Archetype,
    ) {
        fetch.column = Some(&archetype.columns()[*state]);
    }

    unsafe fn fetch<'world>(fetch: &Self::Fetch<'world>, index: usize) -> Self::Item<'world> {
        // SAFETY: Caller guarantee that components accesses are respected and therefore
        // no other system is writing to this component
        let column = fetch
            .column
            .unwrap_unchecked()
            .get()
            .as_mut()
            .unwrap_unchecked();
        column.components_mut().get_mut(index).unwrap_unchecked()
    }

    fn archetype_contains_component<F: Fn(&ComponentId) -> bool>(
        state: &Self::State,
        f: F,
    ) -> bool {
        f(state)
    }

    fn update_archetype_access(state: &Self::State, archetype: &Archetype, access: &mut Access) {
        let position = archetype
            .components()
            .iter()
            .position(|id| id == state)
            .unwrap();

        access.add_write(archetype.components_archetype_ids()[position]);
    }
}

macro_rules! impl_tuples {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(non_snake_case)]
        impl<$($name: Query),*> Query for ($($name),*) {
            type Item<'world> = ($($name::Item<'world>,)*);
            type Fetch<'world> = ($($name::Fetch<'world>,)*);
            type State = ($($name::State,)*);

            fn initialize_state<'world>(world: &'world World) -> Self::State {
                ($($name::initialize_state(world)),*)
            }

            fn initialize_fetch<'world>(world: &'world World, state: &Self::State) -> Self::Fetch<'world> {
                let ($($name),*) = state;
                ($($name::initialize_fetch(world, $name)),*)
            }

            fn set_archetype<'world>(fetch: &mut Self::Fetch<'world>, state: &Self::State, archetype: &'world Archetype) {
                let ($($name,)*) = fetch;
                let ($($state,)*) = state;
                $($name::set_archetype($name, $state, archetype);)*
            }

            unsafe fn fetch<'world>(fetch: &Self::Fetch<'world>, index: usize) -> Self::Item<'world> {
                let ($($name),*) = fetch;
                ($($name::fetch($name, index)),*)
            }

            fn archetype_contains_component<'world, F: Fn(&ComponentId) -> bool>(state: &Self::State, f: F) -> bool {
                let ($($name),*) = state;
                true $(&& $name::archetype_contains_component($name, &f))*
            }

            fn update_archetype_access(state: &Self::State, archetype: &Archetype, access: &mut Access) {
                let ($($name,)*) = state;
                $($name::update_archetype_access($name, archetype, access);)*
            }
        }
    }
}

repeat_tuples!(impl_tuples, 8, F, S);

pub mod state;
