use crate::component::{ComponentId, ComponentInfo, ComponentRegistry};
use crate::entity::Entity;
use crate::sparse_set::{SparseSet, TypeErasedSparseSet};
use fnv::FnvHashMap;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ptr::NonNull;

/// Store a specific type of component inside an archetype
pub(crate) struct Column {
    components: TypeErasedSparseSet,
}

impl Column {
    pub fn new(component_info: &ComponentInfo) -> Self {
        Self {
            components: TypeErasedSparseSet::new(component_info.type_info),
        }
    }

    pub fn components(&self) -> &TypeErasedSparseSet {
        &self.components
    }

    pub fn components_mut(&mut self) -> &mut TypeErasedSparseSet {
        &mut self.components
    }
}

/// Store what archetype to use when a component is added/removed
#[derive(Default)]
pub struct ArchetypeRelationEdge {
    /// Id of the archetype to use when the component is added
    pub add: Option<ArchetypeId>,

    /// Id of the archetype to use when the component is removed
    pub remove: Option<ArchetypeId>,
}

/// Identify a component type in an archetype globally
pub type ComponentArchetypeId = usize;

pub struct Archetype {
    id: ArchetypeId,
    components: Vec<ComponentId>,
    component_archetype_ids: Vec<ComponentArchetypeId>,
    columns: SparseSet<UnsafeCell<Column>>,
    archetype_component_edges: FnvHashMap<ComponentId, ArchetypeRelationEdge>,
    entities: Vec<Entity>,
}

impl Archetype {
    /// # Safety
    ///
    /// `components` must reference valid components
    pub(crate) unsafe fn new_unchecked(
        id: usize,
        component_db: &ComponentRegistry,
        components: Vec<ComponentId>,
        component_archetype_ids: Vec<ComponentArchetypeId>,
    ) -> Self {
        let mut columns = SparseSet::default();
        for id in &components {
            let component = component_db.get_unchecked(id);
            columns.insert(*id, UnsafeCell::new(Column::new(component)));
        }

        Self {
            id,
            components,
            component_archetype_ids,
            columns,
            archetype_component_edges: Default::default(),
            entities: vec![],
        }
    }

    /// # Safety
    ///
    /// `values` must refer to valid component data
    pub unsafe fn insert_row(
        &mut self,
        entity: Entity,
        values: Vec<(ComponentId, NonNull<u8>)>,
    ) -> usize {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
        debug_assert_eq!(
            self.components,
            values.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            "Component ids must match the archetype"
        );

        for (id, value) in values {
            #[cfg(feature = "profiling")]
            puffin::profile_scope!("Add component", id.to_string());
            let column = self.columns.get_mut(id).unwrap_unchecked().get_mut();
            column
                .components
                .insert_unchecked(entity.id() as usize, value);
        }

        self.entities.push(entity);
        self.entities.len() - 1
    }

    /// Remove the entity's row
    /// Takes a `should_drop` function to determines if a specific component should be dropped and not forgotten
    pub fn remove_row<F: Fn(&ComponentId) -> bool>(
        &mut self,
        entity_index: usize,
        entity: Entity,
        should_drop: F,
    ) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
        self.entities.swap_remove(entity_index);

        for id in &self.components {
            if should_drop(id) {
                self.columns
                    .get_mut(*id)
                    .unwrap()
                    .get_mut()
                    .components
                    .remove(entity.id() as usize)
            } else {
                self.columns
                    .get_mut(*id)
                    .unwrap()
                    .get_mut()
                    .components
                    .remove_forget(entity.id() as usize)
            }
        }
    }

    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    pub fn components(&self) -> &[ComponentId] {
        &self.components
    }

    pub fn components_archetype_ids(&self) -> &[ComponentArchetypeId] {
        &self.component_archetype_ids
    }

    pub fn edge(&mut self, component: &ComponentId) -> &mut ArchetypeRelationEdge {
        self.archetype_component_edges
            .entry(*component)
            .or_default()
    }

    pub(crate) fn columns(&self) -> &SparseSet<UnsafeCell<Column>> {
        &self.columns
    }

    pub(crate) fn columns_mut(&mut self) -> &mut SparseSet<UnsafeCell<Column>> {
        &mut self.columns
    }

    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }
}

pub type ArchetypeId = usize;

#[derive(Default)]
pub(crate) struct ArchetypeRegistry {
    archetypes: Vec<Archetype>,
    archetype_id_map: HashMap<Vec<ComponentId>, ArchetypeId>,

    /// Current generation of the registry, updated everytime an archetype is added
    generation: u64,
    next_archetype_component_id: usize,
}

impl ArchetypeRegistry {
    pub fn get(&self, id: ArchetypeId) -> &Archetype {
        &self.archetypes[id]
    }

    pub fn get_mut(&mut self, id: ArchetypeId) -> &mut Archetype {
        &mut self.archetypes[id]
    }

    /// # Safety
    ///
    /// `components` must reference valid components
    pub unsafe fn register(
        &mut self,
        component_registry: &ComponentRegistry,
        components: &[ComponentId],
    ) -> &mut Archetype {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
        // Assign archetype ids
        let components_archetype_ids = components
            .iter()
            .map(|_| {
                let archetype_id = self.next_archetype_component_id;
                self.next_archetype_component_id += 1;
                archetype_id
            })
            .collect::<Vec<_>>();

        let id = self.archetypes.len();
        self.archetypes.push(Archetype::new_unchecked(
            id,
            component_registry,
            components.into(),
            components_archetype_ids,
        ));
        self.archetype_id_map.insert(components.into(), id);
        self.generation += 1;
        &mut self.archetypes[id]
    }

    pub unsafe fn get_or_register_mut(
        &mut self,
        component_registry: &ComponentRegistry,
        components: &[ComponentId],
    ) -> &mut Archetype {
        if let Some(id) = self.archetype_id_map.get(components) {
            &mut self.archetypes[*id]
        } else {
            self.register(component_registry, components)
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}
