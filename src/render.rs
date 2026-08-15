use std::{collections::HashMap, sync::OnceLock, thread, time::Instant};

use crossbeam_channel::{Receiver, bounded};
use eframe::egui::{
    Align2, Color32, ColorImage, Context, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke,
    TextureHandle, TextureOptions, Vec2,
    emath::TSTransform,
    epaint::{CubicBezierShape, TextShape, Vertex},
    pos2, vec2,
};
use include_dir::{Dir, include_dir};

use crate::{
    game::{BabyColor, Figure, FigureKind, Particle, PointerState, TrailMark},
    responses::ShapeKind,
    settings::{CursorEffect, CursorStyle},
};

static EMOJI: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/emoji");
const GLYPH_PREWARM_TEXT: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const GLYPH_FONT_SIZE: f32 = 264.0;

pub fn prewarm_glyphs(ctx: &Context) {
    ctx.fonts_mut(|fonts| {
        let _ = fonts.layout_no_wrap(
            GLYPH_PREWARM_TEXT.to_owned(),
            FontId::proportional(GLYPH_FONT_SIZE),
            Color32::PLACEHOLDER,
        );
    });
}

pub struct TextureCache {
    emoji: HashMap<&'static str, TextureHandle>,
    decoded_emoji: HashMap<String, eframe::egui::ColorImage>,
    decoded_receiver: Receiver<(String, eframe::egui::ColorImage)>,
    hand_gradient: TextureHandle,
}

impl TextureCache {
    pub fn new(ctx: &Context) -> Self {
        let (sender, decoded_receiver) = bounded(EMOJI.files().count());
        let repaint_context = ctx.clone();
        let _ = thread::Builder::new()
            .name("emoji-preloader".to_owned())
            .spawn(move || {
                for file in EMOJI.files() {
                    let Some(file_name) = file
                        .path()
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(str::to_owned)
                    else {
                        continue;
                    };
                    let Ok(image) = image::load_from_memory(file.contents()) else {
                        continue;
                    };
                    let image = image.into_rgba8();
                    let size = [image.width() as usize, image.height() as usize];
                    let color_image =
                        eframe::egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                    if sender.send((file_name, color_image)).is_err() {
                        break;
                    }
                    repaint_context.request_repaint();
                }
            });
        Self {
            emoji: HashMap::new(),
            decoded_emoji: HashMap::new(),
            decoded_receiver,
            hand_gradient: ctx.load_texture(
                "hand-cursor-gradient",
                hand_gradient_image(),
                TextureOptions::LINEAR,
            ),
        }
    }

    fn emoji(&mut self, ctx: &Context, value: &'static str) -> Option<&TextureHandle> {
        while let Ok((file_name, image)) = self.decoded_receiver.try_recv() {
            self.decoded_emoji.insert(file_name, image);
        }
        if !self.emoji.contains_key(value) {
            let file_name = format!(
                "{}.png",
                value
                    .chars()
                    .map(|character| format!("{:x}", character as u32))
                    .collect::<Vec<_>>()
                    .join("-")
            );
            let color_image = self.decoded_emoji.remove(&file_name)?;
            let texture = ctx.load_texture(
                format!("emoji-{file_name}"),
                color_image,
                TextureOptions::LINEAR,
            );
            self.emoji.insert(value, texture);
        }
        self.emoji.get(value)
    }
}

pub fn draw_figure(
    painter: &Painter,
    ctx: &Context,
    cache: &mut TextureCache,
    figure: &Figure,
    display_index: usize,
    now: Instant,
    faces: bool,
) {
    let Some(placement) = figure.placements.get(display_index) else {
        return;
    };
    let opacity = figure.opacity(now);
    if opacity <= 0.0 {
        return;
    }
    let (spawn_scale, spawn_rotation) = figure.spawn_transform(now);
    let (interaction_scale, interaction_rotation) = placement.interaction_transform(now);
    let scale = interaction_scale * spawn_scale;
    let source_rect = placement.rect();
    let center = source_rect.center();
    let rect = Rect::from_center_size(center, source_rect.size() * scale);
    if rect.width() <= 0.01 || rect.height() <= 0.01 {
        return;
    }
    let draw_bounds = Rect::from_center_size(center, Vec2::splat(rect.size().length() + 24.0));
    if !painter.clip_rect().intersects(draw_bounds) {
        return;
    }
    let angle = (spawn_rotation + interaction_rotation).to_radians();
    match figure.kind {
        FigureKind::Glyph(glyph) => draw_glyph(painter, rect, glyph, figure.color, opacity, angle),
        FigureKind::Emoji(emoji) => {
            draw_emoji(painter, ctx, cache, rect, emoji, opacity, angle);
        }
        FigureKind::Shape(kind) => {
            draw_shape(painter, rect, kind, figure.color, opacity, angle);
            if faces {
                draw_face(painter, rect, figure, now, opacity, angle);
            }
        }
    }
}

fn draw_glyph(
    painter: &Painter,
    rect: Rect,
    glyph: char,
    color: BabyColor,
    opacity: f32,
    angle: f32,
) {
    let text = glyph.to_string();
    let font = FontId::proportional(GLYPH_FONT_SIZE);
    let galley = painter.layout_no_wrap(text, font, Color32::PLACEHOLDER);
    let mut text_shape = TextShape::new(Pos2::ZERO, galley, Color32::PLACEHOLDER);
    text_shape.transform(TSTransform::from_scaling(
        (rect.height() * 0.88 / GLYPH_FONT_SIZE).max(1.0 / GLYPH_FONT_SIZE),
    ));
    text_shape.pos = rect.center() - text_shape.galley.size() / 2.0;
    let outline = with_opacity(border_for(color), opacity * 0.75);
    for offset in [
        vec2(-4.0, 0.0),
        vec2(4.0, 0.0),
        vec2(0.0, -4.0),
        vec2(0.0, 4.0),
        vec2(-3.0, -3.0),
        vec2(3.0, -3.0),
        vec2(-3.0, 3.0),
        vec2(3.0, 3.0),
    ] {
        let mut outline_shape = text_shape.clone();
        outline_shape.pos += offset;
        outline_shape.fallback_color = outline;
        painter.add(outline_shape.with_angle_and_anchor(angle, Align2::CENTER_CENTER));
    }
    text_shape.fallback_color = with_opacity(rgb(color), opacity);
    painter.add(text_shape.with_angle_and_anchor(angle, Align2::CENTER_CENTER));
}

fn draw_emoji(
    painter: &Painter,
    ctx: &Context,
    cache: &mut TextureCache,
    rect: Rect,
    emoji: &'static str,
    opacity: f32,
    angle: f32,
) {
    let image_rect =
        Rect::from_center_size(rect.center(), Vec2::splat(rect.size().min_elem() * 0.9));
    if let Some(texture) = cache.emoji(ctx, emoji) {
        let mut mesh = Mesh::with_texture(texture.id());
        mesh.add_rect_with_uv(
            image_rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            with_opacity(Color32::WHITE, opacity),
        );
        for vertex in &mut mesh.vertices {
            vertex.pos = rotate(vertex.pos, image_rect.center(), angle);
        }
        painter.add(Shape::mesh(mesh));
    } else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            emoji,
            FontId::proportional(rect.height() * 0.72),
            with_opacity(Color32::WHITE, opacity),
        );
    }
}

fn draw_shape(
    painter: &Painter,
    rect: Rect,
    kind: ShapeKind,
    color: BabyColor,
    opacity: f32,
    angle: f32,
) {
    let base_points = shape_points(kind, rect);
    let is_star = kind == ShapeKind::Star;
    let shadow_points = base_points
        .iter()
        .map(|point| *point + vec2(7.0, 9.0))
        .collect();
    paint_polygon(
        painter,
        rotated(shadow_points, rect.center(), angle),
        with_opacity(Color32::BLACK, opacity * 0.42),
        Stroke::NONE,
        is_star,
    );
    let dark = adjust(rgb(color), -55);
    let light = adjust(rgb(color), 60);
    for layer in 0..12 {
        let progress = layer as f32 / 11.0;
        let layer_center = rect.center() + vec2(8.0 * progress, -8.0 * progress);
        let scale = 1.0 - progress * 0.055;
        let points = base_points
            .iter()
            .map(|point| layer_center + (*point - rect.center()) * scale)
            .collect();
        paint_polygon(
            painter,
            rotated(points, rect.center(), angle),
            with_opacity(lerp_color(dark, light, progress), opacity),
            Stroke::new(1.0, with_opacity(Color32::BLACK, opacity * 0.2)),
            is_star,
        );
    }
    painter.add(Shape::closed_line(
        rotated(base_points, rect.center(), angle),
        Stroke::new(10.0, with_opacity(border_for(color), opacity)),
    ));
}

fn paint_polygon(
    painter: &Painter,
    points: Vec<Pos2>,
    fill: Color32,
    stroke: Stroke,
    concave: bool,
) {
    if !concave {
        painter.add(Shape::convex_polygon(points, fill, stroke));
        return;
    }

    let center = points
        .iter()
        .fold(Vec2::ZERO, |sum, point| sum + point.to_vec2())
        / points.len() as f32;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center.to_pos2(), fill);
    for point in &points {
        mesh.colored_vertex(*point, fill);
    }
    for index in 0..points.len() {
        mesh.add_triangle(
            0,
            index as u32 + 1,
            (index + 1) as u32 % points.len() as u32 + 1,
        );
    }
    painter.add(Shape::mesh(mesh));
    if stroke.width > 0.0 {
        painter.add(Shape::closed_line(points, stroke));
    }
}

fn shape_points(kind: ShapeKind, rect: Rect) -> Vec<Pos2> {
    let center = rect.center();
    let radius = rect.size() * 0.46;
    match kind {
        ShapeKind::Circle | ShapeKind::Oval => regular_polygon(center, radius, 48, -90.0),
        ShapeKind::Square | ShapeKind::Rectangle => vec![
            pos2(rect.left() + 10.0, rect.top() + 10.0),
            pos2(rect.right() - 10.0, rect.top() + 10.0),
            pos2(rect.right() - 10.0, rect.bottom() - 10.0),
            pos2(rect.left() + 10.0, rect.bottom() - 10.0),
        ],
        ShapeKind::Triangle => regular_polygon(center, radius, 3, -90.0),
        ShapeKind::Pentagon => regular_polygon(center, radius, 5, -90.0),
        ShapeKind::Hexagon => {
            const SIN_60_DEGREES: f32 = 0.866_025_4;
            let vertex_radius = radius.x.min(radius.y / SIN_60_DEGREES);
            regular_polygon(center, Vec2::splat(vertex_radius), 6, 0.0)
        }
        ShapeKind::Septagon => regular_polygon(center, radius, 7, -90.0),
        ShapeKind::Octagon => regular_polygon(center, radius, 8, -112.5),
        ShapeKind::Trapezoid => vec![
            pos2(center.x - radius.x * 0.55, center.y - radius.y),
            pos2(center.x + radius.x * 0.55, center.y - radius.y),
            pos2(center.x + radius.x, center.y + radius.y),
            pos2(center.x - radius.x, center.y + radius.y),
        ],
        ShapeKind::Star => {
            let mut points = Vec::with_capacity(10);
            for index in 0..10 {
                let angle = (-90.0 + index as f32 * 36.0).to_radians();
                let factor = if index % 2 == 0 { 1.0 } else { 0.43 };
                points.push(center + vec2(angle.cos() * radius.x, angle.sin() * radius.y) * factor);
            }
            points
        }
    }
}

fn regular_polygon(center: Pos2, radius: Vec2, sides: usize, rotation_degrees: f32) -> Vec<Pos2> {
    (0..sides)
        .map(|index| {
            let angle = (rotation_degrees + index as f32 * 360.0 / sides as f32).to_radians();
            center + vec2(angle.cos() * radius.x, angle.sin() * radius.y)
        })
        .collect()
}

fn draw_face(
    painter: &Painter,
    rect: Rect,
    figure: &Figure,
    now: Instant,
    opacity: f32,
    angle: f32,
) {
    let ink = contrast_for(figure.color);
    let is_star = matches!(&figure.kind, FigureKind::Shape(ShapeKind::Star));
    let center_offset = if is_star { -0.015 } else { 0.03 };
    let center = rect.center() + vec2(0.0, rect.height() * center_offset);
    let blink_interval = 2.1 + (figure.id % 50) as f32 / 10.0;
    let elapsed = now.duration_since(figure.created).as_secs_f32() + (figure.id % 17) as f32 * 0.19;
    let blinking = elapsed % blink_interval < 0.2;
    let face_size = rect.width().min(rect.height());
    let eye_offset = face_size * if is_star { 0.105 } else { 0.12 };
    let eye_radius = face_size * if is_star { 0.047 } else { 0.055 };
    for x in [-eye_offset, eye_offset] {
        let eye_height = if is_star { -0.48 } else { -0.55 };
        let eye = rotate(
            center + vec2(x, eye_offset * eye_height),
            rect.center(),
            angle,
        );
        if blinking {
            let half = vec2(eye_radius, 0.0);
            painter.line_segment(
                [eye - half, eye + half],
                Stroke::new(4.0, with_opacity(ink, opacity)),
            );
        } else {
            painter.circle_filled(eye, eye_radius, with_opacity(Color32::WHITE, opacity));
            painter.circle_filled(
                eye + vec2(eye_radius * 0.12, eye_radius * 0.08),
                eye_radius * 0.48,
                with_opacity(Color32::BLACK, opacity),
            );
        }
    }
    let mouth_width = eye_offset * if is_star { 1.12 } else { 1.35 };
    let mouth_y = center.y + eye_offset * if is_star { 0.55 } else { 0.65 };
    let smile_depth = eye_offset * if is_star { 0.38 } else { 0.55 };
    let mut smile = Vec::with_capacity(13);
    for index in 0..=12 {
        let t = index as f32 / 12.0;
        let x = center.x - mouth_width + mouth_width * 2.0 * t;
        let y = mouth_y + (1.0 - (t * 2.0 - 1.0).powi(2)) * smile_depth;
        smile.push(rotate(pos2(x, y), rect.center(), angle));
    }
    painter.add(Shape::line(
        smile,
        Stroke::new(5.0, with_opacity(ink, opacity)),
    ));
}

fn contrast_for(color: BabyColor) -> Color32 {
    let [red, green, blue] = color.rgb;
    let luminance = 0.2126 * f32::from(red) + 0.7152 * f32::from(green) + 0.0722 * f32::from(blue);
    if luminance < 70.0 {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

fn border_for(color: BabyColor) -> Color32 {
    if color.rgb == [0, 0, 0] {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

pub fn draw_pointer_effects(painter: &Painter, state: &PointerState, now: Instant) {
    draw_rainbow_trail(painter, state);
    draw_neon_worm(painter, state);
    draw_trail_marks(painter, &state.trail_marks, now);
    for particle in &state.particles {
        draw_particle(painter, particle, now);
    }
    for ripple in &state.piano_ripples {
        let progress = ripple.progress(now);
        let eased = 1.0 - (1.0 - progress).powi(3);
        let opacity = (1.0 - progress).powi(2);
        if let Some(note_label) = &ripple.note_label {
            let font = FontId::proportional(24.0 + 28.0 * eased);
            painter.text(
                ripple.position + vec2(2.0, 2.0),
                Align2::CENTER_CENTER,
                note_label,
                font.clone(),
                with_opacity(Color32::BLACK, opacity * 0.8),
            );
            painter.text(
                ripple.position,
                Align2::CENTER_CENTER,
                note_label,
                font,
                with_opacity(Color32::WHITE, opacity),
            );
        } else {
            let radius = 8.0 + 82.0 * eased;
            painter.circle_stroke(
                ripple.position,
                radius,
                Stroke::new(3.0 - 1.5 * progress, with_opacity(Color32::WHITE, opacity)),
            );
        }
    }
}

fn draw_rainbow_trail(painter: &Painter, state: &PointerState) {
    let rainbow = [
        Color32::RED,
        Color32::from_rgb(255, 145, 0),
        Color32::YELLOW,
        Color32::GREEN,
        Color32::from_rgb(40, 120, 255),
        Color32::from_rgb(110, 65, 210),
    ];
    if state.trail.points().len() > 1 {
        for (index, pair) in state.trail.points().windows(2).enumerate() {
            let fade = 1.0 - index as f32 / state.trail.points().len() as f32;
            painter.line_segment(
                [pair[0], pair[1]],
                Stroke::new(
                    54.0 * fade.max(0.25),
                    with_opacity(rainbow[index % rainbow.len()], state.trail.opacity * fade),
                ),
            );
        }
    }
}

fn draw_neon_worm(painter: &Painter, state: &PointerState) {
    let points = state.neon_worm.points();
    if points.len() < 2 {
        return;
    }
    for (index, pair) in points.windows(2).enumerate().rev() {
        if pair[0].distance_sq(pair[1]) < 0.01 {
            continue;
        }
        let tail = index as f32 / (points.len() - 1) as f32;
        let tail_fade = 1.0 - smoothstep(0.75, 1.0, tail);
        let opacity = state.neon_worm.opacity() * tail_fade;
        let color = neon_palette(index);
        for (width, alpha) in [(24.0, 0.025), (12.0, 0.09), (5.0, 0.32), (2.0, 1.0)] {
            painter.line_segment(
                [pair[0], pair[1]],
                Stroke::new(width, with_opacity(color, opacity * alpha)),
            );
        }
    }
}

fn neon_palette(index: usize) -> Color32 {
    let t = index as f32 * 0.0025;
    let channel = |phase: f32| {
        ((0.5 + 0.5 * (std::f32::consts::TAU * (t + phase)).cos()) * 255.0).round() as u8
    };
    Color32::from_rgb(channel(0.0), channel(0.33), channel(0.67))
}

fn draw_trail_marks(painter: &Painter, marks: &[TrailMark], now: Instant) {
    for pair in marks.windows(2) {
        let [from, to] = pair else {
            continue;
        };
        if from.stroke_id != to.stroke_id || from.effect != to.effect {
            continue;
        }
        let progress = (from.progress(now) + to.progress(now)) * 0.5;
        if from.effect == CursorEffect::FadingTrail {
            draw_fading_segment(painter, from.position, to.position, progress);
        }
    }
    for mark in marks {
        let progress = mark.progress(now);
        if mark.effect == CursorEffect::FadingTrail {
            draw_fading_dot(painter, mark.position, progress);
        }
    }
}

fn fading_trail_color(progress: f32) -> (Color32, f32) {
    let opacity = (1.0 - progress).powf(0.72);
    let color = lerp_color(
        Color32::WHITE,
        Color32::from_rgb(65, 85, 255),
        smoothstep(0.0, 0.82, progress),
    );
    (color, opacity)
}

fn draw_fading_dot(painter: &Painter, position: Pos2, progress: f32) {
    let (color, opacity) = fading_trail_color(progress);
    for (radius, alpha) in [(34.0, 0.08), (30.0, 0.2), (25.0, 1.0)] {
        painter.circle_filled(position, radius, with_opacity(color, opacity * alpha));
    }
}

fn draw_fading_segment(painter: &Painter, from: Pos2, to: Pos2, progress: f32) {
    let (color, opacity) = fading_trail_color(progress);
    for (width, alpha) in [(68.0, 0.08), (60.0, 0.2), (50.0, 1.0)] {
        painter.line_segment(
            [from, to],
            Stroke::new(width, with_opacity(color, opacity * alpha)),
        );
    }
}

fn smoothstep(edge_0: f32, edge_1: f32, value: f32) -> f32 {
    let amount = ((value - edge_0) / (edge_1 - edge_0)).clamp(0.0, 1.0);
    amount * amount * (3.0 - 2.0 * amount)
}

fn draw_particle(painter: &Painter, particle: &Particle, now: Instant) {
    let progress =
        (now.duration_since(particle.created).as_secs_f32() / particle.duration).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(2);
    let scale = particle.scale_from + (particle.scale_to - particle.scale_from) * eased;
    let position = particle.start + particle.drift * progress;
    let color = match particle.effect {
        CursorEffect::Sparkles => with_opacity(
            rgb(particle.color),
            (1.0 - progress) * (0.35 + 0.65 * (progress * std::f32::consts::PI * 7.0).sin().abs()),
        ),
        CursorEffect::Bubbles => {
            with_opacity(rgb(particle.color), 0.78 * (1.0 - progress.powf(1.4)))
        }
        _ => with_opacity(rgb(particle.color), 1.0 - progress),
    };
    if particle.effect == CursorEffect::Bubbles {
        painter.circle_stroke(position, 18.0 * scale, Stroke::new(4.0, color));
        painter.circle_filled(
            position + vec2(-5.0, -6.0) * scale,
            3.0 * scale,
            with_opacity(Color32::WHITE, color.a() as f32 / 255.0),
        );
    } else {
        let radius = Vec2::splat(16.0 * scale);
        let mut points = shape_points(
            ShapeKind::Star,
            Rect::from_center_size(position, radius * 2.0),
        );
        let angle = (particle.rotation_from
            + (particle.rotation_to - particle.rotation_from) * progress)
            .to_radians();
        points = rotated(points, position, angle);
        painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
    }
}

pub fn draw_cursor(
    painter: &Painter,
    cache: &TextureCache,
    position: Pos2,
    style: CursorStyle,
    scale: f32,
) {
    match style {
        CursorStyle::Arrow => {
            let points = vec![
                position,
                position + vec2(4.0, 27.0),
                position + vec2(10.0, 19.0),
                position + vec2(16.0, 31.0),
                position + vec2(21.0, 28.0),
                position + vec2(15.0, 17.0),
                position + vec2(26.0, 15.0),
            ];
            painter.add(Shape::convex_polygon(
                points,
                Color32::WHITE,
                Stroke::new(2.0, Color32::BLACK),
            ));
        }
        CursorStyle::Hand => draw_original_hand_cursor(painter, cache, position, scale),
    }
}

const HAND_BASE_SCALE: f32 = 0.5;
const HAND_GRADIENT_RADIUS: f32 = 98.2089;
const HAND_GRADIENT_CENTER: Pos2 = pos2(92.951_17, 112.492_19);
const HAND_GRADIENT_TEXTURE_SIZE: usize = 256;

struct HandGeometry {
    points: Vec<Pos2>,
    triangles: Vec<[usize; 3]>,
}

fn draw_original_hand_cursor(
    painter: &Painter,
    cache: &TextureCache,
    position: Pos2,
    pulse_scale: f32,
) {
    let geometry = hand_geometry();
    let scale = HAND_BASE_SCALE * pulse_scale;
    let transform = |point: Pos2| position + point.to_vec2() * scale;
    let mut mesh = Mesh::with_texture(cache.hand_gradient.id());
    mesh.reserve_vertices(geometry.points.len());
    mesh.reserve_triangles(geometry.triangles.len());
    for point in &geometry.points {
        mesh.vertices.push(Vertex {
            pos: transform(*point),
            uv: point.to_vec2().to_pos2() / HAND_GRADIENT_TEXTURE_SIZE as f32,
            color: Color32::WHITE,
        });
    }
    for triangle in &geometry.triangles {
        mesh.add_triangle(triangle[0] as u32, triangle[1] as u32, triangle[2] as u32);
    }
    painter.add(Shape::mesh(mesh));
    painter.add(Shape::closed_line(
        geometry.points.iter().copied().map(transform).collect(),
        Stroke::new(10.0 * scale, Color32::BLACK),
    ));
}

fn hand_gradient_color(point: Pos2) -> Color32 {
    let amount = (point.distance(HAND_GRADIENT_CENTER) / HAND_GRADIENT_RADIUS).clamp(0.0, 1.0);
    lerp_color(Color32::CYAN, Color32::BLUE, amount)
}

fn hand_gradient_image() -> ColorImage {
    let mut pixels = Vec::with_capacity(HAND_GRADIENT_TEXTURE_SIZE * HAND_GRADIENT_TEXTURE_SIZE);
    for y in 0..HAND_GRADIENT_TEXTURE_SIZE {
        for x in 0..HAND_GRADIENT_TEXTURE_SIZE {
            pixels.push(hand_gradient_color(pos2(x as f32 + 0.5, y as f32 + 0.5)));
        }
    }
    ColorImage::new(
        [HAND_GRADIENT_TEXTURE_SIZE, HAND_GRADIENT_TEXTURE_SIZE],
        pixels,
    )
}

fn hand_geometry() -> &'static HandGeometry {
    static GEOMETRY: OnceLock<HandGeometry> = OnceLock::new();
    GEOMETRY.get_or_init(|| {
        let mut points = vec![pos2(160.514_65, 200.903_32)];
        append_cubic(
            &mut points,
            pos2(151.557_62, 209.861_33),
            pos2(141.947_27, 216.224_61),
            pos2(131.682_62, 219.984_38),
        );
        append_line(&mut points, pos2(115.522_46, 203.824_22));
        append_cubic(
            &mut points,
            pos2(103.382_81, 206.287_11),
            pos2(89.256_836, 205.027_34),
            pos2(73.132_81, 200.036_13),
        );
        append_cubic(
            &mut points,
            pos2(70.903_32, 198.375),
            pos2(68.467_77, 196.377_93),
            pos2(65.874_02, 194.064_45),
        );
        append_cubic(
            &mut points,
            pos2(63.270_508, 191.768_55),
            pos2(60.490_234, 189.119_14),
            pos2(57.532_227, 186.151_37),
        );
        append_cubic(
            &mut points,
            pos2(46.186_523, 174.805_66),
            pos2(39.262_695, 166.342_77),
            pos2(36.771_484, 160.735_35),
        );
        append_cubic(
            &mut points,
            pos2(34.270_508, 155.136_72),
            pos2(35.194_336, 150.163_09),
            pos2(39.533_203, 145.833_98),
        );
        append_cubic(
            &mut points,
            pos2(40.186_523, 145.180_66),
            pos2(41.082_03, 144.424_8),
            pos2(42.249_023, 143.557_62),
        );
        append_cubic(
            &mut points,
            pos2(35.157_227, 134.450_2),
            pos2(35.800_78, 125.698_24),
            pos2(44.189_453, 117.319_336),
        );
        append_line(&mut points, pos2(47.446_29, 114.063_48));
        append_cubic(
            &mut points,
            pos2(43.685_547, 105.833_984),
            pos2(45.672_85, 97.846_68),
            pos2(53.408_203, 90.111_33),
        );
        append_line(&mut points, pos2(59.697_266, 83.822_266));
        append_line(&mut points, pos2(25.332_031, 48.161_133));
        append_line(&mut points, pos2(19.155_273, 42.189_453));
        append_cubic(
            &mut points,
            pos2(11.999_023, 35.032_227),
            pos2(7.557_617, 28.715_82),
            pos2(5.821_289, 23.219_727),
        );
        append_cubic(
            &mut points,
            pos2(4.085_938, 17.724_121),
            pos2(5.103_516, 13.105_469),
            pos2(8.854_492, 9.345_215),
        );
        append_cubic(
            &mut points,
            pos2(13.052_734, 5.155_762),
            pos2(18.147_46, 3.989_258),
            pos2(24.137_695, 5.874_023),
        );
        append_cubic(
            &mut points,
            pos2(30.137_695, 7.749_512),
            pos2(37.041_992, 12.592_285),
            pos2(44.842_773, 20.401_855),
        );
        append_line(&mut points, pos2(52.754_883, 28.314_453));
        append_line(&mut points, pos2(91.244_14, 65.067_38));
        append_line(&mut points, pos2(127.885_74, 75.686_52));
        append_cubic(
            &mut points,
            pos2(138.055_66, 94.142_58),
            pos2(147.956_05, 117.478_516),
            pos2(157.584_96, 145.721_68),
        );
        append_line(&mut points, pos2(180.902_34, 169.039_06));
        append_cubic(
            &mut points,
            pos2(177.431_64, 180.161_13),
            pos2(170.638_67, 190.780_27),
            pos2(160.514_65, 200.903_32),
        );
        if points
            .last()
            .is_some_and(|last| last.distance_sq(points[0]) < 0.0001)
        {
            points.pop();
        }
        let triangles = triangulate_polygon(&points);
        HandGeometry { points, triangles }
    })
}

fn append_line(points: &mut Vec<Pos2>, end: Pos2) {
    if points
        .last()
        .is_none_or(|last| last.distance_sq(end) > 0.0001)
    {
        points.push(end);
    }
}

fn append_cubic(points: &mut Vec<Pos2>, control_1: Pos2, control_2: Pos2, end: Pos2) {
    let Some(start) = points.last().copied() else {
        return;
    };
    let curve = CubicBezierShape::from_points_stroke(
        [start, control_1, control_2, end],
        false,
        Color32::TRANSPARENT,
        Stroke::NONE,
    );
    points.extend(curve.flatten(Some(0.35)).into_iter().skip(1));
}

fn triangulate_polygon(points: &[Pos2]) -> Vec<[usize; 3]> {
    if points.len() < 3 {
        return Vec::new();
    }
    let counter_clockwise = polygon_area(points) > 0.0;
    let mut remaining = (0..points.len()).collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity(points.len().saturating_sub(2));
    while remaining.len() > 3 {
        let mut ear = None;
        for current in 0..remaining.len() {
            let previous = remaining[(current + remaining.len() - 1) % remaining.len()];
            let point = remaining[current];
            let next = remaining[(current + 1) % remaining.len()];
            let turn = cross(
                points[point] - points[previous],
                points[next] - points[point],
            );
            if (counter_clockwise && turn <= 0.0001) || (!counter_clockwise && turn >= -0.0001) {
                continue;
            }
            if remaining.iter().copied().any(|candidate| {
                candidate != previous
                    && candidate != point
                    && candidate != next
                    && point_in_triangle(
                        points[candidate],
                        points[previous],
                        points[point],
                        points[next],
                    )
            }) {
                continue;
            }
            triangles.push([previous, point, next]);
            ear = Some(current);
            break;
        }
        let Some(ear) = ear else {
            break;
        };
        remaining.remove(ear);
    }
    if remaining.len() == 3 {
        triangles.push([remaining[0], remaining[1], remaining[2]]);
    }
    triangles
}

fn polygon_area(points: &[Pos2]) -> f32 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum::<f32>()
        * 0.5
}

fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn point_in_triangle(point: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let first = cross(b - a, point - a);
    let second = cross(c - b, point - b);
    let third = cross(a - c, point - c);
    let has_negative = first < -0.0001 || second < -0.0001 || third < -0.0001;
    let has_positive = first > 0.0001 || second > 0.0001 || third > 0.0001;
    !has_negative || !has_positive
}

fn rgb(color: BabyColor) -> Color32 {
    Color32::from_rgb(color.rgb[0], color.rgb[1], color.rgb[2])
}

fn adjust(color: Color32, amount: i16) -> Color32 {
    let channel = |value: u8| (i16::from(value) + amount).clamp(0, 255) as u8;
    Color32::from_rgb(channel(color.r()), channel(color.g()), channel(color.b()))
}

fn lerp_color(from: Color32, to: Color32, amount: f32) -> Color32 {
    let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount).round() as u8;
    Color32::from_rgb(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
    )
}

fn with_opacity(color: Color32, opacity: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (opacity.clamp(0.0, 1.0) * f32::from(color.a())).round() as u8,
    )
}

fn rotate(point: Pos2, center: Pos2, angle: f32) -> Pos2 {
    let offset = point - center;
    let (sin, cos) = angle.sin_cos();
    center
        + vec2(
            offset.x * cos - offset.y * sin,
            offset.x * sin + offset.y * cos,
        )
}

fn rotated(points: Vec<Pos2>, center: Pos2, angle: f32) -> Vec<Pos2> {
    points
        .into_iter()
        .map(|point| rotate(point, center, angle))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::COLORS;

    #[test]
    fn only_black_items_receive_white_borders() {
        for color in COLORS {
            let expected = if color.name == "Black" {
                Color32::WHITE
            } else {
                Color32::BLACK
            };
            assert_eq!(border_for(color), expected, "{} item border", color.name);
        }
    }

    #[test]
    fn original_hand_cursor_path_is_fully_triangulated() {
        let geometry = hand_geometry();
        assert!(geometry.points.len() > 40);
        assert_eq!(geometry.triangles.len(), geometry.points.len() - 2);
    }
}
