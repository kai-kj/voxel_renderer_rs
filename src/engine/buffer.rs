use super::errors::Error as EngineError;
use super::instance::Instance;
use super::vulkan as vk;

#[derive(Clone)]
pub struct Buffer<T> {
    length: usize,
    pub(super) buffer: vk::Subbuffer<[T]>,
}

impl<T> Buffer<T>
where
    T: vk::BufferContents + Clone + Copy,
{
    pub fn new(instance: &Instance, data: Vec<T>) -> Result<Buffer<T>, EngineError> {
        let length = data.len();
        if length == 0 {
            return EngineError::ZeroSized.into();
        }

        vk::Buffer::from_iter(
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
        )
        .and_then(|buffer| Ok(Buffer { length, buffer }))
        .map_err(|_| EngineError::VkBufferCreate)
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn total_size(&self) -> usize {
        self.buffer.size() as usize
    }

    pub fn item_size(&self) -> usize {
        size_of::<T>()
    }

    pub fn read(&self, start: usize, length: usize) -> Result<Vec<T>, EngineError> {
        if start + length > self.length() {
            return EngineError::OutOfBoundsRead.into();
        }

        self.buffer
            .read()
            .and_then(|slice| Ok(slice[start..start + length].to_vec()))
            .map_err(|_| EngineError::VkBufferRead)
    }

    pub fn read_all(&self) -> Result<Vec<T>, EngineError> {
        self.read(0, self.length())
    }

    pub fn write(&self, start: usize, data: Vec<T>) -> Result<(), EngineError> {
        if start + data.len() > self.length() {
            return EngineError::OutOfBoundsWrite.into();
        }

        self.buffer
            .write()
            .and_then(|mut slice| {
                slice[start..start + data.len()].copy_from_slice(data.as_slice());
                Ok(())
            })
            .map_err(|_| EngineError::VkBufferWrite)
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
        write!(
            f,
            "Buffer {{ length: {}, total size: {}, item size: {} }}",
            self.length(),
            self.total_size(),
            self.item_size()
        )
    }
}

pub struct BufferBinding {
    pub(super) write_descriptor_set: vk::WriteDescriptorSet,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() {
        let instance = Instance::new().unwrap();
        assert!(Buffer::new(&instance, (0..1024).collect()).is_ok());
    }

    #[test]
    fn zero_sized_creation() {
        let instance = Instance::new().unwrap();
        assert!(Buffer::new(&instance, (0..0).collect()).is_err());
    }

    #[test]
    fn size() {
        let instance = Instance::new().unwrap();
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();

        assert_eq!(buffer.length(), 4);
        assert_eq!(buffer.total_size(), 16);
        assert_eq!(buffer.item_size(), 4);
    }

    #[test]
    fn read_all() {
        let instance = Instance::new().unwrap();
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(buffer.read_all().unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn read_part() {
        let instance = Instance::new().unwrap();
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(buffer.read(1, 2).unwrap(), vec![2, 3]);
    }

    #[test]
    fn out_of_bounds_read() {
        let instance = Instance::new().unwrap();
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();
        assert!(buffer.read(0, 4).is_ok());
        assert!(buffer.read(0, 5).is_err());
        assert!(buffer.read(1, 4).is_err());
        assert!(buffer.read(4, 4).is_err());
    }

    #[test]
    fn write() {
        let instance = Instance::new().unwrap();

        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(buffer.read_all().unwrap(), vec![1, 2, 3, 4]);

        buffer.write(1, vec![5, 6]).unwrap();
        assert_eq!(buffer.read_all().unwrap(), vec![1, 5, 6, 4]);
    }

    #[test]
    fn out_of_bounds_write() {
        let instance = Instance::new().unwrap();
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();
        assert!(buffer.write(0, vec![1, 2, 3, 4]).is_ok());
        assert!(buffer.write(0, vec![1, 2, 3, 4, 5]).is_err());
        assert!(buffer.write(1, vec![1, 2, 3, 4]).is_err());
        assert!(buffer.write(4, vec![1, 2, 3, 4]).is_err());
    }

    #[test]
    fn clone() {
        let instance = Instance::new().unwrap();
        let buffer_a = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();
        let buffer_b = buffer_a.clone();

        assert_eq!(buffer_a.read_all().unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read_all().unwrap(), vec![1, 2, 3, 4]);

        buffer_a.write(1, vec![5, 6]).unwrap();

        assert_eq!(buffer_a.read_all().unwrap(), vec![1, 5, 6, 4]);
        assert_eq!(buffer_b.read_all().unwrap(), vec![1, 5, 6, 4]);
    }
}
