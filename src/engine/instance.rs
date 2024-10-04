use super::errors::Error as EngineError;
use super::vulkan as vk;
use std::sync::Arc;

pub struct Instance {
    instance: Arc<vk::Instance>,
    pub(super) device: Arc<vk::Device>,
    pub(super) queue: Arc<vk::Queue>,
    pub(super) queue_family_index: u32,
    pub(super) memory_allocator: Arc<vk::StandardMemoryAllocator>,
    pub(super) command_buffer_allocator: Arc<vk::StandardCommandBufferAllocator>,
}

impl Instance {
    pub fn new() -> Result<Self, EngineError> {
        let library = match vk::VulkanLibrary::new() {
            Ok(library) => library,
            _ => return EngineError::VkLibraryLoad.into(),
        };

        let instance = match vk::Instance::new(library, vk::InstanceCreateInfo::default()) {
            Ok(instance) => instance,
            _ => return EngineError::VkInstanceCreate.into(),
        };

        let physical_device = instance
            .enumerate_physical_devices()
            .expect("failed to enumerate devices")
            .next()
            .expect("no devices available");

        let queue_family_index = match physical_device.queue_family_properties().iter().position(
            |queue_family_properties| {
                queue_family_properties
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            },
        ) {
            Some(queue_family_index) => queue_family_index as u32,
            None => return EngineError::QueueFamilySearch.into(),
        };

        let (device, mut queues) = match vk::Device::new(
            physical_device,
            vk::DeviceCreateInfo {
                queue_create_infos: vec![vk::QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        ) {
            Ok(device) => device,
            _ => return EngineError::VkDeviceCreate.into(),
        };

        Ok(Self {
            instance: instance.clone(),
            device: device.clone(),
            queue: queues.next().unwrap(),
            queue_family_index,
            memory_allocator: Arc::new(vk::StandardMemoryAllocator::new_default(device.clone())),
            command_buffer_allocator: Arc::new(vk::StandardCommandBufferAllocator::new(
                device.clone(),
                vk::StandardCommandBufferAllocatorCreateInfo::default(),
            )),
        })
    }

    pub fn api_version(&self) -> Version {
        Version {
            major: self.instance.api_version().major,
            minor: self.instance.api_version().minor,
            patch: self.instance.api_version().patch,
        }
    }
}

pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() {
        assert!(Instance::new().is_ok());
    }

    #[test]
    fn version() {
        let instance = Instance::new().unwrap();
        let library = vk::VulkanLibrary::new().unwrap();

        assert!(instance.api_version().major <= library.api_version().major);
        assert!(instance.api_version().minor <= library.api_version().minor);
        assert!(instance.api_version().patch <= library.api_version().patch);
    }
}
