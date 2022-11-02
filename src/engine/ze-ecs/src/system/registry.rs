use crate::system::condition::ExecutionCondition;
use crate::system::schedule::Schedule;
use crate::system::set::IntoSystemSetDesc;
use crate::system::{BoxedSystem, IntoSystemDesc, IntoSystemId, SystemId};
use crate::world::World;
use bitvec::bitvec;
use petgraph::algo::toposort;
use petgraph::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::mem;
use std::rc::Rc;

pub(crate) type NodeId = u32;

enum SystemNodeType {
    SystemSet,
    System(Rc<RefCell<BoxedSystem>>),
}

struct NodeData {
    ty: SystemNodeType,
    parents: HashSet<NodeId>,
    before: Vec<NodeId>,
    after: Vec<NodeId>,
    conditions: Rc<RefCell<Vec<ExecutionCondition>>>,
}

impl NodeData {
    fn is_set(&self) -> bool {
        matches!(self.ty, SystemNodeType::SystemSet)
    }

    fn is_system(&self) -> bool {
        matches!(self.ty, SystemNodeType::System(_))
    }

    fn as_system(&self) -> &Rc<RefCell<BoxedSystem>> {
        match &self.ty {
            SystemNodeType::SystemSet => panic!("Expected a system"),
            SystemNodeType::System(system) => system,
        }
    }
}

/// Registry of system and their system sets
/// Systems are stored inside of a graph, systems must always be placed inside a system set
/// while system sets can be independent (each root system set with no ancestor will create a
/// schedule)
#[derive(Default)]
pub(crate) struct SystemRegistry {
    hierarchy: DiGraphMap<NodeId, ()>,
    node_idx_to_node_data: HashMap<NodeId, NodeData>,
    id_to_node_idx: HashMap<SystemId, NodeId>,
    next_node_id: NodeId,
    schedules: HashMap<NodeId, Schedule>,
    root_nodes_to_update: HashSet<NodeId>,
    nodes_systems_conditions_to_initialize: Vec<NodeId>,
}

impl SystemRegistry {
    pub(crate) fn update(&mut self, world: &mut World) {
        for node in self.nodes_systems_conditions_to_initialize.drain(..) {
            let node_data = self.node_idx_to_node_data.get_mut(&node).unwrap();

            // If node is a system, also initialize the system
            if let SystemNodeType::System(system) = &node_data.ty {
                system.borrow_mut().initialize(world);
            }

            let mut conditions = RefCell::borrow_mut(&node_data.conditions);
            for condition in conditions.iter_mut() {
                condition.initialize(world);
            }
        }

        let mut root_nodes_to_update = mem::take(&mut self.root_nodes_to_update);
        for node in root_nodes_to_update.drain() {
            self.update_root_node(node);
        }
    }

    pub(crate) fn schedule_mut(&mut self, id: impl IntoSystemId) -> &mut Schedule {
        let node_id = self.id_to_node_idx[&id.system_id()];
        self.schedules.get_mut(&node_id).unwrap()
    }

    fn update_root_node(&mut self, root_id: NodeId) {
        let mut dependency_graph = DiGraphMap::<NodeId, ()>::new();
        let mut bfs = Bfs::new(&self.hierarchy, root_id);
        while let Some(node) = bfs.next(&self.hierarchy) {
            let node_info = &self.node_idx_to_node_data[&node];
            dependency_graph.add_node(node);

            for before in &node_info.before {
                dependency_graph.add_edge(node, *before, ());
            }

            for after in &node_info.after {
                dependency_graph.add_edge(*after, node, ());
            }
        }

        let toposorted_dependency_graph =
            toposort(&dependency_graph, None).expect("Cycle in dependency graph");

        let toposorted_systems = toposorted_dependency_graph
            .iter()
            .filter(|node| self.node_idx_to_node_data[node].is_system())
            .cloned()
            .collect::<Vec<_>>();

        let toposorted_sets = toposorted_dependency_graph
            .iter()
            .filter(|node| self.node_idx_to_node_data[node].is_set())
            .cloned()
            .collect::<Vec<_>>();

        let mut schedule = self.schedules.get_mut(&root_id).unwrap();
        schedule.systems = toposorted_systems
            .iter()
            .map(|id| self.node_idx_to_node_data[id].as_system().clone())
            .collect::<Vec<_>>();
        schedule.systems_dependencies = toposorted_systems
            .iter()
            .map(|id| {
                let dependency_count = dependency_graph.neighbors_directed(*id, Incoming).count();
                let dependents = dependency_graph
                    .neighbors_directed(*id, Outgoing)
                    .map(|id| toposorted_systems.iter().position(|x| *x == id).unwrap())
                    .collect::<Vec<_>>();
                (dependency_count, dependents)
            })
            .collect::<Vec<_>>();
        schedule.systems_conditions = toposorted_systems
            .iter()
            .map(|id| self.node_idx_to_node_data[id].conditions.clone())
            .collect::<Vec<_>>();
        schedule.set_conditions = toposorted_sets
            .iter()
            .map(|id| self.node_idx_to_node_data[id].conditions.clone())
            .collect::<Vec<_>>();

        // Fill system_is_in_set_bitset
        schedule.system_is_in_set_bitset = Vec::with_capacity(toposorted_systems.len());
        for id in &toposorted_systems {
            let node = &self.node_idx_to_node_data[id];
            let mut bitset = bitvec![0; toposorted_sets.len()];
            for (set_idx, set_node_id) in toposorted_sets.iter().enumerate() {
                if node.parents.contains(set_node_id) {
                    bitset.set(set_idx, true);
                }
            }
            schedule.system_is_in_set_bitset.push(bitset);
        }

        // Fill set_is_in_set_bitset
        schedule.set_is_in_set_bitset = Vec::with_capacity(toposorted_sets.len());
        for id in &toposorted_sets {
            let node = &self.node_idx_to_node_data[id];
            let mut bitset = bitvec![0; toposorted_sets.len()];
            for (i, other_set_node_id) in toposorted_sets.iter().enumerate() {
                if node.parents.contains(other_set_node_id) {
                    bitset.set(i, true);
                }
            }
            schedule.set_is_in_set_bitset.push(bitset);
        }

        // Fill set_has_system_bitset
        for set_id in &toposorted_sets {
            let mut bitset = bitvec![0; toposorted_systems.len()];
            for (i, system_node_id) in toposorted_systems.iter().enumerate() {
                let system_node = &self.node_idx_to_node_data[system_node_id];
                if system_node.parents.contains(set_id) {
                    bitset.set(i, true);
                }
            }
            schedule.set_has_system_bitset.push(bitset);
        }
    }

    pub fn add_system_set(&mut self, set: impl IntoSystemSetDesc) {
        let set_desc = set.system_set();
        let node_id = self.next_node_id();
        self.hierarchy.add_node(node_id);
        self.id_to_node_idx.insert(set_desc.id, node_id);

        // If there is no parent, this is a root system set (schedule)
        if set_desc.parent_sets.is_empty() {
            self.schedules.insert(node_id, Schedule::default());
        } else {
            for parent_set in &set_desc.parent_sets {
                let parent_node_id = self.id_to_node_idx[parent_set];
                self.hierarchy.add_edge(parent_node_id, node_id, ());
            }
        }

        let before = set_desc
            .before
            .iter()
            .map(|id| {
                *self
                    .id_to_node_idx
                    .get(id)
                    .expect("Node depends on a non existing node")
            })
            .collect::<Vec<_>>();

        let after = set_desc
            .after
            .iter()
            .map(|id| {
                *self
                    .id_to_node_idx
                    .get(id)
                    .expect("Node depends on a non existing node")
            })
            .collect::<Vec<_>>();

        let parents = set_desc
            .parent_sets
            .iter()
            .map(|id| {
                *self
                    .id_to_node_idx
                    .get(id)
                    .expect("Node depends on a non existing node")
            })
            .collect::<HashSet<_>>();

        self.initialize_node(
            node_id,
            SystemNodeType::SystemSet,
            parents,
            before,
            after,
            set_desc.conditions,
        );
    }

    pub fn add_system<Params>(&mut self, system: impl IntoSystemDesc<Params>) {
        let system_desc = system.system_desc();
        let node_id = self.next_node_id();
        let system_id = system_desc.id.expect("System must have an id");
        assert!(
            !self.id_to_node_idx.contains_key(&system_id),
            "System id must be unique"
        );
        self.hierarchy.add_node(node_id);
        self.id_to_node_idx.insert(system_id, node_id);

        let before = system_desc
            .before
            .iter()
            .map(|id| {
                *self.id_to_node_idx.get(id).unwrap_or_else(|| {
                    panic!(
                        "System \"{}\" depends on a non existing system \"{}\"",
                        system_id, id
                    )
                })
            })
            .collect::<Vec<_>>();

        let after = system_desc
            .after
            .iter()
            .map(|id| {
                *self
                    .id_to_node_idx
                    .get(id)
                    .expect("Node depends on a non existing node")
            })
            .collect::<Vec<_>>();

        let parents = system_desc
            .parent_sets
            .iter()
            .map(|id| {
                *self
                    .id_to_node_idx
                    .get(id)
                    .expect("Node depends on a non existing node")
            })
            .collect::<HashSet<_>>();

        assert!(!parents.is_empty(), "A system must have a parent set");

        self.initialize_node(
            node_id,
            SystemNodeType::System(Rc::new(RefCell::new(system_desc.system))),
            parents,
            before,
            after,
            system_desc.conditions,
        );
    }

    fn initialize_node(
        &mut self,
        node_id: NodeId,
        ty: SystemNodeType,
        parents: HashSet<NodeId>,
        before: Vec<NodeId>,
        after: Vec<NodeId>,
        conditions: Vec<ExecutionCondition>,
    ) {
        // Before & afters must have the same parents as ours
        for before in &before {
            let node_info = &self.node_idx_to_node_data[before];
            if node_info.parents.is_disjoint(&parents) {
                panic!(
                    "Node {} depends on a node ({}) in an unrelated system set tree",
                    node_id, before
                );
            }
        }

        for after in &after {
            let node_info = &self.node_idx_to_node_data[after];
            if node_info.parents.is_disjoint(&parents) {
                panic!(
                    "Node {} depends on a node ({}) in an unrelated system set tree",
                    node_id, after
                );
            }
        }

        for &parent_node_id in &parents {
            self.hierarchy.add_edge(parent_node_id, node_id, ());
        }

        let node_data = NodeData {
            ty,
            parents,
            before,
            after,
            conditions: Rc::new(RefCell::new(conditions)),
        };

        self.nodes_systems_conditions_to_initialize.push(node_id);
        self.node_idx_to_node_data.insert(node_id, node_data);

        // We need to tell our root system sets that they need to update
        /// Get the root nodes ids for a node
        fn root_nodes_id(hierarchy: &DiGraphMap<NodeId, ()>, id: NodeId) -> Vec<NodeId> {
            let edges = hierarchy.edges_directed(id, Incoming).collect::<Vec<_>>();
            if edges.is_empty() {
                vec![id]
            } else {
                let mut nodes = vec![];
                for edge in edges {
                    nodes.append(&mut root_nodes_id(hierarchy, edge.source()));
                }
                nodes
            }
        }

        let root_nodes = root_nodes_id(&self.hierarchy, node_id);
        for root_node in root_nodes {
            self.root_nodes_to_update.insert(root_node);
        }
    }

    fn next_node_id(&mut self) -> NodeId {
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        node_id
    }
}

#[cfg(test)]
mod tests {
    use crate::system::registry::SystemRegistry;
    use crate::system::IntoSystemDesc;

    #[test]
    #[should_panic = "System \"A\" depends on a non existing system \"B\""]
    fn non_existing_node_panic() {
        let mut registry = SystemRegistry::default();
        registry.add_system_set("X");
        registry.add_system((|| {}).id("A").in_set("X").before("B"));
        registry.add_system((|| {}).id("B").in_set("X").before("A"));
    }

    #[test]
    #[should_panic = "Node 3 depends on a node (2) in an unrelated system set tree"]
    fn unrelated_dependency_panic() {
        let mut registry = SystemRegistry::default();
        registry.add_system_set("X");
        registry.add_system_set("Y");
        registry.add_system((|| {}).id("A").in_set("X"));
        registry.add_system((|| {}).id("B").in_set("Y").before("A"));
    }
}
