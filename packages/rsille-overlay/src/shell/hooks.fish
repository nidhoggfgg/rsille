# Loaded by fish -C after the user's startup files.
function __rsille_overlay_send
    builtin printf '%s' "$argv[1]" >&@{EVENT_FD}@ 2>/dev/null
end

function __rsille_overlay_exec --on-event fish_preexec
    set -g __rsille_overlay_running 1
    __rsille_overlay_send x(string sub -l 4097 -- "$argv[1]" | string collect)
end

function __rsille_overlay_done --on-event fish_postexec
    set -l result $status
    if set -q __rsille_overlay_running
        set -e __rsille_overlay_running
        __rsille_overlay_send d$result
    end
end

function __rsille_overlay_prompt --on-event fish_prompt --on-event fish_cancel
    __rsille_overlay_send p
end

function __rsille_overlay_read --on-event fish_read
    __rsille_overlay_send r
end
