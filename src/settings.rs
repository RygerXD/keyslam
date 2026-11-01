use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use atomic_write_file::AtomicWriteFile;
use serde::{Deserialize, Serialize};

use crate::paths::executable_directory;

pub const MIN_FADE_AFTER_SECONDS: f32 = 0.0;
pub const MAX_FADE_AFTER_SECONDS: f32 = 120.0;
pub const MIN_ITEMS_KEPT: usize = 5;
pub const MAX_ITEMS_KEPT: usize = 50;
pub const MIN_UI_SCALE_PERCENT: u8 = 50;
pub const MAX_UI_SCALE_PERCENT: u8 = 200;
pub const MIN_ITEM_SIZE_PERCENT: u8 = 50;
pub const MAX_ITEM_SIZE_PERCENT: u8 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundMode {
    Speech,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointerSound {
    SineWave,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PianoScale {
    Chromatic,
    Major,
    Minor,
}

impl PianoScale {
    pub const ALL: [Self; 3] = [Self::Chromatic, Self::Major, Self::Minor];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Chromatic => "Chromatic",
            Self::Major => "Major",
            Self::Minor => "Minor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PianoKey {
    C,
    CSharp,
    D,
    EFlat,
    E,
    F,
    FSharp,
    G,
    AFlat,
    A,
    BFlat,
    B,
}

impl PianoKey {
    pub const ALL: [Self; 12] = [
        Self::C,
        Self::CSharp,
        Self::D,
        Self::EFlat,
        Self::E,
        Self::F,
        Self::FSharp,
        Self::G,
        Self::AFlat,
        Self::A,
        Self::BFlat,
        Self::B,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::CSharp => "C# / Db",
            Self::D => "D",
            Self::EFlat => "Eb",
            Self::E => "E",
            Self::F => "F",
            Self::FSharp => "F# / Gb",
            Self::G => "G",
            Self::AFlat => "Ab",
            Self::A => "A",
            Self::BFlat => "Bb",
            Self::B => "B",
        }
    }

    pub const fn semitone(self) -> i32 {
        match self {
            Self::C => 0,
            Self::CSharp => 1,
            Self::D => 2,
            Self::EFlat => 3,
            Self::E => 4,
            Self::F => 5,
            Self::FSharp => 6,
            Self::G => 7,
            Self::AFlat => 8,
            Self::A => 9,
            Self::BFlat => 10,
            Self::B => 11,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorEffect {
    Rainbow,
    FadingTrail,
    Sparkles,
    Bubbles,
    Coloring,
    PianoRoll,
}

impl CursorEffect {
    pub const ALL: [Self; 6] = [
        Self::Rainbow,
        Self::FadingTrail,
        Self::Sparkles,
        Self::Bubbles,
        Self::Coloring,
        Self::PianoRoll,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Rainbow => "Rainbow ribbon",
            Self::FadingTrail => "Fading mouse trails",
            Self::Sparkles => "Sparkles",
            Self::Bubbles => "Bubbles",
            Self::Coloring => "Coloring mode",
            Self::PianoRoll => "Piano roll",
        }
    }

    pub const fn shadertoy_url(self) -> Option<&'static str> {
        match self {
            Self::FadingTrail => Some("https://www.shadertoy.com/view/mtSGDy"),
            _ => None,
        }
    }

    pub const fn number_key(self) -> &'static str {
        match self {
            Self::Rainbow => "1",
            Self::FadingTrail => "2",
            Self::Sparkles => "3",
            Self::Bubbles => "4",
            Self::Coloring => "5",
            Self::PianoRoll => "6",
        }
    }

    pub fn from_number_key(key_name: &str) -> Option<Self> {
        let digit = key_name.strip_prefix('D').unwrap_or(key_name);
        Self::ALL
            .into_iter()
            .find(|effect| effect.number_key() == digit)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub sound_mode: SoundMode,
    pub extra_key_set: String,
    pub fade_away: bool,
    pub fade_after_seconds: f32,
    pub clear_after: usize,
    pub letter_grouping_timeout_seconds: f32,
    pub group_letters: bool,
    pub pointer_sound: PointerSound,
    pub master_volume_percent: u8,
    pub sound_clip_volume_percent: u8,
    pub paint_color_volume_percent: u8,
    pub piano_note_volume_percent: u8,
    pub sine_wave_volume_percent: u8,
    pub paint_color_speech: bool,
    pub right_click_piano_enabled: bool,
    pub right_click_piano_scale: PianoScale,
    pub right_click_piano_key: PianoKey,
    pub spawn_animations: bool,
    pub interaction_animations: bool,
    pub faces_on_shapes: bool,
    pub background_brightness_percent: u8,
    pub ui_scale_percent: u8,
    pub item_size_percent: u8,
    pub cursor_effect: CursorEffect,
    pub force_uppercase: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound_mode: SoundMode::Speech,
            extra_key_set: "animals".to_owned(),
            fade_away: true,
            fade_after_seconds: 4.0,
            clear_after: 30,
            letter_grouping_timeout_seconds: 1.0,
            group_letters: true,
            pointer_sound: PointerSound::SineWave,
            master_volume_percent: 100,
            sound_clip_volume_percent: 100,
            paint_color_volume_percent: 100,
            piano_note_volume_percent: 100,
            sine_wave_volume_percent: 35,
            paint_color_speech: true,
            right_click_piano_enabled: true,
            right_click_piano_scale: PianoScale::Chromatic,
            right_click_piano_key: PianoKey::C,
            spawn_animations: true,
            interaction_animations: true,
            faces_on_shapes: true,
            background_brightness_percent: 7,
            ui_scale_percent: 100,
            item_size_percent: 100,
            cursor_effect: CursorEffect::Rainbow,
            force_uppercase: true,
        }
    }
}

impl Settings {
    pub fn normalize(&mut self) {
        self.extra_key_set = match self.extra_key_set.as_str() {
            "Animals" => "animals".to_owned(),
            "Foods" => "foods".to_owned(),
            "Instruments" => "instruments".to_owned(),
            name if name.eq_ignore_ascii_case("directions") => "animals".to_owned(),
            _ => self.extra_key_set.clone(),
        };
        self.fade_after_seconds = self
            .fade_after_seconds
            .clamp(MIN_FADE_AFTER_SECONDS, MAX_FADE_AFTER_SECONDS);
        self.clear_after = self.clear_after.clamp(MIN_ITEMS_KEPT, MAX_ITEMS_KEPT);
        self.letter_grouping_timeout_seconds =
            self.letter_grouping_timeout_seconds.clamp(0.1, 10.0);
        self.master_volume_percent = self.master_volume_percent.min(100);
        self.sound_clip_volume_percent = self.sound_clip_volume_percent.min(100);
        self.paint_color_volume_percent = self.paint_color_volume_percent.min(100);
        self.piano_note_volume_percent = self.piano_note_volume_percent.min(100);
        self.sine_wave_volume_percent = self.sine_wave_volume_percent.min(100);
        self.background_brightness_percent = self.background_brightness_percent.min(100);
        self.ui_scale_percent = self
            .ui_scale_percent
            .clamp(MIN_UI_SCALE_PERCENT, MAX_UI_SCALE_PERCENT);
        self.item_size_percent = self
            .item_size_percent
            .clamp(MIN_ITEM_SIZE_PERCENT, MAX_ITEM_SIZE_PERCENT);
    }

    pub fn sound_clip_gain(&self) -> f32 {
        volume_gain(self.master_volume_percent, self.sound_clip_volume_percent)
    }

    pub fn piano_note_gain(&self) -> f32 {
        volume_gain(self.master_volume_percent, self.piano_note_volume_percent)
    }

    pub fn paint_color_gain(&self) -> f32 {
        volume_gain(self.master_volume_percent, self.paint_color_volume_percent)
    }

    pub fn sine_wave_gain(&self) -> f32 {
        volume_gain(self.master_volume_percent, self.sine_wave_volume_percent)
    }
}

fn volume_gain(master_percent: u8, category_percent: u8) -> f32 {
    f32::from(master_percent) * f32::from(category_percent) / 10_000.0
}

#[derive(Debug)]
pub struct SettingsStore {
    path: Option<PathBuf>,
    pub warning: Option<String>,
}

impl SettingsStore {
    pub fn open() -> (Self, Settings) {
        match executable_directory() {
            Ok(directory) => {
                let path = directory.join("settings").join("settings.json");
                let is_new = !path.exists();
                let (mut store, settings) = Self::open_path(path);
                if is_new && let Err(error) = store.save(&settings) {
                    store.warning = Some(format!("Could not create the settings file: {error}"));
                }
                (store, settings)
            }
            Err(error) => (
                Self {
                    path: None,
                    warning: Some(format!(
                        "Could not locate the KeySlam folder; settings will not be saved: {error}"
                    )),
                },
                Settings::default(),
            ),
        }
    }

    fn open_path(path: PathBuf) -> (Self, Settings) {
        let mut store = Self {
            path: Some(path),
            warning: None,
        };
        let mut settings = match store.path.as_deref().map(fs::read) {
            Some(Ok(bytes)) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                store.warning = Some(format!(
                    "Settings were invalid and defaults were loaded: {error}"
                ));
                Settings::default()
            }),
            Some(Err(error)) if error.kind() == io::ErrorKind::NotFound => Settings::default(),
            Some(Err(error)) => {
                store.warning = Some(format!("Settings could not be read: {error}"));
                Settings::default()
            }
            None => Settings::default(),
        };
        settings.normalize();
        (store, settings)
    }

    pub fn save(&mut self, settings: &Settings) -> io::Result<()> {
        let path = self.path.as_deref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "the KeySlam executable folder is unavailable",
            )
        })?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
        let mut file = AtomicWriteFile::open(path)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.commit()?;
        self.warning = None;
        Ok(())
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}
