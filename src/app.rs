use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, bounded};
use eframe::egui::{
    self, Align2, Color32, ComboBox, Context, Event, FontId, Frame, Key, PointerButton, RichText,
    Ui, Vec2, ViewportBuilder, ViewportCommand, ViewportId, WindowLevel, vec2,
};
use rand::Rng;

use crate::{
    audio::AudioSystem,
    game::{FigureKind, Game, pointer_tone},
    localization::Localization,
    platform::{KeyboardGuard, PlatformEvent, install_keyboard_guard},
    render::{self, TextureCache},
    responses::response_for,
    settings::{CursorEffect, CursorStyle, PointerSound, Settings, SettingsStore, SoundMode},
    speech::SpeechSystem,
};

const LAUGHTER_SOUNDS: [&str; 6] = [
    "giggle.wav",
    "babylaugh.wav",
    "babygigl2.wav",
    "ccgiggle.wav",
    "laughingmice.wav",
    "scooby2.wav",
];

#[derive(Debug, Clone)]
pub struct DisplayConfig {
    pub id: u32,
    pub name: String,
    pub position: [f32; 2],
    pub size: Vec2,
    pub primary: bool,
    pub kiosk: bool,
}

impl DisplayConfig {
    pub fn viewport(&self) -> ViewportBuilder {
        let mut builder = ViewportBuilder::default()
            .with_title(format!("BabySmash! — {}", self.name))
            .with_position(self.position)
            .with_inner_size(self.size)
            .with_resizable(!self.kiosk)
            .with_decorations(!self.kiosk);
        if self.kiosk {
            builder = builder
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_taskbar(false);
        }
        builder
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionsTab {
    Audio,
    Visuals,
    Input,
    Letters,
}

pub struct BabySmashApp {
    displays: Vec<DisplayConfig>,
    settings: Settings,
    draft_settings: Settings,
    settings_store: SettingsStore,
    localization: Localization,
    game: Game,
    audio: AudioSystem,
    speech: SpeechSystem,
    textures: TextureCache,
    platform_events: Receiver<PlatformEvent>,
    _keyboard_guard: Option<KeyboardGuard>,
    options_open: bool,
    options_tab: OptionsTab,
    help_visible: bool,
    status: Option<String>,
    last_frame: Instant,
    frame_seconds: f32,
    show_fps: bool,
    frames: u32,
    fps: f32,
    fps_started: Instant,
}

impl BabySmashApp {
    pub fn new(displays: Vec<DisplayConfig>, show_fps: bool) -> Self {
        let (settings_store, settings) = SettingsStore::open();
        let localization = Localization::detect();
        let audio = AudioSystem::new();
        let speech = SpeechSystem::new(localization.locale());
        if settings.startup_sound {
            audio.play_sound("EditedJackPlaysBabySmash.wav");
        }
        let (platform_sender, platform_events) = bounded(128);
        let (keyboard_guard, keyboard_warning) = match install_keyboard_guard(platform_sender) {
            Ok(guard) => (Some(guard), None),
            Err(error) => (None, Some(error)),
        };
        let status = settings_store.warning.clone().or(keyboard_warning);
        let sizes = displays
            .iter()
            .map(|display| display.size)
            .collect::<Vec<_>>();
        Self {
            displays,
            draft_settings: settings.clone(),
            settings,
            settings_store,
            localization,
            game: Game::new(sizes),
            audio,
            speech,
            textures: TextureCache::default(),
            platform_events,
            _keyboard_guard: keyboard_guard,
            options_open: false,
            options_tab: OptionsTab::Audio,
            help_visible: true,
            status,
            last_frame: Instant::now(),
            frame_seconds: 1.0 / 60.0,
            show_fps,
            frames: 0,
            fps: 0.0,
            fps_started: Instant::now(),
        }
    }

    fn process_platform_events(&mut self, ctx: &Context) {
        while let Ok(event) = self.platform_events.try_recv() {
            match event {
                PlatformEvent::Exit => {
                    ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close)
                }
                PlatformEvent::Key(key) if !self.options_open => self.process_key(&key),
                PlatformEvent::Key(_) => {}
            }
        }
    }

    fn process_key(&mut self, key_name: &str) {
        self.help_visible = false;
        let now = Instant::now();
        self.game
            .add_response(response_for(key_name), &self.settings, now);
        match self.settings.sound_mode {
            SoundMode::None => {}
            SoundMode::Laughter => self.play_laughter(),
            SoundMode::Speech => {
                if let Some(figure) = self.game.figures.back() {
                    let phrase = match figure.kind {
                        FigureKind::Glyph(glyph) => self.localization.text(&glyph.to_string()),
                        FigureKind::Emoji(_) => figure.spoken_text.clone(),
                        FigureKind::Shape(shape) => self
                            .localization
                            .color_shape(figure.color.name, shape.name()),
                    };
                    self.speech.speak(phrase);
                }
            }
        }
    }

    fn play_laughter(&self) {
        let index = rand::rng().random_range(0..LAUGHTER_SOUNDS.len());
        self.audio.play_sound(LAUGHTER_SOUNDS[index]);
    }

    fn render_viewport(&mut self, ui: &mut Ui, display_index: usize) {
        let now = Instant::now();
        let rect = ui.max_rect();
        if let Some(display) = self.game.displays.get_mut(display_index) {
            display.size = rect.size();
            display.pointer.update(now, self.frame_seconds);
        }
        self.handle_input(ui, display_index, now);

        let channel = (f32::from(self.settings.background_brightness_percent) * 2.55).round() as u8;
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from_gray(channel));
        for figure in &self.game.figures {
            render::draw_figure(
                ui.painter(),
                ui.ctx(),
                &mut self.textures,
                figure,
                display_index,
                now,
                self.settings.faces_on_shapes,
            );
        }
        if let Some(display) = self.game.displays.get(display_index) {
            render::draw_pointer_effects(ui.painter(), &display.pointer, now);
            if !self.options_open
                && let Some(position) = display.pointer.position
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                render::draw_cursor(ui.painter(), position, self.settings.cursor_style);
            } else if self.options_open {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
            }
        }

        if display_index == 0 && self.help_visible {
            let light_background = self.settings.background_brightness_percent >= 55;
            let primary = if light_background {
                Color32::BLACK
            } else {
                Color32::WHITE
            };
            let secondary = if light_background {
                Color32::from_gray(55)
            } else {
                Color32::from_gray(205)
            };
            ui.painter().text(
                rect.left_top() + vec2(15.0, 15.0),
                Align2::LEFT_TOP,
                "BabySmash! for Rust",
                FontId::proportional(24.0),
                primary,
            );
            ui.painter().text(
                rect.left_top() + vec2(15.0, 52.0),
                Align2::LEFT_TOP,
                "Press any key to start!",
                FontId::proportional(14.0),
                secondary,
            );
            ui.painter().text(
                rect.left_top() + vec2(15.0, 76.0),
                Align2::LEFT_TOP,
                "Alt+F4 exits • Alt+O opens settings",
                FontId::proportional(12.0),
                secondary,
            );
        }

        if display_index == 0 && self.show_fps {
            ui.painter().text(
                rect.right_top() + vec2(-15.0, 15.0),
                Align2::RIGHT_TOP,
                format!("FPS: {:.0} | Items: {}", self.fps, self.game.figures.len()),
                FontId::monospace(14.0),
                Color32::LIGHT_GREEN,
            );
        }
        if display_index == 0
            && let Some(status) = self.current_status()
        {
            ui.painter().text(
                rect.left_bottom() + vec2(15.0, -15.0),
                Align2::LEFT_BOTTOM,
                status,
                FontId::proportional(12.0),
                Color32::from_rgb(255, 190, 90),
            );
        }
    }

    fn handle_input(&mut self, ui: &Ui, display_index: usize, now: Instant) {
        let events = ui.input(|input| input.raw.events.clone());
        for event in events {
            if let Event::Key {
                key: Key::O,
                pressed: true,
                modifiers,
                ..
            } = &event
                && modifiers.alt
            {
                self.open_options();
                continue;
            }
            if let Event::Key {
                key: Key::F4,
                pressed: true,
                modifiers,
                ..
            } = &event
                && modifiers.alt
            {
                ui.ctx()
                    .send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close);
                continue;
            }
            if self.options_open {
                continue;
            }
            match event {
                Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    ..
                } => {
                    let selected = physical_key.unwrap_or(key);
                    self.process_key(selected.name());
                }
                Event::PointerMoved(position) => {
                    let effect = self.settings.cursor_effect;
                    if let Some(display) = self.game.displays.get_mut(display_index) {
                        display.pointer.move_to(position, effect, now);
                        if display.pointer.primary_down
                            && self.settings.pointer_sound == PointerSound::SineWave
                        {
                            let (frequency, pan) = pointer_tone(position, display.size);
                            self.audio.start_or_update_sine(
                                frequency,
                                pan,
                                f32::from(self.settings.sine_wave_volume_percent) / 100.0,
                            );
                        }
                    }
                }
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    ..
                } => {
                    if pressed {
                        if let Some(display) = self.game.displays.get_mut(display_index) {
                            display.pointer.press(pos, self.settings.cursor_effect, now);
                        }
                        let clicked = self.game.interact_at(
                            display_index,
                            pos,
                            now,
                            self.settings.interaction_animations,
                        );
                        if clicked && self.settings.interaction_sounds {
                            self.play_laughter();
                        }
                        match self.settings.pointer_sound {
                            PointerSound::Click => self.audio.play_sound("smallbumblebee.wav"),
                            PointerSound::SineWave => {
                                if let Some(display) = self.game.displays.get(display_index) {
                                    let (frequency, pan) = pointer_tone(pos, display.size);
                                    self.audio.start_or_update_sine(
                                        frequency,
                                        pan,
                                        f32::from(self.settings.sine_wave_volume_percent) / 100.0,
                                    );
                                }
                            }
                            PointerSound::None => {}
                        }
                    } else {
                        if let Some(display) = self.game.displays.get_mut(display_index) {
                            display.pointer.release(now);
                        }
                        self.audio.stop_sine();
                    }
                }
                Event::PointerButton {
                    pos,
                    button: PointerButton::Secondary,
                    pressed: true,
                    ..
                } if self.settings.right_click_piano_enabled => {
                    if let Some(display) = self.game.displays.get(display_index) {
                        let (frequency, _) = pointer_tone(pos, display.size);
                        self.audio.play_piano(frequency);
                    }
                }
                Event::MouseWheel { delta, .. } if delta.y != 0.0 => {
                    self.settings.cursor_effect = self.settings.cursor_effect.cycle(delta.y > 0.0);
                    if let Err(error) = self.settings_store.save(&self.settings) {
                        self.status = Some(format!("Could not save pointer effect: {error}"));
                    }
                }
                Event::PointerGone => {
                    if let Some(display) = self.game.displays.get_mut(display_index) {
                        display.pointer.release(now);
                        display.pointer.position = None;
                    }
                    self.audio.stop_sine();
                }
                _ => {}
            }
        }
    }

    fn open_options(&mut self) {
        self.audio.stop_sine();
        self.draft_settings = self.settings.clone();
        self.options_open = true;
    }

    fn show_options(&mut self, ctx: &Context) {
        if !self.options_open {
            return;
        }
        let mut open = true;
        egui::Window::new("BabySmash! Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size(vec2(720.0, 650.0))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.heading("Settings");
                ui.label("Tune BabySmash for your child, room, and play style.");
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    for (tab, label) in [
                        (OptionsTab::Audio, "Audio"),
                        (OptionsTab::Visuals, "Visuals"),
                        (OptionsTab::Input, "Input"),
                        (OptionsTab::Letters, "Letters"),
                    ] {
                        ui.selectable_value(&mut self.options_tab, tab, label);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(480.0)
                    .show(ui, |ui| match self.options_tab {
                        OptionsTab::Audio => audio_options(ui, &mut self.draft_settings),
                        OptionsTab::Visuals => visual_options(ui, &mut self.draft_settings),
                        OptionsTab::Input => input_options(ui, &mut self.draft_settings),
                        OptionsTab::Letters => letter_options(ui, &mut self.draft_settings),
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).small(),
                    );
                    ui.hyperlink_to(
                        "Send feedback on GitHub",
                        "https://github.com/RygerXD/babysmash/issues",
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Save changes").clicked() {
                            self.draft_settings.normalize();
                            self.settings = self.draft_settings.clone();
                            match self.settings_store.save(&self.settings) {
                                Ok(()) => self.status = None,
                                Err(error) => {
                                    self.status = Some(format!("Could not save settings: {error}"))
                                }
                            }
                            self.options_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.options_open = false;
                        }
                    });
                });
                ui.label(
                    RichText::new(format!(
                        "Settings: {}",
                        self.settings_store.path().display()
                    ))
                    .small()
                    .weak(),
                );
            });
        if !open {
            self.options_open = false;
        }
    }

    fn current_status(&self) -> Option<String> {
        self.status
            .clone()
            .or_else(|| self.audio.status())
            .or_else(|| self.speech.status())
    }
}

impl eframe::App for BabySmashApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();
        self.frame_seconds = now
            .duration_since(self.last_frame)
            .as_secs_f32()
            .clamp(1.0 / 240.0, 0.1);
        self.last_frame = now;
        self.frames = self.frames.saturating_add(1);
        let fps_elapsed = now.duration_since(self.fps_started).as_secs_f32();
        if fps_elapsed >= 1.0 {
            self.fps = self.frames as f32 / fps_elapsed;
            self.frames = 0;
            self.fps_started = now;
        }

        self.process_platform_events(&ctx);
        let background = Color32::from_gray(
            (f32::from(self.settings.background_brightness_percent) * 2.55).round() as u8,
        );
        Frame::new()
            .fill(background)
            .show(ui, |ui| self.render_viewport(ui, 0));
        self.show_options(&ctx);

        for display_index in 1..self.displays.len() {
            let display = self.displays[display_index].clone();
            let viewport_id = ViewportId::from_hash_of(("babysmash-display", display.id));
            ctx.show_viewport_immediate(viewport_id, display.viewport(), |ui, _class| {
                self.render_viewport(ui, display_index);
            });
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

fn section(ui: &mut Ui, title: &str, content: impl FnOnce(&mut Ui)) {
    Frame::group(ui.style()).inner_margin(14.0).show(ui, |ui| {
        ui.heading(title);
        ui.add_space(5.0);
        content(ui);
    });
    ui.add_space(10.0);
}

fn audio_options(ui: &mut Ui, settings: &mut Settings) {
    section(ui, "Key responses", |ui| {
        ui.label("Choose what happens when an item appears.");
        ComboBox::from_id_salt("sound-mode")
            .selected_text(match settings.sound_mode {
                SoundMode::Speech => "Speak the item",
                SoundMode::Laughter => "Play laughter",
                SoundMode::None => "No key sound",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut settings.sound_mode,
                    SoundMode::Speech,
                    "Speak the item",
                );
                ui.selectable_value(
                    &mut settings.sound_mode,
                    SoundMode::Laughter,
                    "Play laughter",
                );
                ui.selectable_value(&mut settings.sound_mode, SoundMode::None, "No key sound");
            });
    });
    section(ui, "Other sounds", |ui| {
        ui.checkbox(
            &mut settings.startup_sound,
            "Play the welcome sound at startup",
        );
        ui.checkbox(
            &mut settings.interaction_sounds,
            "Play sounds when scrolling and tapping shapes",
        );
    });
    section(ui, "Pointer sounds", |ui| {
        ComboBox::from_id_salt("pointer-sound")
            .selected_text(match settings.pointer_sound {
                PointerSound::Click => "Playful click",
                PointerSound::SineWave => "Sinewave instrument",
                PointerSound::None => "No pointer sound",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut settings.pointer_sound,
                    PointerSound::Click,
                    "Playful click",
                );
                ui.selectable_value(
                    &mut settings.pointer_sound,
                    PointerSound::SineWave,
                    "Sinewave instrument",
                );
                ui.selectable_value(
                    &mut settings.pointer_sound,
                    PointerSound::None,
                    "No pointer sound",
                );
            });
        ui.add_enabled_ui(settings.pointer_sound == PointerSound::SineWave, |ui| {
            ui.add(
                egui::Slider::new(&mut settings.sine_wave_volume_percent, 5..=70)
                    .text("Sinewave volume (%)"),
            );
            ui.label("Move vertically for pitch and horizontally for stereo position.");
        });
        ui.checkbox(
            &mut settings.right_click_piano_enabled,
            "Play piano keys on right-click",
        );
    });
}

fn visual_options(ui: &mut Ui, settings: &mut Settings) {
    section(ui, "Shapes and animation", |ui| {
        ui.checkbox(&mut settings.faces_on_shapes, "Show faces on shapes");
        ui.checkbox(
            &mut settings.spawn_animations,
            "Animate items when they appear",
        );
        ui.checkbox(
            &mut settings.interaction_animations,
            "Animate shapes when tapped or scrolled",
        );
    });
    section(ui, "Cleanup", |ui| {
        ui.checkbox(&mut settings.fade_away, "Fade items away");
        ui.add_enabled_ui(settings.fade_away, |ui| {
            ui.add(
                egui::Slider::new(&mut settings.fade_after_seconds, 1.0..=120.0)
                    .text("Fade duration (seconds)"),
            );
        });
        ui.add(egui::Slider::new(&mut settings.clear_after, 5..=200).text("Items kept on screen"));
    });
    section(ui, "Display", |ui| {
        ui.add(
            egui::Slider::new(&mut settings.background_brightness_percent, 0..=100)
                .text("Background brightness (%)"),
        );
    });
}

fn input_options(ui: &mut Ui, settings: &mut Settings) {
    section(ui, "Mouse and pointer", |ui| {
        ComboBox::from_id_salt("cursor-style")
            .selected_text(match settings.cursor_style {
                CursorStyle::Arrow => "Arrow",
                CursorStyle::Hand => "Hand",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut settings.cursor_style, CursorStyle::Arrow, "Arrow");
                ui.selectable_value(&mut settings.cursor_style, CursorStyle::Hand, "Hand");
            });
        ComboBox::from_id_salt("cursor-effect")
            .selected_text(match settings.cursor_effect {
                CursorEffect::None => "None",
                CursorEffect::Rainbow => "Rainbow ribbon",
                CursorEffect::Sparkles => "Sparkles",
                CursorEffect::Bubbles => "Bubbles",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut settings.cursor_effect, CursorEffect::None, "None");
                ui.selectable_value(
                    &mut settings.cursor_effect,
                    CursorEffect::Rainbow,
                    "Rainbow ribbon",
                );
                ui.selectable_value(
                    &mut settings.cursor_effect,
                    CursorEffect::Sparkles,
                    "Sparkles",
                );
                ui.selectable_value(
                    &mut settings.cursor_effect,
                    CursorEffect::Bubbles,
                    "Bubbles",
                );
            });
        ui.label("Use the mouse wheel during play to cycle through the effects.");
    });
}

fn letter_options(ui: &mut Ui, settings: &mut Settings) {
    section(ui, "Words", |ui| {
        ui.checkbox(
            &mut settings.group_letters,
            "Line up consecutive letters to form words",
        );
        ui.add_enabled_ui(settings.group_letters, |ui| {
            ui.add(
                egui::Slider::new(&mut settings.letter_grouping_timeout_seconds, 0.1..=10.0)
                    .text("Start a new word after (seconds)"),
            );
        });
    });
    section(ui, "Typography", |ui| {
        ui.checkbox(&mut settings.force_uppercase, "Use uppercase letters");
    });
}

pub fn display_configs(windowed: bool) -> Vec<DisplayConfig> {
    if windowed {
        return vec![DisplayConfig {
            id: 0,
            name: "Windowed".to_owned(),
            position: [80.0, 80.0],
            size: vec2(1280.0, 800.0),
            primary: true,
            kiosk: false,
        }];
    }
    let mut displays = display_info::DisplayInfo::all()
        .unwrap_or_default()
        .into_iter()
        .map(|display| {
            let scale = display.scale_factor.max(1.0);
            DisplayConfig {
                id: display.id,
                name: if display.friendly_name.is_empty() {
                    display.name
                } else {
                    display.friendly_name
                },
                position: [display.x as f32 / scale, display.y as f32 / scale],
                size: vec2(display.width as f32 / scale, display.height as f32 / scale),
                primary: display.is_primary,
                kiosk: true,
            }
        })
        .collect::<Vec<_>>();
    displays.sort_by_key(|display| !display.primary);
    if displays.is_empty() {
        displays.push(DisplayConfig {
            id: 0,
            name: "Primary display".to_owned(),
            position: [0.0, 0.0],
            size: vec2(1280.0, 800.0),
            primary: true,
            kiosk: true,
        });
    }
    displays
}
