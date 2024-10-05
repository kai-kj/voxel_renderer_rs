use super::errors::Result;
use super::vulkan as vk;
use std::ops::Range;

pub trait Buffer<T>
where
    T: vk::BufferContents + Clone + Copy,
    Self: Sized,
{
    fn len(&self) -> usize;
    fn sub(&self, range: Range<usize>) -> Result<Self>;
    fn bind(&self, binding: u32) -> BufferBinding;
    fn get_vk_buffer(&self) -> &vk::Subbuffer<[T]>;
}

pub struct BufferBinding {
    pub(super) write_descriptor_set: vk::WriteDescriptorSet,
}

pub mod cpu_buffer;
pub mod gpu_buffer;

pub use self::cpu_buffer::CpuBuffer;
pub use self::gpu_buffer::GpuBuffer;
