//! `hsx init <shell>` — print the shell shim to be `eval`'d, e.g.
//! `eval "$(hsx init zsh)"`. Shims are embedded at compile time.

use std::process::ExitCode;

const ZSH: &str = include_str!("../shims/hsx.zsh");
const BASH: &str = include_str!("../shims/hsx.bash");
const POSIX: &str = include_str!("../shims/hsx.posix");

pub fn run(args: &[String]) -> ExitCode {
    match args.first().map(|s| s.as_str()) {
        Some("zsh") => {
            print!("{ZSH}");
            ExitCode::SUCCESS
        }
        Some("bash") => {
            print!("{BASH}");
            ExitCode::SUCCESS
        }
        Some("dash") | Some("ksh") | Some("mksh") | Some("posix") | Some("sh") => {
            print!("{POSIX}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("hsx init: unsupported shell '{other}' (try: zsh, bash, dash, ksh)");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("hsx init: missing shell argument (try: zsh, bash, dash, ksh)");
            ExitCode::FAILURE
        }
    }
}
