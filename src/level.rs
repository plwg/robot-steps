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
    /// Unknown until the program runs: becomes a tree or grass at random.
    Mystery,
}

/// One instruction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Move(Dir),
    Chop(Dir),
    /// The conditional: if there's a tree that way, chop it; otherwise do nothing.
    IfTreeChop(Dir),
}

impl Cmd {
    pub fn dir(self) -> Dir {
        match self {
            Cmd::Move(d) | Cmd::Chop(d) | Cmd::IfTreeChop(d) => d,
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
    /// Legend: `.` grass, `#` tree, `@` rock, `?` mystery (tree or grass),
    /// `R` robot start, `*` star.
    map: &'static [&'static str],
    /// How many blocks fit in the program for this level.
    slots: usize,
    chop: bool,
    repeat: bool,
    check: bool,
}

const fn level(map: &'static [&'static str]) -> LevelDef {
    LevelDef {
        map,
        slots: MAX_PROGRAM,
        chop: false,
        repeat: false,
        check: false,
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

    /// Unlock the "if tree, chop" block (and the axe).
    const fn with_check(mut self) -> Self {
        self.chop = true;
        self.check = true;
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
    // --- More practice with the axe and repeat ---
    level(&["R.#.", "@@#@", "*..."]).with_axe(),
    level(&["R.......", "#######.", "........", ".#######", "*......."]).with_repeat(5),
    // --- If: mystery clouds turn into a tree or grass each time you press
    // play, so the robot has to check before it chops ---
    level(&["R?*"]).with_check(),
    level(&["R.?.*"]).with_check(),
    level(&["R?@", "@.@", "@?*"]).with_check(),
    level(&["@@@@@@", "R?.?.*", "@@@@@@"]).with_check(),
    level(&["*?.R.?*"]).with_check(),
    level(&["R.?.@*", "@@.@@?", "*?...."]).with_check(),
    // --- If and repeat together ---
    level(&["R...?...*"]).with_check().with_repeat(3),
    level(&["R..?..#..", "@@@@@@@@.", "*..?..?.."])
        .with_check()
        .with_repeat(14),
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
    pub check: bool,
    /// Where the mystery clouds are.
    pub mysteries: Vec<Pos>,
}

impl Level {
    pub fn parse(def: &LevelDef) -> Level {
        let rows = def.map;
        let h = rows.len() as i32;
        let w = rows[0].len() as i32;
        let mut tiles = Vec::new();
        let mut start = None;
        let mut stars = Vec::new();
        let mut mysteries = Vec::new();
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(row.len() as i32, w, "level rows must be the same length");
            for (x, ch) in row.chars().enumerate() {
                let p = (x as i32, y as i32);
                tiles.push(match ch {
                    '#' => Tile::Tree,
                    '@' => Tile::Rock,
                    '?' => Tile::Mystery,
                    _ => Tile::Grass,
                });
                match ch {
                    'R' => start = Some(p),
                    '*' => stars.push(p),
                    '?' => mysteries.push(p),
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
            check: def.check,
            mysteries,
        }
    }

    /// The level as drawn while building the program, mysteries unrevealed.
    pub fn world(&self) -> World {
        World {
            w: self.w,
            h: self.h,
            tiles: self.tiles.clone(),
            pos: self.start,
            stars: self.stars.clone(),
        }
    }

    /// Every way the mysteries can turn out (`true` = tree). With two or more
    /// there's always at least one tree and one patch of grass, so a program
    /// that guesses instead of checking can't get lucky every time.
    pub fn outcomes(&self) -> Vec<Vec<bool>> {
        let n = self.mysteries.len();
        let all = (1u32 << n) - 1;
        (0..=all)
            .filter(|&bits| n < 2 || (bits != 0 && bits != all))
            .map(|bits| (0..n).map(|i| bits & (1 << i) != 0).collect())
            .collect()
    }

    /// A world ready for the program to run, with each mystery revealed.
    pub fn revealed(&self, trees: &[bool]) -> World {
        let mut world = self.world();
        for (&(x, y), &tree) in self.mysteries.iter().zip(trees) {
            world.tiles[(y * self.w + x) as usize] = if tree { Tile::Tree } else { Tile::Grass };
        }
        world
    }
}

/// What happened when the robot did one instruction.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    Moved {
        from: Pos,
        star: bool,
    },
    Chopped(Pos),
    /// Looked at `at` for a tree (and chopped it if there was one).
    Checked {
        at: Pos,
        tree: bool,
    },
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
            (Cmd::IfTreeChop(_), tile) => {
                let tree = tile == Some(Tile::Tree);
                if tree {
                    self.tiles[(next.1 * self.w + next.0) as usize] = Tile::Stump;
                }
                Ok(Event::Checked { at: next, tree })
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

    struct Allow {
        repeat: bool,
        chop: bool,
        check: bool,
    }

    impl Allow {
        fn level(level: &Level) -> Allow {
            Allow {
                repeat: level.repeat,
                chop: level.chop,
                check: level.check,
            }
        }
    }

    /// Fewest blocks in a single program that collects every star however the
    /// mysteries turn out (breadth-first search over every block allowed).
    fn fewest_blocks(level: &Level, allow: Allow) -> Option<usize> {
        let mut cmds: Vec<Cmd> = Dir::ALL.map(Cmd::Move).to_vec();
        if allow.chop {
            cmds.extend(Dir::ALL.map(Cmd::Chop));
        }
        if allow.check {
            cmds.extend(Dir::ALL.map(Cmd::IfTreeChop));
        }
        let max_times = if allow.repeat { MAX_TIMES } else { 1 };

        // One world per outcome, run in lockstep. `None` means that world is
        // already won (the game stops as soon as the last star is collected).
        type Worlds = Vec<Option<World>>;
        let start: Worlds = level
            .outcomes()
            .iter()
            .map(|o| Some(level.revealed(o)))
            .collect();
        let mut seen = HashSet::from([start.clone()]);
        let mut queue = VecDeque::from([(start, 0)]);
        while let Some((worlds, blocks)) = queue.pop_front() {
            for &cmd in &cmds {
                let mut next = worlds.clone();
                'reps: for _ in 0..max_times {
                    for slot in next.iter_mut() {
                        if let Some(world) = slot {
                            if world.act(cmd).is_err() {
                                break 'reps;
                            }
                            if world.stars.is_empty() {
                                *slot = None;
                            }
                        }
                    }
                    if next.iter().all(Option::is_none) {
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
            assert_eq!(
                level.check,
                !level.mysteries.is_empty(),
                "level {n}: mysteries and the check block go together"
            );
            let blocks = fewest_blocks(level, Allow::level(level))
                .unwrap_or_else(|| panic!("level {n} cannot be solved"));
            println!("level {n}: {blocks} blocks, {} slots", level.slots);
            assert!(blocks <= level.slots, "level {n} needs {blocks} blocks");
        }
    }

    #[test]
    fn repeat_levels_need_repeat() {
        for (i, level) in all_levels().iter().enumerate() {
            if level.repeat {
                let allow = Allow {
                    repeat: false,
                    ..Allow::level(level)
                };
                let without = fewest_blocks(level, allow).unwrap_or(usize::MAX);
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
        for (i, level) in all_levels().iter().enumerate() {
            if level.chop && !level.check {
                let allow = Allow {
                    chop: false,
                    ..Allow::level(level)
                };
                assert_eq!(
                    fewest_blocks(level, allow),
                    None,
                    "level {} can be done without the axe",
                    i + 1
                );
            }
        }
    }

    #[test]
    fn check_levels_need_the_check() {
        for (i, level) in all_levels().iter().enumerate() {
            if level.check {
                let allow = Allow {
                    check: false,
                    ..Allow::level(level)
                };
                assert_eq!(
                    fewest_blocks(level, allow),
                    None,
                    "level {} can be done without checking",
                    i + 1
                );
            }
        }
    }

    #[test]
    fn mysteries_always_include_a_tree_and_grass() {
        let pair = Level::parse(&level(&["R??"]).with_check());
        assert_eq!(pair.outcomes().len(), 2);
        let single = Level::parse(&level(&["R?"]).with_check());
        assert_eq!(single.outcomes(), vec![vec![false], vec![true]]);
    }

    #[test]
    fn robot_bumps_chops_and_checks() {
        let mut world = Level::parse(&level(&["R#", ".@"])).world();
        assert!(world.act(Cmd::Move(Dir::Right)).is_err());
        assert!(world.act(Cmd::Move(Dir::Up)).is_err());
        assert!(world.act(Cmd::Chop(Dir::Down)).is_err(), "can't chop grass");
        assert_eq!(
            world.act(Cmd::IfTreeChop(Dir::Down)),
            Ok(Event::Checked {
                at: (0, 1),
                tree: false
            }),
            "checking grass is fine"
        );
        assert_eq!(
            world.act(Cmd::IfTreeChop(Dir::Right)),
            Ok(Event::Checked {
                at: (1, 0),
                tree: true
            })
        );
        assert!(
            world.act(Cmd::Move(Dir::Right)).is_ok(),
            "stumps are passable"
        );
        assert!(world.act(Cmd::Chop(Dir::Down)).is_err(), "can't chop rocks");
    }
}
