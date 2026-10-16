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

/// One slot in a program strip: a block, one end of a loop, or a call to a
/// function. Blocks between a `LoopStart` and the next `LoopEnd` (or the end
/// of the strip, if the loop is still open) run `times` times over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    Block(Block),
    LoopStart {
        times: u8,
    },
    LoopEnd,
    /// Run function strip `n` (1 or 2). Only allowed in the main strip, so a
    /// function can never call itself forever.
    Call(usize),
}

/// A slot in the program: (strip, index). Strip 0 is the main program, and
/// strips 1 and 2 are the functions.
pub type Loc = (usize, usize);

/// One action in a running program, with what to show while it happens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Exec {
    pub cmd: Cmd,
    /// Which program slot this came from.
    pub at: Loc,
    /// How many runs of that block are left, counting this one.
    pub left: u8,
    /// The main-strip loop this is inside (its `LoopStart` slot), and how
    /// many runs of it are left.
    pub main_loop: Option<(usize, u8)>,
    /// The same, for a loop inside a function.
    pub fn_loop: Option<(usize, u8)>,
    /// The main-strip call this came from, when running a function.
    pub call: Option<usize>,
}

/// Where in the main strip a function was called from.
#[derive(Clone, Copy)]
struct Context {
    call: Option<usize>,
    main_loop: Option<(usize, u8)>,
}

/// Unroll a program (the main strip, then any functions) into the actions it
/// performs, in order.
pub fn expand(strips: &[Vec<Item>]) -> Vec<Exec> {
    let mut out = Vec::new();
    if !strips.is_empty() {
        let top = Context {
            call: None,
            main_loop: None,
        };
        run_strip(strips, 0, top, &mut out);
    }
    out
}

fn run_strip(strips: &[Vec<Item>], strip: usize, ctx: Context, out: &mut Vec<Exec>) {
    let items = &strips[strip];
    let mut i = 0;
    while i < items.len() {
        if let Item::LoopStart { times } = items[i] {
            let end = (i + 1..items.len())
                .find(|&j| items[j] == Item::LoopEnd)
                .unwrap_or(items.len());
            for k in 0..times {
                for j in i + 1..end {
                    run_item(strips, (strip, j), Some((i, times - k)), ctx, out);
                }
            }
            i = end + 1;
        } else {
            run_item(strips, (strip, i), None, ctx, out);
            i += 1;
        }
    }
}

fn run_item(
    strips: &[Vec<Item>],
    (strip, i): Loc,
    in_loop: Option<(usize, u8)>,
    ctx: Context,
    out: &mut Vec<Exec>,
) {
    match strips[strip][i] {
        Item::Block(b) => {
            for r in 0..b.times {
                out.push(Exec {
                    cmd: b.cmd,
                    at: (strip, i),
                    left: b.times - r,
                    main_loop: if strip == 0 { in_loop } else { ctx.main_loop },
                    fn_loop: if strip == 0 { None } else { in_loop },
                    call: ctx.call,
                });
            }
        }
        Item::Call(f) if strip == 0 && f < strips.len() => {
            let inner = Context {
                call: Some(i),
                main_loop: in_loop,
            };
            run_strip(strips, f, inner, out);
        }
        _ => {}
    }
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
    /// How many function strips (0, 1 or 2), and how many slots each has.
    functions: usize,
    fn_slots: usize,
}

const fn level(map: &'static [&'static str]) -> LevelDef {
    LevelDef {
        map,
        slots: MAX_PROGRAM,
        chop: false,
        repeat: false,
        check: false,
        loops: false,
        functions: 0,
        fn_slots: 0,
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

    /// Add `functions` function strips of `fn_slots` each (plus loops), and
    /// limit the main program to `slots`, so functions are needed to fit.
    const fn with_functions(mut self, slots: usize, functions: usize, fn_slots: usize) -> Self {
        self.functions = functions;
        self.fn_slots = fn_slots;
        self.with_loops(slots)
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
    // --- Functions: a strip of blocks with a shape for a name, which the
    // main program can run (call) as often as it likes ---
    level(&["@*@@*@*", "@.@@.@.", "R......"]).with_functions(6, 1, 3),
    level(&[
        "@@@@@@@*", "@@@@@@..", "@@@@@..@", "@@@...@@", "@@..@@@@", "@..@@@@@", "R.@@@@@@",
    ])
    .with_functions(3, 1, 4),
    level(&["@*@*@*", "@.@.@.", "R#.#.#"])
        .with_axe()
        .with_functions(4, 1, 5),
    level(&["*@@*@*", ".@@.@.", "R.##.#"])
        .with_axe()
        .with_functions(6, 2, 3),
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
    pub functions: usize,
    pub fn_slots: usize,
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
            functions: def.functions,
            fn_slots: def.fn_slots,
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

    /// Known solutions for the function levels: the main strip, then each
    /// function after a `|`. `f1` calls the first function.
    const FUNCTION_SOLUTIONS: &[(usize, &str)] = &[
        (41, "R f1 R3 f1 R2 f1 | U2 D2"),
        (42, "f1 R f1 | [ R U ]3"),
        (43, "[ f1 ]3 | cR R U2 D2 R"),
        (44, "f1 f2 f2 f1 f2 f1 | U2 D2 R | cR R"),
    ];

    /// A whole program: strips separated by `|`.
    fn parse_strips(text: &str) -> Vec<Vec<Item>> {
        text.split('|').map(parse_program).collect()
    }

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
                (Some('f'), None) => Item::Call(times as usize),
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
    fn always_wins(level: &Level, strips: &[Vec<Item>]) -> bool {
        let trace = expand(strips);
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
            .filter(|&n| levels[n - 1].loops && levels[n - 1].functions == 0)
            .collect();
        let solved: Vec<usize> = LOOP_SOLUTIONS.iter().map(|&(n, _)| n).collect();
        assert_eq!(loop_levels, solved, "every loop level needs a solution");
        for &(n, text) in LOOP_SOLUTIONS {
            let level = &levels[n - 1];
            let program = parse_program(text);
            assert!(program.len() <= level.slots, "level {n} solution too long");
            assert!(always_wins(level, &[program]), "level {n} solution fails");
        }
    }

    #[test]
    fn function_levels_have_a_solution_that_fits() {
        let levels = all_levels();
        let function_levels: Vec<usize> = (1..=levels.len())
            .filter(|&n| levels[n - 1].functions > 0)
            .collect();
        let solved: Vec<usize> = FUNCTION_SOLUTIONS.iter().map(|&(n, _)| n).collect();
        assert_eq!(
            function_levels, solved,
            "every function level needs a solution"
        );
        for &(n, text) in FUNCTION_SOLUTIONS {
            let level = &levels[n - 1];
            let strips = parse_strips(text);
            assert_eq!(
                strips.len(),
                1 + level.functions,
                "level {n}: wrong strip count"
            );
            assert!(strips[0].len() <= level.slots, "level {n}: main too long");
            for f in &strips[1..] {
                assert!(f.len() <= level.fn_slots, "level {n}: function too long");
                assert!(
                    !f.iter().any(|i| matches!(i, Item::Call(_))),
                    "level {n}: functions can't call functions"
                );
            }
            assert!(always_wins(level, &strips), "level {n} solution fails");
        }
    }

    enum Status {
        Fail,
        Alive,
        Win,
    }

    /// Run a single-strip program in every outcome.
    fn status(level: &Level, items: &[Item]) -> Status {
        let trace = expand(&[items.to_vec()]);
        let mut all_won = true;
        for outcome in level.outcomes() {
            let mut world = level.revealed(&outcome);
            let mut won = false;
            for exec in &trace {
                if world.act(exec.cmd).is_err() {
                    return Status::Fail;
                }
                if world.stars.is_empty() {
                    won = true;
                    break;
                }
            }
            all_won &= won;
        }
        if all_won { Status::Win } else { Status::Alive }
    }

    /// Can the level be won in `slots` with blocks and (un-nested) loops, but
    /// no functions? A depth-first search over programs that gives up on any
    /// program whose first run already bumps into something.
    fn fits_with_loops(level: &Level, slots: usize) -> bool {
        let mut blocks = Vec::new();
        for dir in Dir::ALL {
            let mut cmds = vec![Cmd::Move(dir)];
            if level.chop {
                cmds.push(Cmd::Chop(dir));
            }
            if level.check {
                cmds.push(Cmd::IfTreeChop(dir));
            }
            for cmd in cmds {
                for times in 1..=MAX_TIMES {
                    blocks.push(Item::Block(Block { cmd, times }));
                }
            }
        }

        fn search(
            level: &Level,
            blocks: &[Item],
            items: &mut Vec<Item>,
            open: Option<usize>,
            slots: usize,
        ) -> bool {
            // A loop still open at the end runs to the end of the program:
            // try every count. (While searching, it runs once.)
            if let Some(start) = open {
                for k in 2..=MAX_TIMES {
                    items[start] = Item::LoopStart { times: k };
                    if matches!(status(level, items), Status::Win) {
                        return true;
                    }
                }
                items[start] = Item::LoopStart { times: 1 };
            }
            if items.len() == slots {
                return false;
            }
            let try_push = |items: &mut Vec<Item>, item: Item, open: Option<usize>| {
                items.push(item);
                let found = match status(level, items) {
                    Status::Win => true,
                    Status::Fail => false,
                    Status::Alive => search(level, blocks, items, open, slots),
                };
                items.pop();
                found
            };
            for &block in blocks {
                if try_push(items, block, open) {
                    return true;
                }
            }
            match open {
                None => try_push(items, Item::LoopStart { times: 1 }, Some(items.len())),
                Some(start) if items.len() > start + 1 => {
                    items.push(Item::LoopEnd);
                    for k in 2..=MAX_TIMES {
                        items[start] = Item::LoopStart { times: k };
                        let found = match status(level, items) {
                            Status::Win => true,
                            Status::Fail => false,
                            Status::Alive => search(level, blocks, items, None, slots),
                        };
                        if found {
                            return true;
                        }
                    }
                    items[start] = Item::LoopStart { times: 1 };
                    items.pop();
                    false
                }
                Some(_) => false,
            }
        }

        search(level, &blocks, &mut Vec::new(), None, slots)
    }

    #[test]
    fn function_levels_need_a_function() {
        for (i, level) in all_levels().iter().enumerate() {
            if level.functions > 0 {
                assert!(
                    !fits_with_loops(level, level.slots),
                    "level {} fits in {} slots with loops alone",
                    i + 1,
                    level.slots
                );
            }
        }
    }

    #[test]
    fn loop_search_finds_loop_solutions() {
        // Sanity check for `fits_with_loops`: it finds the loop levels' answers.
        let levels = all_levels();
        for &(n, _) in LOOP_SOLUTIONS {
            assert!(
                fits_with_loops(&levels[n - 1], levels[n - 1].slots),
                "level {n}"
            );
        }
    }

    #[test]
    fn functions_unroll_where_they_are_called() {
        let strips = parse_strips("U [ f1 ]2 | R [ D ]2");
        let trace = expand(&strips);
        let cmds: Vec<String> = trace.iter().map(|e| format!("{:?}", e.cmd)).collect();
        assert_eq!(
            cmds,
            [
                "Move(Up)",
                "Move(Right)",
                "Move(Down)",
                "Move(Down)",
                "Move(Right)",
                "Move(Down)",
                "Move(Down)"
            ]
        );
        assert_eq!(trace[1].at, (1, 0));
        assert_eq!(trace[1].call, Some(2));
        assert_eq!(trace[1].main_loop, Some((1, 2)));
        assert_eq!(trace[2].fn_loop, Some((1, 2)));
        assert_eq!(trace[5].main_loop, Some((1, 1)));
        // Calls inside a function are ignored, so nothing can loop forever.
        assert_eq!(expand(&parse_strips("f1 | R f1")).len(), 1);
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
        let trace = expand(&[parse_program("U [ R2 D ]2 L")]);
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
        assert_eq!(trace[1].main_loop, Some((1, 2)));
        assert_eq!(trace[4].main_loop, Some((1, 1)));
        assert_eq!(trace[4].left, 2);
        // An unclosed loop runs to the end of the program.
        assert_eq!(expand(&[parse_program("[ R ]3")]).len(), 3);
        let open = parse_program("[ R ]3")[..2].to_vec();
        assert_eq!(expand(&[open]).len(), 3);
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
