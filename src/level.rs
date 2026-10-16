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

/// One slot in the program: a block, or one end of a loop. Blocks between a
/// `LoopStart` and the next `LoopEnd` (or the end of the program, if the
/// loop is still open) run `times` times over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    Block(Block),
    LoopStart { times: u8 },
    LoopEnd,
}

/// One action in a running program, with what to show while it happens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Exec {
    pub cmd: Cmd,
    /// Which program slot this came from.
    pub item: usize,
    /// How many runs of that block are left, counting this one.
    pub left: u8,
    /// The loop this is inside (its `LoopStart` slot), and runs of it left.
    pub in_loop: Option<(usize, u8)>,
}

/// Unroll a program into the actions it performs, in order.
pub fn expand(items: &[Item]) -> Vec<Exec> {
    let mut out = Vec::new();
    let block_runs = |out: &mut Vec<Exec>, i: usize, in_loop| {
        if let Item::Block(b) = items[i] {
            for r in 0..b.times {
                out.push(Exec {
                    cmd: b.cmd,
                    item: i,
                    left: b.times - r,
                    in_loop,
                });
            }
        }
    };
    let mut i = 0;
    while i < items.len() {
        if let Item::LoopStart { times } = items[i] {
            let end = (i + 1..items.len())
                .find(|&j| items[j] == Item::LoopEnd)
                .unwrap_or(items.len());
            for k in 0..times {
                for j in i + 1..end {
                    block_runs(&mut out, j, Some((i, times - k)));
                }
            }
            i = end + 1;
        } else {
            block_runs(&mut out, i, None);
            i += 1;
        }
    }
    out
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
    loops: bool,
}

const fn level(map: &'static [&'static str]) -> LevelDef {
    LevelDef {
        map,
        slots: MAX_PROGRAM,
        chop: false,
        repeat: false,
        check: false,
        loops: false,
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

    /// Unlock loops (and repeat, which sets how many times a loop runs), and
    /// limit the program to `slots`, so a loop is the only way to fit.
    const fn with_loops(mut self, slots: usize) -> Self {
        self.loops = true;
        self.with_repeat(slots)
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
    // --- Loops: repeat a whole group of blocks ---
    level(&[
        "R.@@@@@", "@..@@@@", "@@..@@@", "@@@..@@", "@@@@..@", "@@@@@.*",
    ])
    .with_loops(5),
    level(&["R..@@@@@@", "@@...@@@@", "@@@@...@@", "@@@@@@..*"]).with_loops(5),
    level(&["...@...@..*", ".@.@.@.@.@@", "R@...@...@@"]).with_loops(8),
    level(&["R#.#.#.#*"]).with_axe().with_loops(4),
    level(&["R????*"]).with_check().with_loops(4),
    level(&["R?@@@@", "@.?@@@", "@@.?@@", "@@@.?@", "@@@@.*"])
        .with_check()
        .with_loops(6),
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
    pub loops: bool,
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
            loops: def.loops,
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

    /// Known solutions for the loop levels (too many loop programs to search).
    /// `R` move right, `cR` chop right, `?R` check right, a number repeats a
    /// block, `[` opens a loop and `]4` closes it, running it 4 times.
    const LOOP_SOLUTIONS: &[(usize, &str)] = &[
        (35, "[ R D ]5 R"),
        (36, "[ R2 D ]3 R2"),
        (37, "[ U2 R2 D2 R2 ]2 U2 R2"),
        (38, "[ cR R2 ]4"),
        (39, "[ ?R R ]5"),
        (40, "[ ?R R D ]4 R"),
    ];

    fn parse_program(text: &str) -> Vec<Item> {
        let dir = |c: char| match c {
            'U' => Dir::Up,
            'D' => Dir::Down,
            'L' => Dir::Left,
            'R' => Dir::Right,
            _ => panic!("bad direction {c}"),
        };
        let mut items = Vec::new();
        let mut open = None;
        for token in text.split_whitespace() {
            let digits = token.trim_start_matches(|c: char| !c.is_ascii_digit());
            let times: u8 = digits.parse().unwrap_or(1);
            let head = &token[..token.len() - digits.len()];
            let mut chars = head.chars();
            let item = match (chars.next(), chars.next()) {
                (Some('['), None) => {
                    open = Some(items.len());
                    Item::LoopStart { times: 1 }
                }
                (Some(']'), None) => {
                    items[open.take().expect("] without [")] = Item::LoopStart { times };
                    Item::LoopEnd
                }
                (Some('c'), Some(d)) => Item::Block(Block {
                    cmd: Cmd::Chop(dir(d)),
                    times,
                }),
                (Some('?'), Some(d)) => Item::Block(Block {
                    cmd: Cmd::IfTreeChop(dir(d)),
                    times,
                }),
                (Some(d), None) => Item::Block(Block {
                    cmd: Cmd::Move(dir(d)),
                    times,
                }),
                _ => panic!("bad token {token}"),
            };
            items.push(item);
        }
        items
    }

    /// Does this program win however the mysteries turn out?
    fn always_wins(level: &Level, program: &[Item]) -> bool {
        let trace = expand(program);
        level.outcomes().iter().all(|outcome| {
            let mut world = level.revealed(outcome);
            for exec in &trace {
                if world.act(exec.cmd).is_err() {
                    return false;
                }
                if world.stars.is_empty() {
                    return true;
                }
            }
            false
        })
    }

    #[test]
    fn loop_levels_have_a_solution_that_fits() {
        let levels = all_levels();
        let loop_levels: Vec<usize> = (1..=levels.len())
            .filter(|&n| levels[n - 1].loops)
            .collect();
        let solved: Vec<usize> = LOOP_SOLUTIONS.iter().map(|&(n, _)| n).collect();
        assert_eq!(loop_levels, solved, "every loop level needs a solution");
        for &(n, text) in LOOP_SOLUTIONS {
            let level = &levels[n - 1];
            let program = parse_program(text);
            assert!(program.len() <= level.slots, "level {n} solution too long");
            assert!(always_wins(level, &program), "level {n} solution fails");
        }
    }

    #[test]
    fn loop_levels_need_a_loop() {
        for (i, level) in all_levels().iter().enumerate() {
            if level.loops {
                let without = fewest_blocks(level, Allow::level(level)).unwrap_or(usize::MAX);
                assert!(
                    without > level.slots,
                    "level {} fits in {without} blocks without a loop",
                    i + 1
                );
            }
        }
    }

    #[test]
    fn loops_unroll_in_order() {
        let trace = expand(&parse_program("U [ R2 D ]2 L"));
        let cmds: Vec<String> = trace.iter().map(|e| format!("{:?}", e.cmd)).collect();
        assert_eq!(
            cmds,
            [
                "Move(Up)",
                "Move(Right)",
                "Move(Right)",
                "Move(Down)",
                "Move(Right)",
                "Move(Right)",
                "Move(Down)",
                "Move(Left)"
            ]
        );
        assert_eq!(trace[1].in_loop, Some((1, 2)));
        assert_eq!(trace[4].in_loop, Some((1, 1)));
        assert_eq!(trace[4].left, 2);
        // An unclosed loop runs to the end of the program.
        assert_eq!(expand(&parse_program("[ R ]3")).len(), 3);
        let open = &parse_program("[ R ]3")[..2];
        assert_eq!(expand(open).len(), 3);
    }

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
            // Loop levels only fit with a loop; their known solutions are
            // checked in `loop_levels_have_a_solution_that_fits`.
            if !level.loops {
                assert!(blocks <= level.slots, "level {n} needs {blocks} blocks");
            }
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
