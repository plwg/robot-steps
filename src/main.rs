//! Robot Steps: a first coding game. Tap arrows to build a program, press play,
//! and watch the robot follow it to collect the stars. No reading required.

// On Windows, don't open a console window alongside the game.
#![windows_subsystem = "windows"]

mod level;

use std::f32::consts::PI;

use level::{Block, Cmd, Dir, Event, Exec, Item, Level, MAX_TIMES, Pos, Tile, World};
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
    if scale > 0.3 {
        draw_question(c, 64.0 * s / MAX_CELL, CHECK_SLATE);
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

fn draw_robot(c: Vec2, cell: f32, face: Face, look: Vec2, time: f32, lift: f32) {
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

    let body = Color::new(0.55, 0.78, 0.98, 1.0);
    let trim = Color::new(0.3, 0.5, 0.8, 1.0);
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

fn draw_play_icon(c: Vec2, r: f32) {
    draw_triangle(
        c + vec2(-r * 0.35, -r * 0.45),
        c + vec2(-r * 0.35, r * 0.45),
        c + vec2(r * 0.5, 0.0),
        WHITE,
    );
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
    Play,
    Undo,
    Clear,
    Fullscreen,
}

/// Each fixed button with its centre and half-size. The tool buttons (axe,
/// check, repeat) are laid out per level, as only some levels have them.
const BUTTONS: [(Button, Vec2, f32); 8] = [
    (Button::Arrow(Dir::Up), vec2(1256.0, 130.0), 42.0),
    (Button::Arrow(Dir::Left), vec2(1166.0, 215.0), 42.0),
    (Button::Arrow(Dir::Right), vec2(1346.0, 215.0), 42.0),
    (Button::Arrow(Dir::Down), vec2(1256.0, 300.0), 42.0),
    (Button::Play, vec2(1256.0, 572.0), 64.0),
    (Button::Undo, vec2(1186.0, 728.0), 40.0),
    (Button::Clear, vec2(1326.0, 728.0), 40.0),
    (Button::Fullscreen, vec2(1386.0, 36.0), 22.0),
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
    idx: usize,
    solved: Vec<bool>,
    program: Vec<Item>,
    /// The axe or check button was tapped; the next arrow makes that block.
    armed: Option<Button>,
    world: World,
    mode: Mode,
    timer: f32,
    /// The program unrolled into single actions, and which one is running.
    trace: Vec<Exec>,
    pc: usize,
    /// What the robot is doing right now, for animation.
    event: Option<Event>,
    facing: Dir,
    failed_step: Option<usize>,
    particles: Vec<Particle>,
    pressed: Option<(Button, f32)>,
    time: f32,
    fullscreen: bool,
}

impl Game {
    fn new(fullscreen: bool) -> Game {
        let levels = level::all_levels();
        let solved = vec![false; levels.len()];
        let world = levels[0].world();
        Game {
            levels,
            idx: 0,
            solved,
            program: Vec::new(),
            armed: None,
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
        }
    }

    fn level(&self) -> &Level {
        &self.levels[self.idx]
    }

    fn load(&mut self, idx: usize) {
        self.idx = idx;
        self.program.clear();
        self.particles.clear();
        self.armed = None;
        self.failed_step = None;
        self.reset_robot();
    }

    fn next_level(&mut self) {
        self.load((self.idx + 1) % self.levels.len());
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

    fn has_room(&self) -> bool {
        self.program.len() < self.level().slots
    }

    fn has_blocks(&self) -> bool {
        self.program.iter().any(|i| matches!(i, Item::Block(_)))
    }

    /// The start of a loop that hasn't been closed yet.
    fn open_loop(&self) -> Option<usize> {
        let start = self
            .program
            .iter()
            .rposition(|i| matches!(i, Item::LoopStart { .. }))?;
        let closed = self.program[start..].contains(&Item::LoopEnd);
        (!closed).then_some(start)
    }

    /// The two ends of the loop that slot `i` belongs to, if it's a loop end.
    fn loop_ends(&self, i: usize) -> Option<(usize, Option<usize>)> {
        let start = match self.program[i] {
            Item::LoopStart { .. } => i,
            Item::LoopEnd => self.program[..i]
                .iter()
                .rposition(|it| matches!(it, Item::LoopStart { .. }))?,
            Item::Block(_) => return None,
        };
        let end = (start + 1..self.program.len()).find(|&j| self.program[j] == Item::LoopEnd);
        Some((start, end))
    }

    /// How many times the last thing in the program runs: the last block, or
    /// the whole loop if the program ends with one.
    fn last_times(&mut self) -> Option<(&mut u8, u8)> {
        let last = self.program.len().checked_sub(1)?;
        let (start, _) = self.loop_ends(last).unwrap_or((last, None));
        match &mut self.program[start] {
            Item::Block(b) => Some((&mut b.times, 1)),
            // Looping once isn't a loop, so loops go from 2.
            Item::LoopStart { times } => Some((times, 2)),
            Item::LoopEnd => None,
        }
    }

    /// What tapping repeat would change the last count to.
    fn next_times(&self) -> u8 {
        let last = self.program.len().wrapping_sub(1);
        let (start, _) = self
            .program
            .get(last)
            .and_then(|_| self.loop_ends(last))
            .unwrap_or((last, None));
        match self.program.get(start) {
            Some(Item::Block(b)) => b.times % MAX_TIMES + 1,
            Some(Item::LoopStart { times }) if *times >= MAX_TIMES => 2,
            Some(Item::LoopStart { times }) => times + 1,
            _ => 2,
        }
    }

    fn visible(&self, button: Button) -> bool {
        match button {
            Button::Axe => self.level().chop,
            Button::Check => self.level().check,
            Button::Repeat => self.level().repeat,
            Button::Loop => self.level().loops,
            _ => true,
        }
    }

    fn enabled(&self, button: Button) -> bool {
        let editing = self.mode == Mode::Edit;
        match button {
            Button::Arrow(_) | Button::Axe | Button::Check => editing && self.has_room(),
            Button::Loop => editing && (self.has_room() || self.open_loop().is_some()),
            Button::Repeat | Button::Clear => editing && !self.program.is_empty(),
            Button::Undo => editing && (self.armed.is_some() || !self.program.is_empty()),
            Button::Play => self.running() || (editing && self.has_blocks()),
            Button::Fullscreen => true,
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
                    _ => Cmd::Move(dir),
                };
                self.program.push(Item::Block(Block { cmd, times: 1 }));
            }
            Button::Axe | Button::Check => {
                self.armed = (self.armed != Some(button)).then_some(button);
            }
            Button::Repeat => {
                let next = self.next_times();
                if let Some((times, _)) = self.last_times() {
                    *times = next;
                }
            }
            Button::Loop => match self.open_loop() {
                // Tapped again straight away: never mind.
                Some(start) if start + 1 == self.program.len() => {
                    self.program.pop();
                }
                Some(_) if self.has_room() => self.program.push(Item::LoopEnd),
                Some(_) => {}
                None => self.program.push(Item::LoopStart { times: 2 }),
            },
            Button::Undo if self.armed.is_some() => self.armed = None,
            Button::Undo => {
                self.program.pop();
            }
            Button::Clear => {
                self.program.clear();
                self.armed = None;
            }
            Button::Play if self.running() => self.reset_robot(),
            Button::Play => {
                self.armed = None;
                self.reset_robot();
                self.trace = level::expand(&self.program);
                self.start_run();
            }
            Button::Fullscreen => {
                self.fullscreen = !self.fullscreen;
                set_fullscreen(self.fullscreen);
            }
        }
    }

    /// Reveal what's under the mystery clouds (different every time), then go.
    fn start_run(&mut self) {
        let mysteries = self.level().mysteries.clone();
        if mysteries.is_empty() {
            self.act();
            return;
        }
        let outcomes = self.level().outcomes();
        let outcome = &outcomes[gen_range(0, outcomes.len())];
        self.world = self.level().revealed(outcome);
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
        self.facing = exec.cmd.dir();
        match self.world.act(exec.cmd) {
            Ok(event) => {
                self.event = Some(event);
                self.set_mode(match event {
                    Event::Moved { .. } => Mode::Moving,
                    Event::Chopped(_) => Mode::Chopping,
                    Event::Checked { .. } => Mode::Checking,
                });
            }
            Err(()) => {
                self.failed_step = Some(exec.item);
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
            self.set_mode(Mode::Win);
            self.confetti();
            return;
        }
        self.pc += 1;
        self.act();
    }

    fn update(&mut self, dt: f32) {
        self.time += dt;
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
        // Tap a block in the program to take it out. Tapping either end of a
        // loop takes out the loop but keeps the blocks inside.
        if self.mode == Mode::Edit
            && let Some(i) = (0..self.program.len()).find(|&i| self.slot_rect(i).contains(m))
        {
            match self.loop_ends(i) {
                Some((start, end)) => {
                    if let Some(end) = end {
                        self.program.remove(end);
                    }
                    self.program.remove(start);
                }
                None => {
                    self.program.remove(i);
                }
            }
            self.failed_step = None;
            return;
        }
        // Tap a level dot to jump there.
        if let Some(i) = (0..self.levels.len()).find(|&i| (m - self.level_dot(i)).length() < 20.0) {
            self.load(i);
        }
    }

    fn handle_keys(&mut self) {
        if is_key_pressed(KeyCode::F11) || (is_key_pressed(KeyCode::Escape) && self.fullscreen) {
            self.press(Button::Fullscreen);
        }
        let typed = get_char_pressed();
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
            && let Some((times, min)) = self.last_times()
        {
            *times = (n as u8).max(min);
        }
    }

    // --- geometry -------------------------------------------------------

    /// Every button for this level, with its centre and half-size.
    fn buttons(&self) -> Vec<(Button, Vec2, f32)> {
        let tools: Vec<Button> = [Button::Axe, Button::Check, Button::Repeat, Button::Loop]
            .into_iter()
            .filter(|&b| self.visible(b))
            .collect();
        let middle = (tools.len() as f32 - 1.0) / 2.0;
        let spacing = (272.0 / tools.len() as f32).min(92.0);
        let half = (spacing * 0.45).min(40.0);
        let tool_buttons = tools.into_iter().enumerate().map(move |(i, b)| {
            let x = 1256.0 + (i as f32 - middle) * spacing;
            (b, vec2(x, 412.0), half)
        });
        BUTTONS.into_iter().chain(tool_buttons).collect()
    }

    fn level_dot(&self, i: usize) -> Vec2 {
        let spacing = (1270.0 / (self.levels.len() - 1) as f32).min(52.0);
        vec2(40.0 + i as f32 * spacing, 36.0)
    }

    fn cell_size(&self) -> f32 {
        let l = self.level();
        (BOARD.w / l.w as f32)
            .min(BOARD.h / l.h as f32)
            .min(MAX_CELL)
    }

    fn grid_origin(&self) -> Vec2 {
        let l = self.level();
        BOARD.center() - vec2(l.w as f32, l.h as f32) * self.cell_size() / 2.0
    }

    fn cell_centre_f(&self, p: Vec2) -> Vec2 {
        self.grid_origin() + (p + vec2(0.5, 0.5)) * self.cell_size()
    }

    fn cell_centre(&self, p: Pos) -> (Vec2, f32) {
        (self.cell_centre_f(pos_vec(p)), self.cell_size())
    }

    fn strip_cols(&self) -> usize {
        self.level().slots.min(SLOTS_PER_ROW)
    }

    fn slot_rect(&self, i: usize) -> Rect {
        let cols = self.strip_cols();
        let width = cols as f32 * (SLOT + SLOT_GAP) - SLOT_GAP;
        let x0 = BOARD.center().x - width / 2.0;
        Rect::new(
            x0 + (i % cols) as f32 * (SLOT + SLOT_GAP),
            STRIP_Y + (i / cols) as f32 * (SLOT + SLOT_GAP),
            SLOT,
            SLOT,
        )
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
        self.draw_level_dots();
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
    }

    fn draw_level_dots(&self) {
        for i in 0..self.levels.len() {
            let c = self.level_dot(i);
            if i == self.idx {
                draw_circle(c.x, c.y, 22.0, WHITE);
                draw_circle(c.x, c.y, 19.0, dir_color(Dir::Up));
                draw_label(&format!("{}", i + 1), c, 26.0, WHITE);
            } else if self.solved[i] {
                draw_star(c, 17.0, 0.0, STAR);
            } else {
                draw_circle(c.x, c.y, 11.0, Color::new(1.0, 1.0, 1.0, 0.7));
            }
        }
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
        // While checking, a ring shows where the robot is looking.
        if let (Mode::Checking, Some(Event::Checked { at, .. })) = (self.mode, self.event) {
            let c = self.cell_centre(at).0;
            let pulse = (self.time * 8.0).sin() * 0.5 + 0.5;
            draw_circle_lines(c.x, c.y, cell * (0.42 + pulse * 0.04), 5.0, WHITE);
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
        draw_robot(c, cell, face, look, self.time, lift);
        if self.mode == Mode::Checking {
            draw_thought(c, cell);
        }

        // Swing the axe while chopping (or while trying to chop something
        // that isn't a tree).
        let failed_chop = self.mode == Mode::Bump
            && self
                .failed_step
                .and_then(|i| self.program.get(i))
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
        }
    }

    fn draw_item(r: Rect, item: Item, colour: Color, badge: Option<u8>) {
        rounded_rect(r.x, r.y + 4.0, r.w, r.h, 12.0, darker(colour, 0.7));
        rounded_rect(r.x, r.y, r.w, r.h, 12.0, colour);
        match item {
            Item::Block(b) => match b.cmd {
                Cmd::Move(dir) => draw_arrow(r.center(), r.w * 0.8, dir, WHITE),
                Cmd::Chop(dir) => draw_chop_icon(r.center(), r.w, dir),
                Cmd::IfTreeChop(dir) => draw_check_icon(r.center(), r.w, dir),
            },
            Item::LoopStart { .. } => {
                draw_bracket(r, r.x + r.w * 0.22, true, WHITE);
                draw_loop_icon(r.center() + vec2(r.w * 0.14, 0.0), r.w * 0.42, WHITE);
            }
            Item::LoopEnd => draw_bracket(r, r.x + r.w * 0.6, false, WHITE),
        }
        if let Some(n) = badge {
            let c = vec2(r.x + r.w - 4.0, r.y + 4.0);
            draw_circle(c.x, c.y, 15.0, INK);
            draw_circle(c.x, c.y, 13.0, WHITE);
            draw_label(&format!("{n}"), c, 24.0, INK);
        }
    }

    fn draw_program(&self) {
        let slots = self.level().slots;
        let first = self.slot_rect(0);
        let last = self.slot_rect(slots - 1);
        let right = self.slot_rect(self.strip_cols() - 1);
        rounded_rect(
            first.x - 12.0,
            first.y - 12.0,
            right.right() - first.x + 24.0,
            last.bottom() - first.y + 24.0,
            20.0,
            PANEL,
        );
        let running = matches!(
            self.mode,
            Mode::Moving | Mode::Checking | Mode::Chopping | Mode::Bump
        );
        let current = self.trace.get(self.pc).filter(|_| running);

        // A band behind each loop shows which blocks are inside it.
        for (i, item) in self.program.iter().enumerate() {
            if matches!(item, Item::LoopStart { .. }) {
                let end = self.loop_ends(i).and_then(|(_, e)| e);
                self.draw_loop_band(i, end.unwrap_or(self.program.len() - 1));
            }
        }

        for i in 0..slots {
            let mut r = self.slot_rect(i);
            let Some(&item) = self.program.get(i) else {
                if !self.loop_band_covers(i) {
                    rounded_rect(r.x, r.y, r.w, r.h, 12.0, SLOT_EMPTY);
                }
                continue;
            };
            // How many more times this loop will run, if it's running now.
            let looping = current
                .and_then(|e| e.in_loop)
                .filter(|&(start, _)| start == i)
                .map(|(_, left)| left);
            let active = current.is_some_and(|e| e.item == i);
            let failed = self.failed_step == Some(i);
            let done = current.is_some_and(|e| e.item > i) && looping.is_none();
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
            let badge = match item {
                Item::Block(b) if active => current.map(|e| e.left).filter(|_| b.times > 1),
                Item::Block(b) => (b.times > 1).then_some(b.times),
                Item::LoopStart { times } => looping.or(Some(times)),
                Item::LoopEnd => None,
            };
            Self::draw_item(r, item, colour, badge);
        }
    }

    /// Is empty slot `i` inside a loop that's still open?
    fn loop_band_covers(&self, i: usize) -> bool {
        self.open_loop().is_some() && i == self.program.len()
    }

    /// Shade the slots from `start` to `end`, one row at a time. An open loop
    /// also takes in the next empty slot, where new blocks will go.
    fn draw_loop_band(&self, start: usize, end: usize) {
        let open = self.program.get(end) != Some(&Item::LoopEnd);
        let end = if open {
            (end + 1).min(self.level().slots - 1)
        } else {
            end
        };
        let cols = self.strip_cols();
        let mut i = start;
        while i <= end {
            let row_end = end.min((i / cols + 1) * cols - 1);
            let (a, b) = (self.slot_rect(i), self.slot_rect(row_end));
            rounded_rect(
                a.x - 6.0,
                a.y - 6.0,
                b.right() - a.x + 12.0,
                a.h + 12.0,
                16.0,
                LOOP_BAND,
            );
            i = row_end + 1;
        }
        if open {
            let r = self.slot_rect(end);
            if end >= self.program.len() {
                let pulse = (self.time * 4.0).sin() * 0.5 + 0.5;
                let c = mix(LOOP_BAND, REPEAT_TEAL, 0.3 + pulse * 0.3);
                rounded_rect(r.x, r.y, r.w, r.h, 12.0, c);
            }
        }
    }

    fn draw_buttons(&self) {
        let tools = self.level().chop || self.level().repeat;
        let panel_h = if tools { 400.0 } else { 300.0 };
        rounded_rect(1110.0, 70.0, 292.0, panel_h, 30.0, PANEL);
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
                Button::Play if self.running() => (STOP_RED, half),
                Button::Play => (PLAY_GREEN, half),
                Button::Undo => (UNDO_YELLOW, 18.0),
                Button::Clear => (STOP_RED, 18.0),
                Button::Fullscreen => (PANEL, 10.0),
            };
            let colour = if enabled { colour } else { DISABLED };
            let size = half * 2.0;
            // An armed tool glows to say "now pick which way", and an open
            // loop glows until it's closed.
            let open_loop = button == Button::Loop && self.open_loop().is_some();
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
            if button != Button::Fullscreen {
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
                    draw_play_icon(c, half)
                }
                Button::Undo => draw_undo_icon(c, half),
                Button::Clear => draw_trash_icon(c, half),
                Button::Fullscreen => draw_fullscreen_icon(c, half, INK),
            }
        }
    }

    fn draw_next_prompt(&self) {
        let c = BOARD.center();
        let pulse = 1.0 + (self.time * 5.0).sin() * 0.06;
        let r = 70.0 * pulse;
        draw_circle(c.x, c.y + 6.0, r, darker(PLAY_GREEN, 0.7));
        draw_circle(c.x, c.y, r, PLAY_GREEN);
        draw_arrow(c, r * 1.4, Dir::Right, WHITE);
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

/// Start full screen unless run with `--window`.
fn start_fullscreen() -> bool {
    !std::env::args().any(|a| a == "--window")
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
    loop {
        let camera = fit_camera();
        set_camera(&camera);
        if is_mouse_button_pressed(MouseButton::Left) {
            game.handle_click(camera.screen_to_world(mouse_position().into()));
        }
        game.handle_keys();
        game.update(get_frame_time().min(0.05));
        game.draw();
        next_frame().await;
    }
}
