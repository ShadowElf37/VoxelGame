use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

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
        let world = world::World::new();

        Game {
            game_state: GameState {
                paused: false,
                in_game: true,
            },

            window: None,
            renderer: None,

            hold_cursor: true,
            cursor_moved_by: (0.0, 0.0), // for macos use only

            world,
            clock: clock::Clock::new(),
        }
    }

    pub fn on_focus(&mut self) {
        let window = self.window.clone().unwrap();
        let renderer = self.renderer.as_ref().unwrap();

        if !self.game_state.paused {
            self.hold_cursor = true;
            window.set_cursor_visible(false);

            #[cfg(target_os = "linux")]
            window.set_cursor_grab(winit::window::CursorGrabMode::Locked);

            window.set_cursor_position(renderer.window_center_px).unwrap();

            #[cfg(target_os = "macos")]
            match window.set_cursor_grab(winit::window::CursorGrabMode::Locked) {
                Ok(_) => {
                    // Successfully set cursor grab mode
                }
                Err(e) => {
                    eprintln!("Failed to set cursor grab mode: {:?}", e);
                }
            }
        }
    }

    pub fn on_defocus(&mut self) {
        let window = self.window.clone().unwrap();
        self.hold_cursor = false;
        window.set_cursor_visible(true);

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        match window.set_cursor_grab(winit::window::CursorGrabMode::None) {
            Ok(_) => {
                // Successfully set cursor grab mode
            }
            Err(e) => {
                eprintln!("Failed to set cursor grab mode: {:?}", e);
            }
        }
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
                if let Ok(player_lock) = self.world.entities.write_lock(self.world.player) {
                    let mut player = player_lock.write().unwrap(); // Dereference the write lock

                    match event {
                        DeviceEvent::MouseMotion { delta } => {
                            if self.game_state.in_game && !self.game_state.paused {
                                player.turn_horizontal(renderer.camera.look_sensitivity * delta.0 as f32);
                                player.turn_vertical(renderer.camera.look_sensitivity * delta.1 as f32);
                            }
                            if !cfg!(target_os = "macos") {
                                if self.hold_cursor {
                                    window.set_cursor_position(renderer.window_center_px).unwrap();
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match (self.window.clone(), &mut self.renderer) {
            (Some(window), Some(renderer)) => {
                match event {
                    WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => {
                        if !self.game_state.paused {
                            if let Ok(player_lock) = self.world.entities.read_lock(self.world.player) {
                                let player = player_lock.read().unwrap(); // Dereference the read lock
                                let (destroy_location, place_location, _) = player.get_block_looking_at(&self.world);
                                match button {
                                    winit::event::MouseButton::Left => { /* Handle left click */ }
                                    winit::event::MouseButton::Right => { /* Handle right click */ }
                                    _ => (),
                                }
                            }
                        }
                    }
                    WindowEvent::KeyboardInput { event: KeyEvent { physical_key, state: ElementState::Pressed, repeat: false, .. }, is_synthetic: false, .. } => {
                        if !self.game_state.paused {
                            if let Ok(player_lock) = self.world.entities.write_lock(self.world.player) {
                                let mut player = player_lock.write().unwrap(); // Dereference the write lock
                                match physical_key {
                                    PhysicalKey::Code(KeyCode::KeyW) => { player.desired_movement.forward = true; }
                                    PhysicalKey::Code(KeyCode::KeyS) => { player.desired_movement.backward = true; }
                                    PhysicalKey::Code(KeyCode::KeyD) => { player.desired_movement.right = true; }
                                    PhysicalKey::Code(KeyCode::KeyA) => { player.desired_movement.left = true; }
                                    PhysicalKey::Code(KeyCode::Space) => { player.desired_movement.up = true; }
                                    PhysicalKey::Code(KeyCode::ShiftLeft) => { player.desired_movement.down = true; }
                                    PhysicalKey::Code(KeyCode::KeyR) => { player.desired_movement.sprint = true; }
                                    _ => (),
                                }
                            }
                        }
                        match physical_key {
                            PhysicalKey::Code(KeyCode::Escape) => {
                                self.game_state.paused = !self.game_state.paused;
                                if !self.game_state.paused {
                                    self.on_focus();
                                } else {
                                    self.on_defocus();
                                }
                            }
                            _ => (),
                        }
                    }
                    WindowEvent::KeyboardInput { event: KeyEvent { physical_key, state: ElementState::Released, repeat: false, .. }, is_synthetic: false, .. } => {
                        if let Ok(player_lock) = self.world.entities.write_lock(self.world.player) {
                            let mut player = player_lock.write().unwrap(); // Dereference the write lock
                            match physical_key {
                                PhysicalKey::Code(KeyCode::KeyW) => { player.desired_movement.forward = false; }
                                PhysicalKey::Code(KeyCode::KeyS) => { player.desired_movement.backward = false; }
                                PhysicalKey::Code(KeyCode::KeyD) => { player.desired_movement.right = false; }
                                PhysicalKey::Code(KeyCode::KeyA) => { player.desired_movement.left = false; }
                                PhysicalKey::Code(KeyCode::Space) => { player.desired_movement.up = false; }
                                PhysicalKey::Code(KeyCode::ShiftLeft) => { player.desired_movement.down = false; }
                                PhysicalKey::Code(KeyCode::KeyR) => { player.desired_movement.sprint = false; }
                                _ => (),
                            }
                        }
                    }
                    WindowEvent::CloseRequested => {
                        println!("User exited.");
                        event_loop.exit();
                    }
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(physical_size);
                    }
                    WindowEvent::Focused(f) => {
                        if f {
                            self.on_focus();
                        } else {
                            self.on_defocus();
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        if let Ok(player_lock) = self.world.entities.read_lock(self.world.player) {
                            let player = player_lock.read().unwrap(); // Dereference the read lock
    
                            self.clock.tick();
    
                            let (looking_at_pos, last_air_pos, looking_at_id) = player.get_block_looking_at(&self.world);
                            let facing = player.facing_in_degrees();
                            renderer.text_manager.set_text_on(
                                0,
                                format!(
                                    "Looking at: {:?}\nLast air: {:?}\nBlock ID: {:?}\nFacing: {:?}\nPaused: {}",
                                    looking_at_pos, last_air_pos, looking_at_id, facing, self.game_state.paused
                                ).as_str()
                            );
                        }
    
                        self.world.physics_step(self.clock.tick_time);
    
                        self.world.update_loaded_chunks();
                        self.world.spawn_chunk_updates();
                        if self.world.spawn_mesh_updates() {
                            self.world.get_all_chunk_meshes(&renderer.device);
                        }
    
                        match renderer.render(&self.world) {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => {
                                let size = renderer.size;
                                renderer.resize(size);
                            }
                            Err(e) => eprintln!("{:?}", e),
                        }
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