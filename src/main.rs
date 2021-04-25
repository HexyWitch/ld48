mod constants;
mod game;
#[allow(unused)]
mod gl;
mod graphics;
mod input;
mod mixer;
mod platform;
mod texture_atlas;

use std::sync::Arc;

use constants::{SCREEN_SIZE, TICK_DT};
use game::Game;
use input::InputEvent;

fn main() {
    platform::run(
        "Ludum Dare 48",
        SCREEN_SIZE,
        |gl_context: &mut gl::Context| {
            let mixer = Arc::new(mixer::Mixer::default());
            let mixer_inner = Arc::clone(&mixer);
            platform::start_audio_playback(move |out: &mut [i16]| mixer_inner.poll(out));

            let mut game = Game::new(gl_context, mixer);
            let mut input_vec = Vec::new();
            let mut last_update: f32 = 0.;
            move |dt: f32, inputs: &[InputEvent], gl_context: &mut gl::Context| {
                // accumulate input over several frames
                input_vec.extend_from_slice(inputs);

                // jank ass fixed update loop, skip max 5 frames
                last_update = (last_update + dt).min(TICK_DT * 5.);
                while last_update > TICK_DT {
                    game.update(&input_vec);

                    last_update -= TICK_DT;
                    input_vec.clear();
                }

                game.draw(gl_context);
            }
        },
    )
}
