_rsille_overlay_send() {
  builtin print -rnu @{EVENT_FD}@ -- "$1" 2>/dev/null || :
}

_rsille_overlay_edit() {
  emulate -L zsh
  [[ $CONTEXT == start ]] || return 0
  local line=${BUFFER[1,4097]}
  if [[ $line != "$_rsille_overlay_line" ]]; then
    _rsille_overlay_line=$line
    _rsille_overlay_send "e$line"
  fi
}

# $1 is the line as typed; $3, the parsed text, only when history is off.
_rsille_overlay_exec() {
  emulate -L zsh
  _rsille_overlay_running=1
  _rsille_overlay_line=''
  local line=${1:-$3}
  _rsille_overlay_send "x${line[1,4097]}"
}

_rsille_overlay_prompt() {
  local result=$?
  emulate -L zsh
  if [[ -n $_rsille_overlay_running ]]; then
    _rsille_overlay_send "d$result"
  fi
  _rsille_overlay_running=''
  _rsille_overlay_line=''
  _rsille_overlay_send p
}

_rsille_overlay_init() {
  emulate -L zsh
  precmd_functions+=( _rsille_overlay_prompt )
  preexec_functions+=( _rsille_overlay_exec )
  zmodload zsh/zle || return
  autoload -Uz add-zle-hook-widget
  add-zle-hook-widget line-pre-redraw _rsille_overlay_edit
}

if (( ${+_RSILLE_ORIG_ZDOTDIR} )); then
  ZDOTDIR=$_RSILLE_ORIG_ZDOTDIR
  unset _RSILLE_ORIG_ZDOTDIR
else
  unset ZDOTDIR
fi
[[ -r ${ZDOTDIR-$HOME}/.zshrc ]] && source "${ZDOTDIR-$HOME}/.zshrc"
_rsille_overlay_init
