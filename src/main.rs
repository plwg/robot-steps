//! Robot Steps: a first coding game. Tap arrows to build a program, press play,
//! and watch the robot follow it to collect the stars. No reading required.

// On Windows, don't open a console window alongside the game.
#![windows_subsystem = "windows"]

mod level;
mod progress;

use std::f32::consts::PI;

use level::{
    Block, Cmd, Dir, Event, Exec, Item, Level, Loc, MAX_TIMES, Pos, Sense, Step, Tile, World, tiles,
};
use macroquad::prelude::*;
use macroquad::rand::gen_range;

// Everything is drawn on a fixed 16:9 canvas that is scaled to fit the window.
const W: f32 = 1422.0;
const H: f32 = 800.0;

const STEP_TIME: f32 = 0.45;
const CHOP_TIME: f32 = 0.7;
const BUMP_TIME: f32 = 0.8;
const SAD_TIME: f32 = 1.4;
const CHECK_TIME: f32 = 0.7;
const REVEAL_TIME: f32 = 0.8;

const BOARD: Rect = Rect {
    x: 20.0,
    y: 70.0,
    w: 1060.0,
    h: 540.0,
};
const MAX_CELL: f32 = 108.0;

const SLOTS_PER_ROW: usize = 12;
const SLOT: f32 = 60.0;
const SLOT_GAP: f32 = 8.0;
const STRIP_Y: f32 = 652.0;
/// Loop brackets sit in a gap this wide between tiles.
const BRACKET_W: f32 = 26.0;

const SKY: Color = Color::new(0.62, 0.84, 0.97, 1.0);
const GRASS: Color = Color::new(0.56, 0.82, 0.42, 1.0);
const GRASS_DARK: Color = Color::new(0.50, 0.77, 0.37, 1.0);
const STAR: Color = Color::new(1.0, 0.82, 0.1, 1.0);
const SHADOW: Color = Color::new(0.0, 0.0, 0.0, 0.18);
const PANEL: Color = Color::new(0.93, 0.96, 1.0, 1.0);
const SLOT_EMPTY: Color = Color::new(0.82, 0.88, 0.95, 1.0);
const DISABLED: Color = Color::new(0.75, 0.78, 0.82, 1.0);
const INK: Color = Color::new(0.15, 0.2, 0.32, 1.0);
const PLAY_GREEN: Color = Color::new(0.15, 0.7, 0.25, 1.0);
const STOP_RED: Color = Color::new(0.9, 0.25, 0.25, 1.0);
const UNDO_YELLOW: Color = Color::new(0.95, 0.7, 0.1, 1.0);
const AXE_BROWN: Color = Color::new(0.62, 0.4, 0.22, 1.0);
const REPEAT_TEAL: Color = Color::new(0.1, 0.65, 0.65, 1.0);
const CHECK_SLATE: Color = Color::new(0.33, 0.36, 0.55, 1.0);
/// Each function's shape colour: ● and ▲.
const FN_COLOURS: [Color; 2] = [
    Color::new(1.0, 0.55, 0.3, 1.0),
    Color::new(0.35, 0.85, 0.95, 1.0),
];
const CALL_INK: Color = Color::new(0.24, 0.27, 0.4, 1.0);
const IF_LIME: Color = Color::new(0.5, 0.72, 0.1, 1.0);
const IF_BAND: Color = Color::new(0.85, 0.94, 0.7, 1.0);
const YES_GREEN: Color = Color::new(0.2, 0.75, 0.3, 1.0);
const NO_GREY: Color = Color::new(0.55, 0.58, 0.62, 1.0);
const LOOP_BAND: Color = Color::new(0.72, 0.9, 0.9, 1.0);
const CLOUD: Color = Color::new(0.97, 0.97, 1.0, 1.0);
const WOOD: Color = Color::new(0.55, 0.35, 0.2, 1.0);
const LEAF: Color = Color::new(0.16, 0.55, 0.22, 1.0);

fn dir_color(dir: Dir) -> Color {
    match dir {
        Dir::Up => Color::new(0.25, 0.55, 0.95, 1.0),
        Dir::Down => Color::new(1.0, 0.6, 0.15, 1.0),
        Dir::Left => Color::new(0.65, 0.4, 0.9, 1.0),
        Dir::Right => Color::new(0.95, 0.35, 0.6, 1.0),
    }
}

fn dir_vec(dir: Dir) -> Vec2 {
    let (dx, dy) = dir.delta();
    vec2(dx as f32, dy as f32)
}

fn pos_vec((x, y): Pos) -> Vec2 {
    vec2(x as f32, y as f32)
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        1.0,
    )
}

fn darker(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, c.a)
}

// ---------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    let r = r.min(w / 2.0).min(h / 2.0);
    draw_rectangle(x + r, y, w - 2.0 * r, h, color);
    draw_rectangle(x, y + r, r, h - 2.0 * r, color);
    draw_rectangle(x + w - r, y + r, r, h - 2.0 * r, color);
    for (cx, cy) in [
        (x + r, y + r),
        (x + w - r, y + r),
        (x + r, y + h - r),
        (x + w - r, y + h - r),
    ] {
        draw_circle(cx, cy, r, color);
    }
}

/// A quad given as four corners.
fn draw_quad(a: Vec2, b: Vec2, c: Vec2, d: Vec2, color: Color) {
    draw_triangle(a, b, c, color);
    draw_triangle(a, c, d, color);
}

/// Text centred on a point. Rendered at the real screen resolution so it
/// stays sharp however much the canvas is scaled up.
fn draw_label(text: &str, centre: Vec2, size: f32, color: Color) {
    let scale = (screen_width() / W).min(screen_height() / H) * screen_dpi_scale();
    let font_size = (size * scale).round() as u16;
    let dims = measure_text(text, None, font_size, 1.0 / scale);
    draw_text_ex(
        text,
        centre.x - dims.width / 2.0,
        centre.y + dims.offset_y / 2.0,
        TextParams {
            font_size,
            font_scale: 1.0 / scale,
            color,
            ..Default::default()
        },
    );
}

/// A big question mark, drawn a few times over itself to make it bold.
fn draw_question(centre: Vec2, size: f32, color: Color) {
    let k = size * 0.035;
    for (dx, dy) in [(-k, 0.0), (k, 0.0), (0.0, -k), (0.0, k)] {
        draw_label("?", centre + vec2(dx, dy), size, color);
    }
}

/// A chunky arrow pointing in `dir`, sized to fit a box of `size`.
fn draw_arrow(c: Vec2, size: f32, dir: Dir, color: Color) {
    let d = dir_vec(dir);
    let n = vec2(-d.y, d.x);
    let p = |along: f32, across: f32| c + d * along * size + n * across * size;
    draw_quad(
        p(-0.4, -0.13),
        p(0.05, -0.13),
        p(0.05, 0.13),
        p(-0.4, 0.13),
        color,
    );
    draw_triangle(p(0.42, 0.0), p(0.0, -0.36), p(0.0, 0.36), color);
}

fn draw_star(c: Vec2, r: f32, spin: f32, color: Color) {
    let point = |i: f32, radius: f32| {
        let a = spin - PI / 2.0 + i * PI / 5.0;
        c + vec2(a.cos(), a.sin()) * radius
    };
    for i in 0..5 {
        let i = i as f32 * 2.0;
        let outer = point(i, r);
        draw_triangle(c, point(i - 1.0, r * 0.45), outer, color);
        draw_triangle(c, outer, point(i + 1.0, r * 0.45), color);
    }
}

fn draw_arc(c: Vec2, r: f32, from: f32, to: f32, thickness: f32, color: Color) {
    const SEGMENTS: usize = 16;
    let at = |t: f32| {
        let a = from + (to - from) * t;
        c + vec2(a.cos(), a.sin()) * r
    };
    for i in 0..SEGMENTS {
        let a = at(i as f32 / SEGMENTS as f32);
        let b = at((i + 1) as f32 / SEGMENTS as f32);
        draw_line(a.x, a.y, b.x, b.y, thickness, color);
        draw_circle(b.x, b.y, thickness / 2.0, color);
    }
}

/// A tree that can be shrunk (`scale`) and leaned over (`sway`) while being chopped.
fn draw_tree(c: Vec2, cell: f32, scale: f32, sway: f32) {
    let s = cell * scale;
    let base = c + vec2(0.0, cell * 0.37);
    draw_ellipse(base.x, base.y, s * 0.3, s * 0.08, 0.0, SHADOW);
    let top = base + vec2(sway * s, -s * 0.37);
    draw_quad(
        base - vec2(s * 0.07, 0.0),
        base + vec2(s * 0.07, 0.0),
        top + vec2(s * 0.07, 0.0),
        top - vec2(s * 0.07, 0.0),
        WOOD,
    );
    let light = Color::new(0.24, 0.66, 0.3, 1.0);
    let crown = top - vec2(0.0, s * 0.05);
    draw_circle(crown.x - s * 0.15, crown.y, s * 0.2, LEAF);
    draw_circle(crown.x + s * 0.15, crown.y, s * 0.2, LEAF);
    draw_circle(crown.x, crown.y - s * 0.18, s * 0.24, LEAF);
    draw_circle(crown.x - s * 0.06, crown.y - s * 0.24, s * 0.1, light);
}

fn draw_stump(c: Vec2, cell: f32) {
    let s = cell;
    let base = c + vec2(0.0, s * 0.2);
    draw_ellipse(base.x, base.y + s * 0.08, s * 0.22, s * 0.07, 0.0, SHADOW);
    draw_rectangle(
        base.x - s * 0.14,
        base.y - s * 0.08,
        s * 0.28,
        s * 0.14,
        WOOD,
    );
    draw_ellipse(
        base.x,
        base.y - s * 0.08,
        s * 0.14,
        s * 0.06,
        0.0,
        darker(WOOD, 1.5),
    );
    draw_ellipse(
        base.x,
        base.y - s * 0.08,
        s * 0.07,
        s * 0.03,
        0.0,
        darker(WOOD, 1.2),
    );
}

fn draw_rock(c: Vec2, cell: f32) {
    let s = cell;
    let base = c + vec2(0.0, s * 0.25);
    draw_ellipse(base.x, base.y + s * 0.08, s * 0.36, s * 0.08, 0.0, SHADOW);
    let grey = Color::new(0.55, 0.57, 0.62, 1.0);
    let dark = darker(grey, 0.85);
    draw_circle(base.x - s * 0.14, base.y - s * 0.06, s * 0.2, dark);
    draw_circle(base.x + s * 0.12, base.y - s * 0.04, s * 0.22, dark);
    draw_circle(base.x - s * 0.02, base.y - s * 0.18, s * 0.24, grey);
    draw_circle(
        base.x - s * 0.1,
        base.y - s * 0.26,
        s * 0.08,
        darker(grey, 1.25),
    );
}

/// A puffy cloud with a question mark: a tree or grass, nobody knows yet.
fn draw_mystery(c: Vec2, cell: f32, scale: f32, time: f32) {
    let s = cell * scale;
    let c = c + vec2(0.0, (time * 2.5).sin() * cell * 0.03);
    draw_ellipse(c.x, c.y + cell * 0.36, s * 0.3, s * 0.07, 0.0, SHADOW);
    draw_puff(c, s);
    if scale > 0.3 {
        draw_question(c, 64.0 * s / MAX_CELL, CHECK_SLATE);
    }
}

/// A puffy cloud `s` across, shaded underneath.
fn draw_puff(c: Vec2, s: f32) {
    let shade = darker(CLOUD, 0.88);
    for (dx, dy, r) in [(-0.2, 0.06, 0.17), (0.2, 0.06, 0.17), (0.0, 0.1, 0.2)] {
        draw_circle(c.x + dx * s, c.y + dy * s + s * 0.03, r * s, shade);
    }
    for (dx, dy, r) in [
        (-0.2, 0.04, 0.17),
        (0.2, 0.04, 0.17),
        (-0.08, -0.1, 0.2),
        (0.1, -0.08, 0.19),
        (0.0, 0.08, 0.2),
    ] {
        draw_circle(c.x + dx * s, c.y + dy * s, r * s, CLOUD);
    }
}

/// A thought bubble with a question mark, for when the robot is checking.
fn draw_thought(robot: Vec2, cell: f32) {
    let s = cell;
    let bubble = robot + vec2(s * 0.42, -s * 0.62);
    draw_circle(robot.x + s * 0.2, robot.y - s * 0.32, s * 0.04, WHITE);
    draw_circle(robot.x + s * 0.28, robot.y - s * 0.42, s * 0.06, WHITE);
    draw_circle(bubble.x, bubble.y, s * 0.2, WHITE);
    draw_question(bubble, 44.0 * s / MAX_CELL, CHECK_SLATE);
}

/// An axe with its blade facing `dir`; `swing` tilts it (radians).
fn draw_axe(c: Vec2, size: f32, dir: Dir, swing: f32, handle: Color) {
    let fwd = dir_vec(dir);
    let (s, co) = swing.sin_cos();
    let fwd = vec2(fwd.x * co - fwd.y * s, fwd.x * s + fwd.y * co);
    // The handle runs at right angles to `dir`, with the head at one end.
    let along = vec2(-fwd.y, fwd.x);
    let p = |a: f32, b: f32| c + along * a * size + fwd * b * size;
    draw_quad(
        p(-0.42, -0.05),
        p(-0.42, 0.05),
        p(0.4, 0.05),
        p(0.4, -0.05),
        handle,
    );
    let blade = Color::new(0.78, 0.8, 0.85, 1.0);
    draw_quad(
        p(0.15, 0.0),
        p(0.4, 0.0),
        p(0.5, 0.36),
        p(0.05, 0.36),
        blade,
    );
    draw_quad(
        p(0.05, 0.3),
        p(0.5, 0.3),
        p(0.5, 0.36),
        p(0.05, 0.36),
        WHITE,
    );
}

/// A function's name: a circle for function 1, a triangle for function 2.
fn draw_shape(c: Vec2, r: f32, function: usize) {
    let colour = FN_COLOURS[(function - 1) % FN_COLOURS.len()];
    if function == 1 {
        draw_circle(c.x, c.y, r, colour);
    } else {
        draw_triangle(
            c + vec2(0.0, -r * 1.1),
            c + vec2(-r * 1.05, r * 0.75),
            c + vec2(r * 1.05, r * 0.75),
            colour,
        );
    }
}

/// A square bracket filling part of `r`: `[` if `open`, otherwise `]`.
fn draw_bracket(r: Rect, x: f32, open: bool, color: Color) {
    let t = r.w * 0.11;
    let (top, bottom) = (r.y + r.h * 0.18, r.y + r.h * 0.82);
    let serif = if open { r.w * 0.2 } else { -r.w * 0.2 };
    draw_rectangle(x - t / 2.0, top, t, bottom - top, color);
    for y in [top, bottom - t] {
        let (x0, x1) = (x - t / 2.0, x + serif);
        draw_rectangle(x0.min(x1), y, (x1 - x0).abs() + t / 2.0, t, color);
    }
}

/// A circular "loop" arrow.
fn draw_loop_icon(c: Vec2, r: f32, color: Color) {
    let rr = r * 0.55;
    let thick = r * 0.13;
    let end = -0.35 * PI;
    // The arc runs round to `end`, and the arrowhead carries on past it.
    draw_arc(c, rr, end - 2.0 * PI + 0.9, end, thick, color);
    let out = vec2(end.cos(), end.sin());
    let tip = c + out * rr;
    let tangent = vec2(-end.sin(), end.cos());
    draw_triangle(
        tip - tangent * r * 0.18 + out * r * 0.2,
        tip - tangent * r * 0.18 - out * r * 0.2,
        tip + tangent * r * 0.16,
        color,
    );
}

#[derive(Clone, Copy, PartialEq)]
enum Face {
    Calm,
    Happy,
    Sad,
}

/// Each player's robot has its own colours.
#[derive(Clone, Copy)]
struct Paint {
    body: Color,
    trim: Color,
}

const PAINTS: [Paint; progress::SLOTS] = [
    Paint {
        body: Color::new(0.55, 0.78, 0.98, 1.0),
        trim: Color::new(0.3, 0.5, 0.8, 1.0),
    },
    Paint {
        body: Color::new(1.0, 0.76, 0.45, 1.0),
        trim: Color::new(0.85, 0.48, 0.18, 1.0),
    },
    Paint {
        body: Color::new(0.62, 0.88, 0.55, 1.0),
        trim: Color::new(0.28, 0.62, 0.3, 1.0),
    },
];

fn draw_robot(c: Vec2, cell: f32, face: Face, look: Vec2, time: f32, lift: f32, paint: Paint) {
    let s = cell * 0.8;
    // Shadow stays on the ground while the robot hops.
    draw_ellipse(
        c.x,
        c.y + s * 0.42 + lift,
        s * 0.36 * (1.0 - lift / cell),
        s * 0.08,
        0.0,
        SHADOW,
    );

    let Paint { body, trim } = paint;
    for side in [-1.0, 1.0] {
        draw_circle(c.x + side * s * 0.22, c.y + s * 0.34, s * 0.1, DARKGRAY);
    }
    let bob = (time * 4.0).sin() * s * 0.02;
    draw_line(
        c.x,
        c.y - s * 0.3,
        c.x,
        c.y - s * 0.46 + bob,
        s * 0.04,
        trim,
    );
    draw_circle(c.x, c.y - s * 0.48 + bob, s * 0.07, RED);
    rounded_rect(
        c.x - s * 0.38,
        c.y - s * 0.32,
        s * 0.76,
        s * 0.66,
        s * 0.14,
        trim,
    );
    rounded_rect(
        c.x - s * 0.35,
        c.y - s * 0.29,
        s * 0.7,
        s * 0.6,
        s * 0.12,
        body,
    );
    rounded_rect(
        c.x - s * 0.27,
        c.y - s * 0.2,
        s * 0.54,
        s * 0.38,
        s * 0.08,
        INK,
    );
    let blink = (time % 3.7) < 0.12;
    for side in [-1.0, 1.0] {
        let eye = vec2(c.x + side * s * 0.12, c.y - s * 0.06);
        if blink {
            draw_line(
                eye.x - s * 0.06,
                eye.y,
                eye.x + s * 0.06,
                eye.y,
                s * 0.03,
                WHITE,
            );
        } else {
            draw_circle(eye.x, eye.y, s * 0.075, WHITE);
            let pupil = eye + look * s * 0.03;
            draw_circle(pupil.x, pupil.y, s * 0.038, INK);
        }
    }
    let mouth = Color::new(0.4, 1.0, 0.6, 1.0);
    let m = vec2(c.x, c.y + s * 0.07);
    match face {
        Face::Happy => draw_arc(
            m - vec2(0.0, s * 0.04),
            s * 0.1,
            0.15 * PI,
            0.85 * PI,
            s * 0.03,
            mouth,
        ),
        Face::Sad => draw_arc(
            m + vec2(0.0, s * 0.07),
            s * 0.09,
            1.2 * PI,
            1.8 * PI,
            s * 0.03,
            ORANGE,
        ),
        Face::Calm => draw_line(m.x - s * 0.06, m.y, m.x + s * 0.06, m.y, s * 0.03, mouth),
    }
}

fn draw_play_icon(c: Vec2, r: f32, color: Color) {
    draw_triangle(
        c + vec2(-r * 0.35, -r * 0.45),
        c + vec2(-r * 0.35, r * 0.45),
        c + vec2(r * 0.5, 0.0),
        color,
    );
}

/// A level's number, or for a demo (which is for watching) a play sign.
fn draw_level_label(level: &Level, c: Vec2, size: f32, color: Color) {
    match level.demo {
        Some(_) => draw_play_icon(c + vec2(size * 0.06, 0.0), size * 0.95, color),
        None => draw_label(&level.number.to_string(), c, size, color),
    }
}

/// A "backspace" key shape: remove the last step.
fn draw_undo_icon(c: Vec2, r: f32) {
    let (l, rt, t, b) = (
        c.x - r * 0.55,
        c.x + r * 0.5,
        c.y - r * 0.35,
        c.y + r * 0.35,
    );
    let body_left = l + r * 0.3;
    draw_triangle(vec2(l, c.y), vec2(body_left, t), vec2(body_left, b), WHITE);
    draw_rectangle(body_left, t, rt - body_left, b - t, WHITE);
    let x = vec2(body_left + (rt - body_left) * 0.5, c.y);
    let k = r * 0.15;
    draw_line(x.x - k, x.y - k, x.x + k, x.y + k, r * 0.1, UNDO_YELLOW);
    draw_line(x.x - k, x.y + k, x.x + k, x.y - k, r * 0.1, UNDO_YELLOW);
}

/// A trash can: throw the whole program away.
fn draw_trash_icon(c: Vec2, r: f32) {
    let w = r * 0.7;
    let h = r * 0.75;
    draw_rectangle(c.x - w * 0.6, c.y - h * 0.55, w * 1.2, r * 0.1, WHITE);
    draw_rectangle(c.x - w * 0.15, c.y - h * 0.68, w * 0.3, r * 0.1, WHITE);
    draw_rectangle(c.x - w / 2.0, c.y - h * 0.38, w, h, WHITE);
    for i in [-1.0, 0.0, 1.0] {
        let x = c.x + i * w * 0.25;
        draw_line(x, c.y - h * 0.25, x, c.y + h * 0.5, r * 0.06, STOP_RED);
    }
}

/// Four corner brackets: go full screen (or back).
fn draw_fullscreen_icon(c: Vec2, r: f32, color: Color) {
    let k = r * 0.55;
    let l = r * 0.35;
    let t = r * 0.14;
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let corner = c + vec2(sx * k, sy * k);
        draw_line(corner.x, corner.y, corner.x - sx * l, corner.y, t, color);
        draw_line(corner.x, corner.y, corner.x, corner.y - sy * l, t, color);
    }
}

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Button {
    Arrow(Dir),
    Axe,
    Check,
    Repeat,
    Loop,
    /// Start (then pick a direction) or end an "if" block.
    If(Sense),
    /// Add a call to function 1 or 2.
    Call(usize),
    Play,
    Undo,
    Clear,
    Fullscreen,
    /// Back to the level map.
    Map,
}

/// Each fixed button with its centre and half-size. The tool buttons (axe,
/// check, repeat, loop, calls) and play are laid out per level, as only some
/// levels have the tools.
const BUTTONS: [(Button, Vec2, f32); 8] = [
    (Button::Arrow(Dir::Up), vec2(1256.0, 130.0), 42.0),
    (Button::Arrow(Dir::Left), vec2(1166.0, 215.0), 42.0),
    (Button::Arrow(Dir::Right), vec2(1346.0, 215.0), 42.0),
    (Button::Arrow(Dir::Down), vec2(1256.0, 300.0), 42.0),
    (Button::Undo, vec2(1186.0, 728.0), 40.0),
    (Button::Clear, vec2(1326.0, 728.0), 40.0),
    (Button::Fullscreen, vec2(1386.0, 36.0), 22.0),
    (Button::Map, vec2(48.0, 36.0), 26.0),
];

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    /// Building the program.
    Edit,
    /// Play was pressed: the mystery clouds turn into trees or grass.
    Reveal,
    /// Looking to see if there's a tree.
    Checking,
    /// Walking one step.
    Moving,
    /// Chopping down a tree.
    Chopping,
    /// Walked into something, or swung the axe at something that isn't a tree.
    Bump,
    /// Program finished (or got stuck) without getting every star.
    Sad,
    /// All stars collected!
    Win,
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    color: Color,
    life: f32,
    size: f32,
    spin: f32,
    star: bool,
}

struct Game {
    levels: Vec<Level>,
    /// What the save files call each level.
    keys: Vec<String>,
    idx: usize,
    solved: Vec<bool>,
    /// The main program (strip 0), then each function.
    strips: Vec<Vec<Item>>,
    /// Which strip the buttons add to.
    editing: usize,
    /// The axe or check button was tapped; the next arrow makes that block.
    armed: Option<Button>,
    /// Where the last thing added went, in the strip being edited (it isn't
    /// always the end, when it fills an opened-up slot).
    last_added: Option<usize>,
    world: World,
    mode: Mode,
    timer: f32,
    /// The program unrolled into single actions, and which one is running.
    trace: Vec<Exec>,
    pc: usize,
    /// What the robot is doing right now, for animation.
    event: Option<Event>,
    facing: Dir,
    failed_step: Option<Loc>,
    particles: Vec<Particle>,
    pressed: Option<(Button, f32)>,
    time: f32,
    fullscreen: bool,
    /// Which screen is showing, and whose game it is.
    screen: Screen,
    slot: usize,
    /// Each player's finished levels (for the player cards).
    finished: [usize; progress::SLOTS],
    /// The bin on a player card was tapped once; a second tap (soon after)
    /// erases that player.
    erase_armed: Option<(usize, f32)>,
    /// Where the mouse is, and recent clicks (for the ripple).
    mouse: Vec2,
    clicks: Vec<(Vec2, f32)>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Screen {
    /// Pick one of the three players.
    Players,
    /// The level map for the current player.
    Map,
    /// Playing a level.
    Play,
}

/// The idea each level is about, for colouring the map.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Move,
    Axe,
    Repeat,
    Check,
    Loop,
    If,
    Function,
}

impl Kind {
    fn of(level: &Level) -> Kind {
        if level.functions > 0 {
            Kind::Function
        } else if level.if_tree || level.if_star {
            Kind::If
        } else if level.loops {
            Kind::Loop
        } else if level.check {
            Kind::Check
        } else if level.repeat {
            Kind::Repeat
        } else if level.chop {
            Kind::Axe
        } else {
            Kind::Move
        }
    }

    fn colour(self) -> Color {
        match self {
            Kind::Move => dir_color(Dir::Up),
            Kind::Axe => AXE_BROWN,
            Kind::Repeat => Color::new(0.2, 0.72, 0.72, 1.0),
            Kind::Check => CHECK_SLATE,
            Kind::Loop => Color::new(0.05, 0.5, 0.52, 1.0),
            Kind::If => IF_LIME,
            Kind::Function => FN_COLOURS[0],
        }
    }
}

impl Game {
    fn new(fullscreen: bool) -> Game {
        let levels = level::all_levels();
        let keys: Vec<String> = levels.iter().map(Level::key).collect();
        let solved = progress::load(0, &keys);
        let idx = 0;
        let world = levels[idx].world();
        let mut game = Game {
            levels,
            keys,
            idx,
            solved,
            strips: vec![Vec::new()],
            editing: 0,
            armed: None,
            last_added: None,
            world,
            mode: Mode::Edit,
            timer: 0.0,
            trace: Vec::new(),
            pc: 0,
            event: None,
            facing: Dir::Right,
            failed_step: None,
            particles: Vec::new(),
            pressed: None,
            time: 0.0,
            fullscreen,
            screen: Screen::Players,
            slot: 0,
            finished: [0; progress::SLOTS],
            erase_armed: None,
            mouse: vec2(W / 2.0, H / 2.0),
            clicks: Vec::new(),
        };
        game.load(idx);
        game.count_finished();
        game
    }

    fn paint(&self) -> Paint {
        PAINTS[self.slot]
    }

    /// Re-read how far each player has got, for the player cards.
    fn count_finished(&mut self) {
        for slot in 0..progress::SLOTS {
            self.finished[slot] = progress::load(slot, &self.keys)
                .iter()
                .filter(|&&d| d)
                .count();
        }
    }

    fn choose_player(&mut self, slot: usize) {
        self.slot = slot;
        self.solved = progress::load(slot, &self.keys);
        self.open_map();
    }

    fn open_map(&mut self) {
        self.particles.clear();
        self.reset_robot();
        self.screen = Screen::Map;
    }

    fn open_players(&mut self) {
        self.count_finished();
        self.erase_armed = None;
        self.screen = Screen::Players;
    }

    fn open_level(&mut self, idx: usize) {
        // Set the level up properly (its function strips, and so on).
        self.load(idx);
        self.screen = Screen::Play;
    }

    /// The first level this player hasn't finished, if any.
    fn next_unfinished(&self) -> Option<usize> {
        self.solved.iter().position(|&done| !done)
    }

    fn level(&self) -> &Level {
        &self.levels[self.idx]
    }

    fn load(&mut self, idx: usize) {
        self.idx = idx;
        // A demo starts with its program written, ready to play.
        self.strips = match &self.level().demo {
            Some(strips) => strips.clone(),
            None => vec![Vec::new(); 1 + self.level().functions],
        };
        self.editing = 0;
        self.last_added = None;
        self.particles.clear();
        self.armed = None;
        self.failed_step = None;
        self.reset_robot();
    }

    /// After a win: on to the next level, or back to the map after the last.
    fn next_level(&mut self) {
        if self.idx + 1 < self.levels.len() {
            self.load(self.idx + 1);
        } else {
            self.open_map();
        }
    }

    fn reset_robot(&mut self) {
        self.world = self.level().world();
        self.facing = Dir::Right;
        self.event = None;
        self.pc = 0;
        self.set_mode(Mode::Edit);
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.timer = 0.0;
    }

    fn running(&self) -> bool {
        matches!(
            self.mode,
            Mode::Reveal | Mode::Moving | Mode::Checking | Mode::Chopping | Mode::Bump | Mode::Sad
        )
    }

    /// The strip being edited.
    fn prog(&self) -> &Vec<Item> {
        &self.strips[self.editing]
    }

    fn prog_mut(&mut self) -> &mut Vec<Item> {
        &mut self.strips[self.editing]
    }

    fn slots_of(&self, strip: usize) -> usize {
        if strip == 0 {
            self.level().slots
        } else {
            self.level().fn_slots
        }
    }

    /// Is there a free tile? (Loop brackets don't use tiles.)
    fn has_room(&self) -> bool {
        tiles(self.prog()) < self.slots_of(self.editing)
    }

    /// Does the program do anything at all?
    fn has_actions(&self) -> bool {
        !level::expand(&self.strips, &self.world).is_empty()
    }

    /// Add an item to the strip being edited. Blocks fill an opened-up slot
    /// if there is one; brackets go just before it; otherwise it goes on the
    /// end.
    fn insert(&mut self, item: Item) {
        let items = &mut self.strips[self.editing];
        let gap = items.iter().position(|&i| i == Item::Gap);
        let at = match (gap, item) {
            (Some(g), Item::Block(_) | Item::Call(_) | Item::IfStart { .. }) => {
                items[g] = item;
                g
            }
            (Some(g), _) => {
                items.insert(g, item);
                g
            }
            (None, _) => {
                items.push(item);
                items.len() - 1
            }
        };
        self.last_added = Some(at);
    }

    /// Take out the last thing added (the end of the strip, if that's not
    /// known).
    fn remove_last_added(&mut self) {
        let len = self.prog().len();
        match self.last_added.take().filter(|&i| i < len) {
            Some(i) => {
                self.prog_mut().remove(i);
            }
            None => {
                self.prog_mut().pop();
            }
        }
    }

    /// Is there room for another block: a free tile, or an opened-up slot?
    fn can_place(&self) -> bool {
        self.has_room() || self.prog().contains(&Item::Gap)
    }

    /// The slot whose count the repeat button changes: the last block added
    /// (or the start of the loop, if a loop was just closed).
    fn count_slot(&self) -> Option<usize> {
        let items = self.prog();
        let last = self
            .last_added
            .filter(|&i| i < items.len())
            .or(items.len().checked_sub(1))?;
        let (start, _) = pair_ends(items, last).unwrap_or((last, None));
        matches!(items[start], Item::Block(_) | Item::LoopStart { .. }).then_some(start)
    }

    /// What tapping repeat would change the last count to.
    fn next_times(&self) -> u8 {
        match self.count_slot().map(|i| self.prog()[i]) {
            Some(Item::Block(b)) => b.times % MAX_TIMES + 1,
            Some(Item::LoopStart { times }) if times >= MAX_TIMES => 2,
            Some(Item::LoopStart { times }) => times + 1,
            _ => 2,
        }
    }

    /// Set the last count. Looping once isn't a loop, so loops go from 2.
    fn set_times(&mut self, n: u8) {
        if let Some(i) = self.count_slot() {
            match &mut self.prog_mut()[i] {
                Item::Block(b) => b.times = n,
                Item::LoopStart { times } => *times = n.max(2),
                _ => {}
            }
        }
    }

    fn visible(&self, button: Button) -> bool {
        match button {
            Button::Axe => self.level().chop,
            Button::Check => self.level().check,
            Button::Repeat => self.level().repeat,
            Button::Loop => self.level().loops,
            Button::If(Sense::Tree) => self.level().if_tree,
            Button::If(Sense::Star) => self.level().if_star,
            Button::Call(f) => f <= self.level().functions,
            _ => true,
        }
    }

    fn enabled(&self, button: Button) -> bool {
        let editing = self.mode == Mode::Edit;
        match button {
            Button::Arrow(_) | Button::Axe | Button::Check => editing && self.can_place(),
            // A loop can hold an "if", but an "if" can't hold a loop.
            Button::Loop => {
                let open_if = open_if(self.prog()).is_some();
                editing && (open_loop(self.prog()).is_some() || (self.has_room() && !open_if))
            }
            Button::If(_) => editing && (self.can_place() || open_if(self.prog()).is_some()),
            // Calls only go in the main program.
            Button::Call(_) => editing && self.editing == 0 && self.can_place(),
            Button::Repeat => editing && self.count_slot().is_some(),
            Button::Clear => editing && !self.prog().is_empty(),
            Button::Undo => editing && (self.armed.is_some() || !self.prog().is_empty()),
            Button::Play => self.running() || (editing && self.has_actions()),
            Button::Fullscreen | Button::Map => true,
        }
    }

    fn press(&mut self, button: Button) {
        if !self.visible(button) || !self.enabled(button) {
            return;
        }
        self.pressed = Some((button, self.time));
        if button != Button::Fullscreen {
            self.failed_step = None;
        }
        match button {
            Button::Arrow(dir) => {
                let cmd = match self.armed.take() {
                    Some(Button::Axe) => Cmd::Chop(dir),
                    Some(Button::Check) => Cmd::IfTreeChop(dir),
                    Some(Button::If(sense)) => {
                        self.insert(Item::IfStart { sense, dir });
                        return;
                    }
                    _ => Cmd::Move(dir),
                };
                self.insert(Item::Block(Block { cmd, times: 1 }));
            }
            Button::Axe | Button::Check => {
                self.armed = (self.armed != Some(button)).then_some(button);
            }
            Button::Repeat => self.set_times(self.next_times()),
            Button::Loop => match open_loop(self.prog()) {
                // Tapped again straight away: never mind.
                Some(start) if self.last_added == Some(start) => self.remove_last_added(),
                Some(_) => {
                    // Closing the loop closes any "if" left open inside it.
                    if open_if(self.prog()).is_some() {
                        self.insert(Item::IfEnd);
                    }
                    self.insert(Item::LoopEnd);
                }
                None => self.insert(Item::LoopStart { times: 2 }),
            },
            // With an "if" open, the button closes it; otherwise it's like the
            // axe: tap it, then pick a direction.
            Button::If(_) => match open_if(self.prog()) {
                Some(start) if self.last_added == Some(start) => self.remove_last_added(),
                Some(_) => self.insert(Item::IfEnd),
                None => self.armed = (self.armed != Some(button)).then_some(button),
            },
            Button::Call(f) => {
                self.armed = None;
                self.insert(Item::Call(f));
            }
            Button::Undo if self.armed.is_some() => self.armed = None,
            Button::Undo => self.remove_last_added(),
            Button::Clear => {
                self.prog_mut().clear();
                self.last_added = None;
                self.armed = None;
            }
            Button::Play if self.running() => self.reset_robot(),
            Button::Play => {
                self.armed = None;
                self.reset_robot();
                self.start_run();
            }
            Button::Fullscreen => {
                self.fullscreen = !self.fullscreen;
                set_fullscreen(self.fullscreen);
            }
            Button::Map => self.open_map(),
        }
    }

    /// Reveal what's under the mystery clouds (different every time), then go.
    fn start_run(&mut self) {
        let mysteries = self.level().mysteries.clone();
        if mysteries.is_empty() {
            self.trace = level::expand(&self.strips, &self.world);
            self.act();
            return;
        }
        let outcomes = self.level().outcomes();
        let outcome = &outcomes[gen_range(0, outcomes.len())];
        self.world = self.level().revealed(outcome);
        // A program runs the same way every time in the same world, so the
        // whole run can be worked out now that the clouds are gone.
        self.trace = level::expand(&self.strips, &self.world);
        for p in mysteries {
            let (c, cell) = self.cell_centre(p);
            self.burst(c, cell, CLOUD, false);
        }
        self.set_mode(Mode::Reveal);
    }

    /// Start the next action of the program.
    fn act(&mut self) {
        let Some(exec) = self.trace.get(self.pc).copied() else {
            // Ran out of blocks with stars still left.
            self.set_mode(Mode::Sad);
            return;
        };
        let cmd = match exec.step {
            Step::Do(cmd) => cmd,
            // An "if" looks before deciding.
            Step::Test { dir, yes, .. } => {
                self.facing = dir;
                let (dx, dy) = dir.delta();
                let at = (self.world.pos.0 + dx, self.world.pos.1 + dy);
                self.event = Some(Event::Tested { at, yes });
                self.set_mode(Mode::Checking);
                return;
            }
        };
        self.facing = cmd.dir();
        match self.world.act(cmd) {
            Ok(event) => {
                self.event = Some(event);
                self.set_mode(match event {
                    Event::Moved { .. } => Mode::Moving,
                    Event::Chopped(_) => Mode::Chopping,
                    Event::Checked { .. } | Event::Tested { .. } => Mode::Checking,
                });
            }
            Err(()) => {
                self.failed_step = Some(exec.at);
                self.set_mode(Mode::Bump);
            }
        }
    }

    /// The current action has finished: celebrate, or carry on with the program.
    fn finish_action(&mut self) {
        // Checked and found a tree: now chop it down.
        if let Some(Event::Checked { at, tree: true }) = self.event {
            self.event = Some(Event::Chopped(at));
            self.set_mode(Mode::Chopping);
            return;
        }
        match self.event {
            Some(Event::Moved { star: true, .. }) => {
                let (c, cell) = self.cell_centre(self.world.pos);
                self.burst(c, cell, STAR, true);
            }
            Some(Event::Chopped(p)) => {
                let (c, cell) = self.cell_centre(p);
                self.burst(c - vec2(0.0, cell * 0.2), cell, LEAF, false);
            }
            _ => {}
        }
        self.event = None;
        if self.world.stars.is_empty() {
            self.solved[self.idx] = true;
            progress::save(self.slot, &self.keys, &self.solved);
            self.set_mode(Mode::Win);
            self.confetti();
            return;
        }
        self.pc += 1;
        self.act();
    }

    fn update(&mut self, dt: f32) {
        self.time += dt;
        let now = self.time;
        self.clicks.retain(|&(_, t)| now - t < 0.5);
        self.timer += dt;
        match self.mode {
            Mode::Reveal if self.timer >= REVEAL_TIME => self.act(),
            Mode::Moving if self.timer >= STEP_TIME => self.finish_action(),
            Mode::Checking if self.timer >= CHECK_TIME => self.finish_action(),
            Mode::Chopping if self.timer >= CHOP_TIME => self.finish_action(),
            Mode::Bump if self.timer >= BUMP_TIME => self.set_mode(Mode::Sad),
            Mode::Sad if self.timer >= SAD_TIME => self.reset_robot(),
            Mode::Win if self.timer > 1.5 && (self.timer % 1.2) < dt => self.confetti(),
            _ => {}
        }

        for p in &mut self.particles {
            p.vel.y += 600.0 * dt;
            p.pos += p.vel * dt;
            p.life -= dt;
            p.spin += dt * 5.0;
        }
        self.particles
            .retain(|p| p.life > 0.0 && p.pos.y < H + 50.0);
    }

    fn handle_click(&mut self, m: Vec2) {
        self.clicks.push((m, self.time));
        let (_, fs_centre, fs_half) = BUTTONS
            .into_iter()
            .find(|b| b.0 == Button::Fullscreen)
            .expect("there's a full screen button");
        if (m - fs_centre).abs().max_element() <= fs_half {
            self.press(Button::Fullscreen);
            return;
        }
        match self.screen {
            Screen::Players => self.click_players(m),
            Screen::Map => self.click_map(m),
            Screen::Play => self.click_play(m),
        }
    }

    fn click_play(&mut self, m: Vec2) {
        for (button, centre, half) in self.buttons() {
            if self.visible(button) && (m - centre).abs().max_element() <= half {
                if self.mode == Mode::Win && button != Button::Fullscreen {
                    break;
                }
                self.press(button);
                return;
            }
        }
        if self.mode == Mode::Win {
            if self.timer > 0.8 {
                self.next_level();
            }
            return;
        }
        if self.mode == Mode::Edit {
            self.click_strip(m);
        }
    }

    fn click_players(&mut self, m: Vec2) {
        let armed = self.erase_armed.take();
        for slot in 0..progress::SLOTS {
            if self.finished[slot] > 0 && (m - erase_button(slot)).length() < 30.0 {
                match armed {
                    // Second tap on the same bin: erase that player.
                    Some((s, t)) if s == slot && self.time - t < 4.0 => {
                        progress::save(slot, &self.keys, &vec![false; self.keys.len()]);
                        self.count_finished();
                    }
                    _ => self.erase_armed = Some((slot, self.time)),
                }
                return;
            }
        }
        // Any other tap just cancels a half-done erase.
        if armed.is_some() {
            return;
        }
        if let Some(slot) = (0..progress::SLOTS).find(|&s| player_card(s).contains(m)) {
            self.choose_player(slot);
        }
    }

    fn click_map(&mut self, m: Vec2) {
        if (m - BACK_BUTTON).abs().max_element() <= 30.0 {
            self.open_players();
            return;
        }
        if let Some(i) = (0..self.levels.len()).find(|&i| (m - self.map_node(i)).length() < 40.0) {
            self.open_level(i);
        }
    }

    fn handle_keys(&mut self) {
        if is_key_pressed(KeyCode::F11) || (is_key_pressed(KeyCode::Escape) && self.fullscreen) {
            self.press(Button::Fullscreen);
        }
        let typed = get_char_pressed();
        match self.screen {
            Screen::Players => {
                // 1, 2 or 3 picks a player.
                if let Some(n) = typed.and_then(|c| c.to_digit(10))
                    && (1..=progress::SLOTS as u32).contains(&n)
                {
                    self.choose_player(n as usize - 1);
                }
                return;
            }
            Screen::Map => {
                if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
                    self.open_level(self.next_unfinished().unwrap_or(0));
                } else if is_key_pressed(KeyCode::Backspace) {
                    self.open_players();
                }
                return;
            }
            Screen::Play => {}
        }
        if is_key_pressed(KeyCode::M) {
            self.open_map();
            return;
        }
        if self.mode == Mode::Win {
            if [KeyCode::Space, KeyCode::Enter, KeyCode::Right]
                .into_iter()
                .any(is_key_pressed)
            {
                self.next_level();
            }
            return;
        }
        let keys = [
            (KeyCode::Up, Button::Arrow(Dir::Up)),
            (KeyCode::Down, Button::Arrow(Dir::Down)),
            (KeyCode::Left, Button::Arrow(Dir::Left)),
            (KeyCode::Right, Button::Arrow(Dir::Right)),
            (KeyCode::C, Button::Axe),
            (KeyCode::Q, Button::Check),
            (KeyCode::R, Button::Repeat),
            (KeyCode::L, Button::Loop),
            (KeyCode::T, Button::If(Sense::Tree)),
            (KeyCode::S, Button::If(Sense::Star)),
            (KeyCode::F, Button::Call(1)),
            (KeyCode::G, Button::Call(2)),
            (KeyCode::Space, Button::Play),
            (KeyCode::Enter, Button::Play),
            (KeyCode::Backspace, Button::Undo),
            (KeyCode::Delete, Button::Clear),
        ];
        for (key, button) in keys {
            if is_key_pressed(key) {
                self.press(button);
            }
        }
        // Number keys set how many times the last block (or loop) repeats.
        if let Some(n) = typed.and_then(|ch| ch.to_digit(10))
            && n >= 1
            && self.visible(Button::Repeat)
            && self.enabled(Button::Repeat)
        {
            self.set_times(n as u8);
        }
        // Tab moves between the main program and the functions.
        if is_key_pressed(KeyCode::Tab) && self.mode == Mode::Edit {
            self.editing = (self.editing + 1) % self.strips.len();
        }
    }

    // --- geometry -------------------------------------------------------

    /// The tool buttons this level has: one row of up to four, or two rows.
    fn tools(&self) -> Vec<Vec<Button>> {
        let tools: Vec<Button> = [
            Button::Axe,
            Button::Check,
            Button::Repeat,
            Button::Loop,
            Button::If(Sense::Tree),
            Button::If(Sense::Star),
            Button::Call(1),
            Button::Call(2),
        ]
        .into_iter()
        .filter(|&b| self.visible(b))
        .collect();
        if tools.len() <= 4 {
            vec![tools]
        } else {
            let (a, b) = tools.split_at(tools.len().div_ceil(2));
            vec![a.to_vec(), b.to_vec()]
        }
    }

    /// The bottom of the panel that holds the arrows and tools.
    fn panel_bottom(&self) -> f32 {
        match self.tools().len() {
            _ if self.tools()[0].is_empty() => 370.0,
            1 => 470.0,
            _ => 515.0,
        }
    }

    /// Every button for this level, with its centre and half-size.
    fn buttons(&self) -> Vec<(Button, Vec2, f32)> {
        let rows = self.tools();
        let (ys, max_half): (&[f32], f32) = if rows.len() == 1 {
            (&[412.0], 40.0)
        } else {
            (&[388.0, 472.0], 35.0)
        };
        let widest = rows.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let spacing = (272.0 / widest as f32).min(92.0);
        let half = (spacing * 0.45).min(max_half);
        let mut buttons = BUTTONS.to_vec();
        for (row, &y) in rows.iter().zip(ys) {
            let middle = (row.len() as f32 - 1.0) / 2.0;
            for (i, &b) in row.iter().enumerate() {
                buttons.push((b, vec2(1256.0 + (i as f32 - middle) * spacing, y), half));
            }
        }
        let play = if rows.len() == 1 {
            (vec2(1256.0, 572.0), 64.0)
        } else {
            (vec2(1256.0, 602.0), 56.0)
        };
        buttons.push((Button::Play, play.0, play.1));
        buttons
    }

    /// Where level `i` sits on the map: a path that winds back and forth.
    fn map_node(&self, i: usize) -> Vec2 {
        let cols = MAP_COLS;
        let rows = self.levels.len().div_ceil(cols);
        let (row, mut col) = (i / cols, i % cols);
        if row % 2 == 1 {
            col = cols - 1 - col;
        }
        let x_step = ((W - 200.0) / (cols - 1) as f32).min(154.0);
        let y_step = (600.0 / rows as f32).min(115.0);
        let x0 = W / 2.0 - x_step * (cols - 1) as f32 / 2.0;
        vec2(x0 + col as f32 * x_step, 150.0 + row as f32 * y_step)
    }

    /// Where the map goes. Levels with functions have more program strips,
    /// so the map gets a little shorter.
    fn board(&self) -> Rect {
        if self.level().functions == 0 {
            return BOARD;
        }
        let top = self.strip_origin(0).y - 12.0;
        Rect::new(BOARD.x, BOARD.y, BOARD.w, top - 30.0 - BOARD.y)
    }

    fn cell_size(&self) -> f32 {
        let l = self.level();
        let board = self.board();
        (board.w / l.w as f32)
            .min(board.h / l.h as f32)
            .min(MAX_CELL)
    }

    fn grid_origin(&self) -> Vec2 {
        let l = self.level();
        self.board().center() - vec2(l.w as f32, l.h as f32) * self.cell_size() / 2.0
    }

    fn cell_centre_f(&self, p: Vec2) -> Vec2 {
        self.grid_origin() + (p + vec2(0.5, 0.5)) * self.cell_size()
    }

    fn cell_centre(&self, p: Pos) -> (Vec2, f32) {
        (self.cell_centre_f(pos_vec(p)), self.cell_size())
    }

    fn strip_cols(&self, strip: usize) -> usize {
        self.slots_of(strip).min(SLOTS_PER_ROW)
    }

    /// With functions, every strip is one row with its name (a shape, or ▶
    /// for the main program) in front, stacked at the bottom of the screen.
    fn labelled(&self) -> bool {
        self.level().functions > 0
    }

    /// The left edge and top of a strip's first slot (or its label).
    fn strip_origin(&self, strip: usize) -> Vec2 {
        let step = SLOT + SLOT_GAP;
        if !self.labelled() {
            let width = self.strip_cols(0) as f32 * step - SLOT_GAP;
            return vec2(BOARD.center().x - width / 2.0, STRIP_Y);
        }
        let strips = self.strips.len();
        let widest = (0..strips).map(|s| self.slots_of(s) + 1).max().unwrap_or(1);
        let x = BOARD.center().x - (widest as f32 * step - SLOT_GAP) / 2.0;
        // Each strip is a panel (one row plus padding) with a gap between.
        let panel = SLOT + 24.0;
        let gap = 14.0;
        let total = strips as f32 * panel + (strips - 1) as f32 * gap;
        let top = H - 8.0 - total;
        vec2(x, top + 12.0 + strip as f32 * (panel + gap))
    }

    fn label_rect(&self, strip: usize) -> Rect {
        let o = self.strip_origin(strip);
        Rect::new(o.x, o.y, SLOT, SLOT)
    }

    /// Where each part of a strip goes. Tiles run along in rows; each loop
    /// bracket takes a narrow gap between two tiles.
    fn layout(&self, strip: usize) -> StripLayout {
        let cols = self.strip_cols(strip);
        let step = SLOT + SLOT_GAP;
        let o = self.strip_origin(strip);
        let x0 = if self.labelled() { o.x + step } else { o.x };
        let (mut x, mut y, mut col) = (x0, o.y, 0);
        let wrap = |x: &mut f32, y: &mut f32, col: &mut usize| {
            if *col == cols {
                *col = 0;
                *x = x0;
                *y += step;
            }
        };
        let mut items = Vec::new();
        for item in &self.strips[strip] {
            match item {
                Item::LoopStart { .. } | Item::LoopEnd | Item::IfEnd => {
                    if matches!(item, Item::LoopStart { .. }) {
                        wrap(&mut x, &mut y, &mut col);
                    }
                    items.push(Rect::new(x - SLOT_GAP / 2.0, y, BRACKET_W, SLOT));
                    x += BRACKET_W;
                }
                Item::Block(_) | Item::Call(_) | Item::IfStart { .. } | Item::Gap => {
                    wrap(&mut x, &mut y, &mut col);
                    items.push(Rect::new(x, y, SLOT, SLOT));
                    x += step;
                    col += 1;
                }
            }
        }
        let used = tiles(&self.strips[strip]);
        let mut empty = Vec::new();
        for _ in used..self.slots_of(strip) {
            wrap(&mut x, &mut y, &mut col);
            empty.push(Rect::new(x, y, SLOT, SLOT));
            x += step;
            col += 1;
        }
        // The panel fits a full row of tiles, plus any brackets.
        let full_row = x0 + cols as f32 * step - SLOT_GAP;
        let all = items.iter().chain(&empty);
        let right = all.clone().map(Rect::right).fold(full_row, f32::max);
        let bottom = all.map(Rect::bottom).fold(o.y + SLOT, f32::max);
        let panel = Rect::new(
            o.x - 12.0,
            o.y - 12.0,
            right - o.x + 24.0,
            bottom - o.y + 24.0,
        );
        StripLayout {
            items,
            empty,
            panel,
        }
    }

    /// A tap on a strip picks it for editing, and a tap on a block takes it
    /// out (tapping either end of a loop takes out the loop, but keeps the
    /// blocks inside). Returns whether the tap landed on a strip.
    fn click_strip(&mut self, m: Vec2) -> bool {
        // The eraser on the item under the pointer takes it out.
        if let Some((strip, i)) = self.hovered_item()
            && (m - self.eraser_at(strip, i)).length() < ERASER_R + 4.0
        {
            self.editing = strip;
            self.erase(strip, i);
            return true;
        }
        for strip in 0..self.strips.len() {
            if self.labelled() && self.label_rect(strip).contains(m) {
                self.editing = strip;
                return true;
            }
            let layout = self.layout(strip);
            if layout.empty.iter().any(|r| r.contains(m)) {
                self.editing = strip;
                return true;
            }
            let Some(i) = layout.items.iter().position(|r| r.contains(m)) else {
                continue;
            };
            // Tapping a block opens up an empty slot just before it, for the
            // next block to go in.
            // There's only ever one: an earlier one moves here.
            self.editing = strip;
            let room = tiles(&self.strips[strip]) < self.slots_of(strip);
            let items = &mut self.strips[strip];
            let old = items.iter().position(|&it| it == Item::Gap);
            if items[i] != Item::Gap && (room || old.is_some()) {
                let i = match old {
                    Some(g) => {
                        items.remove(g);
                        if g < i { i - 1 } else { i }
                    }
                    None => i,
                };
                items.insert(i, Item::Gap);
                self.last_added = None;
                self.failed_step = None;
            }
            return true;
        }
        false
    }

    /// Take out item `i` of a strip. Taking out either end of a loop or "if"
    /// takes out both ends, but keeps the blocks inside.
    fn erase(&mut self, strip: usize, i: usize) {
        let items = &mut self.strips[strip];
        match pair_ends(items, i) {
            Some((start, end)) => {
                if let Some(end) = end {
                    items.remove(end);
                }
                items.remove(start);
            }
            None => {
                items.remove(i);
            }
        }
        self.last_added = None;
        self.failed_step = None;
    }

    /// The program item under the pointer, while editing.
    fn hovered_item(&self) -> Option<Loc> {
        if self.screen != Screen::Play || self.mode != Mode::Edit {
            return None;
        }
        (0..self.strips.len()).find_map(|strip| {
            let layout = self.layout(strip);
            // Include the eraser itself, which pokes out of the item.
            let i = layout.items.iter().enumerate().position(|(i, r)| {
                r.contains(self.mouse)
                    || (self.mouse - self.eraser_at(strip, i)).length() < ERASER_R + 4.0
            })?;
            Some((strip, i))
        })
    }

    /// Where an item's eraser goes: the top-left corner of a block, above a
    /// closing bracket, or under an opening one (whose top has the count).
    fn eraser_at(&self, strip: usize, i: usize) -> Vec2 {
        let r = self.layout(strip).items[i];
        match self.strips[strip][i] {
            Item::LoopStart { .. } => vec2(r.center().x, r.bottom() + 8.0),
            Item::LoopEnd | Item::IfEnd => vec2(r.center().x, r.y - 8.0),
            _ => vec2(r.x + 4.0, r.y + 4.0),
        }
    }

    // --- effects --------------------------------------------------------

    fn burst(&mut self, at: Vec2, cell: f32, color: Color, star: bool) {
        for _ in 0..24 {
            let a = gen_range(0.0, 2.0 * PI);
            let speed = gen_range(150.0, 400.0);
            self.particles.push(Particle {
                pos: at,
                vel: vec2(a.cos(), a.sin()) * speed - vec2(0.0, 150.0),
                color,
                life: gen_range(0.5, 1.0),
                size: cell * gen_range(0.06, 0.12),
                spin: gen_range(0.0, PI),
                star,
            });
        }
    }

    fn confetti(&mut self) {
        let colors = [RED, ORANGE, YELLOW, GREEN, SKYBLUE, PURPLE, PINK];
        for _ in 0..120 {
            self.particles.push(Particle {
                pos: vec2(gen_range(0.0, W), gen_range(-200.0, 0.0)),
                vel: vec2(gen_range(-80.0, 80.0), gen_range(0.0, 200.0)),
                color: colors[gen_range(0, colors.len())],
                life: 4.0,
                size: gen_range(6.0, 12.0),
                spin: gen_range(0.0, PI),
                star: gen_range(0, 4) == 0,
            });
        }
    }

    // --- drawing --------------------------------------------------------

    fn draw(&self) {
        clear_background(SKY);
        match self.screen {
            Screen::Players => self.draw_players(),
            Screen::Map => self.draw_map(),
            Screen::Play => self.draw_play(),
        }
        draw_fullscreen_icon(vec2(1386.0, 36.0), 22.0, INK);
        self.draw_cursor();
    }

    fn draw_play(&self) {
        self.draw_board();
        self.draw_program();
        self.draw_buttons();
        for p in &self.particles {
            if p.star {
                draw_star(p.pos, p.size, p.spin, p.color);
            } else {
                let (s, c) = p.spin.sin_cos();
                let d = vec2(c, s) * p.size;
                let n = vec2(-d.y, d.x) * 0.5;
                draw_quad(
                    p.pos - d - n,
                    p.pos + d - n,
                    p.pos + d + n,
                    p.pos - d + n,
                    p.color,
                );
            }
        }
        if self.mode == Mode::Win && self.timer > 0.8 {
            self.draw_next_prompt();
        }
        // The level's number, next to the map button.
        let c = vec2(104.0, 36.0);
        draw_circle(c.x, c.y, 22.0, WHITE);
        draw_circle(c.x, c.y, 19.0, Kind::of(self.level()).colour());
        draw_level_label(self.level(), c, 26.0, WHITE);
    }

    /// Three cards, one per player, each with its own robot and progress.
    fn draw_players(&self) {
        self.draw_scenery();
        draw_title(vec2(W / 2.0, 92.0), self.time);
        let total = self.levels.len();
        for (slot, &paint) in PAINTS.iter().enumerate() {
            let mut r = player_card(slot);
            let hover = r.contains(self.mouse);
            if hover {
                r.y -= 8.0;
            }
            rounded_rect(r.x, r.y + 10.0, r.w, r.h, 30.0, SHADOW);
            rounded_rect(r.x, r.y, r.w, r.h, 30.0, PANEL);
            let robot = vec2(r.center().x, r.y + 150.0);
            let face = if hover { Face::Happy } else { Face::Calm };
            let bob = (self.time * 3.0 + slot as f32).sin().abs() * 8.0 * f32::from(hover);
            draw_robot(
                robot - vec2(0.0, bob),
                230.0,
                face,
                Vec2::ZERO,
                self.time,
                bob,
                paint,
            );

            // A star and how many levels are done, with a bar for how far.
            let done = self.finished[slot];
            let row = vec2(r.center().x, r.y + 300.0);
            draw_star(row - vec2(40.0, 0.0), 26.0, 0.0, STAR);
            draw_label(&format!("{done}"), row + vec2(24.0, 0.0), 52.0, INK);
            let bar = Rect::new(r.x + 40.0, r.y + 350.0, r.w - 80.0, 22.0);
            rounded_rect(bar.x, bar.y, bar.w, bar.h, 11.0, SLOT_EMPTY);
            let fill = bar.w * done as f32 / total as f32;
            if fill > 0.0 {
                rounded_rect(bar.x, bar.y, fill.max(bar.h), bar.h, 11.0, paint.trim);
            }

            // The bin: tap twice to start this player again.
            if done > 0 {
                let c = erase_button(slot);
                let armed = self.erase_armed.is_some_and(|(s, _)| s == slot);
                let pulse = (self.time * 8.0).sin() * 0.5 + 0.5;
                let (colour, size) = if armed {
                    (STOP_RED, 28.0 + pulse * 4.0)
                } else {
                    (DISABLED, 24.0)
                };
                rounded_rect(c.x - size, c.y - size, size * 2.0, size * 2.0, 12.0, colour);
                draw_trash_icon(c, size);
            }
        }
    }

    /// The players screen's backdrop: a sun, drifting clouds, hills with
    /// trees, and a robot hopping along the grass collecting stars.
    fn draw_scenery(&self) {
        let t = self.time;
        let sun = vec2(96.0, 92.0);
        for i in 0..12 {
            let a = t * 0.3 + i as f32 * PI / 6.0;
            let d = vec2(a.cos(), a.sin());
            let (a, b) = (sun + d * 62.0, sun + d * 82.0);
            draw_line(a.x, a.y, b.x, b.y, 8.0, STAR);
        }
        draw_circle(sun.x, sun.y, 52.0, STAR);
        draw_circle(
            sun.x - 14.0,
            sun.y - 14.0,
            18.0,
            Color::new(1.0, 0.9, 0.45, 1.0),
        );

        for (i, (y, size, speed)) in [(60.0, 150.0, 14.0), (150.0, 110.0, 22.0), (40.0, 90.0, 9.0)]
            .into_iter()
            .enumerate()
        {
            let span = W + size * 2.0;
            let x = (i as f32 * 520.0 + t * speed) % span - size;
            draw_puff(vec2(x, y), size);
        }

        for (x, r) in [
            (-60.0, 300.0),
            (420.0, 380.0),
            (980.0, 340.0),
            (1460.0, 300.0),
        ] {
            draw_circle(x, 660.0 + r * 0.55, r, GRASS_DARK);
        }
        for (x, size) in [
            (60.0, 120.0),
            (140.0, 90.0),
            (1300.0, 100.0),
            (1370.0, 130.0),
        ] {
            draw_tree(vec2(x, 560.0 + (130.0 - size) * 0.4), size, 1.0, 0.0);
        }
        draw_rectangle(0.0, 690.0, W, H - 690.0, GRASS);

        // The robot hops along the bottom, picking up the stars as it goes;
        // each trip across it's the next player's robot.
        let lap_time = 16.0;
        let lap = (t / lap_time) as usize;
        let x = (t % lap_time) / lap_time * (W + 160.0) - 80.0;
        let ground = 745.0;
        for i in 0..9 {
            let sx = 150.0 + i as f32 * 140.0;
            if sx > x {
                let spin = (t * 1.5 + i as f32).sin() * 0.2;
                draw_star(vec2(sx, ground - 10.0), 18.0, spin, STAR);
            }
        }
        let hop = (t * PI / STEP_TIME).sin().abs() * 16.0;
        draw_robot(
            vec2(x, ground - 28.0 - hop),
            80.0,
            Face::Happy,
            vec2(1.0, 0.0),
            t,
            hop,
            PAINTS[lap % PAINTS.len()],
        );
    }

    /// The level map: a winding path, finished levels starred, and this
    /// player's robot waiting on the next one.
    fn draw_map(&self) {
        let n = self.levels.len();
        let next = self.next_unfinished();
        // The path joining the levels.
        for i in 1..n {
            let (a, b) = (self.map_node(i - 1), self.map_node(i));
            draw_line(a.x, a.y, b.x, b.y, 16.0, Color::new(1.0, 1.0, 1.0, 0.55));
        }
        for i in 0..n {
            let c = self.map_node(i);
            let kind = Kind::of(&self.levels[i]);
            let colour = kind.colour();
            let mut r = 34.0;
            if (self.mouse - c).length() < 40.0 {
                r += 5.0;
            }
            if next == Some(i) {
                r += (self.time * 4.0).sin().abs() * 4.0;
            }
            draw_circle(c.x, c.y + 4.0, r, darker(colour, 0.7));
            if self.solved[i] {
                draw_circle(c.x, c.y, r, colour);
                draw_level_label(&self.levels[i], c, 30.0, WHITE);
                draw_star(c + vec2(24.0, -24.0), 17.0, 0.0, darker(STAR, 0.8));
                draw_star(c + vec2(24.0, -26.0), 15.0, 0.0, STAR);
            } else {
                draw_circle(c.x, c.y, r, colour);
                draw_circle(c.x, c.y, r - 6.0, WHITE);
                draw_level_label(&self.levels[i], c, 30.0, colour);
            }
        }
        // This player's robot sits on the next level to play.
        if let Some(i) = next {
            let c = self.map_node(i) - vec2(0.0, 58.0);
            let hop = (self.time * 3.0).sin().abs() * 6.0;
            draw_robot(
                c - vec2(0.0, hop),
                70.0,
                Face::Happy,
                Vec2::ZERO,
                self.time,
                hop,
                self.paint(),
            );
        }
        // Back to the players: a left arrow, and whose map this is.
        let b = BACK_BUTTON;
        rounded_rect(
            b.x - 30.0,
            b.y - 26.0,
            60.0,
            60.0,
            14.0,
            darker(dir_color(Dir::Left), 0.7),
        );
        rounded_rect(
            b.x - 30.0,
            b.y - 30.0,
            60.0,
            60.0,
            14.0,
            dir_color(Dir::Left),
        );
        draw_arrow(b, 48.0, Dir::Left, WHITE);
        draw_robot(
            b + vec2(70.0, 2.0),
            64.0,
            Face::Calm,
            Vec2::ZERO,
            self.time,
            0.0,
            self.paint(),
        );
    }

    /// A big, bright pointer that's easy to spot, with a ripple on each click.
    fn draw_cursor(&self) {
        for &(at, t) in &self.clicks {
            let age = self.time - t;
            if age < 0.4 {
                let k = age / 0.4;
                draw_circle_lines(at.x, at.y, 12.0 + k * 30.0, 5.0 * (1.0 - k), STAR);
            }
        }
        let p = self.mouse;
        // A bright yellow ring around the tip, gently pulsing.
        let pulse = (self.time * 3.0).sin().abs();
        draw_circle(p.x, p.y, 22.0, Color::new(1.0, 0.9, 0.2, 0.35));
        draw_circle_lines(
            p.x,
            p.y,
            22.0 + pulse * 3.0,
            4.0,
            Color::new(1.0, 0.85, 0.1, 0.95),
        );
        let s = 1.8;
        let pt = |x: f32, y: f32| p + vec2(x, y) * s;
        let head = [pt(0.0, 0.0), pt(0.0, 30.0), pt(21.0, 21.0)];
        let tail = [
            pt(8.0, 21.0),
            pt(14.0, 19.0),
            pt(20.0, 33.0),
            pt(14.0, 36.0),
        ];
        let ink = Color::new(0.1, 0.12, 0.2, 1.0);
        for shape in [&head[..], &tail[..]] {
            for k in 0..shape.len() {
                let (a, b) = (shape[k], shape[(k + 1) % shape.len()]);
                draw_line(a.x, a.y, b.x, b.y, 7.0, ink);
                draw_circle(a.x, a.y, 3.5, ink);
            }
        }
        draw_triangle(head[0], head[1], head[2], WHITE);
        draw_quad(tail[0], tail[1], tail[2], tail[3], WHITE);
    }

    fn draw_board(&self) {
        let l = self.level();
        let cell = self.cell_size();
        let o = self.grid_origin();
        let (gw, gh) = (l.w as f32 * cell, l.h as f32 * cell);
        rounded_rect(o.x - 10.0, o.y - 6.0, gw + 20.0, gh + 20.0, 18.0, SHADOW);
        rounded_rect(
            o.x - 10.0,
            o.y - 10.0,
            gw + 20.0,
            gh + 20.0,
            18.0,
            darker(GRASS_DARK, 0.85),
        );
        for y in 0..l.h {
            for x in 0..l.w {
                let colour = if (x + y) % 2 == 0 { GRASS } else { GRASS_DARK };
                draw_rectangle(
                    o.x + x as f32 * cell,
                    o.y + y as f32 * cell,
                    cell,
                    cell,
                    colour,
                );
            }
        }
        // Start pad, so it's clear where the robot goes back to.
        let (start, _) = self.cell_centre(l.start);
        draw_circle_lines(
            start.x,
            start.y,
            cell * 0.38,
            4.0,
            Color::new(1.0, 1.0, 1.0, 0.6),
        );

        let chopping = match (self.mode, self.event) {
            (Mode::Chopping, Some(Event::Chopped(p))) => Some(p),
            _ => None,
        };
        // The tree being looked at is still standing until the chop starts.
        let checked_tree = match (self.mode, self.event) {
            (Mode::Checking, Some(Event::Checked { at, tree: true })) => Some(at),
            _ => None,
        };
        for y in 0..l.h {
            for x in 0..l.w {
                let c = self.cell_centre((x, y)).0;
                match self.world.tile((x, y)) {
                    Some(Tile::Tree) => draw_tree(c, cell, 1.0, 0.0),
                    Some(Tile::Rock) => draw_rock(c, cell),
                    Some(Tile::Stump) if chopping == Some((x, y)) => {
                        // Wobble with each axe hit, then topple over.
                        let t = self.timer / CHOP_TIME;
                        draw_stump(c, cell);
                        if t < 0.6 {
                            draw_tree(c, cell, 1.0, (t * 30.0).sin() * 0.08);
                        } else {
                            let fall = (t - 0.6) / 0.4;
                            let side = if self.facing == Dir::Left { -1.0 } else { 1.0 };
                            draw_tree(c, cell, 1.0 - fall, side * fall * 1.2);
                        }
                    }
                    Some(Tile::Stump) if checked_tree == Some((x, y)) => {
                        draw_tree(c, cell, 1.0, 0.0)
                    }
                    Some(Tile::Stump) => draw_stump(c, cell),
                    Some(Tile::Mystery) => draw_mystery(c, cell, 1.0, self.time),
                    _ => {}
                }
            }
        }
        // The clouds shrink away as Play reveals what was underneath.
        if self.mode == Mode::Reveal {
            let t = (self.timer / (REVEAL_TIME * 0.6)).min(1.0);
            for &p in &l.mysteries {
                draw_mystery(self.cell_centre(p).0, cell, 1.0 - t, self.time);
            }
        }
        // While checking, a ring shows where the robot is looking. For an
        // "if", it turns green (yes) or grey (no) halfway through.
        let looking = match (self.mode, self.event) {
            (Mode::Checking, Some(Event::Checked { at, .. })) => Some((at, WHITE)),
            (Mode::Checking, Some(Event::Tested { at, yes })) => {
                let colour = match (self.timer > CHECK_TIME * 0.5, yes) {
                    (false, _) => WHITE,
                    (true, true) => YES_GREEN,
                    (true, false) => NO_GREY,
                };
                Some((at, colour))
            }
            _ => None,
        };
        if let Some((at, colour)) = looking {
            let c = self.cell_centre(at).0;
            let pulse = (self.time * 8.0).sin() * 0.5 + 0.5;
            draw_circle_lines(c.x, c.y, cell * (0.42 + pulse * 0.04), 5.0, colour);
        }

        // Stars bob gently; leftover stars wiggle when the robot is sad.
        // A star being driven onto stays until the robot arrives.
        let arriving = match (self.mode, self.event) {
            (Mode::Moving, Some(Event::Moved { star: true, .. })) => Some(self.world.pos),
            _ => None,
        };
        for (i, &s) in self.world.stars.iter().chain(&arriving).enumerate() {
            let (c, _) = self.cell_centre(s);
            let bob = (self.time * 3.0 + i as f32).sin() * cell * 0.04;
            let wiggle = if self.mode == Mode::Sad {
                (self.time * 20.0).sin() * 0.3
            } else {
                0.0
            };
            draw_star(c + vec2(0.0, bob), cell * 0.33, wiggle, darker(STAR, 0.8));
            draw_star(c + vec2(0.0, bob - 2.0), cell * 0.3, wiggle, STAR);
        }
        self.draw_robot();
    }

    fn draw_robot(&self) {
        let cell = self.cell_size();
        let pos = pos_vec(self.world.pos);
        let look = dir_vec(self.facing);
        let (at, lift, face) = match (self.mode, self.event) {
            (Mode::Moving, Some(Event::Moved { from, .. })) => {
                let t = (self.timer / STEP_TIME).min(1.0);
                let eased = t * t * (3.0 - 2.0 * t);
                let hop = (t * PI).sin() * cell * 0.15;
                (pos_vec(from).lerp(pos, eased), hop, Face::Happy)
            }
            (Mode::Chopping, _) => (pos, 0.0, Face::Happy),
            (Mode::Checking, _) => (pos, 0.0, Face::Calm),
            (Mode::Bump, _) => {
                let t = self.timer / BUMP_TIME;
                let lunge = (t * 3.0).min(1.0) * PI;
                let shake = (self.time * 50.0).sin() * 0.04 * (1.0 - t);
                (
                    pos + look * 0.3 * lunge.sin() + vec2(shake, 0.0),
                    0.0,
                    Face::Sad,
                )
            }
            (Mode::Sad, _) => (pos, 0.0, Face::Sad),
            (Mode::Win, _) => {
                let hop = (self.timer * 8.0).sin().abs() * cell * 0.25;
                (pos, hop, Face::Happy)
            }
            _ => (pos, 0.0, Face::Calm),
        };
        let c = self.cell_centre_f(at) - vec2(0.0, lift);
        draw_robot(c, cell, face, look, self.time, lift, self.paint());
        if self.mode == Mode::Checking {
            draw_thought(c, cell);
        }

        // Swing the axe while chopping (or while trying to chop something
        // that isn't a tree).
        let failed_chop = self.mode == Mode::Bump
            && self
                .failed_step
                .and_then(|(s, i)| self.strips[s].get(i))
                .is_some_and(|it| {
                    matches!(
                        it,
                        Item::Block(Block {
                            cmd: Cmd::Chop(_),
                            ..
                        })
                    )
                });
        if self.mode == Mode::Chopping || failed_chop {
            let swing = (self.time * 25.0).sin() * 0.5;
            draw_axe(
                c + look * cell * 0.45,
                cell * 0.45,
                self.facing,
                swing,
                WOOD,
            );
        }
    }

    fn block_colour(cmd: Cmd) -> Color {
        match cmd {
            Cmd::Move(dir) => dir_color(dir),
            Cmd::Chop(_) => AXE_BROWN,
            Cmd::IfTreeChop(_) => CHECK_SLATE,
        }
    }

    fn item_colour(item: Item) -> Color {
        match item {
            Item::Block(b) => Self::block_colour(b.cmd),
            Item::LoopStart { .. } | Item::LoopEnd => REPEAT_TEAL,
            Item::IfStart { .. } | Item::IfEnd => IF_LIME,
            Item::Call(_) => CALL_INK,
            Item::Gap => SLOT_EMPTY,
        }
    }

    fn draw_item(r: Rect, item: Item, colour: Color, badge: Option<u8>) {
        if let Item::LoopStart { .. } | Item::LoopEnd | Item::IfEnd = item {
            let close = !matches!(item, Item::LoopStart { .. });
            Self::draw_loop_bracket(r, close, colour, badge);
            return;
        }
        rounded_rect(r.x, r.y + 4.0, r.w, r.h, 12.0, darker(colour, 0.7));
        rounded_rect(r.x, r.y, r.w, r.h, 12.0, colour);
        match item {
            Item::Block(b) => match b.cmd {
                Cmd::Move(dir) => draw_arrow(r.center(), r.w * 0.8, dir, WHITE),
                Cmd::Chop(dir) => draw_chop_icon(r.center(), r.w, dir),
                Cmd::IfTreeChop(dir) => draw_check_icon(r.center(), r.w, dir),
            },
            Item::LoopStart { .. } | Item::LoopEnd | Item::IfEnd | Item::Gap => {}
            Item::IfStart { sense, dir } => draw_if_icon(r.center(), r.w, sense, dir),
            Item::Call(f) => draw_shape(r.center(), r.w * 0.28, f),
        }
        if let Some(n) = badge {
            let c = vec2(r.x + r.w - 4.0, r.y + 4.0);
            draw_circle(c.x, c.y, 15.0, INK);
            draw_circle(c.x, c.y, 13.0, WHITE);
            draw_label(&format!("{n}"), c, 24.0, INK);
        }
    }

    fn draw_program(&self) {
        for strip in 0..self.strips.len() {
            self.draw_strip(strip);
        }
        if let Some((strip, i)) = self.hovered_item() {
            let c = self.eraser_at(strip, i);
            let near = (self.mouse - c).length() < ERASER_R + 4.0;
            let r = if near { ERASER_R + 3.0 } else { ERASER_R };
            draw_circle(c.x, c.y, r + 3.0, WHITE);
            draw_circle(c.x, c.y, r, STOP_RED);
            let k = r * 0.45;
            draw_line(c.x - k, c.y - k, c.x + k, c.y + k, 4.0, WHITE);
            draw_line(c.x - k, c.y + k, c.x + k, c.y - k, 4.0, WHITE);
        }
    }

    /// A loop bracket in the gap between tiles: `[` with the loop count on
    /// top, or `]`.
    fn draw_loop_bracket(r: Rect, close: bool, colour: Color, badge: Option<u8>) {
        let t = 7.0;
        let serif = 11.0;
        let (top, bottom) = (r.y - 4.0, r.bottom() + 4.0);
        let x = r.center().x - t / 2.0;
        draw_rectangle(x, top, t, bottom - top, colour);
        let sx = if close { x - serif } else { x };
        for y in [top, bottom - t] {
            draw_rectangle(sx, y, serif + t, t, colour);
        }
        if let Some(n) = badge {
            let c = vec2(r.center().x, top - 2.0);
            draw_circle(c.x, c.y, 15.0, colour);
            draw_circle(c.x, c.y, 12.0, WHITE);
            draw_label(&format!("{n}"), c, 24.0, INK);
        }
    }

    fn draw_strip(&self, strip: usize) {
        let items = &self.strips[strip];
        let layout = self.layout(strip);
        let p = layout.panel;
        // The strip being edited has a yellow outline (when there's a choice).
        if self.labelled() && strip == self.editing && self.mode == Mode::Edit {
            rounded_rect(p.x - 5.0, p.y - 5.0, p.w + 10.0, p.h + 10.0, 25.0, STAR);
        }
        rounded_rect(p.x, p.y, p.w, p.h, 20.0, PANEL);
        if self.labelled() {
            let r = self.label_rect(strip);
            if strip == 0 {
                draw_circle(r.center().x, r.center().y, r.w * 0.42, PLAY_GREEN);
                draw_play_icon(r.center() + vec2(r.w * 0.04, 0.0), r.w * 0.36, WHITE);
            } else {
                draw_shape(r.center(), r.w * 0.36, strip);
            }
        }

        let running = matches!(
            self.mode,
            Mode::Moving | Mode::Checking | Mode::Chopping | Mode::Bump
        );
        let current = self.trace.get(self.pc).filter(|_| running);
        // Where this strip has got to: the running slot here, or for the main
        // program, the call that's running a function.
        let at = current.and_then(|e| match (strip, e.call) {
            (0, Some(call)) => Some(call),
            _ => (e.at.0 == strip).then_some(e.at.1),
        });

        // A band behind each loop (teal) and each "if" (green) shows which
        // blocks are inside it. An open one also takes in the next empty
        // tile, where new blocks will go.
        for loops in [true, false] {
            for (i, item) in items.iter().enumerate() {
                let (is_start, band) = match item {
                    Item::LoopStart { .. } => (loops, LOOP_BAND),
                    Item::IfStart { .. } => (!loops, IF_BAND),
                    _ => (false, LOOP_BAND),
                };
                if !is_start {
                    continue;
                }
                let end = pair_ends(items, i).and_then(|(_, e)| e);
                let mut rects: Vec<Rect> =
                    layout.items[i..=end.unwrap_or(items.len() - 1)].to_vec();
                if end.is_none() {
                    rects.extend(layout.empty.first());
                }
                draw_band(&rects, band);
            }
        }
        // The next empty tile pulses inside whatever is still open.
        let pulse_with = match (open_if(items), open_loop(items)) {
            (Some(_), _) => Some((IF_BAND, IF_LIME)),
            (None, Some(_)) => Some((LOOP_BAND, REPEAT_TEAL)),
            (None, None) => None,
        };
        for (k, r) in layout.empty.iter().enumerate() {
            let colour = match (k, pulse_with) {
                (0, Some((band, strong))) => {
                    let pulse = (self.time * 4.0).sin() * 0.5 + 0.5;
                    mix(band, strong, 0.3 + pulse * 0.3)
                }
                _ => SLOT_EMPTY,
            };
            rounded_rect(r.x, r.y, r.w, r.h, 12.0, colour);
        }

        for (i, &item) in items.iter().enumerate() {
            let mut r = layout.items[i];
            // How many more times this loop will run, if it's running now.
            let looping = current
                .and_then(|e| match strip {
                    0 => e.main_loop,
                    _ if e.at.0 == strip => e.fn_loop,
                    _ => None,
                })
                .filter(|&(start, _)| start == i)
                .map(|(_, left)| left);
            let active = at == Some(i);
            let failed = self.failed_step == Some((strip, i));
            let done = at.is_some_and(|a| a > i) && looping.is_none();
            if active {
                r.y -= 6.0;
            }
            if failed {
                r.x += (self.time * 40.0).sin() * 3.0;
            }
            if active || failed {
                let ring = if failed { STOP_RED } else { WHITE };
                rounded_rect(r.x - 5.0, r.y - 5.0, r.w + 10.0, r.h + 10.0, 16.0, ring);
            }
            let mut colour = Self::item_colour(item);
            if done {
                colour = mix(colour, PANEL, 0.55);
            }
            // While a repeated block or loop runs, its number counts down.
            let running_block = active && current.is_some_and(|e| e.at == (strip, i));
            let badge = match item {
                Item::Block(b) if running_block => current.map(|e| e.left).filter(|_| b.times > 1),
                Item::Block(b) => (b.times > 1).then_some(b.times),
                Item::LoopStart { times } => looping.or(Some(times)),
                Item::LoopEnd | Item::IfStart { .. } | Item::IfEnd | Item::Call(_) | Item::Gap => {
                    None
                }
            };
            if item == Item::Gap {
                // An opened-up slot pulses, waiting for the next block.
                let pulse = (self.time * 5.0).sin() * 0.5 + 0.5;
                let g = 3.0 + pulse * 3.0;
                rounded_rect(
                    r.x - g,
                    r.y - g,
                    r.w + 2.0 * g,
                    r.h + 2.0 * g,
                    14.0 + g,
                    STAR,
                );
                rounded_rect(r.x, r.y, r.w, r.h, 12.0, SLOT_EMPTY);
                continue;
            }
            Self::draw_item(r, item, colour, badge);
            // An "if" shows its answer once the robot has looked: yes or no.
            if running_block
                && let Some(Step::Test { yes, .. }) = current.map(|e| e.step)
                && self.timer > CHECK_TIME * 0.5
            {
                draw_answer(vec2(r.right() - 4.0, r.y + 4.0), yes);
            }
        }
    }

    fn draw_buttons(&self) {
        rounded_rect(1110.0, 70.0, 292.0, self.panel_bottom() - 70.0, 30.0, PANEL);
        for (button, c, half) in self.buttons() {
            if !self.visible(button) {
                continue;
            }
            let squash = match self.pressed {
                Some((b, t)) if b == button && self.time - t < 0.12 => 0.9,
                _ => 1.0,
            };
            let half = half * squash;
            let enabled = self.enabled(button);
            let (colour, round) = match button {
                Button::Arrow(dir) => (dir_color(dir), 18.0),
                Button::Axe => (AXE_BROWN, 18.0),
                Button::Check => (CHECK_SLATE, 18.0),
                Button::Repeat | Button::Loop => (REPEAT_TEAL, 18.0),
                Button::If(_) => (IF_LIME, 18.0),
                Button::Call(_) => (CALL_INK, 18.0),
                Button::Play if self.running() => (STOP_RED, half),
                Button::Play => (PLAY_GREEN, half),
                Button::Undo => (UNDO_YELLOW, 18.0),
                Button::Clear => (STOP_RED, 18.0),
                Button::Fullscreen => (PANEL, 10.0),
                Button::Map => (dir_color(Dir::Up), 14.0),
            };
            let colour = if enabled { colour } else { DISABLED };
            let size = half * 2.0;
            // An armed tool glows to say "now pick which way", and an open
            // loop glows until it's closed.
            // An open loop or "if" makes its button glow until it's closed.
            let open_loop = match button {
                Button::Loop => open_loop(self.prog()).is_some(),
                Button::If(_) => open_if(self.prog()).is_some(),
                _ => false,
            };
            if self.armed == Some(button) || open_loop {
                let g = 6.0 + ((self.time * 6.0).sin() * 0.5 + 0.5) * 4.0;
                rounded_rect(
                    c.x - half - g,
                    c.y - half - g,
                    size + 2.0 * g,
                    size + 2.0 * g,
                    round + g,
                    STAR,
                );
            }
            if button != Button::Fullscreen && button != Button::Map {
                rounded_rect(
                    c.x - half,
                    c.y - half + 6.0,
                    size,
                    size,
                    round,
                    darker(colour, 0.7),
                );
            }
            rounded_rect(c.x - half, c.y - half, size, size, round, colour);
            match button {
                Button::Arrow(dir) => match self.armed {
                    Some(Button::Axe) => draw_chop_icon(c, size, dir),
                    Some(Button::Check) => draw_check_icon(c, size, dir),
                    Some(Button::If(sense)) => draw_if_icon(c, size, sense, dir),
                    _ => draw_arrow(c, size * 0.8, dir, WHITE),
                },
                Button::Axe => draw_axe_badge(c, size * 0.75),
                Button::Check => {
                    draw_question(c - vec2(size * 0.14, size * 0.04), 64.0, WHITE);
                    draw_axe_badge(c + vec2(0.2, 0.18) * size, size * 0.42);
                }
                Button::Repeat => {
                    draw_loop_icon(c, half, WHITE);
                    // Show what the next tap will make the last block (or loop).
                    draw_label(&format!("{}", self.next_times()), c, 28.0, WHITE);
                }
                Button::Call(f) => draw_shape(c, half * 0.55, f),
                // While an "if" is open, the button shows how to close it.
                Button::If(_) if open_loop => {
                    let r = Rect::new(c.x - half, c.y - half, size, size);
                    draw_bracket(r, c.x + size * 0.1, false, WHITE);
                }
                Button::If(sense) => draw_if_icon(c, size, sense, Dir::Right),
                Button::Loop => {
                    let r = Rect::new(c.x - half, c.y - half, size, size);
                    // While a loop is open, the button shows how to close it.
                    if !open_loop {
                        draw_bracket(r, c.x - size * 0.22, true, WHITE);
                    }
                    draw_bracket(r, c.x + size * 0.22, false, WHITE);
                    if !open_loop {
                        draw_loop_icon(c, half * 0.6, WHITE);
                    }
                }
                Button::Play if self.running() => {
                    let s = half * 0.38;
                    draw_rectangle(c.x - s, c.y - s, s * 2.0, s * 2.0, WHITE)
                }
                Button::Play => {
                    if enabled {
                        let pulse = (self.time * 4.0).sin() * 0.5 + 0.5;
                        draw_circle_lines(
                            c.x,
                            c.y,
                            half + 6.0 + pulse * 8.0,
                            4.0,
                            Color::new(0.15, 0.7, 0.25, 0.6 * (1.0 - pulse)),
                        );
                    }
                    draw_play_icon(c, half, WHITE)
                }
                Button::Undo => draw_undo_icon(c, half),
                Button::Clear => draw_trash_icon(c, half),
                Button::Fullscreen => draw_fullscreen_icon(c, half, INK),
                Button::Map => draw_map_icon(c, half),
            }
        }
    }

    fn draw_next_prompt(&self) {
        let c = self.board().center();
        let pulse = 1.0 + (self.time * 5.0).sin() * 0.06;
        let r = 70.0 * pulse;
        draw_circle(c.x, c.y + 6.0, r, darker(PLAY_GREEN, 0.7));
        draw_circle(c.x, c.y, r, PLAY_GREEN);
        draw_arrow(c, r * 1.4, Dir::Right, WHITE);
    }
}

/// The size of the eraser that appears on a program block under the pointer.
const ERASER_R: f32 = 14.0;

/// Levels per row on the level map.
const MAP_COLS: usize = 10;
/// The map's back-to-players button.
const BACK_BUTTON: Vec2 = vec2(52.0, 44.0);

/// Player `slot`'s card on the players screen.
fn player_card(slot: usize) -> Rect {
    let w = 340.0;
    let x = W / 2.0 + (slot as f32 - 1.0) * 400.0 - w / 2.0;
    Rect::new(x, 170.0, w, 460.0)
}

/// The erase button in the corner of a player card.
fn erase_button(slot: usize) -> Vec2 {
    let r = player_card(slot);
    vec2(r.right() - 44.0, r.bottom() - 44.0)
}

/// The game's name in big, bold, bouncing letters, each its own colour.
fn draw_title(centre: Vec2, time: f32) {
    const TITLE: &str = "ROBOT STEPS";
    const SIZE: f32 = 110.0;
    let colours = [
        STOP_RED,
        UNDO_YELLOW,
        PLAY_GREEN,
        REPEAT_TEAL,
        FN_COLOURS[0],
        WHITE,
        IF_LIME,
        FN_COLOURS[1],
        STOP_RED,
        UNDO_YELLOW,
        PLAY_GREEN,
    ];
    let scale = (screen_width() / W).min(screen_height() / H) * screen_dpi_scale();
    let width =
        |text: &str| measure_text(text, None, (SIZE * scale).round() as u16, 1.0 / scale).width;
    let spacing = 6.0;
    let total = width(TITLE) + spacing * (TITLE.len() - 1) as f32;
    let mut x = centre.x - total / 2.0;
    for (i, (ch, colour)) in TITLE.chars().zip(colours).enumerate() {
        let text = ch.to_string();
        let w = width(&text);
        let bob = (time * 3.0 - i as f32 * 0.5).sin() * 6.0;
        let c = vec2(x + w / 2.0, centre.y + bob);
        if ch != ' ' {
            draw_label(&text, c + vec2(0.0, 8.0), SIZE, SHADOW);
            // A thick outline, then the letter drawn a little bold.
            for k in 0..16 {
                let a = k as f32 * PI / 8.0;
                for r in [3.5, 7.0] {
                    draw_label(&text, c + vec2(a.cos(), a.sin()) * r, SIZE, INK);
                }
            }
            for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5)] {
                draw_label(&text, c + vec2(dx, dy), SIZE, colour);
            }
        }
        x += w + spacing;
    }
    // Stars twinkling around the title.
    let half = total / 2.0;
    for (i, (dx, dy, r)) in [
        (-half - 50.0, -30.0, 22.0),
        (half + 50.0, -24.0, 24.0),
        (-half - 22.0, 34.0, 13.0),
        (half + 28.0, 38.0, 14.0),
    ]
    .into_iter()
    .enumerate()
    {
        let twinkle = 0.75 + (time * 4.0 + i as f32 * 1.7).sin() * 0.25;
        let c = centre + vec2(dx, dy);
        draw_star(c, r * twinkle, time * 0.8 + i as f32, STAR);
    }
}

/// A little map: three linked dots.
fn draw_map_icon(c: Vec2, r: f32) {
    let pts = [
        c + vec2(-0.5, 0.35) * r,
        c + vec2(0.0, -0.3) * r,
        c + vec2(0.5, 0.3) * r,
    ];
    draw_line(pts[0].x, pts[0].y, pts[1].x, pts[1].y, r * 0.16, WHITE);
    draw_line(pts[1].x, pts[1].y, pts[2].x, pts[2].y, r * 0.16, WHITE);
    for p in pts {
        draw_circle(p.x, p.y, r * 0.2, WHITE);
    }
    draw_star(pts[2] + vec2(0.0, -r * 0.05), r * 0.22, 0.0, STAR);
}

/// Where each part of a program strip goes on screen.
struct StripLayout {
    /// One rect per item: a tile for each block or call, and a narrow gap
    /// for each loop bracket.
    items: Vec<Rect>,
    /// The empty tiles after the program.
    empty: Vec<Rect>,
    /// The panel behind the strip.
    panel: Rect,
}

/// Shade behind `rects` (the inside of a loop), one row at a time.
fn draw_band(rects: &[Rect], colour: Color) {
    let mut rows: Vec<(f32, f32, f32)> = Vec::new();
    for r in rects {
        match rows.iter_mut().find(|(y, _, _)| *y == r.y) {
            Some(row) => {
                row.1 = row.1.min(r.x);
                row.2 = row.2.max(r.right());
            }
            None => rows.push((r.y, r.x, r.right())),
        }
    }
    for (y, left, right) in rows {
        rounded_rect(
            left - 6.0,
            y - 6.0,
            right - left + 12.0,
            SLOT + 12.0,
            16.0,
            colour,
        );
    }
}

/// The start of a loop in `items` that hasn't been closed yet.
fn open_loop(items: &[Item]) -> Option<usize> {
    let start = items
        .iter()
        .rposition(|i| matches!(i, Item::LoopStart { .. }))?;
    let closed = items[start..].contains(&Item::LoopEnd);
    (!closed).then_some(start)
}

/// The start of an "if" in `items` that hasn't been closed yet.
fn open_if(items: &[Item]) -> Option<usize> {
    let start = items
        .iter()
        .rposition(|i| matches!(i, Item::IfStart { .. }))?;
    let closed = items[start..].contains(&Item::IfEnd);
    (!closed).then_some(start)
}

/// The two ends of the loop or "if" that slot `i` is one end of.
fn pair_ends(items: &[Item], i: usize) -> Option<(usize, Option<usize>)> {
    let (is_start, close): (fn(&Item) -> bool, Item) = match items[i] {
        Item::LoopStart { .. } | Item::LoopEnd => {
            (|it| matches!(it, Item::LoopStart { .. }), Item::LoopEnd)
        }
        Item::IfStart { .. } | Item::IfEnd => {
            (|it| matches!(it, Item::IfStart { .. }), Item::IfEnd)
        }
        Item::Block(_) | Item::Call(_) | Item::Gap => return None,
    };
    let start = if is_start(&items[i]) {
        i
    } else {
        items[..i].iter().rposition(is_start)?
    };
    let end = (start + 1..items.len()).find(|&j| items[j] == close);
    Some((start, end))
}

/// What an "if" looks for: a little tree or star.
fn draw_sense(c: Vec2, size: f32, sense: Sense) {
    match sense {
        Sense::Tree => draw_tree(c - vec2(0.0, size * 0.12), size, 1.0, 0.0),
        Sense::Star => draw_star(c, size * 0.42, 0.0, STAR),
    }
}

/// "If there's a tree (or star) that way": the thing, an arrow, and a "?".
fn draw_if_icon(c: Vec2, size: f32, sense: Sense, dir: Dir) {
    draw_sense(c - vec2(size * 0.16, size * 0.02), size * 0.55, sense);
    draw_arrow(c + vec2(size * 0.24, size * 0.16), size * 0.4, dir, WHITE);
    draw_question(c + vec2(size * 0.24, -size * 0.22), size * 0.36, WHITE);
}

/// The answer to an "if": a green tick, or a grey cross.
fn draw_answer(c: Vec2, yes: bool) {
    draw_circle(c.x, c.y, 15.0, WHITE);
    draw_circle(c.x, c.y, 13.0, if yes { YES_GREEN } else { NO_GREY });
    if yes {
        draw_line(c.x - 6.0, c.y, c.x - 1.5, c.y + 5.0, 3.5, WHITE);
        draw_line(c.x - 1.5, c.y + 5.0, c.x + 6.5, c.y - 5.0, 3.5, WHITE);
    } else {
        draw_line(c.x - 5.0, c.y - 5.0, c.x + 5.0, c.y + 5.0, 3.5, WHITE);
        draw_line(c.x - 5.0, c.y + 5.0, c.x + 5.0, c.y - 5.0, 3.5, WHITE);
    }
}

/// "Chop that way": an arrow with a little axe tucked in the corner.
fn draw_chop_icon(c: Vec2, size: f32, dir: Dir) {
    draw_arrow(c + vec2(0.08, 0.08) * size, size * 0.62, dir, WHITE);
    draw_axe_badge(c - vec2(0.24, 0.24) * size, size * 0.42);
}

/// "If there's a tree that way, chop it": an arrow with a question mark.
fn draw_check_icon(c: Vec2, size: f32, dir: Dir) {
    draw_arrow(c + vec2(0.08, 0.08) * size, size * 0.62, dir, WHITE);
    draw_question(c - vec2(0.22, 0.2) * size, size * 0.62, WHITE);
}

/// The axe as drawn on the axe button, for use as a small badge.
fn draw_axe_badge(c: Vec2, size: f32) {
    draw_axe(c, size, Dir::Left, -0.5, WHITE);
}

/// A camera that maps the fixed W x H canvas onto the window, letterboxed.
fn fit_camera() -> Camera2D {
    let (sw, sh) = (screen_width(), screen_height());
    let scale = (sw / W).min(sh / H);
    Camera2D {
        target: vec2(W / 2.0, H / 2.0),
        zoom: vec2(2.0 * scale / sw, 2.0 * scale / sh),
        ..Default::default()
    }
}

/// Start full screen unless run with `--window`. A web page can only go full
/// screen after a tap, so there the game starts filling the page.
fn start_fullscreen() -> bool {
    !cfg!(target_arch = "wasm32") && !std::env::args().any(|a| a == "--window")
}

/// WSL hands OpenGL programs a software renderer by default, which is far
/// too slow for a full-screen game. Mesa's d3d12 driver uses the real GPU.
fn prefer_gpu_on_wsl() {
    let on_wsl = std::env::var_os("WSL_DISTRO_NAME").is_some();
    if on_wsl && std::env::var_os("GALLIUM_DRIVER").is_none() {
        // SAFETY: runs before the window opens, while there's only one thread.
        unsafe { std::env::set_var("GALLIUM_DRIVER", "d3d12") };
    }
}

fn window_conf() -> Conf {
    prefer_gpu_on_wsl();
    Conf {
        window_title: "Robot Steps".to_owned(),
        window_width: 1422,
        window_height: 800,
        window_resizable: true,
        fullscreen: start_fullscreen(),
        sample_count: 4,
        // Draw at the screen's real resolution when Windows display scaling is on.
        high_dpi: true,
        platform: miniquad::conf::Platform {
            // Wait for the screen's refresh (vsync) for steady animation.
            swap_interval: Some(1),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    rand::srand(miniquad::date::now() as u64);
    let mut game = Game::new(start_fullscreen());
    // The game draws its own big pointer instead.
    show_mouse(false);
    loop {
        let camera = fit_camera();
        set_camera(&camera);
        game.mouse = camera.screen_to_world(mouse_position().into());
        if is_mouse_button_pressed(MouseButton::Left) {
            game.handle_click(game.mouse);
        }
        game.handle_keys();
        game.update(get_frame_time().min(0.05));
        game.draw();
        next_frame().await;
    }
}
