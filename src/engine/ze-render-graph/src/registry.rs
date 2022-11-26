use crate::FrameGraphTextureDesc;
use std::sync::Arc;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceHandle(pub(crate) usize);

pub(crate) struct Texture {
    pub desc: FrameGraphTextureDesc,
    pub resource: Option<Arc<ze_gfx::backend::Texture>>,
}

pub(crate) enum ResourceData {
    Texture(Texture),
    Proxy(ResourceHandle),
}

pub(crate) struct Resource {
    pub name: String,
    pub data: ResourceData,
    pub external: bool,
    pub last_pass_use: Option<usize>,
}

#[derive(Default)]
pub(crate) struct ResourceRegistry {
    resources: Vec<Resource>,
}

impl ResourceRegistry {
    pub fn create_texture(&mut self, name: &str, desc: FrameGraphTextureDesc) -> ResourceHandle {
        assert!(
            !self.resources.iter().any(|res| res.name == name),
            "Resource already exists"
        );
        self.resources.push(Resource {
            name: name.to_string(),
            data: ResourceData::Texture(Texture {
                desc,
                resource: None,
            }),
            external: false,
            last_pass_use: None,
        });
        ResourceHandle(self.resources.len() - 1)
    }

    pub fn create_proxy(&mut self, handle: ResourceHandle) -> ResourceHandle {
        self.resources.push(Resource {
            name: String::default(),
            data: ResourceData::Proxy(handle),
            external: false,
            last_pass_use: None,
        });
        ResourceHandle(self.resources.len() - 1)
    }

    pub fn resource(&self, handle: ResourceHandle) -> &Resource {
        &self.resources[handle.0]
    }

    pub fn resource_mut(&mut self, handle: ResourceHandle) -> &mut Resource {
        &mut self.resources[handle.0]
    }

    pub fn resolve_handle(&self, handle: ResourceHandle) -> ResourceHandle {
        match self.resources[handle.0].data {
            ResourceData::Proxy(proxy) => proxy,
            _ => handle,
        }
    }

    pub fn texture(&self, handle: ResourceHandle) -> &Texture {
        let resource = &self.resources[handle.0];
        if let ResourceData::Texture(texture) = &resource.data {
            texture
        } else {
            panic!("Resource is not a texture");
        }
    }

    pub fn texture_mut(&mut self, handle: ResourceHandle) -> &mut Texture {
        let resource = &mut self.resources[handle.0];
        if let ResourceData::Texture(texture) = &mut resource.data {
            texture
        } else {
            panic!("Resource is not a texture");
        }
    }

    pub fn is_texture(&self, handle: ResourceHandle) -> bool {
        let resource = &self.resources[handle.0];
        matches!(resource.data, ResourceData::Texture(_))
    }

    pub fn is_external(&self, handle: ResourceHandle) -> bool {
        let resource = &self.resources[handle.0];
        resource.external
    }

    pub fn resources(&self) -> &[Resource] {
        &self.resources
    }
}
