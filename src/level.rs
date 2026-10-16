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
    /// Slippery: the robot slides on until something stops it.
    Ice,
    /// Locked: the robot needs a key to get through. The key opens it for good.
    Door,
    /// A door that has been unlocked. The robot can drive through it.
    Open,
}

impl Tile {
    /// Can the robot drive onto this tile (without unlocking anything)?
    fn passable(self) -> bool {
        matches!(self, Tile::Grass | Tile::Stump | Tile::Ice | Tile::Open)
    }
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

/// What an "if" block looks for, next to the robot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sense {
    Tree,
    Star,
}

/// One slot in a program strip: a block, one end of a loop or an "if", or a
/// call to a function.
///
/// Blocks between a `LoopStart` and the next `LoopEnd` (or the end of the
/// strip, if the loop is still open) run `times` times over. Blocks between an
/// `IfStart` and the next `IfEnd` (or the end of the loop or strip it's in)
/// only run if the robot sees `sense` next to it in direction `dir`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    Block(Block),
    LoopStart {
        times: u8,
    },
    LoopEnd,
    IfStart {
        sense: Sense,
        dir: Dir,
    },
    IfEnd,
    /// Run function strip `n` (1 or 2). Only allowed in the main strip, so a
    /// function can never call itself forever.
    Call(usize),
    /// An empty slot opened up in the middle of a program, waiting for a
    /// block. Running skips it.
    Gap,
}

/// How many tiles a strip uses. Loop brackets and the end of an "if" sit
/// between tiles; blocks, calls and the start of an "if" (which shows what
/// it's looking for) each take a tile.
pub fn tiles(items: &[Item]) -> usize {
    items
        .iter()
        .filter(|i| {
            matches!(
                i,
                Item::Block(_) | Item::Call(_) | Item::IfStart { .. } | Item::Gap
            )
        })
        .count()
}

/// A slot in the program: (strip, index). Strip 0 is the main program, and
/// strips 1 and 2 are the functions.
pub type Loc = (usize, usize);

/// One step of a running program.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    /// Do an instruction.
    Do(Cmd),
    /// Look in direction `dir` for `sense`: `yes` if it's there.
    Test { sense: Sense, dir: Dir, yes: bool },
}

/// One step in a running program, with what to show while it happens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Exec {
    pub step: Step,
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

/// Runs a program on a copy of the world, recording each step. What an "if"
/// does depends on what the robot finds, so the program has to be run to be
/// unrolled.
struct Runner<'a> {
    strips: &'a [Vec<Item>],
    world: World,
    out: Vec<Exec>,
    /// The robot got stuck, or collected every star: nothing more happens.
    stopped: bool,
}

/// Unroll a program (the main strip, then any functions) into the steps it
/// takes when run in `world`, in order. It stops after a step that gets the
/// robot stuck, or that collects the last star, just as the game does.
pub fn expand(strips: &[Vec<Item>], world: &World) -> Vec<Exec> {
    let mut runner = Runner {
        strips,
        world: world.clone(),
        out: Vec::new(),
        stopped: false,
    };
    if !strips.is_empty() {
        let top = Context {
            call: None,
            main_loop: None,
        };
        runner.run(0, 0, strips[0].len(), None, top);
    }
    runner.out
}

impl Runner<'_> {
    fn emit(&mut self, step: Step, at: Loc, left: u8, in_loop: Option<(usize, u8)>, ctx: Context) {
        if self.stopped {
            return;
        }
        let main = at.0 == 0;
        self.out.push(Exec {
            step,
            at,
            left,
            main_loop: if main { in_loop } else { ctx.main_loop },
            fn_loop: if main { None } else { in_loop },
            call: ctx.call,
        });
        if let Step::Do(cmd) = step {
            let stuck = self.world.act(cmd).is_err();
            self.stopped = stuck || self.world.stars.is_empty();
        }
    }

    /// Run slots `from..to` of a strip.
    fn run(
        &mut self,
        strip: usize,
        from: usize,
        to: usize,
        in_loop: Option<(usize, u8)>,
        ctx: Context,
    ) {
        let items = &self.strips[strip];
        let mut i = from;
        while i < to && !self.stopped {
            match items[i] {
                Item::LoopStart { times } => {
                    let end = (i + 1..to)
                        .find(|&j| items[j] == Item::LoopEnd)
                        .unwrap_or(to);
                    for k in 0..times {
                        self.run(strip, i + 1, end, Some((i, times - k)), ctx);
                    }
                    i = end + 1;
                }
                Item::IfStart { sense, dir } => {
                    let end = (i + 1..to).find(|&j| items[j] == Item::IfEnd).unwrap_or(to);
                    let yes = self.world.sees(sense, dir);
                    self.emit(Step::Test { sense, dir, yes }, (strip, i), 1, in_loop, ctx);
                    if yes {
                        self.run(strip, i + 1, end, in_loop, ctx);
                    }
                    i = end + 1;
                }
                Item::Block(b) => {
                    for r in 0..b.times {
                        self.emit(Step::Do(b.cmd), (strip, i), b.times - r, in_loop, ctx);
                    }
                    i += 1;
                }
                Item::Call(f) if strip == 0 && f < self.strips.len() => {
                    let inner = Context {
                        call: Some(i),
                        main_loop: in_loop,
                    };
                    self.run(f, 0, self.strips[f].len(), None, inner);
                    i += 1;
                }
                Item::Call(_) | Item::LoopEnd | Item::IfEnd | Item::Gap => i += 1,
            }
        }
    }
}

/// A whole program written as text: strips separated by `|`. Within a strip,
/// `R` moves right, `cR` chops right, `?R` checks right, and a number repeats
/// a block. `[` opens a loop and `]4` closes it, running it 4 times. `{tR`
/// starts "if there's a tree to the right", `{sU` "if there's a star above",
/// and `}` ends it. `f1` calls the first function.
pub fn parse_strips(text: &str) -> Vec<Vec<Item>> {
    text.split('|').map(parse_program).collect()
}

pub fn parse_program(text: &str) -> Vec<Item> {
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
            (Some('}'), None) => Item::IfEnd,
            (Some('{'), Some(what)) => Item::IfStart {
                sense: if what == 't' {
                    Sense::Tree
                } else {
                    Sense::Star
                },
                dir: dir(chars.next().expect("{ needs a direction")),
            },
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

pub struct LevelDef {
    /// Legend: `.` grass, `#` tree, `@` rock, `?` mystery (tree or grass),
    /// `~` ice, `k` key, `D` locked door, `R` robot start, `*` star.
    map: &'static [&'static str],
    /// How many blocks fit in the program for this level.
    slots: usize,
    chop: bool,
    repeat: bool,
    check: bool,
    loops: bool,
    /// Which "if" blocks the level has: looking for trees, and for stars.
    if_tree: bool,
    if_star: bool,
    /// How many function strips (0, 1 or 2), and how many slots each has.
    functions: usize,
    fn_slots: usize,
    /// A demo level comes with its program already written, to show how a
    /// new block is used: just press play.
    demo: Option<&'static str>,
}

const fn level(map: &'static [&'static str]) -> LevelDef {
    LevelDef {
        map,
        slots: MAX_PROGRAM,
        chop: false,
        repeat: false,
        check: false,
        loops: false,
        if_tree: false,
        if_star: false,
        functions: 0,
        fn_slots: 0,
        demo: None,
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

    /// Unlock "if" blocks (and loops) that look for trees and/or stars, and
    /// limit the program to `slots`, so an "if" in a loop is needed to fit.
    const fn with_ifs(mut self, slots: usize, tree: bool, star: bool) -> Self {
        self.if_tree = tree;
        self.if_star = star;
        self.with_loops(slots)
    }

    /// Add `functions` function strips of `fn_slots` each (plus loops), and
    /// limit the main program to `slots`, so functions are needed to fit.
    const fn with_functions(mut self, slots: usize, functions: usize, fn_slots: usize) -> Self {
        self.functions = functions;
        self.fn_slots = fn_slots;
        self.with_loops(slots)
    }

    /// Make this a demo, with `program` (see `parse_strips`) already in place.
    const fn demo(mut self, program: &'static str) -> Self {
        self.demo = Some(program);
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
    // Each new block is first shown in a demo, with the program already
    // written: press play and watch.
    // --- Moving ---
    level(&["R.@", "@.*"]).demo("R D R"),
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
    level(&["R", "#", "*"]).with_axe().demo("cD D D"),
    level(&["R#*"]).with_axe(),
    level(&["..@*", "R.#.", "..@."]).with_axe(),
    level(&["R@*", ".@.", "#@.", "..."]).with_axe(),
    level(&["R##.*"]).with_axe(),
    // --- Repeat: not enough slots without it ---
    level(&["R....", "@@@@.", "@@@@*"])
        .with_repeat(2)
        .demo("R4 D2"),
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
    level(&["R", "?", "*"]).with_check().demo("?D D D"),
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
    // --- Loops: repeat a whole group of blocks. First two familiar maps
    // (levels 7 and 11) with too few slots to do them the old way ---
    level(&["R@@@", "..@@", "@..@", "@@.*"])
        .with_loops(2)
        .demo("[ D R ]3"),
    level(&["R.##", "#..#", "##.*"]).with_loops(4),
    level(&["R#*#*", ".#.#.", "....."]).with_loops(5),
    level(&[
        "R.@@@@@", "@..@@@@", "@@..@@@", "@@@..@@", "@@@@..@", "@@@@@.*",
    ])
    .with_loops(5),
    level(&["R..@@@@@@", "@@...@@@@", "@@@@...@@", "@@@@@@..*"]).with_loops(5),
    level(&["...@...@..*", ".@.@.@.@.@@", "R@...@...@@"]).with_loops(8),
    level(&["R#.#.#.#*"]).with_axe().with_loops(4),
    // --- If: look before you act. Inside a loop, one rule handles whatever
    // the robot finds, wherever it finds it ---
    level(&["R.##..#*"])
        .with_axe()
        .with_ifs(3, true, false)
        .demo("[ {tR cR } R ]7"),
    level(&["R#..#.##.*"]).with_axe().with_ifs(3, true, false),
    level(&["@**@*", "R...."])
        .with_ifs(4, false, true)
        .demo("[ {sU U D } R ]5"),
    level(&["@*@@*@**@", "R........"]).with_ifs(4, false, true),
    level(&["@@**@@@*@", "R#..#.#.."])
        .with_axe()
        .with_ifs(6, true, true),
    // --- Functions: a strip of blocks with a shape for a name, which the
    // main program can run (call) as often as it likes. First a familiar map
    // (the zig-zag from the loop levels) ---
    level(&["*@@*@*", ".@@.@.", "R....."])
        .with_functions(5, 1, 2)
        .demo("f1 R3 f1 R2 f1 | U2 D2"),
    level(&["...@...@..*", ".@.@.@.@.@@", "R@...@...@@"]).with_functions(3, 1, 4),
    level(&["@*@*@@@*", "@.@.@@@.", "R......."]).with_functions(5, 1, 3),
    level(&[
        "@@@@@@@*", "@@@@@@..", "@@@@@..@", "@@@...@@", "@@..@@@@", "@..@@@@@", "R.@@@@@@",
    ])
    .with_functions(3, 1, 4),
    level(&["@*@*@*", "@.@.@.", "R#.#.#"])
        .with_axe()
        .with_functions(4, 1, 5),
    level(&["*@*@*", ".@.@.", "R#.#."])
        .with_axe()
        .with_functions(5, 2, 2)
        .demo("f1 f2 f1 f2 f1 | U2 D2 | cR R2"),
    level(&["*@@*@*", ".@@.@.", "R.##.#"])
        .with_axe()
        .with_functions(6, 2, 3),
    // --- Loops through the mystery clouds: check, then step ---
    level(&["R?.?.?.?*"]).with_check().with_loops(3),
    level(&["R?@@@", "@.?@@", "@@.?@", "@@@.?", "@@@@*"])
        .with_check()
        .with_loops(3),
    // --- If, on paths that turn ---
    level(&["R#@@@", "@..@@", "@@#.@", "@@@*#", "@@@@*"])
        .with_axe()
        .with_ifs(6, true, false),
    level(&[
        "R..@@@@@@",
        "*@...@@@@",
        "@@*@...@@",
        "@@@@@@...",
        "@@@@@@*@.",
        "@@@@@@@@*",
    ])
    .with_ifs(5, false, true),
    // --- Two functions, and functions with the axe and the clouds ---
    level(&[
        "@@@.*@@@@.*@@",
        "@@....@@....@",
        "#..##....##..",
        "R.####*.####*",
    ])
    .with_functions(4, 2, 3),
    level(&["R####", "@@@@*", "####.", "*@@@@", ".####", "@@@@*"])
        .with_axe()
        .with_functions(6, 2, 3),
    level(&[
        "R?...@@@@@@@@",
        "@@@@.@@@@@@@@",
        "@@@@..?*.@@@@",
        "@@@@@@@@.@@@@",
        "@@@@@@@@...?*",
    ])
    .with_check()
    .with_functions(3, 1, 3),
    // --- Functions with "if" blocks inside: one function looks around, and
    // the main program says where ---
    level(&["@**@*@", "R.....", "*@@*@.", "@@*@*.", "......", "@*@*@@"])
        .with_ifs(5, false, true)
        .with_functions(5, 1, 6),
    level(&[
        "@*@*@@@@@@@@@",
        "R....@@@@@@@@",
        "@@@@.@**@@@@@",
        "@@@@.....@@@@",
        "@@@@@@@@.*@*@",
        "@@@@@@@@....*",
    ])
    .with_ifs(5, false, true)
    .with_functions(5, 1, 4),
    level(&[
        "R.#*@@@@", "@@@#@@@@", "@@@..#@@", "@@@@@.@@", "@@@@@#@@", "@@@@@*#*",
    ])
    .with_axe()
    .with_ifs(5, true, false)
    .with_functions(5, 2, 3),
    level(&["@*@**@*", "R.#.##.", "@@@@@@.", ".#.#...", "*@*@@*@"])
        .with_axe()
        .with_ifs(3, true, true)
        .with_functions(3, 2, 6),
    // --- Everything together ---
    level(&[
        "@@@@@@@..##.",
        "@@@@@@@.@*@*",
        "@@@..#.#@@@@",
        "@@@.@@*@@@@@",
        "R.#.@@@@@@@@",
        "@*@*@@@@@@@@",
    ])
    .with_axe()
    .with_ifs(5, true, true)
    .with_functions(5, 2, 6),
    // --- Ice: the robot slides until something stops it (grass, a rock, or
    // the edge), so think ahead about where it will end up ---
    level(&["R~~~@", "@@@~@", "@@@*@"]).demo("R D"),
    level(&["R~~.~~*"]),
    level(&["R~~~~@", "@@@@~@", "*~~~~."]),
    // The star is right under the ice, but the robot slides past it.
    level(&["R~~~~~.", "@@@*@@.", "@@@~@@.", "@@@~..."]),
    level(&["R~@~~", "~~~~~", "~~~~*", "~~~@~"]),
    level(&["~~~~~", "@@~*~", "~~~~~", "~R@~~"]),
    level(&["~~@~~", "*~~@~", "@~~~~", "R~~~@"]),
    // --- Keys: pick one up to open a locked door. Fetch the key first, even
    // when it's the other way ---
    level(&["R.kD*"]).demo("R R R R"),
    level(&["kR.D*"]),
    level(&["k@*", ".@D", "R.."]),
    level(&["R#D*", ".@@@", "k@@@"]).with_axe(),
    // Two doors need two keys.
    level(&["R.D.D*", ".@@@@@", "k@@@@@", "k@@@@@"]).with_repeat(3),
    level(&["R~~~k", "@@@@D", "*~~~."]),
    // A locked door stops the robot sliding, even with the key: it has to
    // step onto the door to open it.
    level(&["*D~~~R", "@@@@@~", "k~~~~~"]),
    level(&["@k@@k@@k@@", "R.D..D..D*"]).with_loops(4),
];

#[derive(Clone)]
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
    pub if_tree: bool,
    pub if_star: bool,
    pub functions: usize,
    pub fn_slots: usize,
    /// Where the mystery clouds are.
    pub mysteries: Vec<Pos>,
    /// Where the keys lie.
    pub keys: Vec<Pos>,
    /// The level's number on the map. Demos aren't numbered: they share the
    /// number of the level they come before.
    pub number: usize,
    /// A demo's program, already written.
    pub demo: Option<Vec<Vec<Item>>>,
}

impl Level {
    pub fn parse(def: &LevelDef) -> Level {
        let mut level = Level::from_rows(def.map);
        level.slots = def.slots;
        level.chop = def.chop;
        level.repeat = def.repeat;
        level.check = def.check;
        level.loops = def.loops;
        level.if_tree = def.if_tree;
        level.if_star = def.if_star;
        level.functions = def.functions;
        level.fn_slots = def.fn_slots;
        level.demo = def.demo.map(parse_strips);
        level
    }

    /// A level from its map (see `LevelDef::map`), with just the arrows.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Level {
        let h = rows.len() as i32;
        let w = rows[0].as_ref().len() as i32;
        let mut tiles = Vec::new();
        let mut start = None;
        let mut stars = Vec::new();
        let mut mysteries = Vec::new();
        let mut keys = Vec::new();
        for (y, row) in rows.iter().enumerate() {
            let row = row.as_ref();
            assert_eq!(row.len() as i32, w, "level rows must be the same length");
            for (x, ch) in row.chars().enumerate() {
                let p = (x as i32, y as i32);
                tiles.push(match ch {
                    '#' => Tile::Tree,
                    '@' => Tile::Rock,
                    '?' => Tile::Mystery,
                    '~' => Tile::Ice,
                    'D' => Tile::Door,
                    _ => Tile::Grass,
                });
                match ch {
                    'R' => start = Some(p),
                    '*' => stars.push(p),
                    '?' => mysteries.push(p),
                    'k' => keys.push(p),
                    '.' | '#' | '@' | '~' | 'D' => {}
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
            slots: MAX_PROGRAM,
            chop: false,
            repeat: false,
            check: false,
            loops: false,
            if_tree: false,
            if_star: false,
            functions: 0,
            fn_slots: 0,
            mysteries,
            keys,
            number: 0,
            demo: None,
        }
    }

    /// What the save file calls this level: its number, or `d` and the
    /// number for a demo. Numbers stay the same when demos are added.
    pub fn key(&self) -> String {
        match self.demo {
            Some(_) => format!("d{}", self.number),
            None => self.number.to_string(),
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
            keys: self.keys.clone(),
            held: 0,
        }
    }

    /// Does the map have any of this tile?
    pub fn has(&self, tile: Tile) -> bool {
        self.tiles.contains(&tile)
    }

    /// One way the mysteries can turn out (`true` = tree), with `coin`
    /// tossed for each. With two or more there's always at least one tree and
    /// one patch of grass, so a program that guesses instead of checking
    /// can't get lucky every time: if they all come out the same, toss again.
    pub fn random_outcome(&self, mut coin: impl FnMut() -> bool) -> Vec<bool> {
        let n = self.mysteries.len();
        loop {
            let outcome: Vec<bool> = (0..n).map(|_| coin()).collect();
            if n < 2 || (outcome.contains(&true) && outcome.contains(&false)) {
                return outcome;
            }
        }
    }

    /// Every way the mysteries can turn out, by the same rule as
    /// `random_outcome`. That doubles with each cloud, so it's only for
    /// checking the game's own levels (which have a few).
    #[cfg(test)]
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
    /// Drove from `from` to where the robot is now: one tile, or further if
    /// it slid on ice. Anything picked up or unlocked is where it stopped.
    Moved {
        from: Pos,
        star: bool,
        key: bool,
        unlocked: bool,
    },
    Chopped(Pos),
    /// Looked at `at` for a tree (and chopped it if there was one).
    Checked {
        at: Pos,
        tree: bool,
    },
    /// An "if" looked at `at`: `yes` if it saw what it was looking for. (The
    /// game uses this for its animation; `World::act` never returns it.)
    Tested {
        at: Pos,
        yes: bool,
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
    /// Keys still lying on the ground, and how many the robot is carrying.
    pub keys: Vec<Pos>,
    pub held: u8,
}

impl World {
    /// Is there a `sense` (a tree, or a star) right next to the robot, in
    /// direction `dir`?
    pub fn sees(&self, sense: Sense, dir: Dir) -> bool {
        let (dx, dy) = dir.delta();
        let next = (self.pos.0 + dx, self.pos.1 + dy);
        match sense {
            Sense::Tree => self.tile(next) == Some(Tile::Tree),
            Sense::Star => self.stars.contains(&next),
        }
    }

    pub fn tile(&self, (x, y): Pos) -> Option<Tile> {
        let inside = x >= 0 && y >= 0 && x < self.w && y < self.h;
        inside.then(|| self.tiles[(y * self.w + x) as usize])
    }

    /// Can the robot drive onto `p`: an open tile, or a door it has a key for?
    fn can_enter(&self, p: Pos) -> bool {
        match self.tile(p) {
            Some(Tile::Door) => self.held > 0,
            Some(tile) => tile.passable(),
            None => false,
        }
    }

    /// Do one instruction. `Err` means the robot got stuck: it bumped into
    /// something, or swung the axe at something that isn't a tree.
    pub fn act(&mut self, cmd: Cmd) -> Result<Event, ()> {
        let (dx, dy) = cmd.dir().delta();
        let step = |(x, y): Pos| (x + dx, y + dy);
        let next = step(self.pos);
        match (cmd, self.tile(next)) {
            (Cmd::Move(_), _) if self.can_enter(next) => {
                let from = self.pos;
                let mut unlocked = false;
                // A key is only used by stepping onto a locked door.
                if self.tile(next) == Some(Tile::Door) {
                    self.held -= 1;
                    self.tiles[(next.1 * self.w + next.0) as usize] = Tile::Open;
                    unlocked = true;
                }
                self.pos = next;
                // On ice the robot keeps sliding until something stops it. A
                // locked door stops it too, key or no key. Sliding into
                // something isn't a bump: that's how it stops.
                while self.tile(self.pos) == Some(Tile::Ice)
                    && self.tile(step(self.pos)).is_some_and(Tile::passable)
                {
                    self.pos = step(self.pos);
                }
                let at = self.pos;
                let star = self.stars.iter().position(|&s| s == at);
                if let Some(i) = star {
                    self.stars.remove(i);
                }
                let key = self.keys.iter().position(|&k| k == at);
                if let Some(i) = key {
                    self.keys.remove(i);
                    self.held += 1;
                }
                Ok(Event::Moved {
                    from,
                    star: star.is_some(),
                    key: key.is_some(),
                    unlocked,
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
    let mut levels: Vec<Level> = LEVELS.iter().map(Level::parse).collect();
    let mut number = levels.iter().filter(|l| l.demo.is_none()).count() + 1;
    for level in levels.iter_mut().rev() {
        if level.demo.is_none() {
            number -= 1;
        }
        level.number = number;
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet, VecDeque};

    /// Known solutions for the loop levels (too many loop programs to search).
    /// `R` move right, `cR` chop right, `?R` check right, a number repeats a
    /// block, `[` opens a loop and `]4` closes it, running it 4 times.
    const LOOP_SOLUTIONS: &[(usize, &str)] = &[
        (35, "[ R D ]3"),
        (36, "[ D2 R2 U2 ]2"),
        (37, "[ R D ]5 R"),
        (38, "[ R2 D ]3 R2"),
        (39, "[ U2 R2 D2 R2 ]2 U2 R2"),
        (40, "[ cR R2 ]4"),
        (49, "[ ?R R2 ]4"),
        (50, "[ ?R R D ]4"),
        (73, "[ R U D R2 ]3"),
    ];

    /// Known solutions for the function levels: the main strip, then each
    /// function after a `|`. `f1` calls the first function.
    const FUNCTION_SOLUTIONS: &[(usize, &str)] = &[
        (44, "f1 f1 f1 | U2 R2 D2 R2"),
        (45, "f1 R f1 R3 f1 | R U2 D2"),
        (46, "f1 R f1 | [ R U ]3"),
        (47, "[ f1 ]3 | cR R U2 D2 R"),
        (48, "f1 f2 f2 f1 f2 f1 | U2 D2 R | cR R"),
        (53, "f1 f2 f1 f2 | [ R U ]3 | [ R D ]3"),
        (54, "f1 D2 f2 D2 f1 D | [ cR R ]4 | [ cL L ]4"),
        (55, "[ f1 D2 ]3 | [ ?R R ]4"),
        (56, "[ f1 R ]5 D3 [ f1 L ]5 | {sU U D } {sD D U }"),
        (57, "f1 D2 f1 D2 f1 | [ {sU U D } R ]4"),
        (
            58,
            "[ f1 ]3 [ f2 ]2 [ f1 ]2 [ f2 ]3 [ f1 ]2 | {tR cR } R | {tD cD } D",
        ),
        (
            59,
            "[ f1 ]6 D2 [ f2 ]6 | {tR cR } R {sU U D } | {tL cL } L {sD D U }",
        ),
        (60, "f1 f2 f1 f2 f1 | [ {tR cR } R {sD D U } ]3 | U2 R"),
    ];

    /// Known solutions for the "if" levels. `{tR` starts "if there's a tree to
    /// the right", `{sU` "if there's a star above", and `}` ends it.
    const IF_SOLUTIONS: &[(usize, &str)] = &[
        (41, "[ {tR cR } R ]9"),
        (42, "[ {sU U D } R ]8"),
        (43, "[ {tR cR } {sU U D } R ]8"),
        (51, "[ {tR cR } R {tD cD } D ]4"),
        (52, "[ {sD D U } R2 D ]5"),
    ];

    /// The levels with numbers, in order (so level `n` is at `n - 1`),
    /// leaving out the demos.
    fn numbered() -> Vec<Level> {
        all_levels()
            .into_iter()
            .filter(|l| l.demo.is_none())
            .collect()
    }

    #[test]
    fn demos_show_a_program_that_wins() {
        let levels = all_levels();
        for (i, level) in levels.iter().enumerate() {
            let Some(strips) = &level.demo else { continue };
            let n = level.number;
            let next = &levels[i + 1];
            assert!(
                next.demo.is_none() && next.number == n,
                "demo d{n} must come just before level {n}"
            );
            assert_eq!(strips.len(), 1 + level.functions, "demo d{n}: strips");
            assert!(tiles(&strips[0]) <= level.slots, "demo d{n}: too long");
            for f in &strips[1..] {
                assert!(tiles(f) <= level.fn_slots, "demo d{n}: function too long");
                assert!(
                    !f.iter().any(|i| matches!(i, Item::Call(_))),
                    "demo d{n}: functions can't call functions"
                );
            }
            assert_eq!(
                level.check,
                !level.mysteries.is_empty(),
                "demo d{n}: mysteries and the check block go together"
            );
            assert!(always_wins(level, strips), "demo d{n} doesn't win");
        }
    }

    #[test]
    fn levels_keep_their_numbers_and_keys() {
        let levels = all_levels();
        let keys: Vec<String> = levels.iter().map(Level::key).collect();
        assert_eq!(keys[..3], ["d1", "1", "2"]);
        let unique: HashSet<&String> = keys.iter().collect();
        assert_eq!(unique.len(), keys.len(), "save keys must be unique");
        let numbers: Vec<usize> = numbered().iter().map(|l| l.number).collect();
        assert_eq!(numbers, (1..=numbers.len()).collect::<Vec<_>>());
        // Levels already played keep their number: 13 is still the first
        // axe level, and 48 the first with two functions.
        let numbered = numbered();
        assert!(numbered[12].chop && (numbered[12].w, numbered[12].h) == (3, 1));
        assert_eq!(numbered[47].functions, 2);
    }

    /// Does this program win however the mysteries turn out?
    fn always_wins(level: &Level, strips: &[Vec<Item>]) -> bool {
        level.outcomes().iter().all(|outcome| {
            let mut world = level.revealed(outcome);
            for exec in expand(strips, &world.clone()) {
                if let Step::Do(cmd) = exec.step {
                    if world.act(cmd).is_err() {
                        return false;
                    }
                    if world.stars.is_empty() {
                        return true;
                    }
                }
            }
            false
        })
    }

    #[test]
    fn loop_levels_have_a_solution_that_fits() {
        let levels = numbered();
        let loop_levels: Vec<usize> = (1..=levels.len())
            .filter(|&n| {
                let l = &levels[n - 1];
                l.loops && l.functions == 0 && !l.if_tree && !l.if_star
            })
            .collect();
        let solved: Vec<usize> = LOOP_SOLUTIONS.iter().map(|&(n, _)| n).collect();
        assert_eq!(loop_levels, solved, "every loop level needs a solution");
        for &(n, text) in LOOP_SOLUTIONS {
            let level = &levels[n - 1];
            let program = parse_program(text);
            assert!(
                tiles(&program) <= level.slots,
                "level {n} solution too long"
            );
            assert!(always_wins(level, &[program]), "level {n} solution fails");
        }
    }

    #[test]
    fn function_levels_have_a_solution_that_fits() {
        let levels = numbered();
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
            assert!(tiles(&strips[0]) <= level.slots, "level {n}: main too long");
            for f in &strips[1..] {
                assert!(tiles(f) <= level.fn_slots, "level {n}: function too long");
                assert!(
                    !f.iter().any(|i| matches!(i, Item::Call(_))),
                    "level {n}: functions can't call functions"
                );
            }
            assert!(always_wins(level, &strips), "level {n} solution fails");
        }
    }

    /// One world per outcome after running a program; `None` once won.
    type Worlds = Vec<Option<World>>;

    /// Run a single-strip program in every outcome: `None` if it bumps into
    /// something in any of them.
    fn run(level: &Level, items: &[Item]) -> Option<Worlds> {
        let strips = [items.to_vec()];
        let mut worlds: Worlds = level
            .outcomes()
            .iter()
            .map(|o| Some(level.revealed(o)))
            .collect();
        for slot in &mut worlds {
            let Some(world) = slot else { continue };
            for exec in expand(&strips, &world.clone()) {
                if let Step::Do(cmd) = exec.step {
                    world.act(cmd).ok()?;
                    if world.stars.is_empty() {
                        *slot = None;
                        break;
                    }
                }
            }
        }
        Some(worlds)
    }

    fn won(worlds: &Worlds) -> bool {
        worlds.iter().all(Option::is_none)
    }

    /// Can the level be won in `slots` tiles with blocks and (un-nested)
    /// loops, but no functions? Returns a winning program if so. A depth-first
    /// search that gives up on any program whose first run already bumps into
    /// something, and never searches on from a position it has already
    /// reached using no more tiles.
    fn fits_with_loops(level: &Level, slots: usize) -> Option<Vec<Item>> {
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

        struct Search<'a> {
            level: &'a Level,
            blocks: Vec<Item>,
            slots: usize,
            /// Positions already searched from (with no loop open), and the
            /// fewest tiles used to get there.
            seen: HashMap<Worlds, usize>,
        }

        impl Search<'_> {
            /// Keep searching from `items`, which ran without bumping.
            fn from(&mut self, items: &mut Vec<Item>, open: Option<usize>, worlds: Worlds) -> bool {
                if won(&worlds) {
                    return true;
                }
                let used = tiles(items);
                match open {
                    None => {
                        if self.seen.get(&worlds).is_some_and(|&u| u <= used) {
                            return false;
                        }
                        self.seen.insert(worlds, used);
                    }
                    // A loop still open at the end runs to the end of the
                    // program: try every count. (While searching, it runs once.)
                    Some(start) => {
                        for k in 2..=MAX_TIMES {
                            items[start] = Item::LoopStart { times: k };
                            if run(self.level, items).is_some_and(|w| won(&w)) {
                                return true;
                            }
                        }
                        items[start] = Item::LoopStart { times: 1 };
                    }
                }
                if used == self.slots {
                    return false;
                }
                for b in 0..self.blocks.len() {
                    let block = self.blocks[b];
                    if self.try_item(items, block, open) {
                        return true;
                    }
                }
                match open {
                    None => self.try_item(items, Item::LoopStart { times: 1 }, Some(items.len())),
                    // Close the loop. A loop round one block is just repeat,
                    // so loops hold at least two.
                    Some(start) if tiles(&items[start..]) >= 2 => {
                        items.push(Item::LoopEnd);
                        for k in 2..=MAX_TIMES {
                            items[start] = Item::LoopStart { times: k };
                            if let Some(worlds) = run(self.level, items)
                                && self.from(items, None, worlds)
                            {
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

            fn try_item(&mut self, items: &mut Vec<Item>, item: Item, open: Option<usize>) -> bool {
                items.push(item);
                let found = match run(self.level, items) {
                    Some(worlds) => self.from(items, open, worlds),
                    None => false,
                };
                if !found {
                    items.pop();
                }
                found
            }
        }

        let mut search = Search {
            level,
            blocks,
            slots,
            seen: HashMap::new(),
        };
        let mut items = Vec::new();
        let start = run(level, &items)?;
        search.from(&mut items, None, start).then_some(items)
    }

    #[test]
    fn function_levels_need_a_function() {
        for (i, level) in numbered().iter().enumerate() {
            if level.functions > 0
                && let Some(program) = fits_with_loops(level, level.slots)
            {
                panic!(
                    "level {} fits in {} slots with loops alone: {program:?}",
                    i + 1,
                    level.slots
                );
            }
        }
    }

    #[test]
    fn if_levels_have_a_solution_that_fits() {
        let levels = numbered();
        let if_levels: Vec<usize> = (1..=levels.len())
            .filter(|&n| {
                let l = &levels[n - 1];
                (l.if_tree || l.if_star) && l.functions == 0
            })
            .collect();
        let solved: Vec<usize> = IF_SOLUTIONS.iter().map(|&(n, _)| n).collect();
        assert_eq!(if_levels, solved, "every if level needs a solution");
        for &(n, text) in IF_SOLUTIONS {
            let level = &levels[n - 1];
            let program = parse_program(text);
            assert!(
                tiles(&program) <= level.slots,
                "level {n} solution too long"
            );
            assert!(always_wins(level, &[program]), "level {n} solution fails");
        }
    }

    #[test]
    fn if_levels_need_an_if() {
        for (i, level) in numbered().iter().enumerate() {
            if (level.if_tree || level.if_star)
                && let Some(program) = fits_with_loops(level, level.slots)
            {
                panic!("level {} fits without an if: {program:?}", i + 1);
            }
        }
    }

    #[test]
    fn ifs_only_run_when_the_robot_sees_something() {
        let world = Level::parse(&level(&["*..", ".R#", "..."])).world();
        let steps = |text: &str| -> Vec<String> {
            expand(&[parse_program(text)], &world)
                .iter()
                .map(|e| format!("{:?}", e.step))
                .collect()
        };
        assert_eq!(
            steps("{tR cR } {sU U }"),
            [
                "Test { sense: Tree, dir: Right, yes: true }",
                "Do(Chop(Right))",
                "Test { sense: Star, dir: Up, yes: false }",
            ]
        );
        // Inside a loop, the "if" ends with the loop if it isn't closed.
        assert_eq!(steps("[ {tD D } L ]2").len(), 4, "test, left, test, left");
    }

    #[test]
    fn loop_search_finds_loop_solutions() {
        // Sanity check for `fits_with_loops`: it finds the loop levels' answers.
        let levels = numbered();
        for &(n, _) in LOOP_SOLUTIONS {
            let level = &levels[n - 1];
            assert!(fits_with_loops(level, level.slots).is_some(), "level {n}");
        }
    }

    /// A big empty field with a far-off star, for unrolling programs.
    fn open_field() -> World {
        let mut rows = vec!["...............".to_string(); 15];
        rows[0].replace_range(0..1, "*");
        rows[7].replace_range(7..8, "R");
        let rows: Vec<&'static str> = rows.into_iter().map(|r| &*r.leak()).collect();
        Level::parse(&level(rows.leak())).world()
    }

    fn moves(strips: &[Vec<Item>]) -> Vec<String> {
        expand(strips, &open_field())
            .iter()
            .map(|e| match e.step {
                Step::Do(cmd) => format!("{cmd:?}"),
                test => format!("{test:?}"),
            })
            .collect()
    }

    #[test]
    fn functions_unroll_where_they_are_called() {
        let strips = parse_strips("U [ f1 ]2 | R [ D ]2");
        let trace = expand(&strips, &open_field());
        let cmds = moves(&strips);
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
        assert_eq!(moves(&parse_strips("f1 | R f1")).len(), 1);
    }

    #[test]
    fn loop_levels_need_a_loop() {
        for (i, level) in numbered().iter().enumerate() {
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
        let strips = [parse_program("U [ R2 D ]2 L")];
        let trace = expand(&strips, &open_field());
        let cmds = moves(&strips);
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
        assert_eq!(moves(&[parse_program("[ R ]3")]).len(), 3);
        let open = parse_program("[ R ]3")[..2].to_vec();
        assert_eq!(moves(&[open]).len(), 3);
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
        for (i, level) in numbered().iter().enumerate() {
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
        for (i, level) in numbered().iter().enumerate() {
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
        for (i, level) in numbered().iter().enumerate() {
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
        for (i, level) in numbered().iter().enumerate() {
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
        // Two trees can't happen, so it tosses again.
        let mut tosses = [true, true, true, false].into_iter();
        let mut coin = || tosses.next().unwrap_or(true);
        assert_eq!(pair.random_outcome(&mut coin), [true, false]);
        assert_eq!(
            single.random_outcome(|| true),
            [true],
            "one cloud can be either"
        );
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

    #[test]
    fn ice_slides_until_something_stops_it() {
        let mut world = Level::parse(&level(&["R~~.~~@", "~~~~~~~", "*~~~~~~"])).world();
        let moved = |from| Event::Moved {
            from,
            star: false,
            key: false,
            unlocked: false,
        };
        // Grass stops the slide, and so does a rock (without a bump).
        assert_eq!(world.act(Cmd::Move(Dir::Right)), Ok(moved((0, 0))));
        assert_eq!(world.pos, (3, 0));
        assert_eq!(world.act(Cmd::Move(Dir::Right)), Ok(moved((3, 0))));
        assert_eq!(world.pos, (5, 0));
        // So does the edge of the map.
        assert!(world.act(Cmd::Move(Dir::Down)).is_ok());
        assert_eq!(world.pos, (5, 2));
        assert!(world.act(Cmd::Move(Dir::Down)).is_err(), "off the map");
        let star = world.act(Cmd::Move(Dir::Left));
        assert!(matches!(star, Ok(Event::Moved { star: true, .. })));
        assert_eq!(world.pos, (0, 2));
    }

    #[test]
    fn keys_open_doors() {
        let mut world = Level::parse(&level(&["kRD.", "..D*"])).world();
        assert!(world.act(Cmd::Move(Dir::Right)).is_err(), "locked");
        let got = world.act(Cmd::Move(Dir::Left));
        assert!(matches!(got, Ok(Event::Moved { key: true, .. })));
        assert_eq!((world.held, world.keys.len()), (1, 0));
        world.act(Cmd::Move(Dir::Right)).unwrap();
        let opened = world.act(Cmd::Move(Dir::Right));
        assert!(matches!(opened, Ok(Event::Moved { unlocked: true, .. })));
        assert_eq!(world.held, 0, "the key is used up");
        world.act(Cmd::Move(Dir::Right)).unwrap();
        // The other door needs another key; the open one stays open.
        assert!(world.act(Cmd::Move(Dir::Down)).is_ok());
        assert!(world.act(Cmd::Move(Dir::Left)).is_err());
        world.act(Cmd::Move(Dir::Up)).unwrap();
        assert!(world.act(Cmd::Move(Dir::Left)).is_ok());
        assert_eq!(world.tile((2, 0)), Some(Tile::Open));
    }

    #[test]
    fn sliding_stops_at_a_door_even_with_a_key() {
        let mut world = Level::parse(&level(&["kR~~D*"])).world();
        world.act(Cmd::Move(Dir::Right)).unwrap();
        assert_eq!(world.pos, (3, 0), "no key yet: stopped at the door");
        world.act(Cmd::Move(Dir::Left)).unwrap();
        world.act(Cmd::Move(Dir::Left)).unwrap();
        world.act(Cmd::Move(Dir::Right)).unwrap();
        assert_eq!((world.pos, world.held), ((1, 0), 1));
        world.act(Cmd::Move(Dir::Right)).unwrap();
        assert_eq!(
            (world.pos, world.held),
            ((3, 0), 1),
            "still stops at the door"
        );
        let opened = world.act(Cmd::Move(Dir::Right));
        assert!(matches!(opened, Ok(Event::Moved { unlocked: true, .. })));
        assert_eq!((world.pos, world.held), ((4, 0), 0));
    }

    #[test]
    fn key_levels_need_the_keys() {
        for (i, level) in numbered().iter().enumerate() {
            if !level.keys.is_empty() {
                let mut without = level.clone();
                without.keys.clear();
                without.slots = MAX_PROGRAM;
                without.repeat = true;
                assert_eq!(
                    fewest_blocks(&without, Allow::level(&without)),
                    None,
                    "level {} can be done without a key",
                    i + 1
                );
            }
        }
    }
}
