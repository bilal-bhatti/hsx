//! Reading and normalizing shell history.
//!
//! Shims feed history as one entry per line, oldest-first (the natural order
//! `fc -ln`/`history` emit). We turn that into a newest-first, de-duplicated
//! list so index 0 is always the most recent command.

use std::io::Read;

/// Read raw history text from a file path, or from stdin when `path` is `None`.
pub fn read(path: Option<&str>) -> String {
    match path {
        Some(p) => std::fs::read_to_string(p).unwrap_or_default(),
        None => {
            let mut s = String::new();
            let _ = std::io::stdin().read_to_string(&mut s);
            s
        }
    }
}

/// Parse raw history into a newest-first, de-duplicated list of entries.
///
/// Blank lines are dropped. When the same command appears multiple times, only
/// its most recent occurrence is kept (matching how shells present history).
pub fn parse(raw: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    // Walk newest-first; the first time we see an entry is its most recent use.
    for line in raw.lines().rev() {
        let entry = line.trim_end_matches(['\r', '\n']);
        if entry.is_empty() {
            continue;
        }
        if seen.insert(entry) {
            out.push(entry.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_first_ordering() {
        let raw = "git status\ngit commit\nls -la\n";
        assert_eq!(parse(raw), vec!["ls -la", "git commit", "git status"]);
    }

    #[test]
    fn dedup_keeps_most_recent() {
        let raw = "ls\ngit status\nls\ncargo build\n";
        // Newest-first, each command once: cargo build, ls (most recent ls), git status.
        assert_eq!(parse(raw), vec!["cargo build", "ls", "git status"]);
    }

    #[test]
    fn drops_blank_lines() {
        let raw = "a\n\n\nb\n";
        assert_eq!(parse(raw), vec!["b", "a"]);
    }

    #[test]
    fn strips_trailing_carriage_returns() {
        let raw = "git push\r\nls\r\n";
        assert_eq!(parse(raw), vec!["ls", "git push"]);
    }
}
