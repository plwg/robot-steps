//! Level data and the rules of the world. No graphics here, so it can be tested.

use std::collections::HashSet;

/// The most blocks a program can ever hold (two rows of twelve slots).
pub const MAX_PROGRAM: usize = 24;
/// The most times a single block can repeat.
pub const MAX_TIMES: u8 = 9;
/// A count, inside a function, that means "the number the function was called
/// with" (shown as the function's shape).
pub const NUMBER: u8 = 0;

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
    /// Grass under a rain cloud that counts down 3, 2, 1, rains for a while,
    /// then starts again. The number is what it shows at the start. The
    /// robot mustn't get rained on.
    Rain(u8),
    /// A river or pond: the robot can't go in.
    Water,
    /// A lily pad on the water (`Bridge` from when it was drawn as one). It
    /// holds the robot once: when the robot steps off, it sinks, and there's
    /// only water left.
    Bridge,
}

/// A rain cloud stays dry for this many ticks (showing 3, 2, 1), then rains
/// for `RAIN_WET`, then starts again.
pub const RAIN_DRY: u8 = 3;
pub const RAIN_WET: u8 = 2;
pub const RAIN_CYCLE: u8 = RAIN_DRY + RAIN_WET;

/// What a rain tile is doing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weather {
    /// Counting down: how many ticks until it rains.
    Dry(u8),
    /// Raining: how many ticks of rain are left, counting this one.
    Raining(u8),
}

impl Weather {
    pub fn raining(self) -> bool {
        matches!(self, Weather::Raining(_))
    }

    /// How far round its cycle the cloud is: 0 to `RAIN_CYCLE - 1`, the dry
    /// ticks first.
    pub fn phase(self) -> u8 {
        match self {
            Weather::Dry(n) => RAIN_DRY - n,
            Weather::Raining(n) => RAIN_CYCLE - n,
        }
    }
}

impl Tile {
    /// Can the robot drive onto this tile (without unlocking anything, and
    /// if it isn't raining there)?
    fn passable(self) -> bool {
        matches!(
            self,
            Tile::Grass | Tile::Stump | Tile::Ice | Tile::Open | Tile::Rain(_) | Tile::Bridge
        )
    }
}

/// One instruction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Move(Dir),
    Chop(Dir),
    /// Sleep for a tick (doing nothing), while the rain clouds count down.
    Sleep,
}

impl Cmd {
    /// Which way it faces the robot (waiting doesn't turn it).
    pub fn dir(self) -> Option<Dir> {
        match self {
            Cmd::Move(d) | Cmd::Chop(d) => Some(d),
            Cmd::Sleep => None,
        }
    }
}

/// An instruction plus how many times to do it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Block {
    pub cmd: Cmd,
    pub times: u8,
}

/// What an "if" block (or a "while") looks for, next to the robot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sense {
    Tree,
    Rock,
    Star,
    Ice,
    Key,
    /// A door, locked or open.
    Door,
    /// Grass (or a stump), with or without a star or key on it.
    Grass,
    /// The edge of the map: nothing there at all.
    Edge,
    /// Anything the robot can't drive onto: a rock, a tree, a locked door it
    /// has no key for, or the edge of the map (but not rain, which passes).
    /// Not offered any more (it looks just like a rock); program text
    /// still has it.
    Wall,
    /// Always there: "while true" goes round for ever, until a break (or the
    /// last star). Every "while" can pick it.
    True,
}

impl Sense {
    /// Everything an "if" can look for, in the order it's offered.
    pub const ALL: [Sense; 8] = [
        Sense::Tree,
        Sense::Rock,
        Sense::Star,
        Sense::Ice,
        Sense::Key,
        Sense::Door,
        Sense::Grass,
        Sense::Edge,
    ];

    /// The sense with this letter in program text.
    fn from_letter(letter: char) -> Sense {
        Sense::with_letter(letter).expect("bad sense")
    }

    /// The sense with this letter, if there is one.
    pub fn with_letter(letter: char) -> Option<Sense> {
        Sense::ALL
            .into_iter()
            .chain([Sense::Wall, Sense::True])
            .find(|s| s.letter() == letter)
    }

    /// Its letter after `{` in program text (and in saved maps).
    pub fn letter(self) -> char {
        match self {
            Sense::Tree => 't',
            Sense::Rock => 'r',
            Sense::Star => 's',
            Sense::Ice => 'i',
            Sense::Key => 'k',
            Sense::Door => 'd',
            Sense::Grass => 'g',
            Sense::Edge => 'e',
            Sense::Wall => 'w',
            Sense::True => 'y',
        }
    }
}

/// One slot in a program strip: a block, one end of a loop or an "if", or a
/// call to a function.
///
/// Blocks between a `LoopStart` and the next `LoopEnd` (or the end of the
/// strip, if the loop is still open) run `times` times over. A `While` loop
/// ends the same way, but runs while the robot sees `sense` in direction
/// `dir` (or with `not`, while it doesn't), looking before each time round. Blocks between an `IfStart` and the next
/// `IfEnd` (or the end of the loop or strip it's in) only run if the robot sees
/// `sense` next to it in direction `dir`; if there's an `Else` in between, the
/// blocks after it run instead when it doesn't.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    Block(Block),
    LoopStart {
        times: u8,
    },
    /// Runs while the robot sees `sense` in direction `dir` (or with `not`,
    /// while it doesn't).
    While {
        sense: Sense,
        dir: Dir,
        not: bool,
    },
    LoopEnd,
    /// With `not`, the blocks run if `sense` *isn't* there.
    IfStart {
        sense: Sense,
        dir: Dir,
        not: bool,
    },
    Else,
    IfEnd,
    /// Run function strip `f` (1 or 2), with the number `n` for any counts
    /// in it that are `NUMBER`. Only allowed in the main strip, so a function
    /// can never call itself forever.
    Call(usize, u8),
    /// An empty slot opened up in the middle of a program, waiting for a
    /// block. Running skips it.
    Gap,
    /// Leave the loop (or "while") it's in, straight away.
    Break,
}

/// How many tiles a strip uses. Loop brackets, "else" and the end of an
/// "if" sit between tiles; blocks, calls and the start of an "if" or a
/// "repeat until" (which show what they're looking for) each take a tile.
pub fn tiles(items: &[Item]) -> usize {
    items.iter().filter(|i| i.is_tile()).count()
}

impl Item {
    /// Does this take a whole tile (rather than sitting between two)?
    pub fn is_tile(self) -> bool {
        matches!(
            self,
            Item::Block(_)
                | Item::Call(..)
                | Item::IfStart { .. }
                | Item::While { .. }
                | Item::Gap
                | Item::Break
        )
    }
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
    /// A "while" that would go round for ever: it's back where it started a
    /// time round before, and still going. The program stops.
    Forever,
    /// A break: leaving the loop it's in.
    Break,
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

/// Where in the main strip a function was called from, and its number.
#[derive(Clone, Copy)]
struct Context {
    call: Option<usize>,
    main_loop: Option<(usize, u8)>,
    number: u8,
}

impl Context {
    /// A count, with `NUMBER` turned into the function's number.
    fn count(self, times: u8) -> u8 {
        if times == NUMBER { self.number } else { times }
    }
}

/// Runs a program on a copy of the world, recording each step. What an "if"
/// does depends on what the robot finds, so the program has to be run to be
/// unrolled.
struct Runner<'a> {
    strips: &'a [Vec<Item>],
    world: World,
    out: Vec<Exec>,
    /// The world just before each step.
    worlds: Vec<World>,
    /// How many steps really happen: the robot got stuck (or wet), or
    /// collected every star, and nothing more happens. The rest of the
    /// program is still unrolled after it, in the world as it was left.
    ran: Option<usize>,
    /// Too many steps to unroll after the end: give up.
    halted: bool,
    /// A break is leaving the loops it's in, up to the nearest one.
    breaking: bool,
}

/// The most steps a program is unrolled to, after its end.
const MAX_STEPS: usize = 1000;

/// A program run all the way through ahead of time: each step, and the world
/// just before it, then the world at the end (so one more world than steps).
/// Only the first `ran` steps happen; the rest of the program follows them,
/// in the world as it was left.
pub struct Simulation {
    pub trace: Vec<Exec>,
    pub worlds: Vec<World>,
    pub ran: usize,
}

/// Unroll a program (the main strip, then any functions) into the steps it
/// takes when run in `world`, in order. It stops after a step that gets the
/// robot stuck, or that collects the last star, just as the game does.
pub fn expand(strips: &[Vec<Item>], world: &World) -> Vec<Exec> {
    let mut sim = simulate(strips, world);
    sim.trace.truncate(sim.ran);
    sim.trace
}

/// The same, keeping the world before each step, and at the end.
pub fn simulate(strips: &[Vec<Item>], world: &World) -> Simulation {
    let mut runner = Runner {
        strips,
        world: world.clone(),
        out: Vec::new(),
        worlds: Vec::new(),
        ran: None,
        halted: false,
        breaking: false,
    };
    if !strips.is_empty() {
        let top = Context {
            call: None,
            main_loop: None,
            number: 1,
        };
        runner.run(0, 0, strips[0].len(), None, top);
    }
    runner.worlds.push(runner.world);
    Simulation {
        ran: runner.ran.unwrap_or(runner.out.len()),
        trace: runner.out,
        worlds: runner.worlds,
    }
}

impl Runner<'_> {
    fn emit(&mut self, step: Step, at: Loc, left: u8, in_loop: Option<(usize, u8)>, ctx: Context) {
        if self.halted {
            return;
        }
        if self.ran.is_some() && self.out.len() >= MAX_STEPS {
            self.halted = true;
            return;
        }
        let main = at.0 == 0;
        self.worlds.push(self.world.clone());
        self.out.push(Exec {
            step,
            at,
            left,
            main_loop: if main { in_loop } else { ctx.main_loop },
            fn_loop: if main { None } else { in_loop },
            call: ctx.call,
        });
        if let (Step::Do(cmd), None) = (step, self.ran) {
            let stuck = self.world.act(cmd).is_err();
            if stuck || self.world.wet() || self.world.stars.is_empty() {
                self.ran = Some(self.out.len());
            }
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
        while i < to && !self.halted && !self.breaking {
            match items[i] {
                Item::LoopStart { times } => {
                    let end = (i + 1..to)
                        .find(|&j| items[j] == Item::LoopEnd)
                        .unwrap_or(to);
                    let times = ctx.count(times);
                    for k in 0..times {
                        self.run(strip, i + 1, end, Some((i, times - k)), ctx);
                        if std::mem::take(&mut self.breaking) {
                            break;
                        }
                    }
                    i = end + 1;
                }
                Item::While { sense, dir, not } => {
                    let end = (i + 1..to)
                        .find(|&j| items[j] == Item::LoopEnd)
                        .unwrap_or(to);
                    // A program always does the same thing from the same
                    // place, so getting back to one is going round for ever.
                    let mut been = HashSet::new();
                    while !self.halted {
                        let yes = self.world.sees(sense, dir) != not;
                        self.emit(Step::Test { sense, dir, yes }, (strip, i), 1, in_loop, ctx);
                        if !yes {
                            break;
                        }
                        if !been.insert(self.world.clone()) {
                            // After the end, nothing moves: go on past it.
                            if self.ran.is_none() {
                                self.emit(Step::Forever, (strip, i), 1, in_loop, ctx);
                                self.ran = Some(self.out.len());
                            }
                            break;
                        }
                        self.run(strip, i + 1, end, Some((i, 0)), ctx);
                        if std::mem::take(&mut self.breaking) {
                            break;
                        }
                    }
                    i = end + 1;
                }
                Item::IfStart { sense, dir, not } => {
                    let end = (i + 1..to).find(|&j| items[j] == Item::IfEnd).unwrap_or(to);
                    let otherwise = (i + 1..end).find(|&j| items[j] == Item::Else);
                    // `yes`: the blocks inside run.
                    let yes = self.world.sees(sense, dir) != not;
                    self.emit(Step::Test { sense, dir, yes }, (strip, i), 1, in_loop, ctx);
                    match (yes, otherwise) {
                        (true, _) => self.run(strip, i + 1, otherwise.unwrap_or(end), in_loop, ctx),
                        (false, Some(e)) => self.run(strip, e + 1, end, in_loop, ctx),
                        (false, None) => {}
                    }
                    i = end + 1;
                }
                Item::Block(b) => {
                    let times = ctx.count(b.times);
                    for r in 0..times {
                        self.emit(Step::Do(b.cmd), (strip, i), times - r, in_loop, ctx);
                    }
                    i += 1;
                }
                Item::Call(f, number) if strip == 0 && f < self.strips.len() => {
                    let inner = Context {
                        call: Some(i),
                        main_loop: in_loop,
                        number,
                    };
                    self.run(f, 0, self.strips[f].len(), None, inner);
                    // A break outside a loop just ends the function.
                    self.breaking = false;
                    i += 1;
                }
                Item::Break => {
                    self.emit(Step::Break, (strip, i), 1, in_loop, ctx);
                    self.breaking = true;
                    i += 1;
                }
                Item::Call(..) | Item::LoopEnd | Item::Else | Item::IfEnd | Item::Gap => i += 1,
            }
        }
    }
}

/// A whole program written as text: strips separated by `|`. Within a strip,
/// `R` moves right, `cR` chops right, `Z` sleeps, and a number repeats a
/// block. `[` opens a loop and `]4` closes it, running it 4 times; `<gR`
/// opens a loop that runs while there's grass to the right, closed with `]`.
/// `{tR` starts "if there's a tree to the right", `{sU` "if there's a star
/// above" (see `Sense::letter` for the rest), and `{!tR` "if there's no
/// tree to the right"; `/` is "else" and `}` ends it. `f1` calls the first function, and
/// `f1n3` calls it with the number 3; inside a function, `#` in place of a
/// count (`R#`, `]#`) means that number. `<y` opens "while true", and `B` is
/// a break.
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
        if let Some(call) = token.strip_prefix('f') {
            let (f, n) = call.split_once('n').unwrap_or((call, "1"));
            items.push(Item::Call(
                f.parse().expect("bad function"),
                n.parse().expect("bad number"),
            ));
            continue;
        }
        let (token, digits) = match token.strip_suffix('#') {
            Some(head) => (head, None),
            None => {
                let digits = token.trim_start_matches(|c: char| !c.is_ascii_digit());
                (&token[..token.len() - digits.len()], Some(digits))
            }
        };
        let times: u8 = match digits {
            None => NUMBER,
            Some(d) => d.parse().unwrap_or(1),
        };
        let mut chars = token.chars();
        let item = match (chars.next(), chars.next()) {
            (Some('}'), None) => Item::IfEnd,
            (Some('/'), None) => Item::Else,
            // `<gR` goes on while there's grass to the right, `<!tR` while
            // there's no tree (and `<R`, while there's a wall).
            (Some('<'), Some(first)) => {
                open = Some(items.len());
                // "While true" looks no way in particular.
                if first == 'y' {
                    items.push(Item::While {
                        sense: Sense::True,
                        dir: Dir::Right,
                        not: false,
                    });
                    continue;
                }
                let not = first == '!';
                let first = if not {
                    chars.next().expect("<! needs a sense")
                } else {
                    first
                };
                match chars.next() {
                    None => Item::While {
                        sense: Sense::Wall,
                        dir: dir(first),
                        not,
                    },
                    Some(d) => Item::While {
                        sense: Sense::from_letter(first),
                        dir: dir(d),
                        not,
                    },
                }
            }
            (Some('{'), Some(what)) => {
                let not = what == '!';
                let what = if not {
                    chars.next().expect("{! needs a sense")
                } else {
                    what
                };
                Item::IfStart {
                    sense: Sense::from_letter(what),
                    dir: dir(chars.next().expect("{ needs a direction")),
                    not,
                }
            }
            (Some('['), None) => {
                open = Some(items.len());
                Item::LoopStart { times: 1 }
            }
            (Some(']'), None) => {
                let start = open.take().expect("] without [");
                if let Item::LoopStart { .. } = items[start] {
                    items[start] = Item::LoopStart { times };
                }
                Item::LoopEnd
            }
            (Some('B'), None) => Item::Break,
            (Some('Z'), None) => Item::Block(Block {
                cmd: Cmd::Sleep,
                times,
            }),
            (Some('c'), Some(d)) => Item::Block(Block {
                cmd: Cmd::Chop(dir(d)),
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

/// A chapter: one idea, starting with a demo of any new block, then levels
/// that use it. Levels are numbered within their chapter (`3-2`), so a
/// chapter can grow without renumbering the rest.
pub struct Chapter {
    pub name: &'static str,
    levels: &'static [LevelDef],
}

/// What an "if" can look for, for the levels' `with_ifs`.
const TREE: &[Sense] = &[Sense::Tree];
const STAR: &[Sense] = &[Sense::Star];
const TREE_STAR: &[Sense] = &[Sense::Tree, Sense::Star];
const ROCK: &[Sense] = &[Sense::Rock];
const GRASS: &[Sense] = &[Sense::Grass];

pub struct LevelDef {
    /// What the save files call this level. It never changes, wherever the
    /// level moves: the levels used to be numbered straight through, and
    /// these are those numbers (`d` and the number for a demo), so old saves
    /// still work. New levels get a name instead.
    id: &'static str,
    /// Legend: `.` grass, `#` tree, `@` rock, `?` mystery (tree or grass),
    /// `~` ice, `k` key, `D` locked door, `1` to `3` a rain cloud (counting
    /// down from that), `w` water, `=` a bridge (it falls in once the robot
    /// has crossed it), `R` robot start, `*` star.
    map: &'static [&'static str],
    /// How many blocks fit in the program for this level.
    slots: usize,
    chop: bool,
    repeat: bool,
    loops: bool,
    /// What the "if" block can look for (none: no "if" block).
    ifs: &'static [Sense],
    /// "While", and "else" and "not" for the "if" blocks (and "not" for
    /// "while" too).
    whiles: bool,
    /// What "while" can look for.
    while_looks: &'static [Sense],
    /// The break block.
    breaks: bool,
    otherwise: bool,
    not: bool,
    /// How many function strips (0, 1 or 2), and how many slots each has.
    functions: usize,
    fn_slots: usize,
    /// Functions are called with a number, which counts inside them can use.
    numbers: bool,
    /// The sleep block.
    sleep: bool,
    /// A demo level comes with its program already written, to show how a
    /// new block is used: just press play.
    demo: Option<&'static str>,
}

const fn level(id: &'static str, map: &'static [&'static str]) -> LevelDef {
    LevelDef {
        id,
        map,
        slots: MAX_PROGRAM,
        chop: false,
        repeat: false,
        loops: false,
        ifs: &[],
        whiles: false,
        while_looks: &[],
        breaks: false,
        otherwise: false,
        not: false,
        functions: 0,
        fn_slots: 0,
        numbers: false,
        sleep: false,
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

    /// Unlock the "if" block (and loops), looking for `senses`, and limit the program to `slots`, so an "if" in a loop is needed to
    /// fit.
    const fn with_ifs(mut self, slots: usize, senses: &'static [Sense]) -> Self {
        self.ifs = senses;
        self.with_loops(slots)
    }

    /// Add `functions` function strips of `fn_slots` each (plus loops), and
    /// limit the main program to `slots`, so functions are needed to fit.
    const fn with_functions(mut self, slots: usize, functions: usize, fn_slots: usize) -> Self {
        self.functions = functions;
        self.fn_slots = fn_slots;
        self.with_loops(slots)
    }

    /// Unlock "while" (and loops), looking for `senses`, and limit the
    /// program to `slots`.
    const fn with_while(mut self, slots: usize, senses: &'static [Sense]) -> Self {
        self.whiles = true;
        self.while_looks = senses;
        self.with_loops(slots)
    }

    /// Unlock the break block.
    const fn with_break(mut self) -> Self {
        self.breaks = true;
        self
    }

    /// Only "while" loops: no counted loops, and no counts on blocks either,
    /// so a "while true" with a break is the way round.
    const fn only_while(mut self) -> Self {
        self.loops = false;
        self.repeat = false;
        self
    }

    /// Unlock "else" for the "if" blocks.
    const fn with_else(mut self) -> Self {
        self.otherwise = true;
        self
    }

    /// Unlock "not" for the "if" blocks: "if there's no tree".
    const fn with_not(mut self) -> Self {
        self.not = true;
        self
    }

    /// Unlock the sleep block (and repeat, to sleep a few ticks), and limit the
    /// program to `slots`.
    const fn with_sleep(mut self, slots: usize) -> Self {
        self.sleep = true;
        self.with_repeat(slots)
    }

    /// Let functions be called with a number, for counts inside them.
    const fn with_numbers(mut self) -> Self {
        self.numbers = true;
        self
    }

    /// Make this a demo, with `program` (see `parse_strips`) already in place.
    const fn demo(mut self, program: &'static str) -> Self {
        self.demo = Some(program);
        self
    }

    /// Unlock the "if" block looking for trees, and the axe: for mystery
    /// clouds, "if there's a tree that way { chop it }".
    const fn with_tree_if(mut self) -> Self {
        self.chop = true;
        self.ifs = &[Sense::Tree];
        self
    }
}

pub const CHAPTERS: &[Chapter] = &[
    // Up, down, left and right, in order.
    Chapter {
        name: "Moving",
        levels: &[
            level("d1", &["R.@", "@.*"]).demo("R D R"),
            level("1", &["R...*"]),
            level("2", &["R", ".", ".", "*"]),
            level("3", &["R..", "##.", "##*"]),
            level("4", &["*##", ".##", "..R"]),
            level("5", &["R.*.*"]),
            level("6", &["R#*", ".#.", "..."]),
            level("7", &["R.##", "#..#", "##.*"]),
            level("8", &["*...*", ".#.#.", "..R.."]),
            level("9", &["R.#..*", ".##.#.", "....#."]),
            level("10", &["*....*", ".####.", "...R.*"]),
            level("11", &["R#*#*", ".#.#.", "....."]),
            level("12", &["*...*", ".#.#.", "..R..", ".#.#.", "*...*"]),
            // A ring of stars to collect all the way round.
            level("star-ring", &["R***", "*@@*", "*@@*", "****"]),
        ],
    },
    // The robot slides until something stops it (grass, a rock, or the edge),
    // so think ahead about where it will end up.
    Chapter {
        name: "Ice",
        levels: &[
            level("d61", &["R~~~@", "@@@~@", "@@@*@"]).demo("R D"),
            level("61", &["R~~.~~*"]),
            level("62", &["R~~~~@", "@@@@~@", "*~~~~."]),
            // The star is right under the ice, but the robot slides past it.
            level("63", &["R~~~~~.", "@@@*@@.", "@@@~@@.", "@@@~..."]),
            level("64", &["R~@~~", "~~~~~", "~~~~*", "~~~@~"]),
            level("65", &["~~~~~", "@@~*~", "~~~~~", "~R@~~"]),
            level("66", &["~~@~~", "*~~@~", "@~~~~", "R~~~@"]),
            // A spiral of ice: each move whooshes all the way along a side, and
            // the last one stops on the star in the middle.
            level("ice-spiral", &["R~~~~", "@@@@~", "~~*@~", "~@@@~", "~~~~~"]),
        ],
    },
    // Chop trees, but rocks stay put.
    Chapter {
        name: "The axe",
        levels: &[
            level("d13", &["R", "#", "*"]).with_axe().demo("cD D D"),
            level("13", &["R#*"]).with_axe(),
            level("14", &["..@*", "R.#.", "..@."]).with_axe(),
            level("15", &["R@*", ".@.", "#@.", "..."]).with_axe(),
            level("16", &["R##.*"]).with_axe(),
            // Timber! Chop, step, chop, step, all the way through.
            level("timber", &["R###*"]).with_axe(),
            level("25", &["R.#.", "@@#@", "*..."]).with_axe(),
        ],
    },
    // Pick up a key to open a locked door. The robot remembers what it's
    // carrying, so fetch the key first, even when it's the other way.
    Chapter {
        name: "Keys and doors",
        levels: &[
            level("d67", &["R.kD*"]).demo("R R R R"),
            level("67", &["kR.D*"]),
            // Down the side path for the key, then back up to the door.
            level("key-hunt", &["R..D*", "@@.@@", "@@k@@"]),
            // A key behind a tree opens the door to the second key, which
            // opens the door to the star.
            level("two-keys", &["R#k@*", "@@D@D", "@@.k."]).with_axe(),
            level("68", &["k@*", ".@D", "R.."]),
            level("69", &["R#D*", ".@@@", "k@@@"]).with_axe(),
            level("71", &["R~~~k", "@@@@D", "*~~~."]),
            // A locked door stops the robot sliding, even with the key: it has to
            // step onto the door to open it.
            level("72", &["*D~~~R", "@@@@@~", "k~~~~~"]),
        ],
    },
    // A lily pad holds the robot once: step off, and it sinks. No going back
    // the same way, so plan the way round.
    Chapter {
        name: "Lily pads",
        levels: &[
            level("d-bridge", &["www", "R=*", "www"]).demo("R R"),
            // Hop from island to island.
            level("bridge-hop", &["wwwwwww", "R=*=*=*", "wwwwwww"]),
            // Over for the key, then back over the other lily pad for the door.
            level("bridge-key", &["R.=.k", "..w..", "..=..", "D@w@@", "*@w@@"]),
            // The island with one lily pad has to come last: the other one
            // has a pad in and a pad out.
            level("bridge-order", &["R.....", "=ww=w=", "*ww*.."]),
            // A sunk lily pad is water: the next slide stops before it.
            level("bridge-ice", &["R~~=~~.", "@@@@*@@"]),
        ],
    },
    // Not enough slots without repeating a block; then with the axe, and keys.
    Chapter {
        name: "Repeat",
        levels: &[
            level("d17", &["R....", "@@@@.", "@@@@*"])
                .with_repeat(2)
                .demo("R4 D2"),
            level("17", &["R.......*"]).with_repeat(3),
            level("18", &["R.....", "#####.", "#####.", "#####.", "*....."]).with_repeat(4),
            level("19", &["R.....", "#####.", "*...#.", "#.###.", "#....."]).with_repeat(6),
            level("20", &["R..*...*", "#######.", "*......."]).with_repeat(4),
            level(
                "26",
                &["R.......", "#######.", "........", ".#######", "*......."],
            )
            .with_repeat(5),
            // Round and round, to the star in the middle.
            level("spiral", &["R....", "@@@@.", "..*@.", ".@@@.", "....."]).with_repeat(5),
            // Two doors need two keys.
            level("70", &["R.D.D*", ".@@@@@", "k@@@@@", "k@@@@@"]).with_repeat(3),
            level("21", &["@@@@@@@@@", "R...#...*", "@@@@@@@@@"])
                .with_axe()
                .with_repeat(4),
            level("22", &["R.#....", ".@#.@@.", ".@#.@*.", "...@..."])
                .with_axe()
                .with_repeat(6),
            level(
                "23",
                &[
                    "R....#..*",
                    "@@@@.@@@.",
                    "*#.....#.",
                    "@@@@@@@@.",
                    "*........",
                ],
            )
            .with_axe()
            .with_repeat(12),
            level(
                "24",
                &[
                    "*#..@..#*",
                    "@@@.@.@@@",
                    "....R....",
                    "@@@.@.@@@",
                    "...#@..#*",
                ],
            )
            .with_axe()
            .with_repeat(20),
        ],
    },
    // Mystery clouds turn into a tree or grass each time you press play, so
    // the robot has to look before it chops.
    Chapter {
        name: "If",
        levels: &[
            level("d27", &["R", "?", "*"])
                .with_tree_if()
                .demo("{tD cD } D D"),
            level("27", &["R?*"]).with_tree_if(),
            level("28", &["R.?.*"]).with_tree_if(),
            level("29", &["R?@", "@.@", "@?*"]).with_tree_if(),
            level("30", &["@@@@@@", "R?.?.*", "@@@@@@"]).with_tree_if(),
            level("31", &["*?.R.?*"]).with_tree_if(),
            level("32", &["R.?.@*", "@@.@@?", "*?...."]).with_tree_if(),
            level("33", &["R...?...*"]).with_tree_if().with_repeat(4),
            level("34", &["R..?..#..", "@@@@@@@@.", "*..?..?.."])
                .with_tree_if()
                .with_repeat(14),
        ],
    },
    // Repeat a whole group of blocks. First two familiar maps (levels 1-7
    // and 1-11) with too few slots to do them the old way.
    Chapter {
        name: "Loops",
        levels: &[
            level("d35", &["R@@@", "..@@", "@..@", "@@.*"])
                .with_loops(2)
                .demo("[ D R ]3"),
            level("35", &["R.##", "#..#", "##.*"]).with_loops(4),
            level("36", &["R#*#*", ".#.#.", "....."]).with_loops(5),
            level(
                "37",
                &[
                    "R.@@@@@", "@..@@@@", "@@..@@@", "@@@..@@", "@@@@..@", "@@@@@.*",
                ],
            )
            .with_loops(5),
            // Up the stairs, a star on every step.
            level(
                "stairs",
                &["@@@@@*", "@@@@*.", "@@@*.@", "@@*.@@", "@*.@@@", "R.@@@@"],
            )
            .with_loops(2),
            level("38", &["R..@@@@@@", "@@...@@@@", "@@@@...@@", "@@@@@@..*"]).with_loops(5),
            level("39", &["...@...@..*", ".@.@.@.@.@@", "R@...@...@@"]).with_loops(8),
            level("40", &["R#.#.#.#*"]).with_axe().with_loops(4),
            level("73", &["@k@@k@@k@@", "R.D..D..D*"]).with_loops(4),
        ],
    },
    // Look before you act: inside a loop, one rule handles whatever the
    // robot finds, wherever it finds it.
    Chapter {
        name: "If inside loops",
        levels: &[
            level("d41", &["R.##..#*"])
                .with_axe()
                .with_ifs(3, TREE)
                .demo("[ {tR cR } R ]7"),
            level("41", &["R#..#.##.*"]).with_axe().with_ifs(3, TREE),
            level("d42", &["@**@*", "R...."])
                .with_ifs(4, STAR)
                .demo("[ {sU U D } R ]5"),
            level("42", &["@*@@*@**@", "R........"]).with_ifs(4, STAR),
            level("43", &["@@**@@@*@", "R#..#.#.."])
                .with_axe()
                .with_ifs(6, TREE_STAR),
            level("49", &["R?.?.?.?*"]).with_tree_if().with_loops(3),
            level("50", &["R?@@@", "@.?@@", "@@.?@", "@@@.?", "@@@@*"])
                .with_tree_if()
                .with_loops(4),
            level("51", &["R#@@@", "@..@@", "@@#.@", "@@@*#", "@@@@*"])
                .with_axe()
                .with_ifs(6, TREE),
            level(
                "52",
                &[
                    "R..@@@@@@",
                    "*@...@@@@",
                    "@@*@...@@",
                    "@@@@@@...",
                    "@@@@@@*@.",
                    "@@@@@@@@*",
                ],
            )
            .with_ifs(5, STAR),
        ],
    },
    // If there's a tree, do one thing; if not, do another.
    Chapter {
        name: "Else",
        levels: &[
            level("d76", &["R.#@", "@.#@", "@..*"])
                .with_ifs(3, TREE)
                .with_else()
                .demo("[ {tR D / R } ]5"),
            level("76", &["R..#@@", "@@..#@", "@@@.#@", "@@@..*"])
                .with_ifs(3, TREE)
                .with_else(),
            level("77", &["R@@@", "...@", "##.@", "@@..", "@@#*"])
                .with_ifs(3, TREE)
                .with_else(),
            // Not a tree this time: "if there's a rock that way".
            level("else-rock", &["@@@..*", "@@@.@@", "@@..@@", "R..@@@"])
                .with_ifs(3, ROCK)
                .with_else(),
        ],
    },
    // Not: "if there's *no* tree". The level only lets the "if" look for one
    // thing, so it's the way to act where that thing isn't.
    Chapter {
        name: "Not",
        levels: &[
            level("not-demo", &["#*#*", "R..."])
                .with_ifs(4, TREE)
                .with_not()
                .demo("[ {!tU U D } R ]4"),
            level("not-1", &["#*##*#**#", "R........"])
                .with_ifs(4, TREE)
                .with_not(),
            level("not-2", &[".@", ".*", ".*", ".@", ".@", ".*", "R@"])
                .with_ifs(4, ROCK)
                .with_not(),
        ],
    },
    // Keep going while there's grass ahead (or, with "not", while there's no
    // tree). The clouds decide how far that is, so no number of times would
    // do.
    Chapter {
        name: "While",
        levels: &[
            level("d74", &["R...@", "@@@*@"])
                .with_while(3, GRASS)
                .demo("<gR R ] D"),
            // Along while there's grass, chop whatever stops it, and on.
            level("74", &["R...?...*"]).with_axe().with_while(5, GRASS),
            // "While there's no tree".
            level("d-while-not", &["R...#.*"])
                .with_axe()
                .with_while(4, TREE)
                .with_not()
                .demo("<!tR R ] cR R3"),
            level("75", &["R", ".", ".", "?", ".", ".", "*"])
                .with_axe()
                .with_while(5, TREE)
                .with_not(),
        ],
    },
    // Break: leave a loop straight away. "While true" goes round for ever
    // until a break; a counted loop can break early too. Without counts the
    // long way round doesn't fit, so look and break.
    Chapter {
        name: "Break",
        levels: &[
            level("d-while-true", &["R....", "@@@.@", "@@@*@"])
                .with_ifs(6, GRASS)
                .with_while(6, &[])
                .with_break()
                .only_while()
                .demo("<y R {gD B } ] D D"),
            // Along until there's grass below, down, and back to the star.
            level("break-1", &["R......", "@?????.", "*......"])
                .with_ifs(8, GRASS)
                .with_while(8, &[])
                .with_break()
                .only_while(),
            // A break leaves a counted loop too.
            level("d-break", &["R.....", "@@@.@@", "@@@*@@"])
                .with_ifs(4, GRASS)
                .with_break()
                .demo("[ R {gD B } ]5 D2"),
            // Down until there's grass to the right, across, and down to the
            // star.
            level(
                "break-2",
                &["R@.", ".?.", ".?.", ".?.", ".?.", ".?.", ".?.", "..*"],
            )
            .with_ifs(8, GRASS)
            .with_while(8, &[])
            .with_break()
            .only_while(),
        ],
    },
    // A strip of blocks with a shape for a name, which the main program can
    // run (call) as often as it likes. First a familiar map (the zig-zag from
    // the loops). Then two functions.
    Chapter {
        name: "Functions",
        levels: &[
            level("d44", &["*@@*@*", ".@@.@.", "R....."])
                .with_functions(5, 1, 2)
                .demo("f1 R3 f1 R2 f1 | U2 D2"),
            level("44", &["...@...@..*", ".@.@.@.@.@@", "R@...@...@@"]).with_functions(3, 1, 4),
            level("45", &["@*@*@@@*", "@.@.@@@.", "R......."]).with_functions(5, 1, 3),
            level(
                "46",
                &[
                    "@@@@@@@*", "@@@@@@..", "@@@@@..@", "@@@...@@", "@@..@@@@", "@..@@@@@",
                    "R.@@@@@@",
                ],
            )
            .with_functions(3, 1, 4),
            level("47", &["@*@*@*", "@.@.@.", "R#.#.#"])
                .with_axe()
                .with_functions(4, 1, 5),
            level("d48", &["*@*@*", ".@.@.", "R#.#."])
                .with_axe()
                .with_functions(5, 2, 2)
                .demo("f1 f2 f1 f2 f1 | U2 D2 | cR R2"),
            // Two functions: a short tower and a tall one, in no fixed order.
            level("fn2-towers", &["@@@*@*@@@*", "@*@.@.@*@.", "@R........"])
                .with_functions(5, 2, 3),
            level("48", &["*@@*@*", ".@@.@.", "R.##.#"])
                .with_axe()
                .with_functions(6, 2, 3),
            level(
                "53",
                &[
                    "@@@.*@@@@.*@@",
                    "@@....@@....@",
                    "#..##....##..",
                    "R.####*.####*",
                ],
            )
            .with_functions(4, 2, 3),
            level(
                "54",
                &["R####", "@@@@*", "####.", "*@@@@", ".####", "@@@@*"],
            )
            .with_axe()
            .with_functions(6, 2, 3),
        ],
    },
    // "If" blocks inside functions: one function looks around, and the main
    // program says where.
    Chapter {
        name: "Functions + if",
        levels: &[
            level(
                "55",
                &[
                    "R?...@@@@@@@@",
                    "@@@@.@@@@@@@@",
                    "@@@@..?*.@@@@",
                    "@@@@@@@@.@@@@",
                    "@@@@@@@@...?*",
                ],
            )
            .with_tree_if()
            .with_functions(3, 1, 3),
            level(
                "56",
                &["@**@*@", "R.....", "*@@*@.", "@@*@*.", "......", "@*@*@@"],
            )
            .with_ifs(5, STAR)
            .with_functions(5, 1, 6),
            level(
                "57",
                &[
                    "@*@*@@@@@@@@@",
                    "R....@@@@@@@@",
                    "@@@@.@**@@@@@",
                    "@@@@.....@@@@",
                    "@@@@@@@@.*@*@",
                    "@@@@@@@@....*",
                ],
            )
            .with_ifs(5, STAR)
            .with_functions(5, 1, 4),
            level(
                "58",
                &[
                    "R.#*@@@@", "@@@#@@@@", "@@@..#@@", "@@@@@.@@", "@@@@@#@@", "@@@@@*#*",
                ],
            )
            .with_axe()
            .with_ifs(5, TREE)
            .with_functions(5, 2, 3),
        ],
    },
    // Say how big each time you call it.
    Chapter {
        name: "Function numbers",
        levels: &[
            level(
                "d78",
                &[
                    "@@@...*", "@@@.@@@", "@@@.@@@", "@...@@@", "@.@@@@@", "..@@@@@", "R@@@@@@",
                ],
            )
            .with_functions(3, 1, 2)
            .with_numbers()
            .demo("f1n1 f1n2 f1n3 | U# R#"),
            level(
                "78",
                &[
                    "R...@@@", "@@@.@@@", "@@@.@@@", "@@@...@", "@@@@@.@", "@@@@@..", "@@@@@@*",
                ],
            )
            .with_functions(3, 1, 2)
            .with_numbers(),
            level("79", &["@@*@@@@", "@@.@@@*", "@@.@*@.", "R......"])
                .with_functions(3, 1, 3)
                .with_numbers(),
            // Hills of 2, 3 and 1: up a step at a time, then down. The loops
            // inside the function go round its number of times.
            level(
                "fn-hills",
                &[
                    "@@@@@@@*.@@@@",
                    "@@*.@@....@@@",
                    "@......@@..*.",
                    "R.@@..@@@@...",
                ],
            )
            .with_functions(3, 1, 4)
            .with_numbers(),
        ],
    },
    // Rain clouds count down 3, 2, 1, then rain for a bit, and start again.
    // Every step takes a tick, so sleep while it rains (or hurry
    // across before it starts). Don't get wet!
    Chapter {
        name: "Rain",
        levels: &[
            level("rain-demo", &["R.2.*"]).with_sleep(3).demo("R Z3 R3"),
            level("rain-1", &["R1.*"]).with_sleep(2),
            // Hurry across the first one; sleep through the second.
            level("rain-2", &["R.12.*"]).with_sleep(2),
            level("rain-3", &["R3.1.*"]).with_sleep(2),
            level("rain-4", &["R12.3*"]).with_sleep(2),
            level("rain-5", &["R2.1.3.*"]).with_sleep(3),
            // Two ways round: which one keeps dry?
            level("rain-6", &["R@@@", ".2.*", "@1@."]).with_sleep(3),
            // Two ways to the star. Over the top, there are too many clouds
            // to get under before it rains (and no sleep block); along the
            // bottom, the ice slides all the way in one go.
            level("rain-ice", &[".1111.", "R@@@@*", "~~~~~."]),
            // The same sleep each time: a loop.
            level("rain-7", &["R1.1.1.1.*"]).with_sleep(2).with_loops(2),
            level("rain-8", &["R.2.2.2.2*"]).with_sleep(3).with_loops(3),
        ],
    },
    // Every block so far.
    Chapter {
        name: "All together",
        levels: &[
            level(
                "59",
                &["@*@**@*", "R.#.##.", "@@@@@@.", ".#.#...", "*@*@@*@"],
            )
            .with_axe()
            .with_ifs(3, TREE_STAR)
            .with_functions(3, 2, 6),
            level(
                "60",
                &[
                    "@@@@@@@..##.",
                    "@@@@@@@.@*@*",
                    "@@@..#.#@@@@",
                    "@@@.@@*@@@@@",
                    "R.#.@@@@@@@@",
                    "@*@*@@@@@@@@",
                ],
            )
            .with_axe()
            .with_ifs(5, TREE_STAR)
            .with_functions(5, 2, 6),
        ],
    },
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
    pub loops: bool,
    /// What the "if" block can look for (none: no "if" block).
    pub ifs: Vec<Sense>,
    pub whiles: bool,
    /// What "while" can look for.
    pub while_looks: Vec<Sense>,
    /// "While there's *no*...": the game's own levels have it with "not" for
    /// the "if" too, and made-up maps always.
    pub while_not: bool,
    /// The break block.
    pub breaks: bool,
    pub otherwise: bool,
    pub not: bool,
    pub functions: usize,
    pub fn_slots: usize,
    pub numbers: bool,
    pub sleep: bool,
    /// Where the mystery clouds are.
    pub mysteries: Vec<Pos>,
    /// Where the keys lie.
    pub keys: Vec<Pos>,
    /// What the save files call it (see `LevelDef::id`).
    pub id: &'static str,
    /// Its chapter (from 1), and its number in the chapter (from 1). Demos
    /// aren't numbered: they share the number of the level they come before.
    pub chapter: usize,
    pub number: usize,
    /// A demo's program, already written.
    pub demo: Option<Vec<Vec<Item>>>,
}

impl Level {
    pub fn parse(def: &LevelDef) -> Level {
        let mut level = Level::from_rows(def.map);
        level.id = def.id;
        level.slots = def.slots;
        level.chop = def.chop;
        level.repeat = def.repeat;
        level.loops = def.loops;
        level.ifs = def.ifs.to_vec();
        level.whiles = def.whiles;
        level.while_looks = def.while_looks.to_vec();
        // Every "while" in the game's own levels can be "while true".
        if def.whiles {
            level.while_looks.push(Sense::True);
        }
        level.breaks = def.breaks;
        level.otherwise = def.otherwise;
        level.not = def.not;
        level.while_not = def.not;
        level.functions = def.functions;
        level.fn_slots = def.fn_slots;
        level.numbers = def.numbers;
        level.sleep = def.sleep;
        level.demo = def.demo.map(parse_strips);
        // A demo's strips just fit its program.
        if let Some(strips) = &level.demo {
            level.slots = tiles(&strips[0]);
            level.fn_slots = strips[1..].iter().map(|f| tiles(f)).max().unwrap_or(0);
        }
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
                    '1'..='3' => Tile::Rain(ch as u8 - b'0'),
                    'w' => Tile::Water,
                    '=' => Tile::Bridge,
                    _ => Tile::Grass,
                });
                match ch {
                    'R' => start = Some(p),
                    '*' => stars.push(p),
                    '?' => mysteries.push(p),
                    'k' => keys.push(p),
                    '.' | '#' | '@' | '~' | 'D' | 'w' | '=' | '1'..='3' => {}
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
            loops: false,
            ifs: Vec::new(),
            whiles: false,
            while_looks: Vec::new(),
            breaks: false,
            while_not: false,
            otherwise: false,
            not: false,
            functions: 0,
            fn_slots: 0,
            numbers: false,
            sleep: false,
            mysteries,
            keys,
            id: "",
            chapter: 0,
            number: 0,
            demo: None,
        }
    }

    /// What the save file calls this level.
    pub fn key(&self) -> String {
        self.id.to_string()
    }

    /// Its chapter and number, as shown: `3-2`.
    pub fn label(&self) -> String {
        format!("{}-{}", self.chapter, self.number)
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
            clock: 0,
        }
    }

    /// Does the map have any of this tile?
    pub fn has(&self, tile: Tile) -> bool {
        self.tiles.contains(&tile)
    }

    /// One way the mysteries can turn out (`true` = tree), with `coin`
    /// tossed for each: any way at all, all trees or all grass included.
    pub fn random_outcome(&self, mut coin: impl FnMut() -> bool) -> Vec<bool> {
        (0..self.mysteries.len()).map(|_| coin()).collect()
    }

    /// Every way the mysteries can turn out. That doubles with each cloud,
    /// so it's only for checking the game's own levels (which have a few).
    #[cfg(test)]
    pub fn outcomes(&self) -> Vec<Vec<bool>> {
        let n = self.mysteries.len();
        (0..1u32 << n)
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
    /// `fell`: it stepped off a bridge at `from`, which fell in.
    Moved {
        from: Pos,
        star: bool,
        key: bool,
        unlocked: bool,
        fell: bool,
    },
    Chopped(Pos),
    /// Slept a tick.
    Slept,
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
    /// Ticks so far (one per action), round the rain clouds' cycle: all the
    /// clouds need to know.
    clock: u8,
}

impl World {
    /// Is there a `sense` (a tree, or a star) right next to the robot, in
    /// direction `dir`?
    pub fn sees(&self, sense: Sense, dir: Dir) -> bool {
        let (dx, dy) = dir.delta();
        let next = (self.pos.0 + dx, self.pos.1 + dy);
        match sense {
            Sense::Tree => self.tile(next) == Some(Tile::Tree),
            Sense::Rock => self.tile(next) == Some(Tile::Rock),
            Sense::Star => self.stars.contains(&next),
            Sense::Ice => self.tile(next) == Some(Tile::Ice),
            Sense::Key => self.keys.contains(&next),
            Sense::Door => matches!(self.tile(next), Some(Tile::Door | Tile::Open)),
            Sense::Grass => matches!(self.tile(next), Some(Tile::Grass | Tile::Stump)),
            Sense::Edge => self.tile(next).is_none(),
            Sense::True => true,
            // Rain passes, so it isn't a wall.
            Sense::Wall => match self.tile(next) {
                Some(Tile::Door) => self.held == 0,
                Some(tile) => !tile.passable(),
                None => true,
            },
        }
    }

    pub fn tile(&self, (x, y): Pos) -> Option<Tile> {
        let inside = x >= 0 && y >= 0 && x < self.w && y < self.h;
        inside.then(|| self.tiles[(y * self.w + x) as usize])
    }

    /// Can the robot drive onto `p`: an open tile (where it isn't raining),
    /// or a door it has a key for?
    fn can_enter(&self, p: Pos) -> bool {
        match self.tile(p) {
            Some(Tile::Door) => self.held > 0,
            Some(tile) => tile.passable() && !self.weather(p).is_some_and(Weather::raining),
            None => false,
        }
    }

    /// What the rain cloud at `p` is doing, if there's one there.
    pub fn weather(&self, p: Pos) -> Option<Weather> {
        let Some(Tile::Rain(start)) = self.tile(p) else {
            return None;
        };
        let phase = (RAIN_DRY - start + self.clock) % RAIN_CYCLE;
        Some(if phase < RAIN_DRY {
            Weather::Dry(RAIN_DRY - phase)
        } else {
            Weather::Raining(RAIN_CYCLE - phase)
        })
    }

    /// Is it raining from any of the clouds?
    pub fn raining(&self) -> bool {
        (0..self.h).any(|y| (0..self.w).any(|x| self.weather((x, y)).is_some_and(Weather::raining)))
    }

    /// Is the robot getting rained on?
    pub fn wet(&self) -> bool {
        self.weather(self.pos).is_some_and(Weather::raining)
    }

    /// Every action takes a tick, and the rain clouds count down. (Without
    /// any, time doesn't matter, so it doesn't pass: the same place is the
    /// same place, for spotting a "while" that would never end.)
    fn tick(&mut self) {
        if self.tiles.iter().any(|t| matches!(t, Tile::Rain(_))) {
            self.clock = (self.clock + 1) % RAIN_CYCLE;
        }
    }

    /// Do one instruction. `Err` means the robot got stuck: it bumped into
    /// something (or into rain), or swung the axe at something that isn't a
    /// tree. If it ends up under rain that starts, it's `wet`.
    ///
    /// A bump has to stop the program, not just leave the robot where it is:
    /// otherwise walking into a wall on purpose lines the robot up, and some
    /// levels (6-1, 8-1 and 11-2) can be done without the tool they teach.
    pub fn act(&mut self, cmd: Cmd) -> Result<Event, ()> {
        let Some(dir) = cmd.dir() else {
            self.tick();
            return Ok(Event::Slept);
        };
        let (dx, dy) = dir.delta();
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
                // Stepping off a bridge, it falls in behind the robot.
                let fell = self.tile(from) == Some(Tile::Bridge);
                if fell {
                    self.tiles[(from.1 * self.w + from.0) as usize] = Tile::Water;
                }
                self.pos = next;
                // On ice the robot keeps sliding until something stops it. A
                // locked door stops it too, key or no key. Sliding into
                // something isn't a bump: that's how it stops.
                while self.tile(self.pos) == Some(Tile::Ice)
                    && self.tile(step(self.pos)).is_some_and(Tile::passable)
                    && !self.weather(step(self.pos)).is_some_and(Weather::raining)
                {
                    self.pos = step(self.pos);
                }
                self.tick();
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
                    fell,
                })
            }
            (Cmd::Chop(_), Some(Tile::Tree)) => {
                self.tiles[(next.1 * self.w + next.0) as usize] = Tile::Stump;
                self.tick();
                Ok(Event::Chopped(next))
            }
            _ => Err(()),
        }
    }
}

/// The name of chapter `chapter` (from 1).
pub fn chapter_name(chapter: usize) -> &'static str {
    CHAPTERS.get(chapter.wrapping_sub(1)).map_or("", |c| c.name)
}

pub fn all_levels() -> Vec<Level> {
    let mut levels = Vec::new();
    for (c, chapter) in CHAPTERS.iter().enumerate() {
        let mut these: Vec<Level> = chapter.levels.iter().map(Level::parse).collect();
        // A demo shares the number of the level after it.
        let mut number = these.iter().filter(|l| l.demo.is_none()).count() + 1;
        for level in these.iter_mut().rev() {
            if level.demo.is_none() {
                number -= 1;
            }
            level.chapter = c + 1;
            level.number = number.max(1);
        }
        levels.extend(these);
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    /// Known solutions for the loop levels (too many loop programs to search).
    /// `R` move right, `cR` chop right, a number repeats a
    /// block, `[` opens a loop and `]4` closes it, running it 4 times.
    const LOOP_SOLUTIONS: &[(&str, &str)] = &[
        ("35", "[ R D ]3"),
        ("36", "[ D2 R2 U2 ]2"),
        ("37", "[ R D ]5 R"),
        ("38", "[ R2 D ]3 R2"),
        ("39", "[ U2 R2 D2 R2 ]2 U2 R2"),
        ("40", "[ cR R2 ]4"),
        ("73", "[ R U D R2 ]3"),
        ("stairs", "[ R U ]5"),
        ("rain-7", "[ Z3 R2 ]5"),
        ("rain-8", "R [ Z3 R2 ]4"),
    ];

    /// Known solutions for the "while" levels. `<gR` starts "while there's
    /// grass to the right", `<!tD` "while there's no tree below", and `<y`
    /// "while true"; `B` is a break.
    const WHILE_SOLUTIONS: &[(&str, &str)] = &[
        ("74", "<gR R ] cR <gR R ]"),
        ("75", "<!tD D ] cD <!tD D ]"),
        ("break-1", "<y R {gD B } ] D D <y L ]"),
        ("break-2", "<y D {gR B } ] R R <y D ]"),
    ];

    /// Known solutions for the function levels: the main strip, then each
    /// function after a `|`. `f1` calls the first function.
    const FUNCTION_SOLUTIONS: &[(&str, &str)] = &[
        ("44", "f1 f1 f1 | U2 R2 D2 R2"),
        ("45", "f1 R f1 R3 f1 | R U2 D2"),
        ("46", "f1 R f1 | [ R U ]3"),
        ("47", "[ f1 ]3 | cR R U2 D2 R"),
        ("fn2-towers", "f1 f2 f2 f1 f2 | U D R2 | U2 D2 R2"),
        ("48", "f1 f2 f2 f1 f2 f1 | U2 D2 R | cR R"),
        ("53", "f1 f2 f1 f2 | [ R U ]3 | [ R D ]3"),
        ("54", "f1 D2 f2 D2 f1 D | [ cR R ]4 | [ cL L ]4"),
        ("55", "[ f1 D2 ]3 | [ {tR cR } R ]4"),
        ("56", "[ f1 R ]5 D3 [ f1 L ]5 | {sU U D } {sD D U }"),
        ("57", "f1 D2 f1 D2 f1 | [ {sU U D } R ]4"),
        (
            "58",
            "[ f1 ]3 [ f2 ]2 [ f1 ]2 [ f2 ]3 [ f1 ]2 | {tR cR } R | {tD cD } D",
        ),
        (
            "59",
            "[ f1 ]6 D2 [ f2 ]6 | {tR cR } R {sU U D } | {tL cL } L {sD D U }",
        ),
        ("60", "f1 f2 f1 f2 f1 | [ {tR cR } R {sD D U } ]3 | U2 R"),
        // `f1n3` calls the function with the number 3, and `R#` inside it
        // moves right that many times.
        ("78", "f1n3 f1n2 f1n1 | R# D#"),
        ("79", "f1n3 f1n1 f1n2 | R2 U# D#"),
        // `]#` closes a loop that runs the function's number of times.
        ("fn-hills", "f1n2 f1n3 f1n1 | [ R U ]# [ R D ]#"),
    ];

    /// Known solutions for the "if" levels. `{tR` starts "if there's a tree to
    /// the right", `{sU` "if there's a star above", and `}` ends it.
    const IF_SOLUTIONS: &[(&str, &str)] = &[
        ("41", "[ {tR cR } R ]9"),
        ("42", "[ {sU U D } R ]8"),
        ("43", "[ {tR cR } {sU U D } R ]8"),
        // Through the mystery clouds: look, chop if it's a tree, then step.
        ("49", "[ {tR cR } R2 ]4"),
        ("50", "[ {tR cR } R D ]4"),
        ("51", "[ {tR cR } R {tD cD } D ]4"),
        ("52", "[ {sD D U } R2 D ]5"),
        // `/` is "else".
        ("76", "[ {tR D / R } ]8"),
        ("77", "[ {tD R / D } ]7"),
        ("else-rock", "[ {rR U / R } ]8"),
        // `{!tU` is "if there's no tree above".
        ("not-1", "[ {!tU U D } R ]8"),
        ("not-2", "[ {!rR R L } U ]6"),
    ];

    /// The levels with numbers, in order, leaving out the demos.
    fn numbered() -> Vec<Level> {
        all_levels()
            .into_iter()
            .filter(|l| l.demo.is_none())
            .collect()
    }

    /// The level the save files call `id`.
    fn by_id(id: &str) -> Level {
        all_levels()
            .into_iter()
            .find(|l| l.id == id)
            .unwrap_or_else(|| panic!("no level {id}"))
    }

    /// Check every level `has` picks out has a solution in `solutions`, and
    /// that each solution fits and always wins.
    fn check_solutions(has: impl Fn(&Level) -> bool, solutions: &[(&str, &str)]) {
        let mut levels: Vec<&str> = numbered().iter().filter(|l| has(l)).map(|l| l.id).collect();
        let mut solved: Vec<&str> = solutions.iter().map(|&(id, _)| id).collect();
        levels.sort();
        solved.sort();
        assert_eq!(levels, solved, "every level needs a solution");
        for &(id, text) in solutions {
            let level = by_id(id);
            let strips = parse_strips(text);
            assert_eq!(strips.len(), 1 + level.functions, "level {id}: strips");
            assert!(tiles(&strips[0]) <= level.slots, "level {id}: too long");
            for f in &strips[1..] {
                assert!(tiles(f) <= level.fn_slots, "level {id}: function too long");
                assert!(
                    !f.iter().any(|i| matches!(i, Item::Call(..))),
                    "level {id}: functions can't call functions"
                );
            }
            assert!(always_wins(&level, &strips), "level {id} solution fails");
        }
    }

    #[test]
    fn demos_show_a_program_that_wins() {
        let levels = all_levels();
        for (i, level) in levels.iter().enumerate() {
            let Some(strips) = &level.demo else { continue };
            let n = level.id;
            let next = &levels[i + 1];
            assert!(
                next.demo.is_none() && next.label() == level.label(),
                "demo {n} must come just before a level, sharing its number"
            );
            assert_eq!(strips.len(), 1 + level.functions, "demo {n}: strips");
            assert!(tiles(&strips[0]) <= level.slots, "demo {n}: too long");
            for f in &strips[1..] {
                assert!(tiles(f) <= level.fn_slots, "demo {n}: function too long");
                assert!(
                    !f.iter().any(|i| matches!(i, Item::Call(..))),
                    "demo {n}: functions can't call functions"
                );
            }
            assert!(
                level.mysteries.is_empty() || level.ifs.contains(&Sense::Tree),
                "demo {n}: mysteries need an \"if\" that looks for trees"
            );
            assert!(always_wins(level, strips), "demo {n} doesn't win");
        }
    }

    #[test]
    fn old_saves_still_line_up() {
        let levels = all_levels();
        let keys: Vec<String> = levels.iter().map(Level::key).collect();
        let unique: HashSet<&String> = keys.iter().collect();
        assert_eq!(unique.len(), keys.len(), "save keys must be unique");
        // Saves from when levels were numbered straight through, 1 to 79,
        // with a demo (`d13`) before each new block.
        for n in 1..=79 {
            assert!(keys.contains(&n.to_string()), "level {n} is gone");
        }
        for d in [1, 13, 17, 27, 35, 41, 42, 44, 48, 61, 67, 74, 76, 78] {
            assert!(keys.contains(&format!("d{d}")), "demo d{d} is gone");
        }
        // They're the same levels: 13 was the first axe level, and 48 the
        // first with two functions.
        let first_axe = by_id("13");
        assert!(first_axe.chop && (first_axe.w, first_axe.h) == (3, 1));
        assert_eq!(by_id("48").functions, 2);
        // Each chapter counts up from 1.
        for chapter in 1..=CHAPTERS.len() {
            let numbers: Vec<usize> = numbered()
                .iter()
                .filter(|l| l.chapter == chapter)
                .map(|l| l.number)
                .collect();
            assert_eq!(numbers, (1..=numbers.len()).collect::<Vec<_>>());
        }
    }

    /// Does this program win however the mysteries turn out?
    fn always_wins(level: &Level, strips: &[Vec<Item>]) -> bool {
        level.outcomes().iter().all(|outcome| {
            let mut world = level.revealed(outcome);
            for exec in expand(strips, &world.clone()) {
                if let Step::Do(cmd) = exec.step {
                    if world.act(cmd).is_err() || world.wet() {
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
        check_solutions(
            |l| l.loops && l.functions == 0 && l.ifs.is_empty() && !l.whiles,
            LOOP_SOLUTIONS,
        );
    }

    #[test]
    fn function_levels_have_a_solution_that_fits() {
        check_solutions(|l| l.functions > 0, FUNCTION_SOLUTIONS);
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
                    if world.wet() {
                        return None;
                    }
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
    /// (If the level can, it also tries "if there's a tree, chop it", as one
    /// piece: see `fits_with`.)
    fn fits_with_loops(level: &Level, slots: usize) -> Option<Vec<Item>> {
        fits_with(level, slots, &[], true)
    }

    /// "If there's a tree that way { chop it }", the way through a mystery
    /// cloud: two tiles.
    fn if_chop(dir: Dir) -> [Item; 3] {
        [
            Item::IfStart {
                sense: Sense::Tree,
                dir,
                not: false,
            },
            Item::Block(Block {
                cmd: Cmd::Chop(dir),
                times: 1,
            }),
            Item::IfEnd,
        ]
    }

    /// Can the level use `if_chop`?
    fn can_if_chop(level: &Level) -> bool {
        level.chop && level.ifs.contains(&Sense::Tree)
    }

    /// The same as `fits_with_loops`, with any "if" blocks in `ifs` (each an
    /// `IfStart`, with no "else") inside the loops too, and `if_chop` pieces
    /// if `chop_trees` and the level can.
    fn fits_with(level: &Level, slots: usize, ifs: &[Item], chop_trees: bool) -> Option<Vec<Item>> {
        let mut blocks: Vec<Vec<Item>> = Vec::new();
        for dir in Dir::ALL {
            let mut cmds = vec![Cmd::Move(dir)];
            if level.chop {
                cmds.push(Cmd::Chop(dir));
            }
            if level.sleep && dir == Dir::Up {
                cmds.push(Cmd::Sleep);
            }
            for cmd in cmds {
                for times in 1..=MAX_TIMES {
                    blocks.push(vec![Item::Block(Block { cmd, times })]);
                }
            }
            if chop_trees && can_if_chop(level) {
                blocks.push(if_chop(dir).to_vec());
            }
        }

        struct Search<'a> {
            level: &'a Level,
            /// Blocks, and pieces of a few items that go in together.
            blocks: Vec<Vec<Item>>,
            ifs: &'a [Item],
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
                // An "if" can't go inside another one.
                let open_if = items
                    .iter()
                    .rposition(|i| matches!(i, Item::IfStart { .. }))
                    .filter(|&k| !items[k..].contains(&Item::IfEnd));
                for b in 0..self.blocks.len() {
                    let block = self.blocks[b].clone();
                    let has_if = block.iter().any(|i| matches!(i, Item::IfStart { .. }));
                    if has_if && open_if.is_some() {
                        continue;
                    }
                    if self.try_items(items, &block, open) {
                        return true;
                    }
                }
                // Inside a loop, start an "if", or end one with something in
                // it. (One left open ends with the loop.)
                if open.is_some() {
                    match open_if {
                        None => {
                            for &start_if in self.ifs {
                                if self.try_item(items, start_if, open) {
                                    return true;
                                }
                            }
                        }
                        Some(k) if tiles(&items[k..]) >= 2 => {
                            if self.try_item(items, Item::IfEnd, open) {
                                return true;
                            }
                        }
                        Some(_) => {}
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
                self.try_items(items, &[item], open)
            }

            fn try_items(
                &mut self,
                items: &mut Vec<Item>,
                add: &[Item],
                open: Option<usize>,
            ) -> bool {
                let len = items.len();
                if tiles(items) + tiles(add) > self.slots {
                    return false;
                }
                items.extend_from_slice(add);
                let found = match run(self.level, items) {
                    Some(worlds) => self.from(items, open, worlds),
                    None => false,
                };
                if !found {
                    items.truncate(len);
                }
                found
            }
        }

        let mut search = Search {
            level,
            blocks,
            ifs,
            slots,
            seen: HashMap::new(),
        };
        let mut items = Vec::new();
        let start = run(level, &items)?;
        search.from(&mut items, None, start).then_some(items)
    }

    #[test]
    fn function_levels_need_a_function() {
        for level in numbered().iter() {
            if level.functions > 0
                && let Some(program) = fits_with_loops(level, level.slots)
            {
                panic!(
                    "level {} fits in {} slots with loops alone: {program:?}",
                    level.id, level.slots
                );
            }
        }
    }

    #[test]
    fn if_levels_have_a_solution_that_fits() {
        check_solutions(
            |l| !l.ifs.is_empty() && l.loops && l.functions == 0,
            IF_SOLUTIONS,
        );
    }

    #[test]
    fn else_levels_need_else() {
        for level in numbered().iter() {
            if level.otherwise {
                let mut ifs = Vec::new();
                for dir in Dir::ALL {
                    for &sense in &level.ifs {
                        ifs.push(Item::IfStart {
                            sense,
                            dir,
                            not: false,
                        });
                    }
                }
                if let Some(program) = fits_with(level, level.slots, &ifs, true) {
                    panic!("level {} fits without else: {program:?}", level.id);
                }
            }
        }
    }

    #[test]
    fn while_levels_have_a_solution_that_fits() {
        check_solutions(|l| l.whiles, WHILE_SOLUTIONS);
    }

    #[test]
    fn while_levels_need_while() {
        for level in numbered().iter().filter(|l| l.whiles) {
            if level.loops
                && let Some(program) = fits_with_loops(level, level.slots)
            {
                panic!("level {} can be done without while: {program:?}", level.id);
            }
            // Without counts, going the long way round doesn't fit.
            if let Some(blocks) = fewest_blocks(level, Allow::level(level)) {
                assert!(
                    blocks > level.slots,
                    "level {} fits in plain blocks",
                    level.id
                );
            }
        }
    }

    /// Can a function level be won with a function that has no number in it?
    /// Tries every function of plain blocks that fits, and for each, the
    /// shortest main program of blocks and calls (breadth-first). Loops
    /// aren't tried.
    fn fits_without_numbers(level: &Level) -> Option<String> {
        let mut blocks = Vec::new();
        for dir in Dir::ALL {
            for times in 1..=MAX_TIMES {
                blocks.push(Block {
                    cmd: Cmd::Move(dir),
                    times,
                });
            }
        }
        assert!(!level.chop && level.ifs.is_empty(), "only moves are tried");
        let mut functions: Vec<Vec<Block>> = blocks.iter().map(|&b| vec![b]).collect();
        for len in 1..level.fn_slots {
            let longer: Vec<Vec<Block>> = functions
                .iter()
                .filter(|f| f.len() == len)
                .flat_map(|f| blocks.iter().map(move |&b| [f.clone(), vec![b]].concat()))
                .collect();
            functions.extend(longer);
        }
        let start = level.world();
        for function in &functions {
            // Each step is a block, or a call (`None`).
            let steps: Vec<Option<Block>> =
                blocks.iter().copied().map(Some).chain([None]).collect();
            let mut seen = HashSet::from([start.clone()]);
            let mut frontier = vec![start.clone()];
            for _ in 0..level.slots {
                let mut next = Vec::new();
                for world in &frontier {
                    'step: for &step in &steps {
                        let run = match step {
                            Some(b) => vec![b],
                            None => function.clone(),
                        };
                        let mut w = world.clone();
                        for b in run {
                            for _ in 0..b.times {
                                if w.act(b.cmd).is_err() {
                                    continue 'step;
                                }
                                if w.stars.is_empty() {
                                    return Some(format!("{function:?}"));
                                }
                            }
                        }
                        if seen.insert(w.clone()) {
                            next.push(w);
                        }
                    }
                }
                frontier = next;
            }
        }
        None
    }

    #[test]
    fn number_levels_need_numbers() {
        // The search does find a plain function when there is one.
        let mut easy = Level::from_rows(&["R.*.*"]);
        (easy.slots, easy.functions, easy.fn_slots) = (2, 1, 1);
        assert!(fits_without_numbers(&easy).is_some());
        for level in numbered().iter() {
            if level.numbers
                && let Some(function) = fits_without_numbers(level)
            {
                panic!("level {} fits with the function {function}", level.id);
            }
        }
    }

    /// Can the level be won in its slots with blocks (counts up to 3, and
    /// chopping if the level has the axe) and "repeat until" loops looking
    /// for any of `senses`, with "not" or without as `nots` allows? A search
    /// that gives up on a program as soon as it loses.
    fn fits_with_while(level: &Level, senses: &[Sense], nots: &[bool]) -> bool {
        let mut pieces = Vec::new();
        for dir in Dir::ALL {
            for times in 1..=3 {
                pieces.push(Item::Block(Block {
                    cmd: Cmd::Move(dir),
                    times,
                }));
            }
            if level.chop {
                pieces.push(Item::Block(Block {
                    cmd: Cmd::Chop(dir),
                    times: 1,
                }));
            }
            for &sense in senses {
                for &not in nots {
                    pieces.push(Item::While { sense, dir, not });
                }
            }
        }
        fn search(level: &Level, pieces: &[Item], items: &mut Vec<Item>) -> bool {
            let Some(worlds) = run(level, items) else {
                return false;
            };
            if won(&worlds) {
                return true;
            }
            let open = items
                .iter()
                .rposition(|i| matches!(i, Item::While { .. }))
                .filter(|&k| !items[k..].contains(&Item::LoopEnd));
            if open.is_some_and(|k| tiles(&items[k..]) >= 2) {
                items.push(Item::LoopEnd);
                if search(level, pieces, items) {
                    return true;
                }
                items.pop();
            }
            if tiles(items) == level.slots {
                return false;
            }
            for &piece in pieces {
                if open.is_some() && matches!(piece, Item::While { .. }) {
                    continue;
                }
                items.push(piece);
                if search(level, pieces, items) {
                    return true;
                }
                items.pop();
            }
            false
        }
        search(level, &pieces, &mut Vec::new())
    }

    #[test]
    fn while_levels_need_what_they_look_for() {
        // Break levels need an "if" too, which this search doesn't try.
        for level in numbered().iter().filter(|l| l.whiles && !l.breaks) {
            let nots: &[bool] = if level.while_not {
                &[false, true]
            } else {
                &[false]
            };
            assert!(
                fits_with_while(level, &level.while_looks, nots),
                "level {} can't be done",
                level.id
            );
            // "Not" isn't checked: "while there's a tree below { chop it }"
            // works like an "if" wherever a cloud is, so a level can always
            // be done that way too.
        }
    }

    #[test]
    fn while_goes_on_while_it_sees_it() {
        let world = Level::parse(&level("test", &["R...@", ".#..*"])).world();
        let steps = |text: &str| -> Vec<String> {
            expand(&[parse_program(text)], &world)
                .iter()
                .map(|e| format!("{:?}", e.step))
                .collect()
        };
        let looks = |yes| {
            format!(
                "{:?}",
                Step::Test {
                    sense: Sense::Grass,
                    dir: Dir::Right,
                    yes
                }
            )
        };
        let right = "Do(Move(Right))".to_string();
        let yes = looks(true);
        assert_eq!(
            steps("<gR R ]"),
            [&yes, &right, &yes, &right, &yes, &right, &looks(false)].map(String::clone)
        );
        // With "not": while there's no rock to the right, the same way.
        assert_eq!(steps("<!rR R ]").len(), 7);
        // It looks first, so with no grass there nothing runs.
        let below = Level::parse(&level("test", &["R", "@", "*"])).world();
        assert_eq!(expand(&[parse_program("<gD D ] R")], &below).len(), 2);
        // Going nowhere would go on for ever, so it stops.
        let stuck = steps("<gR D U ]");
        assert_eq!(stuck.last().unwrap(), "Forever");
        assert_eq!(stuck.len(), 5, "looks, down, up, looks again, gives up");
    }

    #[test]
    fn break_leaves_the_loop_it_is_in() {
        let world = Level::parse(&level("test", &["R....", "@@@.@", "@@@*@"])).world();
        let cmds = |text: &str| -> Vec<String> {
            expand(&[parse_program(text)], &world)
                .iter()
                .filter_map(|e| match e.step {
                    Step::Do(cmd) => Some(format!("{cmd:?}")),
                    Step::Break => Some("Break".into()),
                    _ => None,
                })
                .collect()
        };
        let (r, d) = ("Move(Right)", "Move(Down)");
        // A counted loop, and "while true", both stop at the gap.
        assert_eq!(cmds("[ R {gD B } ]4 D2"), [r, r, r, "Break", d, d]);
        assert_eq!(cmds("<y R {gD B } ] D D"), [r, r, r, "Break", d, d]);
        // "While true" with no break goes until it's back where it was.
        let world = Level::parse(&level("test", &["R.", ".*"])).world();
        let steps = expand(&[parse_program("<y R L ]")], &world);
        assert_eq!(steps.last().unwrap().step, Step::Forever);
        // A break outside a loop in a function ends just the function.
        let world = Level::parse(&level("test", &["R..*"])).world();
        let strips = parse_strips("f1 R | R B R");
        assert_eq!(expand(&strips, &world).len(), 3, "R, break, R");
    }

    #[test]
    fn else_runs_when_the_if_says_no() {
        let world = Level::parse(&level("test", &["R#.", "...", "..*"])).world();
        let moves = |text: &str| -> Vec<String> {
            expand(&[parse_program(text)], &world)
                .iter()
                .filter_map(|e| match e.step {
                    Step::Do(cmd) => Some(format!("{cmd:?}")),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(moves("{tR D / R }"), ["Move(Down)"]);
        assert_eq!(moves("{tD D / R }"), ["Move(Right)"], "no tree below");
        assert_eq!(moves("{tR / R } D"), ["Move(Down)"], "nothing for yes");
    }

    #[test]
    fn functions_use_the_number_they_are_called_with() {
        let strips = parse_strips("f1n2 f1n3 f1 | R# [ U ]#");
        assert_eq!(strips[0][0], Item::Call(1, 2));
        let cmds = moves(&strips);
        let count = |c: &str| cmds.iter().filter(|m| *m == c).count();
        assert_eq!(count("Move(Right)"), 2 + 3 + 1);
        assert_eq!(count("Move(Up)"), 2 + 3 + 1);
        // A number in the main program has no function to come from: it's 1.
        assert_eq!(moves(&parse_strips("R#")).len(), 1);
    }

    #[test]
    fn if_levels_need_an_if() {
        for level in numbered().iter() {
            if level.ifs.is_empty() {
                continue;
            }
            if level.loops {
                if let Some(program) = fits_with(level, level.slots, &[], false) {
                    panic!("level {} fits without an if: {program:?}", level.id);
                }
            } else {
                let allow = Allow {
                    if_chop: false,
                    ..Allow::level(level)
                };
                assert!(
                    !fits_in_blocks(level, allow),
                    "level {} can be done without an if",
                    level.id
                );
            }
        }
    }

    #[test]
    fn ifs_only_run_when_the_robot_sees_something() {
        let world = Level::parse(&level("test", &["*..", ".R#", "..."])).world();
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
        for &(id, _) in LOOP_SOLUTIONS {
            let level = by_id(id);
            assert!(fits_with_loops(&level, level.slots).is_some(), "level {id}");
        }
    }

    /// A big empty field with a far-off star, for unrolling programs.
    fn open_field() -> World {
        let mut rows = vec!["...............".to_string(); 15];
        rows[0].replace_range(0..1, "*");
        rows[7].replace_range(7..8, "R");
        let rows: Vec<&'static str> = rows.into_iter().map(|r| &*r.leak()).collect();
        Level::parse(&level("test", rows.leak())).world()
    }

    #[test]
    fn simulation_keeps_the_rest_of_the_program() {
        // R8 bumps into the edge on its 8th go, from the middle of the field.
        let strips = [parse_program("R8 D2 {sU U }")];
        let sim = simulate(&strips, &open_field());
        assert_eq!(sim.ran, 8);
        assert_eq!(expand(&strips, &open_field()).len(), 8);
        // After that nothing moves, but the rest is still there.
        assert_eq!(sim.trace.len(), 11);
        assert_eq!(sim.worlds.len(), 12);
        assert!(sim.worlds[8..].iter().all(|w| w.pos == (14, 7)));
        // A "while" after the end goes round once (look, D, look),
        // since nothing moves, then the program goes on.
        let strips = [parse_program("R8 <gU D ] L")];
        let sim = simulate(&strips, &open_field());
        assert_eq!((sim.ran, sim.trace.len()), (8, 12));
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
        for level in numbered().iter() {
            if level.loops {
                let without = fewest_blocks(level, Allow::level(level)).unwrap_or(usize::MAX);
                assert!(
                    without > level.slots,
                    "level {} fits in {without} blocks without a loop",
                    level.id
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
        /// "If there's a tree that way { chop it }" (two tiles).
        if_chop: bool,
        sleep: bool,
    }

    impl Allow {
        fn level(level: &Level) -> Allow {
            Allow {
                repeat: level.repeat,
                chop: level.chop,
                if_chop: can_if_chop(level),
                sleep: level.sleep,
            }
        }
    }

    /// Fewest tiles in a single program of blocks (and "if tree, chop" pieces)
    /// that collects every star however the mysteries turn out: a search
    /// outwards, a tile at a time, over every block allowed.
    fn fewest_blocks(level: &Level, allow: Allow) -> Option<usize> {
        /// A block, or "if there's a tree that way, chop it".
        #[derive(Clone, Copy)]
        enum Piece {
            Do(Cmd),
            IfChop(Dir),
        }
        let mut steps: Vec<Piece> = Dir::ALL.map(|d| Piece::Do(Cmd::Move(d))).to_vec();
        if allow.chop {
            steps.extend(Dir::ALL.map(|d| Piece::Do(Cmd::Chop(d))));
        }
        if allow.if_chop {
            steps.extend(Dir::ALL.map(Piece::IfChop));
        }
        if allow.sleep {
            steps.push(Piece::Do(Cmd::Sleep));
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
        // Positions to search on from, by how many tiles it took to get
        // there; a piece takes one tile or two, so the cheapest come first.
        let mut best = HashMap::from([(start.clone(), 0)]);
        let mut by_cost: Vec<Vec<Worlds>> = vec![vec![start]];
        let mut won: Option<usize> = None;
        let mut cost = 0;
        while cost < by_cost.len() {
            // Every cheaper way has been tried.
            if won.is_some_and(|w| w <= cost) {
                return won;
            }
            for worlds in std::mem::take(&mut by_cost[cost]) {
                if best[&worlds] < cost {
                    continue;
                }
                for &step in &steps {
                    let (tiles, times) = match step {
                        Piece::Do(_) => (1, max_times),
                        Piece::IfChop(_) => (2, 1),
                    };
                    let mut next = worlds.clone();
                    'reps: for _ in 0..times {
                        for world in next.iter_mut().flatten() {
                            let did = match step {
                                Piece::Do(cmd) => world.act(cmd).map(|_| ()),
                                Piece::IfChop(dir) if world.sees(Sense::Tree, dir) => {
                                    world.act(Cmd::Chop(dir)).map(|_| ())
                                }
                                Piece::IfChop(_) => Ok(()),
                            };
                            if did.is_err() || world.wet() {
                                break 'reps;
                            }
                        }
                        for slot in next.iter_mut() {
                            if slot.as_ref().is_some_and(|w| w.stars.is_empty()) {
                                *slot = None;
                            }
                        }
                        let total = cost + tiles;
                        if next.iter().all(Option::is_none) {
                            won = Some(won.map_or(total, |w| w.min(total)));
                            break 'reps;
                        }
                        if best.get(&next).is_none_or(|&b| total < b) {
                            best.insert(next.clone(), total);
                            if by_cost.len() <= total {
                                by_cost.resize(total + 1, Vec::new());
                            }
                            by_cost[total].push(next.clone());
                        }
                    }
                }
            }
            cost += 1;
        }
        won
    }

    #[test]
    fn every_level_is_solvable_within_its_slots() {
        for level in numbered().iter() {
            let n = level.id;
            assert!(!level.stars.is_empty(), "level {n} has no stars");
            assert!(level.slots <= MAX_PROGRAM, "level {n} has too many slots");
            // Clouds need a way to deal with them: looking for a tree, or
            // going on until the robot gets to them.
            assert!(
                level.mysteries.is_empty() || level.ifs.contains(&Sense::Tree) || level.whiles,
                "level {n}: mysteries need a way to look at them"
            );
            // "Repeat until" levels can only be done with one; their known
            // solutions are checked in `while_levels_have_a_solution_that_fits`.
            if level.whiles {
                continue;
            }
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
        for level in numbered().iter() {
            if level.repeat {
                let allow = Allow {
                    repeat: false,
                    ..Allow::level(level)
                };
                let without = fewest_blocks(level, allow).unwrap_or(usize::MAX);
                assert!(
                    without > level.slots,
                    "level {} fits in {without} blocks without repeat",
                    level.id
                );
            }
        }
    }

    #[test]
    fn axe_levels_need_the_axe() {
        for level in numbered().iter() {
            if level.chop {
                let allow = Allow {
                    chop: false,
                    if_chop: false,
                    ..Allow::level(level)
                };
                assert_eq!(
                    fewest_blocks(level, allow),
                    None,
                    "level {} can be done without the axe",
                    level.id
                );
            }
        }
    }

    /// Can the level be won with a program of plain blocks? Where loops
    /// could squash a long one down, any at all; with no loops (and no
    /// counts), only one that fits in its slots.
    fn fits_in_blocks(level: &Level, allow: Allow) -> bool {
        let blocks = fewest_blocks(level, allow);
        if level.loops {
            blocks.is_some()
        } else {
            blocks.is_some_and(|b| b <= level.slots)
        }
    }

    #[test]
    fn cloud_levels_need_to_look() {
        for level in numbered().iter() {
            if !level.mysteries.is_empty() {
                let allow = Allow {
                    if_chop: false,
                    ..Allow::level(level)
                };
                assert!(
                    !fits_in_blocks(level, allow),
                    "level {} can be done without looking",
                    level.id
                );
            }
        }
    }

    #[test]
    fn mysteries_can_come_out_any_way() {
        let pair = Level::parse(&level("test", &["R??"]).with_tree_if());
        assert_eq!(pair.outcomes().len(), 4, "both trees, both grass too");
        let single = Level::parse(&level("test", &["R?"]).with_tree_if());
        assert_eq!(single.outcomes(), vec![vec![false], vec![true]]);
        assert_eq!(pair.random_outcome(|| true), [true, true]);
    }

    #[test]
    fn robot_bumps_and_chops() {
        let mut world = Level::parse(&level("test", &["R#", ".@"])).world();
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

    #[test]
    fn ice_slides_until_something_stops_it() {
        let mut world = Level::parse(&level("test", &["R~~.~~@", "~~~~~~~", "*~~~~~~"])).world();
        let moved = |from| Event::Moved {
            from,
            star: false,
            key: false,
            unlocked: false,
            fell: false,
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
    fn bridges_hold_the_robot_once() {
        let mut world = Level::parse(&level("test", &["R=.", "ww.", "*=."])).world();
        assert!(world.act(Cmd::Move(Dir::Down)).is_err(), "water");
        world.act(Cmd::Move(Dir::Right)).unwrap();
        let off = world.act(Cmd::Move(Dir::Right));
        assert!(matches!(off, Ok(Event::Moved { fell: true, .. })));
        assert_eq!(world.tile((1, 0)), Some(Tile::Water), "it fell in");
        assert!(world.act(Cmd::Move(Dir::Left)).is_err(), "no way back");
        // A slide stops on a bridge, like on grass.
        let mut world = Level::parse(&level("test", &["R~~=~*"])).world();
        world.act(Cmd::Move(Dir::Right)).unwrap();
        assert_eq!(world.pos, (3, 0));
    }

    #[test]
    fn keys_open_doors() {
        let mut world = Level::parse(&level("test", &["kRD.", "..D*"])).world();
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
        let mut world = Level::parse(&level("test", &["kR~~D*"])).world();
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
        for level in numbered().iter() {
            if !level.keys.is_empty() {
                let mut without = level.clone();
                without.keys.clear();
                without.slots = MAX_PROGRAM;
                without.repeat = true;
                assert_eq!(
                    fewest_blocks(&without, Allow::level(&without)),
                    None,
                    "level {} can be done without a key",
                    level.id
                );
            }
        }
    }

    #[test]
    fn rain_counts_down_then_rains() {
        let mut world = Level::parse(&level("test", &["R2..*"])).world();
        assert_eq!(world.weather((1, 0)), Some(Weather::Dry(2)));
        world.act(Cmd::Sleep).unwrap();
        assert_eq!(world.weather((1, 0)), Some(Weather::Dry(1)));
        world.act(Cmd::Sleep).unwrap();
        assert_eq!(world.weather((1, 0)), Some(Weather::Raining(2)));
        assert!(
            world.act(Cmd::Move(Dir::Right)).is_err(),
            "can't drive into rain"
        );
        world.act(Cmd::Sleep).unwrap();
        world.act(Cmd::Sleep).unwrap();
        assert_eq!(
            world.weather((1, 0)),
            Some(Weather::Dry(3)),
            "and round again"
        );
        // Onto it with 3 to go, and still dry after the step...
        world.act(Cmd::Move(Dir::Right)).unwrap();
        assert!(!world.wet());
        // ...but standing there until it rains is getting wet.
        world.act(Cmd::Sleep).unwrap();
        assert!(!world.wet());
        world.act(Cmd::Sleep).unwrap();
        assert!(world.wet());
    }

    #[test]
    fn rain_levels_need_sleep() {
        for level in numbered().iter().filter(|l| l.sleep) {
            let mut without = level.clone();
            without.sleep = false;
            if level.loops {
                if let Some(program) = fits_with_loops(&without, level.slots) {
                    panic!("level {} fits without sleeping: {program:?}", level.id);
                }
            } else {
                let tiles = fewest_blocks(&without, Allow::level(&without)).unwrap_or(usize::MAX);
                assert!(
                    tiles > level.slots,
                    "level {} fits in {tiles} without sleeping",
                    level.id
                );
            }
        }
    }

    #[test]
    fn rain_ice_needs_the_ice() {
        // Over the top, under the clouds, can't be done, even with sleep.
        let mut level = by_id("rain-ice");
        for x in 0..level.w {
            let i = (2 * level.w + x) as usize;
            if level.tiles[i] == Tile::Ice {
                level.tiles[i] = Tile::Rock;
            }
        }
        (level.sleep, level.repeat) = (true, true);
        assert_eq!(fewest_blocks(&level, Allow::level(&level)), None);
    }

    #[test]
    fn not_levels_need_not() {
        for level in numbered().iter().filter(|l| l.not) {
            let mut ifs = Vec::new();
            for dir in Dir::ALL {
                for &sense in &level.ifs {
                    ifs.push(Item::IfStart {
                        sense,
                        dir,
                        not: false,
                    });
                }
            }
            if let Some(program) = fits_with(level, level.slots, &ifs, true) {
                panic!("level {} fits without not: {program:?}", level.id);
            }
        }
    }

    #[test]
    fn not_turns_an_if_round() {
        let world = Level::parse(&level("test", &["R#", "..", "*."])).world();
        let moves = |text: &str| -> Vec<String> {
            expand(&[parse_program(text)], &world)
                .iter()
                .filter_map(|e| match e.step {
                    Step::Do(cmd) => Some(format!("{cmd:?}")),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(moves("{!tR D } {!tD D }"), ["Move(Down)"]);
        // Down past the tree, then no tree to the right: so right.
        assert_eq!(moves("{tR D } {!tR R }"), ["Move(Down)", "Move(Right)"]);
    }
}
