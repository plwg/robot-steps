//! Which levels have been finished, for each of the three players, saved
//! between games as a list of level numbers in a small text file per player.

/// How many players (save slots) there are.
pub const SLOTS: usize = 3;

/// `progress-1.txt` to `progress-3.txt`, for slots 0 to 2.
fn name(slot: usize) -> String {
    format!("progress-{}.txt", slot + 1)
}

/// Finished levels for one player, one flag per level. Missing or unreadable
/// progress just means nothing is finished yet. Before there were players,
/// progress was kept in `progress.txt`; that becomes the first player's.
pub fn load(slot: usize, levels: usize) -> Vec<bool> {
    let text = store::read(&name(slot))
        .or_else(|| (slot == 0).then(|| store::read("progress.txt")).flatten())
        .unwrap_or_default();
    parse(&text, levels)
}

/// Save one player's finished levels. Failing to save shouldn't stop the
/// game, so errors are ignored.
pub fn save(slot: usize, solved: &[bool]) {
    store::write(&name(slot), &format(solved));
}

/// Saves are small text files in a folder: `%APPDATA%\RobotSteps` on
/// Windows, otherwise `~/.local/share/robot_steps`.
#[cfg(not(target_arch = "wasm32"))]
mod store {
    use std::path::PathBuf;

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

    pub fn read(name: &str) -> Option<String> {
        std::fs::read_to_string(dir()?.join(name)).ok()
    }

    pub fn write(name: &str, text: &str) {
        if let Some(dir) = dir() {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join(name), text);
        }
    }
}

/// In a web browser there are no files, so saves go in the browser's local
/// storage instead, through the little plugin in `web/storage.js`.
#[cfg(target_arch = "wasm32")]
mod store {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        /// Copy the saved text into `buf` (if it fits) and return its length,
        /// or -1 if there's nothing saved.
        fn robot_storage_read(key: *const u8, key_len: usize, buf: *mut u8, cap: usize) -> i32;
        fn robot_storage_write(key: *const u8, key_len: usize, text: *const u8, len: usize);
    }

    /// Lets the JavaScript side check it matches.
    #[unsafe(no_mangle)]
    pub extern "C" fn robot_storage_crate_version() -> u32 {
        1
    }

    pub fn read(name: &str) -> Option<String> {
        let mut buf = vec![0u8; 1024];
        loop {
            // SAFETY: both pointers are valid for the lengths given.
            let len = unsafe {
                robot_storage_read(name.as_ptr(), name.len(), buf.as_mut_ptr(), buf.len())
            };
            let len = usize::try_from(len).ok()?;
            if len <= buf.len() {
                buf.truncate(len);
                return String::from_utf8(buf).ok();
            }
            buf = vec![0u8; len];
        }
    }

    pub fn write(name: &str, text: &str) {
        // SAFETY: both pointers are valid for the lengths given.
        unsafe { robot_storage_write(name.as_ptr(), name.len(), text.as_ptr(), text.len()) };
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
