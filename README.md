# Robot Steps

A first coding game for young children (made for a 5-year-old). Tap arrows to
build a program, press play, and watch the robot follow it to collect the
stars. There's no reading required.

Across 40 levels it introduces:

- **Sequences:** up, down, left and right, in order
- **Debugging:** when the robot bumps into something, the step that went wrong
  shakes red, and the program stays put to be fixed
- **Actions:** an axe block chops down trees (but not rocks)
- **Repeat:** run one block several times when there aren't enough slots
- **Conditionals:** mystery clouds become a tree or grass each run, so the
  robot has to check before it chops
- **Loops:** repeat a whole group of blocks

Finished levels are saved, and the game carries on from the first unfinished one.

## Running

Needs [Rust](https://rustup.rs).

```bash
cargo run --release              # full screen (F11 or Esc to leave)
cargo run --release -- --window  # in a window
```

On Linux, macroquad needs the usual X11/OpenGL libraries (`libx11`, `libxi`,
`libgl`).

### Windows (from Linux or WSL)

```bash
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
# -> target/x86_64-pc-windows-gnu/release/robot_steps.exe
```

Under WSL the game runs on the GPU, but WSL caps full screen at 4K to about
40 fps. The native Windows build runs at a steady 60.

## Keyboard (for grown-ups)

| Key | Does |
|---|---|
| Arrows | Add a move |
| C, then an arrow | Chop that way |
| Q, then an arrow | If there's a tree that way, chop it |
| R / 2-9 | Repeat the last block (or loop) |
| L | Open or close a loop |
| Space / Enter | Play or stop |
| Backspace / Delete | Undo / clear |
| F11 | Toggle full screen |

## Levels

Levels live in `src/level.rs` as small text maps: `R` robot, `*` star,
`#` tree, `@` rock, `?` mystery and `.` grass. `cargo test` proves that every
level can be solved within its slots, and that each tool is actually needed
where it's introduced.
