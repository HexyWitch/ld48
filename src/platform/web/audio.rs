use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::AudioProcessingEvent;

pub fn start_audio_playback<F: FnMut(&mut [i16]) + 'static + Send>(mut f: F) {
    let audio_context = web_sys::AudioContext::new().unwrap();
    let script_processor_node = audio_context.create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(4096, 0, 2).unwrap();

    let mut int_buffer = Vec::new();
    let mut float_buffer_left = Vec::new();
    let mut float_buffer_right = Vec::new();
    let on_audio_process = Closure::wrap(Box::new(move |event: AudioProcessingEvent| {
        let audio_buffer = event.output_buffer().unwrap();
        let len = audio_buffer.length() as usize;
        int_buffer.resize(len * 2, 0);
        f(&mut int_buffer);
        for (i, s) in int_buffer.drain(0..).enumerate() {
            if i % 2 == 0 {
                float_buffer_left.push(s as f32 / 32768.);
            } else {
                float_buffer_right.push(s as f32 / 32768.);
            }
        }
        audio_buffer
            .copy_to_channel(&mut float_buffer_left, 0)
            .unwrap();
        audio_buffer
            .copy_to_channel(&mut float_buffer_right, 1)
            .unwrap();
        float_buffer_left.clear();
        float_buffer_right.clear();
    }) as Box<dyn FnMut(AudioProcessingEvent)>);

    script_processor_node
        .connect_with_audio_node(&audio_context.destination())
        .unwrap();
    script_processor_node.set_onaudioprocess(Some(on_audio_process.as_ref().unchecked_ref()));
    on_audio_process.forget();
}
