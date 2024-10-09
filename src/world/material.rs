use crate::preamble::*;

#[derive(BufferContents, Copy, Clone)]
#[repr(C)]
struct MaterialProperties {
    pub color: glam::Vec3,
    padding_1: u32,
    pub properties: glam::Vec4,
}

impl MaterialProperties {
    pub fn new(color: glam::Vec3, properties: glam::Vec4) -> Self {
        Self {
            color,
            padding_1: 0,
            properties,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment() {
        let code = r"
            #version 460
            struct Material { vec3 color; vec4 properties; };
            layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
            layout(binding = 0) buffer buffer_1 { Material material; };
            ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
            ivec2 size = ivec2(gl_NumWorkGroups.xy * gl_WorkGroupSize.xy);
            void main() {
                material.color = vec3(1, 2, 3);
                material.properties = vec4(4, 5, 6, 7);
            }
        ";

        let instance = Instance::new().unwrap();
        let program = Program::new(&instance, &code, "test", "main").unwrap();

        let material =
            MaterialProperties::new(glam::vec3(0.0, 0.0, 0.0), glam::vec4(0.0, 0.0, 0.0, 0.0));

        let material_buffer = Buffer::from_vec(&instance, vec![material]).unwrap();

        TaskBuilder::new(&instance)
            .unwrap()
            .run_program(&program, (1, 1, 1), vec![material_buffer.bind(0)])
            .unwrap()
            .build_submit_and_wait()
            .unwrap();

        let material = material_buffer.read().unwrap()[0];

        assert_eq!(material.color, glam::vec3(1.0, 2.0, 3.0));
        assert_eq!(material.properties, glam::vec4(4.0, 5.0, 6.0, 7.0));
    }
}
