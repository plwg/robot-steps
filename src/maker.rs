//! The map maker: a screen of everyone's maps, and an editor where maps are
//! painted tile by tile and given the blocks they allow.

use macroquad::prelude::*;

use crate::custom::{self, CustomMap, MAX_MAPS, Tool};
use crate::level::{Dir, Weather};
use crate::sound::Sfx;
use crate::{
    AXE_BROWN, BACK_BUTTON, CALL_INK, DISABLED, Face, GRASS, GRASS_DARK, Game, IF_LIME, INK,
    MAX_CELL, PANEL, PLAY_GREEN, Paint, REPEAT_TEAL, SHADOW, SKY, SLEEP_SLATE, STAR, STOP_RED,
    Screen, W, darker, dir_color, draw_answer, draw_arrow, draw_axe_badge, draw_bracket, draw_door,
    draw_ice, draw_key, draw_label, draw_loop_icon, draw_mystery, draw_no_sign, draw_number_icon,
    draw_play_icon, draw_question, draw_rain_cloud, draw_robot, draw_rock, draw_shape,
    draw_sleep_icon, draw_star, draw_trash_icon, draw_tree, draw_until_icon, mix, progress,
    rounded_rect,
};

/// Where the map maker is up to.
pub struct Maker {
    /// The map being edited.
    map: usize,
    /// What a tap on the board paints.
    paint: char,
    /// The cell painted last while the pointer is held down, so dragging
    /// paints each cell once.
    painted: Option<(usize, usize)>,
    /// A map's bin was tapped once; a second tap (soon after) deletes it.
    delete_armed: Option<(usize, f32)>,
}

impl Maker {
    pub fn new() -> Maker {
        Maker {
            map: 0,
            paint: '#',
            painted: None,
            delete_armed: None,
        }
    }
}

/// The map maker's button on the level map.
pub const MAKER_BUTTON: Vec2 = vec2(W - 150.0, 38.0);

/// Where the editor's map goes.
const BOARD: Rect = Rect {
    x: 20.0,
    y: 80.0,
    w: 1060.0,
    h: 520.0,
};

/// The editor's buttons.
#[derive(Clone, Copy, PartialEq)]
enum Control {
    Paint(char),
    Tool(Tool),
    /// Make something (width, height, slots, function slots) one less or
    /// one more.
    Less(Setting),
    More(Setting),
    Play,
}

#[derive(Clone, Copy, PartialEq)]
enum Setting {
    Width,
    Height,
    Slots,
    FnSlots,
}

const SETTINGS: [Setting; 4] = [
    Setting::Width,
    Setting::Height,
    Setting::Slots,
    Setting::FnSlots,
];

/// The middle of each setting's row of controls, under the map.
fn setting_centre(i: usize) -> Vec2 {
    vec2(170.0 + i as f32 * 250.0, 690.0)
}

/// One map's card on the maps screen.
fn card(i: usize) -> Rect {
    let (col, row) = ((i % 4) as f32, (i / 4) as f32);
    let w = 290.0;
    Rect::new(
        W / 2.0 + (col - 1.5) * 320.0 - w / 2.0,
        130.0 + row * 300.0,
        w,
        260.0,
    )
}

fn edit_button(i: usize) -> Vec2 {
    let r = card(i);
    vec2(r.x + 48.0, r.bottom() - 40.0)
}

fn delete_button(i: usize) -> Vec2 {
    let r = card(i);
    vec2(r.right() - 48.0, r.bottom() - 40.0)
}

fn hit(m: Vec2, c: Vec2, half: f32) -> bool {
    (m - c).abs().max_element() <= half
}

impl Game {
    // --- moving between screens -----------------------------------------

    pub(crate) fn open_maps(&mut self) {
        self.maker.delete_armed = None;
        self.custom = None;
        self.particles.clear();
        self.screen = Screen::Maps;
    }

    fn open_maker(&mut self, i: usize) {
        self.sfx(Sfx::Tap);
        self.maker.map = i;
        self.maker.painted = None;
        self.screen = Screen::Maker;
    }

    fn save_maps(&self) {
        progress::save_maps(&self.maps);
    }

    /// Play map `i`; afterwards, go back to `from`.
    fn play_custom(&mut self, i: usize, from: Screen) {
        self.sfx(Sfx::Tap);
        self.custom = Some((self.maps[i].to_level(), from));
        self.maker.map = i;
        self.start_level();
        self.screen = Screen::Play;
    }

    /// Done with the level being played: back to the map, or to the map
    /// maker if it was a made-up map.
    pub(crate) fn leave_level(&mut self) {
        match self.custom.take() {
            Some((_, Screen::Maker)) => {
                let i = self.maker.map;
                self.open_maker(i);
            }
            Some(_) => self.open_maps(),
            None => self.open_map(),
        }
    }

    // --- the maps screen --------------------------------------------------

    /// Backspace on the maker's screens: the same as the back arrow.
    pub(crate) fn click_back(&mut self) {
        match self.screen {
            Screen::Maker => self.click_maker(BACK_BUTTON),
            _ => self.click_maps(BACK_BUTTON),
        }
    }

    pub(crate) fn click_maps(&mut self, m: Vec2) {
        let armed = self.maker.delete_armed.take();
        if hit(m, BACK_BUTTON, 30.0) {
            self.sfx(Sfx::Tap);
            self.open_map();
            return;
        }
        for i in 0..self.maps.len() {
            if (m - delete_button(i)).length() < 30.0 {
                self.sfx(Sfx::Tap);
                match armed {
                    // Second tap on the same bin: delete that map.
                    Some((j, t)) if j == i && self.time - t < 4.0 => {
                        self.maps.remove(i);
                        self.save_maps();
                    }
                    _ => self.maker.delete_armed = Some((i, self.time)),
                }
                return;
            }
            if (m - edit_button(i)).length() < 30.0 {
                self.open_maker(i);
                return;
            }
            if card(i).contains(m) {
                if self.maps[i].playable() {
                    self.play_custom(i, Screen::Maps);
                } else {
                    self.open_maker(i);
                }
                return;
            }
        }
        // The last card makes a new map.
        let n = self.maps.len();
        if n < MAX_MAPS && card(n).contains(m) {
            self.maps.push(CustomMap::new());
            self.save_maps();
            self.open_maker(n);
        }
    }

    pub(crate) fn draw_maps(&self) {
        draw_back_button();
        draw_pencil_icon(vec2(W / 2.0 - 35.0, 70.0), 30.0, INK);
        draw_star(vec2(W / 2.0 + 35.0, 70.0), 22.0, 0.0, STAR);
        for (i, map) in self.maps.iter().enumerate() {
            let mut r = card(i);
            let hover = r.contains(self.mouse);
            if hover {
                r.y -= 6.0;
            }
            rounded_rect(r.x, r.y + 8.0, r.w, r.h, 26.0, SHADOW);
            rounded_rect(r.x, r.y, r.w, r.h, 26.0, PANEL);
            let thumb = Rect::new(r.x + 16.0, r.y + 16.0, r.w - 32.0, r.h - 92.0);
            draw_custom_map(map, thumb, self.time, self.paint());
            if !map.playable() {
                // Not finished: it needs a robot and a star.
                draw_rectangle(
                    thumb.x,
                    thumb.y,
                    thumb.w,
                    thumb.h,
                    Color::new(1.0, 1.0, 1.0, 0.6),
                );
            }
            let e = edit_button(i) - vec2(0.0, card(i).y - r.y);
            rounded_rect(e.x - 26.0, e.y - 26.0, 52.0, 52.0, 14.0, CALL_INK);
            draw_pencil_icon(e, 20.0, WHITE);
            if map.playable() {
                let p = vec2(r.center().x, e.y);
                draw_circle(p.x, p.y, 26.0, PLAY_GREEN);
                draw_play_icon(p + vec2(2.0, 0.0), 22.0, WHITE);
            }
            let d = delete_button(i) - vec2(0.0, card(i).y - r.y);
            let armed = self.maker.delete_armed.is_some_and(|(j, _)| j == i);
            let pulse = (self.time * 8.0).sin() * 0.5 + 0.5;
            let (colour, size) = if armed {
                (STOP_RED, 26.0 + pulse * 4.0)
            } else {
                (DISABLED, 24.0)
            };
            rounded_rect(d.x - size, d.y - size, size * 2.0, size * 2.0, 12.0, colour);
            draw_trash_icon(d, size);
        }
        let n = self.maps.len();
        if n < MAX_MAPS {
            let r = card(n);
            let hover = r.contains(self.mouse);
            let colour = if hover { WHITE } else { mix(SKY, WHITE, 0.6) };
            rounded_rect(r.x, r.y, r.w, r.h, 26.0, colour);
            let c = r.center();
            let k = if hover { 1.1 } else { 1.0 };
            draw_circle(c.x, c.y, 52.0 * k, PLAY_GREEN);
            draw_rectangle(c.x - 30.0 * k, c.y - 8.0 * k, 60.0 * k, 16.0 * k, WHITE);
            draw_rectangle(c.x - 8.0 * k, c.y - 30.0 * k, 16.0 * k, 60.0 * k, WHITE);
        }
    }

    // --- the editor -----------------------------------------------------

    fn map(&self) -> &CustomMap {
        &self.maps[self.maker.map]
    }

    fn map_mut(&mut self) -> &mut CustomMap {
        &mut self.maps[self.maker.map]
    }

    /// The board's top-left corner and cell size.
    fn maker_grid(&self) -> (Vec2, f32) {
        let m = self.map();
        let (w, h) = (m.w as f32, m.h as f32);
        let cell = (BOARD.w / w).min(BOARD.h / h).min(MAX_CELL);
        (BOARD.center() - vec2(w, h) * cell / 2.0, cell)
    }

    /// The cell under the pointer, if it's on the board.
    fn maker_cell(&self, m: Vec2) -> Option<(usize, usize)> {
        let (o, cell) = self.maker_grid();
        let p = (m - o) / cell;
        let inside =
            p.x >= 0.0 && p.y >= 0.0 && p.x < self.map().w as f32 && p.y < self.map().h as f32;
        inside.then_some((p.x as usize, p.y as usize))
    }

    /// Every editor button, with its centre and half-size.
    fn controls(&self) -> Vec<(Control, Vec2, f32)> {
        let mut out = Vec::new();
        for (i, &p) in custom::PAINTS.iter().enumerate() {
            let (col, row) = ((i % 4) as f32, (i / 4) as f32);
            out.push((
                Control::Paint(p),
                vec2(1256.0 + (col - 1.5) * 70.0, 128.0 + row * 82.0),
                31.0,
            ));
        }
        for (i, &t) in Tool::ALL.iter().enumerate() {
            let (col, row) = ((i % 6) as f32, (i / 6) as f32);
            out.push((
                Control::Tool(t),
                vec2(1139.0 + col * 47.0, 436.0 + row * 66.0),
                21.0,
            ));
        }
        for (i, &s) in SETTINGS.iter().enumerate() {
            if s == Setting::FnSlots && self.map().functions() == 0 {
                continue;
            }
            let c = setting_centre(i);
            out.push((Control::Less(s), c - vec2(10.0, 0.0), 24.0));
            out.push((Control::More(s), c + vec2(100.0, 0.0), 24.0));
        }
        out.push((Control::Play, vec2(1256.0, 680.0), 60.0));
        out
    }

    fn setting(&self, s: Setting) -> usize {
        let m = self.map();
        match s {
            Setting::Width => m.w,
            Setting::Height => m.h,
            Setting::Slots => m.slots,
            Setting::FnSlots => m.fn_slots,
        }
    }

    fn change(&mut self, s: Setting, by: isize) {
        let n = self.setting(s).saturating_add_signed(by);
        let m = self.map_mut();
        match s {
            Setting::Width => m.resize(n, m.h),
            Setting::Height => m.resize(m.w, n),
            Setting::Slots => m.set_slots(n, false),
            Setting::FnSlots => m.set_slots(n, true),
        }
    }

    pub(crate) fn click_maker(&mut self, m: Vec2) {
        if hit(m, BACK_BUTTON, 30.0) {
            self.sfx(Sfx::Tap);
            self.save_maps();
            self.open_maps();
            return;
        }
        if let Some(cell) = self.maker_cell(m) {
            self.paint_at(cell);
            return;
        }
        let Some((control, ..)) = self.controls().into_iter().find(|&(_, c, h)| hit(m, c, h))
        else {
            return;
        };
        match control {
            Control::Paint(p) => self.maker.paint = p,
            Control::Tool(t) => self.map_mut().toggle(t),
            Control::Less(s) => self.change(s, -1),
            Control::More(s) => self.change(s, 1),
            Control::Play if self.map().playable() => {
                self.save_maps();
                let i = self.maker.map;
                self.play_custom(i, Screen::Maker);
                return;
            }
            Control::Play => return,
        }
        self.sfx(Sfx::Tap);
    }

    /// Paint while the pointer is held down on the board, so dragging paints
    /// a line of tiles.
    pub(crate) fn drag_paint(&mut self) {
        if self.screen != Screen::Maker || !is_mouse_button_down(MouseButton::Left) {
            self.maker.painted = None;
            return;
        }
        if let Some(cell) = self.maker_cell(self.mouse) {
            self.paint_at(cell);
        }
    }

    /// Paint one cell (once per press, however long the pointer stays on it).
    fn paint_at(&mut self, (x, y): (usize, usize)) {
        if self.maker.painted == Some((x, y)) {
            return;
        }
        let fresh = self.maker.painted.is_none();
        self.maker.painted = Some((x, y));
        let mut paint = self.maker.paint;
        // Tapping a rain cloud with rain counts it down: 3, 2, 1, 3...
        let here = self.map().get(x, y);
        if paint == '3' && fresh && matches!(here, '1'..='3') {
            paint = match here {
                '3' => '2',
                '2' => '1',
                _ => '3',
            };
        }
        if here != paint {
            self.map_mut().paint(x, y, paint);
            self.sfx(Sfx::Step(Dir::Up));
        }
    }

    pub(crate) fn draw_maker(&self) {
        draw_back_button();
        let (o, cell) = self.maker_grid();
        let map = self.map();
        let size = vec2(map.w as f32, map.h as f32) * cell;
        let board = Rect::new(o.x, o.y, size.x, size.y);
        draw_custom_map(map, board, self.time, self.paint());
        // A grid, so each tile is easy to aim at.
        let line = Color::new(1.0, 1.0, 1.0, 0.35);
        for x in 1..map.w {
            let lx = o.x + x as f32 * cell;
            draw_line(lx, o.y, lx, o.y + size.y, 2.0, line);
        }
        for y in 1..map.h {
            let ly = o.y + y as f32 * cell;
            draw_line(o.x, ly, o.x + size.x, ly, 2.0, line);
        }
        // What a tap would paint, under the pointer.
        if let Some((x, y)) = self.maker_cell(self.mouse) {
            let c = o + vec2(x as f32 + 0.5, y as f32 + 0.5) * cell;
            draw_rectangle_lines(c.x - cell / 2.0, c.y - cell / 2.0, cell, cell, 6.0, STAR);
        }

        // The tiles to paint with, and the blocks the map allows.
        rounded_rect(40.0, 640.0, 1020.0, 100.0, 24.0, PANEL);
        rounded_rect(1110.0, 80.0, 292.0, 290.0, 30.0, PANEL);
        rounded_rect(1110.0, 390.0, 292.0, 160.0, 30.0, PANEL);
        for (control, c, half) in self.controls() {
            match control {
                Control::Paint(p) => {
                    if self.maker.paint == p {
                        let g = 6.0 + ((self.time * 6.0).sin() * 0.5 + 0.5) * 3.0;
                        rounded_rect(
                            c.x - half - g,
                            c.y - half - g,
                            2.0 * (half + g),
                            2.0 * (half + g),
                            18.0 + g,
                            STAR,
                        );
                    }
                    rounded_rect(c.x - half, c.y - half, half * 2.0, half * 2.0, 14.0, GRASS);
                    draw_tile(p, c, half * 2.0, self.time, self.paint());
                }
                Control::Tool(t) => {
                    let on = map.has(t);
                    let colour = if on { tool_colour(t) } else { DISABLED };
                    rounded_rect(
                        c.x - half,
                        c.y - half + 5.0,
                        half * 2.0,
                        half * 2.0,
                        14.0,
                        darker(colour, 0.7),
                    );
                    rounded_rect(c.x - half, c.y - half, half * 2.0, half * 2.0, 14.0, colour);
                    draw_tool_icon(t, c, half);
                }
                Control::Less(s) | Control::More(s) => {
                    let more = matches!(control, Control::More(_));
                    let enabled = match (s, more) {
                        (Setting::Width, true) => map.w < custom::MAX_W,
                        (Setting::Height, true) => map.h < custom::MAX_H,
                        (Setting::Width, false) => map.w > 2,
                        (Setting::Height | Setting::Slots | Setting::FnSlots, false) => {
                            self.setting(s) > 1
                        }
                        _ => true,
                    };
                    let colour = if enabled { INK } else { DISABLED };
                    rounded_rect(c.x - half, c.y - half, half * 2.0, half * 2.0, 12.0, colour);
                    draw_rectangle(c.x - 12.0, c.y - 3.0, 24.0, 6.0, WHITE);
                    if more {
                        draw_rectangle(c.x - 3.0, c.y - 12.0, 6.0, 24.0, WHITE);
                    }
                }
                Control::Play => {
                    let colour = if map.playable() { PLAY_GREEN } else { DISABLED };
                    draw_circle(c.x, c.y + 6.0, half, darker(colour, 0.7));
                    draw_circle(c.x, c.y, half, colour);
                    draw_play_icon(c, half, WHITE);
                }
            }
        }
        // Each setting: what it is, and its number.
        for (i, &s) in SETTINGS.iter().enumerate() {
            if s == Setting::FnSlots && map.functions() == 0 {
                continue;
            }
            let c = setting_centre(i);
            let icon = c - vec2(80.0, 0.0);
            match s {
                Setting::Width => {
                    draw_arrow(
                        icon - vec2(12.0, 0.0),
                        30.0,
                        Dir::Left,
                        dir_color(Dir::Left),
                    );
                    draw_arrow(
                        icon + vec2(12.0, 0.0),
                        30.0,
                        Dir::Right,
                        dir_color(Dir::Right),
                    );
                }
                Setting::Height => {
                    draw_arrow(icon - vec2(0.0, 12.0), 30.0, Dir::Up, dir_color(Dir::Up));
                    draw_arrow(
                        icon + vec2(0.0, 12.0),
                        30.0,
                        Dir::Down,
                        dir_color(Dir::Down),
                    );
                }
                Setting::Slots => {
                    for k in [-1.0, 1.0] {
                        rounded_rect(
                            icon.x + k * 13.0 - 11.0,
                            icon.y - 11.0,
                            22.0,
                            22.0,
                            5.0,
                            dir_color(Dir::Right),
                        );
                    }
                }
                Setting::FnSlots => draw_shape(icon, 16.0, 1),
            }
            draw_label(&self.setting(s).to_string(), c + vec2(45.0, 0.0), 40.0, INK);
        }
    }
}

/// The back-to-the-map arrow, top left.
fn draw_back_button() {
    let b = BACK_BUTTON;
    let colour = dir_color(Dir::Left);
    rounded_rect(
        b.x - 30.0,
        b.y - 26.0,
        60.0,
        60.0,
        14.0,
        darker(colour, 0.7),
    );
    rounded_rect(b.x - 30.0, b.y - 30.0, 60.0, 60.0, 14.0, colour);
    draw_arrow(b, 48.0, Dir::Left, WHITE);
}

fn tool_colour(tool: Tool) -> Color {
    match tool {
        Tool::Axe => AXE_BROWN,
        Tool::Sleep => SLEEP_SLATE,
        Tool::Repeat | Tool::Loops | Tool::Until => REPEAT_TEAL,
        Tool::If | Tool::Else | Tool::Not => IF_LIME,
        Tool::Fn1 | Tool::Fn2 | Tool::Numbers => CALL_INK,
    }
}

/// The same pictures as on the game's own buttons.
fn draw_tool_icon(tool: Tool, c: Vec2, half: f32) {
    let size = half * 2.0;
    match tool {
        Tool::Axe => draw_axe_badge(c, size * 0.75),
        Tool::Repeat => draw_loop_icon(c, half, WHITE),
        Tool::Loops => {
            let r = Rect::new(c.x - half, c.y - half, size, size);
            draw_bracket(r, c.x - size * 0.22, true, WHITE);
            draw_bracket(r, c.x + size * 0.22, false, WHITE);
        }
        Tool::If => draw_question(c, size * 0.8, WHITE),
        Tool::Until => draw_until_icon(c, size, Dir::Right),
        Tool::Else => draw_answer(c, false),
        Tool::Fn1 => draw_shape(c, half * 0.55, 1),
        Tool::Fn2 => draw_shape(c, half * 0.55, 2),
        Tool::Numbers => draw_number_icon(c, half, 1),
        Tool::Sleep => draw_sleep_icon(c, size, WHITE),
        Tool::Not => draw_no_sign(c, half * 0.6),
    }
}

/// One tile of a made-up map, `cell` across.
fn draw_tile(what: char, c: Vec2, cell: f32, time: f32, paint: Paint) {
    match what {
        '#' => draw_tree(c, cell, 1.0, 0.0),
        '@' => draw_rock(c, cell),
        '~' => draw_ice(c.x - cell / 2.0, c.y - cell / 2.0, cell, false),
        '?' => draw_mystery(c, cell, 1.0, time),
        'k' => draw_key(c, cell * 0.7, crate::GOLD),
        '1'..='3' => {
            let n = what as u8 - b'0';
            // Bigger than in the game, as there's no robot to keep clear of.
            draw_rain_cloud(c + vec2(0.0, cell * 0.2), cell * 1.4, Weather::Dry(n), time);
        }
        'D' => draw_door(c, cell, 0.0),
        '*' => draw_star(c, cell * 0.3, 0.0, STAR),
        'R' => draw_robot(c, cell, Face::Calm, Vec2::ZERO, time, 0.0, paint),
        _ => {}
    }
}

/// A made-up map, fitted into `r`.
fn draw_custom_map(map: &CustomMap, r: Rect, time: f32, paint: Paint) {
    let cell = (r.w / map.w as f32).min(r.h / map.h as f32);
    let o = r.center() - vec2(map.w as f32, map.h as f32) * cell / 2.0;
    for y in 0..map.h {
        for x in 0..map.w {
            let p = o + vec2(x as f32, y as f32) * cell;
            let dark = (x + y) % 2 == 1;
            if map.get(x, y) == '~' {
                draw_ice(p.x, p.y, cell, dark);
            } else {
                let colour = if dark { GRASS_DARK } else { GRASS };
                draw_rectangle(p.x, p.y, cell, cell, colour);
            }
        }
    }
    for y in 0..map.h {
        for x in 0..map.w {
            let c = o + vec2(x as f32 + 0.5, y as f32 + 0.5) * cell;
            let what = map.get(x, y);
            if what != '~' {
                draw_tile(what, c, cell, time, paint);
            }
        }
    }
}

/// A pencil, pointing down to the left.
pub fn draw_pencil_icon(c: Vec2, r: f32, colour: Color) {
    let d = vec2(-1.0, 1.0).normalize();
    let n = vec2(-d.y, d.x);
    let p = |along: f32, across: f32| c + d * along * r + n * across * r;
    let body = [p(-0.9, -0.28), p(0.45, -0.28), p(0.45, 0.28), p(-0.9, 0.28)];
    draw_triangle(body[0], body[1], body[2], colour);
    draw_triangle(body[0], body[2], body[3], colour);
    draw_triangle(p(0.45, -0.28), p(1.0, 0.0), p(0.45, 0.28), colour);
    let eraser = if colour == WHITE {
        STOP_RED
    } else {
        darker(colour, 1.6)
    };
    draw_triangle(p(-0.9, -0.28), p(-0.7, -0.28), p(-0.7, 0.28), eraser);
    draw_triangle(p(-0.9, -0.28), p(-0.7, 0.28), p(-0.9, 0.28), eraser);
}
