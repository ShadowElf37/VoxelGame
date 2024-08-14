pub struct TextureSet {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
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
    pub fn from_fp_vec(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, fp_vec: Vec<&str>) -> Self {
        use image::{ImageBuffer, Rgba, ImageReader};

        fn load_rgba8(fp: &str) -> ImageBuffer<Rgba<u8>, Vec<u8>> {ImageReader::open(fp).expect(&format!("Failed to load {}", fp)).decode().expect(&format!("Failed to decode {}", fp)).into_rgba8()}

        let mut dimensions: Vec<(u32, u32)> = vec![];
        let mut img_array_raw: Vec<u8> = vec![];
        for fp in &fp_vec {
            let img_buffer = load_rgba8(fp);
            dimensions.push(img_buffer.dimensions());
            img_array_raw.extend(img_buffer.into_raw());
        }
        // assert that the image dimensions are all equal so they can go in the array without scrambling or misalignment
        assert!(dimensions.iter().map(|(x,y)| x).min() == dimensions.iter().map(|(x,y)| x).max());
        assert!(dimensions.iter().map(|(x,y)| y).min() == dimensions.iter().map(|(x,y)| y).max());
        let dimensions = dimensions[0];

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: fp_vec.len().try_into().expect("Please do not load more than 4 billion textures. Thank you."),
        };

        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                // All textures are stored as 3D, we represent our 2D texture
                // by setting depth to 1.
                size: texture_size,
                mip_level_count: 1, // We'll talk about this a little later
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // Most images are stored using sRGB, so we need to reflect that here.
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
                // COPY_DST means that we want to copy data to this texture
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: None,
                // This is the same as with the SurfaceConfig. It
                // specifies what texture formats can be used to
                // create TextureViews for this texture. The base
                // texture format (Rgba8UnormSrgb in this case) is
                // always supported. Note that using a different
                // texture format is not supported on the WebGL2
                // backend.
                view_formats: &[],
            }
        );

        queue.write_texture(
            // Tells wgpu where to copy the pixel data
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual pixel data
            &img_array_raw,
            // The layout of the texture
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor{
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });


        let bind_group = device.create_bind_group(
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
        );

        Self {
            texture,
            view,
            sampler,
            bind_group,
        }
    }
}