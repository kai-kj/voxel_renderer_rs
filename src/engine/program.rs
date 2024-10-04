use super::errors::{EngineError, Result};
use super::instance::Instance;
use super::vulkan as vk;
use shaderc;
use std::sync::Arc;

pub struct Program {
    warnings: String,
    pub(super) compute_pipeline: Arc<vk::ComputePipeline>,
}

impl Program {
    pub fn new(
        instance: &Instance,
        source: &str,
        file_name: &str,
        entry_point: &str,
    ) -> Result<Program> {
        let compiler = shaderc::Compiler::new().unwrap();

        let spirv = match compiler.compile_into_spirv(
            source,
            shaderc::ShaderKind::Compute,
            file_name,
            entry_point,
            None,
        ) {
            Ok(result) => result,
            Err(shaderc::Error::CompilationError(_, error_info)) => {
                return EngineError::CompileError(error_info).into_result()
            }
            Err(e) => return Err(e.into()),
        };

        let shared_module = {
            unsafe {
                vk::ShaderModule::new(
                    instance.device.clone(),
                    vk::ShaderModuleCreateInfo::new(spirv.as_binary()),
                )
            }
        }?;

        let shared_module = shared_module
            .entry_point(entry_point)
            .ok_or(EngineError::WrongSpecialization)?;

        let stage = vk::PipelineShaderStageCreateInfo::new(shared_module);

        let layout = vk::PipelineLayout::new(
            instance.device.clone(),
            vk::PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                .into_pipeline_layout_create_info(instance.device.clone())
                .unwrap(),
        )?;

        let compute_pipeline = vk::ComputePipeline::new(
            instance.device.clone(),
            None,
            vk::ComputePipelineCreateInfo::stage_layout(stage, layout),
        )?;

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
    fn compile() -> Result<()> {
        let code = r"
            #version 460
            void main() { uint idx = gl_GlobalInvocationID.x; }
        ";
        let instance = Instance::new()?;
        assert!(Program::new(&instance, &code, "test.glsl", "main").is_ok());
        Ok(())
    }

    #[test]
    fn compile_syntax_error() -> Result<()> {
        let code = r"
            #version 460
            void main() { uint idx = gl_GlobalInvocationID.x }
        ";
        let instance = Instance::new()?;
        assert!(Program::new(&instance, &code, "test.glsl", "main").is_err());
        Ok(())
    }

    #[test]
    fn compile_entry_point_error() -> Result<()> {
        let code = r"
            #version 460
            void main() { uint idx = gl_GlobalInvocationID.x; }
        ";
        let instance = Instance::new()?;
        assert!(Program::new(&instance, &code, "test.glsl", "not_main").is_err());
        Ok(())
    }
}
