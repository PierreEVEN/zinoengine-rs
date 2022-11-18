use crate::access::Access;
use crate::archetype::ArchetypeId;
use crate::system::param::{SystemParam, SystemParamFetch, SystemParamState};
use crate::system::{IntoSystem, System};
use crate::world::World;
use std::marker::PhantomData;
use ze_core::sync::SyncUnsafeCell;
use ze_ecs_macros::repeat_tuples_no_skip;

/// Implementation of [`System`] for functions/closures
pub struct FuncSystem<
    Input,
    Output,
    Param: SystemParam,
    F: FuncSystemFunction<Input, Output, Param, Marker>,
    Marker,
> {
    func: F,
    param_state: Option<Param::Fetch>,
    archetype_access: Access,
    archetype_registry_generation: u64,
    _marker: PhantomData<fn(Input, Marker) -> Output>,
}

impl<Input, Output, Param, F, Marker> System for FuncSystem<Input, Output, Param, F, Marker>
where
    Input: 'static,
    Output: 'static,
    Param: SystemParam + 'static,
    Marker: 'static,
    F: FuncSystemFunction<Input, Output, Param, Marker>,
{
    type Input = Input;
    type Output = Output;

    fn initialize(&mut self, world: &mut World) {
        self.param_state = Some(Param::Fetch::initialize(world, &mut self.archetype_access));
    }

    unsafe fn run(&mut self, input: Self::Input, world: &SyncUnsafeCell<World>) -> Self::Output {
        // SAFETY: Fetch is safe since caller is responsible for ensuring that
        // data is not accessed by multiple systems that collides
        let param = Param::Fetch::param(
            self.param_state
                .as_mut()
                .expect("Parameter state not initialized"),
            world,
        );
        self.func.run(input, param)
    }

    fn update_archetype_access(&mut self, world: &World) {
        if self.archetype_registry_generation != world.archetype_registry.generation() {
            let old_generation = self.archetype_registry_generation;
            self.archetype_registry_generation = world.archetype_registry.generation();

            // Using the generation number we can compute what archetypes were added
            for i in old_generation..self.archetype_registry_generation {
                self.param_state.as_mut().unwrap().update_archetype_access(
                    world.archetype_registry.get(i as ArchetypeId),
                    &mut self.archetype_access,
                );
            }
        }
    }

    fn archetype_access(&self) -> &Access {
        &self.archetype_access
    }
}

pub trait FuncSystemFunction<Input, Output, Param: SystemParam, Marker>:
    Send + Sync + 'static
{
    fn run(
        &mut self,
        input: Input,
        param: <<Param as SystemParam>::Fetch as SystemParamFetch>::Item,
    ) -> Output;
}

pub struct NoInputMarker;
pub struct InputMarker;

macro_rules! impl_func_system_function_trait {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Output, F: Send + Sync + 'static, $($param: SystemParam),*> FuncSystemFunction<(), Output, ($($param,)*), NoInputMarker> for F
        where for<'a> &'a mut F:
            FnMut($($param),*) -> Output +
            FnMut($(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> Output,
            Output: 'static
        {
            fn run(&mut self, _: (), param: <<($($param,)*) as SystemParam>::Fetch as SystemParamFetch>::Item) -> Output {
                #[allow(clippy::too_many_arguments)]
                fn run_func<Output, $($param,)*>(mut f: impl FnMut($($param,)*) -> Output, $($param: $param,)*) -> Output {
                    f($($param,)*)
                }
                let ($($param,)*) = param;
                run_func(self, $($param),*)
           }
        }

        #[allow(non_snake_case)]
        impl<Input, Output, F: Send + Sync + 'static, $($param: SystemParam),*> FuncSystemFunction<Input, Output, ($($param,)*), InputMarker> for F
        where for<'a> &'a mut F:
            FnMut(Input, $($param),*) -> Output +
            FnMut(Input, $(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> Output,
            Output: 'static
        {
            fn run(&mut self, input: Input, param: <<($($param,)*) as SystemParam>::Fetch as SystemParamFetch>::Item) -> Output {
                #[allow(clippy::too_many_arguments)]
                fn run_func<Input, Output, $($param,)*>(mut f: impl FnMut(Input, $($param,)*) -> Output, input: Input, $($param: $param,)*) -> Output {
                    f(input, $($param,)*)
                }
                let ($($param,)*) = param;
                run_func(self, input, $($param),*)
            }
        }
    };
}

repeat_tuples_no_skip!(impl_func_system_function_trait, 8, A);

impl<Input, Output, Param, F, Marker> IntoSystem<Input, Output, (Param, Marker)> for F
where
    Input: 'static,
    Output: 'static,
    Param: SystemParam + 'static,
    Marker: 'static,
    F: FuncSystemFunction<Input, Output, Param, Marker> + Send + Sync + 'static,
{
    type System = FuncSystem<Input, Output, Param, F, Marker>;

    fn into_system(self) -> Self::System {
        FuncSystem {
            func: self,
            param_state: None,
            archetype_access: Default::default(),
            archetype_registry_generation: 0,
            _marker: Default::default(),
        }
    }
}
