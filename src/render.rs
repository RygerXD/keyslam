use std::{collections::HashMap, time::Instant};

use eframe::egui::{
    Align2, Color32, Context, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, TextureHandle,
    TextureOptions, Vec2, epaint::TextShape, pos2, vec2,
};
use include_dir::{Dir, include_dir};

use crate::{
    game::{BabyColor, Figure, FigureKind, Particle, PointerState},
    responses::ShapeKind,
    settings::{CursorEffect, CursorStyle},
};

static EMOJI: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/emoji");

#[derive(Default)]
pub struct TextureCache {
    emoji: HashMap<&'static str, TextureHandle>,
}

impl TextureCache {
    fn emoji(&mut self, ctx: &Context, value: &'static str) -> Option<&TextureHandle> {
        if !self.emoji.contains_key(value) {
            let file_name = format!(
                "{}.png",
                value
                    .chars()
                    .map(|character| format!("{:x}", character as u32))
                    .collect::<Vec<_>>()
                    .join("-")
            );
            let file = EMOJI.get_file(&file_name)?;
            let image = image::load_from_memory(file.contents()).ok()?.into_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            let color_image =
                eframe::egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
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
    let font = FontId::proportional(rect.height() * 0.88);
    let galley = painter.layout_no_wrap(text, font, Color32::PLACEHOLDER);
    let position = rect.center() - galley.size() / 2.0;
    let outline = with_opacity(contrast_for(color), opacity * 0.75);
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
        painter.add(
            TextShape::new(position + offset, galley.clone(), outline)
                .with_angle_and_anchor(angle, Align2::CENTER_CENTER),
        );
    }
    painter.add(
        TextShape::new(position, galley, with_opacity(rgb(color), opacity))
            .with_angle_and_anchor(angle, Align2::CENTER_CENTER),
    );
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
    let shadow_points = base_points
        .iter()
        .map(|point| *point + vec2(7.0, 9.0))
        .collect();
    painter.add(Shape::convex_polygon(
        rotated(shadow_points, rect.center(), angle),
        with_opacity(Color32::BLACK, opacity * 0.42),
        Stroke::NONE,
    ));
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
        painter.add(Shape::convex_polygon(
            rotated(points, rect.center(), angle),
            with_opacity(lerp_color(dark, light, progress), opacity),
            Stroke::new(1.0, with_opacity(Color32::BLACK, opacity * 0.2)),
        ));
    }
    painter.add(Shape::closed_line(
        rotated(base_points, rect.center(), angle),
        Stroke::new(10.0, with_opacity(contrast_for(color), opacity)),
    ));
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
        ShapeKind::Hexagon => regular_polygon(center, radius, 6, -90.0),
        ShapeKind::Septagon => regular_polygon(center, radius, 7, -90.0),
        ShapeKind::Octagon => regular_polygon(center, radius, 8, -90.0),
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
    let center = rect.center() + vec2(0.0, rect.height() * 0.03);
    let blink_interval = 2.1 + (figure.id % 50) as f32 / 10.0;
    let elapsed = now.duration_since(figure.created).as_secs_f32() + (figure.id % 17) as f32 * 0.19;
    let blinking = elapsed % blink_interval < 0.2;
    let eye_offset = rect.width().min(rect.height()) * 0.12;
    let eye_radius = rect.width().min(rect.height()) * 0.055;
    for x in [-eye_offset, eye_offset] {
        let eye = rotate(center + vec2(x, -eye_offset * 0.55), rect.center(), angle);
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
    let mouth_width = eye_offset * 1.35;
    let mouth_y = center.y + eye_offset * 0.65;
    let mut smile = Vec::with_capacity(13);
    for index in 0..=12 {
        let t = index as f32 / 12.0;
        let x = center.x - mouth_width + mouth_width * 2.0 * t;
        let y = mouth_y + (1.0 - (t * 2.0 - 1.0).powi(2)) * eye_offset * 0.55;
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

pub fn draw_pointer_effects(painter: &Painter, state: &PointerState, now: Instant) {
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
                    18.0 * fade.max(0.25),
                    with_opacity(rainbow[index % rainbow.len()], state.trail.opacity * fade),
                ),
            );
        }
    }
    for particle in &state.particles {
        draw_particle(painter, particle, now);
    }
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

pub fn draw_cursor(painter: &Painter, position: Pos2, style: CursorStyle) {
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
        CursorStyle::Hand => {
            painter.circle_filled(position + vec2(11.0, 16.0), 11.0, Color32::WHITE);
            painter.rect_filled(
                Rect::from_min_size(position + vec2(7.0, 0.0), vec2(8.0, 20.0)),
                4.0,
                Color32::WHITE,
            );
            painter.circle_stroke(
                position + vec2(11.0, 16.0),
                11.0,
                Stroke::new(2.0, Color32::BLACK),
            );
        }
    }
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
