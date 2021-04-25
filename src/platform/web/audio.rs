use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Sample,
};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::AudioProcessingEvent;

pub fn start_audio_playback<F: FnMut(&mut [i16]) + 'static + Send>(mut f: F) {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("no output device available");

    let supported_output_config = device
        .supported_output_configs()
        .unwrap()
        .next()
        .unwrap()
        .with_max_sample_rate();

    match supported_output_config.sample_format() {
        cpal::SampleFormat::F32 => {}
        _ => {
            panic!("Output format not supported");
        }
    }

    let output_config = supported_output_config.config();

    let mut intermediate_buffer = Vec::new();

    let stream = device
        .build_output_stream(
            &output_config,
            move |data, _| {
                intermediate_buffer.clear();
                intermediate_buffer.resize(data.len(), 0);
                f(&mut intermediate_buffer);
                for (i, sample) in intermediate_buffer.drain(0..).enumerate() {
                    data[i] = sample.to_f32();
                }
            },
            |e| panic!("{}", e),
        )
        .unwrap();
    stream.play().unwrap();
    std::mem::forget(stream);
}
