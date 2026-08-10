use std::{
    collections::HashMap,
    io::Cursor,
    sync::{Arc, Mutex},
};

use cpal::{
    Device, Error, ErrorKind, FromSample, I24, OutputCallbackInfo, Sample, SampleFormat,
    SizedSample, Stream, StreamConfig, U24,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};
use include_dir::{Dir, include_dir};
use rodio::Source;

static SOUNDS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/sounds");
const MAX_VOICES: usize = 32;

#[derive(Debug)]
struct AudioClip {
    samples: Vec<f32>,
    channels: usize,
    sample_rate: f64,
}

impl AudioClip {
    fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1)
    }

    fn stereo_frame(&self, position: f64) -> (f32, f32) {
        let frame = position.floor() as usize;
        let next = (frame + 1).min(self.frames().saturating_sub(1));
        let fraction = (position - frame as f64) as f32;
        let sample = |frame_index: usize, channel: usize| {
            let source_channel = channel.min(self.channels.saturating_sub(1));
            self.samples[frame_index * self.channels + source_channel]
        };
        let left = sample(frame, 0) + (sample(next, 0) - sample(frame, 0)) * fraction;
        let right = if self.channels == 1 {
            left
        } else {
            sample(frame, 1) + (sample(next, 1) - sample(frame, 1)) * fraction
        };
        (left, right)
    }
}

enum AudioCommand {
    Play(Arc<AudioClip>, f32),
    Sine {
        frequency: f32,
        pan: f32,
        volume: f32,
    },
    StopSine,
}

struct Playback {
    clip: Arc<AudioClip>,
    position: f64,
    gain: f32,
}

struct Mixer {
    receiver: Receiver<AudioCommand>,
    voices: Vec<Playback>,
    output_rate: f64,
    output_channels: usize,
    sine_frequency: f32,
    sine_pan: f32,
    sine_target_volume: f32,
    sine_volume: f32,
    sine_phase: f32,
}

impl Mixer {
    fn new(receiver: Receiver<AudioCommand>, config: &StreamConfig) -> Self {
        Self {
            receiver,
            voices: Vec::new(),
            output_rate: f64::from(config.sample_rate),
            output_channels: usize::from(config.channels),
            sine_frequency: 440.0,
            sine_pan: 0.0,
            sine_target_volume: 0.0,
            sine_volume: 0.0,
            sine_phase: 0.0,
        }
    }

    fn drain_commands(&mut self) {
        while let Ok(command) = self.receiver.try_recv() {
            match command {
                AudioCommand::Play(clip, gain) => {
                    if self.voices.len() >= MAX_VOICES {
                        self.voices.remove(0);
                    }
                    self.voices.push(Playback {
                        clip,
                        position: 0.0,
                        gain,
                    });
                }
                AudioCommand::Sine {
                    frequency,
                    pan,
                    volume,
                } => {
                    self.sine_frequency = frequency.clamp(20.0, 8_000.0);
                    self.sine_pan = pan.clamp(-1.0, 1.0);
                    self.sine_target_volume = volume.clamp(0.0, 0.7);
                }
                AudioCommand::StopSine => self.sine_target_volume = 0.0,
            }
        }
    }

    fn write<T>(&mut self, output: &mut [T])
    where
        T: Sample + FromSample<f32>,
    {
        self.drain_commands();
        for frame in output.chunks_mut(self.output_channels) {
            let mut left = 0.0;
            let mut right = 0.0;
            for voice in &mut self.voices {
                let (sample_left, sample_right) = voice.clip.stereo_frame(voice.position);
                left += sample_left * voice.gain;
                right += sample_right * voice.gain;
                voice.position += voice.clip.sample_rate / self.output_rate;
            }
            self.voices
                .retain(|voice| voice.position < voice.clip.frames() as f64 - 1.0);

            self.sine_volume += (self.sine_target_volume - self.sine_volume) * 0.008;
            if self.sine_volume > 0.0001 {
                self.sine_phase = (self.sine_phase
                    + self.sine_frequency * std::f32::consts::TAU / self.output_rate as f32)
                    % std::f32::consts::TAU;
                let tone = self.sine_phase.sin() * self.sine_volume;
                left += tone * ((1.0 - self.sine_pan) * 0.5).sqrt();
                right += tone * ((1.0 + self.sine_pan) * 0.5).sqrt();
            }

            let left = left.clamp(-1.0, 1.0);
            let right = right.clamp(-1.0, 1.0);
            for (channel, sample) in frame.iter_mut().enumerate() {
                let value = match channel {
                    0 => left,
                    1 => right,
                    _ => (left + right) * 0.5,
                };
                *sample = T::from_sample(value);
            }
        }
    }
}

pub struct AudioSystem {
    sender: Sender<AudioCommand>,
    clips: HashMap<&'static str, Arc<AudioClip>>,
    piano: Mutex<HashMap<i32, Arc<AudioClip>>>,
    _stream: Option<Stream>,
    status: Arc<Mutex<Option<String>>>,
}

impl AudioSystem {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(64);
        let status = Arc::new(Mutex::new(None));
        let mut clips = HashMap::new();
        for name in ["smallbumblebee.wav"] {
            match SOUNDS
                .get_file(name)
                .and_then(|file| decode_wav(file.contents()).ok())
            {
                Some(clip) => {
                    clips.insert(name, Arc::new(clip));
                }
                None => set_status(&status, format!("Could not decode bundled sound {name}")),
            }
        }
        let stream = match open_stream(receiver, Arc::clone(&status)) {
            Ok(stream) => Some(stream),
            Err(error) => {
                set_status(&status, format!("Audio is unavailable: {error}"));
                None
            }
        };
        Self {
            sender,
            clips,
            piano: Mutex::new(HashMap::new()),
            _stream: stream,
            status,
        }
    }

    pub fn play_sound(&self, name: &'static str) {
        if let Some(clip) = self.clips.get(name) {
            self.send(AudioCommand::Play(Arc::clone(clip), 0.75));
        }
    }

    pub fn play_piano(&self, frequency: f32) {
        let note = midi_note(frequency);
        let clip = self.piano.lock().ok().map(|mut cache| {
            Arc::clone(
                cache
                    .entry(note)
                    .or_insert_with(|| Arc::new(piano_clip(note))),
            )
        });
        if let Some(clip) = clip {
            self.send(AudioCommand::Play(clip, 1.0));
        }
    }

    pub fn start_or_update_sine(&self, frequency: f32, pan: f32, volume: f32) {
        self.send(AudioCommand::Sine {
            frequency,
            pan,
            volume,
        });
    }

    pub fn stop_sine(&self) {
        self.send(AudioCommand::StopSine);
    }

    pub fn status(&self) -> Option<String> {
        self.status.lock().ok().and_then(|status| status.clone())
    }

    fn send(&self, command: AudioCommand) {
        match self.sender.try_send(command) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => {}
            Err(TrySendError::Full(_)) => {
                set_status(
                    &self.status,
                    "Audio queue is busy; an effect was skipped".to_owned(),
                );
            }
        }
    }
}

fn set_status(status: &Arc<Mutex<Option<String>>>, message: String) {
    if let Ok(mut current) = status.lock() {
        *current = Some(message);
    }
}

fn open_stream(
    receiver: Receiver<AudioCommand>,
    status: Arc<Mutex<Option<String>>>,
) -> Result<Stream, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no default output device".to_owned())?;
    let supported = device
        .default_output_config()
        .map_err(|error| error.to_string())?;
    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let stream = match sample_format {
        SampleFormat::I8 => build_stream::<i8>(&device, config, receiver, status),
        SampleFormat::I16 => build_stream::<i16>(&device, config, receiver, status),
        SampleFormat::I24 => build_stream::<I24>(&device, config, receiver, status),
        SampleFormat::I32 => build_stream::<i32>(&device, config, receiver, status),
        SampleFormat::I64 => build_stream::<i64>(&device, config, receiver, status),
        SampleFormat::U8 => build_stream::<u8>(&device, config, receiver, status),
        SampleFormat::U16 => build_stream::<u16>(&device, config, receiver, status),
        SampleFormat::U24 => build_stream::<U24>(&device, config, receiver, status),
        SampleFormat::U32 => build_stream::<u32>(&device, config, receiver, status),
        SampleFormat::U64 => build_stream::<u64>(&device, config, receiver, status),
        SampleFormat::F32 => build_stream::<f32>(&device, config, receiver, status),
        SampleFormat::F64 => build_stream::<f64>(&device, config, receiver, status),
        format => return Err(format!("unsupported output format {format}")),
    }
    .map_err(|error| error.to_string())?;
    stream.play().map_err(|error| error.to_string())?;
    Ok(stream)
}

fn build_stream<T>(
    device: &Device,
    config: StreamConfig,
    receiver: Receiver<AudioCommand>,
    status: Arc<Mutex<Option<String>>>,
) -> Result<Stream, cpal::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let mut mixer = Mixer::new(receiver, &config);
    device.build_output_stream(
        config,
        move |output: &mut [T], _: &OutputCallbackInfo| mixer.write(output),
        move |error: Error| {
            let message = match error.kind() {
                ErrorKind::DeviceChanged => {
                    "Audio device changed; restart BabySmash to reconnect".to_owned()
                }
                ErrorKind::Xrun => "Audio playback fell behind briefly".to_owned(),
                ErrorKind::RealtimeDenied => {
                    "The operating system denied real-time audio".to_owned()
                }
                _ => format!("Audio stream error: {error}"),
            };
            set_status(&status, message);
        },
        None,
    )
}

fn decode_wav(bytes: &[u8]) -> Result<AudioClip, String> {
    let decoder =
        rodio::Decoder::try_from(Cursor::new(bytes.to_vec())).map_err(|error| error.to_string())?;
    let channels = usize::from(decoder.channels().get());
    let sample_rate = f64::from(decoder.sample_rate().get());
    let samples = decoder.collect();
    Ok(AudioClip {
        samples,
        channels,
        sample_rate,
    })
}

fn midi_note(frequency: f32) -> i32 {
    (69.0 + 12.0 * (frequency.max(1.0) / 440.0).log2())
        .round()
        .clamp(36.0, 96.0) as i32
}

fn piano_clip(note: i32) -> AudioClip {
    const SAMPLE_RATE: usize = 44_100;
    const DURATION: f32 = 1.25;
    let frequency = 440.0 * 2.0_f32.powf((note - 69) as f32 / 12.0);
    let frames = (SAMPLE_RATE as f32 * DURATION) as usize;
    let mut samples = Vec::with_capacity(frames * 2);
    for frame in 0..frames {
        let time = frame as f32 / SAMPLE_RATE as f32;
        let attack = (time / 0.008).min(1.0);
        let signal = (std::f32::consts::TAU * frequency * time).sin() * (-2.8 * time).exp()
            + 0.42 * (std::f32::consts::TAU * frequency * 2.01 * time).sin() * (-4.2 * time).exp()
            + 0.18 * (std::f32::consts::TAU * frequency * 3.03 * time).sin() * (-5.8 * time).exp()
            + 0.08 * (std::f32::consts::TAU * frequency * 4.08 * time).sin() * (-7.5 * time).exp();
        let sample = signal * attack * 0.45 / 1.68;
        samples.extend([sample, sample]);
    }
    AudioClip {
        samples,
        channels: 2,
        sample_rate: SAMPLE_RATE as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_sound_decodes() {
        for file in SOUNDS.files() {
            let clip = decode_wav(file.contents()).unwrap_or_else(|error| {
                panic!("{} did not decode: {error}", file.path().display())
            });
            assert!(clip.frames() > 0);
        }
    }

    #[test]
    fn piano_range_is_clamped() {
        assert_eq!(midi_note(1.0), 36);
        assert_eq!(midi_note(440.0), 69);
        assert_eq!(midi_note(50_000.0), 96);
    }
}
