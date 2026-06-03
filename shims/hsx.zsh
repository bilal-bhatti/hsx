# hsx — cross-shell history substring search (zsh integration)
#
# Install (in ~/.zshrc):   eval "$(hsx init zsh)"
#
# Type part of a command, then press Up/Down to cycle through history entries
# containing it; the match is highlighted. With an empty line, Up/Down behave
# like ordinary history navigation.

typeset -g _HSX_HIST="${TMPDIR:-/tmp}/hsx.$$.hist"
typeset -g _HSX_QUERY=""
typeset -g _HSX_INDEX=-1
typeset -g _HSX_LAST=""
typeset -g _HSX_ACTIVE=0     # 1 while a search result is on the line
typeset -ga _HSX_REGION      # region_highlight entries to (re)apply
typeset -ga _HSX_APPLIED     # entries we added on the last redraw (to remove)
: ${HSX_CASE:=smart}
: ${HSX_HIGHLIGHT_FOUND:=fg=magenta,bold}
: ${HSX_HIGHLIGHT_NOT_FOUND:=fg=red,bold}

# Refresh the per-session history dump before each prompt (oldest-first).
_hsx_dump() { fc -ln 1 2>/dev/null > "$_HSX_HIST" }
_hsx_cleanup() { rm -f "$_HSX_HIST" 2>/dev/null }

autoload -Uz add-zsh-hook
add-zsh-hook precmd _hsx_dump
add-zsh-hook zshexit _hsx_cleanup

_hsx_move() {
  local dir="$1"

  # If the buffer changed since our last result, begin a fresh search rooted on
  # the new text. This is what makes repeated Up keep filtering on what you typed.
  if [[ "$BUFFER" != "$_HSX_LAST" ]]; then
    _HSX_QUERY="$BUFFER"
    _HSX_INDEX=-1
  fi

  local out new_index hl found buffer
  out="$(hsx cycle --query "$_HSX_QUERY" --direction "$dir" \
                   --index "$_HSX_INDEX" --case "$HSX_CASE" --hist "$_HSX_HIST")"

  # Three header lines, then the buffer (which may itself contain newlines).
  new_index="${out%%$'\n'*}"; out="${out#*$'\n'}"
  hl="${out%%$'\n'*}";        out="${out#*$'\n'}"   # space-separated start:end spans
  found="${out%%$'\n'*}";     out="${out#*$'\n'}"
  buffer="$out"

  _HSX_INDEX="$new_index"
  BUFFER="$buffer"
  CURSOR=${#BUFFER}
  _HSX_LAST="$BUFFER"

  # Stash the highlight spans; the pre-redraw hook applies them *after* any
  # syntax highlighter rebuilds region_highlight, so our colors survive.
  _HSX_REGION=()
  if [[ "$found" == 1 && -n "$hl" ]]; then
    local pair
    for pair in ${=hl}; do   # ${=hl} word-splits on whitespace
      _HSX_REGION+=("${pair%%:*} ${pair##*:} $HSX_HIGHLIGHT_FOUND")
    done
  elif [[ "$found" == 0 ]]; then
    _HSX_REGION=("0 ${#BUFFER} $HSX_HIGHLIGHT_NOT_FOUND")
  fi
  _HSX_ACTIVE=1
}

# Remove exactly the entries hsx added on the previous redraw. A syntax
# highlighter (e.g. zsh-syntax-highlighting) may *preserve* our foreign entries
# when it rebuilds region_highlight, so we can't assume they were cleared — we
# strip our own to stop them piling up across keypresses.
_hsx_strip_applied() {
  (( ${#_HSX_APPLIED} )) || return
  local -a kept
  local e a drop
  for e in "${region_highlight[@]}"; do
    drop=0
    for a in "${_HSX_APPLIED[@]}"; do
      [[ "$e" == "$a" ]] && { drop=1; break }
    done
    (( drop )) || kept+=("$e")
  done
  region_highlight=("${kept[@]}")
  _HSX_APPLIED=()
}

# Re-apply our spans on every redraw while a search is active. Runs after the
# syntax highlighter (we keep it last, see _hsx_line_init).
_hsx_apply_highlight() {
  _hsx_strip_applied
  (( _HSX_ACTIVE )) || return
  # The moment the user edits the recalled line, stop highlighting.
  if [[ "$BUFFER" != "$_HSX_LAST" ]]; then
    _HSX_ACTIVE=0
    _HSX_REGION=()
    return
  fi
  region_highlight+=("${_HSX_REGION[@]}")
  _HSX_APPLIED=("${_HSX_REGION[@]}")
}

# At the start of each prompt: clear stale state, and re-register our highlight
# hook so it runs LAST — after any syntax highlighter — regardless of the order
# plugins were loaded in.
_hsx_line_init() {
  _HSX_ACTIVE=0
  _HSX_REGION=()
  _HSX_APPLIED=()
  add-zle-hook-widget -d zle-line-pre-redraw _hsx_apply_highlight 2>/dev/null
  add-zle-hook-widget zle-line-pre-redraw _hsx_apply_highlight
}

autoload -Uz add-zle-hook-widget
add-zle-hook-widget zle-line-init _hsx_line_init
add-zle-hook-widget zle-line-pre-redraw _hsx_apply_highlight

hsx-up()   { _hsx_move up }
hsx-down() { _hsx_move down }
zle -N hsx-up
zle -N hsx-down

# Cursor keys in both normal (^[[A) and application (^[OA) modes.
bindkey '^[[A' hsx-up
bindkey '^[OA' hsx-up
bindkey '^[[B' hsx-down
bindkey '^[OB' hsx-down
