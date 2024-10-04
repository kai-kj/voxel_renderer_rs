use super::errors::Result;
use super::instance::Instance;
use super::program::Program;
use super::vulkan as vk;

use crate::engine::buffer::Buffer;
use crate::engine::buffer::BufferBinding;
use std::future::IntoFuture;
use std::sync::Arc;
use vulkano::pipeline::Pipeline;
use vulkano::sync::GpuFuture;

pub struct TaskFuture {
    future: Box<dyn GpuFuture>,
}

impl TaskFuture {
    pub fn wait(self) -> Result<()> {
        self.future.then_signal_fence_and_flush()?.wait(None)?;
        Ok(())
    }
}

pub struct Task<'a> {
    instance: &'a Instance,
    command_buffer: Arc<vk::PrimaryAutoCommandBuffer<Arc<vk::StandardCommandBufferAllocator>>>,
}

impl<'a> Task<'a> {
    pub fn submit(&self) -> Result<TaskFuture> {
        let future = vk::sync::now(self.instance.device.clone())
            .then_execute(self.instance.queue.clone(), self.command_buffer.clone())?
            .boxed();
        Ok(TaskFuture { future })
    }
}

pub struct TaskBuilder<'a> {
    instance: &'a Instance,
    builder: vk::AutoCommandBufferBuilder<
        vk::PrimaryAutoCommandBuffer<Arc<vk::StandardCommandBufferAllocator>>,
        Arc<vk::StandardCommandBufferAllocator>,
    >,
}

impl<'a> TaskBuilder<'a> {
    pub fn new(instance: &'a Instance) -> Result<Self> {
        let builder = vk::AutoCommandBufferBuilder::primary(
            &instance.command_buffer_allocator,
            instance.queue_family_index,
            vk::CommandBufferUsage::MultipleSubmit,
        )?;
        Ok(Self { instance, builder })
    }

    pub fn build(self) -> Result<Task<'a>> {
        let command_buffer = self.builder.build()?;
        Ok(Task {
            instance: self.instance,
            command_buffer,
        })
    }

    pub fn copy_buffer<T>(mut self, src: &Buffer<T>, dst: &Buffer<T>) -> Result<TaskBuilder<'a>>
    where
        T: vk::BufferContents + Clone + Copy,
    {
        self.builder.copy_buffer(vk::CopyBufferInfo::buffers(
            src.buffer.clone(),
            dst.buffer.clone(),
        ))?;
        Ok(self)
    }

    pub fn run_program(
        mut self,
        program: &Program,
        wg_size: (u32, u32, u32),
        bindings: Vec<BufferBinding>,
    ) -> Result<TaskBuilder<'a>> {
        let descriptor_set_allocator = vk::StandardDescriptorSetAllocator::new(
            self.instance.device.clone(),
            Default::default(),
        );

        let descriptor_set_layout = program
            .compute_pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();

        let descriptor_writes: Vec<vk::WriteDescriptorSet> = bindings
            .iter()
            .map(|b| b.write_descriptor_set.clone())
            .collect();

        let descriptor_set = vk::PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            descriptor_set_layout.clone(),
            descriptor_writes,
            [],
        )?;

        self.builder
            .bind_pipeline_compute(program.compute_pipeline.clone())?
            .bind_descriptor_sets(
                vk::PipelineBindPoint::Compute,
                program.compute_pipeline.layout().clone(),
                0,
                descriptor_set,
            )?
            .dispatch(wg_size.into())?;

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_buffer() -> Result<()> {
        let instance = Instance::new()?;
        let buffer_a = Buffer::new(&instance, vec![1, 2, 3, 4])?;
        let buffer_b = Buffer::new(&instance, vec![0, 0, 0, 0])?;

        TaskBuilder::new(&instance)?
            .copy_buffer(&buffer_a, &buffer_b)?
            .build()?
            .submit()?
            .wait()?;

        assert_eq!(buffer_a.read()?, vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read()?, vec![1, 2, 3, 4]);

        Ok(())
    }

    #[test]
    fn copy_buffer_twice() -> Result<()> {
        let instance = Instance::new()?;

        let buffer_a = Buffer::new(&instance, vec![1, 2, 3, 4])?;
        let buffer_b = Buffer::new(&instance, vec![0, 0, 0, 0])?;

        let task = TaskBuilder::new(&instance)?
            .copy_buffer(&buffer_a, &buffer_b)?
            .build()?;

        task.submit()?.wait()?;

        assert_eq!(buffer_a.read()?, vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read()?, vec![1, 2, 3, 4]);

        buffer_a.write(vec![5, 6, 7, 8]).unwrap();

        task.submit()?.wait()?;

        assert_eq!(buffer_a.read()?, vec![5, 6, 7, 8]);
        assert_eq!(buffer_b.read()?, vec![5, 6, 7, 8]);

        Ok(())
    }

    #[test]
    fn copy_buffer_sub() -> Result<()> {
        let instance = Instance::new()?;

        let buffer_a = Buffer::new(&instance, vec![1, 2, 3, 4])?;
        let buffer_b = Buffer::new(&instance, vec![0, 0, 0, 0])?;

        let task = TaskBuilder::new(&instance)?
            .copy_buffer(&buffer_a.sub(0..2)?, &buffer_b.sub(1..3)?)?
            .build()?;

        task.submit()?.wait()?;

        assert_eq!(buffer_a.read()?, vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read()?, vec![0, 1, 2, 0]);

        Ok(())
    }

    #[test]
    fn run_program() -> Result<()> {
        let code = r"
            #version 460
            layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
            layout(binding = 0) buffer Data { uint data[]; };
            void main() { data[gl_GlobalInvocationID.x] *= 2; }
        ";

        let instance = Instance::new()?;
        let program = Program::new(&instance, &code, "test.glsl", "main")?;
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4])?;

        TaskBuilder::new(&instance)?
            .run_program(&program, (4, 1, 1), vec![buffer.bind(0)])?
            .build()?
            .submit()?
            .wait()?;

        assert_eq!(buffer.read()?, vec![2, 4, 6, 8]);

        Ok(())
    }

    #[test]
    fn run_program_twice() -> Result<()> {
        let code = r"
            #version 460
            layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
            layout(binding = 0) buffer Data { uint data[]; };
            void main() { data[gl_GlobalInvocationID.x] *= 2; }
        ";

        let instance = Instance::new()?;
        let program = Program::new(&instance, &code, "test.glsl", "main")?;
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4])?;

        let task = TaskBuilder::new(&instance)?
            .run_program(&program, (4, 1, 1), vec![buffer.bind(0)])?
            .build()?;

        task.submit()?.wait()?;
        assert_eq!(buffer.read()?, vec![2, 4, 6, 8]);

        task.submit()?.wait()?;
        assert_eq!(buffer.read()?, vec![4, 8, 12, 16]);

        Ok(())
    }

    #[test]
    fn run_program_wrong_binding() -> Result<()> {
        let code = r"
            #version 460
            layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
            layout(binding = 0) buffer Data { uint data[]; };
            void main() { data[gl_GlobalInvocationID.x] *= 2; }
        ";

        let instance = Instance::new()?;
        let program = Program::new(&instance, &code, "test.glsl", "main")?;
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4])?;

        assert!(TaskBuilder::new(&instance)?
            .run_program(&program, (4, 1, 1), vec![buffer.bind(1)])
            .is_err());

        Ok(())
    }
}
