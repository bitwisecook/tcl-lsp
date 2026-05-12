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

    # Primary verb names only — aliases (clean, related, get, mv, sanitize,
    # ...) are still accepted by the CLI but intentionally omitted from
    # the completion list to avoid clutter.
    local verbs="cleanup completion convert diff enrich-pcapng enrich-wireshark \
        explain explain-flow extract fetch graph grep irule merge pcap-remap \
        pull push redact rename split stats tmsh unredact validate"
    local global_opts="-h --help --help-all --version"

    local cleanup_opts="--json --keep --no-keep-common -o --output -h --help"
    local convert_opts="--application --tenant --report -o --output -h --help"
    local diff_opts="--json -o --output -h --help"
    local enrich_pcapng_opts="--all --config --dry-run --keylog -h --help"
    local enrich_wireshark_opts="--config --force -h --help"
    local explain_opts="--json -o --output -h --help"
    local explain_flow_opts="--json --keylog --max-event-lines --no-event-bodies \
        -o --output --simulate --tshark --tshark-filter -h --help"
    local extract_opts="--include-extras --output -h --help"
    local fetch_opts="--format --host --insecure --no-prompt --output --password \
        --port --print-path --ssh-port --timeout --transport --user -h --help"
    local fetch_formats="scf ucs"
    local fetch_transports="rest ssh auto"
    local graph_opts="--format --max-depth -o --output --reverse --seed -h --help"
    local graph_formats="dot json mermaid"
    local grep_opts="-e --regex --direction --max-depth --max-nodes --full \
        --cidr --recurse --json -o --output -h --help"
    local grep_directions="forward reverse both"
    local merge_opts="-o --output -h --help"
    local pcap_remap_opts="--list-schemas --on-unknown --reverse --schema -h --help"
    local pull_opts="--host --insecure --json --no-prompt --password --port \
        --timeout --user -h --help"
    local push_opts="--create --dry-run --host --insecure --no-prompt --password \
        --port --timeout --user -h --help"
    local redact_opts="--keep-ips --map-file --remap-private --seed --shuffle \
        --source-cidr --target-cidr -h --help"
    local rename_opts="--in-place --output --write -h --help"
    local split_opts="-h --help"
    local stats_opts="--json -o --output --top -h --help"
    local tmsh_opts="--include --modify --output -h --help"
    local unredact_opts="-h --help"
    local validate_opts="--category --format -o --output --severity -h --help"
    local validate_severities="error warning info"
    local validate_formats="text json sarif"

    local completion_shells="bash fish zsh"
    local completion_opts="--hint -h --help"

    local irule_actions="event-order event-info lint trace extract format \
        minify context"
    local irule_common_opts="--source --dialect -o --output -h --help"
    local irule_event_order_opts="$irule_common_opts --json --package-path \
        --no-recursive"
    local irule_event_info_opts="--json -o --output -h --help"
    local irule_lint_opts="$irule_common_opts --json --severity"
    local irule_trace_opts="$irule_common_opts --json"
    local irule_format_opts="$irule_common_opts"
    local irule_minify_opts="$irule_common_opts --compact --aggressive \
        --isolated --symbol-map --json"
    local irule_context_opts="$irule_common_opts --rule --no-transitive --json"
    local irule_severities="error warning info"

    # First positional after the command name: pick a verb.
    if [[ $cword -eq 1 ]]; then
        if [[ "$cur" == -* ]]; then
            COMPREPLY=( $(compgen -W "$global_opts" -- "$cur") )
        else
            COMPREPLY=( $(compgen -W "$verbs" -- "$cur") )
        fi
        return
    fi

    # Flags that take a value with an enumerable completion — keyed on $prev.
    case "$prev" in
        -o|--output|--keylog|--keep|--map-file|--symbol-map|--config|--include|--modify)
            COMPREPLY=( $(compgen -A file -- "$cur") )
            return
            ;;
        --max-depth|--max-nodes|--max-event-lines|--top|--port|--ssh-port|--timeout|--seed)
            return  # numeric — no completion
            ;;
    esac

    # Resolve the verb word, normalising the documented aliases so the
    # right case-branch fires even when the user typed an alias.
    local verb="${words[1]}"
    case "$verb" in
        clean)            verb=cleanup ;;
        related)          verb=grep ;;
        changes)          verb=diff ;;
        get)              verb=fetch ;;
        deps)             verb=graph ;;
        describe)         verb=explain ;;
        ucs2scf)          verb=extract ;;
        summary)          verb=stats ;;
        scf2tmsh)         verb=tmsh ;;
        mv)               verb=rename ;;
        sanitize)         verb=redact ;;
        unmap)            verb=unredact ;;
        enrich)           verb=enrich-pcapng ;;
        ws-profile)       verb=enrich-wireshark ;;
        pcapmap)          verb=pcap-remap ;;
        lint)             verb=validate ;;
    esac

    case "$verb" in
        cleanup)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$cleanup_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        convert)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$convert_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        diff)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$diff_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        enrich-pcapng)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$enrich_pcapng_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
            fi
            ;;
        enrich-wireshark)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$enrich_wireshark_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
            fi
            ;;
        explain)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$explain_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        explain-flow)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$explain_flow_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
            fi
            ;;
        extract)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$extract_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
            fi
            ;;
        fetch)
            case "$prev" in
                --format)
                    COMPREPLY=( $(compgen -W "$fetch_formats" -- "$cur") )
                    return
                    ;;
                --transport)
                    COMPREPLY=( $(compgen -W "$fetch_transports" -- "$cur") )
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$fetch_opts" -- "$cur") )
            fi
            ;;
        graph)
            case "$prev" in
                --format)
                    COMPREPLY=( $(compgen -W "$graph_formats" -- "$cur") )
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$graph_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        grep)
            case "$prev" in
                --direction)
                    COMPREPLY=( $(compgen -W "$grep_directions" -- "$cur") )
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$grep_opts" -- "$cur") )
            else
                # First non-flag positional is the pattern (free-form).
                # Subsequent positionals are config files.
                local non_flag_count=0
                local i
                for ((i=2; i<cword; i++)); do
                    case "${words[i]}" in
                        -*) ;;
                        *)
                            case "${words[i-1]}" in
                                --direction|--max-depth|--max-nodes|-o|--output)
                                    ;;
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
                    _f5_complete_conf_files
                fi
            fi
            ;;
        merge)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$merge_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        pcap-remap)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$pcap_remap_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -A file -- "$cur") )
            fi
            ;;
        pull)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$pull_opts" -- "$cur") )
            fi
            ;;
        push)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$push_opts" -- "$cur") )
            fi
            ;;
        redact)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$redact_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        rename)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$rename_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        split)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$split_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        stats)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$stats_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        tmsh)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$tmsh_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        unredact)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$unredact_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        validate)
            case "$prev" in
                --severity)
                    COMPREPLY=( $(compgen -W "$validate_severities" -- "$cur") )
                    return
                    ;;
                --format)
                    COMPREPLY=( $(compgen -W "$validate_formats" -- "$cur") )
                    return
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$validate_opts" -- "$cur") )
            else
                _f5_complete_conf_files
            fi
            ;;
        completion)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "$completion_opts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$completion_shells" -- "$cur") )
            fi
            ;;
        irule)
            # f5 irule <action> [opts...]
            if [[ $cword -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "$irule_actions" -- "$cur") )
                return
            fi
            local action="${words[2]}"
            # Normalise irule action aliases.
            case "$action" in
                eventorder) action=event-order ;;
                eventinfo)  action=event-info ;;
                fmt)        action=format ;;
                min)        action=minify ;;
            esac
            case "$action" in
                event-order)
                    case "$prev" in
                        --dialect)
                            COMPREPLY=( $(compgen -W "f5-irules" -- "$cur") )
                            return
                            ;;
                    esac
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "$irule_event_order_opts" -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -A file -- "$cur") )
                        COMPREPLY+=( $(compgen -A directory -- "$cur") )
                    fi
                    ;;
                event-info)
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "$irule_event_info_opts" -- "$cur") )
                    fi
                    ;;
                lint)
                    case "$prev" in
                        --severity)
                            COMPREPLY=( $(compgen -W "$irule_severities" -- "$cur") )
                            return
                            ;;
                    esac
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "$irule_lint_opts" -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -A file -- "$cur") )
                    fi
                    ;;
                trace)
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "$irule_trace_opts" -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -A file -- "$cur") )
                    fi
                    ;;
                extract)
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "-h --help" -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -A file -- "$cur") )
                        COMPREPLY+=( $(compgen -A directory -- "$cur") )
                    fi
                    ;;
                format)
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "$irule_format_opts" -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -A file -- "$cur") )
                    fi
                    ;;
                minify)
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "$irule_minify_opts" -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -A file -- "$cur") )
                    fi
                    ;;
                context)
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "$irule_context_opts" -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -A file -- "$cur") )
                    fi
                    ;;
            esac
            ;;
    esac
}

# Helper: positional config file completion — restrict to .conf / .scf
# plus directories (so users can tab into the directory containing
# their configs).
_f5_complete_conf_files() {
    local files dirs f
    files=( $(compgen -A file -- "$cur") )
    dirs=( $(compgen -A directory -- "$cur") )
    COMPREPLY=()
    for f in "${files[@]}"; do
        case "$f" in
            *.conf|*.scf|*.ucs|-) COMPREPLY+=( "$f" ) ;;
        esac
    done
    COMPREPLY+=( "${dirs[@]}" )
}

complete -F _f5_complete f5
complete -F _f5_complete f5.pyz
