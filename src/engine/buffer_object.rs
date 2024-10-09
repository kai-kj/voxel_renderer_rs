// use crate::engine::*;
// use crate::errors::{AppError, AppResult};
// use std::marker::PhantomData;
// use std::ops::{Deref, DerefMut};
//
// pub mod buffer_object_state {
//     pub struct Synced;
//     pub struct CpuChange;
//     pub struct GpuChange;
// }
//
// pub struct BufferObject<'a, T, State>
// where
//     T: BufferContents + Clone + Copy,
// {
//     value: T,
//     instance: &'a Instance,
//     primary_buffer: GpuBuffer<'a, T>,
//     staging_buffer: CpuBuffer<'a, T>,
//     state: PhantomData<State>,
// }
//
// impl<'a, T> BufferObject<'a, T, buffer_object_state::Synced>
// where
//     T: BufferContents + Clone + Copy,
// {
//     pub fn allow_cpu_access(self) -> BufferObject<'a, T, buffer_object_state::CpuChange> {
//         BufferObject {
//             value: self.value,
//             instance: self.instance,
//             primary_buffer: self.primary_buffer,
//             staging_buffer: self.staging_buffer,
//             state: PhantomData,
//         }
//     }
//
//     pub fn allow_gpu_access(self) -> BufferObject<'a, T, buffer_object_state::GpuChange> {
//         BufferObject {
//             value: self.value,
//             instance: self.instance,
//             primary_buffer: self.primary_buffer,
//             staging_buffer: self.staging_buffer,
//             state: PhantomData,
//         }
//     }
// }
//
// impl<'a, T> BufferObject<'a, T, buffer_object_state::CpuChange>
// where
//     T: BufferContents + Clone + Copy,
// {
//     pub fn new(
//         instance: &'a Instance,
//         value: T,
//     ) -> AppResult<BufferObject<T, buffer_object_state::CpuChange>> {
//         let primary_buffer = GpuBuffer::<T>::new(instance, 1)?;
//         let staging_buffer = CpuBuffer::<T>::new(instance, 1)?;
//         Ok(Self {
//             value,
//             instance,
//             primary_buffer,
//             staging_buffer,
//             state: PhantomData,
//         })
//     }
//
//     pub fn sync(self) -> AppResult<BufferObject<'a, T, buffer_object_state::Synced>> {
//         self.staging_buffer.write(vec![self.value])?;
//
//         TaskBuilder::new(self.instance)?
//             .copy_buffer(&self.staging_buffer, &self.primary_buffer)?
//             .build_submit_and_wait()?;
//
//         Ok(BufferObject {
//             value: self.value,
//             instance: self.instance,
//             primary_buffer: self.primary_buffer,
//             staging_buffer: self.staging_buffer,
//             state: PhantomData,
//         })
//     }
// }
//
// impl<'a, T> BufferObject<'a, T, buffer_object_state::GpuChange>
// where
//     T: BufferContents + Clone + Copy,
// {
//     pub fn buffer(&self) -> &GpuBuffer<'a, T> {
//         &self.primary_buffer
//     }
//
//     pub fn sync(mut self) -> AppResult<BufferObject<'a, T, buffer_object_state::Synced>> {
//         TaskBuilder::new(self.instance)?
//             .copy_buffer(&self.primary_buffer, &self.staging_buffer)?
//             .build_submit_and_wait()?;
//
//         self.value = self.staging_buffer.read()?[0];
//
//         Ok(BufferObject {
//             value: self.value,
//             instance: self.instance,
//             primary_buffer: self.primary_buffer,
//             staging_buffer: self.staging_buffer,
//             state: PhantomData,
//         })
//     }
// }
//
// impl<'a, T> Deref for BufferObject<'a, T, buffer_object_state::CpuChange>
// where
//     T: BufferContents + Clone + Copy,
// {
//     type Target = T;
//     fn deref(&self) -> &T {
//         &self.value
//     }
// }
//
// impl<'a, T> DerefMut for BufferObject<'a, T, buffer_object_state::CpuChange>
// where
//     T: BufferContents + Clone + Copy,
// {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.value
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn sync() -> AppResult<()> {
//         let instance = Instance::new()?;
//         let data_a = BufferObject::new(&instance, 1)?.sync()?.allow_cpu_access();
//         let data_b = BufferObject::new(&instance, 2)?.sync()?.allow_cpu_access();
//
//         assert_eq!(*data_a, 1);
//         assert_eq!(*data_b, 2);
//
//         let data_a = data_a.sync()?.allow_gpu_access();
//         let data_b = data_b.sync()?.allow_gpu_access();
//
//         TaskBuilder::new(&instance)?
//             .copy_buffer(data_a.buffer(), data_b.buffer())?
//             .build_submit_and_wait()?;
//
//         let mut data_a = data_a.sync()?.allow_cpu_access();
//         let data_b = data_b.sync()?.allow_cpu_access();
//
//         assert_eq!(*data_a, 1);
//         assert_eq!(*data_b, 1);
//
//         *data_a = 3;
//
//         let data_a = data_a.sync()?.allow_gpu_access();
//         let data_b = data_b.sync()?.allow_gpu_access();
//
//         TaskBuilder::new(&instance)?
//             .copy_buffer(data_a.buffer(), data_b.buffer())?
//             .build_submit_and_wait()?;
//
//         let data_a = data_a.sync()?.allow_cpu_access();
//         let data_b = data_b.sync()?.allow_cpu_access();
//
//         assert_eq!(*data_a, 3);
//         assert_eq!(*data_b, 3);
//
//         Ok(())
//     }
// }
