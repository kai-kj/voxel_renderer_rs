#[derive(Debug, PartialEq)]
pub enum Error {
    VkLibraryLoad,
    VkInstanceCreate,
    QueueFamilySearch,
    VkDeviceCreate,
    VkBufferCreate,
    VkBufferRead,
    VkBufferWrite,
    OutOfBoundsRead,
    OutOfBoundsWrite,
    ZeroSized,
    VkCommandBufferBuilderCreate,
    VkCommandSubmit,
    VkCommandBufferCreate,
    VkCommandBufferSubmit,
    VkFutureWait,
    CompileError(String),
    VkCompileError,
    VkSharedModuleCreate,
    VkShaderModuleSpecialization,
    VkPipelineLayoutCreate,
    VkComputePipelineCreate,
    VkDescriptorSetLayoutCreate,
    VkDescriptorSetCreate,
    UnequalBufferSize,
}

impl<T> Into<Result<T, Error>> for Error {
    fn into(self) -> Result<T, Error> {
        Err(self)
    }
}
