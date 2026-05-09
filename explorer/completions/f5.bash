# bash completion for the f5 BIG-IP CLI.
#
# Install (system-wide):
#   sudo cp f5.bash /etc/bash_completion.d/f5
#
# Install (user):
#   mkdir -p ~/.local/share/bash-completion/completions
#   f5 completion bash > ~/.local/share/bash-completion/completions/f5
#
# Or source it directly from your ~/.bashrc:
#   source <(f5 completion bash)

_f5_complete() {
    local cur prev words cword
    if declare -F _init_completion >/dev/null 2>&1; then
        _init_completion -n = || return
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
        prev="${COMP_WORDS[COMP_CWORD-1]}"
        words=("${COMP_WORDS[@]}")
        cword=$COMP_CWORD
    fi

    local verbs="cleanup clean completion grep related irule"
    local global_opts="-h --help --help-all --version"
    local cleanup_opts="--json --keep --no-keep-common -o --output -h --help"
    local grep_opts="-e --regex --direction --max-depth --max-nodes --full --json -o --output -h --help"
    local grep_directions="forward reverse both"
    local completion_shells="bash fish zsh"
    local irule_actions="event-order eventorder event-info eventinfo"
    local irule_event_order_opts="--source --package-path --no-recursive \
        --dialect --output -o --json -h --help"
    local irule_event_info_opts="--json --output -o -h --help"

    # First positional after the command name: pick a verb.
    if [[ $cword -eq 1 ]]; then
        if [[ "$cur" == -* ]]; then
            COMPREPLY=( $(compgen -W "$global_opts" -- "$cur") )
        else
            COMPREPLY=( $(compgen -W "$verbs" -- "$cur") )
        fi
        return
    fi

    case "${words[1]}" in
        cleanup|clean)
            case "$prev" in
                --keep)
                    # Free-form path / partition prefix — fall through to file
                    # completion so users can tab-complete a partition name.
                    COMPREPLY=( $(compgen -A file -- "$cur") )
                    return
                    ;;
                -o|--output)
                    COMPREPLY=( $(compgen -A file -- "$cur") )
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$cleanup_opts" -- "$cur") )
            else
                # Positional: any .conf or .scf file plus directories.
                local files=( $(compgen -A file -- "$cur") )
                local dirs=( $(compgen -A directory -- "$cur") )
                COMPREPLY=()
                for f in "${files[@]}"; do
                    case "$f" in
                        *.conf|*.scf|-) COMPREPLY+=( "$f" ) ;;
                    esac
                done
                COMPREPLY+=( "${dirs[@]}" )
            fi
            ;;
        grep|related)
            case "$prev" in
                --direction)
                    COMPREPLY=( $(compgen -W "$grep_directions" -- "$cur") )
                    return
                    ;;
                --max-depth|--max-nodes)
                    # Numeric — no completion candidates.
                    COMPREPLY=()
                    return
                    ;;
                -o|--output)
                    COMPREPLY=( $(compgen -A file -- "$cur") )
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$grep_opts" -- "$cur") )
            else
                # First non-flag positional is the pattern (free-form).  Subsequent
                # positionals are config files — restrict to .conf / .scf plus dirs.
                local non_flag_count=0
                local i
                for ((i=2; i<cword; i++)); do
                    case "${words[i]}" in
                        -*) ;;  # flag
                        *)
                            case "${words[i-1]}" in
                                --direction|--max-depth|--max-nodes|-o|--output)
                                    ;;  # value of preceding flag
                                *)
                                    non_flag_count=$((non_flag_count+1))
                                    ;;
                            esac
                            ;;
                    esac
                done
                if [[ $non_flag_count -lt 1 ]]; then
                    COMPREPLY=()
                else
                    local files=( $(compgen -A file -- "$cur") )
                    local dirs=( $(compgen -A directory -- "$cur") )
                    COMPREPLY=()
                    for f in "${files[@]}"; do
                        case "$f" in
                            *.conf|*.scf|-) COMPREPLY+=( "$f" ) ;;
                        esac
                    done
                    COMPREPLY+=( "${dirs[@]}" )
                fi
            fi
            ;;
        completion)
            COMPREPLY=( $(compgen -W "$completion_shells" -- "$cur") )
            ;;
        irule)
            # f5 irule <action> [opts...]
            if [[ $cword -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "$irule_actions" -- "$cur") )
                return
            fi
            case "${words[2]}" in
                event-order|eventorder)
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "$irule_event_order_opts" -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -A file -- "$cur") )
                        COMPREPLY+=( $(compgen -A directory -- "$cur") )
                    fi
                    ;;
                event-info|eventinfo)
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "$irule_event_info_opts" -- "$cur") )
                    fi
                    ;;
            esac
            ;;
    esac
}

complete -F _f5_complete f5
complete -F _f5_complete f5.pyz
