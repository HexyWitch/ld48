use cpal::{
    traits::{DeviceTrait, EventLoopTrait, HostTrait},
    Sample, StreamData, UnknownTypeOutputBuffer,
};

pub fn start_audio_playback<F: FnMut(&mut [i16]) + 'static + Send>(mut f: F) {
    std::thread::spawn(move || {
        let host = cpal::default_host();
        let event_loop = host.event_loop();

        let device = host
            .default_output_device()
            .expect("no output device available");

        let formats: Vec<cpal::SupportedFormat> = device
            .supported_output_formats()
            .unwrap()
            .collect::<Vec<_>>();
        let output_format = device
            .supported_output_formats()
            .unwrap()
            .next()
            .unwrap()
            .with_max_sample_rate();

        let stream_id = event_loop
            .build_output_stream(&device, &output_format)
            .expect("Output format not supported");
        event_loop.play_stream(stream_id).unwrap();

        let mut intermediate_buffer = Vec::new();
        event_loop.run(move |stream_id, stream_result| {
            let stream_buffer = match stream_result {
                Ok(data) => match data {
                    StreamData::Output {
                        buffer: UnknownTypeOutputBuffer::F32(buffer),
                    } => Some(buffer),
                    StreamData::Output { buffer: _ } => {
                        log::warn!(
                            "Unexpected output data format from stream id: {:?}",
                            stream_id
                        );
                        return;
                    }
                    _ => None,
                },
                Err(e) => {
                    log::error!("Stream error: {}", e);
                    return;
                }
            };

            if let Some(mut buffer) = stream_buffer {
                intermediate_buffer.clear();
                intermediate_buffer.resize(buffer.len(), 0);
                f(&mut intermediate_buffer);
                for (i, sample) in intermediate_buffer.drain(0..).enumerate() {
                    buffer[i] = sample.to_f32();
                }
            }
        });
    });
}
