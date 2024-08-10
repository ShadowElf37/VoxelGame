#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex2D {
    pos: [f32; 2],
    uv: [f32; 2],
}

pub struct Font {
    //font_map: [u16; 0xffff],
    //char_images: wgpu::Texture,
}

impl Font {
    pub fn from_ttf(device: &wgpu::Device, queue: &wgpu::Queue, fp: &str) -> Font {
        


        Self {

        }
    }
}



/*
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

impl Font {
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
}*/