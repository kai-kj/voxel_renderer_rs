// use std::ops;
// use vulkano::buffer::BufferContents;

// use crate::preamble::*;

// pub struct BufferObject<'a, T>
// where
//     T: BufferContents + Clone + Copy,
// {
//     value: T,
//     instance: &'a Instance,
//     primary_buffer: GpuBuffer<'a, T>,
//     staging_buffer: CpuBuffer<'a, T>,
// }
//
// impl<'a, T> BufferObject<'a, T>
// where
//     T: BufferContents + Clone + Copy,
// {
//     pub fn new(instance: &'a Instance, value: T) -> AppResult<BufferObject<'a, T>> {
//         Ok(Self {
//             value,
//             instance,
//             primary_buffer: GpuBuffer::new(&instance, 1)?,
//             staging_buffer: CpuBuffer::new(&instance, 1)?,
//         })
//     }
//
//     pub fn upload(&self) -> AppResult<()> {
//         self.staging_buffer.write(vec![self.value])?;
//
//         TaskBuilder::new(self.instance)?
//             .copy_buffer(&self.staging_buffer, &self.primary_buffer)?
//             .build_submit_and_wait()?;
//
//         Ok(())
//     }
//
//     pub fn download(&mut self) -> AppResult<()> {
//         TaskBuilder::new(self.instance)?
//             .copy_buffer(&self.primary_buffer, &self.staging_buffer)?
//             .build_submit_and_wait()?;
//
//         self.value = self.staging_buffer.read()?[0];
//
//         Ok(())
//     }
// }
//
// impl<'a, T> ops::Deref for BufferObject<'a, T>
// where
//     T: BufferContents + Clone + Copy,
// {
//     type Target = T;
//     fn deref(&self) -> &T {
//         &self.value
//     }
// }
//
// impl<'a, T> ops::DerefMut for BufferObject<'a, T>
// where
//     T: BufferContents + Clone + Copy,
// {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.value
//     }
// }
//
// trait BufferObject<T>
// where
//     T: BufferContents + Clone + Copy,
// {
//     fn get_value(&self) -> T;
//
//     fn set_value(&self, value: T);
//
//     fn get_primary_buffer(&self) -> &Buffer<T, buffer_location::Gpu>;
//
//     fn get_staging_buffer(&self) -> &Buffer<T, buffer_location::Cpu>;
//
//     fn get_instance(&self) -> &Instance;
//
//     fn upload(&self) -> AppResult<()> {
//         self.get_staging_buffer().write(vec![self.get_value()])?;
//
//         TaskBuilder::new(self.get_instance())?
//             .copy_buffer(self.get_staging_buffer(), self.get_primary_buffer())?
//             .build_submit_and_wait()?;
//
//         Ok(())
//     }
//
//     fn download(&mut self) -> AppResult<()> {
//         TaskBuilder::new(self.get_instance())?
//             .copy_buffer(self.get_primary_buffer(), self.get_staging_buffer())?
//             .build_submit_and_wait()?;
//
//         self.set_value(self.get_staging_buffer().read()?[0]);
//
//         Ok(())
//     }
// }
