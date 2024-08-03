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
use std::sync::Arc;
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

pub struct BufferSet {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    //pub normals_buffer: Subbuffer<[Normal]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub uniform_buffer_allocator: SubbufferAllocator,
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

    pub pipeline: Option<Arc<GraphicsPipeline>>,
    pub buffer_set: Option<Arc<BufferSet>>,
    pub shaders: Vec<EntryPoint>,
    pub textures: Vec<Arc<ImageView>>,
    pub texture_sampler: Arc<Sampler>,
    pub depth_buffer: Arc<ImageView>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
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
        // creates new swapchain. does not create new framebuffers.
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
        )
    }
    fn get_render_pass(device: Arc<Device>, image_format: Format) -> Arc<RenderPass> {
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
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

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
        /*if .iter().all(|x| x.0 != image_format) {
            panic!("Device doesn't support R8G8B8A8_SRGB. WHAT THE FUCK DO I DO :(")
        }*/
        //println!("Using first color format of {:?}", physical_device.surface_formats(&surface, Default::default()).unwrap());

        let (mut swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: capabilities.min_image_count + 1, // How many buffers to use in the swapchain
                image_format: image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT, // What the images are going to be used for
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
                        border_color: BorderColor::FloatOpaqueBlack,
                        ..Default::default()
                    },
                ).unwrap();

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
            textures: vec![],
            texture_sampler: texture_sampler,
            pipeline: None,
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

    pub fn create_pipeline(&mut self) {
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

        self.pipeline = Some(pipeline);
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

    pub fn get_pipeline_layout(&self) -> Arc<PipelineLayout> {
        self.pipeline.clone().unwrap().layout().clone()
    }

    pub fn create_descriptor_set(
        &self,
        set_idx: usize,
        desc: Vec<WriteDescriptorSet>,
    ) -> Arc<PersistentDescriptorSet> {
        let layout = self.get_pipeline_layout();
        PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            layout.set_layouts()[set_idx].clone(),
            desc,
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
        use std::iter::zip;
        vec![WriteDescriptorSet::image_view_sampler_array(1, 0, self.textures.iter().map(|t| (t.clone(), self.texture_sampler.clone())))]
        /*self.textures.iter().enumerate().map(|(i, tex)| {
            WriteDescriptorSet::image_view_sampler(1, tex.clone(), self.texture_sampler.clone())
        }).collect()*/
    }

    pub fn load_texture(&mut self, fp: &str, command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>)-> (Subbuffer<[u8]>, Arc<Image>) {
        use image::{ImageBuffer, Rgba, ImageReader, DynamicImage};
        use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};

        let img = ImageReader::open(fp).expect(&format!("Failed to load {}", fp)).decode().expect(&format!("Failed to decode {}", fp)).into_rgba8();
        let w = img.width();
        let h = img.height();

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
            img.into_raw(),
        ).expect("failed to create staging buffer");

        let texture = Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                extent: [w, h, 1],
                usage: ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
                mip_levels: 1,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        ).expect("Failed to create texture buffer in load_texture()");

        self.textures.push(ImageView::new_default(texture.clone()).unwrap());

        command_builder.copy_buffer_to_image(vulkano::command_buffer::CopyBufferToImageInfo::buffer_image(
            staging_buffer.clone(),
            texture.clone(),
        )).unwrap();
        return (staging_buffer, texture);
    }
}
