# hsx — cross-shell history substring search (bash integration)
#
# Install (in ~/.bashrc):   eval "$(hsx init bash)"
#
# Requires bash 4.0+: the in-line search rewrites the line via $READLINE_LINE /
# $READLINE_POINT, which were added in bash 4.0. macOS ships bash 3.2 as
# /bin/bash — install a newer one ("brew install bash") and use that, or use zsh.
#
# Type part of a command, then press Up/Down to cycle through history entries
# containing it. (bash/readline has no API to colorize the match, so unlike zsh
# there is no in-line highlight.) With an empty line, Up/Down walk history.

# Wrapped in a function so the bash-version guard can `return` cleanly even when
# this is loaded via `eval` at the interactive top level. The functions defined
# inside become global once it runs.
_hsx_setup() {
  if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 4 ]; then
    printf 'hsx: bash %s is too old for in-line history search (needs bash 4.0+).\n' "${BASH_VERSION:-?}" >&2
    printf 'hsx: install a newer bash ("brew install bash") and use it, or use zsh.\n' >&2
    return
  fi

  _HSX_HIST="${TMPDIR:-/tmp}/hsx.$$.hist"
  _HSX_QUERY=""
  _HSX_INDEX=-1
  _HSX_LAST=""
  : "${HSX_CASE:=smart}"

  # Refresh the per-session history dump before each prompt: plain commands,
  # oldest-first, with the leading "  NNN  " line numbers stripped.
  _hsx_dump() {
    history | sed 's/^[[:space:]]*[0-9]\{1,\}[[:space:]]*//' > "$_HSX_HIST"
  }
  case ";${PROMPT_COMMAND};" in
    *";_hsx_dump;"*) ;;
    *) PROMPT_COMMAND="_hsx_dump${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;;
  esac

  # Clean up the dump on exit without clobbering an existing EXIT trap.
  if [ -z "$(trap -p EXIT)" ]; then
    trap 'rm -f "$_HSX_HIST"' EXIT
  fi

  _hsx_move() {
    local dir="$1"

    if [[ "$READLINE_LINE" != "$_HSX_LAST" ]]; then
      _HSX_QUERY="$READLINE_LINE"
      _HSX_INDEX=-1
    fi

    local out new_index buffer
    out="$(hsx cycle --query "$_HSX_QUERY" --direction "$dir" \
                     --index "$_HSX_INDEX" --case "$HSX_CASE" --hist "$_HSX_HIST")"

    # Three header lines, then the buffer. bash/readline can't highlight, so the
    # highlights and found lines are skipped.
    new_index="${out%%$'\n'*}"; out="${out#*$'\n'}"
    out="${out#*$'\n'}"   # skip highlights line
    out="${out#*$'\n'}"   # skip found line
    buffer="$out"

    _HSX_INDEX="$new_index"
    READLINE_LINE="$buffer"
    READLINE_POINT="${#READLINE_LINE}"
    _HSX_LAST="$READLINE_LINE"
  }

  _hsx_up()   { _hsx_move up; }
  _hsx_down() { _hsx_move down; }

  # Cursor keys in both normal (\e[A) and application (\eOA) modes.
  bind -x '"\e[A": _hsx_up'
  bind -x '"\eOA": _hsx_up'
  bind -x '"\e[B": _hsx_down'
  bind -x '"\eOB": _hsx_down'
}

_hsx_setup
unset -f _hsx_setup
