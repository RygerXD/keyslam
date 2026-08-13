use std::{
    collections::HashMap,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use cpal::{
    Device, Error, ErrorKind, FromSample, I24, OutputCallbackInfo, Sample, SampleFormat,
    SizedSample, Stream, StreamConfig, U24,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};
use directories::ProjectDirs;
use include_dir::{Dir, include_dir};
use rodio::Source;

static SOUNDS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/sounds");
static SPEECH: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/speech");
const MAX_VOICES: usize = 32;
const SPEECH_LOADER_THREADS: usize = 2;

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
    Play(Vec<Arc<AudioClip>>, f32),
    Sine {
        frequency: f32,
        pan: f32,
        volume: f32,
    },
    StopSine,
}

struct Playback {
    clips: Vec<Arc<AudioClip>>,
    clip_index: usize,
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
                AudioCommand::Play(clips, gain) => {
                    if self.voices.len() >= MAX_VOICES {
                        self.voices.remove(0);
                    }
                    self.voices.push(Playback {
                        clips,
                        clip_index: 0,
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
                while voice.clip_index + 1 < voice.clips.len()
                    && voice.position >= voice.clips[voice.clip_index].frames() as f64 - 1.0
                {
                    voice.clip_index += 1;
                    voice.position = 0.0;
                }
                let clip = &voice.clips[voice.clip_index];
                if voice.position >= clip.frames() as f64 - 1.0 {
                    continue;
                }
                let (sample_left, sample_right) = clip.stereo_frame(voice.position);
                left += sample_left * voice.gain;
                right += sample_right * voice.gain;
                voice.position += clip.sample_rate / self.output_rate;
            }
            self.voices.retain(|voice| {
                voice.clip_index + 1 < voice.clips.len()
                    || voice.position < voice.clips[voice.clip_index].frames() as f64 - 1.0
            });

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
    speech_sender: Sender<Vec<String>>,
    clips: HashMap<&'static str, Arc<AudioClip>>,
    piano: Mutex<HashMap<i32, Arc<AudioClip>>>,
    _stream: Option<Stream>,
    status: Arc<Mutex<Option<String>>>,
}

impl AudioSystem {
    pub fn new(locale: &str) -> Self {
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
        let speech_root = speech_root();
        if let Err(error) = install_customizable_speech(&speech_root, locale) {
            set_status(
                &status,
                format!("Could not prepare the customizable speech folder: {error}"),
            );
        }
        let stream = match open_stream(receiver, Arc::clone(&status)) {
            Ok(stream) => Some(stream),
            Err(error) => {
                set_status(&status, format!("Audio is unavailable: {error}"));
                None
            }
        };
        let (speech_sender, speech_receiver) = bounded(64);
        start_speech_workers(
            speech_receiver,
            sender.clone(),
            locale.to_owned(),
            speech_root,
            Arc::clone(&status),
        );
        Self {
            sender,
            speech_sender,
            clips,
            piano: Mutex::new(HashMap::new()),
            _stream: stream,
            status,
        }
    }

    pub fn play_sound(&self, name: &'static str) {
        if let Some(clip) = self.clips.get(name) {
            self.send(AudioCommand::Play(vec![Arc::clone(clip)], 0.75));
        }
    }

    pub fn play_speech(&self, keys: &[String]) {
        match self.speech_sender.try_send(keys.to_vec()) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => {}
            Err(TrySendError::Full(_)) => {
                set_status(
                    &self.status,
                    "Speech queue is busy; a phrase was skipped".to_owned(),
                );
            }
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
            self.send(AudioCommand::Play(vec![clip], 1.0));
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

fn start_speech_workers(
    receiver: Receiver<Vec<String>>,
    audio_sender: Sender<AudioCommand>,
    locale: String,
    speech_root: PathBuf,
    status: Arc<Mutex<Option<String>>>,
) {
    let cache = Arc::new(Mutex::new(HashMap::<String, Arc<AudioClip>>::new()));
    for worker_index in 0..SPEECH_LOADER_THREADS {
        let receiver = receiver.clone();
        let audio_sender = audio_sender.clone();
        let locale = locale.clone();
        let speech_root = speech_root.clone();
        let status = Arc::clone(&status);
        let cache = Arc::clone(&cache);
        let _ = thread::Builder::new()
            .name(format!("speech-loader-{worker_index}"))
            .spawn(move || {
                while let Ok(keys) = receiver.recv() {
                    let clips = keys
                        .iter()
                        .filter_map(|key| speech_clip(key, &locale, &speech_root, &cache, &status))
                        .collect::<Vec<_>>();
                    if clips.len() == keys.len() {
                        try_send_audio(&audio_sender, &status, AudioCommand::Play(clips, 1.0));
                    }
                }
            });
    }
}

fn speech_clip(
    key: &str,
    locale: &str,
    speech_root: &Path,
    cache: &Mutex<HashMap<String, Arc<AudioClip>>>,
    status: &Arc<Mutex<Option<String>>>,
) -> Option<Arc<AudioClip>> {
    if let Some(clip) = cache.lock().ok().and_then(|clips| clips.get(key).cloned()) {
        return Some(clip);
    }

    let localized = Path::new(locale).join(key);
    let common = Path::new("common").join(key);
    let loaded = [localized, common].into_iter().find_map(|relative| {
        let external = speech_root.join(&relative);
        let bytes = fs::read(&external).ok().or_else(|| {
            SPEECH
                .get_file(&relative)
                .map(|file| file.contents().to_vec())
        })?;
        match decode_opus(&bytes) {
            Ok(clip) => Some(Arc::new(trim_speech_silence(clip))),
            Err(error) => {
                set_status(
                    status,
                    format!(
                        "Could not decode speech clip {}: {error}",
                        external.display()
                    ),
                );
                None
            }
        }
    });
    if let Some(clip) = &loaded
        && let Ok(mut clips) = cache.lock()
    {
        clips.insert(key.to_owned(), Arc::clone(clip));
    }
    if loaded.is_none() {
        set_status(status, format!("Speech clip is missing: {key}"));
    }
    loaded
}

fn try_send_audio(
    sender: &Sender<AudioCommand>,
    status: &Arc<Mutex<Option<String>>>,
    command: AudioCommand,
) {
    match sender.try_send(command) {
        Ok(()) | Err(TrySendError::Disconnected(_)) => {}
        Err(TrySendError::Full(_)) => set_status(
            status,
            "Audio queue is busy; an effect was skipped".to_owned(),
        ),
    }
}

fn speech_root() -> PathBuf {
    let root = ProjectDirs::from("com", "KeySlam", "KeySlam").map_or_else(
        || PathBuf::from("speech"),
        |dirs| dirs.config_dir().join("speech"),
    );
    migrate_legacy_speech(&root);
    root
}

fn migrate_legacy_speech(root: &Path) {
    if root.exists() {
        return;
    }
    let Some(legacy_dirs) = ProjectDirs::from("com", "BabySmash", "BabySmash Rust") else {
        return;
    };
    let legacy_root = legacy_dirs.config_dir().join("speech");
    let _ = copy_directory(&legacy_root, root);
}

fn copy_directory(source: &Path, destination: &Path) -> std::io::Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn install_customizable_speech(root: &Path, locale: &str) -> Result<(), String> {
    copy_speech_dir(&SPEECH, root, locale)
}

fn copy_speech_dir(directory: &Dir<'_>, root: &Path, locale: &str) -> Result<(), String> {
    for file in directory.files() {
        let path = file.path();
        let is_selected = path.components().next().is_some_and(|component| {
            component.as_os_str() == "common" || component.as_os_str() == locale
        });
        if !is_selected {
            continue;
        }
        let destination = root.join(path);
        if destination.exists() {
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(destination, file.contents()).map_err(|error| error.to_string())?;
    }
    for child in directory.dirs() {
        copy_speech_dir(child, root, locale)?;
    }
    Ok(())
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
                    "Audio device changed; restart KeySlam to reconnect".to_owned()
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

fn decode_opus(bytes: &[u8]) -> Result<AudioClip, String> {
    let (samples, head) = ruopus::decode_ogg_opus(bytes).map_err(|error| error.to_string())?;
    let channels = usize::from(head.channel_count);
    if channels == 0 || samples.len() < channels * 2 {
        return Err("clip contains no audio".to_owned());
    }
    Ok(AudioClip {
        samples,
        channels,
        sample_rate: 48_000.0,
    })
}

fn trim_speech_silence(mut clip: AudioClip) -> AudioClip {
    const RELATIVE_SILENCE_THRESHOLD: f32 = 0.005;
    const MIN_SILENCE_THRESHOLD: f32 = 0.0005;
    const EDGE_PADDING_SECONDS: f64 = 0.015;

    let frames = clip.frames();
    let peak = clip
        .samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    let threshold = (peak * RELATIVE_SILENCE_THRESHOLD).max(MIN_SILENCE_THRESHOLD);
    let frame_is_audible = |frame: usize| {
        clip.samples[frame * clip.channels..(frame + 1) * clip.channels]
            .iter()
            .any(|sample| sample.abs() >= threshold)
    };
    let Some(first_audible) = (0..frames).find(|&frame| frame_is_audible(frame)) else {
        return clip;
    };
    let Some(last_audible) = (0..frames).rfind(|&frame| frame_is_audible(frame)) else {
        return clip;
    };
    let padding = (clip.sample_rate * EDGE_PADDING_SECONDS).round() as usize;
    let start = first_audible.saturating_sub(padding);
    let end = (last_audible + 1 + padding).min(frames);
    clip.samples = clip.samples[start * clip.channels..end * clip.channels].to_vec();
    clip
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
