pub use uuid::Uuid;
pub use ze_core_derive::*;

pub trait TypeUuid {
    fn type_uuid() -> Uuid;
}

/// Like `TypeUuid` but allow to get the type UUID behind a `dyn` trait object
pub trait DynTypeUuid {
    fn dyn_type_uuid(&self) -> Uuid;
}

impl<T: TypeUuid> DynTypeUuid for T {
    fn dyn_type_uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}
