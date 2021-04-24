mod audio;

use crate::{
    gl,
    input::{InputEvent, Key, MouseButton},
};

use euclid::{point2, vec2};
use glutin::event::{
    ElementState, KeyboardInput, MouseButton as GlutinMouseButton, MouseScrollDelta, VirtualKeyCode,
};

pub use audio::start_audio_playback;

#[cfg(not(target_arch = "wasm32"))]
pub fn run<
    F: Fn(&mut gl::Context) -> U,
    U: FnMut(f32, &[InputEvent], &mut gl::Context) + 'static,
>(
    title: &str,
    size: (u32, u32),
    f: F,
) {
    use glutin::{
        event,
        event::WindowEvent,
        event_loop::{ControlFlow, EventLoop},
    };
    use std::time::Instant;

    env_logger::init();
    let event_loop = EventLoop::new();
    let mut wb = glutin::window::WindowBuilder::new();
    wb = wb
        .with_title(title)
        .with_inner_size(glutin::dpi::LogicalSize::new(size.0, size.1))
        .with_resizable(false);
    let windowed_context = unsafe {
        glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGlEs, (2, 0)))
            .build_windowed(wb, &event_loop)
            .unwrap()
            .make_current()
            .unwrap()
    };

    let mut gl_context =
        gl::Context::from_glow_context(glow::Context::from_loader_function(|addr| {
            windowed_context.get_proc_address(addr)
        }));

    let mut update_fn = f(&mut gl_context);

    let mut input_events = Vec::new();
    let mut last_time = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            event::Event::MainEventsCleared => windowed_context.window().request_redraw(),
            event::Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                log::info!("Resize to {:?}", size);
            }
            event::Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(key),
                            state,
                            ..
                        },
                    ..
                } => {
                    if let Some(key) = get_key(key) {
                        match state {
                            ElementState::Pressed => {
                                input_events.push(InputEvent::KeyDown(key));
                            }
                            ElementState::Released => {
                                input_events.push(InputEvent::KeyUp(key));
                            }
                        }
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let button = get_mouse_button(button);
                    match state {
                        ElementState::Pressed => {
                            input_events.push(InputEvent::MouseDown(button));
                        }
                        ElementState::Released => {
                            input_events.push(InputEvent::MouseUp(button));
                        }
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        input_events.push(InputEvent::MouseWheel(vec2(x, y)));
                    }
                    MouseScrollDelta::PixelDelta(p) => {
                        input_events.push(InputEvent::MouseWheel(vec2(p.x as f32, p.y as f32)));
                    }
                },
                WindowEvent::CursorMoved { position, .. } => {
                    let position = position.to_logical(1.0);
                    input_events.push(InputEvent::MouseMove(point2(position.x, position.y)));
                }
                _ => {}
            },
            event::Event::RedrawRequested(_) => {
                let now = Instant::now();
                let dt = (now - last_time).as_micros() as f32 / 1_000_000.;
                last_time = now;
                update_fn(dt, &input_events, &mut gl_context);
                input_events.clear();
                windowed_context.swap_buffers().unwrap();
                unsafe { gl_context.maintain() };
            }
            _ => {}
        }
    });
}

fn get_key(vk: VirtualKeyCode) -> Option<Key> {
    match vk {
        VirtualKeyCode::A => Some(Key::A),
        VirtualKeyCode::B => Some(Key::B),
        VirtualKeyCode::C => Some(Key::C),
        VirtualKeyCode::D => Some(Key::D),
        VirtualKeyCode::E => Some(Key::E),
        VirtualKeyCode::F => Some(Key::F),
        VirtualKeyCode::G => Some(Key::G),
        VirtualKeyCode::H => Some(Key::H),
        VirtualKeyCode::I => Some(Key::I),
        VirtualKeyCode::J => Some(Key::J),
        VirtualKeyCode::K => Some(Key::K),
        VirtualKeyCode::L => Some(Key::L),
        VirtualKeyCode::M => Some(Key::M),
        VirtualKeyCode::N => Some(Key::N),
        VirtualKeyCode::O => Some(Key::O),
        VirtualKeyCode::P => Some(Key::P),
        VirtualKeyCode::Q => Some(Key::Q),
        VirtualKeyCode::R => Some(Key::R),
        VirtualKeyCode::S => Some(Key::S),
        VirtualKeyCode::T => Some(Key::T),
        VirtualKeyCode::U => Some(Key::U),
        VirtualKeyCode::V => Some(Key::V),
        VirtualKeyCode::W => Some(Key::W),
        VirtualKeyCode::X => Some(Key::X),
        VirtualKeyCode::Y => Some(Key::Y),
        VirtualKeyCode::Z => Some(Key::Z),
        VirtualKeyCode::Space => Some(Key::Space),
        VirtualKeyCode::Back => Some(Key::Backspace),
        VirtualKeyCode::Return => Some(Key::Return),
        VirtualKeyCode::Escape => Some(Key::Escape),
        VirtualKeyCode::Slash => Some(Key::Slash),
        VirtualKeyCode::Home => Some(Key::Home),
        VirtualKeyCode::Delete => Some(Key::Delete),
        VirtualKeyCode::End => Some(Key::End),
        VirtualKeyCode::Left => Some(Key::Left),
        VirtualKeyCode::Up => Some(Key::Up),
        VirtualKeyCode::Right => Some(Key::Right),
        VirtualKeyCode::Down => Some(Key::Down),
        _ => None,
    }
}

fn get_mouse_button(button: GlutinMouseButton) -> MouseButton {
    match button {
        GlutinMouseButton::Left => MouseButton::Left,
        GlutinMouseButton::Middle => MouseButton::Middle,
        GlutinMouseButton::Right => MouseButton::Right,
        GlutinMouseButton::Other(b) => MouseButton::Other(b),
    }
}
