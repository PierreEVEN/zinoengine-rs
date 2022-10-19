use crate::TextureInfo;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use ze_core::sparse_vec::SparseVec;
use ze_gfx::backend::{RenderTargetView, Texture};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PhysicalResourceHandle(usize);

pub enum PhysicalResourceTextureView {
    RTV(Arc<RenderTargetView>),
}

enum PhysicalResource {
    Texture(Arc<Texture>, PhysicalResourceTextureView),
    _Buffer(),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    UnknownResource,
    InvalidResourceType,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

/// Registry containing physical resources used by a render graph
pub struct PhysicalResourceRegistry {
    resources: SparseVec<PhysicalResource>,
    resource_name_map: HashMap<String, PhysicalResourceHandle>,
}

impl PhysicalResourceRegistry {
    pub fn new() -> Self {
        Self {
            resources: SparseVec::default(),
            resource_name_map: Default::default(),
        }
    }

    pub fn insert_or_update_existing_texture(
        &mut self,
        name: &str,
        texture: Arc<Texture>,
        view: PhysicalResourceTextureView,
    ) -> PhysicalResourceHandle {
        if let Some(resource_handle) = self.resource_name_map.get(name) {
            let resource = &mut self.resources[resource_handle.0];
            if let PhysicalResource::Texture(existing_texture, existing_view) = resource {
                *existing_texture = texture;
                *existing_view = view;
                *resource_handle
            } else {
                panic!("Existing resource {} is not a texture!", name)
            }
        } else {
            let index = self
                .resources
                .push(PhysicalResource::Texture(texture, view));
            let handle = PhysicalResourceHandle(index);
            self.resource_name_map.insert(name.to_string(), handle);
            handle
        }
    }

    pub fn remove_resource(&mut self, name: &str) -> Result<(), Error> {
        if let Some(resource_handle) = self.resource_name_map.remove(name) {
            self.resources.remove(resource_handle.0);
            Ok(())
        } else {
            Err(Error::UnknownResource)
        }
    }

    pub fn handle_from_name(&self, name: &str) -> Result<PhysicalResourceHandle, Error> {
        if let Some(handle) = self.resource_name_map.get(name) {
            Ok(*handle)
        } else {
            Err(Error::UnknownResource)
        }
    }

    pub fn texture(&self, handle: PhysicalResourceHandle) -> Result<&Arc<Texture>, Error> {
        let resource = &self.resources[handle.0];
        if let PhysicalResource::Texture(texture, _) = resource {
            return Ok(texture);
        }

        Err(Error::InvalidResourceType)
    }

    pub fn render_target_view(
        &self,
        handle: PhysicalResourceHandle,
    ) -> Result<&Arc<RenderTargetView>, Error> {
        let resource = &self.resources[handle.0];
        if let PhysicalResource::Texture(_, PhysicalResourceTextureView::RTV(rtv)) = resource {
            return Ok(rtv);
        }

        Err(Error::InvalidResourceType)
    }

    pub fn get_or_create_texture(&mut self, name: &str, _: &TextureInfo) -> PhysicalResourceHandle {
        if let Some(resource_handle) = self.resource_name_map.get(name) {
            return *resource_handle;
        }

        todo!()
    }
}
