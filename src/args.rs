//! Shared command-line parsing helpers for the subcommands.

/// Consume the value following a flag at `args[*i]`, advancing `*i` past it.
/// Returns an empty string if the flag was given with no following value.
pub fn take(args: &[String], i: &mut usize) -> String {
    *i += 1;
    args.get(*i).cloned().unwrap_or_default()
}
