//! Maps made in the game's map maker: the tiles, which blocks they allow, and
//! how they're saved. No graphics here, so it can be tested.

use crate::level::{Level, MAX_PROGRAM, Sense};

/// The most maps that can be kept.
pub const MAX_MAPS: usize = 8;
/// How big a map can be.
pub const MAX_W: usize = 12;
pub const MAX_H: usize = 7;
/// With functions, every strip is a single row with its name in front.
const MAX_ROW_SLOTS: usize = 11;

/// What can be painted, as the level text uses it. A rain cloud is painted
/// as `3`; tapping one again counts it down to 2, then 1, then 3 again.
pub const PAINTS: [char; 12] = ['.', '#', '@', '~', 'w', '=', '?', 'k', 'D', '3', '*', 'R'];

/// Can a map's cell hold `c`: something painted, or a rain cloud at any
/// count?
fn paintable(c: char) -> bool {
    PAINTS.contains(&c) || matches!(c, '1' | '2')
}

/// A block a map can allow.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Axe,
    /// The sleep block. Rain clouds always bring it. (Saved as "wait",
    /// its old name.)
    Sleep,
    Repeat,
    Loops,
    /// "While" (saved as "until", its old name).
    While,
    /// The "if" block. It can look for anything on the map, or the edge.
    If,
    Else,
    /// "Not", for the "if": "if there's no...".
    Not,
    /// The first function (●), and the second (▲).
    Fn1,
    Fn2,
    /// Functions called with a number.
    Numbers,
    /// The break block, and "while true".
    Break,
}

impl Tool {
    /// In the order they're shown, and saved: each comes after the blocks it
    /// needs.
    pub const ALL: [Tool; 12] = [
        Tool::Axe,
        Tool::Sleep,
        Tool::Repeat,
        Tool::Loops,
        Tool::While,
        Tool::If,
        Tool::Else,
        Tool::Not,
        Tool::Fn1,
        Tool::Fn2,
        Tool::Numbers,
        Tool::Break,
    ];

    fn name(self) -> &'static str {
        match self {
            Tool::Axe => "axe",
            Tool::Repeat => "repeat",
            Tool::Loops => "loops",
            Tool::While => "until",
            Tool::Else => "else",
            Tool::Numbers => "numbers",
            Tool::Sleep => "wait",
            Tool::Not => "not",
            Tool::If => "if",
            Tool::Fn1 => "fn1",
            Tool::Fn2 => "fn2",
            Tool::Break => "break",
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
    ifs: bool,
    whiles: bool,
    otherwise: bool,
    functions: usize,
    numbers: bool,
    sleep: bool,
    not: bool,
    breaks: bool,
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
            ifs: false,
            whiles: false,
            otherwise: false,
            functions: 0,
            numbers: false,
            sleep: false,
            not: false,
            breaks: false,
        };
        map.paint(0, 1, 'R');
        map.paint(5, 2, '*');
        map
    }

    pub fn get(&self, x: usize, y: usize) -> char {
        self.cells[y * self.w + x]
    }

    /// Paint one cell. There's only one robot, so painting it somewhere
    /// moves it.
    pub fn paint(&mut self, x: usize, y: usize, what: char) {
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
            Tool::If => self.ifs,
            Tool::While => self.whiles,
            Tool::Else => self.otherwise,
            Tool::Fn1 => self.functions >= 1,
            Tool::Fn2 => self.functions >= 2,
            Tool::Numbers => self.numbers,
            Tool::Sleep => self.sleep,
            Tool::Not => self.not,
            Tool::Break => self.breaks,
        }
    }

    pub fn functions(&self) -> usize {
        self.functions
    }

    /// Turn a block on or off. Some blocks need others: loops need repeat
    /// (for their count), and "repeat until" and functions need loops, just
    /// as in the game's own levels. The second function needs the
    /// first, "else" needs an "if", numbers need a function, and a break
    /// needs a loop and an "if" (to decide when).
    pub fn toggle(&mut self, tool: Tool) {
        let on = !self.has(tool);
        match tool {
            Tool::Axe => self.axe = on,
            Tool::Repeat => self.repeat = on,
            Tool::Loops => self.loops = on,
            Tool::While => self.whiles = on,
            Tool::If => self.ifs = on,
            Tool::Else => self.otherwise = on,
            Tool::Fn1 => self.functions = usize::from(on),
            Tool::Fn2 => self.functions = if on { 2 } else { 1 },
            Tool::Numbers => self.numbers = on,
            Tool::Sleep => self.sleep = on,
            Tool::Not => self.not = on,
            Tool::Break => self.breaks = on,
        }
        if on {
            self.ifs |= self.otherwise || self.not || self.breaks;
            self.loops |= self.breaks;
            if self.numbers && self.functions == 0 {
                self.functions = 1;
            }
            self.loops |= self.whiles || self.functions > 0;
            self.repeat |= self.loops;
        } else if !self.repeat {
            self.loops = false;
        }
        if !self.loops {
            (self.whiles, self.functions, self.breaks) = (false, 0, false);
        }
        self.breaks &= self.ifs;
        self.otherwise &= self.ifs;
        self.not &= self.ifs;
        self.numbers &= self.functions > 0;
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

    /// The map as a level to play. Mystery clouds come with the "if" block
    /// and the axe, to look before chopping. The "if" can look for anything
    /// on the map (a cloud might be a tree), grass, and the edge.
    pub fn to_level(&self) -> Level {
        let mut level = Level::from_rows(&self.rows());
        let clouds = self.cells.contains(&'?');
        level.slots = self.slots;
        level.fn_slots = self.fn_slots;
        level.chop = self.axe || clouds;
        level.repeat = self.repeat;
        level.loops = self.loops;
        if self.ifs || clouds {
            let on_map = |c: char| self.cells.contains(&c);
            level.ifs = Sense::ALL
                .into_iter()
                .filter(|s| match s {
                    Sense::Tree => on_map('#') || clouds,
                    Sense::Rock => on_map('@'),
                    Sense::Ice => on_map('~'),
                    Sense::Key => on_map('k'),
                    Sense::Door => on_map('D'),
                    Sense::Star | Sense::Grass | Sense::Edge => true,
                    Sense::Wall | Sense::True => false,
                })
                .collect();
        }
        level.whiles = self.whiles;
        level.breaks = self.breaks;
        // "While" can look for whatever an "if" could.
        if self.whiles {
            let on_map = |c: char| self.cells.contains(&c);
            level.while_looks = Sense::ALL
                .into_iter()
                .filter(|s| match s {
                    Sense::Tree => on_map('#') || clouds,
                    Sense::Rock => on_map('@'),
                    Sense::Ice => on_map('~'),
                    Sense::Key => on_map('k'),
                    Sense::Door => on_map('D'),
                    Sense::Star | Sense::Grass | Sense::Edge => true,
                    Sense::Wall | Sense::True => false,
                })
                .collect();
        }
        level.otherwise = self.otherwise;
        level.functions = self.functions;
        level.numbers = self.numbers;
        level.not = self.not;
        level.sleep = self.sleep || self.cells.iter().any(|&c| matches!(c, '1'..='3'));
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
                if paintable(c) {
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
                    // Maps saved before there was one "if" block.
                    let word = match word {
                        "iftree" | "ifstar" => "if",
                        w => w,
                    };
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
    fn if_looks_for_what_is_on_the_map() {
        let mut m = CustomMap::new();
        m.toggle(Tool::If);
        m.paint(1, 1, '@');
        m.paint(2, 1, '~');
        let ifs = m.to_level().ifs;
        assert_eq!(
            ifs,
            [
                Sense::Rock,
                Sense::Star,
                Sense::Ice,
                Sense::Grass,
                Sense::Edge
            ]
        );
        // Old saves had two "if" blocks; either one is the "if" now.
        let old = parse("map slots=4 loops ifstar\nR*\nend\n");
        assert!(old[0].has(Tool::If));
    }

    #[test]
    fn rain_clouds_count_down_and_bring_sleep() {
        let mut m = CustomMap::new();
        m.paint(2, 1, '3');
        assert!(m.to_level().sleep, "rain brings the sleep block");
        let maps = parse(&format(std::slice::from_ref(&m)));
        assert_eq!(maps[0].get(2, 1), '3');
        m.paint(2, 1, '1');
        let maps = parse(&format(std::slice::from_ref(&m)));
        assert_eq!(maps[0].get(2, 1), '1', "any count is kept");
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
        m.toggle(Tool::While);
        assert!(m.has(Tool::Loops) && m.has(Tool::Repeat));
        m.toggle(Tool::Fn2);
        assert_eq!(m.functions(), 2);
        assert!(
            m.slots <= MAX_ROW_SLOTS,
            "fits on one row next to functions"
        );
        // Without repeat there's nothing to count loops with.
        m.toggle(Tool::Repeat);
        assert!(!m.has(Tool::Loops) && !m.has(Tool::While) && m.functions() == 0);
        m.toggle(Tool::Fn1);
        m.toggle(Tool::Fn1);
        assert_eq!(m.functions(), 0);
        // "Else" brings an "if", and numbers bring a function; each goes
        // when what it needs does.
        m.toggle(Tool::Else);
        assert!(m.has(Tool::If));
        m.toggle(Tool::If);
        assert!(!m.has(Tool::Else));
        m.toggle(Tool::Numbers);
        assert_eq!(m.functions(), 1);
        m.toggle(Tool::Fn1);
        assert!(!m.has(Tool::Numbers));
        m.toggle(Tool::While);
        assert!(m.has(Tool::Loops));
        m.toggle(Tool::Loops);
        assert!(!m.has(Tool::While));
    }

    #[test]
    fn new_blocks_are_saved_and_played() {
        let mut m = CustomMap::new();
        for tool in [Tool::While, Tool::Else, Tool::Numbers] {
            m.toggle(tool);
        }
        let maps = parse(&format(std::slice::from_ref(&m)));
        assert_eq!(maps, [m.clone()]);
        let level = m.to_level();
        assert!(level.whiles && level.otherwise && level.numbers && !level.ifs.is_empty());
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
    fn a_map_full_of_clouds_still_plays() {
        let mut m = CustomMap::new();
        m.resize(MAX_W, MAX_H);
        for y in 0..MAX_H {
            for x in 0..MAX_W {
                m.paint(x, y, '?');
            }
        }
        m.paint(0, 0, 'R');
        m.paint(1, 0, '*');
        let level = m.to_level();
        assert_eq!(level.mysteries.len(), MAX_W * MAX_H - 2);
        let mut toss = false;
        let outcome = level.random_outcome(|| {
            toss = !toss;
            toss
        });
        assert_eq!(outcome.len(), level.mysteries.len());
    }

    #[test]
    fn clouds_bring_the_if_block() {
        let mut m = CustomMap::new();
        m.paint(1, 1, '?');
        let level = m.to_level();
        assert!(level.ifs.contains(&Sense::Tree) && level.chop);
        assert_eq!(level.mysteries, vec![(1, 1)]);
        assert!(!CustomMap::new().to_level().chop);
    }
}
