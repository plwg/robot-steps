//! Which levels have been finished, saved between games as a list of level
//! numbers in a small text file.

use std::path::PathBuf;

/// `%APPDATA%\RobotSteps\progress.txt` on Windows, otherwise
/// `~/.local/share/robot_steps/progress.txt`.
fn path() -> Option<PathBuf> {
    let dir = match std::env::var_os("APPDATA") {
        Some(appdata) => PathBuf::from(appdata).join("RobotSteps"),
        None => {
            let data = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
                })?;
            data.join("robot_steps")
        }
    };
    Some(dir.join("progress.txt"))
}

/// Finished levels, one flag per level. Missing or unreadable progress just
/// means nothing is finished yet.
pub fn load(levels: usize) -> Vec<bool> {
    let text = path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    parse(&text, levels)
}

/// Save the finished levels. Failing to save shouldn't stop the game, so
/// errors are ignored.
pub fn save(solved: &[bool]) {
    if let Some(p) = path() {
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
