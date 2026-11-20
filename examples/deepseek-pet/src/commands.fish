# Loaded by fish -C after the user's startup files.
function __rsille_pet_send
    builtin printf '%s' "$argv[1]" >&@EVENT_FD@
end

function __rsille_pet_run --on-event fish_preexec
    set -g __rsille_pet_kind 0
    if not string match -qr '[;&|()<>`\n]' -- "$argv[1]"
        set -l line (string replace -ar '[ \t]+' ' ' -- (string trim -- "$argv[1]"))
        switch "$line"
@COMMAND_CASES@
        end
    end
    __rsille_pet_send r$__rsille_pet_kind
end

function __rsille_pet_done --on-event fish_postexec
    set -l result $status
    if set -q __rsille_pet_kind; and test "$__rsille_pet_kind" != 0
        switch $result
            case 0
                __rsille_pet_send +$__rsille_pet_kind
            case 130
                __rsille_pet_send i$__rsille_pet_kind
            case '*'
                __rsille_pet_send -$__rsille_pet_kind
        end
    end
    set -e __rsille_pet_kind
end

function __rsille_pet_prompt --on-event fish_prompt --on-event fish_cancel
    __rsille_pet_send p0
end

function __rsille_pet_read --on-event fish_read
    __rsille_pet_send r0
end
