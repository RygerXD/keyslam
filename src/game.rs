use std::{collections::VecDeque, time::Instant};

use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};
use rand::Rng;

use crate::{
    responses::{KeyResponse, ResponseKind, ShapeKind},
    settings::{CursorEffect, Settings},
};

const LETTER_GAP: f32 = 8.0;
const LETTER_PADDING: f32 = 24.0;
const MAX_PARTICLES: usize = 72;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BabyColor {
    pub rgb: [u8; 3],
    pub name: &'static str,
}

pub const COLORS: [BabyColor; 9] = [
    BabyColor {
        rgb: [255, 0, 0],
        name: "Red",
    },
    BabyColor {
        rgb: [0, 0, 255],
        name: "Blue",
    },
    BabyColor {
        rgb: [255, 255, 0],
        name: "Yellow",
    },
    BabyColor {
        rgb: [0, 128, 0],
        name: "Green",
    },
    BabyColor {
        rgb: [128, 0, 128],
        name: "Purple",
    },
    BabyColor {
        rgb: [255, 192, 203],
        name: "Pink",
    },
    BabyColor {
        rgb: [255, 165, 0],
        name: "Orange",
    },
    BabyColor {
        rgb: [210, 180, 140],
        name: "Tan",
    },
    BabyColor {
        rgb: [128, 128, 128],
        name: "Gray",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FigureKind {
    Glyph(char),
    Emoji(&'static str),
    Shape(ShapeKind),
}

#[derive(Debug, Clone)]
pub struct Figure {
    pub id: u64,
    pub kind: FigureKind,
    pub color: BabyColor,
    pub spoken_text: String,
    pub created: Instant,
    pub fade_duration: Option<f32>,
    pub animate_spawn: bool,
    pub placements: Vec<Placement>,
}

impl Figure {
    pub fn opacity(&self, now: Instant) -> f32 {
        self.fade_duration.map_or(1.0, |seconds| {
            (1.0 - now.duration_since(self.created).as_secs_f32() / seconds).clamp(0.0, 1.0)
        })
    }

    pub fn spawn_transform(&self, now: Instant) -> (f32, f32) {
        if !self.animate_spawn {
            return (1.0, 0.0);
        }
        let progress = now
            .duration_since(self.created)
            .as_secs_f32()
            .clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        (eased, 360.0 * (1.0 - eased))
    }
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub top_left: Pos2,
    pub size: Vec2,
    pub interaction: Option<Interaction>,
}

impl Placement {
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.top_left, self.size)
    }

    pub fn interaction_transform(&self, now: Instant) -> (Vec2, f32) {
        let Some(interaction) = &self.interaction else {
            return (Vec2::splat(1.0), 0.0);
        };
        let progress = (now.duration_since(interaction.started).as_secs_f32()
            / interaction.kind.duration())
        .clamp(0.0, 1.0);
        interaction.kind.transform(progress)
    }
}

#[derive(Debug, Clone)]
pub struct Interaction {
    pub kind: InteractionKind,
    pub started: Instant,
}

#[derive(Debug, Clone, Copy)]
pub enum InteractionKind {
    Jiggle,
    Throb,
    Rotate,
    Snap,
}

impl InteractionKind {
    fn duration(self) -> f32 {
        match self {
            Self::Jiggle | Self::Throb => 0.5,
            Self::Rotate => 1.0,
            Self::Snap => 0.3,
        }
    }

    fn transform(self, progress: f32) -> (Vec2, f32) {
        const JIGGLE: [f32; 9] = [0.0, 10.0, 0.0, -10.0, 0.0, 5.0, 0.0, -5.0, 0.0];
        const THROB: [f32; 9] = [1.0, 1.1, 1.0, 0.9, 1.0, 1.05, 1.0, 0.95, 1.0];
        const ROTATE: [f32; 9] = [0.0, -5.0, 0.0, 90.0, 180.0, 270.0, 360.0, 365.0, 360.0];
        const SNAP: [f32; 3] = [1.0, 0.0, 1.0];
        match self {
            Self::Jiggle => (Vec2::splat(1.0), interpolate(&JIGGLE, progress)),
            Self::Throb => (Vec2::splat(interpolate(&THROB, progress)), 0.0),
            Self::Rotate => (Vec2::splat(1.0), interpolate(&ROTATE, progress)),
            Self::Snap => (vec2(1.0, interpolate(&SNAP, progress)), 0.0),
        }
    }
}

fn interpolate(frames: &[f32], progress: f32) -> f32 {
    let position = progress * (frames.len() - 1) as f32;
    let lower = position.floor() as usize;
    let upper = (lower + 1).min(frames.len() - 1);
    frames[lower] + (frames[upper] - frames[lower]) * (position - lower as f32)
}

#[derive(Debug)]
pub struct Game {
    pub figures: VecDeque<Figure>,
    pub displays: Vec<DisplayState>,
    next_id: u64,
    grouped_color: Option<(BabyColor, Instant)>,
}

impl Game {
    pub fn new(display_sizes: impl IntoIterator<Item = Vec2>) -> Self {
        Self {
            figures: VecDeque::new(),
            displays: display_sizes.into_iter().map(DisplayState::new).collect(),
            next_id: 1,
            grouped_color: None,
        }
    }

    pub fn add_response(
        &mut self,
        response: KeyResponse,
        settings: &Settings,
        now: Instant,
    ) -> String {
        let mut rng = rand::rng();
        let candidate = COLORS[rng.random_range(0..COLORS.len())];
        let (kind, default_speech, grouped_letter) = match response.kind {
            ResponseKind::Glyph(mut glyph) => {
                if !settings.force_uppercase {
                    glyph = glyph.to_ascii_lowercase();
                }
                (
                    FigureKind::Glyph(glyph),
                    glyph.to_string(),
                    glyph.is_ascii_alphabetic(),
                )
            }
            ResponseKind::Emoji(emoji) => (
                FigureKind::Emoji(emoji),
                response.spoken_text.to_owned(),
                false,
            ),
            ResponseKind::Shape(shape) => (
                FigureKind::Shape(shape),
                response.spoken_text.to_owned(),
                false,
            ),
        };
        let color = if grouped_letter && settings.group_letters {
            let timeout = settings.letter_grouping_timeout_seconds;
            if let Some((color, last)) = self.grouped_color {
                if now.duration_since(last).as_secs_f32() <= timeout {
                    self.grouped_color = Some((color, now));
                    color
                } else {
                    self.grouped_color = Some((candidate, now));
                    candidate
                }
            } else {
                self.grouped_color = Some((candidate, now));
                candidate
            }
        } else {
            self.grouped_color = None;
            candidate
        };

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let size = size_for(&kind);
        let placements = self
            .displays
            .iter()
            .map(|display| Placement {
                top_left: random_position(&mut rng, display.size, size),
                size,
                interaction: None,
            })
            .collect();
        self.figures.push_back(Figure {
            id,
            kind,
            color,
            spoken_text: default_speech.clone(),
            created: now,
            fade_duration: settings.fade_away.then_some(settings.fade_after_seconds),
            animate_spawn: settings.spawn_animations,
            placements,
        });

        for display_index in 0..self.displays.len() {
            self.update_letter_run(
                display_index,
                id,
                grouped_letter && settings.group_letters,
                now,
                settings,
            );
        }

        while self.figures.len() > settings.clear_after {
            if let Some(removed) = self.figures.pop_front() {
                for display in &mut self.displays {
                    display
                        .letter_run
                        .retain(|figure_id| *figure_id != removed.id);
                }
            }
        }
        default_speech
    }

    fn update_letter_run(
        &mut self,
        display_index: usize,
        figure_id: u64,
        continues: bool,
        now: Instant,
        settings: &Settings,
    ) {
        if !continues {
            self.displays[display_index].letter_run.clear();
            self.displays[display_index].last_letter = None;
            return;
        }

        let expired = self.displays[display_index]
            .last_letter
            .is_some_and(|last| {
                now.duration_since(last).as_secs_f32() > settings.letter_grouping_timeout_seconds
            });
        if expired {
            self.displays[display_index].letter_run.clear();
        }
        if self.displays[display_index].letter_run.is_empty()
            && let Some((anchor, top)) = self
                .placement(figure_id, display_index)
                .map(|placement| (placement.rect().center().x, placement.top_left.y))
        {
            self.displays[display_index].letter_anchor_x = anchor;
            self.displays[display_index].letter_top = top;
        }
        self.displays[display_index].letter_run.push(figure_id);
        self.displays[display_index].last_letter = Some(now);
        self.arrange_letter_run(display_index);
    }

    fn arrange_letter_run(&mut self, display_index: usize) {
        let display = &self.displays[display_index];
        let ids = display.letter_run.clone();
        let display_size = display.size;
        let anchor = display.letter_anchor_x;
        let requested_top = display.letter_top;
        let sizes: Vec<Vec2> = ids
            .iter()
            .filter_map(|id| {
                self.placement(*id, display_index)
                    .map(|placement| placement.size)
            })
            .collect();
        if sizes.is_empty() {
            return;
        }
        let total_width =
            sizes.iter().map(|size| size.x).sum::<f32>() + LETTER_GAP * (sizes.len() - 1) as f32;
        let max_left = (display_size.x - LETTER_PADDING - total_width).max(LETTER_PADDING);
        let mut left = if total_width >= (display_size.x - LETTER_PADDING * 2.0).max(0.0) {
            display_size.x - LETTER_PADDING - total_width
        } else {
            (anchor - total_width / 2.0).clamp(LETTER_PADDING, max_left)
        };
        let max_height = sizes.iter().map(|size| size.y).fold(0.0_f32, f32::max);
        let top = requested_top.clamp(
            LETTER_PADDING,
            (display_size.y - LETTER_PADDING - max_height).max(LETTER_PADDING),
        );
        for (id, size) in ids.into_iter().zip(sizes) {
            if let Some(placement) = self.placement_mut(id, display_index) {
                placement.top_left = pos2(left, top + (max_height - size.y) / 2.0);
                left += size.x + LETTER_GAP;
            }
        }
    }

    pub fn interact_at(
        &mut self,
        display_index: usize,
        point: Pos2,
        now: Instant,
        animate: bool,
    ) -> bool {
        for figure in self.figures.iter_mut().rev() {
            if figure.opacity(now) <= 0.1 {
                continue;
            }
            let Some(placement) = figure.placements.get_mut(display_index) else {
                continue;
            };
            if placement.rect().contains(point) {
                if animate {
                    placement.interaction = Some(Interaction {
                        kind: match rand::rng().random_range(0..4) {
                            0 => InteractionKind::Jiggle,
                            1 => InteractionKind::Throb,
                            2 => InteractionKind::Rotate,
                            _ => InteractionKind::Snap,
                        },
                        started: now,
                    });
                }
                return true;
            }
        }
        false
    }

    fn placement(&self, id: u64, display_index: usize) -> Option<&Placement> {
        self.figures
            .iter()
            .find(|figure| figure.id == id)
            .and_then(|figure| figure.placements.get(display_index))
    }

    fn placement_mut(&mut self, id: u64, display_index: usize) -> Option<&mut Placement> {
        self.figures
            .iter_mut()
            .find(|figure| figure.id == id)
            .and_then(|figure| figure.placements.get_mut(display_index))
    }
}

fn random_position(rng: &mut impl Rng, display: Vec2, figure: Vec2) -> Pos2 {
    pos2(
        rng.random_range(0.0..=(display.x - figure.x).max(0.0)),
        rng.random_range(0.0..=(display.y - figure.y).max(0.0)),
    )
}

pub fn size_for(kind: &FigureKind) -> Vec2 {
    match kind {
        FigureKind::Glyph(_) => vec2(220.0, 300.0),
        FigureKind::Emoji(_) => Vec2::splat(340.0),
        FigureKind::Shape(ShapeKind::Rectangle) => vec2(300.0, 207.0),
        FigureKind::Shape(ShapeKind::Oval) => vec2(300.0, 210.0),
        FigureKind::Shape(_) => Vec2::splat(240.0),
    }
}

#[derive(Debug)]
pub struct DisplayState {
    pub size: Vec2,
    pub letter_run: Vec<u64>,
    pub letter_anchor_x: f32,
    pub letter_top: f32,
    pub last_letter: Option<Instant>,
    pub pointer: PointerState,
}

impl DisplayState {
    fn new(size: Vec2) -> Self {
        Self {
            size,
            letter_run: Vec::new(),
            letter_anchor_x: 0.0,
            letter_top: 0.0,
            last_letter: None,
            pointer: PointerState::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct PointerState {
    pub position: Option<Pos2>,
    pub primary_down: bool,
    pub trail: RainbowTrail,
    pub particles: Vec<Particle>,
    last_effect_position: Option<Pos2>,
    next_effect_spacing: f32,
}

impl PointerState {
    pub fn press(&mut self, position: Pos2, effect: CursorEffect, now: Instant) {
        self.primary_down = true;
        self.position = Some(position);
        self.last_effect_position = None;
        self.emit(position, effect, now);
    }

    pub fn move_to(&mut self, position: Pos2, effect: CursorEffect, now: Instant) {
        self.position = Some(position);
        if self.primary_down {
            self.emit(position, effect, now);
        }
    }

    pub fn release(&mut self, now: Instant) {
        self.primary_down = false;
        self.last_effect_position = None;
        self.trail.stop(now);
    }

    pub fn update(&mut self, now: Instant, frame_seconds: f32) {
        self.trail.advance(now, frame_seconds);
        self.particles.retain(|particle| {
            now.duration_since(particle.created).as_secs_f32() < particle.duration
        });
    }

    fn emit(&mut self, position: Pos2, effect: CursorEffect, now: Instant) {
        match effect {
            CursorEffect::None => {
                self.last_effect_position = None;
                self.trail.stop(now);
            }
            CursorEffect::Rainbow => {
                self.last_effect_position = None;
                self.trail.move_to(position);
            }
            CursorEffect::Sparkles | CursorEffect::Bubbles => {
                self.trail.stop(now);
                if self.last_effect_position.is_some_and(|last| {
                    last.distance_sq(position) < self.next_effect_spacing.powi(2)
                }) {
                    return;
                }
                self.last_effect_position = Some(position);
                let mut rng = rand::rng();
                self.next_effect_spacing = match effect {
                    CursorEffect::Sparkles => rng.random_range(22.0..50.0),
                    CursorEffect::Bubbles => rng.random_range(28.0..62.0),
                    _ => 24.0,
                };
                let drift_x = rng.random_range(-18.0..=18.0);
                let (drift, duration, scale_from, scale_to, rotation) = match effect {
                    CursorEffect::Sparkles => {
                        (vec2(drift_x, -34.0), 0.9, 0.2, 1.55, (-20.0, 100.0))
                    }
                    CursorEffect::Bubbles => {
                        (vec2(drift_x * 1.4, -78.0), 1.35, 0.55, 1.45, (-8.0, 12.0))
                    }
                    _ => return,
                };
                self.particles.push(Particle {
                    effect,
                    created: now,
                    start: position,
                    drift,
                    duration,
                    scale_from,
                    scale_to,
                    rotation_from: rotation.0,
                    rotation_to: rotation.1,
                    color: COLORS[rng.random_range(0..COLORS.len())],
                });
                if self.particles.len() > MAX_PARTICLES {
                    let remove_count = self.particles.len() - MAX_PARTICLES;
                    self.particles.drain(0..remove_count);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Particle {
    pub effect: CursorEffect,
    pub created: Instant,
    pub start: Pos2,
    pub drift: Vec2,
    pub duration: f32,
    pub scale_from: f32,
    pub scale_to: f32,
    pub rotation_from: f32,
    pub rotation_to: f32,
    pub color: BabyColor,
}

#[derive(Debug, Default)]
pub struct RainbowTrail {
    points: Vec<Pos2>,
    target: Pos2,
    active: bool,
    fading_since: Option<Instant>,
    pub opacity: f32,
}

impl RainbowTrail {
    pub fn points(&self) -> &[Pos2] {
        &self.points
    }

    fn move_to(&mut self, point: Pos2) {
        self.target = point;
        if !self.active {
            self.points = vec![point; 32];
            self.active = true;
        }
        self.fading_since = None;
        self.opacity = 1.0;
    }

    fn stop(&mut self, now: Instant) {
        if self.active && self.fading_since.is_none() {
            self.fading_since = Some(now);
        }
    }

    fn advance(&mut self, now: Instant, frame_seconds: f32) {
        if !self.active {
            return;
        }
        let frame_scale = (frame_seconds * 60.0).clamp(0.25, 3.0);
        let head_blend = 1.0 - 0.45_f32.powf(frame_scale);
        let follower_blend = 1.0 - 0.58_f32.powf(frame_scale);
        self.points[0] = self.points[0].lerp(self.target, head_blend);
        for index in 1..self.points.len() {
            self.points[index] = self.points[index].lerp(self.points[index - 1], follower_blend);
        }
        if let Some(started) = self.fading_since {
            self.opacity = (1.0 - now.duration_since(started).as_secs_f32() / 0.28).max(0.0);
            if self.opacity <= 0.0 {
                self.active = false;
                self.points.clear();
                self.fading_since = None;
            }
        }
    }
}

pub fn pointer_tone(point: Pos2, size: Vec2) -> (f32, f32) {
    let x = (point.x / size.x.max(1.0)).clamp(0.0, 1.0);
    let y = (point.y / size.y.max(1.0)).clamp(0.0, 1.0);
    (880.0 * 0.125_f32.powf(y), x * 2.0 - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::responses::response_for;

    #[test]
    fn clear_after_is_a_hard_bound() {
        let mut game = Game::new([vec2(1920.0, 1080.0)]);
        let settings = Settings {
            clear_after: 5,
            ..Settings::default()
        };
        let now = Instant::now();
        for _ in 0..20 {
            game.add_response(response_for("A"), &settings, now);
        }
        assert_eq!(game.figures.len(), 5);
    }

    #[test]
    fn tone_matches_upstream_screen_mapping() {
        let (top_left, pan_left) = pointer_tone(pos2(0.0, 0.0), vec2(100.0, 100.0));
        let (bottom_right, pan_right) = pointer_tone(pos2(100.0, 100.0), vec2(100.0, 100.0));
        assert!((top_left - 880.0).abs() < 0.01);
        assert!((bottom_right - 110.0).abs() < 0.01);
        assert_eq!(pan_left, -1.0);
        assert_eq!(pan_right, 1.0);
    }
}
