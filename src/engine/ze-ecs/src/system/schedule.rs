use crate::system::condition::ExecutionCondition;
use crate::system::BoxedSystem;
use bitvec::vec::BitVec;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
pub struct Schedule {
    pub systems: Vec<Rc<RefCell<BoxedSystem>>>,

    /// Number of dependencies of each systems and the dependants of the dependency
    pub systems_dependencies: Vec<(usize, Vec<usize>)>,
    pub systems_conditions: Vec<Rc<RefCell<Vec<ExecutionCondition>>>>,
    pub set_conditions: Vec<Rc<RefCell<Vec<ExecutionCondition>>>>,

    /// Tells if a system is contained inside a set
    pub system_is_in_set_bitset: Vec<BitVec>,

    /// Tells if set is contained inside a set
    pub set_is_in_set_bitset: Vec<BitVec>,

    /// Tells if set has a system
    pub set_has_system_bitset: Vec<BitVec>,
}
