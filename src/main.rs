use winit::event::KeyEvent;
use winit::event::ElementState;
use winit::keyboard::PhysicalKey;
use winit::keyboard::KeyCode;
use winit::event::DeviceId;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, DeviceEvent, Event};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use glam::{Vec3};
use crate::chunk::CHUNK_SIZE_F;

mod renderer;
mod geometry;
mod world;
mod clock;
mod entity;
mod camera;
mod texturing;
mod block;
mod chunk;
mod memarena;

#[derive(Default)]
pub struct GameState {
    pub paused: bool,
    pub in_game: bool,
}

struct Game<'a> {
    pub game_state: GameState,

    window: Option<Arc<Window>>,
    renderer: Option<renderer::Renderer<'a>>,

    hold_cursor: bool,
    cursor_moved_by: (f64, f64),

    world: world::World,
    clock: clock::Clock,
}

impl Game<'_> {
    pub async fn new(event_loop: &EventLoop<()>) -> Self {
        //let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());

        let world = world::World::new();

        Game {
            game_state: GameState {
                paused: false,
                in_game: true,
            },

            window: None,
            renderer: None,

            hold_cursor: true,
            cursor_moved_by:  (0.0, 0.0), // for macos use only

            world,
            clock: clock::Clock::new(),
        }
    }

    // MARK: UPDATE
    pub fn update(&mut self, dt: f32) {
        // Extract player position before any mutable borrow
        let player_pos = {
            let player_entity = self.world.entities.read_lock(self.world.player).unwrap();
            player_entity.pos
        };
    
        // These methods require mutable access to the player entity
        self.world.load_chunks_around_player();
        self.world.unload_chunks_outside_radius();
        self.world.physics_step(dt);
    }

    pub fn on_focus(&mut self) {
        let window = self.window.clone().unwrap();
        let renderer = self.renderer.as_ref().unwrap();

        if !self.game_state.paused {
            self.hold_cursor = true;
            window.set_cursor_visible(false);

            #[cfg(any(target_os = "macos", target_os = "linux"))]
            window.set_cursor_grab(winit::window::CursorGrabMode::Locked);

            window.set_cursor_position(renderer.window_center_px).unwrap();
        }
    }
    pub fn on_defocus(&mut self) {
        let window = self.window.clone().unwrap();
        self.hold_cursor = false;
        window.set_cursor_visible(true);

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        window.set_cursor_grab(winit::window::CursorGrabMode::None);
    }
}

impl ApplicationHandler for Game<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let t = std::time::Instant::now();

        println!("Initializing window...");
        let window = event_loop.create_window(Window::default_attributes()).unwrap();
        window.set_title("Minecraft");
        window.request_redraw();
        window.focus_window();
        self.window = Some(Arc::new(window));

        println!("Initializing renderer... ({:.2?})", t.elapsed());
        let mut renderer = pollster::block_on(renderer::Renderer::new(self.window.clone().unwrap()));
        renderer.load_texture_set(self.world.block_properties.collect_textures());
        
        println!("Generating chunks... ({:.2?})", t.elapsed());
        self.world.generate_all_chunks_around_player();

        println!("Done! ({:.2?})", t.elapsed());

        self.renderer = Some(renderer);
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        match (self.window.clone(), &mut self.renderer) {
            (Some(window), Some(renderer)) => {
                let mut player = self.world.entities.write_lock(self.world.player).unwrap();

                match event {
                    DeviceEvent::MouseMotion {delta} => {
                        if self.game_state.in_game && !self.game_state.paused {
                            player.turn_horizontal(renderer.camera.look_sensitivity * delta.0 as f32);
                            player.turn_vertical(renderer.camera.look_sensitivity * delta.1 as f32);
                        }
                        if !cfg!(target_os = "macos") {
                            
                            if self.hold_cursor {
                                window.set_cursor_position(renderer.window_center_px).unwrap();
                            }
                        }
                        //println!("Mouse moved: {:?} {} {} {}", delta, self.game_state.in_game, self.game_state.paused, self.hold_cursor);
                    },
                    _ => ()
                }
            }
            _ => ()
        }

        
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match (self.window.clone(), &mut self.renderer) {
            (Some(window), Some(renderer)) => {

                match event {
                    //WindowEvent::CursorMoved { position, .. } => { }

                    WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => {
                        if !self.game_state.paused {
                            let (destroy_location, place_location, _) = self.world.entities.read_lock(self.world.player).unwrap().get_block_looking_at(&self.world);
                            match button {
                                winit::event::MouseButton::Left => {
                                    self.world.set_block_id_at(destroy_location.x, destroy_location.y, destroy_location.z, 0);
                                },
                                winit::event::MouseButton::Right => {
                                    let player_pos = self.world.entities.read_lock(self.world.player).unwrap().pos.floor();
                                    if place_location != player_pos && place_location != player_pos + Vec3::Z{
                                        self.world.set_block_id_at(place_location.x, place_location.y, place_location.z, 6);
                                    }
                                },
                                winit::event::MouseButton::Middle => (),
                                _ => ()
                            }
                        }
                    }

                    WindowEvent::KeyboardInput {event: KeyEvent{physical_key, state: ElementState::Pressed, repeat:false, ..}, is_synthetic: false, ..} => {
                        if !self.game_state.paused {
                            let mut player = self.world.entities.write_lock(self.world.player).unwrap();
                            match physical_key {
                                PhysicalKey::Code(KeyCode::KeyW) => {player.desired_movement.FORWARD = true;}
                                PhysicalKey::Code(KeyCode::KeyS) => {player.desired_movement.BACKWARD = true;}
                                PhysicalKey::Code(KeyCode::KeyD) => {player.desired_movement.RIGHT = true;}
                                PhysicalKey::Code(KeyCode::KeyA) => {player.desired_movement.LEFT = true;}
                                PhysicalKey::Code(KeyCode::Space) => {player.desired_movement.UP = true;}
                                PhysicalKey::Code(KeyCode::ShiftLeft) => {player.desired_movement.DOWN = true;}
                                PhysicalKey::Code(KeyCode::KeyR) => {player.desired_movement.SPRINT = true;}
                                _ => ()
                            }
                        }
                        match physical_key {
                            PhysicalKey::Code(KeyCode::Escape) => {
                                self.game_state.paused = !self.game_state.paused;  
                                if !self.game_state.paused { // inverse because we unpaused on the line above. necessary because on_focus queries pause state
                                    self.on_focus();
                                } else {
                                    self.on_defocus();
                                }
                            }
                            _ => ()
                        }
                    }

                    WindowEvent::KeyboardInput {event: KeyEvent{physical_key, state: ElementState::Released, repeat:false, ..}, is_synthetic: false, ..} => {
                        let mut player = self.world.entities.write_lock(self.world.player).unwrap();
                        match physical_key {
                            PhysicalKey::Code(KeyCode::KeyW) => {player.desired_movement.FORWARD = false;}
                            PhysicalKey::Code(KeyCode::KeyS) => {player.desired_movement.BACKWARD = false;}
                            PhysicalKey::Code(KeyCode::KeyD) => {player.desired_movement.RIGHT = false;}
                            PhysicalKey::Code(KeyCode::KeyA) => {player.desired_movement.LEFT = false;}
                            PhysicalKey::Code(KeyCode::Space) => {player.desired_movement.UP = false;}
                            PhysicalKey::Code(KeyCode::ShiftLeft) => {player.desired_movement.DOWN = false;}
                            PhysicalKey::Code(KeyCode::KeyR) => {player.desired_movement.SPRINT = false;}
                            _ => ()
                        }
                    }

                    WindowEvent::CloseRequested => {
                        println!("User exited.");
                        event_loop.exit();
                    },

                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(physical_size);
                    }

                    //WindowEvent::ScaleFactorChanged{scale_factor, ..} => {
                    //    renderer.ui_scale = scale_factor as f32;
                    //}

                    WindowEvent::Focused(f) => {
                        if f {
                            self.on_focus();
                        } else {
                            self.on_defocus();
                        }
                    }
                    
                    WindowEvent::RedrawRequested => {
                        let player = self.world.entities.read_lock(self.world.player).unwrap();
                    
                        self.clock.tick();
                    
                        if true {
                            let (looking_at_pos, last_air_pos, looking_at_id) = player.get_block_looking_at(&self.world);
                            let facing = player.facing_in_degrees();
                            
                            // Calculate chunk coordinates
                            let chunk_x = (player.pos.x / CHUNK_SIZE_F).floor() as i32;
                            let chunk_y = (player.pos.y / CHUNK_SIZE_F).floor() as i32;
                            let chunk_z = (player.pos.z / CHUNK_SIZE_F).floor() as i32;
                        
                            renderer.text_manager.set_text_on(
                                0,
                                format!(
                                    "Frame={} Time={:.1} FPS={:.1}\nX=({:.2}, {:.2}, {:.2})\nV=({:.2}, {:.2}, {:.2})\nφ={:.0}° ϴ={:.0}°\nLooking: {} ({:.0}, {:.0}, {:.0})\nChunk: ({}, {}, {})\nW={} H={}\nPAUSED = {}",
                                    self.clock.tick, self.clock.time, self.clock.tps,
                                    player.pos.x, player.pos.y, player.pos.z,
                                    player.vel.x, player.vel.y, player.vel.z,
                                    facing.x, facing.y,
                                    self.world.block_properties.by_id(looking_at_id).name, looking_at_pos.x, looking_at_pos.y, looking_at_pos.z,
                                    chunk_x, chunk_y, chunk_z, // Add chunk coordinates here
                                    renderer.size.width, renderer.size.height,
                                    self.game_state.paused
                                ).as_str()
                            );
                        }
                    
                        drop(player); // Drop the player borrow here
                    
                        // Store tick_time in a local variable
                        let tick_time = self.clock.tick_time;
                    
                        // MARK:
                        // Call update with the local tick_time variable
                        self.update(tick_time);
                    
                        // Spawn chunk updates
                        self.world.spawn_chunk_updates();
                        
                        // Spawn mesh updates and get all chunk meshes if necessary
                        if self.world.spawn_mesh_updates() {
                            self.world.get_all_chunk_meshes(&renderer.device);
                        }
                    
                        // Get the window size
                        let size = window.inner_size();
                    
                        // Render the world
                        match renderer.render(&self.world) {
                            Ok(_) => {}
                            // Reconfigure the surface if lost
                            Err(wgpu::SurfaceError::Lost) => {
                                renderer.resize(size);
                            },
                            // All other errors (Outdated, Timeout) should be resolved by the next frame
                            Err(e) => eprintln!("{:?}", e),
                        }
                    
                        // Request a redraw
                        window.request_redraw();
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }

}


fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut game = pollster::block_on(Game::new(&event_loop));
    
    event_loop.run_app(&mut game).unwrap();
}