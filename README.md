# Robot Steps

A first coding game for young children (made for a 5-year-old). Tap arrows to
build a program, press play, and watch the robot follow it to collect the
stars. There's no reading required.

**[Play it in your browser](https://plwg.github.io/robot-steps/)**

![The level map: finished levels have a star, and the player's robot waits on the next one](docs/map.png)

| Choose a player | Playing a level |
|---|---|
| ![Three players, each with their own robot and progress](docs/players.png) | ![Level 48: the main program calls the ● and ▲ functions, and the robot is partway through ▲, chopping a tree](docs/level.png) |

Across 82 levels in 14 chapters, numbered within each chapter (`3-2`), it
introduces, in the order programming is usually taught:

1. **Moving:** up, down, left and right, in order
2. **Ice:** the robot slides until grass, a rock or the edge stops it, so it
   has to think ahead about where it will end up (and sometimes go the long
   way round)
3. **The axe:** chop down trees (but not rocks)
4. **Keys and doors:** a locked door needs a key, and each key opens one
   door. The robot remembers what it's carrying, so the order matters: fetch
   the key first, even when it's the other way
5. **Repeat:** run one block several times when there aren't enough slots
6. **If:** mystery clouds become a tree or grass each run, so the robot has
   to look before it chops: "if there's a tree to the right { chop it }". Tap
   "if", pick what to look for (shown as map tiles: a tree, rock, star, ice,
   key, door, grass, or the edge of the map, whichever the level offers),
   then which way. The blocks to run go inside its brackets
7. **Loops:** repeat a whole group of blocks
8. **If inside loops:** "if there's a tree to the right { chop it }" or "if
   there's a star above { fetch it }" in a loop: one rule handles whatever
   the robot finds, wherever it finds it
9. **Else:** "if there's a tree to the right { go down } otherwise { go
   right }". The grey ✗ between the blocks marks where the "otherwise" part
   starts
10. **Repeat until:** "keep going until there's a wall that way". The robot
    looks before each time round. Mystery clouds decide how far it is, so no
    fixed number of times would do. One that would go round for ever (back
    where it started, and still no wall) stops and shakes red
11. **Functions:** extra strips of blocks named with a shape (● and ▲) that
    the main program calls whenever it needs them
12. **Functions that look:** "if" blocks inside functions
13. **Functions with a number:** call ● with a number (set with repeat), and
    inside it a count can be "●'s number" instead of a fixed one, so one
    function makes stairs of 1, 2 and 3
14. **Everything together**

Throughout, **debugging**: when the robot bumps into something, the step
that went wrong shakes red, and the program stays put to be fixed. Tapping a
block opens an empty slot before it for a missing step, and the eraser that
appears on a block under the pointer takes it out.

Each new block is first shown in a demo level (▶ on the map): the program is
already written, so just press play and watch how it's used. Then each new
tool starts on a map from earlier in the game, with too few slots
to solve it the old way, before moving on to new maps.

Every action has a sound, and each direction plays its own note, so a
program plays a little tune. The speaker in the corner turns the sound off.

Three players each have their own robot colour and their own progress. A
level map shows which levels are finished, and the player's robot waits on
the next one. The game draws a big, easy-to-spot pointer.

## Making maps

The pencil button on the level map opens the map maker. Tap the green **+**
to start a map, then pick a tile on the right (grass, tree, rock, ice, mystery
cloud, key, door, star or the robot) and tap or drag across the map to paint
it. Under the map, − and + change its width and height and how many slots
the program has (and each function, when there are functions). The buttons
under the tiles choose the blocks it allows: the axe, repeat, loops, "repeat
until", "if", "else", the two functions and numbers for them. Some bring
others along, as in the game's own levels: loops need repeat; "repeat until"
and functions need loops; "else" needs an "if"; and numbers need a function.
Mystery clouds always come with "if" and the axe. The "if" can look for
anything painted on the map, grass, and the edge.

Press play to try the map out. The arrow at the top left goes back to
everyone's maps, where a map can be played, changed with the pencil, or
thrown away by tapping its bin twice. There's room for 8. They're shared by
all three players (in `maps.txt` next to the progress files, or the browser's
local storage), and finishing one doesn't count towards a player's levels.

## Running

Needs [Rust](https://rustup.rs).

```bash
cargo run --release              # full screen (F11 or Esc to leave)
cargo run --release -- --window  # in a window
```

On Linux, macroquad needs the usual X11/OpenGL libraries (`libx11`, `libxi`,
`libgl`), and ALSA for sound (`libasound2-dev`).

### Windows (from Linux or WSL)

```bash
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
# -> target/x86_64-pc-windows-gnu/release/robot_steps.exe
```

Under WSL the game runs on the GPU, but WSL caps full screen at 4K to about
40 fps. The native Windows build runs at a steady 60.

WSL also has no sound card, so the Linux build is silent there: ALSA reports
it can't open a PCM device, and each sound prints "Audio thread died". The
game still plays fine. For sound under WSL, route ALSA through WSLg's
PulseAudio server: install `libasound2-plugins` and put this in
`~/.asoundrc`:

```
pcm.!default { type pulse }
ctl.!default { type pulse }
```

### Web

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
mkdir -p site && cp web/* target/wasm32-unknown-unknown/release/robot_steps.wasm site/
python3 -m http.server -d site   # then open http://localhost:8000
```

`web/mq_js_bundle.js` is macroquad's loader, copied from the macroquad crate
(update it with macroquad). Its sound functions are linked by name, which
`.cargo/config.toml` allows. In the browser, each player's progress is kept
in local storage instead of a file. Every push to `master` publishes the web
version to GitHub Pages (`.github/workflows/pages.yml`).

## Keyboard (for grown-ups)

| Key | Does |
|---|---|
| Arrows | Add a move |
| C, then an arrow | Chop that way |
| R / 2-9 | Repeat the last block (or loop) |
| L | Open or close a loop |
| I, then a choice and an arrow | Start an "if" (again to close) |
| E | Else, inside an "if" |
| W, then an arrow | Start "repeat until there's a wall that way" (again to close) |
| N | In a function, make the last count the function's number |
| F / G | Call function ● / ▲ |
| Tab | Switch between the main program and the functions |
| Space / Enter | Play or stop |
| Backspace / Delete | Undo / clear |
| 1 / 2 / 3 | Pick a player |
| Enter (on the map) | Play the next unfinished level |
| M | Back to the map |
| F11 | Toggle full screen |

## Levels

Levels live in `src/level.rs`, in chapters, as small text maps: `R` robot,
`*` star, `#` tree, `@` rock, `?` mystery, `~` ice, `k` key, `D` locked door
and `.` grass. A demo level adds its program as text with `.demo(...)`.
Each level has a name for the save files that never changes, however the
levels are moved around: the levels used to be numbered straight through,
1 to 79, and those numbers are their names (`d13` for the demo before 13),
so old saves still work. New levels get a name instead. `cargo test` proves that every
level can be solved within its slots, and that each tool is actually needed
where it's introduced.

Bumping into a rock, a tree, a locked door or the edge of the map stops the
program; it isn't a harmless "stay put" (unlike sliding on ice into something,
which is just how the slide stops). The levels depend on this: if bumps did
nothing, the robot could walk into a wall on purpose to line itself up, and
some levels could be done without the tool they teach (levels 6-1 and 8-1
without an "if", 11-2 without a function).
