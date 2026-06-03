# hsx — portable cross-shell history substring search

A cross-shell take on [`zsh-history-substring-search`](https://github.com/zsh-users/zsh-history-substring-search):
type part of a command, press **↑ / ↓**, and walk through the history entries that
*contain* what you typed — with the match highlighted (in zsh). One small Rust
binary holds the matching logic; thin per-shell shims wire it into each shell's
line editor, the same way `zoxide`, `starship`, and `fzf` do.

The query is matched by **word**: it's split on whitespace and every token must
appear in the entry, in order but not necessarily adjacent. So `pu up` matches
`pulumi --stack dev up`, and each matched word is highlighted separately. A
single word behaves like a plain substring search.

History is presented **newest-first and de-duplicated** — when the same command
appears more than once, only its most recent run is shown.

## What you get, per shell

Shells differ a lot in whether a key binding can read and rewrite the line being
edited, so the experience differs too:

| Shell    | Up/Down in-line cycling | Match highlight | Notes |
|----------|:----------------------:|:---------------:|-------|
| **zsh**  | ✅ | ✅ | Full fidelity (ZLE widget + `region_highlight`). |
| **bash** | ✅ | ❌ | Needs **bash 4.0+** (`$READLINE_LINE`); macOS's stock bash 3.2 is too old. readline has no API to colorize the match. |
| **ksh**  | ❌ | ❌ | Uses the full-screen picker (`hpick`) instead — ksh won't expose the edit buffer to a function. |
| **dash** | ❌ | ❌ | Picker only, and dash keeps no history; point `HSX_HISTFILE` at one. |

## Install

Build the binary and put it on your `PATH`:

```sh
cargo build --release
cp target/release/hsx /usr/local/bin/   # or anywhere on $PATH
```

Then add the integration for your shell:

```sh
# ~/.zshrc
eval "$(hsx init zsh)"

# ~/.bashrc
eval "$(hsx init bash)"

# ksh / dash rc file
eval "$(hsx init ksh)"     # or: eval "$(hsx init dash)"
```

### zsh / bash

Just type a fragment and press **↑ / ↓**:

- ↑ walks backward (older) through entries matching your words; ↓ walks forward.
- Pressing ↑ again keeps filtering on what you *originally* typed, not on the
  recalled line.
- On an empty line, ↑ / ↓ behave like ordinary history navigation.
- When nothing matches, zsh paints the line in the "not found" color.

The match highlight works alongside **zsh-syntax-highlighting** and
**fast-syntax-highlighting** automatically — hsx re-applies its spans from a
`zle-line-pre-redraw` hook that runs after the highlighter, so no special plugin
load order is required.

### ksh / dash

These shells can't rewrite the line from a function, so you get a full-screen
picker. Run **`hpick`**, type to filter, and:

- **↑ / ↓** (or **Ctrl-P / Ctrl-N**) move through matches
- **Backspace** edits the filter
- **Enter** runs the selection
- **Esc** (or **Ctrl-C / Ctrl-G**) cancels

dash keeps no history of its own — give the picker something to search:

```sh
export HSX_HISTFILE="$HOME/.bash_history"   # or any newline-delimited file
```

## Configuration

These are read by the shell integration (the shim), not the `hsx` binary itself,
which is flag-driven. Set them in your rc file **before** the `eval "$(hsx init …)"`.

| Variable | Default | Meaning |
|----------|---------|---------|
| `HSX_CASE` | `smart` | `smart` (case-insensitive unless the query has an uppercase char), `sensitive`, or `insensitive`. |
| `HSX_HISTFILE` | — | Explicit history file for shells without one (dash). |
| `HSX_HIGHLIGHT_FOUND` | `fg=magenta,bold` | zsh highlight spec for the matched substring. |
| `HSX_HIGHLIGHT_NOT_FOUND` | `fg=red,bold` | zsh highlight spec when nothing matches. |

## How it works

- **`hsx init <shell>`** prints the shell shim (embedded in the binary).
- **`hsx cycle`** is invoked once per ↑/↓ keypress (zsh/bash). It's stateless:
  the shim passes the search root, direction, and current match index; the binary
  returns the next match plus a new index, which the shim remembers in shell
  variables. History is fed in via a per-prompt dump file, so the binary never
  has to parse each shell's on-disk history format.
- **`hsx pick`** renders the full-screen picker (ksh/dash). Its UI draws to
  stderr and prints the chosen command to stdout, so it composes cleanly; the
  terminal is restored via an RAII guard and a panic hook.

## Development

```sh
cargo test           # unit tests for history parsing and matching
cargo build          # debug build
cargo build --release
```

## Limitations

- bash: requires bash 4.0+ (needs `$READLINE_LINE`/`$READLINE_POINT`); on older
  bash the shim prints a notice and leaves ↑/↓ alone. No in-line match highlight
  either way (readline limitation).
- dash: no bindable keys and no native history; picker-only, needs `HSX_HISTFILE`.
- ksh: picker-only; in-line cycling isn't feasible.
- zsh: the ↑ binding replaces `up-line-or-history`, so cursor-up *within* a
  multi-line buffer recalls history instead. Multi-line history entries cycle as
  a whole; the picker shows only their first line.
