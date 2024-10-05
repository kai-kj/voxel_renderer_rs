use super::super::{
    buffer::{Buffer, BufferBinding},
    errors::{EngineError, Result},
    instance::Instance,
    vulkan as vk,
};
use std::ops::Range;

#[derive(Clone)]
pub struct GpuBuffer<T> {
    pub(super) buffer: vk::Subbuffer<[T]>,
}

impl<T> GpuBuffer<T>
where
    T: vk::BufferContents + Clone + Copy,
{
    pub fn empty(instance: &Instance, len: usize) -> Result<GpuBuffer<T>> {
        if len == 0 {
            return EngineError::ZeroSized.into_result();
        }

        let buffer = vk::Buffer::new_slice(
            instance.memory_allocator.clone(),
            vk::BufferCreateInfo {
                usage: vk::BufferUsage::STORAGE_BUFFER
                    | vk::BufferUsage::TRANSFER_SRC
                    | vk::BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            vk::AllocationCreateInfo {
                memory_type_filter: vk::MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
            len as vk::DeviceSize,
        )?;

        Ok(GpuBuffer { buffer })
    }
}

impl<T> Buffer<T> for GpuBuffer<T>
where
    T: vk::BufferContents + Clone + Copy,
{
    fn len(&self) -> usize {
        self.buffer.len() as usize
    }

    fn sub(&self, range: Range<usize>) -> Result<GpuBuffer<T>> {
        if range.start >= range.end {
            return EngineError::ZeroSized.into_result();
        }

        if range.end > self.len() {
            return EngineError::OutOfBounds.into_result();
        }

        Ok(GpuBuffer {
            buffer: self.buffer.clone().slice(Range {
                start: range.start as u64,
                end: range.end as u64,
            }),
        })
    }

    fn bind(&self, binding: u32) -> BufferBinding {
        BufferBinding {
            write_descriptor_set: vk::WriteDescriptorSet::buffer(binding, self.buffer.clone()),
        }
    }

    fn get_vk_buffer(&self) -> &vk::Subbuffer<[T]> {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() -> Result<()> {
        let instance = Instance::new()?;
        assert!(GpuBuffer::<u32>::empty(&instance, 1024).is_ok());
        Ok(())
    }

    #[test]
    fn zero_sized_creation() -> Result<()> {
        let instance = Instance::new()?;
        assert!(GpuBuffer::<u32>::empty(&instance, 0).is_err());
        Ok(())
    }

    #[test]
    fn size() -> Result<()> {
        let instance = Instance::new()?;
        let buffer = GpuBuffer::<u32>::empty(&instance, 1024)?;
        assert_eq!(buffer.len(), 1024);
        Ok(())
    }
}
