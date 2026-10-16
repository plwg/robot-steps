//! Level data and the rules of movement. No graphics here, so it can be tested.

/// The longest program the child can build (two rows of twelve slots).
pub const MAX_PROGRAM: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
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

/// Map legend: `.` grass, `#` tree (blocks the robot), `R` robot start, `*` star.
pub const LEVELS: &[&[&str]] = &[
    // 1. Just go right.
    &["R...*"],
    // 2. Just go down.
    &["R", ".", ".", "*"],
    // 3. Turn a corner.
    &["R..", "##.", "##*"],
    // 4. Left and up.
    &["*##", ".##", "..R"],
    // 5. Two stars in a row.
    &["R.*.*"],
    // 6. Go around the trees.
    &["R#*", ".#.", "..."],
    // 7. Stairs.
    &["R.##", "#..#", "##.*"],
    // 8. Stars on both sides.
    &["*...*", ".#.#.", "..R.."],
    // 9. A little maze.
    &["R.#..*", ".##.#.", "....#."],
    // 10. Three stars.
    &["*....*", ".####.", "...R.*"],
    // 11. Up and down the hills.
    &["R#*#*", ".#.#.", "....."],
    // 12. All four corners.
    &["*...*", ".#.#.", "..R..", ".#.#.", "*...*"],
];

pub struct Level {
    pub w: i32,
    pub h: i32,
    walls: Vec<bool>,
    pub start: Pos,
    pub stars: Vec<Pos>,
}

impl Level {
    pub fn parse(rows: &[&str]) -> Level {
        let h = rows.len() as i32;
        let w = rows[0].len() as i32;
        let mut walls = Vec::new();
        let mut start = None;
        let mut stars = Vec::new();
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(row.len() as i32, w, "level rows must be the same length");
            for (x, ch) in row.chars().enumerate() {
                let p = (x as i32, y as i32);
                walls.push(ch == '#');
                match ch {
                    'R' => start = Some(p),
                    '*' => stars.push(p),
                    '.' | '#' => {}
                    other => panic!("unknown level tile {other:?}"),
                }
            }
        }
        Level {
            w,
            h,
            walls,
            start: start.expect("level needs a robot (R)"),
            stars,
        }
    }

    pub fn is_wall(&self, (x, y): Pos) -> bool {
        self.walls[(y * self.w + x) as usize]
    }

    fn can_enter(&self, p: Pos) -> bool {
        p.0 >= 0 && p.1 >= 0 && p.0 < self.w && p.1 < self.h && !self.is_wall(p)
    }

    /// Where the robot ends up after one move, or `None` if it bumps into something.
    pub fn step(&self, (x, y): Pos, dir: Dir) -> Option<Pos> {
        let (dx, dy) = dir.delta();
        let next = (x + dx, y + dy);
        self.can_enter(next).then_some(next)
    }
}

pub fn all_levels() -> Vec<Level> {
    LEVELS.iter().map(|rows| Level::parse(rows)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashSet, VecDeque};

    const ALL_DIRS: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    /// Fewest moves needed to collect every star (breadth-first search).
    fn shortest_solution(level: &Level) -> Option<usize> {
        let all = (1u32 << level.stars.len()) - 1;
        let collect = |p: Pos, mask: u32| {
            level
                .stars
                .iter()
                .position(|&s| s == p)
                .map_or(mask, |i| mask | 1 << i)
        };
        let first = (level.start, collect(level.start, 0));
        let mut seen = HashSet::from([first]);
        let mut queue = VecDeque::from([(first, 0)]);
        while let Some(((pos, mask), dist)) = queue.pop_front() {
            if mask == all {
                return Some(dist);
            }
            for dir in ALL_DIRS {
                if let Some(next) = level.step(pos, dir) {
                    let state = (next, collect(next, mask));
                    if seen.insert(state) {
                        queue.push_back((state, dist + 1));
                    }
                }
            }
        }
        None
    }

    #[test]
    fn every_level_is_solvable_within_the_program_limit() {
        for (i, level) in all_levels().iter().enumerate() {
            assert!(!level.stars.is_empty(), "level {} has no stars", i + 1);
            let moves = shortest_solution(level)
                .unwrap_or_else(|| panic!("level {} cannot be solved", i + 1));
            assert!(moves <= MAX_PROGRAM, "level {} needs {moves} moves", i + 1);
        }
    }

    #[test]
    fn robot_bumps_into_trees_and_edges() {
        let level = Level::parse(&["R#", ".."]);
        assert_eq!(level.step((0, 0), Dir::Right), None);
        assert_eq!(level.step((0, 0), Dir::Up), None);
        assert_eq!(level.step((0, 0), Dir::Down), Some((0, 1)));
    }
}
