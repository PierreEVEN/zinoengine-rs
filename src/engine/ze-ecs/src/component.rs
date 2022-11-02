use crate::erased_vec::TypeInfo;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;

/// Data storage for a entity
pub trait Component: Send + Sync + 'static {
    fn component_id() -> ComponentId;
}

/// Global component id counter used for derive macros
#[allow(dead_code)] // Implemented by derive macros
pub static COMPONENT_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub type ComponentId = usize;

pub struct ComponentInfo {
    pub(crate) type_info: TypeInfo,
}

impl ComponentInfo {
    pub fn new<T: Component>() -> Self {
        Self {
            type_info: TypeInfo::new::<T>(),
        }
    }
}

#[derive(Default)]
pub(crate) struct ComponentRegistry {
    components: HashMap<ComponentId, ComponentInfo>,
}

impl ComponentRegistry {
    pub fn register(&mut self, id: ComponentId, info: ComponentInfo) {
        assert!(!self.components.contains_key(&id));
        self.components.insert(id, info);
    }

    pub fn get(&self, id: &ComponentId) -> Option<&ComponentInfo> {
        self.components.get(id)
    }

    /// # Safety
    ///
    /// `id` must point to a valid component
    pub unsafe fn get_unchecked(&self, id: &ComponentId) -> &ComponentInfo {
        debug_assert!(self.components.contains_key(id));
        &self.components[id]
    }
}

#[cfg(test)]
mod tests {
    use crate::component::{Component, ComponentId, ComponentInfo, ComponentRegistry};
    use std::alloc::Layout;

    #[test]
    fn registry_register_and_get() {
        struct MyComponent {
            _a: i32,
            _b: i32,
            _c: Vec<u128>,
        }

        impl Component for MyComponent {
            fn component_id() -> ComponentId {
                0
            }
        }

        let mut registry = ComponentRegistry::default();
        registry.register(0, ComponentInfo::new::<MyComponent>());
        assert_eq!(
            registry.get(&0).unwrap().type_info.layout,
            Layout::new::<MyComponent>()
        );
    }
}
