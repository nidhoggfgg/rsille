_rsille_pet_send() {
  builtin print -rnu @{EVENT_FD}@ -- "$1" 2>/dev/null || :
}

_rsille_pet_classify() {
  emulate -L zsh
  reply=''
  (( ${#1} <= 4096 )) || return 0
  local line=$1
  # Attribute the exit status only to a simple foreground command.
  [[ $line == *[\;\|\&\<\>\(\)\`]* || $line == *$'\n'* ]] && return 0
  local -a words=( ${(z)line} )
  case "${(j: :)words}" in
@{COMMAND_CASES}@
  esac
  if [[ -z $reply && $2 == edit ]]; then
    case "${words[1]}" in
@{FAMILY_CASES}@
    esac
  fi
}

_rsille_pet_edit() {
  emulate -L zsh
  [[ $CONTEXT == start ]] || return 0
  local reply
  _rsille_pet_classify "$BUFFER" edit
  if [[ $reply != $_rsille_pet_preview ]]; then
    _rsille_pet_preview=$reply
    _rsille_pet_send "e${reply:-0}"
  fi
}

_rsille_pet_run() {
  emulate -L zsh
  local reply
  _rsille_pet_classify "$3"
  _rsille_pet_active=$reply
  if [[ -n $reply ]]; then
    _rsille_pet_send "r$reply"
  elif [[ -n $_rsille_pet_preview ]]; then
    _rsille_pet_send e0
  fi
  _rsille_pet_preview=''
}

_rsille_pet_prompt() {
  local result=$?
  emulate -L zsh
  if [[ -n $_rsille_pet_active ]]; then
    local phase=+
    (( result == 0 )) || phase=-
    (( result == 130 )) && phase=i
    _rsille_pet_send "$phase$_rsille_pet_active"
  elif [[ -n $_rsille_pet_preview ]]; then
    _rsille_pet_send e0
  fi
  _rsille_pet_active=''
  _rsille_pet_preview=''
}

_rsille_pet_init() {
  emulate -L zsh
  precmd_functions+=( _rsille_pet_prompt )
  preexec_functions+=( _rsille_pet_run )
  zmodload zsh/zle || return
  autoload -Uz add-zle-hook-widget
  add-zle-hook-widget line-pre-redraw _rsille_pet_edit
}

if (( ${+_RSILLE_ORIG_ZDOTDIR} )); then
  ZDOTDIR=$_RSILLE_ORIG_ZDOTDIR
  unset _RSILLE_ORIG_ZDOTDIR
else
  unset ZDOTDIR
fi
[[ -r ${ZDOTDIR-$HOME}/.zshrc ]] && source "${ZDOTDIR-$HOME}/.zshrc"
_rsille_pet_init
