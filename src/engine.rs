use vulkano::image::ImageAspects;
use vulkano::image::ImageSubresourceLayers;
use vulkano::image::ImageLayout;
use vulkano::command_buffer::ImageBlit;
use vulkano::command_buffer::BlitImageInfo;
use vulkano::image::sampler::BorderColor;
use vulkano::image::sampler::SamplerCreateInfo;
use vulkano::image::sampler::Filter;
use vulkano::image::sampler::SamplerMipmapMode;
use vulkano::image::sampler::SamplerAddressMode;
use vulkano::image::sampler::Sampler;
use vulkano::pipeline::graphics::rasterization::FrontFace;
use vulkano::pipeline::graphics::rasterization::CullMode;
use vulkano::pipeline::graphics::vertex_input::Vertex as vulkan_vertex;
use crate::camera;
use crate::vk_select_device;
use std::sync::{Arc, Mutex};
use vulkano::buffer::allocator::SubbufferAllocator;
use vulkano::buffer::allocator::SubbufferAllocatorCreateInfo;
use vulkano::buffer::BufferContents;
use vulkano::buffer::Subbuffer;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::RenderPassBeginInfo;
use vulkano::command_buffer::SubpassBeginInfo;
use vulkano::command_buffer::SubpassContents;
use vulkano::command_buffer::SubpassEndInfo;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage, PrimaryAutoCommandBuffer,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::layout::DescriptorSetLayout;
use vulkano::descriptor_set::DescriptorSet;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::DeviceExtensions;
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo, QueueFlags};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType};
use vulkano::instance::InstanceCreateFlags;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::ColorBlendAttachmentState;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::depth_stencil::DepthState;
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::VertexDefinition;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::Pipeline;
use vulkano::render_pass::Framebuffer;
use vulkano::render_pass::FramebufferCreateInfo;
use vulkano::render_pass::RenderPass;
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;
use vulkano::shader::ShaderModule;
use vulkano::swapchain;
use vulkano::swapchain::Surface;
use vulkano::swapchain::SwapchainCreateInfo;
use vulkano::sync::future::{FenceSignalFuture, NowFuture};
use vulkano::VulkanLibrary;
use vulkano::{Validated, VulkanError};
use winit::dpi::PhysicalSize;
use winit::window::Window;
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::swapchain::SwapchainPresentInfo;
use vulkano::sync;
use vulkano::command_buffer::ClearColorImageInfo;
use vulkano::command_buffer::CopyImageToBufferInfo;
use vulkano::format::ClearColorValue;
use vulkano::image::ImageUsage;
use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::PipelineBindPoint;
use vulkano::pipeline::{ComputePipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::swapchain::Swapchain;
use vulkano::sync::GpuFuture;
use winit::event_loop::EventLoop;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;


use crate::clock;
use crate::world;

#[derive(vulkano::buffer::BufferContents, vulkano::pipeline::graphics::vertex_input::Vertex)]
#[repr(C)]
pub struct Vertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
    #[format(R32_UINT)]
    pub tex_index: u32,
    //#[format(R32G32B32_SFLOAT)]
    //pub normal: [f32; 3],
}

#[derive(vulkano::buffer::BufferContents, vulkano::pipeline::graphics::vertex_input::Vertex)]
#[repr(C)]
pub struct Vertex2D {
    #[format(R32G32_SFLOAT)]
    pub pos: [f32; 2]
}

pub struct BufferSet {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    //pub normals_buffer: Subbuffer<[Normal]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub uniform_buffer_allocator: SubbufferAllocator,
    pub ui_vertex_buffer: Subbuffer<[Vertex2D]>
}

pub struct TextObj {
    text: String,
    x: u32,
    y: u32,
    scale: f32,
}
impl TextObj {
    pub fn new(text: String, x: u32, y: u32, scale: f32,) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self{text: text, x: x, y: y, scale: scale}))
    }
    pub fn write(&mut self, s: String) {
        self.text = s;
    }
}
pub struct TextManager {
    font_map: [usize; 256],
    char_images: Vec<Arc<Image>>,
}
impl TextManager {
    pub fn new() -> Self {
        let mut fm = [300; 256];
        fm[10] = 500; // \n handled separately
        
        Self {
            font_map: fm, // char : image_index
            char_images: vec![],
        }
    }
    fn get_image_index_from_char(&self, c: char) -> usize {
        let result = self.font_map[c as usize]; // non-ascii characters aren't decoded by str.chars(), so we don't have to worry about them
        if result == 300 { // an ascii character that wasn't loaded in assets/font/ (for example, NULL)
            return self.font_map[63]; // question mark
        }
        return result;
        
    }
    fn render(&self, s: &str) -> Vec<Arc<Image>> {
        s.chars().map(|c| self.char_images[self.get_image_index_from_char(c)].clone()).collect()
    }
    fn blit_text_obj_info(&self, tobj: Arc<Mutex<TextObj>>, target: Arc<Image>) -> Vec<BlitImageInfo> {
        let isl = ImageSubresourceLayers {
            aspects: ImageAspects::COLOR,
            mip_level: 1,
            array_layers: (0..1),
        };


        let t_unlocked = tobj.lock().unwrap();
        let src_images = self.render(&t_unlocked.text);
        let mut x = t_unlocked.x;
        let mut y = t_unlocked.y;

        let target_extent = target.extent();

        src_images.iter().map(|src| {
            let src_extent = src.extent();
            let w_eff = ((src_extent[0] as f32)*t_unlocked.scale).round() as u32;
            let h_eff = ((src_extent[1] as f32)*t_unlocked.scale).round() as u32;
            let mut info = BlitImageInfo::images(src.clone(), target.clone());
            //println!("{:?} {:?}", info.regions[0].dst_offsets, [[x,y,0], [x+w_eff, y+h_eff, 1]]);
            info.regions[0].dst_offsets = [[x,y,0], [x+w_eff, y+h_eff, 1]];
            x += w_eff;
            if x+w_eff > target_extent[0] {
                y += h_eff;
                x = t_unlocked.x;
            }
            if y+h_eff > target_extent[1] {
                y = t_unlocked.y;
            }
            info
        }).collect()
    }

    pub fn blit_all(&self, texts: &Vec<Arc<Mutex<TextObj>>>, target: Arc<Image>, mut command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        for text in texts.iter() {
            for letter in self.blit_text_obj_info(text.clone(), target.clone()) {
                command_builder = command_builder.blit_image(letter).unwrap();
            }
        }
    }
}

pub struct UILayer {
    pub buffer: Arc<ImageView>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub vs: EntryPoint,
    pub fs: EntryPoint,

    pub texts: Vec<Arc<Mutex<TextObj>>>,
}
impl UILayer {
    pub fn new(game: &mut Game, vs: EntryPoint, fs: EntryPoint) -> Self {
        Self {
            buffer: Self::get_new_buffer(game.memory_allocator.clone(), game.dimensions, game.image_format),
            pipeline: Self::get_new_pipeline(vs.clone(), fs.clone(), game.device.clone(), game.render_pass.clone(), game.window.inner_size().into()),
            vs: vs,
            fs: fs,

            texts: vec![],
        }
    }

    pub fn get_new_buffer(memory_allocator: Arc<StandardMemoryAllocator>, dimensions: [u32; 2], image_format: Format) -> Arc<ImageView> {
        return ImageView::new_default(Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: image_format,
                extent: [dimensions[0], dimensions[1], 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        ).expect("Failed to create ui buffer!")).expect("Failed to create view over ui buffer!");
    }
    pub fn get_new_pipeline(vs: EntryPoint, fs: EntryPoint, device: Arc<Device>, render_pass: Arc<RenderPass>, extent: [f32; 2]) -> Arc<GraphicsPipeline> {

        let vertex_input_state = Vertex2D::per_vertex()//[Vertex::per_vertex(), Normal::per_vertex()]
            .definition(&vs.info().input_interface)
            .unwrap();

        let stages: Vec<PipelineShaderStageCreateInfo> = [vs.clone(), fs.clone()].iter().map(|shader| PipelineShaderStageCreateInfo::new(shader.clone())).collect();

        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap();

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        return GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState {
                    viewports: [Viewport {
                        // it is apparently ok to make one viewport eternal and resize it with `viewport.extent = new_dimensions.into()`, but teapot.rs does this
                        offset: [0.0, 0.0],
                        extent: extent,
                        depth_range: 0.0..=1.0,
                    }]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState{
                    cull_mode: CullMode::Back,
                    front_face: FrontFace::CounterClockwise,
                    ..Default::default()
                    }),
                depth_stencil_state: Some(DepthStencilState {
                    depth: Some(DepthState::simple()),
                    ..Default::default()
                }),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        ).unwrap();
    }
}

pub struct Game {
    pub device: Arc<vulkano::device::Device>,
    pub physical_device: Arc<PhysicalDevice>,
    pub queue: Arc<vulkano::device::Queue>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub command_buffer_allocator: StandardCommandBufferAllocator, //please do not arc
    pub descriptor_set_allocator: StandardDescriptorSetAllocator, //please do not arc

    pub swapchain: Arc<vulkano::swapchain::Swapchain>,
    pub recreate_swapchain: bool,
    pub images: Vec<Arc<Image>>,
    pub render_pass: Arc<vulkano::render_pass::RenderPass>,

    pub graphics_pipeline: Option<Arc<GraphicsPipeline>>,
    pub buffer_set: Option<Arc<BufferSet>>,
    pub shaders: Vec<EntryPoint>,
    pub textures: Vec<Arc<ImageView>>,
    pub texture_arrays: Vec<Arc<ImageView>>,
    pub texture_sampler: Arc<Sampler>,
    pub depth_buffer: Arc<ImageView>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub ui_layers: Vec<UILayer>,
    pub text_manager: TextManager,
    pub image_format: vulkano::format::Format,
    pub dimensions: [u32; 2],
    pub aspect_ratio: f32,

    pub window: Arc<Window>,
    pub surface: Arc<Surface>,
    pub hold_cursor: bool,

    pub clock: clock::Clock,
    pub world: world::World,
}

impl Game {
    // WM SECTION
    pub fn on_focus(&mut self) {
        self.hold_cursor = true;
        self.window.set_cursor_grab(winit::window::CursorGrabMode::Locked);
        self.window.set_cursor_visible(false);
    }
    pub fn on_defocus(&mut self) {
        self.hold_cursor = false;
        self.window.set_cursor_grab(winit::window::CursorGrabMode::None);
        self.window.set_cursor_visible(true);
    }

    // SWAPCHAIN SECTION
    pub fn recreate_swapchain(&mut self) {
        // creates new swapchain, frame (and ui) buffers, and graphics pipeline. the ui pipelines should survive resizes.
        self.update_dimensions();

        let (new_swapchain, new_images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: self.dimensions.into(),
                ..self.swapchain.create_info()
            })
            .expect("failed to recreate swapchain: {e}");

        self.swapchain = new_swapchain;
        self.images = new_images;

        self.create_framebuffers();
        self.create_graphics_pipeline();
    }
    fn get_framebuffers_internal(
        memory_allocator: Arc<StandardMemoryAllocator>,
        render_pass: Arc<RenderPass>,
        images: &[Arc<Image>],
    ) -> (Arc<ImageView>, Vec<Arc<Framebuffer>>) {
        let depth_buffer = ImageView::new_default(
            Image::new(
                memory_allocator.clone(),
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::D16_UNORM,
                    extent: images[0].extent(),
                    usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                    ..Default::default()
                },
            )
            .expect("Failed to create depth buffer image :("),
        )
        .expect("Failed to create depth buffer imageview :(");

        let framebuffers = images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view, depth_buffer.clone()],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();

        return (depth_buffer, framebuffers);
    }
    pub fn create_framebuffers(&mut self) {
        // please only ever do this after recreate() or creating new swapchain and images
        (self.depth_buffer, self.framebuffers) = Self::get_framebuffers_internal(
            self.memory_allocator.clone(),
            self.render_pass.clone(),
            &self.images,
        );
    }
    fn get_render_pass(device: Arc<Device>, image_format: Format) -> Arc<RenderPass> {
        use vulkano::render_pass::{RenderPass, RenderPassCreateInfo, AttachmentDescription, SubpassDescription, SubpassDependency};
        /*RenderPass {
            device.clone(),
            device.
            RenderPassCreateInfo {
                attachments: vec![
                    AttachmentDescription {
                        
                    }
                ]
            }
        };*/

        vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: image_format,
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
                depth_stencil: {
                    format: Format::D16_UNORM,
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {depth_stencil},
            },
        )
        .unwrap()
    }
    pub fn update_dimensions(&mut self) {
        self.dimensions = self.window.inner_size().into();
        self.aspect_ratio = self.dimensions[0] as f32 / self.dimensions[1] as f32;
    }

    pub fn new(event_loop: &EventLoop<()>) -> Self {
        // CREATE VK INSTANCE
        let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());

        let required_extensions = Surface::required_extensions(&window);
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create instance");

        // CREATE WINDOW

        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

        // USER-REQUIRED EXTENSIONS
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        // GET BEST GRAPHICS DEVICE BASED ON WINDOW PROPERTIES AND REQUIRED EXTENSIONS
        let (physical_device, queue_family_index) =
            vk_select_device::select_physical_device(&instance, &surface, &device_extensions);

        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                // here we pass the desired queue family to use by index
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let queue = queues.next().unwrap();

        // SWAPCHAIN
        let capabilities = physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");
        let dimensions = window.inner_size();
        let composite_alpha = capabilities
            .supported_composite_alpha
            .into_iter()
            .next()
            .unwrap();
        let image_format = physical_device.surface_formats(&surface, Default::default()).unwrap()[0].0;

        let (mut swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: capabilities.min_image_count + 1, // How many buffers to use in the swapchain
                image_format: image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST, // What the images are going to be used for
                composite_alpha,
                ..Default::default()
            },
        )
        .unwrap();

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let render_pass = Self::get_render_pass(device.clone(), image_format);

        let (depth_buffer, framebuffers) =
            Self::get_framebuffers_internal(memory_allocator.clone(), render_pass.clone(), &images);

        let texture_sampler = Sampler::new(
                    device.clone(),
                    SamplerCreateInfo {
                        mag_filter: Filter::Nearest,
                        min_filter: Filter::Nearest,
                        mipmap_mode: SamplerMipmapMode::Nearest,
                        address_mode: [SamplerAddressMode::ClampToBorder; 3],
                        mip_lod_bias: 0.0,
                        ..Default::default()
                    },
                ).unwrap();

        let mut text_manager = TextManager::new();


        // BUNDLE IT ALL UP
        return Self {
            device: device.clone(),
            physical_device: physical_device.clone(),
            queue: queue.clone(),
            memory_allocator: memory_allocator.clone(),
            command_buffer_allocator: (StandardCommandBufferAllocator::new(
                device.clone(),
                StandardCommandBufferAllocatorCreateInfo::default(),
            )),
            descriptor_set_allocator: (StandardDescriptorSetAllocator::new(
                device.clone(),
                Default::default(),
            )),

            swapchain: swapchain,
            recreate_swapchain: false,
            images: images,
            render_pass: render_pass,
            depth_buffer: depth_buffer,
            framebuffers: framebuffers,
            ui_layers: vec![],
            textures: vec![],
            texture_arrays: vec![],
            texture_sampler: texture_sampler,
            graphics_pipeline: None,
            buffer_set: None,
            shaders: vec![],

            image_format: image_format,
            dimensions: dimensions.into(),
            aspect_ratio: dimensions.width as f32 / dimensions.height as f32,

            window: window,
            surface: surface,
            hold_cursor: true,

            clock: clock::Clock::new(),
            world: world::World::new(),

            text_manager: text_manager,
        };
    }

    pub fn push_shader(&mut self, shader: EntryPoint) {
        self.shaders.push(shader);
    }

    pub fn alloc_image(
        &self,
        format: Format,
        extent: [u32; 3],
        usage: ImageUsage,
    ) -> Arc<vulkano::image::Image> {
        //ImageView::new_default(
        Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: format,
                extent: extent,
                usage: usage,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .expect("Failed to create image :(")
        //).expect("Failed to create image view :(")
    }

    fn create_compute_pipeline_from_shader(&self, cs: EntryPoint) -> Arc<ComputePipeline>{
        let stage = PipelineShaderStageCreateInfo::new(cs);
        let layout = PipelineLayout::new(
            self.device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                .into_pipeline_layout_create_info(self.device.clone()).unwrap(),
        ).unwrap();
        let compute_pipeline = ComputePipeline::new(
            self.device.clone(),
            None,
            ComputePipelineCreateInfo::stage_layout(stage, layout),
        ).expect("failed to create compute pipeline");
        return compute_pipeline;
    }

    pub fn submit_ui_pipelines(&mut self, acquired_image_index: u32, mut builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        for layer in &self.ui_layers {
            let layout = layer.pipeline.layout().clone();

            let desc_set_writers = vec![
                WriteDescriptorSet::image_view_sampler(0, layer.buffer.clone(), self.texture_sampler.clone())
            ];
            let desc_set = self.create_descriptor_set(0, desc_set_writers, layout.clone());
            builder
                .bind_pipeline_graphics(layer.pipeline.clone()).unwrap()
                .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, desc_set).unwrap()
                .bind_vertex_buffers(0, self.buffer_set.clone().unwrap().ui_vertex_buffer.clone()).unwrap()
                .draw(self.buffer_set.clone().unwrap().ui_vertex_buffer.len() as u32, 1, 0, 0);
        }
    }

    fn create_ui_pipelines(&mut self) {
        for layer in self.ui_layers.iter_mut() {
            layer.buffer = UILayer::get_new_buffer(self.memory_allocator.clone(), self.dimensions, self.image_format);
            layer.pipeline = UILayer::get_new_pipeline(layer.vs.clone(), layer.fs.clone(), self.device.clone(), self.render_pass.clone(), self.window.inner_size().into());
        }
    }

    pub fn create_all_pipelines(&mut self) {
        self.create_ui_pipelines();
        self.create_graphics_pipeline();
    }

    pub fn submit_graphics_pipeline(&mut self, acquired_image_index: u32, extra_desc_set_writers: Vec<WriteDescriptorSet>, mut builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        let buffer_set = self.buffer_set.clone().unwrap();
        let mut desc_set_writers = self.get_texture_descriptor_set_writers();
        desc_set_writers.extend(extra_desc_set_writers);
        let layout = self.graphics_pipeline.clone().unwrap().layout().clone();
        let desc_set = self.create_descriptor_set(0, desc_set_writers, layout.clone());

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        Some(self.world.sky_color.into()),
                        Some(1f32.into()),
                    ],
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[acquired_image_index as usize].clone(),
                    )
                },
                Default::default(),
            ).unwrap()
            .bind_pipeline_graphics(self.graphics_pipeline.clone().unwrap()).unwrap()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, desc_set).unwrap()
            .bind_vertex_buffers(0, buffer_set.vertex_buffer.clone()).unwrap()
            .bind_index_buffer(buffer_set.index_buffer.clone()).unwrap();

        unsafe {
            builder.draw_indexed(buffer_set.index_buffer.len() as u32, 1, 0, 0, 0).unwrap();
        }

        
    }

    fn create_graphics_pipeline(&mut self) {
        let extent: [f32; 2] = self.window.inner_size().into();

        let vertex_input_state = Vertex::per_vertex()//[Vertex::per_vertex(), Normal::per_vertex()]
            .definition(&self.shaders[0].info().input_interface)
            .unwrap();

        let stages: Vec<PipelineShaderStageCreateInfo> = self.shaders.iter().map(|shader| PipelineShaderStageCreateInfo::new(shader.clone())).collect();

        let layout = PipelineLayout::new(
            self.device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(self.device.clone())
                .unwrap(),
        )
        .unwrap();

        let subpass = Subpass::from(self.render_pass.clone(), 0).unwrap();

        let pipeline = GraphicsPipeline::new(
            self.device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState {
                    viewports: [Viewport {
                        // it is apparently ok to make one viewport eternal and resize it with `viewport.extent = new_dimensions.into()`, but teapot.rs does this
                        offset: [0.0, 0.0],
                        extent: extent,
                        depth_range: 0.0..=1.0,
                    }]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState{
                    cull_mode: CullMode::Back,
                    front_face: FrontFace::CounterClockwise,
                    ..Default::default()
                    }),
                depth_stencil_state: Some(DepthStencilState {
                    depth: Some(DepthState::simple()),
                    ..Default::default()
                }),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap();

        self.graphics_pipeline = Some(pipeline);
    }

    pub fn create_command_buffer_builder(
        &self,
    ) -> AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
        // ONE TIME BUFFER
        // check the windowing tutorial for creating and reusing 3 command buffers
        AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap()
    }
    pub fn submit_command_buffer_builder(&self, builder: AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> FenceSignalFuture<CommandBufferExecFuture<NowFuture>> {
        // this must move the builder. do not ever attach & to the builder parameter.
        let command_buffer = builder.build().unwrap();
        let commands_future = sync::now(self.device.clone())
            .then_execute(self.queue.clone(), command_buffer).unwrap()
            .then_signal_fence_and_flush().unwrap();
        commands_future
    }

    pub fn alloc_buffer_from_vector<T>(
        &self,
        vector: Vec<T>,
        usage: BufferUsage,
        cpu_readable: bool,
    ) -> vulkano::buffer::Subbuffer<[T]>
    where
        T: BufferContents,
    {
        let memflags = MemoryTypeFilter::PREFER_HOST;
        if cpu_readable {
            let memflags = memflags | MemoryTypeFilter::HOST_RANDOM_ACCESS;
        }

        Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: usage,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: memflags,
                ..Default::default()
            },
            vector,
        )
        .expect("failed to create buffer")
    }

    pub fn alloc_vertex_buffer<T: BufferContents>(
        &mut self,
        vertices: Vec<T>,
    ) -> vulkano::buffer::Subbuffer<[T]> {
        Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vertices,
        )
        .expect("failed to create vertex buffer")
    }

    pub fn create_descriptor_set(
        &self,
        set_idx: usize,
        desc_set: Vec<WriteDescriptorSet>,
        layout: Arc<PipelineLayout>,
    ) -> Arc<PersistentDescriptorSet> {
        PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            layout.set_layouts()[set_idx].clone(),
            desc_set,
            [],
        )
        .unwrap()
    }

    pub fn make_subbuffer_allocator(&self, buffer_usage: BufferUsage) -> SubbufferAllocator {
        SubbufferAllocator::new(
            self.memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: buffer_usage,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        )
    }




    // TEXTURING

    pub fn get_texture_descriptor_set_writers(&self) -> Vec<WriteDescriptorSet>{
        let mut writers: Vec<WriteDescriptorSet> = vec![];
        if !self.textures.is_empty() {
            writers.push(WriteDescriptorSet::image_view_sampler_array(1, 0, self.textures.iter().map(|t| (t.clone(), self.texture_sampler.clone()))));
        }
        if !self.texture_arrays.is_empty() {
            writers.push(WriteDescriptorSet::image_view_sampler_array(2, 0, self.texture_arrays.iter().map(|t| (t.clone(), self.texture_sampler.clone()))));
        }
        return writers;
        /*self.textures.iter().enumerate().map(|(i, tex)| {
            WriteDescriptorSet::image_view_sampler(1, tex.clone(), self.texture_sampler.clone())
        }).collect()*/
    }

    fn create_image_load_chain(&mut self, fp_vec: Vec<&str>, img_usages_besides_transfer_dst: ImageUsage) -> (Subbuffer<[u8]>, Arc<Image>, Arc<Image>) {
        // length of the vec will decide the number of array layers
        use image::{ImageBuffer, Rgba, ImageReader, DynamicImage};

        fn load_rgba8(fp: &str) -> ImageBuffer<Rgba<u8>, Vec<u8>> {ImageReader::open(fp).expect(&format!("Failed to load {}", fp)).decode().expect(&format!("Failed to decode {}", fp)).into_rgba8()}

        let mut w: Vec<u32> = vec![];
        let mut h: Vec<u32> = vec![];
        let mut img_array_raw: Vec<u8> = vec![];
        for fp in &fp_vec {
            let img_buffer = load_rgba8(fp);
            w.push(img_buffer.width());
            h.push(img_buffer.height());
            img_array_raw.extend(img_buffer.into_raw());
        }

        assert!(w.iter().min() == w.iter().max());
        assert!(h.iter().min() == h.iter().max()); // checks all w, h the same - necessary for sampler2DArray to work

        let staging_buffer = Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_RANDOM_ACCESS,
                ..Default::default()
            },
            img_array_raw,
        ).expect("failed to create staging buffer");

        let img1 = Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UINT,
                extent: [w[0], h[0], 1],
                usage: ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST,
                mip_levels: 1,
                array_layers: fp_vec.len() as u32,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        ).expect("Failed to create image buffer 1 in create_image_load_chain()");

        let img2 = Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: self.image_format,
                extent: [w[0], h[0], 1],
                usage: img_usages_besides_transfer_dst | ImageUsage::TRANSFER_DST,
                mip_levels: 1,
                array_layers: fp_vec.len() as u32,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        ).expect("Failed to create image buffer 2 in create_image_load_chain()");

        return (staging_buffer, img1, img2);
    }

    pub fn load_texture_arrays(&mut self, fp_vec_vec: Vec<Vec<&str>>, mut texture_upload_builder_stage1: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, mut texture_upload_builder_stage2: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        // Two stages are required due to formatting
        // The images are loaded into memory from disk and forced into RGBA8
        //   Stage 1: Copy images to a device-side Image in R8G8B8A8_UINT
        //   Stage 2: Copy R8G8B8A8_UINT Image to self.image_format Image
        // The Vulkan copy in stage 2 ensures the format converts correctly
        let mut stagetex_tex_pairs: Vec<(Arc<Image>, Arc<Image>)> = vec![];
        for fp_vec in fp_vec_vec {
            let (staging_buffer, staging_texture, texture) = self.create_image_load_chain(fp_vec, ImageUsage::SAMPLED);

            self.texture_arrays.push(ImageView::new_default(texture.clone()).unwrap());
            stagetex_tex_pairs.push((staging_texture, texture.clone()));

            texture_upload_builder_stage1.copy_buffer_to_image(vulkano::command_buffer::CopyBufferToImageInfo::buffer_image(
                staging_buffer.clone(),
                texture.clone(),
            )).unwrap();
        }
        for pair in stagetex_tex_pairs {
            texture_upload_builder_stage2.copy_image(vulkano::command_buffer::CopyImageInfo::images(pair.0, pair.1)).unwrap();
        }
    }

    pub fn load_font_pngs(&mut self, font_folder: &str, mut texture_upload_builder_stage1: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, mut texture_upload_builder_stage2: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        use std::fs;

        let mut stageimg_img_pairs: Vec<(Arc<Image>, Arc<Image>)> = vec![];
        let paths = fs::read_dir(font_folder).unwrap();
        for path in paths {
            let path = path.unwrap().path();
            let char_code: i32 = path.as_path().file_stem().unwrap().to_str().unwrap().parse::<i32>().unwrap_or(0);
            if char_code == 0 {continue;}
            //println!("CHAR CODE: {:?}", char_code);
            if path.metadata().unwrap().is_file() {
                let (staging_buffer, staging_character_image, character_image) = self.create_image_load_chain(vec![path.as_path().to_str().expect("non-unicode path encountered @yovel")], ImageUsage::TRANSFER_SRC);

                self.text_manager.char_images.push(character_image.clone());
                self.text_manager.font_map[char_code as usize] = (self.text_manager.char_images.len()-1).into();

                texture_upload_builder_stage1.copy_buffer_to_image(vulkano::command_buffer::CopyBufferToImageInfo::buffer_image(
                    staging_buffer.clone(),
                    character_image.clone(),
                )).unwrap();
            }
        }
        for pair in stageimg_img_pairs {
            texture_upload_builder_stage2.copy_image(vulkano::command_buffer::CopyImageInfo::images(pair.0, pair.1)).unwrap();
        }
    }
}
