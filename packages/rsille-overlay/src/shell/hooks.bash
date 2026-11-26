# Loaded by bash --rcfile in place of ~/.bashrc. Builds of bash that read a
# system bashrc still read it before this file.
[[ -r ~/.bashrc ]] && source ~/.bashrc

__rsille_overlay_send() {
  builtin printf '%s' "$1" >&@{EVENT_FD}@ 2>/dev/null
}

# Expanded through PS0 in a subshell, after a non-empty line is read and
# added to history, just before it runs.
__rsille_overlay_exec() {
  local line
  line=$(HISTTIMEFORMAT= builtin fc -ln -0 2>/dev/null)
  line=${line#"${line%%[![:space:]]*}"}
  __rsille_overlay_send "x${line:0:4097}"
}

__rsille_overlay_prompt() {
  local result=$?
  if [[ -n ${__rsille_overlay_ran-} ]]; then
    __rsille_overlay_ran=
    __rsille_overlay_send "d$result"
  fi
  __rsille_overlay_send p
  return $result
}

# The subscript is evaluated in the interactive shell itself and marks a
# command as started; the element is always unset, so it expands to nothing.
PS0="${PS0-}"'${__rsille_overlay_null[__rsille_overlay_ran=1]-}$(__rsille_overlay_exec)'
PROMPT_COMMAND="__rsille_overlay_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
