use crate::system::condition::{ExecutionCondition, IntoExecutionCondition};
use crate::system::{IntoSystemId, SystemId};
use std::collections::HashSet;

pub struct SystemSetDesc {
    pub(crate) id: SystemId,
    pub(crate) parent_sets: HashSet<SystemId>,
    pub(crate) before: Vec<SystemId>,
    pub(crate) after: Vec<SystemId>,
    pub(crate) conditions: Vec<ExecutionCondition>,
}

impl SystemSetDesc {
    pub(crate) fn new(id: SystemId) -> Self {
        Self {
            id,
            parent_sets: HashSet::default(),
            before: vec![],
            after: vec![],
            conditions: vec![],
        }
    }
}

pub trait IntoSystemSetDesc {
    fn system_set(self) -> SystemSetDesc;
    fn in_set(self, set: impl IntoSystemId) -> SystemSetDesc;
    fn before(self, id: impl IntoSystemId) -> SystemSetDesc;
    fn after(self, id: impl IntoSystemId) -> SystemSetDesc;
    fn condition<Params>(self, condition: impl IntoExecutionCondition<Params>) -> SystemSetDesc;
}

impl<T: IntoSystemId> IntoSystemSetDesc for T {
    fn system_set(self) -> SystemSetDesc {
        SystemSetDesc::new(self.system_id())
    }

    fn in_set(self, set: impl IntoSystemId) -> SystemSetDesc {
        SystemSetDesc::new(self.system_id()).in_set(set)
    }

    fn before(self, set: impl IntoSystemId) -> SystemSetDesc {
        SystemSetDesc::new(self.system_id()).before(set)
    }

    fn after(self, set: impl IntoSystemId) -> SystemSetDesc {
        SystemSetDesc::new(self.system_id()).after(set)
    }

    fn condition<Params>(self, condition: impl IntoExecutionCondition<Params>) -> SystemSetDesc {
        SystemSetDesc::new(self.system_id()).condition(condition)
    }
}

impl IntoSystemSetDesc for SystemSetDesc {
    fn system_set(self) -> SystemSetDesc {
        self
    }

    fn in_set(mut self, set: impl IntoSystemId) -> SystemSetDesc {
        self.parent_sets.insert(set.system_id());
        self
    }

    fn before(mut self, set: impl IntoSystemId) -> SystemSetDesc {
        self.before.push(set.system_id());
        self
    }

    fn after(mut self, set: impl IntoSystemId) -> SystemSetDesc {
        self.after.push(set.system_id());
        self
    }

    fn condition<Params>(
        mut self,
        condition: impl IntoExecutionCondition<Params>,
    ) -> SystemSetDesc {
        self.conditions.push(Box::new(condition.into_system()));
        self
    }
}
