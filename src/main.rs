use winit::window::CursorGrabMode;
use winit::event::DeviceId;
use winit::event::DeviceEvent;
use winit::keyboard::PhysicalKey;
use winit::keyboard::KeyCode;
use winit::event::ElementState;
use std::sync::Arc;
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::RenderPassBeginInfo;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::pipeline::Pipeline;
use vulkano::pipeline::PipelineBindPoint;
use vulkano::swapchain::SwapchainPresentInfo;
use vulkano::sync;
use vulkano::sync::GpuFuture;
use vulkano::Validated;
use vulkano::VulkanError;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use winit::dpi::LogicalPosition;

mod camera;
mod engine;
mod vk_select_device;
mod world;
mod entity;
mod clock;

impl ApplicationHandler for engine::Game {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        /*self.window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );*/
        //self.window.clone().request_redraw();
        //println!("Redraw requested!");
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        let mut player = self.world.entities[0].as_mut().unwrap();

        match event {
            DeviceEvent::MouseMotion {delta} => {
                if self.world.game_state.in_game && !self.world.game_state.paused {
                    player.turn_horizontal(self.world.camera.look_sensitivity * delta.0 as f32);
                    player.turn_vertical(-self.world.camera.look_sensitivity * delta.1 as f32);
                }
                if self.hold_cursor {
                    self.window.set_cursor_position(LogicalPosition::new(self.dimensions[0]/2, self.dimensions[1]/2));
                }
            },
            _ => ()
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let mut player = self.world.entities[0].as_mut().unwrap();

        // if in_game {}
        match event {
            WindowEvent::CloseRequested => {
                println!("User exited.");
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                //println!("Resize!");
                self.recreate_swapchain = true;
            }
            WindowEvent::Focused(f) => {
                if f {
                    self.on_focus();
                } else {
                    self.on_defocus();
                }
            }

            WindowEvent::KeyboardInput {event: KeyEvent{physical_key, state: ElementState::Pressed, repeat:false, ..}, is_synthetic: false, ..} => {
                match physical_key {
                    PhysicalKey::Code(KeyCode::KeyW) => {player.desired_movement.FORWARD = true;}
                    PhysicalKey::Code(KeyCode::KeyS) => {player.desired_movement.BACKWARD = true;}
                    PhysicalKey::Code(KeyCode::KeyD) => {player.desired_movement.RIGHT = true;}
                    PhysicalKey::Code(KeyCode::KeyA) => {player.desired_movement.LEFT = true;}
                    PhysicalKey::Code(KeyCode::Space) => {player.desired_movement.UP = true;}
                    PhysicalKey::Code(KeyCode::ShiftLeft) => {player.desired_movement.DOWN = true;}

                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.world.game_state.paused {
                            self.on_focus();
                        } else {
                            self.on_defocus();
                        }
                        self.world.game_state.paused = !self.world.game_state.paused;             
                    }
                    _ => ()
                }
            }
            WindowEvent::KeyboardInput {event: KeyEvent{physical_key, state: ElementState::Released, repeat:false, ..}, is_synthetic: false, ..} => {
                match physical_key {
                    PhysicalKey::Code(KeyCode::KeyW) => {player.desired_movement.FORWARD = false;}
                    PhysicalKey::Code(KeyCode::KeyS) => {player.desired_movement.BACKWARD = false;}
                    PhysicalKey::Code(KeyCode::KeyD) => {player.desired_movement.RIGHT = false;}
                    PhysicalKey::Code(KeyCode::KeyA) => {player.desired_movement.LEFT = false;}
                    PhysicalKey::Code(KeyCode::Space) => {player.desired_movement.UP = false;}
                    PhysicalKey::Code(KeyCode::ShiftLeft) => {player.desired_movement.DOWN = false;}
                    _ => ()
                }
            }
            
            WindowEvent::RedrawRequested => {
                self.clock.tick();
                self.world.physics_step(self.clock.frame_time);

                let player = self.world.entities[0].as_ref().unwrap();
                println!(
                    "Frame:{} Time:{:.3} Fps:{:.1} | X:{:.2} Y:{:.2} Z:{:.2} | W:{} H:{}",
                    self.clock.frame, self.clock.time, self.clock.fps, player.pos.x, player.pos.y, player.pos.z, self.dimensions[0], self.dimensions[1],
                );

                if self.recreate_swapchain {
                    //println!("Swapchain regenerating!");
                    self.recreate_swapchain = false;

                    self.recreate_swapchain();
                    self.create_framebuffers();
                    self.create_pipeline();
                }

                // SET UP USER DATA TO PUSH TO GPU
                let projdata_buffer = {
                    let proj = self.world.camera.get_proj_mat(self.aspect_ratio);
                    let view = self.world.camera.get_view_mat(self.world.get_camera_entity());
                    let uniform_data = vs::Data {
                        //facing: self.world.get_camera_entity().as_ref().unwrap().facing.extend(0.0).to_array(),
                        //cam_pos: self.world.get_camera_entity().as_ref().unwrap().pos.extend(0.0).to_array(),
                        view: view.to_cols_array_2d(),
                        proj: proj.to_cols_array_2d(),
                    };

                    let subbuffer = self.buffer_set.clone().unwrap().uniform_buffer_allocator.allocate_sized().unwrap();
                    *subbuffer.write().unwrap() = uniform_data;

                    subbuffer
                };

                // CREATE DESCRIPTOR_SET NEEDED TO ATTACH THE projdata_buffer
                // can also be used to attach other buffers (vert and norm are handled directly in the command buffer)
                let desc_set = self
                    .create_descriptor_set(0, vec![WriteDescriptorSet::buffer(0, projdata_buffer)]);

                // acquire next image in the swapchain
                let (acquired_image_index, suboptimal, acquire_image_future) =
                    match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None)
                        .map_err(Validated::unwrap)
                    {
                        Ok(r) => r,
                        Err(VulkanError::OutOfDate) => {
                            self.recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("failed to acquire next image: {e}"),
                    };
                if suboptimal {
                    self.recreate_swapchain = true;
                }

                let buffer_set = self.buffer_set.clone().unwrap();

                // build command buffer
                let mut builder = self.create_command_buffer_builder();
                builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            clear_values: vec![
                                Some([0.0, 0.0, 0.0, 1.0].into()),
                                Some(1f32.into()),
                            ],
                            ..RenderPassBeginInfo::framebuffer(
                                self.framebuffers[acquired_image_index as usize].clone(),
                            )
                        },
                        Default::default(),
                    )
                    .unwrap()
                    .bind_pipeline_graphics(self.pipeline.clone().unwrap())
                    .unwrap()
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        self.get_pipeline_layout(),
                        0,
                        desc_set,
                    )
                    .unwrap()
                    .bind_vertex_buffers(0, buffer_set.vertex_buffer.clone())//, buffer_set.normals_buffer.clone()))
                    .unwrap()
                    .bind_index_buffer(buffer_set.index_buffer.clone())
                    .unwrap();

                unsafe {
                    builder
                        .draw_indexed(buffer_set.index_buffer.len() as u32, 1, 0, 0, 0)
                        .unwrap();
                }

                builder.end_render_pass(Default::default()).unwrap();
                let command_buffer = builder.build().unwrap();

                // FUTURES
                // FUTURES
                // FUTURES
                // FUTURES

                let commands_future = sync::now(self.device.clone())
                    .join(acquire_image_future)
                    .then_execute(self.queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(
                        self.queue.clone(),
                        SwapchainPresentInfo::swapchain_image_index(
                            self.swapchain.clone(),
                            acquired_image_index,
                        ),
                    )
                    .then_signal_fence_and_flush();

                match commands_future.map_err(Validated::unwrap) {
                    Ok(future) => {
                        // Wait for the GPU to finish.
                        future.wait(None).unwrap();
                    }
                    Err(vulkano::VulkanError::OutOfDate) => {
                        self.recreate_swapchain = true;
                    }
                    Err(e) => {
                        println!("failed to flush future: {e}");
                    }
                }

                self.window.clone().request_redraw();
            }
            _ => (),
        }
    }
}


// MAIN

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/vert.glsl"
    }
}
mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/frag.glsl"
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().expect("Failed to even create an event loop!");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut vk = engine::Game::new(&event_loop);
    vk.window.set_title("Minecraft");

    println!("{:?}", vk.window.inner_size());

    vk.world.spawn_at_sp();

    let vs = vs::load(vk.device.clone()).expect("failed to create vs module (your fault)").entry_point("main").unwrap();
    let fs = fs::load(vk.device.clone()).expect("failed to create fs module (your fault)").entry_point("main").unwrap();
    vk.push_shader(vs);
    vk.push_shader(fs);

    let vertices = vec![
        engine::Vertex {
            position: [0.0, 0.0, 1.0],
        },
        engine::Vertex {
            position: [0.0, 0.0, 0.0],
        },
        engine::Vertex {
            position: [1.0, 0.0, 0.0],
        },
        engine::Vertex {
            position: [1.0, 0.0, 1.0],
        },
    ];

    let indices: Vec<u32> = vec![0, 1, 2, 2, 3, 0];;
    //vec![0, 1, 2, 2, 3, 0];

    let buffer_set = engine::BufferSet {
        vertex_buffer: vk.alloc_vertex_buffer(vertices),
        //normals_buffer: vk.alloc_vertex_buffer(normals),
        index_buffer: vk.alloc_buffer_from_vector(indices, BufferUsage::INDEX_BUFFER, true),
        uniform_buffer_allocator: vk.make_subbuffer_allocator(BufferUsage::UNIFORM_BUFFER),
    };
    vk.buffer_set = Some(Arc::new(buffer_set));

    vk.create_pipeline();

    event_loop.run_app(&mut vk);
}
