use std::sync::{Arc, RwLock};
use wgpu::include_wgsl;
use wgpu::util::DeviceExt;
use wgpu::PresentMode;
use crate::texturing;
use crate::world;
use crate::camera;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameData {
    projview: [[f32; 4]; 4]
}

struct TextObject {
    buffer: glyphon::Buffer,
    x: f32,
    y: f32,
}
impl TextObject {
    pub fn new(tm: &mut TextManager, font_size: f32, x: f32, y: f32) -> Self {
        let mut buffer = glyphon::Buffer::new(&mut tm.font_system, glyphon::Metrics::new(font_size, font_size*1.2));
        buffer.set_size(
            &mut tm.font_system,
            Some(tm.screen_size.0),
            Some(tm.screen_size.1),
        );

        Self {
            buffer,
            x,
            y
        }
    }

    pub fn get_text_area(&self, tm: &TextManager) -> glyphon::TextArea {
        glyphon::TextArea {
            buffer: &self.buffer,
            left: self.x,
            top: self.y,
            scale: tm.ui_scale,
            bounds: glyphon::TextBounds {
                left: 0,
                right: tm.screen_size.0 as i32,
                top: 0,
                bottom: tm.screen_size.1 as i32,
            },
            default_color: glyphon::Color::rgb(0, 0, 0),
        }
    }
}

pub struct TextManager {
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    pub text_renderer: glyphon::TextRenderer,

    screen_size: (f32, f32),
    ui_scale: f32,

    text_objects: Vec<TextObject>
}
impl TextManager {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, surface_format: wgpu::TextureFormat, screen_size: winit::dpi::PhysicalSize<u32>, depth_stencil: Option<wgpu::DepthStencilState>) -> Self {
        let fonts_to_load = std::fs::read_dir("assets/fonts/").unwrap().map(|path| glyphon::cosmic_text::fontdb::Source::File(path.unwrap().path()));
        let font_system = glyphon::FontSystem::new_with_fonts(fonts_to_load);
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(device);
        let viewport = glyphon::Viewport::new(device, &cache);
        let mut atlas = glyphon::TextAtlas::with_color_mode(device, queue, &cache, surface_format, glyphon::ColorMode::Accurate);
        let text_renderer = glyphon::TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), depth_stencil);

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,

            screen_size: (screen_size.width as f32, screen_size.height as f32),
            ui_scale: 1.0,
            text_objects: Vec::new(),
        }
    }

    pub fn new_text_object(&mut self, font_size: f32, x: f32, y: f32){
        let to = TextObject::new(self, font_size, x, y);
        self.text_objects.push(to);
    }

    pub fn set_text_on(&mut self, index: usize, text: &str) {
        let to = &mut self.text_objects[index];
        to.buffer.set_text(&mut self.font_system, text, glyphon::Attrs::new().family(glyphon::Family::Name("BigBlueTermPlus Nerd Font Mono")), glyphon::Shaping::Basic);
        to.buffer.shape_until_scroll(&mut self.font_system, false);
    }

    pub fn on_resize(&mut self, screen_size: winit::dpi::PhysicalSize<u32>, ui_scale: f32) {
        self.screen_size = (screen_size.width as f32, screen_size.height as f32);
        self.ui_scale = ui_scale;
        for tobj in self.text_objects.iter_mut() {
            tobj.buffer.set_size(
                &mut self.font_system,
                Some(self.screen_size.0),
                Some(self.screen_size.1),
            );
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let text_areas = self.text_objects.iter().map(|tobj| tobj.get_text_area(&self)).collect::<Vec<glyphon::TextArea>>();
        self.text_renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas, // Add this argument
            &self.viewport, // Add this argument
            text_areas.iter().cloned(),
            &mut self.swash_cache
        ).unwrap();
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.text_renderer.render(&self.atlas, &self.viewport, render_pass).unwrap();
    }
}

pub struct Renderer<'a> {
    pub device: Arc<RwLock<wgpu::Device>>,
    queue: Arc<RwLock<wgpu::Queue>>,

    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'a>,
    surface_config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub aspect_ratio: f32,
    pub window_center_px: winit::dpi::PhysicalPosition<u32>,
    pub ui_scale: f32,
    pub ui_scale_manual_adjust: f32,

    pub camera: Arc<RwLock<camera::Camera>>,

    pub pipeline: Option<wgpu::RenderPipeline>,
    pub shader: wgpu::ShaderModule,
    pub index_buffer: Option<wgpu::Buffer>,
    index_counts: Vec<u32>,
    depth_texture_view: wgpu::TextureView,
    depth_texture_sampler: wgpu::Sampler,
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    frame_data_buffer: wgpu::Buffer,
    frame_data_bind_group: wgpu::BindGroup,
    frame_data_bind_group_layout: wgpu::BindGroupLayout,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_sets: Vec<texturing::TextureSet>,

    pub text_manager: Arc<RwLock<TextManager>>,
}

impl<'a> Renderer<'a> {
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();
        let aspect_ratio = size.width as f32 / size.height as f32;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.expect("Failed to get adapter");

        println!("Using backend {}", adapter.get_info().backend.to_str().to_uppercase());

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            label: None,
        }, None).await.expect("Failed to get device and queue from adapter");

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(surface_capabilities.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 3,
        };
        surface.configure(&device, &surface_config);

        let depth_stencil_state = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            anisotropy_clamp: 0,
            border_color: None,
            label: Some("Depth Texture Sampler"),
        });

        let frame_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frame Data Buffer"),
            size: std::mem::size_of::<FrameData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_data_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("frame_data_bind_group_layout"),
        });
        let frame_data_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &frame_data_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame_data_buffer.as_entire_binding(),
                },
            ],
            label: Some("frame_data_bind_group"),
        });

        let shader = device.create_shader_module(include_wgsl!("main.wgsl"));

        let texture_bind_group_layout = device.create_bind_group_layout(&texturing::TEXTURE_SET_LAYOUT_DESC);

        println!("Loading fonts...");
        let mut text_manager = TextManager::new(&device, &queue, surface_format, size, depth_stencil_state.clone());
        text_manager.new_text_object(12.0, 10.0, 10.0);

        Self {
            device: Arc::new(RwLock::new(device)),
            queue: Arc::new(RwLock::new(queue)),
            window,
            surface,
            surface_config,
            size,
            aspect_ratio,
            window_center_px: winit::dpi::PhysicalPosition::new(size.width / 2, size.height / 2),
            ui_scale: 1.0,
            ui_scale_manual_adjust: 0.0,
            camera: Arc::new(RwLock::new(camera::Camera::new(aspect_ratio))),
            pipeline: None,
            shader,
            index_buffer: None,
            index_counts: Vec::new(),
            depth_texture_view,
            depth_texture_sampler,
            depth_stencil_state,
            frame_data_buffer,
            frame_data_bind_group,
            frame_data_bind_group_layout,
            texture_bind_group_layout,
            texture_sets: Vec::new(),
            text_manager: Arc::new(RwLock::new(text_manager)),
        }
    }

    pub fn push_indices(&mut self, indices: Vec<u32>, index_offsets: Vec<u32>) {
        let device = self.device.read().unwrap();
        self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        }));
        self.index_counts = index_offsets;
    }

    fn create_main_pipeline(&self) -> wgpu::RenderPipeline {
        let device = self.device.read().unwrap();
        let mut bind_group_layouts: Vec<&wgpu::BindGroupLayout> = vec![];
        bind_group_layouts.push(&self.frame_data_bind_group_layout);
        for _ in 0..self.texture_sets.len() {
            bind_group_layouts.push(&self.texture_bind_group_layout);
        }

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: "main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: self.depth_stencil_state.clone(),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })
    }

    pub fn load_texture_set(&mut self, fp_vec: Vec<String>) {
        println!("Loading texture set...");
        let device = self.device.read().unwrap();
        let queue = self.queue.read().unwrap();
        self.texture_sets.push(texturing::TextureSet::from_fp_vec(&device, &queue, &self.texture_bind_group_layout, fp_vec))
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.aspect_ratio = new_size.width as f32 / new_size.height as f32;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            let device = self.device.read().unwrap();
            self.surface.configure(&device, &self.surface_config);
        }
    }

    pub fn render(&mut self, _world: &world::World) -> Result<(), wgpu::SurfaceError> {
        if self.pipeline.is_none() {
            self.pipeline = Some(self.create_main_pipeline());
        }

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let device = self.device.read().unwrap();
        let queue = self.queue.read().unwrap();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        self.text_manager.write().unwrap().prepare(&device, &queue);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some(pipeline) = &self.pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, &self.frame_data_bind_group, &[]);
                for (i, texture_set) in self.texture_sets.iter().enumerate() {
                    render_pass.set_bind_group((i + 1) as u32, &texture_set.bind_group, &[]);
                }
                if let Some(index_buffer) = &self.index_buffer {
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    for &index_count in &self.index_counts {
                        render_pass.draw_indexed(0..index_count, 0, 0..1);
                    }
                }
            }

            self.text_manager.read().unwrap().render(&mut render_pass);
        }

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.text_manager.write().unwrap().atlas.trim();

        Ok(())
    }
}