use super::preamble::*;
use image;

pub struct Renderer {
    instance: Instance,
    render_program: Program,
}

impl Renderer {
    pub fn new(instance: Instance, render_shader: &str) -> Result<Renderer> {
        let render_program = Program::new(&instance, render_shader, "render.glsl", "main")?;
        Ok(Renderer {
            instance,
            render_program,
        })
    }

    pub fn render(&self, image_size: glam::UVec2) -> Result<image::RgbaImage> {
        if image_size.x == 0 || image_size.y == 0 {
            return Err(anyhow!("invalid image size"));
        }

        let image = CpuBuffer::<f32>::new(
            &self.instance,
            4 * image_size.x as usize * image_size.y as usize,
        )?;

        TaskBuilder::new(&self.instance)?
            .run_program(
                &self.render_program,
                (image_size.x as usize, image_size.y as usize, 1),
                vec![image.bind(0)],
            )?
            .build()?
            .submit()?
            .wait()?;

        let image =
            image::Rgba32FImage::from_raw(image_size.x, image_size.y, image.read()?).unwrap();

        Ok(image::DynamicImage::ImageRgba32F(image).into_rgba8())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_image() {
        let code = r"
            #version 460
            layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
            layout(binding = 0) buffer Image { vec4 image[]; };
            ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
            ivec2 size = ivec2(gl_NumWorkGroups.xy * gl_WorkGroupSize.xy);
            void main() {
                image[pos.y * size.x + pos.x] = vec4(1.0);
            }
        ";

        let reference_image = image::ImageReader::open("test_references/blank_image.png")
            .unwrap()
            .decode()
            .unwrap()
            .into_rgba8();

        let instance = Instance::new().unwrap();
        let rendered_image = Renderer::new(instance, code)
            .unwrap()
            .render(glam::UVec2::new(720, 480))
            .unwrap();

        assert_eq!(reference_image, rendered_image);
    }

    #[test]
    fn grad_image() {
        let code = r"
            #version 460
            layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
            layout(binding = 0) buffer Image { vec4 image[]; };
            ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
            ivec2 size = ivec2(gl_NumWorkGroups.xy * gl_WorkGroupSize.xy);
            void main() {
                image[pos.y * size.x + pos.x] = vec4(vec2(pos) / vec2(size), 0.0, 1.0);
            }
        ";

        let reference_image = image::ImageReader::open("test_references/grad_image.png")
            .unwrap()
            .decode()
            .unwrap()
            .into_rgba8();

        let instance = Instance::new().unwrap();
        let rendered_image = Renderer::new(instance, code)
            .unwrap()
            .render(glam::UVec2::new(720, 480))
            .unwrap();

        assert_eq!(reference_image, rendered_image);
    }
}
