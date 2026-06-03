//! `hsx pick` — a full-screen history picker for shells that can't do in-line
//! cycling (dash, ksh) and as an optional fzf-style chooser anywhere.
//!
//! The UI is drawn on **stderr** so the chosen command can be printed cleanly to
//! **stdout** for capture (`cmd=$(hsx pick ...)`). The terminal is torn down via
//! an RAII guard on the normal path and a panic hook on the abnormal one.

use crate::history;
use crate::search::{self, Case};
use std::io::{self, Write};
use std::process::ExitCode;

use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

pub fn run(args: &[String]) -> ExitCode {
    let mut query = String::new();
    let mut case = Case::Smart;
    let mut hist: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--query" => {
                i += 1;
                query = args.get(i).cloned().unwrap_or_default();
            }
            "--case" => {
                i += 1;
                case = Case::parse(&args.get(i).cloned().unwrap_or_default());
            }
            "--hist" => {
                i += 1;
                hist = args.get(i).cloned();
            }
            _ => {}
        }
        i += 1;
    }

    let entries = history::parse(&history::read(hist.as_deref()));

    match interact(&entries, query, case) {
        Ok(Some(choice)) => {
            println!("{choice}");
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::from(130), // cancelled (SIGINT convention)
        Err(_) => ExitCode::FAILURE,
    }
}

/// RAII guard: enter raw mode + alt screen on construction, restore on drop.
struct TermGuard;

impl TermGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stderr(), EnterAlternateScreen)?;
        Ok(TermGuard)
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stderr(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

fn interact(entries: &[String], mut query: String, case: Case) -> io::Result<Option<String>> {
    // Restore the terminal even if something panics (Drop won't run with
    // panic=abort, but the hook fires before the process aborts).
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stderr(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let _guard = TermGuard::new()?;
    let mut selected: usize = 0;

    loop {
        let filtered = search::matches(entries, &query, case);
        if selected >= filtered.len() {
            selected = filtered.len().saturating_sub(1);
        }
        draw(&query, &filtered, selected)?;

        match event::read()? {
            Event::Key(k) if k.kind != KeyEventKind::Release => match (k.code, k.modifiers) {
                (KeyCode::Esc, _)
                | (KeyCode::Char('c'), KeyModifiers::CONTROL)
                | (KeyCode::Char('g'), KeyModifiers::CONTROL) => return Ok(None),
                (KeyCode::Enter, _) => return Ok(filtered.get(selected).map(|s| (*s).clone())),
                (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                    selected = selected.saturating_sub(1);
                }
                (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                    if selected + 1 < filtered.len() {
                        selected += 1;
                    }
                }
                (KeyCode::Backspace, _) => {
                    query.pop();
                    selected = 0;
                }
                (KeyCode::Char(c), m) if m.is_empty() || m == KeyModifiers::SHIFT => {
                    query.push(c);
                    selected = 0;
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn draw(query: &str, items: &[&String], selected: usize) -> io::Result<()> {
    const PROMPT: &str = "history> ";
    let mut out = io::stderr();
    let (cols, rows) = size().unwrap_or((80, 24));
    let (cols, rows) = (cols as usize, rows as usize);

    queue!(
        out,
        Clear(ClearType::All),
        MoveTo(0, 0),
        Print(truncate(&format!("{PROMPT}{query}"), cols))
    )?;

    let list_rows = rows.saturating_sub(1);
    for (row, item) in items.iter().take(list_rows).enumerate() {
        let line = truncate(item.lines().next().unwrap_or(""), cols);
        queue!(out, MoveTo(0, (row + 1) as u16))?;
        if row == selected {
            queue!(out, SetAttribute(Attribute::Reverse), Print(line), SetAttribute(Attribute::Reset))?;
        } else {
            queue!(out, Print(line))?;
        }
    }

    let cursor_x = (PROMPT.chars().count() + query.chars().count()).min(cols.saturating_sub(1));
    queue!(out, MoveTo(cursor_x as u16, 0))?;
    out.flush()
}

/// Truncate to `max` display columns (approximated by char count), adding an
/// ellipsis when shortened.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let keep = max.saturating_sub(1);
        let mut t: String = s.chars().take(keep).collect();
        t.push('…');
        t
    }
}
