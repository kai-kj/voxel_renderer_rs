use crate::preamble::*;

#[derive(BufferContents, Copy, Clone)]
#[repr(C)]
struct CameraProperties {
    pub pos: glam::Vec3,
    padding_1: u32,
    pub rot: glam::Vec3,
    padding_2: u32,
    pub sensor_size: glam::Vec2,
    pub focal_distance: f32,
}

impl CameraProperties {
    pub fn new(
        pos: glam::Vec3,
        rot: glam::Vec3,
        sensor_size: glam::Vec2,
        focal_distance: f32,
    ) -> Self {
        Self {
            pos,
            padding_1: 0,
            rot,
            padding_2: 0,
            sensor_size,
            focal_distance,
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
            struct Camera { vec3 pos; vec3 rot; vec2 sensor_size; float focal_distance; };
            layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
            layout(binding = 0) buffer buffer_1 { Camera camera; };
            ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
            ivec2 size = ivec2(gl_NumWorkGroups.xy * gl_WorkGroupSize.xy);
            void main() {
                camera.pos = vec3(1, 2, 3);
                camera.rot = vec3(4, 5, 6);
                camera.sensor_size = vec2(7, 8);
                camera.focal_distance = 9;
            }
        ";

        let instance = Instance::new().unwrap();
        let program = Program::new(&instance, &code, "test", "main").unwrap();

        let camera = CameraProperties::new(
            glam::vec3(0.0, 0.0, 0.0),
            glam::vec3(0.0, 0.0, 0.0),
            glam::vec2(0.0, 0.0),
            0.0,
        );

        let camera_buffer = Buffer::from_vec(&instance, vec![camera]).unwrap();

        TaskBuilder::new(&instance)
            .unwrap()
            .run_program(&program, (1, 1, 1), vec![camera_buffer.bind(0)])
            .unwrap()
            .build_submit_and_wait()
            .unwrap();

        let camera = camera_buffer.read().unwrap()[0];

        assert_eq!(camera.pos, glam::vec3(1.0, 2.0, 3.0));
        assert_eq!(camera.rot, glam::vec3(4.0, 5.0, 6.0));
        assert_eq!(camera.sensor_size, glam::vec2(7.0, 8.0));
        assert_eq!(camera.focal_distance, 9.0);
    }
}
