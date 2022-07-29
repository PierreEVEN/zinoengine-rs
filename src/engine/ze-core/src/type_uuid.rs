pub use uuid::Uuid;
pub use ze_core_derive::*;

pub trait TypeUuid {
    fn type_uuid() -> Uuid;
}
