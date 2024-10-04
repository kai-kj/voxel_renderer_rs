use super::errors::Error as EngineError;
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
    pub fn wait(self) -> Result<(), EngineError> {
        self.future
            .then_signal_fence_and_flush()
            .and_then(|future| future.wait(None))
            .and_then(|_| Ok(()))
            .map_err(|_| EngineError::VkFutureWait)
    }
}

pub struct Task<'a> {
    instance: &'a Instance,
    command_buffer: Arc<vk::PrimaryAutoCommandBuffer<Arc<vk::StandardCommandBufferAllocator>>>,
}

impl<'a> Task<'a> {
    pub fn submit(&self) -> Result<TaskFuture, EngineError> {
        vk::sync::now(self.instance.device.clone())
            .then_execute(self.instance.queue.clone(), self.command_buffer.clone())
            .and_then(|future| {
                Ok(TaskFuture {
                    future: future.boxed(),
                })
            })
            .map_err(|_| EngineError::VkCommandBufferSubmit)
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
    pub fn new(instance: &'a Instance) -> Result<Self, EngineError> {
        vk::AutoCommandBufferBuilder::primary(
            &instance.command_buffer_allocator,
            instance.queue_family_index,
            vk::CommandBufferUsage::MultipleSubmit,
        )
        .and_then(|builder| Ok(Self { instance, builder }))
        .map_err(|_| EngineError::VkCommandBufferBuilderCreate)
    }

    pub fn build(self) -> Result<Task<'a>, EngineError> {
        self.builder
            .build()
            .and_then(|command_buffer| {
                Ok(Task {
                    instance: self.instance,
                    command_buffer,
                })
            })
            .map_err(|_| EngineError::VkCommandBufferCreate)
    }

    pub fn copy_buffer<T>(
        mut self,
        src: &Buffer<T>,
        dst: &Buffer<T>,
    ) -> Result<TaskBuilder<'a>, EngineError>
    where
        T: vk::BufferContents + Clone + Copy,
    {
        match self.builder.copy_buffer(vk::CopyBufferInfo::buffers(
            src.buffer.clone(),
            dst.buffer.clone(),
        )) {
            Ok(_) => Ok(self),
            _ => EngineError::VkCommandSubmit.into(),
        }
    }

    pub fn run_program(
        mut self,
        program: &Program,
        wg_size: (u32, u32, u32),
        bindings: Vec<BufferBinding>,
    ) -> Result<TaskBuilder<'a>, EngineError> {
        let descriptor_set_allocator = vk::StandardDescriptorSetAllocator::new(
            self.instance.device.clone(),
            Default::default(),
        );

        let descriptor_set_layout = match program.compute_pipeline.layout().set_layouts().get(0) {
            Some(layout) => layout,
            _ => return EngineError::VkDescriptorSetLayoutCreate.into(),
        };

        let descriptor_writes: Vec<vk::WriteDescriptorSet> = bindings
            .iter()
            .map(|b| b.write_descriptor_set.clone())
            .collect();

        let descriptor_set = match vk::PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            descriptor_set_layout.clone(),
            descriptor_writes,
            [],
        ) {
            Ok(descriptor_set) => descriptor_set,
            _ => return EngineError::VkDescriptorSetCreate.into(),
        };

        match self
            .builder
            .bind_pipeline_compute(program.compute_pipeline.clone())
            .and_then(|builder| {
                builder.bind_descriptor_sets(
                    vk::PipelineBindPoint::Compute,
                    program.compute_pipeline.layout().clone(),
                    0,
                    descriptor_set,
                )
            })
            .and_then(|builder| builder.dispatch(wg_size.into()))
        {
            Ok(_) => Ok(self),
            _ => EngineError::VkCommandSubmit.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_buffer() {
        let instance = Instance::new().unwrap();
        let buffer_a = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();
        let buffer_b = Buffer::new(&instance, vec![0, 0, 0, 0]).unwrap();

        TaskBuilder::new(&instance)
            .unwrap()
            .copy_buffer(&buffer_a, &buffer_b)
            .unwrap()
            .build()
            .unwrap()
            .submit()
            .unwrap()
            .wait()
            .unwrap();

        assert_eq!(buffer_a.read_all().unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read_all().unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn copy_buffer_twice() {
        let instance = Instance::new().unwrap();

        let buffer_a = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();
        let buffer_b = Buffer::new(&instance, vec![0, 0, 0, 0]).unwrap();

        let task = TaskBuilder::new(&instance)
            .unwrap()
            .copy_buffer(&buffer_a, &buffer_b)
            .unwrap()
            .build()
            .unwrap();

        task.submit().unwrap().wait().unwrap();

        assert_eq!(buffer_a.read_all().unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read_all().unwrap(), vec![1, 2, 3, 4]);

        buffer_a.write(0, vec![5, 6, 7, 8]).unwrap();

        task.submit().unwrap().wait().unwrap();

        assert_eq!(buffer_a.read_all().unwrap(), vec![5, 6, 7, 8]);
        assert_eq!(buffer_b.read_all().unwrap(), vec![5, 6, 7, 8]);
    }

    #[test]
    fn run_program() {
        let code = r"
            #version 460
            layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
            layout(binding = 0) buffer Data { uint data[]; };
            void main() { data[gl_GlobalInvocationID.x] *= 2; }
        ";

        let instance = Instance::new().unwrap();
        let program = Program::new(&instance, &code, "test.glsl", "main").unwrap();
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();

        TaskBuilder::new(&instance)
            .unwrap()
            .run_program(&program, (4, 1, 1), vec![buffer.bind(0)])
            .unwrap()
            .build()
            .unwrap()
            .submit()
            .unwrap()
            .wait()
            .unwrap();

        assert_eq!(buffer.read_all().unwrap(), vec![2, 4, 6, 8]);
    }

    #[test]
    fn run_program_twice() {
        let code = r"
            #version 460
            layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
            layout(binding = 0) buffer Data { uint data[]; };
            void main() { data[gl_GlobalInvocationID.x] *= 2; }
        ";

        let instance = Instance::new().unwrap();
        let program = Program::new(&instance, &code, "test.glsl", "main").unwrap();
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();

        let task = TaskBuilder::new(&instance)
            .unwrap()
            .run_program(&program, (4, 1, 1), vec![buffer.bind(0)])
            .unwrap()
            .build()
            .unwrap();

        task.submit().unwrap().wait().unwrap();
        assert_eq!(buffer.read_all().unwrap(), vec![2, 4, 6, 8]);

        task.submit().unwrap().wait().unwrap();
        assert_eq!(buffer.read_all().unwrap(), vec![4, 8, 12, 16]);
    }

    #[test]
    fn run_program_wrong_binding() {
        let code = r"
            #version 460
            layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
            layout(binding = 0) buffer Data { uint data[]; };
            void main() { data[gl_GlobalInvocationID.x] *= 2; }
        ";

        let instance = Instance::new().unwrap();
        let program = Program::new(&instance, &code, "test.glsl", "main").unwrap();
        let buffer = Buffer::new(&instance, vec![1, 2, 3, 4]).unwrap();

        assert!(TaskBuilder::new(&instance)
            .unwrap()
            .run_program(&program, (4, 1, 1), vec![buffer.bind(1)])
            .is_err());
    }
}
