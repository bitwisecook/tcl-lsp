# bash completion for the `tcl` CLI.
#
# Install (system-wide):
#   sudo cp tcl.bash /etc/bash_completion.d/tcl
#
# Install (user):
#   mkdir -p ~/.local/share/bash-completion/completions
#   tcl completion bash > ~/.local/share/bash-completion/completions/tcl
#
# Or source it directly from your ~/.bashrc:
#   source <(tcl completion bash)
#
# iRules-specific verbs (event-order, event-info) live on the `f5` CLI
# under `f5 irule <subverb>` — install `f5 completion bash` for those.

_tcl_complete() {
    local cur prev words cword
    if declare -F _init_completion >/dev/null 2>&1; then
        _init_completion -n = || return
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
        prev="${COMP_WORDS[COMP_CWORD-1]}"
        words=("${COMP_WORDS[@]}")
        cword=$COMP_CWORD
    fi

    local verbs="opt diag lint validate format minify unminify-error \
        symbols diagram callgraph symbolgraph dataflow command-info \
        convert dis compwasm highlight diff explore \
        help completion pkg venv docker"
    local global_opts="-h --help --help-all --version"

    local common_opts="--source --package-path --no-recursive --dialect \
        --output -o --colour --color --no-colour --no-color --tabs"
    local diag_opts="--json --disable --enable"
    local format_opts="--indent-size --indent-style --max-line-length \
        --goal-line-length --expand-bodies --no-semicolons --keep-semicolons"
    local opt_opts="--profile --disable --enable"
    local minify_opts="--compact --symbol-map --aggressive --isolated"
    local unminify_opts="--symbol-map --error -e --error-file --minified \
        --original --output -o"
    local highlight_opts="--format --output -o"
    local diff_opts="--show --json --output -o"
    local explore_opts="--show --output -o"
    local lookup_opts="--json --dialect"
    local help_opts="--json --dialect --limit"
    local completion_shells="bash fish zsh"
    local completion_opts="--hint -h --help"

    local dialects="cadence-eda-tcl expect f5-bigip f5-iapps f5-irules \
        f5-tmsh intel-quartus-eda-tcl mentor-eda-tcl synopsys-eda-tcl \
        tcl8.4 tcl8.5 tcl8.6 tcl9.0 xilinx-eda-tcl"
    local opt_profiles="off readability standard full aggressive"
    local indent_styles="spaces tabs"
    local highlight_formats="ansi html"
    local diff_layers="ast ir cfg"
    local explore_views="ir cfg ssa interproc types opt gvn shimmer taint \
        irules callouts asm wasm all compiler optimiser"

    local pkg_actions="init install list tree verify info add remove update \
        sync outdated why vendor run freeze search"
    local venv_actions="create delete info activate deactivate list update run"
    local docker_actions="create recipe info"
    local docker_tcl_versions="8.4 8.5 8.6 9.0"

    # First positional after the command name: pick a verb.
    if [[ $cword -eq 1 ]]; then
        if [[ "$cur" == -* ]]; then
            COMPREPLY=( $(compgen -W "$global_opts" -- "$cur") )
        else
            COMPREPLY=( $(compgen -W "$verbs" -- "$cur") )
        fi
        return
    fi

    # Flags that always take a value with no enumerable completion go through
    # generic file completion (or none) regardless of which verb.
    case "$prev" in
        --dialect)
            COMPREPLY=( $(compgen -W "$dialects" -- "$cur") )
            return
            ;;
        --profile)
            COMPREPLY=( $(compgen -W "$opt_profiles" -- "$cur") )
            return
            ;;
        --indent-style)
            COMPREPLY=( $(compgen -W "$indent_styles" -- "$cur") )
            return
            ;;
        --format)
            COMPREPLY=( $(compgen -W "$highlight_formats" -- "$cur") )
            return
            ;;
        -o|--output|--symbol-map|--error-file|--minified|--original)
            COMPREPLY=( $(compgen -A file -- "$cur") )
            return
            ;;
        --package-path)
            COMPREPLY=( $(compgen -A directory -- "$cur") )
            return
            ;;
        --indent-size|--max-line-length|--goal-line-length|--tabs|--limit)
            return  # numeric, no completion
            ;;
    esac

    local verb="${words[1]}"

    case "$verb" in
        opt|optimise|optimize)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$common_opts $opt_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
                COMPREPLY+=( $(compgen -A directory -- "$cur") )
            fi
            ;;
        format|fmt)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$common_opts $format_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
                COMPREPLY+=( $(compgen -A directory -- "$cur") )
            fi
            ;;
        minify|min)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$common_opts $minify_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
                COMPREPLY+=( $(compgen -A directory -- "$cur") )
            fi
            ;;
        unminify-error|umerr)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$unminify_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
            fi
            ;;
        diag|diagnostics|lint|validate)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$common_opts $diag_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
                COMPREPLY+=( $(compgen -A directory -- "$cur") )
            fi
            ;;
        symbols|syms|diagram|callgraph|call-graph|symbolgraph|symbol-graph|\
        dataflow|convert)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$common_opts --json" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
                COMPREPLY+=( $(compgen -A directory -- "$cur") )
            fi
            ;;
        dis|asm|disassemble|compwasm|wasm)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$common_opts --wat-output" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
                COMPREPLY+=( $(compgen -A directory -- "$cur") )
            fi
            ;;
        highlight|hl)
            case "$prev" in
                --show)
                    COMPREPLY=( $(compgen -W "$diff_layers" -- "$cur") )
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$common_opts $highlight_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
            fi
            ;;
        diff)
            case "$prev" in
                --show)
                    COMPREPLY=( $(compgen -W "$diff_layers" -- "$cur") )
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$common_opts $diff_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
            fi
            ;;
        explore)
            case "$prev" in
                --show)
                    COMPREPLY=( $(compgen -W "$explore_views" -- "$cur") )
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$common_opts $explore_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
                COMPREPLY+=( $(compgen -A directory -- "$cur") )
            fi
            ;;
        command-info|commandinfo|cmd-info)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$lookup_opts" -- "$cur") )
            fi
            ;;
        help|docs)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$help_opts" -- "$cur") )
            fi
            ;;
        completion)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$completion_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$completion_shells" -- "$cur") )
            fi
            ;;
        pkg)
            if [[ $cword -eq 2 ]]; then
                if [[ "$cur" == -* ]]; then
                    COMPREPLY=( $(compgen -W "--json --offline -h --help" -- "$cur") )
                else
                    COMPREPLY=( $(compgen -W "$pkg_actions" -- "$cur") )
                fi
                return
            fi
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--json --offline --frozen --no-dev \
                    --force --dev -h --help" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
            fi
            ;;
        venv)
            if [[ $cword -eq 2 ]]; then
                if [[ "$cur" == -* ]]; then
                    COMPREPLY=( $(compgen -W "--json -h --help" -- "$cur") )
                else
                    COMPREPLY=( $(compgen -W "$venv_actions" -- "$cur") )
                fi
                return
            fi
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--json --force -h --help" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
                COMPREPLY+=( $(compgen -A directory -- "$cur") )
            fi
            ;;
        docker)
            if [[ $cword -eq 2 ]]; then
                if [[ "$cur" == -* ]]; then
                    COMPREPLY=( $(compgen -W "--json -h --help" -- "$cur") )
                else
                    COMPREPLY=( $(compgen -W "$docker_actions" -- "$cur") )
                fi
                return
            fi
            case "$prev" in
                --tcl-version)
                    COMPREPLY=( $(compgen -W "$docker_tcl_versions" -- "$cur") )
                    return
                    ;;
                -o|--output|--workdir|--entrypoint|--extra-package|--label|\
                --env|--cli-version)
                    COMPREPLY=( $(compgen -A file -- "$cur") )
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--tcl-version --output -o --workdir \
                    --entrypoint --venv --no-copy --no-packages --extra-package \
                    --label --env --cli-version --force --json -h --help" \
                    -- "$cur") )
            fi
            ;;
        *)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$common_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
                COMPREPLY+=( $(compgen -A directory -- "$cur") )
            fi
            ;;
    esac
}

complete -F _tcl_complete tcl
complete -F _tcl_complete tcl.pyz
