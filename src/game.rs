use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::OnceLock,
    time::Instant,
};

use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};
use rand::Rng;

use crate::{
    responses::{KeyResponse, ResponseKind, ShapeKind},
    settings::{CursorEffect, PianoKey, PianoScale, Settings},
};

const LETTER_ADVANCE: f32 = 208.0;
const LETTER_PADDING: f32 = 24.0;
const GLYPH_SIZE: Vec2 = Vec2::new(220.0, 300.0);
const MAX_PARTICLES: usize = 72;
const MAX_TRAIL_MARKS: usize = 512;
const TRAIL_MARK_SPACING: f32 = 5.0;
const FADING_TRAIL_SECONDS: f32 = 1.35;
const NEON_POINT_COUNT: usize = 1_024;
const NEON_WINDOW_LENGTH: usize = 32;
const NEON_WINDOW_OFFSET: isize = (NEON_WINDOW_LENGTH / 2) as isize;
const NEON_REFERENCE_FPS: f32 = 165.0;
const NEON_MAX_STEPS_PER_FRAME: usize = 8;
const MAX_CONCURRENT_ITEM_ANIMATIONS: usize = 50;
const REMOVAL_FADE_SECONDS: f32 = 1.0;
const CURSOR_PULSE_SECONDS: f32 = 0.3;
const CURSOR_GROW_SCALE: f32 = 1.28;
const CURSOR_SHRINK_SCALE: f32 = 0.76;
const PIANO_RIPPLE_SECONDS: f32 = 0.8;
const MAX_PIANO_RIPPLES: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BabyColor {
    pub rgb: [u8; 3],
    pub name: &'static str,
    pub speech_name: &'static str,
}

// Chroma-Notes pitch colors occupy the first twelve entries in chromatic order
// from C through B, with KeySlam's custom pink as B. White, gray, and black are
// additional painting/item choices that intentionally stay at the end.
pub const COLORS: [BabyColor; 15] = [
    BabyColor {
        rgb: [255, 0, 0],
        name: "Red",
        speech_name: "Red",
    },
    BabyColor {
        rgb: [255, 69, 0],
        name: "Orange-red",
        speech_name: "Orange",
    },
    BabyColor {
        rgb: [255, 140, 0],
        name: "Orange",
        speech_name: "Orange",
    },
    BabyColor {
        rgb: [255, 191, 0],
        name: "Amber",
        speech_name: "Yellow",
    },
    BabyColor {
        rgb: [255, 255, 0],
        name: "Yellow",
        speech_name: "Yellow",
    },
    BabyColor {
        rgb: [154, 205, 50],
        name: "Lime green",
        speech_name: "Green",
    },
    BabyColor {
        rgb: [0, 170, 70],
        name: "Green",
        speech_name: "Green",
    },
    BabyColor {
        rgb: [0, 170, 170],
        name: "Teal",
        speech_name: "Blue",
    },
    BabyColor {
        rgb: [0, 0, 255],
        name: "Blue",
        speech_name: "Blue",
    },
    BabyColor {
        rgb: [75, 0, 180],
        name: "Indigo",
        speech_name: "Violet",
    },
    BabyColor {
        rgb: [148, 0, 211],
        name: "Violet",
        speech_name: "Violet",
    },
    BabyColor {
        rgb: [255, 20, 147],
        name: "Pink",
        speech_name: "Pink",
    },
    BabyColor {
        rgb: [255, 255, 255],
        name: "White",
        speech_name: "White",
    },
    BabyColor {
        rgb: [128, 128, 128],
        name: "Gray",
        speech_name: "Gray",
    },
    BabyColor {
        rgb: [0, 0, 0],
        name: "Black",
        speech_name: "Black",
    },
];

pub fn chroma_color_for_note(note: i32) -> BabyColor {
    COLORS[note.rem_euclid(12) as usize]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FigureKind {
    Glyph(char),
    Animal {
        image: Option<&'static str>,
        fallback_emoji: &'static str,
    },
    Shape(ShapeKind),
}

#[derive(Debug, Clone)]
pub struct Figure {
    pub id: u64,
    pub kind: FigureKind,
    pub color: BabyColor,
    pub spoken_text: String,
    pub created: Instant,
    pub fade_after: Option<f32>,
    pub animate_spawn: bool,
    pub placements: Vec<Placement>,
}

impl Figure {
    pub fn opacity(&self, now: Instant) -> f32 {
        self.fade_after.map_or(1.0, |visible_seconds| {
            let fade_elapsed = now.duration_since(self.created).as_secs_f32() - visible_seconds;
            if fade_elapsed <= 0.0 {
                1.0
            } else {
                (1.0 - fade_elapsed / REMOVAL_FADE_SECONDS).clamp(0.0, 1.0)
            }
        })
    }

    fn fade_has_started(&self, now: Instant) -> bool {
        self.fade_after.is_some_and(|visible_seconds| {
            now.duration_since(self.created).as_secs_f32() >= visible_seconds
        })
    }

    fn start_removal_fade(&mut self, now: Instant) {
        let visible_seconds = now.duration_since(self.created).as_secs_f32();
        self.fade_after = Some(
            self.fade_after
                .map_or(visible_seconds, |scheduled| scheduled.min(visible_seconds)),
        );
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

    fn is_animating(&self, now: Instant) -> bool {
        let age = now.duration_since(self.created).as_secs_f32();
        let spawn_is_active = self.animate_spawn && age < 1.0;
        let fade_is_active = self.fade_after.is_some_and(|visible_seconds| {
            age >= visible_seconds && age < visible_seconds + REMOVAL_FADE_SECONDS
        });
        spawn_is_active
            || fade_is_active
            || self.placements.iter().any(|placement| {
                placement.interaction.as_ref().is_some_and(|interaction| {
                    now.duration_since(interaction.started).as_secs_f32()
                        < interaction.kind.duration()
                })
            })
    }

    fn finish_active_animations(&mut self, now: Instant) {
        let age = now.duration_since(self.created).as_secs_f32();
        if self.animate_spawn && age < 1.0 {
            self.animate_spawn = false;
        }
        if self.fade_after.is_some_and(|visible_seconds| {
            age >= visible_seconds && age < visible_seconds + REMOVAL_FADE_SECONDS
        }) {
            self.fade_after = Some(age - REMOVAL_FADE_SECONDS);
        }
        for placement in &mut self.placements {
            if placement.interaction.as_ref().is_some_and(|interaction| {
                now.duration_since(interaction.started).as_secs_f32() < interaction.kind.duration()
            }) {
                placement.interaction = None;
            }
        }
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
    next_color_index: usize,
    image_cycles: HashMap<String, usize>,
    active_word: Option<ActiveWord>,
}

#[derive(Debug, Clone, Copy)]
struct ActiveWord {
    color: BabyColor,
    last_letter: Instant,
}

impl Game {
    pub fn new(display_sizes: impl IntoIterator<Item = Vec2>) -> Self {
        Self {
            figures: VecDeque::new(),
            displays: display_sizes.into_iter().map(DisplayState::new).collect(),
            next_id: 1,
            next_color_index: 0,
            image_cycles: HashMap::new(),
            active_word: None,
        }
    }

    pub fn has_active_item_animation(&self, now: Instant) -> bool {
        self.figures.iter().any(|figure| figure.is_animating(now))
    }

    pub fn limit_active_item_animations(&mut self, now: Instant) {
        let mut animations_to_finish = self
            .figures
            .iter()
            .filter(|figure| figure.is_animating(now))
            .count()
            .saturating_sub(MAX_CONCURRENT_ITEM_ANIMATIONS);
        for figure in &mut self.figures {
            if animations_to_finish == 0 {
                break;
            }
            if figure.is_animating(now) {
                figure.finish_active_animations(now);
                animations_to_finish -= 1;
            }
        }
    }

    pub fn add_response(
        &mut self,
        response: KeyResponse,
        settings: &Settings,
        now: Instant,
    ) -> String {
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
            ResponseKind::Emoji(emoji) => {
                let image =
                    crate::images::next_animal_image(response.spoken_text, &mut self.image_cycles);
                (
                    FigureKind::Animal {
                        image,
                        fallback_emoji: emoji,
                    },
                    response.spoken_text.to_owned(),
                    false,
                )
            }
            ResponseKind::Shape(shape) => (
                FigureKind::Shape(shape),
                response.spoken_text.to_owned(),
                false,
            ),
        };
        let (color, continues_word) = self.color_for_response(&kind, grouped_letter, settings, now);

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let size = size_for(&kind);
        let placements = if continues_word {
            self.displays
                .iter()
                .map(|_| Placement {
                    top_left: Pos2::ZERO,
                    size,
                    interaction: None,
                })
                .collect()
        } else {
            let mut rng = rand::rng();
            self.displays
                .iter()
                .enumerate()
                .map(|(display_index, display)| {
                    let mut occupied = self
                        .figures
                        .iter()
                        .filter_map(|figure| {
                            (figure.opacity(now) > 0.05)
                                .then(|| figure.placements.get(display_index).map(Placement::rect))
                                .flatten()
                        })
                        .collect::<Vec<_>>();
                    if display_index == 0 {
                        occupied.push(Rect::from_min_max(pos2(8.0, 8.0), pos2(330.0, 42.0)));
                    }
                    if settings.cursor_effect == CursorEffect::Coloring {
                        occupied.push(Rect::from_min_max(
                            pos2(0.0, (display.size.y - 72.0).max(0.0)),
                            pos2(display.size.x, display.size.y),
                        ));
                        occupied.push(Rect::from_min_max(
                            pos2(0.0, 52.0),
                            pos2(70.0, (display.size.y - 72.0).max(52.0)),
                        ));
                    }
                    Placement {
                        top_left: best_available_position(&mut rng, display.size, size, occupied),
                        size,
                        interaction: None,
                    }
                })
                .collect()
        };
        self.figures.push_back(Figure {
            id,
            kind,
            color,
            spoken_text: default_speech.clone(),
            created: now,
            fade_after: (settings.fade_away && settings.fade_after_seconds > 0.0)
                .then_some(settings.fade_after_seconds),
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
        self.fade_items_over_limit(settings.clear_after, now);
        default_speech
    }

    fn fade_items_over_limit(&mut self, item_limit: usize, now: Instant) {
        let mut items_kept = 0;
        for figure in self.figures.iter_mut().rev() {
            if figure.fade_has_started(now) {
                break;
            }
            if items_kept < item_limit {
                items_kept += 1;
            } else {
                figure.start_removal_fade(now);
            }
        }
    }

    pub fn clear(&mut self) {
        self.figures.clear();
        self.active_word = None;
        for display in &mut self.displays {
            display.letter_run.clear();
            display.last_letter = None;
        }
    }

    pub fn remove_expired(&mut self, now: Instant) {
        let expired = self
            .figures
            .iter()
            .filter(|figure| figure.fade_after.is_some() && figure.opacity(now) <= 0.0)
            .map(|figure| figure.id)
            .collect::<HashSet<_>>();
        if expired.is_empty() {
            return;
        }
        self.figures.retain(|figure| !expired.contains(&figure.id));
        for display in &mut self.displays {
            display
                .letter_run
                .retain(|figure_id| !expired.contains(figure_id));
            if display.letter_run.is_empty() {
                display.last_letter = None;
            }
        }
    }

    fn color_for_response(
        &mut self,
        kind: &FigureKind,
        grouped_letter: bool,
        settings: &Settings,
        now: Instant,
    ) -> (BabyColor, bool) {
        if grouped_letter && settings.group_letters {
            let current_word = self.active_word.filter(|word| {
                now.duration_since(word.last_letter).as_secs_f32()
                    <= settings.letter_grouping_timeout_seconds
            });
            let (color, continues_word) = match current_word {
                Some(word) => (word.color, true),
                None => (self.take_next_color(), false),
            };
            self.active_word = Some(ActiveWord {
                color,
                last_letter: now,
            });
            return (color, continues_word);
        }

        self.active_word = None;
        let color = if matches!(kind, FigureKind::Glyph(_) | FigureKind::Shape(_)) {
            self.take_next_color()
        } else {
            COLORS[self.next_color_index]
        };
        (color, false)
    }

    fn take_next_color(&mut self) -> BabyColor {
        let color = COLORS[self.next_color_index];
        self.next_color_index = (self.next_color_index + 1) % COLORS.len();
        color
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
        let ids = display.letter_run.iter().copied().collect::<HashSet<_>>();
        let display_size = display.size;
        let anchor = display.letter_anchor_x;
        let requested_top = display.letter_top;
        if ids.is_empty() {
            return;
        }
        let total_width = GLYPH_SIZE.x + LETTER_ADVANCE * (ids.len() - 1) as f32;
        let max_left = (display_size.x - LETTER_PADDING - total_width).max(LETTER_PADDING);
        let mut left = if total_width >= (display_size.x - LETTER_PADDING * 2.0).max(0.0) {
            display_size.x - LETTER_PADDING - total_width
        } else {
            (anchor - total_width / 2.0).clamp(LETTER_PADDING, max_left)
        };
        let top = requested_top.clamp(
            LETTER_PADDING,
            (display_size.y - LETTER_PADDING - GLYPH_SIZE.y).max(LETTER_PADDING),
        );
        for figure in &mut self.figures {
            if ids.contains(&figure.id)
                && let Some(placement) = figure.placements.get_mut(display_index)
            {
                placement.top_left = pos2(left, top);
                left += LETTER_ADVANCE;
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
}

fn best_available_position(
    rng: &mut impl Rng,
    display: Vec2,
    figure: Vec2,
    occupied: impl IntoIterator<Item = Rect>,
) -> Pos2 {
    const RANDOM_ATTEMPTS: usize = 48;
    const EDGE_OBSTACLES: usize = 4;

    let occupied = occupied.into_iter().collect::<Vec<_>>();
    let max_x = (display.x - figure.x).max(0.0);
    let max_y = (display.y - figure.y).max(0.0);
    let mut best = Pos2::ZERO;
    let mut best_overlap = f32::INFINITY;

    for _ in 0..RANDOM_ATTEMPTS {
        let candidate = pos2(rng.random_range(0.0..=max_x), rng.random_range(0.0..=max_y));
        let candidate_rect = Rect::from_min_size(candidate, figure);
        let overlap = occupied
            .iter()
            .map(|rect| overlap_area(candidate_rect, *rect))
            .sum::<f32>();
        if overlap == 0.0 {
            return candidate;
        }
        if overlap < best_overlap {
            best = candidate;
            best_overlap = overlap;
        }
    }

    // A bounded set of edge-aligned candidates finds narrow gaps without the
    // quadratic candidate explosion caused by combining every figure edge.
    let mut x_candidates = vec![0.0, max_x];
    let mut y_candidates = vec![0.0, max_y];
    for rect in occupied.iter().rev().take(EDGE_OBSTACLES) {
        x_candidates.extend([rect.min.x - figure.x, rect.max.x]);
        y_candidates.extend([rect.min.y - figure.y, rect.max.y]);
    }
    for x in x_candidates {
        for &y in &y_candidates {
            let candidate = pos2(x.clamp(0.0, max_x), y.clamp(0.0, max_y));
            let candidate_rect = Rect::from_min_size(candidate, figure);
            let overlap = occupied
                .iter()
                .map(|rect| overlap_area(candidate_rect, *rect))
                .sum::<f32>();
            if overlap == 0.0 {
                return candidate;
            }
            if overlap < best_overlap {
                best = candidate;
                best_overlap = overlap;
            }
        }
    }
    best
}

fn overlap_area(left: Rect, right: Rect) -> f32 {
    let width = (left.max.x.min(right.max.x) - left.min.x.max(right.min.x)).max(0.0);
    let height = (left.max.y.min(right.max.y) - left.min.y.max(right.min.y)).max(0.0);
    width * height
}

pub fn size_for(kind: &FigureKind) -> Vec2 {
    match kind {
        FigureKind::Glyph(_) => GLYPH_SIZE,
        FigureKind::Animal { .. } => Vec2::splat(340.0),
        FigureKind::Shape(shape) => match shape {
            ShapeKind::Oval => vec2(190.0, 250.0),
            ShapeKind::Rectangle => vec2(300.0, 207.0),
            ShapeKind::Triangle => vec2(248.0, 180.0),
            ShapeKind::Square => Vec2::splat(207.0),
            ShapeKind::Pentagon | ShapeKind::Septagon | ShapeKind::Octagon => Vec2::splat(260.0),
            ShapeKind::Hexagon => vec2(236.0, 205.0),
            ShapeKind::Trapezoid => vec2(310.0, 165.0),
            ShapeKind::Circle => Vec2::splat(212.0),
            ShapeKind::Star => vec2(253.0, 243.0),
        },
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

#[derive(Debug, Clone, Copy)]
struct CursorPulse {
    started: Instant,
    initial_scale: f32,
}

#[derive(Debug, Default)]
pub struct PointerState {
    pub position: Option<Pos2>,
    pub primary_down: bool,
    pub trail: RainbowTrail,
    pub neon_worm: NeonWormTrail,
    pub trail_marks: Vec<TrailMark>,
    pub particles: Vec<Particle>,
    pub piano_ripples: Vec<PianoRipple>,
    last_effect_position: Option<Pos2>,
    next_effect_spacing: f32,
    stroke_sequence: u64,
    cursor_pulse: Option<CursorPulse>,
}

#[derive(Debug, Clone)]
pub struct PianoRipple {
    pub position: Pos2,
    pub created: Instant,
    pub note_label: Option<String>,
    pub color: BabyColor,
}

impl PianoRipple {
    pub fn progress(&self, now: Instant) -> f32 {
        (now.duration_since(self.created).as_secs_f32() / PIANO_RIPPLE_SECONDS).clamp(0.0, 1.0)
    }
}

impl PointerState {
    pub fn press(&mut self, position: Pos2, effect: CursorEffect, now: Instant) {
        self.primary_down = true;
        self.position = Some(position);
        self.last_effect_position = None;
        self.stroke_sequence = self.stroke_sequence.wrapping_add(1);
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
        self.neon_worm.release();
    }

    pub fn pulse_cursor(&mut self, grow: bool, now: Instant) {
        self.cursor_pulse = Some(CursorPulse {
            started: now,
            initial_scale: if grow {
                CURSOR_GROW_SCALE
            } else {
                CURSOR_SHRINK_SCALE
            },
        });
    }

    pub fn piano_ripple(
        &mut self,
        position: Pos2,
        note_label: Option<String>,
        color: BabyColor,
        now: Instant,
    ) {
        if self.piano_ripples.len() == MAX_PIANO_RIPPLES {
            self.piano_ripples.remove(0);
        }
        self.piano_ripples.push(PianoRipple {
            position,
            created: now,
            note_label,
            color,
        });
    }

    pub fn cursor_scale(&self, now: Instant) -> f32 {
        let Some(pulse) = self.cursor_pulse else {
            return 1.0;
        };
        let progress = (now.duration_since(pulse.started).as_secs_f32() / CURSOR_PULSE_SECONDS)
            .clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        pulse.initial_scale + (1.0 - pulse.initial_scale) * eased
    }

    pub fn update(&mut self, now: Instant, frame_seconds: f32, bounds: Vec2, effect: CursorEffect) {
        self.trail.advance(now, frame_seconds);
        if effect == CursorEffect::NeonWorm {
            let initial = self
                .position
                .unwrap_or_else(|| pos2(bounds.x * 0.5, bounds.y * 0.5));
            self.neon_worm.ensure(initial);
        } else {
            self.neon_worm.stop(now);
        }
        self.neon_worm.advance(now, frame_seconds, bounds);
        self.particles.retain(|particle| {
            now.duration_since(particle.created).as_secs_f32() < particle.duration
        });
        self.trail_marks
            .retain(|mark| now.duration_since(mark.created).as_secs_f32() < mark.duration);
        self.piano_ripples
            .retain(|ripple| ripple.progress(now) < 1.0);
        if self.cursor_pulse.is_some_and(|pulse| {
            now.duration_since(pulse.started).as_secs_f32() >= CURSOR_PULSE_SECONDS
        }) {
            self.cursor_pulse = None;
        }
    }

    fn emit(&mut self, position: Pos2, effect: CursorEffect, now: Instant) {
        match effect {
            CursorEffect::None | CursorEffect::Coloring | CursorEffect::PianoRoll => {
                self.last_effect_position = None;
                self.trail.stop(now);
                self.neon_worm.stop(now);
            }
            CursorEffect::Rainbow => {
                self.last_effect_position = None;
                self.neon_worm.stop(now);
                self.trail.move_to(position);
            }
            CursorEffect::FadingTrail => {
                self.trail.stop(now);
                self.neon_worm.stop(now);
                self.push_trail_mark(position, effect, now, FADING_TRAIL_SECONDS);
            }
            CursorEffect::NeonWorm => {
                self.last_effect_position = None;
                self.trail.stop(now);
                self.neon_worm.move_to(position);
            }
            CursorEffect::Sparkles | CursorEffect::Bubbles => {
                self.trail.stop(now);
                self.neon_worm.stop(now);
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

    fn push_trail_mark(
        &mut self,
        position: Pos2,
        effect: CursorEffect,
        now: Instant,
        duration: f32,
    ) {
        if self.trail_marks.last().is_some_and(|last| {
            last.effect == effect
                && last.stroke_id == self.stroke_sequence
                && last.position.distance_sq(position) < TRAIL_MARK_SPACING.powi(2)
        }) {
            return;
        }
        self.trail_marks.push(TrailMark {
            effect,
            created: now,
            duration,
            position,
            stroke_id: self.stroke_sequence,
        });
        if self.trail_marks.len() > MAX_TRAIL_MARKS {
            let remove_count = self.trail_marks.len() - MAX_TRAIL_MARKS;
            self.trail_marks.drain(0..remove_count);
        }
    }
}

#[derive(Debug, Default)]
pub struct NeonWormTrail {
    points: Vec<Pos2>,
    next_points: Vec<Pos2>,
    target: Pos2,
    input_active: bool,
    fading_since: Option<Instant>,
    opacity: f32,
    step_accumulator: f32,
    simulation_time: f32,
}

impl NeonWormTrail {
    pub fn points(&self) -> &[Pos2] {
        &self.points
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    fn ensure(&mut self, initial: Pos2) {
        if self.points.is_empty() {
            self.points = vec![initial; NEON_POINT_COUNT];
            self.next_points = vec![initial; NEON_POINT_COUNT];
            self.target = initial;
            self.step_accumulator = 0.0;
            self.simulation_time = 0.0;
        }
        self.fading_since = None;
        self.opacity = 1.0;
    }

    fn move_to(&mut self, position: Pos2) {
        self.ensure(position);
        self.target = position;
        self.input_active = true;
    }

    fn release(&mut self) {
        self.input_active = false;
    }

    fn stop(&mut self, now: Instant) {
        self.input_active = false;
        if !self.points.is_empty() && self.fading_since.is_none() {
            self.fading_since = Some(now);
        }
    }

    fn advance(&mut self, now: Instant, frame_seconds: f32, bounds: Vec2) {
        if self.points.is_empty() {
            return;
        }
        if let Some(started) = self.fading_since {
            self.opacity = (1.0 - now.duration_since(started).as_secs_f32() / 0.28).max(0.0);
            if self.opacity <= 0.0 {
                self.points.clear();
                self.next_points.clear();
                self.fading_since = None;
                return;
            }
        }

        self.step_accumulator = (self.step_accumulator
            + frame_seconds.max(0.0) * NEON_REFERENCE_FPS)
            .min(NEON_MAX_STEPS_PER_FRAME as f32);
        let step_count = self.step_accumulator.floor() as usize;
        self.step_accumulator -= step_count as f32;
        for _ in 0..step_count {
            self.simulation_time += 1.0 / NEON_REFERENCE_FPS;
            self.advance_one_step(bounds);
        }
    }

    fn advance_one_step(&mut self, bounds: Vec2) {
        let previous_head = self.points[0];
        self.next_points[0] = if self.input_active {
            previous_head + (self.target - previous_head) * 0.1
        } else {
            neon_random_walk(previous_head, bounds, self.simulation_time)
        };

        let weights = neon_filter_weights();
        let gravity = pos2(
            (0.6 * (self.simulation_time * 0.27).cos() + 0.5) * bounds.x,
            (0.6 * (self.simulation_time * 0.27).sin() + 0.5) * bounds.y,
        );
        for index in 1..NEON_POINT_COUNT {
            let mut filtered = Vec2::ZERO;
            for (tap, weight) in weights.iter().copied().enumerate() {
                let source = (index as isize + tap as isize - NEON_WINDOW_OFFSET)
                    .clamp(0, (NEON_POINT_COUNT - 1) as isize)
                    as usize;
                filtered += self.points[source].to_vec2() * weight;
            }
            let point = pos2(filtered.x, filtered.y);
            self.next_points[index] = point + (gravity - point) * 0.0003;
        }
        std::mem::swap(&mut self.points, &mut self.next_points);
    }
}

fn neon_filter_weights() -> &'static [f32; NEON_WINDOW_LENGTH + 1] {
    static WEIGHTS: OnceLock<[f32; NEON_WINDOW_LENGTH + 1]> = OnceLock::new();
    WEIGHTS.get_or_init(|| {
        let raw = std::array::from_fn(|index| neon_window(index as f32));
        let sum = raw.iter().sum::<f32>();
        raw.map(|weight| weight / sum)
    })
}

fn neon_window(mut value: f32) -> f32 {
    value -= NEON_WINDOW_OFFSET as f32;
    shader_sinc((value + 5.0) * 0.31) + shader_sinc_shelf(value + 5.55, 0.02, 0.04) * 0.03
        - shader_sinc_shelf(value, 0.08, 0.20) * 0.05
}

fn shader_sinc(mut value: f32) -> f32 {
    if value == 0.0 {
        return 1.0;
    }
    value *= std::f32::consts::PI;
    value.sin() / value
}

fn shader_sinc_shelf(value: f32, low: f32, high: f32) -> f32 {
    shader_sinc(value * high) - shader_sinc(value * low)
}

fn neon_random_walk(point: Pos2, bounds: Vec2, time: f32) -> Pos2 {
    let safe_bounds = vec2(bounds.x.max(1.0), bounds.y.max(1.0));
    let mut normalized = vec2(point.x / safe_bounds.x, point.y / safe_bounds.y);
    normalized +=
        rotate_shader_noise(shader_nrand2(vec2(time, 0.0)), time.cos(), time.sin()) * 0.008;
    normalized += rotate_shader_noise(
        shader_nrand2(vec2(time, 1.0)),
        (time * 3.5).cos(),
        (time * 3.1).sin(),
    ) * 0.007;
    normalized += (Vec2::splat(0.5) - normalized) * 0.02;
    pos2(normalized.x * safe_bounds.x, normalized.y * safe_bounds.y)
}

fn rotate_shader_noise(angle_source: Vec2, cos: f32, sin: f32) -> Vec2 {
    vec2(
        angle_source.x * cos + angle_source.y * sin,
        -angle_source.x * sin + angle_source.y * cos,
    )
}

fn shader_nrand2(value: Vec2) -> Vec2 {
    vec2(
        shader_nrand(vec2(value.x * -3.2145, value.y * 1.2345)),
        shader_nrand(vec2(value.x * -5.4321, value.y * 3.4521)),
    )
}

fn shader_nrand(value: Vec2) -> f32 {
    let random = (value.x * 12.9898 + value.y * 78.233).sin() * 43_758.547;
    random - random.floor()
}

#[derive(Debug, Clone, Copy)]
pub struct TrailMark {
    pub effect: CursorEffect,
    pub created: Instant,
    pub duration: f32,
    pub position: Pos2,
    pub stroke_id: u64,
}

impl TrailMark {
    pub fn progress(self, now: Instant) -> f32 {
        (now.duration_since(self.created).as_secs_f32() / self.duration).clamp(0.0, 1.0)
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

pub fn piano_tone(point: Pos2, size: Vec2, scale: PianoScale, key: PianoKey) -> f32 {
    let note = piano_note(point, size, scale, key);
    440.0 * 2.0_f32.powf((note - 69) as f32 / 12.0)
}

pub fn piano_note(point: Pos2, size: Vec2, scale: PianoScale, key: PianoKey) -> i32 {
    let (frequency, _) = pointer_tone(point, size);
    let target_note = 69.0 + 12.0 * (frequency / 440.0).log2();
    if scale == PianoScale::Chromatic {
        return target_note.round() as i32;
    }

    let intervals: &[i32] = match scale {
        PianoScale::Chromatic => unreachable!(),
        PianoScale::Major => &[0, 2, 4, 5, 7, 9, 11],
        PianoScale::Minor => &[0, 2, 3, 5, 7, 8, 10],
    };
    ((target_note.floor() as i32 - 6)..=(target_note.ceil() as i32 + 6))
        .filter(|note| intervals.contains(&((*note - key.semitone()).rem_euclid(12))))
        .min_by(|left, right| {
            ((*left as f32 - target_note).abs()).total_cmp(&((*right as f32 - target_note).abs()))
        })
        .unwrap_or(target_note.round() as i32)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::responses::response_for;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn item_limit_fades_oldest_items_before_removing_them() {
        let mut game = Game::new([vec2(1920.0, 1080.0)]);
        let settings = Settings {
            clear_after: 5,
            fade_after_seconds: 120.0,
            ..Settings::default()
        };
        let now = Instant::now();
        for _ in 0..20 {
            game.add_response(response_for("A"), &settings, now);
        }

        assert_eq!(
            game.figures
                .iter()
                .filter(|figure| !figure.fade_has_started(now))
                .count(),
            5
        );
        assert_eq!(
            game.figures
                .iter()
                .filter(|figure| figure.fade_has_started(now))
                .count(),
            15
        );
        assert!((game.figures[0].opacity(now + Duration::from_millis(500)) - 0.5).abs() < 0.001);

        game.remove_expired(now + Duration::from_secs(1));
        assert_eq!(game.figures.len(), 5);
    }

    #[test]
    fn colored_items_share_the_requested_color_cycle() {
        let mut game = Game::new([vec2(1920.0, 1080.0)]);
        let settings = Settings::default();
        let now = Instant::now();
        for key in [
            "A", "1", "NumPad0", "B", "2", "NumPad1", "C", "3", "NumPad2", "D", "4", "NumPad3",
            "E", "5", "NumPad4",
        ] {
            game.add_response(response_for(key), &settings, now);
        }

        let names = game
            .figures
            .iter()
            .map(|figure| figure.color.name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "Red",
                "Orange-red",
                "Orange",
                "Amber",
                "Yellow",
                "Lime green",
                "Green",
                "Teal",
                "Blue",
                "Indigo",
                "Violet",
                "Pink",
                "White",
                "Gray",
                "Black",
            ]
        );

        game.add_response(response_for("F"), &settings, now);
        assert_eq!(
            game.figures.back().map(|figure| figure.color.name),
            Some("Red")
        );
    }

    #[test]
    fn letters_inside_the_word_timeout_share_one_color() {
        let mut game = Game::new([vec2(1920.0, 1080.0)]);
        let settings = Settings {
            letter_grouping_timeout_seconds: 1.0,
            ..Settings::default()
        };
        let started = Instant::now();
        game.add_response(response_for("A"), &settings, started);
        game.add_response(
            response_for("B"),
            &settings,
            started + Duration::from_millis(750),
        );
        game.add_response(
            response_for("C"),
            &settings,
            started + Duration::from_millis(1500),
        );

        let colors = game
            .figures
            .iter()
            .map(|figure| figure.color.name)
            .collect::<Vec<_>>();
        assert_eq!(colors, ["Red", "Red", "Red"]);

        game.add_response(
            response_for("D"),
            &settings,
            started + Duration::from_millis(2501),
        );
        assert_eq!(
            game.figures.back().map(|figure| figure.color.name),
            Some("Orange-red")
        );
    }

    #[test]
    fn disabling_word_grouping_restores_per_letter_colors() {
        let mut game = Game::new([vec2(1920.0, 1080.0)]);
        let settings = Settings {
            group_letters: false,
            ..Settings::default()
        };
        let now = Instant::now();
        game.add_response(response_for("A"), &settings, now);
        game.add_response(response_for("B"), &settings, now);

        let colors = game
            .figures
            .iter()
            .map(|figure| figure.color.name)
            .collect::<Vec<_>>();
        assert_eq!(colors, ["Red", "Orange-red"]);
    }

    #[test]
    fn placement_uses_free_space_when_it_is_available() {
        let occupied = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 300.0));
        let mut rng = StdRng::seed_from_u64(7);
        let position = best_available_position(
            &mut rng,
            vec2(1000.0, 700.0),
            vec2(220.0, 300.0),
            [occupied],
        );
        let placed = Rect::from_min_size(position, vec2(220.0, 300.0));
        assert_eq!(overlap_area(placed, occupied), 0.0);
    }

    #[test]
    fn items_stay_opaque_then_fade_over_one_second() {
        let mut game = Game::new([vec2(1920.0, 1080.0)]);
        let settings = Settings {
            fade_away: true,
            fade_after_seconds: 4.0,
            ..Settings::default()
        };
        let created = Instant::now();
        game.add_response(response_for("A"), &settings, created);
        let Some(figure) = game.figures.back() else {
            panic!("a response should create a figure");
        };

        assert_eq!(figure.opacity(created + Duration::from_millis(3999)), 1.0);
        assert!((figure.opacity(created + Duration::from_millis(4500)) - 0.5).abs() < 0.001);
        assert_eq!(figure.opacity(created + Duration::from_secs(5)), 0.0);
        game.remove_expired(created + Duration::from_secs(5));
        assert!(game.figures.is_empty());
    }

    #[test]
    fn zero_visible_seconds_disables_time_based_removal() {
        let mut game = Game::new([vec2(1920.0, 1080.0)]);
        let settings = Settings {
            fade_away: true,
            fade_after_seconds: 0.0,
            ..Settings::default()
        };
        let created = Instant::now();
        game.add_response(response_for("A"), &settings, created);

        assert_eq!(game.figures[0].fade_after, None);
        assert_eq!(
            game.figures[0].opacity(created + Duration::from_secs(24 * 60 * 60)),
            1.0
        );
        game.remove_expired(created + Duration::from_secs(24 * 60 * 60));
        assert_eq!(game.figures.len(), 1);
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

    #[test]
    fn cursor_wheel_pulses_return_to_normal_size() {
        let started = Instant::now();
        let mut pointer = PointerState::default();

        pointer.pulse_cursor(true, started);
        assert_eq!(pointer.cursor_scale(started), CURSOR_GROW_SCALE);
        assert!(pointer.cursor_scale(started + Duration::from_millis(150)) < CURSOR_GROW_SCALE);
        pointer.update(
            started + Duration::from_millis(301),
            Duration::from_millis(16).as_secs_f32(),
            vec2(100.0, 100.0),
            CursorEffect::None,
        );
        assert_eq!(
            pointer.cursor_scale(started + Duration::from_millis(301)),
            1.0
        );

        pointer.pulse_cursor(false, started);
        assert_eq!(pointer.cursor_scale(started), CURSOR_SHRINK_SCALE);
        assert!(pointer.cursor_scale(started + Duration::from_millis(150)) > CURSOR_SHRINK_SCALE);
    }

    #[test]
    fn shadertoy_trails_use_bounded_effect_specific_state() {
        let started = Instant::now();
        let mut pointer = PointerState::default();

        pointer.press(pos2(20.0, 20.0), CursorEffect::FadingTrail, started);
        pointer.move_to(
            pos2(40.0, 30.0),
            CursorEffect::FadingTrail,
            started + Duration::from_millis(16),
        );
        assert_eq!(pointer.trail_marks.len(), 2);
        assert!(
            pointer
                .trail_marks
                .iter()
                .all(|mark| mark.effect == CursorEffect::FadingTrail)
        );
        for index in 2..=(MAX_TRAIL_MARKS + 20) {
            pointer.move_to(
                pos2(index as f32 * 6.0, 30.0),
                CursorEffect::FadingTrail,
                started + Duration::from_millis(index as u64),
            );
        }
        assert_eq!(pointer.trail_marks.len(), MAX_TRAIL_MARKS);

        pointer.release(started + Duration::from_millis(300));
        pointer.update(
            started + Duration::from_secs(2),
            Duration::from_millis(16).as_secs_f32(),
            vec2(1_920.0, 1_080.0),
            CursorEffect::FadingTrail,
        );
        assert!(pointer.trail_marks.is_empty());

        pointer.press(
            pos2(60.0, 50.0),
            CursorEffect::NeonWorm,
            started + Duration::from_secs(2),
        );
        assert_eq!(pointer.neon_worm.points().len(), NEON_POINT_COUNT);
        let previous_head = pointer.neon_worm.points()[0];
        pointer.release(started + Duration::from_millis(2_001));
        pointer.update(
            started + Duration::from_millis(2_034),
            0.033,
            vec2(1_920.0, 1_080.0),
            CursorEffect::NeonWorm,
        );
        assert_ne!(pointer.neon_worm.points()[0], previous_head);

        let weights = neon_filter_weights();
        assert_eq!(weights.len(), NEON_WINDOW_LENGTH + 1);
        assert!((weights.iter().sum::<f32>() - 1.0).abs() < 0.0001);
        assert!(weights.iter().any(|weight| *weight < 0.0));
    }

    #[test]
    fn shapes_keep_their_original_dimensions() {
        for (shape, expected) in [
            (ShapeKind::Oval, vec2(190.0, 250.0)),
            (ShapeKind::Rectangle, vec2(300.0, 207.0)),
            (ShapeKind::Triangle, vec2(248.0, 180.0)),
            (ShapeKind::Square, Vec2::splat(207.0)),
            (ShapeKind::Pentagon, Vec2::splat(260.0)),
            (ShapeKind::Hexagon, vec2(236.0, 205.0)),
            (ShapeKind::Septagon, Vec2::splat(260.0)),
            (ShapeKind::Octagon, Vec2::splat(260.0)),
            (ShapeKind::Trapezoid, vec2(310.0, 165.0)),
            (ShapeKind::Circle, Vec2::splat(212.0)),
            (ShapeKind::Star, vec2(253.0, 243.0)),
        ] {
            assert_eq!(size_for(&FigureKind::Shape(shape)), expected);
        }
    }
}
