use std::sync::Arc;
use image::{ImageBuffer, Rgba, ImageReader};

pub struct TextureSet {
    pub texture: Arc<wgpu::Texture>,
    pub view: Arc<wgpu::TextureView>,
    pub sampler: Arc<wgpu::Sampler>,
    pub bind_group: Arc<wgpu::BindGroup>,
}

pub const TEXTURE_SET_LAYOUT_DESC: wgpu::BindGroupLayoutDescriptor = wgpu::BindGroupLayoutDescriptor {
    entries: &[
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2Array,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
    ],
    label: Some("texture_bind_group_layout"),
};

impl TextureSet {
    pub fn from_fp_vec(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, fp_vec: Vec<String>) -> Self {

        fn load_rgba8(fp: &str) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
            ImageReader::open(fp)
                .expect(&format!("Failed to load {}", fp))
                .decode()
                .expect(&format!("Failed to decode {}", fp))
                .into_rgba8()
        }

        let mut dimensions: Vec<(u32, u32)> = vec![];
        let mut img_array_raw: Vec<u8> = vec![];
        for fp in &fp_vec {
            let img_buffer = load_rgba8(fp);
            dimensions.push(img_buffer.dimensions());
            img_array_raw.extend(img_buffer.into_raw());
        }
        // assert that the image dimensions are all equal so they can go in the array without scrambling or misalignment
        assert!(dimensions.iter().all(|&(x, y)| x == dimensions[0].0 && y == dimensions[0].1));
        let dimensions = dimensions[0];

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: fp_vec.len().try_into().expect("Please do not load more than 4 billion textures. Thank you."),
        };

        let texture = Arc::new(device.create_texture(
            &wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: None,
                view_formats: &[],
            }
        ));

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &img_array_raw,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let view = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        }));

        let sampler = Arc::new(device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }));

        let bind_group = Arc::new(device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    }
                ],
                label: None,
            }
        ));

        Self {
            texture,
            view,
            sampler,
            bind_group,
        }
    }
}