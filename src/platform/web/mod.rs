mod audio;

use std::rc::Rc;

use euclid::{point2, vec2};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{HtmlElement, KeyboardEvent, MouseEvent, WheelEvent};

use crate::{
    gl,
    input::{InputEvent, Key, MouseButton},
};

pub use audio::start_audio_playback;

pub fn run<
    F: Fn(&mut gl::Context) -> U,
    U: FnMut(f32, &[InputEvent], &mut gl::Context) + 'static,
>(
    title: &str,
    size: (u32, u32),
    f: F,
) {
    use std::cell::RefCell;

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).unwrap();

    let document = web_sys::window()
        .and_then(|win| win.document())
        .expect("Cannot get document");
    document.set_title(title);

    let canvas = document
        .create_element("canvas")
        .expect("Cannot create canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("Cannot get canvas element");
    document
        .body()
        .expect("Cannot get document body")
        .append_child(&canvas)
        .expect("Cannot insert canvas into document body");
    canvas
        .set_attribute("width", &format!("{}", size.0))
        .expect("cannot set width");
    canvas
        .set_attribute("height", &format!("{}", size.1))
        .expect("cannot set height");

    let webgl1_context = canvas
        .get_context("webgl")
        .expect("1")
        .expect("2")
        .dyn_into::<web_sys::WebGlRenderingContext>()
        .expect("3");

    let glow_context = glow::Context::from_webgl1_context(webgl1_context);
    let mut gl_context = gl::Context::from_glow_context(glow_context);

    let mut update_fn = f(&mut gl_context);

    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g = Rc::clone(&f);
    let mut last_time = None;

    let input_events = Rc::new(RefCell::new(Vec::new()));

    let input_stream = HtmlEventStream::new(canvas.clone().dyn_into().unwrap(), {
        let input_events = Rc::clone(&input_events);
        move |window_event| match window_event {
            HtmlEvent::KeyDown(key_event) => {
                if let Some(key) = get_key_from_code(&key_event.code()) {
                    input_events.borrow_mut().push(InputEvent::KeyDown(key));
                }
            }
            HtmlEvent::KeyUp(key_event) => {
                if let Some(key) = get_key_from_code(&key_event.code()) {
                    input_events.borrow_mut().push(InputEvent::KeyUp(key));
                }
            }
            HtmlEvent::MouseDown(mouse_event) => {
                input_events
                    .borrow_mut()
                    .push(InputEvent::MouseDown(get_mouse_button(
                        mouse_event.button(),
                    )));
            }
            HtmlEvent::MouseUp(mouse_event) => {
                input_events
                    .borrow_mut()
                    .push(InputEvent::MouseUp(get_mouse_button(mouse_event.button())));
            }
            HtmlEvent::MouseMove(mouse_event) => {
                input_events.borrow_mut().push(InputEvent::MouseMove(point2(
                    mouse_event.offset_x() as f32,
                    mouse_event.offset_y() as f32,
                )));
            }
            HtmlEvent::MouseWheel(wheel_event) => {
                input_events.borrow_mut().push(InputEvent::MouseWheel(vec2(
                    wheel_event.delta_x() as f32,
                    -wheel_event.delta_y() as f32,
                )));
            }
        }
    });

    wasm_bindgen_futures::spawn_local(async move {
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64| {
            // Keep input_stream alive for the lifetime of the client
            let _ = &input_stream;

            let dt = (time - last_time.unwrap_or(time)) / 1000.;
            update_fn(dt as f32, &input_events.borrow(), &mut gl_context);
            input_events.borrow_mut().clear();
            last_time = Some(time);

            web_sys::window()
                .expect("no global window")
                .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
                .expect("could not request animation frame");
        }) as Box<dyn FnMut(f64)>));

        web_sys::window()
            .expect("no global window")
            .request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .expect("could not request animation frame");
    })
}

pub enum HtmlEvent {
    KeyDown(KeyboardEvent),
    KeyUp(KeyboardEvent),
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    MouseMove(MouseEvent),
    MouseWheel(WheelEvent),
}

/// Multiplexes different window-level input events into a single callback, automatically removing
/// and cleaning up event handlers on drop.
pub struct HtmlEventStream {
    mouse_element: HtmlElement,
    _on_key_down: Closure<dyn FnMut(KeyboardEvent)>,
    _on_key_up: Closure<dyn FnMut(KeyboardEvent)>,
    _on_mouse_down: Closure<dyn FnMut(MouseEvent)>,
    _on_mouse_up: Closure<dyn FnMut(MouseEvent)>,
    _on_mouse_move: Closure<dyn FnMut(MouseEvent)>,
    _on_mouse_wheel: Closure<dyn FnMut(WheelEvent)>,
}

impl HtmlEventStream {
    /// Handled input events will result in a call of the given callback until the returned
    /// `InputStream` is dropped.
    ///
    /// Mouse events are handled at the element level on the given `mouse_element`, key events are
    /// handled at the window level.
    pub fn new(
        mouse_element: HtmlElement,
        callback: impl Fn(HtmlEvent) + 'static,
    ) -> HtmlEventStream {
        let callback = Rc::new(callback);

        let on_key_down = Closure::wrap(Box::new({
            let callback = Rc::clone(&callback);
            move |keyboard_event| {
                callback(HtmlEvent::KeyDown(keyboard_event));
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);

        let on_key_up = Closure::wrap(Box::new({
            let callback = Rc::clone(&callback);
            move |keyboard_event| {
                callback(HtmlEvent::KeyUp(keyboard_event));
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);

        let on_mouse_down = Closure::wrap(Box::new({
            let callback = Rc::clone(&callback);
            move |mouse_event| {
                callback(HtmlEvent::MouseDown(mouse_event));
            }
        }) as Box<dyn FnMut(MouseEvent)>);

        let on_mouse_up = Closure::wrap(Box::new({
            let callback = Rc::clone(&callback);
            move |mouse_event| {
                callback(HtmlEvent::MouseUp(mouse_event));
            }
        }) as Box<dyn FnMut(MouseEvent)>);

        let on_mouse_wheel = Closure::wrap(Box::new({
            let callback = Rc::clone(&callback);
            move |wheel_event| {
                callback(HtmlEvent::MouseWheel(wheel_event));
            }
        }) as Box<dyn FnMut(WheelEvent)>);

        let on_mouse_move = Closure::wrap(Box::new({
            let callback = Rc::clone(&callback);
            move |mouse_event| {
                callback(HtmlEvent::MouseMove(mouse_event));
            }
        }) as Box<dyn FnMut(MouseEvent)>);

        let window = web_sys::window().unwrap();
        window.set_onkeydown(Some(on_key_down.as_ref().unchecked_ref()));
        window.set_onkeyup(Some(on_key_up.as_ref().unchecked_ref()));
        mouse_element.set_onmousedown(Some(on_mouse_down.as_ref().unchecked_ref()));
        mouse_element.set_onmouseup(Some(on_mouse_up.as_ref().unchecked_ref()));
        mouse_element.set_onmousemove(Some(on_mouse_move.as_ref().unchecked_ref()));
        mouse_element.set_onwheel(Some(on_mouse_wheel.as_ref().unchecked_ref()));

        HtmlEventStream {
            mouse_element,
            _on_key_down: on_key_down,
            _on_key_up: on_key_up,
            _on_mouse_down: on_mouse_down,
            _on_mouse_up: on_mouse_up,
            _on_mouse_move: on_mouse_move,
            _on_mouse_wheel: on_mouse_wheel,
        }
    }
}

impl Drop for HtmlEventStream {
    fn drop(&mut self) {
        let window = web_sys::window().unwrap();
        window.set_onkeydown(None);
        window.set_onkeyup(None);
        self.mouse_element.set_onmousedown(None);
        self.mouse_element.set_onmouseup(None);
        self.mouse_element.set_onmousemove(None);
    }
}

fn get_key_from_code(key: &str) -> Option<Key> {
    match key {
        "KeyA" => Some(Key::A),
        "KeyB" => Some(Key::B),
        "KeyC" => Some(Key::C),
        "KeyD" => Some(Key::D),
        "KeyE" => Some(Key::E),
        "KeyF" => Some(Key::F),
        "KeyG" => Some(Key::G),
        "KeyH" => Some(Key::H),
        "KeyI" => Some(Key::I),
        "KeyJ" => Some(Key::J),
        "KeyK" => Some(Key::K),
        "KeyL" => Some(Key::L),
        "KeyM" => Some(Key::M),
        "KeyN" => Some(Key::N),
        "KeyO" => Some(Key::O),
        "KeyP" => Some(Key::P),
        "KeyQ" => Some(Key::Q),
        "KeyR" => Some(Key::R),
        "KeyS" => Some(Key::S),
        "KeyT" => Some(Key::T),
        "KeyU" => Some(Key::U),
        "KeyV" => Some(Key::V),
        "KeyW" => Some(Key::W),
        "KeyX" => Some(Key::X),
        "KeyY" => Some(Key::Y),
        "KeyZ" => Some(Key::Z),
        "Space" => Some(Key::Space),
        "Backspace" => Some(Key::Backspace),
        "Enter" => Some(Key::Return),
        "Escape" => Some(Key::Escape),
        "Home" => Some(Key::Home),
        "Delete" => Some(Key::Delete),
        "End" => Some(Key::End),
        "Slash" => Some(Key::Slash),
        "ArrowLeft" => Some(Key::Left),
        "ArrowUp" => Some(Key::Up),
        "ArrowRight" => Some(Key::Right),
        "ArrowDown" => Some(Key::Down),
        _ => None,
    }
}

fn get_mouse_button(button: i16) -> MouseButton {
    match button {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        n => MouseButton::Other(n as u8),
    }
}
