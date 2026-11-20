# This temporary .zshenv is used only by the interactive pet shell.
_rsille_startup_dir=$ZDOTDIR
if (( ${+_RSILLE_ORIG_ZDOTDIR} )); then
  export ZDOTDIR=$_RSILLE_ORIG_ZDOTDIR
  unset _RSILLE_ORIG_ZDOTDIR
else
  unset ZDOTDIR
fi

[[ -r ${ZDOTDIR-$HOME}/.zshenv ]] && source "${ZDOTDIR-$HOME}/.zshenv"
if [[ -o rcs ]]; then
  if (( ${+ZDOTDIR} )); then
    _RSILLE_ORIG_ZDOTDIR=$ZDOTDIR
  fi
  ZDOTDIR=$_rsille_startup_dir
fi
unset _rsille_startup_dir
