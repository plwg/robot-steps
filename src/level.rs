//! Level data and the rules of the world. No graphics here, so it can be tested.

/// The most blocks a program can ever hold (two rows of twelve slots).
pub const MAX_PROGRAM: usize = 24;
/// The most times a single block can repeat.
pub const MAX_TIMES: u8 = 9;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    #[cfg(test)]
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }
}

pub type Pos = (i32, i32);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tile {
    Grass,
    /// Blocks the robot, but can be chopped down with the axe.
    Tree,
    /// What's left after chopping a tree. The robot can drive over it.
    Stump,
    /// Blocks the robot for good.
    Rock,
}

/// One instruction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Move(Dir),
    Chop(Dir),
}

impl Cmd {
    pub fn dir(self) -> Dir {
        match self {
            Cmd::Move(d) | Cmd::Chop(d) => d,
        }
    }
}

/// An instruction plus how many times to do it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Block {
    pub cmd: Cmd,
    pub times: u8,
}

pub struct LevelDef {
    /// Legend: `.` grass, `#` tree, `@` rock, `R` robot start, `*` star.
    map: &'static [&'static str],
    /// How many blocks fit in the program for this level.
    slots: usize,
    chop: bool,
    repeat: bool,
}

const fn level(map: &'static [&'static str]) -> LevelDef {
    LevelDef {
        map,
        slots: MAX_PROGRAM,
        chop: false,
        repeat: false,
    }
}

impl LevelDef {
    const fn with_axe(mut self) -> Self {
        self.chop = true;
        self
    }

    /// Unlock the repeat button and limit the program to `slots` blocks, so
    /// repeating is the only way to fit.
    const fn with_repeat(mut self, slots: usize) -> Self {
        self.repeat = true;
        self.slots = slots;
        self
    }
}

pub const LEVELS: &[LevelDef] = &[
    // --- Moving ---
    level(&["R...*"]),
    level(&["R", ".", ".", "*"]),
    level(&["R..", "##.", "##*"]),
    level(&["*##", ".##", "..R"]),
    level(&["R.*.*"]),
    level(&["R#*", ".#.", "..."]),
    level(&["R.##", "#..#", "##.*"]),
    level(&["*...*", ".#.#.", "..R.."]),
    level(&["R.#..*", ".##.#.", "....#."]),
    level(&["*....*", ".####.", "...R.*"]),
    level(&["R#*#*", ".#.#.", "....."]),
    level(&["*...*", ".#.#.", "..R..", ".#.#.", "*...*"]),
    // --- The axe: chop trees, but rocks stay put ---
    level(&["R#*"]).with_axe(),
    level(&["..@*", "R.#.", "..@."]).with_axe(),
    level(&["R@*", ".@.", "#@.", "..."]).with_axe(),
    level(&["R##.*"]).with_axe(),
    // --- Repeat: not enough slots without it ---
    level(&["R.......*"]).with_repeat(3),
    level(&["R.....", "#####.", "#####.", "#####.", "*....."]).with_repeat(4),
    level(&["R.....", "#####.", "*...#.", "#.###.", "#....."]).with_repeat(6),
    level(&["R..*...*", "#######.", "*......."]).with_repeat(4),
    // --- Axe and repeat together ---
    level(&["@@@@@@@@@", "R...#...*", "@@@@@@@@@"])
        .with_axe()
        .with_repeat(4),
    level(&["R.#....", ".@#.@@.", ".@#.@*.", "...@..."])
        .with_axe()
        .with_repeat(6),
    level(&[
        "R....#..*",
        "@@@@.@@@.",
        "*#.....#.",
        "@@@@@@@@.",
        "*........",
    ])
    .with_axe()
    .with_repeat(12),
    level(&[
        "*#..@..#*",
        "@@@.@.@@@",
        "....R....",
        "@@@.@.@@@",
        "...#@..#*",
    ])
    .with_axe()
    .with_repeat(20),
];

pub struct Level {
    pub w: i32,
    pub h: i32,
    tiles: Vec<Tile>,
    pub start: Pos,
    pub stars: Vec<Pos>,
    pub slots: usize,
    pub chop: bool,
    pub repeat: bool,
}

impl Level {
    pub fn parse(def: &LevelDef) -> Level {
        let rows = def.map;
        let h = rows.len() as i32;
        let w = rows[0].len() as i32;
        let mut tiles = Vec::new();
        let mut start = None;
        let mut stars = Vec::new();
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(row.len() as i32, w, "level rows must be the same length");
            for (x, ch) in row.chars().enumerate() {
                let p = (x as i32, y as i32);
                tiles.push(match ch {
                    '#' => Tile::Tree,
                    '@' => Tile::Rock,
                    _ => Tile::Grass,
                });
                match ch {
                    'R' => start = Some(p),
                    '*' => stars.push(p),
                    '.' | '#' | '@' => {}
                    other => panic!("unknown level tile {other:?}"),
                }
            }
        }
        Level {
            w,
            h,
            tiles,
            start: start.expect("level needs a robot (R)"),
            stars,
            slots: def.slots,
            chop: def.chop,
            repeat: def.repeat,
        }
    }

    /// A fresh copy of the world, ready for the program to run.
    pub fn world(&self) -> World {
        World {
            w: self.w,
            h: self.h,
            tiles: self.tiles.clone(),
            pos: self.start,
            stars: self.stars.clone(),
        }
    }
}

/// What happened when the robot did one instruction.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    Moved { from: Pos, star: bool },
    Chopped(Pos),
}

/// The changing state of a level while a program runs.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct World {
    pub w: i32,
    pub h: i32,
    tiles: Vec<Tile>,
    pub pos: Pos,
    /// Stars not collected yet.
    pub stars: Vec<Pos>,
}

impl World {
    pub fn tile(&self, (x, y): Pos) -> Option<Tile> {
        let inside = x >= 0 && y >= 0 && x < self.w && y < self.h;
        inside.then(|| self.tiles[(y * self.w + x) as usize])
    }

    /// Do one instruction. `Err` means the robot got stuck: it bumped into
    /// something, or swung the axe at something that isn't a tree.
    pub fn act(&mut self, cmd: Cmd) -> Result<Event, ()> {
        let (dx, dy) = cmd.dir().delta();
        let next = (self.pos.0 + dx, self.pos.1 + dy);
        match (cmd, self.tile(next)) {
            (Cmd::Move(_), Some(Tile::Grass | Tile::Stump)) => {
                let from = self.pos;
                self.pos = next;
                let star = self.stars.iter().position(|&s| s == next);
                if let Some(i) = star {
                    self.stars.remove(i);
                }
                Ok(Event::Moved {
                    from,
                    star: star.is_some(),
                })
            }
            (Cmd::Chop(_), Some(Tile::Tree)) => {
                self.tiles[(next.1 * self.w + next.0) as usize] = Tile::Stump;
                Ok(Event::Chopped(next))
            }
            _ => Err(()),
        }
    }
}

pub fn all_levels() -> Vec<Level> {
    LEVELS.iter().map(Level::parse).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashSet, VecDeque};

    /// Fewest blocks needed to collect every star (breadth-first search over
    /// every block the level allows).
    fn fewest_blocks(level: &Level, allow_repeat: bool) -> Option<usize> {
        let mut cmds: Vec<Cmd> = Dir::ALL.map(Cmd::Move).to_vec();
        if level.chop {
            cmds.extend(Dir::ALL.map(Cmd::Chop));
        }
        let max_times = if allow_repeat { MAX_TIMES } else { 1 };

        let start = level.world();
        let mut seen = HashSet::from([start.clone()]);
        let mut queue = VecDeque::from([(start, 0)]);
        while let Some((world, blocks)) = queue.pop_front() {
            for &cmd in &cmds {
                let mut next = world.clone();
                for _ in 0..max_times {
                    if next.act(cmd).is_err() {
                        break;
                    }
                    if next.stars.is_empty() {
                        return Some(blocks + 1);
                    }
                    if seen.insert(next.clone()) {
                        queue.push_back((next.clone(), blocks + 1));
                    }
                }
            }
        }
        None
    }

    #[test]
    fn every_level_is_solvable_within_its_slots() {
        for (i, level) in all_levels().iter().enumerate() {
            let n = i + 1;
            assert!(!level.stars.is_empty(), "level {n} has no stars");
            assert!(level.slots <= MAX_PROGRAM, "level {n} has too many slots");
            let blocks = fewest_blocks(level, level.repeat)
                .unwrap_or_else(|| panic!("level {n} cannot be solved"));
            println!("level {n}: {blocks} blocks, {} slots", level.slots);
            assert!(blocks <= level.slots, "level {n} needs {blocks} blocks");
        }
    }

    #[test]
    fn repeat_levels_need_repeat() {
        for (i, level) in all_levels().iter().enumerate() {
            if level.repeat {
                let without = fewest_blocks(level, false).unwrap_or(usize::MAX);
                assert!(
                    without > level.slots,
                    "level {} fits in {without} blocks without repeat",
                    i + 1
                );
            }
        }
    }

    #[test]
    fn axe_levels_need_the_axe() {
        for (i, def) in LEVELS.iter().enumerate() {
            if def.chop {
                let mut level = Level::parse(def);
                level.chop = false;
                assert_eq!(
                    fewest_blocks(&level, true),
                    None,
                    "level {} can be done without the axe",
                    i + 1
                );
            }
        }
    }

    #[test]
    fn robot_bumps_and_chops() {
        let mut world = Level::parse(&level(&["R#", ".@"])).world();
        assert!(world.act(Cmd::Move(Dir::Right)).is_err());
        assert!(world.act(Cmd::Move(Dir::Up)).is_err());
        assert!(world.act(Cmd::Chop(Dir::Down)).is_err(), "can't chop grass");
        assert_eq!(world.act(Cmd::Chop(Dir::Right)), Ok(Event::Chopped((1, 0))));
        assert!(
            world.act(Cmd::Move(Dir::Right)).is_ok(),
            "stumps are passable"
        );
        assert!(world.act(Cmd::Chop(Dir::Down)).is_err(), "can't chop rocks");
    }
}
