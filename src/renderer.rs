use std::path::PathBuf;
use crate::texturing;
use wgpu::BufferDescriptor;
use crate::world;
use crate::geometry;
use crate::camera;
use glam::{Vec3A, Mat4};
use wgpu::include_wgsl;
use wgpu::util::DeviceExt;
use std::sync::Arc;
use wgpu::PresentMode;
use crate::geometry::Vertex;
use crate::ui;

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
        println!("{:?}", tm.ui_scale);
        glyphon::TextArea {
            buffer: &self.buffer,
            left: self.x,
            top: self.y,
            scale: tm.ui_scale,
            bounds: glyphon::TextBounds {
                left: 0,
                top: 0,
                right: tm.screen_size.0 as i32,
                bottom: tm.screen_size.1 as i32,
            },
            default_color: glyphon::Color::rgb(0, 0, 0),
        }
    }
}

pub struct TextManager {
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    //cache: glyphon::Cache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    pub text_renderer: glyphon::TextRenderer,

    screen_size: (f32, f32),
    ui_scale: f32,

    text_objects: Vec<TextObject>
}
impl TextManager {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, surface_format: wgpu::TextureFormat, screen_size: winit::dpi::PhysicalSize<u32>) -> Self {
        println!("{:?}",
            std::fs::read_dir("assets/fonts/").unwrap().map(|path| path.unwrap().path()).collect::<Vec<PathBuf>>()
        );

        let fonts_to_load = std::fs::read_dir("assets/fonts/").unwrap().map(|path| glyphon::cosmic_text::fontdb::Source::File(path.unwrap().path()));
        let font_system = glyphon::FontSystem::new_with_fonts(fonts_to_load);
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(device);
        let viewport = glyphon::Viewport::new(device, &cache);
        let mut atlas = glyphon::TextAtlas::with_color_mode(device, queue, &cache, surface_format, glyphon::ColorMode::Accurate);
        let text_renderer = glyphon::TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        //println!("{:?}", font_system.get_font_matches(glyphon::Attrs::new().family(glyphon::Family::Name("asjkdhgasjdhgas"))));

        Self {
            font_system,
            swash_cache,
            //cache,
            viewport,
            atlas,
            text_renderer,

            screen_size: (screen_size.width as f32, screen_size.height as f32),
            ui_scale: 1.0,

            text_objects: vec![]
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
        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            ).unwrap();
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.text_renderer.render(&self.atlas, &self.viewport, render_pass).unwrap();
    }
}

pub struct Renderer<'a> {
    device: wgpu::Device,
    queue: wgpu::Queue,

    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'a>,
    surface_config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub aspect_ratio: f32,
    pub ui_scale: f32,

    pub camera: camera::Camera,

    // for main 3d rendering, not ui stuff (that will be in UILayers)
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub shader: wgpu::ShaderModule,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    frame_data_buffer: wgpu::Buffer,
    frame_data_bind_group: wgpu::BindGroup,
    frame_data_bind_group_layout: wgpu::BindGroupLayout,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_sets: Vec<texturing::TextureSet>,

    pub text_manager: TextManager,
    //debug_text: TextObject,
}

impl<'a> Renderer<'a> {
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();
        let aspect_ratio = size.width as f32 / size.height as f32;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.expect("Failed to get adapter");

        println!("Using backend {}", adapter.get_info().backend.to_str().to_uppercase());

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::PUSH_CONSTANTS,
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None, // Trace path
        ).await.expect("Failed to get device and queue from adapter");

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(surface_capabilities.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };


        // MAKE PIPELINE
        let verts = geometry::CUBE;
        let indices: Vec<u32> = (0..36).map(|i| {
            [0u32, 1u32, 2u32, 2u32, 3u32, 0u32][(i%6) as usize] + i / 6 * 4
        }).collect();

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let frame_data_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Frame Data Buffer"),
                size: std::mem::size_of::<FrameData>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );
        let frame_data_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("frame_data_bind_group_layout"),
        });
        let frame_data_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &frame_data_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame_data_buffer.as_entire_binding(),
                }
            ],
            label: Some("frame_data_bind_group"),
        });


        let shader = device.create_shader_module(include_wgsl!("main.wgsl"));

        let texture_bind_group_layout = device.create_bind_group_layout(&texturing::TEXTURE_SET_LAYOUT_DESC);

        //let pipeline = Self::create_main_pipeline(&device, &shader, &pipeline_layout, &surface_config);
        let mut text_manager = TextManager::new(&device, &queue, surface_format, size);
        text_manager.new_text_object(12.0, 10.0, 10.0);

        Self {
            device,
            queue,

            window: window.clone(),
            surface,
            surface_config,
            size,
            aspect_ratio,
            ui_scale: window.scale_factor() as f32,

            camera: camera::Camera::new(aspect_ratio),

            pipeline: None,
            shader,
            vertex_buffer,
            index_buffer,
            frame_data_buffer,
            frame_data_bind_group,
            frame_data_bind_group_layout,

            texture_bind_group_layout,
            texture_sets: vec![],

            text_manager,
        }
    }

    fn create_main_pipeline(&self) -> wgpu::RenderPipeline {
        let mut bind_group_layouts: Vec<&wgpu::BindGroupLayout> = vec![];
        bind_group_layouts.push(&self.frame_data_bind_group_layout);
        for _ in 0..self.texture_sets.len() {
            bind_group_layouts.push(&self.texture_bind_group_layout);
        }

        let pipeline_layout = self.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &bind_group_layouts,
                push_constant_ranges: &[],
            }
        );        

        self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: "vs_main", // 1.
                buffers: &[
                    geometry::Vertex::desc(),
                ], // 2.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState { // 3.
                module: &self.shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { // 4.
                    format: self.surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },    
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1, // 2.
                mask: !0, // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
            cache: None, // 6.
        })
    }

    pub fn load_texture_set(&mut self, fp_vec: Vec<&str>) {
        self.texture_sets.push(texturing::TextureSet::from_fp_vec(&self.device, &self.queue, &self.texture_bind_group_layout, fp_vec))
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.aspect_ratio = self.size.width as f32 / self.size.height as f32;

            self.text_manager.on_resize(new_size, self.ui_scale);
            self.text_manager.viewport.update(
                &self.queue,
                glyphon::Resolution {
                    width: new_size.width,
                    height: new_size.height,
                },
            );

            self.camera.set_aspect_ratio(self.aspect_ratio);
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn render(&mut self, world: &world::World) -> Result<(), wgpu::SurfaceError> {
        if self.pipeline.is_none() {
            self.pipeline = Some(self.create_main_pipeline());
        }

        // get framebuffer (wgpu considers every Image to be a texture) and view
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        // create command buffer builder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        self.text_manager.prepare(&self.device, &self.queue);

        // create render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: world.sky_color[0].into(),
                            g: world.sky_color[1].into(),
                            b: world.sky_color[2].into(),
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // FRAME DATA UNIFORM
            let data = FrameData {
                projview: self.camera.get_projview(&world.player).to_cols_array_2d()
            };
            self.queue.write_buffer(
                &self.frame_data_buffer,
                0,
                bytemuck::cast_slice(&[data])
            );

            // SEND IT ALL IN
            render_pass.set_pipeline(self.pipeline.as_ref().unwrap()); // 2.
            render_pass.set_bind_group(0, &self.frame_data_bind_group, &[]);
            for (i, texset) in self.texture_sets.iter().enumerate() {
                render_pass.set_bind_group((i+1) as u32, &texset.bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32); // 1.
            render_pass.draw_indexed(0..36, 0, 0..1); // 2.

            self.text_manager.render(&mut render_pass);
        }

    // submit will accept anything that implements IntoIter
    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();

    self.text_manager.atlas.trim();

    Ok(())

    }

}