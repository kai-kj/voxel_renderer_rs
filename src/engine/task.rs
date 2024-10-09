use super::*;
use std::future::IntoFuture;
use std::sync::Arc;
use thiserror::Error;
use vulkano::pipeline::Pipeline;
use vulkano::sync::GpuFuture;

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("failed to fence and flush vulkan future")]
    VulkanFutureFenceFlushFailed,
    #[error("failed to wait for future")]
    WaitFailed,
    #[error("failed to submit task")]
    TaskSubmissionFailed,
    #[error("failed to create vulkan command buffer builder")]
    VulkanCommandBufferBuilderCreationFailed,
    #[error("failed to build vulkan command buffer")]
    VulkanCommandBufferBuildFailed,
    #[error("failed to record vulkan copy buffer command")]
    VulkanCopyBufferFailed,
    #[error("failed to create vulkan descriptor set")]
    VulkanDescriptorSetCreationFailed,
    #[error("failed to record vulkan descriptor bind command")]
    VulkanDescriptorSetBindingFailed,
    #[error("failed to record vulkan pipeline bind command")]
    VulkanPipelineBindingFailed,
    #[error("failed to dispatch vulkan command buffer")]
    VulkanDispatchFailed,
}

pub struct TaskFuture {
    future: Box<dyn GpuFuture>,
}

impl TaskFuture {
    pub fn wait(self) -> Result<(), TaskError> {
        self.future
            .then_signal_fence_and_flush()
            .map_err(|_| TaskError::VulkanFutureFenceFlushFailed)?
            .wait(None)
            .map_err(|_| TaskError::WaitFailed)?;
        Ok(())
    }
}

pub struct Task {
    device: Arc<vk::Device>,
    queue: Arc<vk::Queue>,
    command_buffer: Arc<vk::PrimaryAutoCommandBuffer<Arc<vk::StandardCommandBufferAllocator>>>,
}

impl Task {
    pub fn submit(&self) -> Result<TaskFuture, TaskError> {
        let future = vk::sync::now(self.device.clone())
            .then_execute(self.queue.clone(), self.command_buffer.clone())
            .map_err(|_| TaskError::TaskSubmissionFailed)?
            .boxed();
        Ok(TaskFuture { future })
    }

    pub fn submit_and_wait(&self) -> Result<(), TaskError> {
        self.submit()?.wait()?;
        Ok(())
    }
}

pub struct TaskBuilder {
    device: Arc<vk::Device>,
    queue: Arc<vk::Queue>,
    builder: vk::AutoCommandBufferBuilder<
        vk::PrimaryAutoCommandBuffer<Arc<vk::StandardCommandBufferAllocator>>,
        Arc<vk::StandardCommandBufferAllocator>,
    >,
}

impl TaskBuilder {
    pub fn new(instance: &Instance) -> Result<Self, TaskError> {
        Ok(Self {
            device: instance.device.clone(),
            queue: instance.queue.clone(),
            builder: vk::AutoCommandBufferBuilder::primary(
                &instance.command_buffer_allocator,
                instance.queue_family_index,
                vk::CommandBufferUsage::MultipleSubmit,
            )
            .map_err(|_| TaskError::VulkanCommandBufferBuilderCreationFailed)?,
        })
    }

    pub fn build(self) -> Result<Task, TaskError> {
        Ok(Task {
            device: self.device,
            queue: self.queue,
            command_buffer: self
                .builder
                .build()
                .map_err(|_| TaskError::VulkanCommandBufferBuildFailed)?,
        })
    }

    pub fn build_submit_and_wait(self) -> Result<(), TaskError> {
        self.build()?.submit_and_wait()?;
        Ok(())
    }

    pub fn copy_buffer<T: BufferContents + Clone + Copy, BufferLocSrc, BufferLocDst>(
        mut self,
        src: &Buffer<T, BufferLocSrc>,
        dst: &Buffer<T, BufferLocDst>,
    ) -> Result<TaskBuilder, TaskError> {
        self.builder
            .copy_buffer(vk::CopyBufferInfo::buffers(
                src.get_vk_buffer().clone(),
                dst.get_vk_buffer().clone(),
            ))
            .map_err(|_| TaskError::VulkanCopyBufferFailed)?;
        Ok(self)
    }

    pub fn run_program(
        mut self,
        program: &Program,
        wg_size: (usize, usize, usize),
        bindings: Vec<BufferBinding>,
    ) -> Result<TaskBuilder, TaskError> {
        let descriptor_set_allocator =
            vk::StandardDescriptorSetAllocator::new(self.device.clone(), Default::default());

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
        )
        .map_err(|_| TaskError::VulkanDescriptorSetCreationFailed)?;

        self.builder
            .bind_pipeline_compute(program.compute_pipeline.clone())
            .map_err(|_| TaskError::VulkanPipelineBindingFailed)?
            .bind_descriptor_sets(
                vk::PipelineBindPoint::Compute,
                program.compute_pipeline.layout().clone(),
                0,
                descriptor_set,
            )
            .map_err(|_| TaskError::VulkanDescriptorSetBindingFailed)?
            .dispatch([wg_size.0 as u32, wg_size.1 as u32, wg_size.2 as u32])
            .map_err(|_| TaskError::VulkanDispatchFailed)?;

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_buffer() {
        let instance = Instance::new().unwrap();
        let buffer_a = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();
        let buffer_b = CpuBuffer::new(&instance, 4).unwrap();

        TaskBuilder::new(&instance)
            .unwrap()
            .copy_buffer(&buffer_a, &buffer_b)
            .unwrap()
            .build_submit_and_wait()
            .unwrap();

        assert_eq!(buffer_a.read().unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read().unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn copy_buffer_twice() {
        let instance = Instance::new().unwrap();

        let buffer_a = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();
        let buffer_b = CpuBuffer::new(&instance, 4).unwrap();

        let task = TaskBuilder::new(&instance)
            .unwrap()
            .copy_buffer(&buffer_a, &buffer_b)
            .unwrap()
            .build()
            .unwrap();

        task.submit_and_wait().unwrap();

        assert_eq!(buffer_a.read().unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read().unwrap(), vec![1, 2, 3, 4]);

        buffer_a.write(vec![5, 6, 7, 8]).unwrap();

        task.submit_and_wait().unwrap();

        assert_eq!(buffer_a.read().unwrap(), vec![5, 6, 7, 8]);
        assert_eq!(buffer_b.read().unwrap(), vec![5, 6, 7, 8]);
    }

    #[test]
    fn copy_buffer_sub() {
        let instance = Instance::new().unwrap();

        let buffer_a = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();
        let buffer_b = CpuBuffer::new(&instance, 4).unwrap();

        TaskBuilder::new(&instance)
            .unwrap()
            .copy_buffer(&buffer_a.sub(0..2).unwrap(), &buffer_b.sub(1..3).unwrap())
            .unwrap()
            .build_submit_and_wait()
            .unwrap();

        assert_eq!(buffer_a.read().unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(buffer_b.read().unwrap(), vec![0, 1, 2, 0]);
    }

    #[test]
    fn copy_gpu_buffer() {
        let instance = Instance::new().unwrap();
        let buffer_a = CpuBuffer::<u32>::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();
        let buffer_b = GpuBuffer::<u32>::new(&instance, 4).unwrap();
        let buffer_c = CpuBuffer::<u32>::new(&instance, 4).unwrap();

        TaskBuilder::new(&instance)
            .unwrap()
            .copy_buffer(&buffer_a, &buffer_b)
            .unwrap()
            .copy_buffer(&buffer_b, &buffer_c)
            .unwrap()
            .build_submit_and_wait()
            .unwrap();

        assert_eq!(buffer_a.read().unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(buffer_c.read().unwrap(), vec![1, 2, 3, 4]);
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
        let buffer = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();

        TaskBuilder::new(&instance)
            .unwrap()
            .run_program(&program, (4, 1, 1), vec![buffer.bind(0)])
            .unwrap()
            .build_submit_and_wait()
            .unwrap();

        assert_eq!(buffer.read().unwrap(), vec![2, 4, 6, 8]);
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
        let buffer = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();

        let task = TaskBuilder::new(&instance)
            .unwrap()
            .run_program(&program, (4, 1, 1), vec![buffer.bind(0)])
            .unwrap()
            .build()
            .unwrap();

        task.submit_and_wait().unwrap();
        assert_eq!(buffer.read().unwrap(), vec![2, 4, 6, 8]);

        task.submit_and_wait().unwrap();
        assert_eq!(buffer.read().unwrap(), vec![4, 8, 12, 16]);
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
        let buffer = CpuBuffer::from_vec(&instance, vec![1, 2, 3, 4]).unwrap();

        assert!(TaskBuilder::new(&instance)
            .unwrap()
            .run_program(&program, (4, 1, 1), vec![buffer.bind(1)])
            .is_err());
    }
}
