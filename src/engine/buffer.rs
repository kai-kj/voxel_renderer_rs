use super::errors::{EngineError, Result};
use super::instance::Instance;
use super::vulkan as vk;
use std::cmp::min;
use std::ops::Range;

#[derive(Clone)]
pub struct Buffer<T> {
    pub(super) buffer: vk::Subbuffer<[T]>,
}

impl<T> Buffer<T>
where
    T: vk::BufferContents + Clone + Copy,
{
    pub fn new(instance: &Instance, data: Vec<T>) -> Result<Buffer<T>> {
        if data.len() == 0 {
            return EngineError::ZeroSized.into_result();
        }

        let buffer = vk::Buffer::from_iter(
            instance.memory_allocator.clone(),
            vk::BufferCreateInfo {
                usage: vk::BufferUsage::STORAGE_BUFFER
                    | vk::BufferUsage::TRANSFER_SRC
                    | vk::BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            vk::AllocationCreateInfo {
                memory_type_filter: vk::MemoryTypeFilter::PREFER_DEVICE
                    | vk::MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data,
        )?;

        Ok(Buffer { buffer })
    }

    pub fn len(&self) -> usize {
        self.buffer.len() as usize
    }
    pub fn sub(&self, range: Range<usize>) -> Result<Buffer<T>> {
        if range.start >= range.end {
            return EngineError::ZeroSized.into_result();
        }

        if range.end > self.len() {
            return EngineError::OutOfBounds.into_result();
        }

        Ok(Buffer {
            buffer: self.buffer.clone().slice(Range {
                start: range.start as u64,
                end: range.end as u64,
            }),
        })
    }

    pub fn read(&self) -> Result<Vec<T>> {
        Ok(self.buffer.read()?.to_vec())
    }

    pub fn write(&self, data: Vec<T>) -> Result<()> {
        self.buffer.write()?[..min(data.len(), self.len())].copy_from_slice(data.as_slice());
        Ok(())
    }

    pub fn bind(&self, binding: u32) -> BufferBinding {
        BufferBinding {
            write_descriptor_set: vk::WriteDescriptorSet::buffer(binding, self.buffer.clone()),
        }
    }
}

impl<T> std::fmt::Debug for Buffer<T>
where
    T: vk::BufferContents + Clone + Copy,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Buffer {{ len: {} }}", self.len())
    }
}

pub struct BufferBinding {
    pub(super) write_descriptor_set: vk::WriteDescriptorSet,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() -> Result<()> {
        let instance = Instance::new()?;
        assert!(Buffer::new(&instance, (0..1024).collect()).is_ok());
        Ok(())
    }

    #[test]
    fn zero_sized_creation() -> Result<()> {
        let instance = Instance::new()?;
        assert!(Buffer::new(&instance, (0..0).collect()).is_err());
        Ok(())
    }

    #[test]
    fn size() -> Result<()> {
        let instance = Instance::new()?;
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4])?;
        assert_eq!(buffer.len(), 4);
        Ok(())
    }

    #[test]
    fn read() -> Result<()> {
        let instance = Instance::new()?;
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4])?;
        assert_eq!(buffer.read()?, vec![1, 2, 3, 4]);
        Ok(())
    }

    #[test]
    fn write() -> Result<()> {
        let instance = Instance::new()?;
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4])?;

        buffer.write(vec![5, 6, 7, 8])?;
        assert_eq!(buffer.read()?, vec![5, 6, 7, 8]);

        buffer.write(vec![9, 10])?;
        assert_eq!(buffer.read()?, vec![9, 10, 7, 8]);

        Ok(())
    }

    #[test]
    fn subregion_read() -> Result<()> {
        let instance = Instance::new()?;
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4])?;

        assert_eq!(buffer.sub(0..4)?.read()?, vec![1, 2, 3, 4]);
        assert_eq!(buffer.sub(1..3)?.read()?, vec![2, 3]);

        Ok(())
    }

    #[test]
    fn subregion_write() -> Result<()> {
        let instance = Instance::new()?;
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4])?;
        let sub_buffer = buffer.sub(1..3)?;

        sub_buffer.write(vec![5, 6])?;
        assert_eq!(sub_buffer.read()?, vec![5, 6]);
        assert_eq!(buffer.read()?, vec![1, 5, 6, 4]);

        buffer.write(vec![7, 8, 9, 10])?;
        assert_eq!(sub_buffer.read()?, vec![8, 9]);
        assert_eq!(buffer.read()?, vec![7, 8, 9, 10]);

        Ok(())
    }

    #[test]
    fn out_of_bounds() -> Result<()> {
        let instance = Instance::new()?;
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4])?;
        assert!(buffer.sub(0..4).is_ok());
        assert!(buffer.sub(0..0).is_err());
        assert!(buffer.sub(0..5).is_err());
        assert!(buffer.sub(4..8).is_err());
        Ok(())
    }

    #[test]
    fn clone() -> Result<()> {
        let instance = Instance::new()?;
        let buffer_a = Buffer::new(&instance, vec![1, 2, 3, 4])?;
        let buffer_b = buffer_a.clone();

        assert_eq!(buffer_a.read()?, vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read()?, vec![1, 2, 3, 4]);

        buffer_a.sub(1..3)?.write(vec![5, 6])?;

        assert_eq!(buffer_a.read()?, vec![1, 5, 6, 4]);
        assert_eq!(buffer_b.read()?, vec![1, 5, 6, 4]);

        Ok(())
    }
}
