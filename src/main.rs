//! hsx — portable cross-shell history substring search.
//!
//! Subcommands:
//!   init <shell>   print the shell shim to eval (zsh|bash|dash|ksh)
//!   cycle ...      one ↑/↓ step of in-line search (used by zsh/bash shims)
//!   pick ...       full-screen history picker (used by dash/ksh, or anywhere)

mod args;
mod cycle;
mod history;
mod init;
mod pick;
mod search;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("init") => init::run(&args[1..]),
        Some("cycle") => cycle::run(&args[1..]),
        Some("pick") => pick::run(&args[1..]),
        Some("-V") | Some("--version") => {
            println!("hsx {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("-h") | Some("--help") | Some("help") | None => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("hsx: unknown subcommand '{other}'\n");
            print_help();
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "hsx {} — portable cross-shell history substring search\n\
\n\
USAGE:\n\
    hsx <command> [options]\n\
\n\
COMMANDS:\n\
    init <shell>    Print the shell integration to eval. Add to your rc file:\n\
                      zsh:   eval \"$(hsx init zsh)\"\n\
                      bash:  eval \"$(hsx init bash)\"\n\
                      ksh:   eval \"$(hsx init ksh)\"\n\
                      dash:  eval \"$(hsx init dash)\"\n\
    cycle [...]     One up/down search step (called by the zsh/bash shims).\n\
    pick  [...]     Full-screen history picker (dash/ksh, or as a chooser).\n\
\n\
ENV (read by the shell integration, not the subcommands directly):\n\
    HSX_CASE        smart | sensitive | insensitive   (default: smart)\n\
    HSX_HISTFILE    Explicit history file for shells with none (e.g. dash).\n\
    HSX_HIGHLIGHT_FOUND / HSX_HIGHLIGHT_NOT_FOUND  zsh-only highlight specs.",
        env!("CARGO_PKG_VERSION")
    );
}
