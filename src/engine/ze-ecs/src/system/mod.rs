use crate::access::Access;
use crate::system::condition::{ExecutionCondition, IntoExecutionCondition};
use crate::world::World;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use ze_core::sync::SyncUnsafeCell;

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct SystemId(&'static str);

impl Display for SystemId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait IntoSystemId {
    fn system_id(&self) -> SystemId;
}

impl IntoSystemId for &'static str {
    fn system_id(&self) -> SystemId {
        SystemId(self)
    }
}

pub trait System: Send + Sync + 'static {
    type Input;
    type Output;

    fn initialize(&mut self, world: &mut World);

    /// # Safety
    ///
    /// Caller must ensure that a system is run in a way that ensures that
    /// it will not collide with other systems while accessing archetype data
    unsafe fn run(&mut self, input: Self::Input, world: &SyncUnsafeCell<World>) -> Self::Output;

    /// Update the archetype access of this system
    fn update_archetype_access(&mut self, world: &World);

    /// Get the archetype access of this system
    fn archetype_access(&self) -> &Access;
}

pub type BoxedSystem<Input = (), Output = ()> = Box<dyn System<Input = Input, Output = Output>>;

pub trait IntoSystem<Input, Output, Params> {
    type System: System<Input = Input, Output = Output>;

    fn into_system(self) -> Self::System;
}

/// Describe a system and what are its dependencies
pub struct SystemDesc {
    id: Option<SystemId>,
    system: BoxedSystem<(), ()>,
    parent_sets: HashSet<SystemId>,
    before: Vec<SystemId>,
    after: Vec<SystemId>,
    conditions: Vec<ExecutionCondition>,
}

impl SystemDesc {
    pub fn new(system: BoxedSystem<(), ()>) -> Self {
        Self {
            id: None,
            system,
            parent_sets: HashSet::default(),
            before: vec![],
            after: vec![],
            conditions: vec![],
        }
    }
}

pub trait IntoSystemDesc<Params> {
    fn system_desc(self) -> SystemDesc;
    fn id(self, id: impl IntoSystemId) -> SystemDesc;
    fn in_set(self, set: impl IntoSystemId) -> SystemDesc;
    fn before(self, id: impl IntoSystemId) -> SystemDesc;
    fn after(self, id: impl IntoSystemId) -> SystemDesc;
    fn condition<CondParams>(
        self,
        condition: impl IntoExecutionCondition<CondParams>,
    ) -> SystemDesc;
}

impl<Params, F> IntoSystemDesc<Params> for F
where
    F: IntoSystem<(), (), Params>,
{
    fn system_desc(self) -> SystemDesc {
        SystemDesc::new(Box::new(self.into_system()))
    }

    fn id(self, id: impl IntoSystemId) -> SystemDesc {
        SystemDesc::new(Box::new(self.into_system())).id(id)
    }

    fn in_set(self, set: impl IntoSystemId) -> SystemDesc {
        SystemDesc::new(Box::new(self.into_system())).in_set(set)
    }

    fn before(self, id: impl IntoSystemId) -> SystemDesc {
        SystemDesc::new(Box::new(self.into_system())).before(id)
    }

    fn after(self, id: impl IntoSystemId) -> SystemDesc {
        SystemDesc::new(Box::new(self.into_system())).after(id)
    }

    fn condition<CondParams>(
        self,
        condition: impl IntoExecutionCondition<CondParams>,
    ) -> SystemDesc {
        SystemDesc::new(Box::new(self.into_system())).condition(condition)
    }
}

impl IntoSystemDesc<()> for SystemDesc {
    fn system_desc(self) -> SystemDesc {
        self
    }

    fn id(mut self, id: impl IntoSystemId) -> SystemDesc {
        self.id = Some(id.system_id());
        self
    }

    fn in_set(mut self, set: impl IntoSystemId) -> SystemDesc {
        self.parent_sets.insert(set.system_id());
        self
    }

    fn before(mut self, id: impl IntoSystemId) -> SystemDesc {
        self.before.push(id.system_id());
        self
    }

    fn after(mut self, id: impl IntoSystemId) -> SystemDesc {
        self.after.push(id.system_id());
        self
    }

    fn condition<CondParams>(
        mut self,
        condition: impl IntoExecutionCondition<CondParams>,
    ) -> SystemDesc {
        self.conditions.push(Box::new(condition.into_system()));
        self
    }
}

pub mod condition;
pub mod executor;
pub mod func_system;
pub mod param;
pub mod query;
pub mod registry;
pub mod schedule;
pub mod set;
