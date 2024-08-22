use std::sync::Arc;
use std::fs;
use image::{ImageBuffer, Rgba, ImageReader};

pub struct TextureSet {
    pub texture: Arc<wgpu::Texture>,
    pub view: Arc<wgpu::TextureView>,
    pub sampler: Arc<wgpu::Sampler>,
    pub bind_group: Arc<wgpu::BindGroup>,
}

impl TextureSet {
    pub fn from_fp_vec(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, fp_vec: Vec<String>) -> Self {
        println!("Loading textures from file paths: {:?}", fp_vec);

        fn load_rgba8(fp: &str) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
            println!("Loading image from file path: {}", fp);
        
            // Check if the file exists
            if !fs::metadata(fp).is_ok() {
                return Err(format!("File does not exist: {}", fp));
            }
            // Attempt to open the file
            let reader = ImageReader::open(fp).map_err(|e| format!("Failed to load {}: {}", fp, e))?;
            // Attempt to decode the image
            let image = reader.decode().map_err(|e| format!("Failed to decode {}: {}", fp, e))?;
            // Convert to RGBA8 format
            let rgba_image = image.into_rgba8();
            Ok(rgba_image)
        }

        let mut dimensions: Vec<(u32, u32)> = vec![];
        let mut img_array_raw: Vec<u8> = vec![];
        for fp in &fp_vec {
            let img_buffer = load_rgba8(fp);
            let dim = img_buffer.clone().unwrap().dimensions();
            println!("Loaded image dimensions: {:?}", dim);
            dimensions.push(dim);
            img_array_raw.extend(img_buffer.unwrap().into_raw());
        }
        assert!(dimensions.iter().all(|&(x, y)| x == dimensions[0].0 && y == dimensions[0].1));
        let dimensions = dimensions[0];
        println!("Final texture dimensions: {:?}", dimensions);

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