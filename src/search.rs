//! Matching history entries against a query.
//!
//! The query is split on whitespace into tokens; an entry matches when every
//! token appears in it, in order and non-overlapping (so `pu up` matches
//! `pulumi --stack dev up`). Each matched token yields its own highlight span.
//!
//! Spans are reported in **character** units (not bytes) because zsh's
//! `region_highlight` and `CURSOR` are character-indexed.

/// Case-matching mode for searches.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Case {
    /// Case-insensitive unless the query contains an uppercase character.
    Smart,
    /// Always case-sensitive.
    Sensitive,
    /// Always case-insensitive.
    Insensitive,
}

impl Case {
    pub fn parse(s: &str) -> Case {
        match s {
            "sensitive" => Case::Sensitive,
            "insensitive" => Case::Insensitive,
            _ => Case::Smart,
        }
    }
}

/// Whether this query should match case-insensitively under `case`.
fn insensitive(case: Case, query: &str) -> bool {
    match case {
        Case::Sensitive => false,
        Case::Insensitive => true,
        Case::Smart => !query.chars().any(|c| c.is_uppercase()),
    }
}

/// A compiled query: one character "needle" per whitespace-separated token,
/// plus the case mode resolved once against the whole query. Building this once
/// keeps the per-entry work to just scanning the entry — the query is split and
/// char-converted a single time rather than once per history entry.
struct Pattern {
    needles: Vec<Vec<char>>,
    ci: bool,
}

impl Pattern {
    fn new(query: &str, case: Case) -> Pattern {
        Pattern {
            ci: insensitive(case, query),
            needles: query.split_whitespace().map(|t| t.chars().collect()).collect(),
        }
    }

    /// Whether `entry` matches: every needle found in order and non-overlapping.
    /// An empty pattern (query with no tokens) matches everything. Allocates the
    /// entry's char vector but no spans, since the filter pass only needs a bool.
    fn is_match(&self, entry: &str) -> bool {
        if self.needles.is_empty() {
            return true;
        }
        let chars: Vec<char> = entry.chars().collect();
        let mut pos = 0;
        for needle in &self.needles {
            match find_from(&chars, needle, pos, self.ci) {
                // Advance past this match so the next token comes after it, in order.
                Some((_, end)) => pos = end,
                None => return false,
            }
        }
        true
    }

    /// Highlight spans for `entry`, one `(start, end)` per token, or `None` if
    /// any token is missing. An empty pattern yields `Some(empty)`.
    fn spans(&self, entry: &str) -> Option<Vec<(usize, usize)>> {
        if self.needles.is_empty() {
            return Some(Vec::new());
        }
        let chars: Vec<char> = entry.chars().collect();
        let mut pos = 0;
        let mut spans = Vec::with_capacity(self.needles.len());
        for needle in &self.needles {
            // Advance past this match so the next token comes after it, in order.
            let (start, end) = find_from(&chars, needle, pos, self.ci)?;
            spans.push((start, end));
            pos = end;
        }
        Some(spans)
    }
}

/// Match `query` against `entry`, returning one `(start, end)` character span
/// per query token, or `None` if any token is missing. A query with no tokens
/// (empty/whitespace) matches everything with no spans.
pub fn find_spans(entry: &str, query: &str, case: Case) -> Option<Vec<(usize, usize)>> {
    Pattern::new(query, case).spans(entry)
}

/// Find `needle` in `haystack` starting at character index `start`.
fn find_from(haystack: &[char], needle: &[char], start: usize, ci: bool) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return Some((start, start));
    }
    if needle.len() > haystack.len() {
        return None;
    }
    let last = haystack.len() - needle.len();
    let mut i = start;
    while i <= last {
        if (0..needle.len()).all(|k| char_eq(haystack[i + k], needle[k], ci)) {
            return Some((i, i + needle.len()));
        }
        i += 1;
    }
    None
}

fn char_eq(a: char, b: char, ci: bool) -> bool {
    if ci {
        // The Unicode lowercase compare also covers ASCII; `a == b` is just a
        // fast path for the common identical-char case.
        a == b || a.to_lowercase().eq(b.to_lowercase())
    } else {
        a == b
    }
}

/// All entries matching `query`, preserving input order (newest-first).
pub fn matches<'a>(entries: &'a [String], query: &str, case: Case) -> Vec<&'a String> {
    let pat = Pattern::new(query, case);
    entries.iter().filter(|e| pat.is_match(e)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn single_token_substring() {
        assert_eq!(find_spans("git commit -m", "comm", Case::Sensitive), Some(vec![(4, 8)]));
        assert_eq!(find_spans("ls -la", "zzz", Case::Sensitive), None);
    }

    #[test]
    fn multi_token_in_order_non_adjacent() {
        // The motivating case: "pu up" should match "pulumi --stack dev up".
        assert_eq!(
            find_spans("pulumi --stack dev up", "pu up", Case::Smart),
            Some(vec![(0, 2), (19, 21)])
        );
    }

    #[test]
    fn tokens_must_appear_in_order() {
        // "up" comes before "pu" here, so the second token can't be found after the first.
        assert_eq!(find_spans("up pulumi", "pu up", Case::Smart), None);
    }

    #[test]
    fn tokens_do_not_overlap() {
        // Two "ab" tokens need two separate occurrences.
        assert!(find_spans("abab", "ab ab", Case::Sensitive).is_some());
        assert!(find_spans("ab", "ab ab", Case::Sensitive).is_none());
    }

    #[test]
    fn empty_query_matches_all() {
        let entries = s(&["a", "b", "c"]);
        assert_eq!(matches(&entries, "", Case::Smart).len(), 3);
        assert_eq!(matches(&entries, "   ", Case::Smart).len(), 3);
    }

    #[test]
    fn smart_case() {
        assert!(find_spans("Git Commit", "git", Case::Smart).is_some());
        assert!(find_spans("git commit", "Git", Case::Smart).is_none());
    }

    #[test]
    fn explicit_sensitive_and_insensitive() {
        assert!(find_spans("GIT", "git", Case::Sensitive).is_none());
        assert!(find_spans("GIT", "git", Case::Insensitive).is_some());
    }

    #[test]
    fn spans_are_character_indexed() {
        // 'é' is one char but two bytes; offsets after it must be char-indexed.
        assert_eq!(find_spans("é cargo", "cargo", Case::Sensitive), Some(vec![(2, 7)]));
    }

    #[test]
    fn filters_match_list() {
        let entries = s(&["cargo build", "git status", "cargo test"]);
        let got: Vec<&String> = matches(&entries, "cargo", Case::Smart);
        assert_eq!(got, vec![&"cargo build".to_string(), &"cargo test".to_string()]);
    }
}
