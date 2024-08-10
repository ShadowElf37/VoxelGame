use glam::f32::Vec3A;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        }
    }
}

pub enum Facing {
    N = 0,
    E = 1,
    W = 2,
    S = 3,
    U = 4,
    D = 5,
}

const UVS: [[f32; 2]; 4] = [
    [0.0, 0.0],
    [0.0, 1.0],
    [1.0, 1.0],
    [1.0, 0.0],
];

pub const CUBE: [Vertex; 24] = [
    // N
    Vertex { pos: [1.0, 1.0, 1.0], uv: UVS[0], },
    Vertex { pos: [1.0, 1.0, 0.0], uv: UVS[1], },
    Vertex { pos: [0.0, 1.0, 0.0], uv: UVS[2], },
    Vertex { pos: [0.0, 1.0, 1.0], uv: UVS[3], },
    // E
    Vertex { pos: [1.0, 0.0, 1.0], uv: UVS[0], },
    Vertex { pos: [1.0, 0.0, 0.0], uv: UVS[1], },
    Vertex { pos: [1.0, 1.0, 0.0], uv: UVS[2], },
    Vertex { pos: [1.0, 1.0, 1.0], uv: UVS[3], },
    // W
    Vertex { pos: [0.0, 1.0, 1.0], uv: UVS[0], },
    Vertex { pos: [0.0, 1.0, 0.0], uv: UVS[1], },
    Vertex { pos: [0.0, 0.0, 0.0], uv: UVS[2], },
    Vertex { pos: [0.0, 0.0, 1.0], uv: UVS[3], },
    // S
    Vertex { pos: [0.0, 0.0, 1.0], uv: UVS[0], },
    Vertex { pos: [0.0, 0.0, 0.0], uv: UVS[1], },
    Vertex { pos: [1.0, 0.0, 0.0], uv: UVS[2], },
    Vertex { pos: [1.0, 0.0, 1.0], uv: UVS[3], },
    // U
    Vertex { pos: [0.0, 1.0, 1.0], uv: UVS[0], },
    Vertex { pos: [0.0, 0.0, 1.0], uv: UVS[1], },
    Vertex { pos: [1.0, 0.0, 1.0], uv: UVS[2], },
    Vertex { pos: [1.0, 1.0, 1.0], uv: UVS[3], },
    // D
    Vertex { pos: [0.0, 0.0, 0.0], uv: UVS[0], },
    Vertex { pos: [0.0, 1.0, 0.0], uv: UVS[1], },
    Vertex { pos: [1.0, 1.0, 0.0], uv: UVS[2], },
    Vertex { pos: [1.0, 0.0, 0.0], uv: UVS[3], },
];