use std::time::{Duration, Instant};

use crate::{
    audio::AudioSystem,
    game::{BabyColor, COLORS, FigureKind, Game, pointer_tone},
    localization::Localization,
    platform::{KeyboardGuard, PlatformEvent, install_keyboard_guard, pressed_numpad_key},
    render::{self, TextureCache},
    responses::response_for,
    settings::{CursorEffect, CursorStyle, PointerSound, Settings, SettingsStore, SoundMode},
    speech::SpeechSystem,
};
use crossbeam_channel::{Receiver, bounded};
use eframe::egui::{
    self, Align2, Color32, ComboBox, Context, Event, FontId, Frame, Key, Painter, PointerButton,
    Pos2, Rect, RichText, Stroke, Ui, Vec2, ViewportBuilder, ViewportCommand, ViewportId,
    WindowLevel, pos2, vec2,
};

const MIN_BRUSH_SIZE: f32 = 6.0;
const MAX_BRUSH_SIZE: f32 = 96.0;
const MAX_PAINT_POINTS_PER_DISPLAY: usize = 20_000;

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

#[derive(Debug)]
struct PaintStroke {
    points: Vec<Pos2>,
    color: BabyColor,
    brush_size: f32,
}

#[derive(Debug)]
struct ColoringState {
    selected_color: usize,
    brush_size: f32,
    strokes: Vec<Vec<PaintStroke>>,
    active_strokes: Vec<Option<usize>>,
    slider_dragging: Vec<bool>,
    point_counts: Vec<usize>,
}

impl ColoringState {
    fn new(display_count: usize) -> Self {
        Self {
            selected_color: 0,
            brush_size: 36.0,
            strokes: (0..display_count).map(|_| Vec::new()).collect(),
            active_strokes: vec![None; display_count],
            slider_dragging: vec![false; display_count],
            point_counts: vec![0; display_count],
        }
    }

    fn begin_stroke(&mut self, display_index: usize, position: Pos2) {
        self.end_stroke(display_index);
        while self.point_counts[display_index] >= MAX_PAINT_POINTS_PER_DISPLAY {
            let Some(removed) = self.strokes[display_index].first() else {
                self.point_counts[display_index] = 0;
                break;
            };
            self.point_counts[display_index] =
                self.point_counts[display_index].saturating_sub(removed.points.len());
            self.strokes[display_index].remove(0);
        }
        self.strokes[display_index].push(PaintStroke {
            points: vec![position],
            color: COLORS[self.selected_color],
            brush_size: self.brush_size,
        });
        self.point_counts[display_index] += 1;
        self.active_strokes[display_index] = Some(self.strokes[display_index].len() - 1);
    }

    fn extend_stroke(&mut self, display_index: usize, position: Pos2) {
        if self.point_counts[display_index] >= MAX_PAINT_POINTS_PER_DISPLAY {
            return;
        }
        let Some(stroke_index) = self.active_strokes[display_index] else {
            return;
        };
        let Some(stroke) = self.strokes[display_index].get_mut(stroke_index) else {
            self.active_strokes[display_index] = None;
            return;
        };
        if stroke
            .points
            .last()
            .is_none_or(|last| last.distance_sq(position) >= 2.25)
        {
            stroke.points.push(position);
            self.point_counts[display_index] += 1;
        }
    }

    fn end_stroke(&mut self, display_index: usize) {
        self.active_strokes[display_index] = None;
        self.slider_dragging[display_index] = false;
    }

    fn end_all_strokes(&mut self) {
        self.active_strokes.fill(None);
        self.slider_dragging.fill(false);
    }

    fn clear(&mut self) {
        for strokes in &mut self.strokes {
            strokes.clear();
        }
        self.point_counts.fill(0);
        self.end_all_strokes();
    }
}

#[derive(Debug)]
struct ColoringLayout {
    swatches: Vec<Rect>,
    clear_button: Rect,
    bottom_panel: Rect,
    slider_panel: Rect,
    slider_track: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColoringControl {
    Canvas,
    Swatch(usize),
    Clear,
    BrushSlider,
    Panel,
}

fn coloring_layout(display: Vec2) -> ColoringLayout {
    let margin = 12.0;
    let gap = 6.0;
    let clear_width = 112.0_f32.min((display.x * 0.18).max(72.0));
    let swatch_size =
        ((display.x - margin * 2.0 - clear_width - gap * 12.0) / 12.0).clamp(12.0, 42.0);
    let total_width = swatch_size * 12.0 + gap * 12.0 + clear_width;
    let start_x = ((display.x - total_width) / 2.0).max(4.0);
    let swatch_top = (display.y - margin - swatch_size).max(0.0);
    let swatches = (0..COLORS.len())
        .map(|index| {
            let left = start_x + index as f32 * (swatch_size + gap);
            Rect::from_min_size(pos2(left, swatch_top), Vec2::splat(swatch_size))
        })
        .collect::<Vec<_>>();
    let clear_left = start_x + 12.0 * (swatch_size + gap);
    let clear_button =
        Rect::from_min_size(pos2(clear_left, swatch_top), vec2(clear_width, swatch_size));
    let bottom_panel = Rect::from_min_max(
        pos2((start_x - 8.0).max(0.0), (swatch_top - 8.0).max(0.0)),
        pos2((clear_button.right() + 8.0).min(display.x), display.y),
    );

    let slider_bottom = (bottom_panel.top() - 14.0).max(112.0);
    let slider_panel = Rect::from_min_max(pos2(10.0, 58.0), pos2(62.0, slider_bottom));
    let slider_track = Rect::from_min_max(
        pos2(slider_panel.center().x - 4.0, slider_panel.top() + 16.0),
        pos2(slider_panel.center().x + 4.0, slider_panel.bottom() - 16.0),
    );

    ColoringLayout {
        swatches,
        clear_button,
        bottom_panel,
        slider_panel,
        slider_track,
    }
}

fn coloring_control_at(position: Pos2, layout: &ColoringLayout) -> ColoringControl {
    if let Some(index) = layout
        .swatches
        .iter()
        .position(|swatch| swatch.contains(position))
    {
        ColoringControl::Swatch(index)
    } else if layout.clear_button.contains(position) {
        ColoringControl::Clear
    } else if layout.slider_panel.contains(position) {
        ColoringControl::BrushSlider
    } else if layout.bottom_panel.contains(position) {
        ColoringControl::Panel
    } else {
        ColoringControl::Canvas
    }
}

fn brush_size_for(position: Pos2, track: Rect) -> f32 {
    let progress = ((track.bottom() - position.y) / track.height().max(1.0)).clamp(0.0, 1.0);
    MIN_BRUSH_SIZE + (MAX_BRUSH_SIZE - MIN_BRUSH_SIZE) * progress
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
    coloring: ColoringState,
    platform_events: Receiver<PlatformEvent>,
    _keyboard_guard: Option<KeyboardGuard>,
    options_open: bool,
    options_tab: OptionsTab,
    status: Option<String>,
    last_frame: Instant,
    frame_seconds: f32,
}

impl BabySmashApp {
    pub fn new(displays: Vec<DisplayConfig>) -> Self {
        let (settings_store, settings) = SettingsStore::open();
        let localization = Localization::detect();
        let audio = AudioSystem::new();
        let speech = SpeechSystem::new(localization.locale());
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
        let coloring = ColoringState::new(displays.len());
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
            coloring,
            platform_events,
            _keyboard_guard: keyboard_guard,
            options_open: false,
            options_tab: OptionsTab::Audio,
            status,
            last_frame: Instant::now(),
            frame_seconds: 1.0 / 60.0,
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
        let now = Instant::now();
        self.game
            .add_response(response_for(key_name), &self.settings, now);
        match self.settings.sound_mode {
            SoundMode::None => {}
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

    fn draw_painting(&self, painter: &Painter, display_index: usize) {
        let Some(strokes) = self.coloring.strokes.get(display_index) else {
            return;
        };
        for stroke in strokes {
            let color = Color32::from_rgb(
                stroke.color.rgb[0],
                stroke.color.rgb[1],
                stroke.color.rgb[2],
            );
            if let [point] = stroke.points.as_slice() {
                painter.circle_filled(*point, stroke.brush_size / 2.0, color);
            } else {
                for pair in stroke.points.windows(2) {
                    painter.line_segment([pair[0], pair[1]], Stroke::new(stroke.brush_size, color));
                }
            }
        }
    }

    fn draw_coloring_controls(&self, painter: &Painter, display: Vec2) {
        let layout = coloring_layout(display);
        painter.rect_filled(layout.bottom_panel, 10.0, Color32::from_black_alpha(205));
        painter.rect_filled(layout.slider_panel, 10.0, Color32::from_black_alpha(205));

        for (index, swatch) in layout.swatches.iter().enumerate() {
            let center = swatch.center();
            let radius = swatch.width().min(swatch.height()) / 2.0 - 2.0;
            if index == self.coloring.selected_color {
                painter.circle_filled(center, radius + 3.0, Color32::WHITE);
                painter.circle_filled(center, radius + 1.0, Color32::BLACK);
            }
            let color = COLORS[index];
            painter.circle_filled(
                center,
                radius,
                Color32::from_rgb(color.rgb[0], color.rgb[1], color.rgb[2]),
            );
        }

        painter.rect_filled(layout.clear_button, 7.0, Color32::from_rgb(115, 35, 35));
        painter.text(
            layout.clear_button.center(),
            Align2::CENTER_CENTER,
            "Clear screen",
            FontId::proportional((layout.clear_button.height() * 0.34).clamp(10.0, 16.0)),
            Color32::WHITE,
        );

        painter.line_segment(
            [
                layout.slider_track.center_top(),
                layout.slider_track.center_bottom(),
            ],
            Stroke::new(8.0, Color32::from_gray(85)),
        );
        let progress =
            (self.coloring.brush_size - MIN_BRUSH_SIZE) / (MAX_BRUSH_SIZE - MIN_BRUSH_SIZE);
        let thumb_y = layout.slider_track.bottom() - layout.slider_track.height() * progress;
        let thumb = pos2(layout.slider_track.center().x, thumb_y);
        let selected = COLORS[self.coloring.selected_color];
        painter.circle_filled(thumb, 12.0, Color32::WHITE);
        painter.circle_filled(
            thumb,
            9.0,
            Color32::from_rgb(selected.rgb[0], selected.rgb[1], selected.rgb[2]),
        );
    }

    fn render_viewport(&mut self, ui: &mut Ui, display_index: usize) {
        let now = Instant::now();
        let rect = ui.max_rect();
        if let Some(display) = self.game.displays.get_mut(display_index) {
            display.size = rect.size();
            display.pointer.update(now, self.frame_seconds);
        }
        self.handle_input(ui, display_index, now);

        let brightness = if self.options_open {
            self.draft_settings.background_brightness_percent
        } else {
            self.settings.background_brightness_percent
        };
        let channel = (f32::from(brightness) * 2.55).round() as u8;
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from_gray(channel));
        self.draw_painting(ui.painter(), display_index);
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
                if self.settings.cursor_effect == CursorEffect::Coloring {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                } else {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                    render::draw_cursor(ui.painter(), position, self.settings.cursor_style);
                }
            } else if self.options_open {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
            }
        }

        if display_index == 0 {
            let light_background = brightness >= 55;
            let secondary = if light_background {
                Color32::from_gray(55)
            } else {
                Color32::from_gray(205)
            };
            ui.painter().text(
                rect.left_top() + vec2(15.0, 15.0),
                Align2::LEFT_TOP,
                "Alt+F4 exits • Alt+O opens settings",
                FontId::proportional(14.0),
                secondary,
            );
        }
        if self.settings.cursor_effect == CursorEffect::Coloring {
            self.draw_coloring_controls(ui.painter(), rect.size());
        }
    }

    fn move_coloring_pointer(&mut self, display_index: usize, position: Pos2, now: Instant) {
        let Some(display) = self.game.displays.get_mut(display_index) else {
            return;
        };
        let layout = coloring_layout(display.size);
        if self.coloring.slider_dragging[display_index] {
            self.coloring.brush_size = brush_size_for(position, layout.slider_track);
        } else if self.coloring.active_strokes[display_index].is_some() {
            if coloring_control_at(position, &layout) == ColoringControl::Canvas {
                self.coloring.extend_stroke(display_index, position);
            } else {
                self.coloring.end_stroke(display_index);
            }
        }
        display
            .pointer
            .move_to(position, CursorEffect::Coloring, now);
    }

    fn press_coloring_pointer(
        &mut self,
        display_index: usize,
        position: Pos2,
        pressed: bool,
        now: Instant,
    ) {
        let Some(display) = self.game.displays.get_mut(display_index) else {
            return;
        };
        if !pressed {
            display.pointer.release(now);
            self.coloring.end_stroke(display_index);
            self.audio.stop_sine();
            return;
        }

        display.pointer.press(position, CursorEffect::Coloring, now);
        let layout = coloring_layout(display.size);
        match coloring_control_at(position, &layout) {
            ColoringControl::Canvas => self.coloring.begin_stroke(display_index, position),
            ColoringControl::Swatch(index) => {
                self.coloring.selected_color = index;
                self.coloring.end_stroke(display_index);
            }
            ColoringControl::Clear => {
                self.game.clear();
                self.coloring.clear();
            }
            ColoringControl::BrushSlider => {
                self.coloring.end_stroke(display_index);
                self.coloring.slider_dragging[display_index] = true;
                self.coloring.brush_size = brush_size_for(position, layout.slider_track);
            }
            ColoringControl::Panel => self.coloring.end_stroke(display_index),
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
                    let key_name = if matches!(
                        selected,
                        Key::Num0
                            | Key::Num1
                            | Key::Num2
                            | Key::Num3
                            | Key::Num4
                            | Key::Num5
                            | Key::Num6
                            | Key::Num7
                            | Key::Num8
                            | Key::Num9
                    ) {
                        pressed_numpad_key().unwrap_or(selected.name())
                    } else {
                        selected.name()
                    };
                    self.process_key(key_name);
                }
                Event::PointerMoved(position) => {
                    let effect = self.settings.cursor_effect;
                    if effect == CursorEffect::Coloring {
                        self.move_coloring_pointer(display_index, position, now);
                        continue;
                    }
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
                    if self.settings.cursor_effect == CursorEffect::Coloring {
                        self.press_coloring_pointer(display_index, pos, pressed, now);
                        continue;
                    }
                    if pressed {
                        if let Some(display) = self.game.displays.get_mut(display_index) {
                            display.pointer.press(pos, self.settings.cursor_effect, now);
                        }
                        self.game.interact_at(
                            display_index,
                            pos,
                            now,
                            self.settings.interaction_animations,
                        );
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
                    self.coloring.end_all_strokes();
                    self.settings.cursor_effect = self.settings.cursor_effect.cycle(delta.y > 0.0);
                    if let Err(error) = self.settings_store.save(&self.settings) {
                        self.status = Some(format!("Could not save pointer effect: {error}"));
                    }
                }
                Event::PointerGone => {
                    self.coloring.end_stroke(display_index);
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
        self.coloring.end_all_strokes();
        let now = Instant::now();
        for display in &mut self.game.displays {
            display.pointer.release(now);
        }
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
                if let Some(status) = self.current_status() {
                    ui.colored_label(Color32::from_rgb(180, 100, 0), status);
                }
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

        self.process_platform_events(&ctx);
        self.game.remove_expired(Instant::now());
        let brightness = if self.options_open {
            self.draft_settings.background_brightness_percent
        } else {
            self.settings.background_brightness_percent
        };
        let background = Color32::from_gray((f32::from(brightness) * 2.55).round() as u8);
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
                SoundMode::None => "No key sound",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut settings.sound_mode,
                    SoundMode::Speech,
                    "Speak the item",
                );
                ui.selectable_value(&mut settings.sound_mode, SoundMode::None, "No key sound");
            });
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
                    .text("Keep items visible for (seconds)"),
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
                CursorEffect::Coloring => "Coloring mode",
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
                ui.selectable_value(
                    &mut settings.cursor_effect,
                    CursorEffect::Coloring,
                    "Coloring mode",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coloring_layout_exposes_all_swatches_and_clear_button() {
        let layout = coloring_layout(vec2(1280.0, 800.0));
        assert_eq!(layout.swatches.len(), COLORS.len());
        for (index, swatch) in layout.swatches.iter().enumerate() {
            assert_eq!(
                coloring_control_at(swatch.center(), &layout),
                ColoringControl::Swatch(index)
            );
        }
        assert_eq!(
            coloring_control_at(layout.clear_button.center(), &layout),
            ColoringControl::Clear
        );
    }

    #[test]
    fn brush_slider_maps_bottom_to_small_and_top_to_large() {
        let layout = coloring_layout(vec2(1280.0, 800.0));
        assert_eq!(
            brush_size_for(layout.slider_track.center_bottom(), layout.slider_track),
            MIN_BRUSH_SIZE
        );
        assert_eq!(
            brush_size_for(layout.slider_track.center_top(), layout.slider_track),
            MAX_BRUSH_SIZE
        );
    }

    #[test]
    fn coloring_state_records_selected_background_strokes_and_clears_them() {
        let mut coloring = ColoringState::new(1);
        coloring.selected_color = COLORS.len() - 1;
        coloring.brush_size = 48.0;
        coloring.begin_stroke(0, pos2(100.0, 100.0));
        coloring.extend_stroke(0, pos2(140.0, 100.0));

        assert_eq!(coloring.strokes[0].len(), 1);
        assert_eq!(coloring.strokes[0][0].color.name, "Black");
        assert_eq!(coloring.strokes[0][0].brush_size, 48.0);
        assert_eq!(coloring.strokes[0][0].points.len(), 2);

        coloring.clear();
        assert!(coloring.strokes[0].is_empty());
        assert_eq!(coloring.point_counts[0], 0);
    }
}
