use crate::access::Access;
use crate::archetype::Archetype;
use crate::world::World;
use ze_core::sync::SyncUnsafeCell;
use ze_ecs_macros::repeat_tuples_no_skip;

/// Parameter of a [`System`], that can be fetched later on
pub trait SystemParam: Sized {
    type Fetch: for<'world, 'state> SystemParamFetch<'world, 'state>;
}

/// Store the state of a [`SystemParam`]
pub trait SystemParamState: Send + Sync + 'static {
    /// Initialize the state of the system parameter, optionally filling archetype access
    fn initialize(world: &mut World, archetype_access: &mut Access) -> Self;

    /// Called when a new archetype is added by the system so it can update its archetype access
    /// depending on its parameters
    fn update_archetype_access(&mut self, archetype: &Archetype, archetype_access: &mut Access);
}

/// Fetch a [`SystemParam`] for a given [`System`] from a [`SystemParamState`]
pub trait SystemParamFetch<'world, 'state>: SystemParamState {
    type Item: SystemParam<Fetch = Self>;

    /// # Safety
    ///
    /// `world` must be correctly used
    unsafe fn param(state: &'state mut Self, world: &'world SyncUnsafeCell<World>) -> Self::Item;
}

macro_rules! impl_system_param_tuple {
    ($($param:ident),*) => {
        impl<$($param: SystemParam),*> SystemParam for ($($param,)*) {
            type Fetch = ($($param::Fetch,)*);
        }

        #[allow(non_snake_case, clippy::unused_unit)]
        impl<$($param: SystemParamState),*> SystemParamState for ($($param,)*) {
            fn initialize(_world: &mut World, _archetype_access: &mut Access) -> Self {
                ($($param::initialize(_world, _archetype_access),)*)
            }

            fn update_archetype_access(&mut self, _archetype: &Archetype, _archetype_access: &mut Access) {
                let ($($param,)*) = self;
                $($param.update_archetype_access(_archetype, _archetype_access);)*
            }
        }

        #[allow(non_snake_case, clippy::unused_unit)]
        impl<'world, 'state, $($param: SystemParamFetch<'world, 'state>),*> SystemParamFetch<'world, 'state> for ($($param,)*) {
            type Item = ($($param::Item,)*);

            unsafe fn param(state: &'state mut Self, _world: &'world SyncUnsafeCell<World>) -> Self::Item {
                let ($($param,)*) = state;
                ($($param::param($param, _world),)*)
            }
        }
    };
}

repeat_tuples_no_skip!(impl_system_param_tuple, 8, A);
