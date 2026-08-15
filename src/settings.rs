use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use atomic_write_file::AtomicWriteFile;
use directories::ProjectDirs;
use serde::{Deserialize, Deserializer, Serialize};

pub const MIN_FADE_AFTER_SECONDS: f32 = 0.0;
pub const MAX_FADE_AFTER_SECONDS: f32 = 120.0;
pub const MIN_ITEMS_KEPT: usize = 5;
pub const MAX_ITEMS_KEPT: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundMode {
    Speech,
    #[serde(other)]
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointerSound {
    Click,
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
pub enum CursorStyle {
    Arrow,
    Hand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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

#[derive(Deserialize)]
enum StoredCursorEffect {
    None,
    Rainbow,
    FadingTrail,
    NeonWorm,
    Sparkles,
    Bubbles,
    Coloring,
    PianoRoll,
    #[serde(other)]
    Removed,
}

impl<'de> Deserialize<'de> for CursorEffect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match StoredCursorEffect::deserialize(deserializer)? {
            StoredCursorEffect::None | StoredCursorEffect::Removed => Self::None,
            StoredCursorEffect::Rainbow => Self::Rainbow,
            StoredCursorEffect::FadingTrail => Self::FadingTrail,
            StoredCursorEffect::NeonWorm => Self::NeonWorm,
            StoredCursorEffect::Sparkles => Self::Sparkles,
            StoredCursorEffect::Bubbles => Self::Bubbles,
            StoredCursorEffect::Coloring => Self::Coloring,
            StoredCursorEffect::PianoRoll => Self::PianoRoll,
        })
    }
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
#[serde(default)]
pub struct Settings {
    pub sound_mode: SoundMode,
    pub fade_away: bool,
    pub fade_after_seconds: f32,
    pub clear_after: usize,
    pub letter_grouping_timeout_seconds: f32,
    pub group_letters: bool,
    pub pointer_sound: PointerSound,
    pub sine_wave_volume_percent: u8,
    pub right_click_piano_enabled: bool,
    pub right_click_piano_scale: PianoScale,
    pub right_click_piano_key: PianoKey,
    pub spawn_animations: bool,
    pub interaction_animations: bool,
    pub faces_on_shapes: bool,
    pub background_brightness_percent: u8,
    pub cursor_style: CursorStyle,
    pub cursor_effect: CursorEffect,
    pub force_uppercase: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound_mode: SoundMode::Speech,
            fade_away: true,
            fade_after_seconds: 4.0,
            clear_after: 30,
            letter_grouping_timeout_seconds: 1.0,
            group_letters: true,
            pointer_sound: PointerSound::SineWave,
            sine_wave_volume_percent: 35,
            right_click_piano_enabled: true,
            right_click_piano_scale: PianoScale::Chromatic,
            right_click_piano_key: PianoKey::C,
            spawn_animations: true,
            interaction_animations: true,
            faces_on_shapes: true,
            background_brightness_percent: 7,
            cursor_style: CursorStyle::Hand,
            cursor_effect: CursorEffect::Rainbow,
            force_uppercase: true,
        }
    }
}

impl Settings {
    pub fn normalize(&mut self) {
        self.fade_after_seconds = self
            .fade_after_seconds
            .clamp(MIN_FADE_AFTER_SECONDS, MAX_FADE_AFTER_SECONDS);
        self.clear_after = self.clear_after.clamp(MIN_ITEMS_KEPT, MAX_ITEMS_KEPT);
        self.letter_grouping_timeout_seconds =
            self.letter_grouping_timeout_seconds.clamp(0.1, 10.0);
        self.sine_wave_volume_percent = self.sine_wave_volume_percent.clamp(5, 70);
        self.background_brightness_percent = self.background_brightness_percent.min(100);
    }
}

#[derive(Debug)]
pub struct SettingsStore {
    path: PathBuf,
    pub warning: Option<String>,
}

impl SettingsStore {
    pub fn open() -> (Self, Settings) {
        let path = ProjectDirs::from("com", "KeySlam", "KeySlam").map_or_else(
            || PathBuf::from("settings.json"),
            |dirs| dirs.config_dir().join("settings.json"),
        );
        migrate_legacy_settings(&path);
        Self::open_path(path)
    }

    fn open_path(path: PathBuf) -> (Self, Settings) {
        let mut store = Self {
            path,
            warning: None,
        };
        let mut settings = match fs::read(&store.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                store.warning = Some(format!(
                    "Settings were invalid and defaults were loaded: {error}"
                ));
                Settings::default()
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Settings::default(),
            Err(error) => {
                store.warning = Some(format!("Settings could not be read: {error}"));
                Settings::default()
            }
        };
        settings.normalize();
        (store, settings)
    }

    pub fn save(&mut self, settings: &Settings) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
        let mut file = AtomicWriteFile::open(&self.path)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.commit()?;
        self.warning = None;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn migrate_legacy_settings(path: &Path) {
    if path.exists() {
        return;
    }
    let Some(legacy_dirs) = ProjectDirs::from("com", "BabySmash", "BabySmash Rust") else {
        return;
    };
    let legacy_path = legacy_dirs.config_dir().join("settings.json");
    if !legacy_path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::copy(legacy_path, path);
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
        assert_eq!(settings.cursor_style, CursorStyle::Hand);
    }

    #[test]
    fn unsafe_values_are_clamped() {
        let mut settings = Settings {
            fade_after_seconds: -4.0,
            clear_after: usize::MAX,
            letter_grouping_timeout_seconds: f32::INFINITY,
            sine_wave_volume_percent: 100,
            background_brightness_percent: 250,
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.fade_after_seconds, MIN_FADE_AFTER_SECONDS);
        assert_eq!(settings.clear_after, MAX_ITEMS_KEPT);
        assert_eq!(settings.letter_grouping_timeout_seconds, 10.0);
        assert_eq!(settings.sine_wave_volume_percent, 70);
        assert_eq!(settings.background_brightness_percent, 100);
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
    fn removed_bump_map_setting_migrates_to_no_effect() -> serde_json::Result<()> {
        let effect = serde_json::from_str::<CursorEffect>("\"BumpMapTrail\"")?;
        assert_eq!(effect, CursorEffect::None);
        Ok(())
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
