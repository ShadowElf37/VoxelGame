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

mod renderer;
mod geometry;
mod world;
mod clock;
mod entity;
mod camera;
mod texturing;
mod ui;

#[derive(Default)]
pub struct GameState {
    pub paused: bool,
    pub in_game: bool,
}

struct Game<'a> {
    pub game_state: GameState,

    window: Arc<Window>,
    hold_cursor: bool,
    renderer: renderer::Renderer<'a>,
    world: world::World,
    clock: clock::Clock,
}

impl Game<'_> {
    pub async fn new(event_loop: &EventLoop<()>) -> Self {
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
        window.set_title("Minecraft");

        Game {
            game_state: GameState {
                paused: false,
                in_game: true,
            },

            window: window.clone(),
            hold_cursor: true,
            renderer: renderer::Renderer::new(window.clone()).await,
            world: world::World::new(),
            clock: clock::Clock::new(),
        }
    }

    pub fn on_focus(&mut self) {
        self.hold_cursor = true;
        self.window.set_cursor_visible(false);
    }
    pub fn on_defocus(&mut self) {
        self.hold_cursor = false;
        self.window.set_cursor_visible(true);
    }
}

impl ApplicationHandler for Game<'_> {
    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        let player = &mut self.world.player;

        match event {
            DeviceEvent::MouseMotion {delta} => {
                //println!("Mouse moved: {:?} {} {} {}", delta, self.game_state.in_game, self.game_state.paused, self.hold_cursor);
                if self.game_state.in_game && !self.game_state.paused {
                    player.turn_horizontal(self.renderer.camera.look_sensitivity * delta.0 as f32);
                    player.turn_vertical(self.renderer.camera.look_sensitivity * delta.1 as f32);
                }
                if self.hold_cursor {
                    self.window.set_cursor_position(winit::dpi::LogicalPosition::new(self.renderer.size.width/2, self.renderer.size.height/2)).unwrap();
                }
            },
            _ => ()
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let player = &mut self.world.player;
        match event {
            WindowEvent::KeyboardInput {event: KeyEvent{physical_key, state: ElementState::Pressed, repeat:false, ..}, is_synthetic: false, ..} => {
                match physical_key {
                    PhysicalKey::Code(KeyCode::KeyW) => {player.desired_movement.FORWARD = true;}
                    PhysicalKey::Code(KeyCode::KeyS) => {player.desired_movement.BACKWARD = true;}
                    PhysicalKey::Code(KeyCode::KeyD) => {player.desired_movement.RIGHT = true;}
                    PhysicalKey::Code(KeyCode::KeyA) => {player.desired_movement.LEFT = true;}
                    PhysicalKey::Code(KeyCode::Space) => {player.desired_movement.UP = true;}
                    PhysicalKey::Code(KeyCode::ShiftLeft) => {player.desired_movement.DOWN = true;}

                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.game_state.paused {
                            self.on_focus();
                        } else {
                            self.on_defocus();
                        }
                        self.game_state.paused = !self.game_state.paused;             
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

            WindowEvent::CloseRequested => {
                println!("User exited.");
                event_loop.exit();
            },
            WindowEvent::Resized(physical_size) => {
                self.renderer.resize(physical_size);
            }
            WindowEvent::ScaleFactorChanged{scale_factor, ..} => {
                self.renderer.ui_scale = scale_factor as f32;
            }
            WindowEvent::Focused(f) => {
                if f {
                    self.on_focus();
                } else {
                    self.on_defocus();
                }
            }
            // ...
            WindowEvent::RedrawRequested => {
                //self.renderer.update();
                self.clock.tick();
                self.renderer.text_manager.set_text_on(
                    0,
                    format!(
                        "Frame:{} Time:{:.3} Fps:{:.1} | X:{:.2} Y:{:.2} Z:{:.2} | W:{} H:{}",
                        self.clock.tick, self.clock.time, self.clock.tps, player.pos.x, player.pos.y, player.pos.z, self.renderer.size.width, self.renderer.size.height,
                    ).as_str()
                );
                
                self.world.physics_step(self.clock.tick_time);

                match self.renderer.render(&self.world) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => self.renderer.resize(self.renderer.size),
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
                self.window.request_redraw();
            }
            _ => (),
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {}
}

async fn run() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut game = Game::new(&event_loop).await;
    game.renderer.load_texture_set(vec![
        r"assets\textures\cobblestone.png"
    ]);
    event_loop.run_app(&mut game).unwrap();
}

fn main() {
    pollster::block_on(run());
}