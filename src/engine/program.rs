use super::*;
use shaderc;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProgramError {
    #[error("failed to compile shader: {0}")]
    CompilationFailed(String),
    #[error("failed to create vulkan shader module")]
    VulkanShaderModuleCreationFailed,
    #[error("failed to find entry point \"{0}\"")]
    EntryPointNotFound(String),
    #[error("failed to create vulkan pipeline layout")]
    VulkanPipelineLayoutCreationFailed,
    #[error("failed to create vulkan pipeline")]
    VulkanPipelineCreationFailed,
}

pub struct Program {
    warnings: String,
    pub(super) compute_pipeline: Arc<vk::ComputePipeline>,
}

impl Program {
    pub fn new(
        instance: &Instance,
        source: &str,
        name: &str,
        entry_point: &str,
    ) -> Result<Program, ProgramError> {
        let compiler = shaderc::Compiler::new().unwrap();

        let spirv = match compiler.compile_into_spirv(
            source,
            shaderc::ShaderKind::Compute,
            name,
            entry_point,
            None,
        ) {
            Ok(result) => result,
            Err(shaderc::Error::CompilationError(_, error_info)) => {
                return Err(ProgramError::CompilationFailed(error_info))
            }
            Err(e) => panic!("unknown SPIR-V compile error: {:?}", e),
        };

        let shared_module = {
            unsafe {
                vk::ShaderModule::new(
                    instance.device.clone(),
                    vk::ShaderModuleCreateInfo::new(spirv.as_binary()),
                )
                .map_err(|_| ProgramError::VulkanShaderModuleCreationFailed)?
                .entry_point(entry_point)
                .ok_or(ProgramError::EntryPointNotFound(entry_point.to_string()))?
            }
        };

        let stage = vk::PipelineShaderStageCreateInfo::new(shared_module);

        let layout = vk::PipelineLayout::new(
            instance.device.clone(),
            vk::PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                .into_pipeline_layout_create_info(instance.device.clone())
                .unwrap(),
        )
        .map_err(|_| ProgramError::VulkanPipelineLayoutCreationFailed)?;

        let compute_pipeline = vk::ComputePipeline::new(
            instance.device.clone(),
            None,
            vk::ComputePipelineCreateInfo::stage_layout(stage, layout),
        )
        .map_err(|_| ProgramError::VulkanPipelineCreationFailed)?;

        Ok(Program {
            warnings: spirv.get_warning_messages(),
            compute_pipeline,
        })
    }

    pub fn get_warnings(&self) -> String {
        self.warnings.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile() {
        let code = r"
            #version 460
            void main() { uint idx = gl_GlobalInvocationID.x; }
        ";
        let instance = Instance::new().unwrap();
        assert!(Program::new(&instance, &code, "test.glsl", "main").is_ok());
    }

    #[test]
    fn compile_syntax_error() {
        let code = r"
            #version 460
            void main() { uint idx = gl_GlobalInvocationID.x }
        ";
        let instance = Instance::new().unwrap();
        assert!(Program::new(&instance, &code, "test.glsl", "main").is_err());
    }

    #[test]
    fn compile_entry_point_error() {
        let code = r"
            #version 460
            void main() { uint idx = gl_GlobalInvocationID.x; }
        ";
        let instance = Instance::new().unwrap();
        assert!(Program::new(&instance, &code, "test.glsl", "not_main").is_err());
    }
}
