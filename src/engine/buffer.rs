use super::*;
use std::{cmp::min, error::Error, fmt::Debug, marker::PhantomData, ops::Range};
use thiserror::Error;
pub use vk::BufferContents;

#[derive(Error, Debug)]
pub enum BufferError {
    #[error("requested buffer range is empty")]
    RangeIsEmpty,
    #[error("requested buffer range is out of bounds")]
    RangeIsOutOfBounds,
    #[error("requested buffer length is out of bounds")]
    LengthIsZero,
    #[error("failed to read from vulkan buffer")]
    VulkanBufferReadFailed,
    #[error("failed to write to vulkan buffer")]
    VulkanBufferWriteFailed,
    #[error("failed to create vulkan buffer")]
    VulkanBufferCreationFailed,
}

pub mod buffer_location {
    #[derive(Clone, Debug)]
    pub struct Cpu;
    #[derive(Clone, Debug)]
    pub struct Gpu;
}

pub type CpuBuffer<T> = Buffer<T, buffer_location::Cpu>;
pub type GpuBuffer<T> = Buffer<T, buffer_location::Gpu>;

#[derive(Clone)]
pub struct Buffer<T, Location>
where
    T: BufferContents + Clone + Copy,
    Self: Sized,
{
    buffer: vk::Subbuffer<[T]>,
    location: PhantomData<Location>,
}

impl<'a, T, Location> Buffer<T, Location>
where
    T: BufferContents + Clone + Copy,
    Self: Sized,
{
    pub fn len(&self) -> usize {
        self.buffer.len() as usize
    }

    pub fn sub(&self, range: Range<usize>) -> Result<Buffer<T, Location>, BufferError> {
        if range.start >= range.end {
            return Err(BufferError::RangeIsEmpty);
        }

        if range.end > self.len() {
            return Err(BufferError::RangeIsOutOfBounds);
        }

        Ok(Buffer {
            buffer: self.buffer.clone().slice(Range {
                start: range.start as u64,
                end: range.end as u64,
            }),
            location: PhantomData,
        })
    }

    pub fn bind(&self, binding: u32) -> BufferBinding {
        BufferBinding {
            write_descriptor_set: vk::WriteDescriptorSet::buffer(binding, self.buffer.clone()),
        }
    }

    pub(super) fn get_vk_buffer(&self) -> &vk::Subbuffer<[T]> {
        &self.buffer
    }
}

impl<'a, T> Buffer<T, buffer_location::Cpu>
where
    T: BufferContents + Clone + Copy,
    Self: Sized,
{
    pub fn new(
        instance: &'a Instance,
        len: usize,
    ) -> Result<Buffer<T, buffer_location::Cpu>, BufferError> {
        if len == 0 {
            return Err(BufferError::LengthIsZero);
        }

        Ok(Buffer {
            buffer: create_vk_buffer(instance, default_buffer_usage(), default_cpu_memory(), len)?,
            location: PhantomData,
        })
    }

    pub fn from_vec(
        instance: &'a Instance,
        data: Vec<T>,
    ) -> Result<Buffer<T, buffer_location::Cpu>, BufferError> {
        let buffer = Self::new(instance, data.len())?;
        buffer.write(data)?;
        Ok(buffer)
    }

    pub fn read(&self) -> Result<Vec<T>, BufferError> {
        Ok(self
            .buffer
            .read()
            .map_err(|_| BufferError::VulkanBufferReadFailed)?
            .to_vec())
    }

    pub fn write(&self, data: Vec<T>) -> Result<(), BufferError> {
        self.buffer
            .write()
            .map_err(|_| BufferError::VulkanBufferWriteFailed)?[..min(data.len(), self.len())]
            .copy_from_slice(data.as_slice());
        Ok(())
    }
}

impl<'a, T> Buffer<T, buffer_location::Gpu>
where
    T: BufferContents + Clone + Copy,
    Self: Sized,
{
    pub fn new(
        instance: &'a Instance,
        len: usize,
    ) -> Result<Buffer<T, buffer_location::Gpu>, BufferError> {
        if len == 0 {
            return Err(BufferError::LengthIsZero);
        }

        Ok(Buffer {
            buffer: create_vk_buffer(instance, default_buffer_usage(), default_gpu_memory(), len)?,
            location: PhantomData,
        })
    }
}

pub struct BufferBinding {
    pub(super) write_descriptor_set: vk::WriteDescriptorSet,
}

fn create_vk_buffer<T>(
    instance: &Instance,
    usage: vk::BufferUsage,
    memory_type_filter: vk::MemoryTypeFilter,
    len: usize,
) -> Result<vk::Subbuffer<[T]>, BufferError>
where
    T: BufferContents + Clone + Copy,
{
    Ok(vk::Buffer::new_slice(
        instance.memory_allocator.clone(),
        vk::BufferCreateInfo {
            usage,
            ..Default::default()
        },
        vk::AllocationCreateInfo {
            memory_type_filter,
            ..Default::default()
        },
        len as vk::DeviceSize,
    )
    .map_err(|_| BufferError::VulkanBufferCreationFailed)?)
}

fn default_buffer_usage() -> vk::BufferUsage {
    vk::BufferUsage::STORAGE_BUFFER | vk::BufferUsage::TRANSFER_SRC | vk::BufferUsage::TRANSFER_DST
}

fn default_cpu_memory() -> vk::MemoryTypeFilter {
    vk::MemoryTypeFilter::PREFER_HOST | vk::MemoryTypeFilter::HOST_SEQUENTIAL_WRITE
}

fn default_gpu_memory() -> vk::MemoryTypeFilter {
    vk::MemoryTypeFilter::PREFER_DEVICE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() {
        let instance = Instance::new().unwrap();
        assert!(CpuBuffer::from_vec(&instance, (0..1024).collect()).is_ok());
    }

    #[test]
    fn zero_sized_creation() {
        let instance = Instance::new().unwrap();
        assert!(CpuBuffer::from_vec(&instance, (0..0).collect()).is_err());
    }

    #[test]
    fn size() {
        let instance = Instance::new().unwrap();
        let buffer = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(buffer.len(), 4);
    }

    #[test]
    fn read() {
        let instance = Instance::new().unwrap();
        let buffer = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(buffer.read().unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn write() {
        let instance = Instance::new().unwrap();
        let buffer = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();

        buffer.write(vec![5, 6, 7, 8]).unwrap();
        assert_eq!(buffer.read().unwrap(), vec![5, 6, 7, 8]);

        buffer.write(vec![9, 10]).unwrap();
        assert_eq!(buffer.read().unwrap(), vec![9, 10, 7, 8]);
    }

    #[test]
    fn subregion_read() {
        let instance = Instance::new().unwrap();
        let buffer = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();

        assert_eq!(buffer.sub(0..4).unwrap().read().unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(buffer.sub(1..3).unwrap().read().unwrap(), vec![2, 3]);
    }

    #[test]
    fn subregion_write() {
        let instance = Instance::new().unwrap();
        let buffer = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();
        let sub_buffer = buffer.sub(1..3).unwrap();

        sub_buffer.write(vec![5, 6]).unwrap();
        assert_eq!(sub_buffer.read().unwrap(), vec![5, 6]);
        assert_eq!(buffer.read().unwrap(), vec![1, 5, 6, 4]);

        buffer.write(vec![7, 8, 9, 10]).unwrap();
        assert_eq!(sub_buffer.read().unwrap(), vec![8, 9]);
        assert_eq!(buffer.read().unwrap(), vec![7, 8, 9, 10]);
    }

    #[test]
    fn out_of_bounds() {
        let instance = Instance::new().unwrap();
        let buffer = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();
        assert!(buffer.sub(0..4).is_ok());
        assert!(buffer.sub(0..0).is_err());
        assert!(buffer.sub(0..5).is_err());
        assert!(buffer.sub(4..8).is_err());
    }

    #[test]
    fn clone() {
        let instance = Instance::new().unwrap();
        let buffer_a = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();
        let buffer_b = buffer_a.clone();

        assert_eq!(buffer_a.read().unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read().unwrap(), vec![1, 2, 3, 4]);

        buffer_a.sub(1..3).unwrap().write(vec![5, 6]).unwrap();

        assert_eq!(buffer_a.read().unwrap(), vec![1, 5, 6, 4]);
        assert_eq!(buffer_b.read().unwrap(), vec![1, 5, 6, 4]);
    }
}
