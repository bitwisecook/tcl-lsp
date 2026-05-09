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

    local verbs="cleanup clean completion"
    local global_opts="-h --help --help-all --version"
    local cleanup_opts="--json --keep --no-keep-common -o --output -h --help"
    local completion_shells="bash fish zsh"

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
        completion)
            COMPREPLY=( $(compgen -W "$completion_shells" -- "$cur") )
            ;;
    esac
}

complete -F _f5_complete f5
complete -F _f5_complete f5.pyz
