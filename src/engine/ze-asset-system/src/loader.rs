use crate::Asset;
use std::io::Read;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    CannotDeserialize,
}

pub trait AssetLoader {
    fn load(&self, uuid: Uuid, asset: &mut dyn Read) -> Result<Arc<dyn Asset>, Error>;
}
