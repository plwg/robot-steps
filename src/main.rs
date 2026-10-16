//! Robot Steps: a first coding game. Tap arrows to build a program, press play,
//! and watch the robot follow it to collect the stars. No reading required.

mod level;

use std::f32::consts::PI;

use level::{Dir, Level, MAX_PROGRAM, Pos};
use macroquad::prelude::*;
use macroquad::rand::gen_range;

// Everything is drawn on a fixed 1200x800 canvas that is scaled to fit the window.
const W: f32 = 1200.0;
const H: f32 = 800.0;

const STEP_TIME: f32 = 0.5;
const BUMP_TIME: f32 = 0.8;
const SAD_TIME: f32 = 1.4;

const BOARD: Rect = Rect {
    x: 20.0,
    y: 70.0,
    w: 820.0,
    h: 560.0,
};
const MAX_CELL: f32 = 108.0;

const SLOTS_PER_ROW: usize = 12;
const SLOT: f32 = 60.0;
const SLOT_GAP: f32 = 8.0;
const STRIP_X: f32 = 26.0;
const STRIP_Y: f32 = 652.0;

const SKY: Color = Color::new(0.62, 0.84, 0.97, 1.0);
const GRASS: Color = Color::new(0.56, 0.82, 0.42, 1.0);
const GRASS_DARK: Color = Color::new(0.50, 0.77, 0.37, 1.0);
const STAR: Color = Color::new(1.0, 0.82, 0.1, 1.0);
const SHADOW: Color = Color::new(0.0, 0.0, 0.0, 0.18);
const PANEL: Color = Color::new(0.93, 0.96, 1.0, 1.0);
const SLOT_EMPTY: Color = Color::new(0.82, 0.88, 0.95, 1.0);
const PLAY_GREEN: Color = Color::new(0.15, 0.7, 0.25, 1.0);
const STOP_RED: Color = Color::new(0.9, 0.25, 0.25, 1.0);

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

/// A chunky arrow pointing in `dir`, sized to fit a box of `size`.
fn draw_arrow(c: Vec2, size: f32, dir: Dir, color: Color) {
    let d = dir_vec(dir);
    let n = vec2(-d.y, d.x);
    let p = |along: f32, across: f32| c + d * along * size + n * across * size;
    draw_triangle(p(-0.4, -0.13), p(0.05, -0.13), p(0.05, 0.13), color);
    draw_triangle(p(-0.4, -0.13), p(0.05, 0.13), p(-0.4, 0.13), color);
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
    const SEGMENTS: usize = 12;
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

fn draw_tree(c: Vec2, cell: f32) {
    let s = cell;
    draw_ellipse(c.x, c.y + s * 0.36, s * 0.3, s * 0.08, 0.0, SHADOW);
    draw_rectangle(
        c.x - s * 0.07,
        c.y + s * 0.05,
        s * 0.14,
        s * 0.32,
        Color::new(0.55, 0.35, 0.2, 1.0),
    );
    let leaf = Color::new(0.16, 0.55, 0.22, 1.0);
    let light = Color::new(0.24, 0.66, 0.3, 1.0);
    draw_circle(c.x - s * 0.15, c.y - s * 0.02, s * 0.2, leaf);
    draw_circle(c.x + s * 0.15, c.y - s * 0.02, s * 0.2, leaf);
    draw_circle(c.x, c.y - s * 0.2, s * 0.24, leaf);
    draw_circle(c.x - s * 0.06, c.y - s * 0.26, s * 0.1, light);
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
    // Wheels.
    for side in [-1.0, 1.0] {
        draw_circle(c.x + side * s * 0.22, c.y + s * 0.34, s * 0.1, DARKGRAY);
    }
    // Antenna with a bobbing light.
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
    // Body and face screen.
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
    let screen = Color::new(0.15, 0.2, 0.32, 1.0);
    rounded_rect(
        c.x - s * 0.27,
        c.y - s * 0.2,
        s * 0.54,
        s * 0.38,
        s * 0.08,
        screen,
    );
    // Eyes (blink now and then).
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
            draw_circle(pupil.x, pupil.y, s * 0.038, screen);
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
    let tip = vec2(l, c.y);
    let body_left = l + r * 0.3;
    draw_triangle(tip, vec2(body_left, t), vec2(body_left, b), WHITE);
    draw_rectangle(body_left, t, rt - body_left, b - t, WHITE);
    let x = vec2(body_left + (rt - body_left) * 0.5, c.y);
    let k = r * 0.15;
    let col = Color::new(0.95, 0.7, 0.1, 1.0);
    draw_line(x.x - k, x.y - k, x.x + k, x.y + k, r * 0.1, col);
    draw_line(x.x - k, x.y + k, x.x + k, x.y - k, r * 0.1, col);
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

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Button {
    Arrow(Dir),
    Play,
    Undo,
    Clear,
}

/// Each button with its centre and half-size.
const BUTTONS: [(Button, Vec2, f32); 7] = [
    (Button::Arrow(Dir::Up), vec2(1020.0, 160.0), 46.0),
    (Button::Arrow(Dir::Left), vec2(932.0, 250.0), 46.0),
    (Button::Arrow(Dir::Right), vec2(1108.0, 250.0), 46.0),
    (Button::Arrow(Dir::Down), vec2(1020.0, 340.0), 46.0),
    (Button::Play, vec2(1020.0, 510.0), 70.0),
    (Button::Undo, vec2(950.0, 680.0), 45.0),
    (Button::Clear, vec2(1090.0, 680.0), 45.0),
];

fn slot_rect(i: usize) -> Rect {
    let col = (i % SLOTS_PER_ROW) as f32;
    let row = (i / SLOTS_PER_ROW) as f32;
    Rect::new(
        STRIP_X + col * (SLOT + SLOT_GAP),
        STRIP_Y + row * (SLOT + SLOT_GAP),
        SLOT,
        SLOT,
    )
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    /// Building the program.
    Edit,
    /// Walking one step of the program.
    Moving,
    /// Walked into a tree or the edge.
    Bump,
    /// Program finished (or bumped) without getting every star.
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
    program: Vec<Dir>,
    mode: Mode,
    timer: f32,
    step: usize,
    pos: Pos,
    from: Pos,
    stars: Vec<Pos>,
    failed_step: Option<usize>,
    particles: Vec<Particle>,
    pressed: Option<(Button, f32)>,
    time: f32,
}

impl Game {
    fn new() -> Game {
        let levels = level::all_levels();
        let solved = vec![false; levels.len()];
        let mut game = Game {
            levels,
            idx: 0,
            solved,
            program: Vec::new(),
            mode: Mode::Edit,
            timer: 0.0,
            step: 0,
            pos: (0, 0),
            from: (0, 0),
            stars: Vec::new(),
            failed_step: None,
            particles: Vec::new(),
            pressed: None,
            time: 0.0,
        };
        game.load(0);
        game
    }

    fn level(&self) -> &Level {
        &self.levels[self.idx]
    }

    fn load(&mut self, idx: usize) {
        self.idx = idx;
        self.program.clear();
        self.particles.clear();
        self.reset_robot();
    }

    fn reset_robot(&mut self) {
        self.pos = self.level().start;
        self.from = self.pos;
        self.stars = self.level().stars.clone();
        self.step = 0;
        self.set_mode(Mode::Edit);
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.timer = 0.0;
    }

    fn press(&mut self, button: Button) {
        self.pressed = Some((button, self.time));
        let editing = self.mode == Mode::Edit;
        match button {
            Button::Arrow(dir) if editing && self.program.len() < MAX_PROGRAM => {
                self.program.push(dir);
                self.failed_step = None;
            }
            Button::Undo if editing => {
                self.program.pop();
                self.failed_step = None;
            }
            Button::Clear if editing => {
                self.program.clear();
                self.failed_step = None;
            }
            Button::Play if editing && !self.program.is_empty() => self.run(),
            Button::Play if matches!(self.mode, Mode::Moving | Mode::Bump | Mode::Sad) => {
                self.reset_robot()
            }
            _ => {}
        }
    }

    fn run(&mut self) {
        self.reset_robot();
        self.failed_step = None;
        self.begin_step();
    }

    fn begin_step(&mut self) {
        let Some(&dir) = self.program.get(self.step) else {
            // Ran out of steps with stars still left.
            self.set_mode(Mode::Sad);
            return;
        };
        match self.level().step(self.pos, dir) {
            Some(next) => {
                self.from = self.pos;
                self.pos = next;
                self.set_mode(Mode::Moving);
            }
            None => {
                self.failed_step = Some(self.step);
                self.set_mode(Mode::Bump);
            }
        }
    }

    fn update(&mut self, dt: f32) {
        self.time += dt;
        self.timer += dt;
        match self.mode {
            Mode::Moving if self.timer >= STEP_TIME => {
                if let Some(i) = self.stars.iter().position(|&s| s == self.pos) {
                    self.stars.remove(i);
                    let (centre, cell) = self.cell_centre(self.pos);
                    self.burst(centre, cell);
                }
                if self.stars.is_empty() {
                    self.solved[self.idx] = true;
                    self.set_mode(Mode::Win);
                    self.confetti();
                } else {
                    self.step += 1;
                    self.begin_step();
                }
            }
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
        if self.mode == Mode::Win {
            if self.timer > 0.8 {
                self.load((self.idx + 1) % self.levels.len());
            }
            return;
        }
        for (button, centre, half) in BUTTONS {
            if (m - centre).abs().max_element() <= half {
                self.press(button);
                return;
            }
        }
        // Tap a step in the program to take it out.
        if self.mode == Mode::Edit
            && let Some(i) = (0..self.program.len()).find(|&i| slot_rect(i).contains(m))
        {
            self.program.remove(i);
            self.failed_step = None;
            return;
        }
        // Tap a level dot to jump there.
        if let Some(i) = (0..self.levels.len()).find(|&i| (m - level_dot(i)).length() < 20.0) {
            self.load(i);
        }
    }

    fn handle_keys(&mut self) {
        let keys = [
            (KeyCode::Up, Button::Arrow(Dir::Up)),
            (KeyCode::Down, Button::Arrow(Dir::Down)),
            (KeyCode::Left, Button::Arrow(Dir::Left)),
            (KeyCode::Right, Button::Arrow(Dir::Right)),
            (KeyCode::Space, Button::Play),
            (KeyCode::Enter, Button::Play),
            (KeyCode::Backspace, Button::Undo),
            (KeyCode::Delete, Button::Clear),
        ];
        for (key, button) in keys {
            if is_key_pressed(key) {
                if self.mode == Mode::Win {
                    self.load((self.idx + 1) % self.levels.len());
                } else {
                    self.press(button);
                }
            }
        }
    }

    // --- geometry -------------------------------------------------------

    fn cell_size(&self) -> f32 {
        let l = self.level();
        (BOARD.w / l.w as f32)
            .min(BOARD.h / l.h as f32)
            .min(MAX_CELL)
    }

    fn grid_origin(&self) -> Vec2 {
        let l = self.level();
        let cell = self.cell_size();
        BOARD.center() - vec2(l.w as f32, l.h as f32) * cell / 2.0
    }

    /// Centre of a grid position (fractional positions allowed) and the cell size.
    fn cell_centre_f(&self, p: Vec2) -> Vec2 {
        self.grid_origin() + (p + vec2(0.5, 0.5)) * self.cell_size()
    }

    fn cell_centre(&self, (x, y): Pos) -> (Vec2, f32) {
        (
            self.cell_centre_f(vec2(x as f32, y as f32)),
            self.cell_size(),
        )
    }

    // --- effects --------------------------------------------------------

    fn burst(&mut self, at: Vec2, cell: f32) {
        for _ in 0..24 {
            let a = gen_range(0.0, 2.0 * PI);
            let speed = gen_range(150.0, 400.0);
            self.particles.push(Particle {
                pos: at,
                vel: vec2(a.cos(), a.sin()) * speed - vec2(0.0, 150.0),
                color: STAR,
                life: gen_range(0.5, 1.0),
                size: cell * gen_range(0.06, 0.12),
                spin: 0.0,
                star: true,
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
                draw_triangle(p.pos - d - n, p.pos + d - n, p.pos + d + n, p.color);
                draw_triangle(p.pos - d - n, p.pos + d + n, p.pos - d + n, p.color);
            }
        }
        if self.mode == Mode::Win && self.timer > 0.8 {
            self.draw_next_prompt();
        }
    }

    fn draw_level_dots(&self) {
        for i in 0..self.levels.len() {
            let c = level_dot(i);
            if i == self.idx {
                draw_circle(c.x, c.y, 22.0, WHITE);
                draw_circle(c.x, c.y, 19.0, Color::new(0.25, 0.55, 0.95, 1.0));
                let label = format!("{}", i + 1);
                let size = measure_text(&label, None, 30, 1.0);
                draw_text(
                    &label,
                    c.x - size.width / 2.0,
                    c.y + size.offset_y / 2.0,
                    30.0,
                    WHITE,
                );
            } else if self.solved[i] {
                draw_star(c, 18.0, 0.0, STAR);
            } else {
                draw_circle(c.x, c.y, 12.0, Color::new(1.0, 1.0, 1.0, 0.7));
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

        for y in 0..l.h {
            for x in 0..l.w {
                if l.is_wall((x, y)) {
                    draw_tree(self.cell_centre((x, y)).0, cell);
                }
            }
        }
        // Stars bob gently; leftover stars wiggle when the robot is sad.
        for (i, &s) in self.stars.iter().enumerate() {
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
        let pos = vec2(self.pos.0 as f32, self.pos.1 as f32);
        let current_dir = self.program.get(self.step).copied();
        let look = current_dir.map(dir_vec).unwrap_or(Vec2::ZERO);
        let (at, lift, face) = match self.mode {
            Mode::Moving => {
                let t = (self.timer / STEP_TIME).min(1.0);
                let eased = t * t * (3.0 - 2.0 * t);
                let from = vec2(self.from.0 as f32, self.from.1 as f32);
                let hop = (t * PI).sin() * cell * 0.15;
                (from.lerp(pos, eased), hop, Face::Happy)
            }
            Mode::Bump => {
                let t = self.timer / BUMP_TIME;
                let lunge = (t * 3.0).min(1.0) * PI;
                let shake = (self.time * 50.0).sin() * 0.04 * (1.0 - t);
                (
                    pos + look * 0.3 * lunge.sin() + vec2(shake, 0.0),
                    0.0,
                    Face::Sad,
                )
            }
            Mode::Sad => (pos, 0.0, Face::Sad),
            Mode::Win => {
                let hop = (self.timer * 8.0).sin().abs() * cell * 0.25;
                (pos, hop, Face::Happy)
            }
            Mode::Edit => (pos, 0.0, Face::Calm),
        };
        let c = self.cell_centre_f(at) - vec2(0.0, lift);
        draw_robot(c, cell, face, look, self.time, lift);
    }

    fn draw_program(&self) {
        let rows = MAX_PROGRAM.div_ceil(SLOTS_PER_ROW) as f32;
        rounded_rect(
            STRIP_X - 12.0,
            STRIP_Y - 12.0,
            SLOTS_PER_ROW as f32 * (SLOT + SLOT_GAP) - SLOT_GAP + 24.0,
            rows * (SLOT + SLOT_GAP) - SLOT_GAP + 24.0,
            20.0,
            PANEL,
        );
        let running = matches!(self.mode, Mode::Moving | Mode::Bump);
        for i in 0..MAX_PROGRAM {
            let r = slot_rect(i);
            let Some(&dir) = self.program.get(i) else {
                rounded_rect(r.x, r.y, r.w, r.h, 12.0, SLOT_EMPTY);
                continue;
            };
            let active = running && i == self.step;
            let failed = self.failed_step == Some(i);
            let done = running && i < self.step;
            let mut r = r;
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
            let mut colour = dir_color(dir);
            if done {
                colour = mix(colour, PANEL, 0.55);
            }
            rounded_rect(r.x, r.y + 4.0, r.w, r.h, 12.0, darker(colour, 0.7));
            rounded_rect(r.x, r.y, r.w, r.h, 12.0, colour);
            draw_arrow(r.center(), r.w * 0.8, dir, WHITE);
        }
    }

    fn draw_buttons(&self) {
        rounded_rect(870.0, 70.0, 300.0, 370.0, 30.0, PANEL);
        let running = matches!(self.mode, Mode::Moving | Mode::Bump | Mode::Sad);
        for (button, c, half) in BUTTONS {
            let squash = match self.pressed {
                Some((b, t)) if b == button && self.time - t < 0.12 => 0.9,
                _ => 1.0,
            };
            let half = half * squash;
            let (colour, round) = match button {
                Button::Arrow(dir) => (dir_color(dir), 18.0),
                Button::Play if running => (STOP_RED, half),
                Button::Play => (PLAY_GREEN, half),
                Button::Undo => (Color::new(0.95, 0.7, 0.1, 1.0), 18.0),
                Button::Clear => (STOP_RED, 18.0),
            };
            let editing = self.mode == Mode::Edit;
            let enabled = match button {
                Button::Arrow(_) => editing && self.program.len() < MAX_PROGRAM,
                Button::Undo | Button::Clear => editing && !self.program.is_empty(),
                Button::Play => running || (editing && !self.program.is_empty()),
            };
            let colour = if enabled {
                colour
            } else {
                Color::new(0.75, 0.78, 0.82, 1.0)
            };
            let size = half * 2.0;
            rounded_rect(
                c.x - half,
                c.y - half + 6.0,
                size,
                size,
                round,
                darker(colour, 0.7),
            );
            rounded_rect(c.x - half, c.y - half, size, size, round, colour);
            match button {
                Button::Arrow(dir) => draw_arrow(c, size * 0.8, dir, WHITE),
                Button::Play if running => {
                    let s = half * 0.38;
                    draw_rectangle(c.x - s, c.y - s, s * 2.0, s * 2.0, WHITE)
                }
                Button::Play => {
                    // Pulse to say "press me!" once there's a program to run.
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
            }
        }
    }

    fn draw_next_prompt(&self) {
        let c = vec2(BOARD.center().x, BOARD.center().y);
        let pulse = 1.0 + (self.time * 5.0).sin() * 0.06;
        let r = 70.0 * pulse;
        draw_circle(c.x, c.y + 6.0, r, darker(PLAY_GREEN, 0.7));
        draw_circle(c.x, c.y, r, PLAY_GREEN);
        draw_arrow(c, r * 1.4, Dir::Right, WHITE);
    }
}

fn level_dot(i: usize) -> Vec2 {
    vec2(50.0 + i as f32 * 56.0, 36.0)
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

fn window_conf() -> Conf {
    Conf {
        window_title: "Robot Steps".to_owned(),
        window_width: 1200,
        window_height: 800,
        window_resizable: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    rand::srand(miniquad::date::now() as u64);
    let mut game = Game::new();
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
