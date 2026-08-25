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
    None,
    Rainbow,
    FadingTrail,
    NeonWorm,
    Sparkles,
    Bubbles,
    Coloring,
    PianoRoll,
}

impl CursorEffect {
    pub const ALL: [Self; 8] = [
        Self::None,
        Self::Rainbow,
        Self::FadingTrail,
        Self::NeonWorm,
        Self::Sparkles,
        Self::Bubbles,
        Self::Coloring,
        Self::PianoRoll,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Rainbow => "Rainbow ribbon",
            Self::FadingTrail => "Fading mouse trails",
            Self::NeonWorm => "Neon worm",
            Self::Sparkles => "Sparkles",
            Self::Bubbles => "Bubbles",
            Self::Coloring => "Coloring mode",
            Self::PianoRoll => "Piano roll",
        }
    }

    pub const fn shadertoy_url(self) -> Option<&'static str> {
        match self {
            Self::FadingTrail => Some("https://www.shadertoy.com/view/mtSGDy"),
            Self::NeonWorm => Some("https://www.shadertoy.com/view/clB3RK"),
            _ => None,
        }
    }

    pub fn cycle(self, forward: bool) -> Self {
        let current = Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(0);
        let offset = if forward { 1 } else { Self::ALL.len() - 1 };
        Self::ALL[(current + offset) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_settings_use_upstream_defaults() {
        let path = std::env::temp_dir().join(format!(
            "keyslam-settings-{}-missing.json",
            std::process::id()
        ));
        let (_, settings) = SettingsStore::open_path(path);
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn unsafe_values_are_clamped() {
        let mut settings = Settings {
            fade_after_seconds: -4.0,
            clear_after: usize::MAX,
            letter_grouping_timeout_seconds: f32::INFINITY,
            master_volume_percent: 200,
            sound_clip_volume_percent: 150,
            paint_color_volume_percent: 175,
            piano_note_volume_percent: 125,
            sine_wave_volume_percent: 100,
            background_brightness_percent: 250,
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.fade_after_seconds, MIN_FADE_AFTER_SECONDS);
        assert_eq!(settings.clear_after, MAX_ITEMS_KEPT);
        assert_eq!(settings.letter_grouping_timeout_seconds, 10.0);
        assert_eq!(settings.master_volume_percent, 100);
        assert_eq!(settings.sound_clip_volume_percent, 100);
        assert_eq!(settings.paint_color_volume_percent, 100);
        assert_eq!(settings.piano_note_volume_percent, 100);
        assert_eq!(settings.sine_wave_volume_percent, 100);
        assert_eq!(settings.background_brightness_percent, 100);
    }

    #[test]
    fn master_volume_scales_each_audio_category() {
        let settings = Settings {
            master_volume_percent: 50,
            sound_clip_volume_percent: 80,
            paint_color_volume_percent: 70,
            piano_note_volume_percent: 60,
            sine_wave_volume_percent: 40,
            ..Settings::default()
        };
        assert_eq!(settings.sound_clip_gain(), 0.4);
        assert_eq!(settings.paint_color_gain(), 0.35);
        assert_eq!(settings.piano_note_gain(), 0.3);
        assert_eq!(settings.sine_wave_gain(), 0.2);
    }

    #[test]
    fn cleanup_ranges_match_the_settings_controls() {
        assert_eq!(MIN_FADE_AFTER_SECONDS, 0.0);
        assert_eq!(MAX_FADE_AFTER_SECONDS, 120.0);
        assert_eq!(MIN_ITEMS_KEPT, 5);
        assert_eq!(MAX_ITEMS_KEPT, 50);

        let mut settings = Settings {
            fade_after_seconds: 121.0,
            clear_after: 0,
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.fade_after_seconds, MAX_FADE_AFTER_SECONDS);
        assert_eq!(settings.clear_after, MIN_ITEMS_KEPT);
    }

    #[test]
    fn special_modes_are_part_of_the_scroll_wheel_cycle() {
        assert_eq!(CursorEffect::Bubbles.cycle(true), CursorEffect::Coloring);
        assert_eq!(CursorEffect::Coloring.cycle(true), CursorEffect::PianoRoll);
        assert_eq!(CursorEffect::PianoRoll.cycle(true), CursorEffect::None);
        assert_eq!(CursorEffect::PianoRoll.cycle(false), CursorEffect::Coloring);
    }

    #[test]
    fn shadertoy_trails_have_labels_and_source_links() {
        for (effect, label, url) in [
            (
                CursorEffect::FadingTrail,
                "Fading mouse trails",
                "https://www.shadertoy.com/view/mtSGDy",
            ),
            (
                CursorEffect::NeonWorm,
                "Neon worm",
                "https://www.shadertoy.com/view/clB3RK",
            ),
        ] {
            assert_eq!(effect.label(), label);
            assert_eq!(effect.shadertoy_url(), Some(url));
        }
    }

    #[test]
    fn settings_round_trip_through_atomic_file() -> io::Result<()> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("keyslam-settings-{}-{nonce}", std::process::id()));
        let path = directory.join("settings.json");
        let (mut store, mut settings) = SettingsStore::open_path(path.clone());
        settings.cursor_effect = CursorEffect::Coloring;
        settings.clear_after = 47;
        store.save(&settings)?;

        let (_, loaded) = SettingsStore::open_path(path.clone());
        assert_eq!(loaded, settings);
        fs::remove_file(path)?;
        fs::remove_dir(directory)?;
        Ok(())
    }
}
