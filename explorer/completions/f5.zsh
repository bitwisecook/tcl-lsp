#compdef f5
#
# zsh completion for the f5 BIG-IP CLI.
#
# Install (per-user):
#   mkdir -p "${ZDOTDIR:-$HOME}/.zsh/completions"
#   f5 completion zsh > "${ZDOTDIR:-$HOME}/.zsh/completions/_f5"
#
#   # Then add to .zshrc, before `compinit`:
#   #   fpath=("${ZDOTDIR:-$HOME}/.zsh/completions" $fpath)
#
# Or, with oh-my-zsh:
#   f5 completion zsh > ~/.oh-my-zsh/completions/_f5
#
# Or system-wide on systems where ``$fpath`` already covers it
# (e.g. /usr/share/zsh/site-functions):
#   sudo f5 completion zsh > /usr/share/zsh/site-functions/_f5

_f5() {
    local context curcontext="$curcontext" state line
    typeset -A opt_args

    _arguments -C \
        '(-h --help)'{-h,--help}'[Show brief help and exit]' \
        '--help-all[Show full help for every verb and exit]' \
        '--version[Print build version and exit]' \
        '1: :->verb' \
        '*::arg:->args'

    case "$state" in
        verb)
            local -a verbs
            verbs=(
                'cleanup:Generate tmsh delete commands for objects unreferenced by any virtual'
                'clean:Alias for cleanup'
                'completion:Print shell completion script for bash/fish/zsh'
                'grep:List every BIG-IP object related to a given object path or regex'
                'related:Alias for grep'
                'irule:iRules-specific analysis (event-order, event-info, ...)'
            )
            _describe -t verbs 'verb' verbs
            ;;
        args)
            case "$line[1]" in
                cleanup|clean)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--json[Emit cleanup report as JSON]' \
                        '*--keep[Object full-path or partition prefix to retain]:keep:_files' \
                        '--no-keep-common[Do not auto-keep /Common/*]' \
                        '(-o --output)'{-o,--output}'[Write output here (default: stdout)]:output file:_files' \
                        '*:bigip config:_files -g "*.{conf,scf}"'
                    ;;
                grep|related)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '(-e --regex)'{-e,--regex}'[Treat PATTERN as a Python regular expression]' \
                        '--direction[Which edges to traverse]:direction:(forward reverse both)' \
                        '--max-depth[Stop BFS after N hops]:max depth:' \
                        '--max-nodes[Cap result at N objects]:max nodes:' \
                        '--full[Print each object full body]' \
                        '--json[Emit grep report as JSON]' \
                        '(-o --output)'{-o,--output}'[Write output here (default: stdout)]:output file:_files' \
                        '1:pattern:' \
                        '*:bigip config:_files -g "*.{conf,scf}"'
                    ;;
                completion)
                    _arguments '1:shell:(bash fish zsh)'
                    ;;
                irule)
                    if (( CURRENT == 2 )); then
                        local -a irule_actions
                        irule_actions=(
                            'event-order:Show iRules events in canonical firing order'
                            'eventorder:Alias for event-order'
                            'event-info:Look up iRules event metadata and valid commands'
                            'eventinfo:Alias for event-info'
                        )
                        _describe -t irule_actions 'action' irule_actions
                    else
                        case "$line[2]" in
                            event-order|eventorder)
                                _arguments \
                                    '*--source[Inline iRules source text (repeatable)]:source text' \
                                    '*--package-path[Directory to scan for pkgIndex.tcl]:dir:_files -/' \
                                    '--no-recursive[Do not recurse into directory inputs]' \
                                    '--dialect[Dialect profile]:dialect:(f5-irules)' \
                                    '(-o --output)'{-o,--output}'[Output path (- for stdout)]:output file:_files' \
                                    '--json[Emit event ordering as JSON]' \
                                    '*:input:_files'
                                ;;
                            event-info|eventinfo)
                                _arguments \
                                    '--json[Emit event metadata as JSON]' \
                                    '(-o --output)'{-o,--output}'[Output path]:output file:_files' \
                                    '1:event:'
                                ;;
                        esac
                    fi
                    ;;
            esac
            ;;
    esac
}

_f5 "$@"
