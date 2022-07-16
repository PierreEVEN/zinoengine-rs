use crate::TextureInfo;
use std::collections::HashMap;
use std::sync::Arc;
use ze_gfx::backend::{RenderTargetView, Texture};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PhysicalResourceHandle(usize);

pub enum PhysicalResourceTextureView {
    RTV(Arc<RenderTargetView>),
}

enum PhysicalResource {
    Texture(Arc<Texture>, PhysicalResourceTextureView),
}

/// Registry containing physical resources used by a render graph
pub struct PhysicalResourceRegistry {
    resources: Vec<PhysicalResource>,
    resource_name_map: HashMap<String, PhysicalResourceHandle>,
}

impl PhysicalResourceRegistry {
    pub fn new() -> Self {
        Self {
            resources: vec![],
            resource_name_map: Default::default(),
        }
    }

    pub fn get_handle_from_name(&self, name: &str) -> Result<PhysicalResourceHandle, ()> {
        if let Some(handle) = self.resource_name_map.get(name) {
            Ok(*handle)
        } else {
            Err(())
        }
    }

    pub fn get_texture(&self, handle: PhysicalResourceHandle) -> Result<&Arc<Texture>, ()> {
        let resource = &self.resources[handle.0];
        if let PhysicalResource::Texture(texture, _) = resource {
            return Ok(texture);
        }

        Err(())
    }

    pub fn get_render_target_view(
        &self,
        handle: PhysicalResourceHandle,
    ) -> Result<&Arc<RenderTargetView>, ()> {
        let resource = &self.resources[handle.0];
        if let PhysicalResource::Texture(_, PhysicalResourceTextureView::RTV(rtv)) = resource {
            return Ok(rtv);
        }

        Err(())
    }

    pub fn insert_or_update_existing_texture(
        &mut self,
        name: &str,
        texture: Arc<Texture>,
        view: PhysicalResourceTextureView,
    ) -> PhysicalResourceHandle {
        if let Some(resource_handle) = self.resource_name_map.get(name) {
            let mut resource = &mut self.resources[resource_handle.0];
            if let PhysicalResource::Texture(existing_texture, existing_view) = resource {
                *existing_texture = texture;
                *existing_view = view;
                *resource_handle
            } else {
                panic!("Existing resource {} is not a texture!", name)
            }
        } else {
            let handle = PhysicalResourceHandle(self.resources.len());
            self.resources
                .push(PhysicalResource::Texture(texture, view));
            self.resource_name_map.insert(name.to_string(), handle);
            handle
        }
    }

    pub fn get_or_create_texture(
        &mut self,
        name: &str,
        info: &TextureInfo,
    ) -> PhysicalResourceHandle {
        if let Some(resource_handle) = self.resource_name_map.get(name) {
            return *resource_handle;
        }

        todo!()
    }
}
