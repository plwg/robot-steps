//! Maps made in the game's map maker: the tiles, which blocks they allow, and
//! how they're saved. No graphics here, so it can be tested.

use crate::level::{Level, MAX_PROGRAM};

/// The most maps that can be kept.
pub const MAX_MAPS: usize = 8;
/// How big a map can be.
pub const MAX_W: usize = 12;
pub const MAX_H: usize = 7;
/// The game tries every way the mystery clouds can turn out, so a map can't
/// have too many.
pub const MAX_CLOUDS: usize = 8;
/// With functions, every strip is a single row with its name in front.
const MAX_ROW_SLOTS: usize = 11;

/// What can be painted, as the level text uses it.
pub const PAINTS: [char; 9] = ['.', '#', '@', '~', '?', 'k', 'D', '*', 'R'];

/// A block a map can allow.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Axe,
    Repeat,
    Loops,
    IfTree,
    IfStar,
    /// The first function (●), and the second (▲).
    Fn1,
    Fn2,
}

impl Tool {
    pub const ALL: [Tool; 7] = [
        Tool::Axe,
        Tool::Repeat,
        Tool::Loops,
        Tool::IfTree,
        Tool::IfStar,
        Tool::Fn1,
        Tool::Fn2,
    ];

    fn name(self) -> &'static str {
        match self {
            Tool::Axe => "axe",
            Tool::Repeat => "repeat",
            Tool::Loops => "loops",
            Tool::IfTree => "iftree",
            Tool::IfStar => "ifstar",
            Tool::Fn1 => "fn1",
            Tool::Fn2 => "fn2",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct CustomMap {
    pub w: usize,
    pub h: usize,
    /// Row by row, in the level legend (see `PAINTS`).
    cells: Vec<char>,
    /// How many blocks fit in the main program, and in each function.
    pub slots: usize,
    pub fn_slots: usize,
    axe: bool,
    repeat: bool,
    loops: bool,
    if_tree: bool,
    if_star: bool,
    functions: usize,
}

impl CustomMap {
    /// A fresh map: a robot on one side and a star on the other.
    pub fn new() -> CustomMap {
        let mut map = CustomMap {
            w: 6,
            h: 4,
            cells: vec!['.'; 24],
            slots: 12,
            fn_slots: 4,
            axe: false,
            repeat: false,
            loops: false,
            if_tree: false,
            if_star: false,
            functions: 0,
        };
        map.paint(0, 1, 'R');
        map.paint(5, 2, '*');
        map
    }

    pub fn get(&self, x: usize, y: usize) -> char {
        self.cells[y * self.w + x]
    }

    /// Paint one cell. There's only one robot, so painting it somewhere
    /// moves it. Once there are `MAX_CLOUDS` clouds, no more can be added.
    pub fn paint(&mut self, x: usize, y: usize, what: char) {
        let clouds = self.cells.iter().filter(|&&c| c == '?').count();
        if what == '?' && clouds >= MAX_CLOUDS {
            return;
        }
        if what == 'R' {
            for c in &mut self.cells {
                if *c == 'R' {
                    *c = '.';
                }
            }
        }
        self.cells[y * self.w + x] = what;
    }

    /// Change the size, keeping what's in the top-left corner.
    pub fn resize(&mut self, w: usize, h: usize) {
        let (w, h) = (w.clamp(2, MAX_W), h.clamp(1, MAX_H));
        let mut cells = vec!['.'; w * h];
        for y in 0..h.min(self.h) {
            for x in 0..w.min(self.w) {
                cells[y * w + x] = self.get(x, y);
            }
        }
        (self.w, self.h, self.cells) = (w, h, cells);
    }

    /// Change how many slots the main program (or each function) has.
    pub fn set_slots(&mut self, slots: usize, function: bool) {
        let max = if self.functions > 0 {
            MAX_ROW_SLOTS
        } else {
            MAX_PROGRAM
        };
        if function {
            self.fn_slots = slots.clamp(1, MAX_ROW_SLOTS);
        } else {
            self.slots = slots.clamp(1, max);
        }
    }

    pub fn has(&self, tool: Tool) -> bool {
        match tool {
            Tool::Axe => self.axe,
            Tool::Repeat => self.repeat,
            Tool::Loops => self.loops,
            Tool::IfTree => self.if_tree,
            Tool::IfStar => self.if_star,
            Tool::Fn1 => self.functions >= 1,
            Tool::Fn2 => self.functions >= 2,
        }
    }

    pub fn functions(&self) -> usize {
        self.functions
    }

    /// Turn a block on or off. Some blocks need others: loops need repeat
    /// (for their count), and "if" blocks and functions need loops, just as
    /// in the game's own levels. The second function needs the first.
    pub fn toggle(&mut self, tool: Tool) {
        let on = !self.has(tool);
        match tool {
            Tool::Axe => self.axe = on,
            Tool::Repeat => self.repeat = on,
            Tool::Loops => self.loops = on,
            Tool::IfTree => self.if_tree = on,
            Tool::IfStar => self.if_star = on,
            Tool::Fn1 => self.functions = usize::from(on),
            Tool::Fn2 => self.functions = if on { 2 } else { 1 },
        }
        let needs_loops = self.if_tree || self.if_star || self.functions > 0;
        if on {
            self.loops |= needs_loops;
            self.repeat |= self.loops;
        } else if !self.repeat {
            self.loops = false;
        }
        if !self.loops {
            (self.if_tree, self.if_star, self.functions) = (false, false, 0);
        }
        // Make the main program fit on one row next to the functions.
        self.set_slots(self.slots, false);
    }

    /// Can it be played: is there a robot, and a star to fetch?
    pub fn playable(&self) -> bool {
        self.cells.contains(&'R') && self.cells.contains(&'*')
    }

    pub fn rows(&self) -> Vec<String> {
        self.cells
            .chunks(self.w)
            .map(|r| r.iter().collect())
            .collect()
    }

    /// The map as a level to play. Mystery clouds come with the block that
    /// checks them (and the axe).
    pub fn to_level(&self) -> Level {
        let mut level = Level::from_rows(&self.rows());
        let clouds = self.cells.contains(&'?');
        level.slots = self.slots;
        level.fn_slots = self.fn_slots;
        level.chop = self.axe || clouds;
        level.check = clouds;
        level.repeat = self.repeat;
        level.loops = self.loops;
        level.if_tree = self.if_tree;
        level.if_star = self.if_star;
        level.functions = self.functions;
        level
    }
}

/// All the maps, as saved: each is a line of settings, its rows, and `end`.
pub fn format(maps: &[CustomMap]) -> String {
    let mut out = String::new();
    for m in maps {
        out += &format!("map slots={} fn_slots={}", m.slots, m.fn_slots);
        for tool in Tool::ALL {
            if m.has(tool) {
                out += " ";
                out += tool.name();
            }
        }
        out += "\n";
        for row in m.rows() {
            out += &row;
            out += "\n";
        }
        out += "end\n";
    }
    out
}

/// Read saved maps, skipping anything that doesn't make sense.
pub fn parse(text: &str) -> Vec<CustomMap> {
    let mut maps = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let Some(settings) = line.strip_prefix("map") else {
            continue;
        };
        let rows: Vec<&str> = lines.by_ref().take_while(|l| *l != "end").collect();
        let w = rows.first().map_or(0, |r| r.chars().count());
        let fits = (2..=MAX_W).contains(&w) && (1..=MAX_H).contains(&rows.len());
        if !fits || rows.iter().any(|r| r.chars().count() != w) {
            continue;
        }
        let mut map = CustomMap {
            w,
            h: rows.len(),
            cells: vec!['.'; w * rows.len()],
            ..CustomMap::new()
        };
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                if PAINTS.contains(&c) {
                    map.paint(x, y, c);
                }
            }
        }
        let mut slots = (map.slots, map.fn_slots);
        for word in settings.split_whitespace() {
            match word.split_once('=') {
                Some(("slots", n)) => slots.0 = n.parse().unwrap_or(slots.0),
                Some(("fn_slots", n)) => slots.1 = n.parse().unwrap_or(slots.1),
                _ => {
                    if let Some(&tool) = Tool::ALL.iter().find(|t| t.name() == word)
                        && !map.has(tool)
                    {
                        map.toggle(tool);
                    }
                }
            }
        }
        map.set_slots(slots.0, false);
        map.set_slots(slots.1, true);
        maps.push(map);
        if maps.len() == MAX_MAPS {
            break;
        }
    }
    maps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_round_trip() {
        let mut a = CustomMap::new();
        a.paint(2, 0, '#');
        a.paint(3, 3, '~');
        a.toggle(Tool::Axe);
        a.toggle(Tool::Fn2);
        a.set_slots(5, false);
        a.set_slots(3, true);
        let mut b = CustomMap::new();
        b.resize(3, 2);
        let maps = vec![a, b];
        assert_eq!(parse(&format(&maps)), maps);
    }

    #[test]
    fn junk_is_skipped() {
        let text = "hello\nmap slots=3 wings\nR*\nend\nmap\nR.\n*\nend\nmap\nRx*\nend\n";
        let maps = parse(text);
        assert_eq!(maps.len(), 2, "the uneven one is skipped");
        assert_eq!(maps[0].slots, 3);
        assert_eq!(maps[1].rows(), ["R.*"], "unknown tiles become grass");
    }

    #[test]
    fn blocks_bring_what_they_need() {
        let mut m = CustomMap::new();
        m.toggle(Tool::IfStar);
        assert!(m.has(Tool::Loops) && m.has(Tool::Repeat));
        m.toggle(Tool::Fn2);
        assert_eq!(m.functions(), 2);
        assert!(
            m.slots <= MAX_ROW_SLOTS,
            "fits on one row next to functions"
        );
        // Without repeat there's nothing to count loops with.
        m.toggle(Tool::Repeat);
        assert!(!m.has(Tool::Loops) && !m.has(Tool::IfStar) && m.functions() == 0);
        m.toggle(Tool::Fn1);
        m.toggle(Tool::Fn1);
        assert_eq!(m.functions(), 0);
    }

    #[test]
    fn one_robot_and_resizing_keeps_the_corner() {
        let mut m = CustomMap::new();
        m.paint(3, 3, 'R');
        assert_eq!(m.rows().concat().matches('R').count(), 1);
        m.resize(4, 4);
        assert_eq!(m.get(3, 3), 'R');
        assert!(!m.playable(), "the star was cut off");
        m.resize(99, 0);
        assert_eq!((m.w, m.h), (MAX_W, 1));
    }

    #[test]
    fn clouds_are_limited() {
        let mut m = CustomMap::new();
        m.resize(MAX_W, MAX_H);
        for x in 0..MAX_W {
            m.paint(x, 0, '?');
        }
        assert_eq!(m.rows()[0].matches('?').count(), MAX_CLOUDS);
        let saved = format(&[m]).replace("..", "??");
        let clouds = parse(&saved)[0].rows().concat().matches('?').count();
        assert_eq!(clouds, MAX_CLOUDS, "a saved file can't sneak more in");
        assert_eq!(parse(&saved)[0].to_level().outcomes().len(), 254);
    }

    #[test]
    fn clouds_bring_the_check_block() {
        let mut m = CustomMap::new();
        m.paint(1, 1, '?');
        let level = m.to_level();
        assert!(level.check && level.chop);
        assert_eq!(level.mysteries, vec![(1, 1)]);
        assert!(!CustomMap::new().to_level().chop);
    }
}
