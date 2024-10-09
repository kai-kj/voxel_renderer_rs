use crate::preamble::*;

#[derive(BufferContents, Copy, Clone)]
#[repr(C)]
struct SceneProperties {
    pub size: glam::UVec3,
}
