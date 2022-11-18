use crate::archetype::ArchetypeId;

#[derive(Copy, Clone, Hash, Eq, Ord, PartialOrd, PartialEq)]
pub struct Entity {
    generation: u32,
    id: u32,
}

impl Entity {
    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

#[derive(Default)]
pub(crate) struct EntityData {
    generation: u32,
    archetype: Option<(ArchetypeId, usize)>,
}

#[derive(Default)]
pub(crate) struct EntityRegistry {
    datas: Vec<EntityData>,
    available_ids: Vec<u32>,
    free_id: u32,
    entity_count: u32,
}

impl EntityRegistry {
    pub fn alloc(&mut self) -> Entity {
        if let Some(id) = self.available_ids.pop() {
            self.entity_count += 1;
            Entity {
                generation: self.datas[id as usize].generation,
                id,
            }
        } else {
            self.datas.push(EntityData::default());
            let id = self.free_id;
            self.free_id += 1;
            self.entity_count += 1;
            Entity { generation: 0, id }
        }
    }

    pub fn free(&mut self, entity: Entity) {
        let data = &mut self.datas[entity.id as usize];
        data.generation += 1;
        data.archetype = None;
        self.entity_count -= 1;
        self.available_ids.push(entity.id);
    }

    pub fn set_archetype_id(&mut self, entity: Entity, archetype: ArchetypeId, index: usize) {
        self.datas[entity.id as usize].archetype = Some((archetype, index));
    }

    pub fn is_valid(&self, entity: Entity) -> bool {
        self.datas[entity.id as usize].generation == entity.generation
    }

    pub fn archetype_id(&self, entity: Entity) -> (ArchetypeId, usize) {
        debug_assert!(self.is_valid(entity));
        self.datas[entity.id as usize]
            .archetype
            .expect("Entity must have an archetype")
    }

    /// Get an entity handle from its id
    pub fn entity(&self, id: u32) -> Entity {
        Entity {
            generation: self.datas[id as usize].generation,
            id,
        }
    }

    pub fn count(&self) -> u32 {
        self.entity_count
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::EntityRegistry;

    #[test]
    fn alloc() {
        let mut registry = EntityRegistry::default();
        let entity = registry.alloc();
        assert_eq!(entity.generation(), 0);
        assert_eq!(entity.id(), 0);
        assert!(registry.is_valid(entity));
    }

    #[test]
    fn free() {
        let mut registry = EntityRegistry::default();
        let entity = registry.alloc();
        registry.free(entity);
        assert!(!registry.is_valid(entity));
    }

    #[test]
    fn id_recycling() {
        let mut registry = EntityRegistry::default();
        let entity = registry.alloc();
        registry.free(entity);
        assert!(!registry.is_valid(entity));

        let entity = registry.alloc();
        assert_eq!(entity.generation(), 1);
        assert_eq!(entity.id(), 0);
        assert!(registry.is_valid(entity));
    }
}
