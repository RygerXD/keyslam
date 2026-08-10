use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use atomic_write_file::AtomicWriteFile;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

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
pub enum CursorStyle {
    Arrow,
    Hand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorEffect {
    None,
    Rainbow,
    Sparkles,
    Bubbles,
    Coloring,
}

impl CursorEffect {
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Rainbow,
        Self::Sparkles,
        Self::Bubbles,
        Self::Coloring,
    ];

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
            spawn_animations: true,
            interaction_animations: true,
            faces_on_shapes: true,
            background_brightness_percent: 7,
            cursor_style: CursorStyle::Arrow,
            cursor_effect: CursorEffect::Rainbow,
            force_uppercase: true,
        }
    }
}

impl Settings {
    pub fn normalize(&mut self) {
        self.fade_after_seconds = self.fade_after_seconds.clamp(1.0, 120.0);
        self.clear_after = self.clear_after.clamp(5, 200);
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
        let path = ProjectDirs::from("com", "BabySmash", "BabySmash Rust").map_or_else(
            || PathBuf::from("settings.json"),
            |dirs| dirs.config_dir().join("settings.json"),
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_settings_use_upstream_defaults() {
        let path = std::env::temp_dir().join(format!(
            "babysmash-settings-{}-missing.json",
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
            sine_wave_volume_percent: 100,
            background_brightness_percent: 250,
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.fade_after_seconds, 1.0);
        assert_eq!(settings.clear_after, 200);
        assert_eq!(settings.letter_grouping_timeout_seconds, 10.0);
        assert_eq!(settings.sine_wave_volume_percent, 70);
        assert_eq!(settings.background_brightness_percent, 100);
    }

    #[test]
    fn coloring_mode_is_part_of_the_scroll_wheel_cycle() {
        assert_eq!(CursorEffect::Bubbles.cycle(true), CursorEffect::Coloring);
        assert_eq!(CursorEffect::Coloring.cycle(true), CursorEffect::None);
        assert_eq!(CursorEffect::Coloring.cycle(false), CursorEffect::Bubbles);
    }

    #[test]
    fn settings_round_trip_through_atomic_file() -> io::Result<()> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("babysmash-settings-{}-{nonce}", std::process::id()));
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
