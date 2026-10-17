# Building and running

Needs [Rust](https://rustup.rs).

```bash
cargo run --release              # full screen (F11 or Esc to leave)
cargo run --release -- --window  # in a window
```

On Linux, macroquad needs the usual X11/OpenGL libraries (`libx11`, `libxi`,
`libgl`), and ALSA for sound (`libasound2-dev`).

Progress is saved per player in small text files (`progress-1.txt` to
`progress-3.txt`, bests in `challenge-1.txt` to `challenge-3.txt`, and made
maps in `maps.txt`): in `%APPDATA%\RobotSteps` on Windows, otherwise
`~/.local/share/robot_steps`. In the browser they're kept in local storage.

## Windows (from Linux or WSL)

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

## Web

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
mkdir -p site && cp web/* target/wasm32-unknown-unknown/release/robot_steps.wasm site/
python3 -m http.server -d site   # then open http://localhost:8000
```

`web/mq_js_bundle.js` is macroquad's loader, copied from the macroquad crate
(update it with macroquad). Its sound functions are linked by name, which
`.cargo/config.toml` allows. Every push to `master` publishes the web
version to GitHub Pages (`.github/workflows/pages.yml`).

## Levels

Levels live in `src/level.rs`, in chapters, as small text maps: `R` robot,
`*` star, `#` tree, `@` rock, `?` mystery, `~` ice, `k` key, `D` locked door,
`w` water, `=` a lily pad, `1` to `3` a rain cloud (counting down from that)
and `.` grass. A demo level adds its program as text with `.demo(...)`; see
`parse_strips` for how programs are written (`R` moves right, `[ R U ]3` is
a loop, `{tR cR }` an "if", `<gR R ]` a "while", `<y` "while true", `B` a
break, and so on).

Each level has a name for the save files that never changes, however the
levels are moved around: the levels used to be numbered straight through,
1 to 79, and those numbers are their names (`d13` for the demo before 13),
so old saves still work. New levels get a name instead.

`cargo test` proves that every level can be solved within its slots, however
the mystery clouds turn out (every way, all trees and all grass included),
and that each tool is actually needed where it's introduced. Two exceptions:
"not" with "while" (a "while there's a tree { chop it }" works like an "if",
so there's always a way without it), and break, whose levels are only
checked against plain blocks, not every mix of loops and "if"s.

Bumping into a rock, a tree, a locked door or the edge of the map stops the
program; it isn't a harmless "stay put" (unlike sliding on ice into
something, which is just how the slide stops). The levels depend on this: if
bumps did nothing, the robot could walk into a wall on purpose to line
itself up, and some levels could be done without the tool they teach (some
"if" levels without an "if", and a function level without a function).
