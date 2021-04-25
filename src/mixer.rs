use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use anyhow::Error;
use lewton::inside_ogg::OggStreamReader;

pub struct Mixer {
    playing: Arc<Mutex<HashMap<usize, AudioInstance>>>,
    next_id: AtomicUsize,
}

impl Default for Mixer {
    fn default() -> Self {
        Self {
            playing: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicUsize::new(0),
        }
    }
}

impl Mixer {
    pub fn load_ogg(&self, bytes: &[u8]) -> Result<Audio, Error> {
        let mut reader = OggStreamReader::new(std::io::Cursor::new(bytes))?;
        let mut buffer = Vec::new();
        while let Some(pck_samples) = reader.read_dec_packet_itl()? {
            for sample in pck_samples {
                buffer.push(sample);
            }
        }

        Ok(Audio {
            buffer: Arc::new(buffer),
        })
    }

    pub fn play(&self, audio: &Audio, volume: f32, do_loop: bool) -> AudioInstanceHandle {
        let instance = AudioInstance {
            audio: Audio {
                buffer: audio.buffer.clone(),
            },
            index: 0,
            volume,
            do_loop,
        };
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.playing.lock().unwrap().insert(id, instance);
        AudioInstanceHandle(id)
    }

    pub fn stop(&self, handle: &AudioInstanceHandle) {
        self.playing.lock().unwrap().remove(&handle.0);
    }

    pub fn set_volume(&self, handle: &AudioInstanceHandle, volume: f32) {
        let mut instances = self.playing.lock().unwrap();
        if let Some(instance) = instances.get_mut(&handle.0) {
            instance.volume = volume;
        };
    }

    pub fn set_looping(&self, handle: &AudioInstanceHandle, do_loop: bool) {
        let mut instances = self.playing.lock().unwrap();
        if let Some(instance) = instances.get_mut(&handle.0) {
            instance.do_loop = do_loop;
        };
    }

    pub fn poll(&self, out: &mut [i16]) {
        let mut instances = self.playing.lock().unwrap();

        let mut finished = Vec::new();
        for (id, instance) in instances.iter_mut() {
            let requested_samples = out.len();
            let remaining_samples = if instance.do_loop {
                requested_samples
            } else {
                instance.audio.buffer.len() - instance.index
            };
            for i in 0..requested_samples.min(remaining_samples) {
                let instance_i = (instance.index + i) % instance.audio.buffer.len();
                out[i] += ((instance.audio.buffer[instance_i] as f32 / i16::max_value() as f32)
                    * instance.volume
                    * i16::max_value() as f32)
                    .floor() as i16;
            }
            if requested_samples >= remaining_samples && !instance.do_loop {
                finished.push(*id);
            } else {
                instance.index = (instance.index + requested_samples) % instance.audio.buffer.len();
            }
        }
        for id in finished.into_iter().rev() {
            instances.remove(&id);
        }
    }
}

pub struct Audio {
    buffer: Arc<Vec<i16>>,
}

pub struct AudioInstance {
    audio: Audio,
    index: usize,
    volume: f32,
    do_loop: bool,
}

pub struct AudioInstanceHandle(usize);
