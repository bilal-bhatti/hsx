//! `hsx cycle` — the in-line cycling path for zsh and bash.
//!
//! Called once per ↑/↓ keypress. It is stateless: the shim passes the current
//! search root (`--query`), the direction, and the current match `--index`, and
//! we return the next match plus a new index for the shim to remember. This is
//! what lets repeated ↑ keep filtering on the *original* typed root.
//!
//! ## Output protocol (stdout)
//! Three header lines followed by the buffer (which may itself contain newlines):
//! ```text
//! <new_index>\n      signed integer; -1 means "showing the original query"
//! <highlights>\n     space-separated "start:end" char-offset spans (may be empty)
//! <found 0|1>\n      0 => nothing matched (paint "not found")
//! <buffer...>        the line to place in the editor
//! ```

use crate::args::take;
use crate::history;
use crate::search::{self, Case};
use std::process::ExitCode;

#[derive(Clone, Copy)]
enum Direction {
    Up,
    Down,
}

pub fn run(args: &[String]) -> ExitCode {
    let mut query = String::new();
    let mut direction = Direction::Up;
    let mut index: i64 = -1;
    let mut case = Case::Smart;
    let mut hist: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--query" => query = take(args, &mut i),
            "--direction" => {
                direction = match take(args, &mut i).as_str() {
                    "down" => Direction::Down,
                    _ => Direction::Up,
                }
            }
            "--index" => index = take(args, &mut i).parse().unwrap_or(-1),
            "--case" => case = Case::parse(&take(args, &mut i)),
            "--hist" => hist = Some(take(args, &mut i)),
            _ => {}
        }
        i += 1;
    }

    let raw = history::read(hist.as_deref());
    let entries = history::parse(&raw);
    let matches = search::matches(&entries, &query, case);

    // index == -1 means the original typed query is on the line.
    let (buffer, spans, found, new_index): (String, Vec<(usize, usize)>, bool, i64) =
        if matches.is_empty() {
            // Nothing matched: keep the user's text, signal "not found".
            (query.clone(), Vec::new(), false, -1)
        } else {
            match direction {
                Direction::Up => {
                    // Older in time = higher index. Clamp at the oldest match.
                    let target = (index + 1).clamp(0, matches.len() as i64 - 1);
                    let entry = matches[target as usize];
                    (entry.clone(), spans_of(entry, &query, case), true, target)
                }
                Direction::Down => {
                    // Newer in time = lower index; below 0 returns to the query.
                    let target = index - 1;
                    if target <= -1 {
                        (query.clone(), Vec::new(), true, -1)
                    } else {
                        let entry = matches[target as usize];
                        (entry.clone(), spans_of(entry, &query, case), true, target)
                    }
                }
            }
        };

    let hl = spans
        .iter()
        .map(|(s, e)| format!("{s}:{e}"))
        .collect::<Vec<_>>()
        .join(" ");
    print!("{new_index}\n{hl}\n{}\n{buffer}", if found { 1 } else { 0 });
    ExitCode::SUCCESS
}

/// Highlight spans for `query` within `entry` (empty if somehow unmatched).
fn spans_of(entry: &str, query: &str, case: Case) -> Vec<(usize, usize)> {
    search::find_spans(entry, query, case).unwrap_or_default()
}
