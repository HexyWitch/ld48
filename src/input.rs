use euclid::default::{Point2D, Vector2D};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Space,
    Backspace,
    Return,
    Escape,
    Slash,
    Home,
    Delete,
    End,
    Left,
    Up,
    Right,
    Down,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Other(u8),
}

#[derive(Copy, Clone, Debug)]
pub enum InputEvent {
    KeyDown(Key),
    KeyUp(Key),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    MouseMove(Point2D<f32>),
    MouseWheel(Vector2D<f32>),
}
