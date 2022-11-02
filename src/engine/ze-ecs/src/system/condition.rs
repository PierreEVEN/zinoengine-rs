use crate::system::{BoxedSystem, IntoSystem};

pub type ExecutionCondition = BoxedSystem<(), bool>;

pub trait IntoExecutionCondition<Params>: IntoSystem<(), bool, Params> {}
impl<Params, S> IntoExecutionCondition<Params> for S where S: IntoSystem<(), bool, Params> {}
