//! Which levels have been finished, for each of the three players, saved
//! between games as a list of level numbers in a small text file per player.

use std::path::PathBuf;

/// How many players (save slots) there are.
pub const SLOTS: usize = 3;

/// The folder the saves live in: `%APPDATA%\RobotSteps` on Windows,
/// otherwise `~/.local/share/robot_steps`.
fn dir() -> Option<PathBuf> {
    match std::env::var_os("APPDATA") {
        Some(appdata) => Some(PathBuf::from(appdata).join("RobotSteps")),
        None => {
            let data = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
                })?;
            Some(data.join("robot_steps"))
        }
    }
}

/// `progress-1.txt` to `progress-3.txt`, for slots 0 to 2.
fn path(slot: usize) -> Option<PathBuf> {
    Some(dir()?.join(format!("progress-{}.txt", slot + 1)))
}

/// Finished levels for one player, one flag per level. Missing or unreadable
/// progress just means nothing is finished yet. Before there were players,
/// progress was kept in `progress.txt`; that becomes the first player's.
pub fn load(slot: usize, levels: usize) -> Vec<bool> {
    let read = |p: Option<PathBuf>| p.and_then(|p| std::fs::read_to_string(p).ok());
    let text = read(path(slot))
        .or_else(|| {
            (slot == 0)
                .then(|| read(dir().map(|d| d.join("progress.txt"))))
                .flatten()
        })
        .unwrap_or_default();
    parse(&text, levels)
}

/// Save one player's finished levels. Failing to save shouldn't stop the
/// game, so errors are ignored.
pub fn save(slot: usize, solved: &[bool]) {
    if let Some(p) = path(slot) {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, format(solved));
    }
}

fn parse(text: &str, levels: usize) -> Vec<bool> {
    let mut solved = vec![false; levels];
    for n in text
        .split_whitespace()
        .filter_map(|w| w.parse::<usize>().ok())
    {
        if (1..=levels).contains(&n) {
            solved[n - 1] = true;
        }
    }
    solved
}

fn format(solved: &[bool]) -> String {
    let numbers: Vec<String> = (1..=solved.len())
        .filter(|&n| solved[n - 1])
        .map(|n| n.to_string())
        .collect();
    numbers.join(" ") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_ignores_junk() {
        let solved = [true, false, true, true];
        assert_eq!(format(&solved), "1 3 4\n");
        assert_eq!(parse(&format(&solved), 4), solved);
        assert_eq!(parse("2 oops 0 99\n", 3), [false, true, false]);
    }
}
