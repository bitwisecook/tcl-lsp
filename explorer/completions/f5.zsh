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
            # Primary verb names only — aliases (clean, related, get, mv, ...)
            # are still accepted by the CLI but intentionally omitted here.
            local -a verbs
            verbs=(
                'cleanup:Generate tmsh delete commands for objects unreferenced by any virtual'
                'completion:Print shell completion script for bash/fish/zsh'
                'convert:Convert between UCS / SCF / AS3 declaration formats'
                'diff:Object-aware diff between two bigip.conf / SCF files'
                'enrich-pcapng:Annotate a pcapng with BIG-IP context'
                'enrich-wireshark:Generate a Wireshark profile for BIG-IP captures'
                'explain:Describe the resolved profile/iRule/pool plan for a virtual'
                'explain-flow:Trace each PCAP flow through the BIG-IP config'
                'extract:Convert a local UCS archive to an SCF text file'
                'fetch:Pull SCF or UCS from a live BIG-IP device'
                'graph:Emit the object reference graph as DOT, JSON, or Mermaid'
                'grep:List every BIG-IP object related to a given object path or regex'
                'irule:iRules-specific analysis (event-order, lint, trace, ...)'
                'merge:Concatenate split per-partition SCFs back into a single bigip.conf'
                'pcap-remap:Apply a redact map to a PCAP capture'
                'pull:Fetch a single object (virtual/pool/node/rule) from a live BIG-IP'
                'push:Replace or create a single object on a live BIG-IP via iControl REST'
                'redact:Strip secrets and remap public IPs into a configurable pool'
                'rename:Rename a BIG-IP object full-path and update every reference'
                'split:Split an SCF into per-partition files under a directory'
                'stats:Print object counts, partition breakdown, and top-references'
                'tmsh:Emit a tmsh script that creates every object in the input'
                'unredact:Reverse a previous redact using its sidecar map file'
                'validate:Run BIG-IP best-practice / structural checks'
            )
            _describe -t verbs 'verb' verbs
            ;;
        args)
            # Normalise the documented aliases so the right branch fires
            # even when the user typed an alias.
            local verb="$line[1]"
            case "$verb" in
                clean)      verb=cleanup ;;
                related)    verb=grep ;;
                changes)    verb=diff ;;
                get)        verb=fetch ;;
                deps)       verb=graph ;;
                describe)   verb=explain ;;
                ucs2scf)    verb=extract ;;
                summary)    verb=stats ;;
                scf2tmsh)   verb=tmsh ;;
                mv)         verb=rename ;;
                sanitize)   verb=redact ;;
                unmap)      verb=unredact ;;
                enrich)     verb=enrich-pcapng ;;
                ws-profile) verb=enrich-wireshark ;;
                pcapmap)    verb=pcap-remap ;;
                lint)       verb=validate ;;
            esac
            case "$verb" in
                cleanup)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--json[Emit cleanup report as JSON]' \
                        '*--keep[Object full-path or partition prefix to retain]:keep:_files' \
                        '--no-keep-common[Do not auto-keep /Common/*]' \
                        '(-o --output)'{-o,--output}'[Write output here]:output file:_files' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                convert)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--application[AS3 application name]:application' \
                        '--tenant[AS3 tenant name]:tenant' \
                        '--report[Write a conversion report]:report file:_files' \
                        '(-o --output)'{-o,--output}'[Output path]:output file:_files' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                diff)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--json[Emit diff as JSON]' \
                        '(-o --output)'{-o,--output}'[Output path]:output file:_files' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                enrich-pcapng)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--all[Annotate every packet]' \
                        '--config[BIG-IP config]:config:_files -g "*.{conf,scf,ucs}"' \
                        '--dry-run[Do not write output]' \
                        '--keylog[TLS key log file]:keylog:_files' \
                        '*:input:_files'
                    ;;
                enrich-wireshark)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--config[BIG-IP config]:config:_files' \
                        '--force[Overwrite existing profile directory]' \
                        '*:input:_files'
                    ;;
                explain)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--json[Emit plan as JSON]' \
                        '(-o --output)'{-o,--output}'[Output path]:output file:_files' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                explain-flow)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--json[Emit flows as JSON]' \
                        '--keylog[TLS key log file]:keylog:_files' \
                        '--max-event-lines[Cap each event body at N lines]:N' \
                        '--no-event-bodies[Do not show iRule event bodies]' \
                        '(-o --output)'{-o,--output}'[Output path]:output file:_files' \
                        '--simulate[Simulate flow execution]' \
                        '--tshark[Path to tshark binary]:tshark:_files' \
                        '--tshark-filter[Display filter passed to tshark]:filter' \
                        '*:input:_files'
                    ;;
                extract)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--include-extras[Also extract sidecar files (certs, keys, ...)]' \
                        '--output[Output SCF path]:output:_files' \
                        '*:ucs:_files -g "*.ucs"'
                    ;;
                fetch)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--format[Output format]:format:(scf ucs)' \
                        '--host[BIG-IP host]:host' \
                        '--insecure[Skip TLS verification]' \
                        '--no-prompt[Fail rather than prompt for credentials]' \
                        '--output[Output path]:output:_files' \
                        '--password[BIG-IP admin password]:password' \
                        '--port[iControl REST TCP port]:port' \
                        '--print-path[Print saved path to stdout]' \
                        '--ssh-port[SSH TCP port]:port' \
                        '--timeout[Request timeout (seconds)]:seconds' \
                        '--transport[Transport mechanism]:transport:(rest ssh auto)' \
                        '--user[BIG-IP admin user]:user'
                    ;;
                graph)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--format[Output format]:format:(dot json mermaid)' \
                        '--max-depth[Stop BFS after N hops]:N' \
                        '(-o --output)'{-o,--output}'[Output path]:output file:_files' \
                        '--reverse[Traverse reverse edges (referenced-by)]' \
                        '--seed[Restrict graph to objects reachable from seed]:seed' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                grep)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '(-e --regex)'{-e,--regex}'[Treat PATTERN as a Python regular expression]' \
                        '--direction[Which edges to traverse]:direction:(forward reverse both)' \
                        '--max-depth[Stop BFS after N hops]:max depth:' \
                        '--max-nodes[Cap result at N objects]:max nodes:' \
                        '--full[Print each object full body]' \
                        '--cidr[Treat PATTERN as a CIDR range]' \
                        '--recurse[Recurse through reference edges]' \
                        '--json[Emit grep report as JSON]' \
                        '(-o --output)'{-o,--output}'[Write output here]:output file:_files' \
                        '1:pattern:' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                merge)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '(-o --output)'{-o,--output}'[Output path]:output file:_files' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                pcap-remap)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--list-schemas[List built-in mapping schemas]' \
                        '--on-unknown[Behaviour for unmapped addresses]:mode' \
                        '--reverse[Apply the map in reverse]' \
                        '--schema[Use a built-in schema]:schema' \
                        '*:input:_files'
                    ;;
                pull)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--host[BIG-IP host]:host' \
                        '--insecure[Skip TLS verification]' \
                        '--json[Emit object as JSON]' \
                        '--no-prompt[Fail rather than prompt for credentials]' \
                        '--password[BIG-IP admin password]:password' \
                        '--port[iControl REST TCP port]:port' \
                        '--timeout[Request timeout (seconds)]:seconds' \
                        '--user[BIG-IP admin user]:user'
                    ;;
                push)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--create[Create if missing]' \
                        '--dry-run[Print the API call but do not send]' \
                        '--host[BIG-IP host]:host' \
                        '--insecure[Skip TLS verification]' \
                        '--no-prompt[Fail rather than prompt for credentials]' \
                        '--password[BIG-IP admin password]:password' \
                        '--port[iControl REST TCP port]:port' \
                        '--timeout[Request timeout (seconds)]:seconds' \
                        '--user[BIG-IP admin user]:user'
                    ;;
                redact)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--keep-ips[Do not remap any IPs]' \
                        '--map-file[Write the redaction map sidecar here]:map file:_files' \
                        '--remap-private[Remap RFC1918 addresses too]' \
                        '--seed[Deterministic remap seed]:seed' \
                        '--shuffle[Randomly permute target IP allocations]' \
                        '--source-cidr[CIDR(s) to remap (repeatable)]:cidr' \
                        '--target-cidr[CIDR pool to allocate replacements from]:cidr' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                rename)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--in-place[Rewrite each input file in place]' \
                        '--output[Output path]:output:_files' \
                        '--write[Persist changes (default is preview)]' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                split)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                stats)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--json[Emit stats as JSON]' \
                        '(-o --output)'{-o,--output}'[Output path]:output file:_files' \
                        '--top[Show top N referenced objects]:N' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                tmsh)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--include[Include only matching object kinds]:kind' \
                        '--modify[Emit `modify` commands for existing objects]' \
                        '--output[Output path]:output:_files' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                unredact)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                validate)
                    _arguments \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        '--category[Filter to a lint-rule category]:category' \
                        '--format[Output format]:format:(text json sarif)' \
                        '(-o --output)'{-o,--output}'[Output path]:output file:_files' \
                        '--severity[Minimum severity to report]:severity:(error warning info)' \
                        '*:bigip config:_files -g "*.{conf,scf,ucs}"'
                    ;;
                completion)
                    _arguments \
                        '--hint[Print install instructions to stderr]' \
                        '1:shell:(bash fish zsh)'
                    ;;
                irule)
                    if (( CURRENT == 2 )); then
                        local -a irule_actions
                        irule_actions=(
                            'event-order:Show iRules events in canonical firing order'
                            'event-info:Look up iRules event metadata and valid commands'
                            'lint:Apply iRule-only lint rules'
                            'trace:Static event-flow trace from a starting event'
                            'extract:Split a config into per-rule .tcl files'
                            'format:Pretty-print iRule source'
                            'minify:Strip whitespace/comments from iRule source'
                            'context:Bundle each iRule with the BIG-IP objects it references'
                        )
                        _describe -t irule_actions 'action' irule_actions
                    else
                        local action="$line[2]"
                        case "$action" in
                            eventorder) action=event-order ;;
                            eventinfo)  action=event-info ;;
                            fmt)        action=format ;;
                            min)        action=minify ;;
                        esac
                        local common_irule_args=(
                            '*--source[Inline iRules source text (repeatable)]:source text'
                            '--dialect[Dialect profile]:dialect:(f5-irules)'
                            '(-o --output)'{-o,--output}'[Output path]:output file:_files'
                            '*:input:_files'
                        )
                        case "$action" in
                            event-order)
                                _arguments \
                                    $common_irule_args \
                                    '*--package-path[Directory to scan for pkgIndex.tcl]:dir:_files -/' \
                                    '--no-recursive[Do not recurse into directory inputs]' \
                                    '--json[Emit event ordering as JSON]'
                                ;;
                            event-info)
                                _arguments \
                                    '--json[Emit event metadata as JSON]' \
                                    '(-o --output)'{-o,--output}'[Output path]:output file:_files' \
                                    '1:event:'
                                ;;
                            lint)
                                _arguments \
                                    $common_irule_args \
                                    '--json[Emit lint results as JSON]' \
                                    '--severity[Filter to a severity level]:severity:(error warning info)'
                                ;;
                            trace)
                                _arguments \
                                    $common_irule_args \
                                    '--json[Emit trace as JSON]' \
                                    '1:event:'
                                ;;
                            extract)
                                _arguments \
                                    '1:input:_files -g "*.{conf,scf,ucs}"' \
                                    '2:output dir:_files -/'
                                ;;
                            format)
                                _arguments $common_irule_args
                                ;;
                            minify)
                                _arguments \
                                    $common_irule_args \
                                    '--compact[Compact variable and proc names]' \
                                    '--aggressive[Run all optimisations + name compaction]' \
                                    '--isolated[Compact global-scope names too]' \
                                    '--symbol-map[Write symbol map to FILE]:map file:_files' \
                                    '--json[Emit minify report as JSON]'
                                ;;
                            context)
                                _arguments \
                                    $common_irule_args \
                                    '*--rule[Limit output to rules with this full path (repeatable)]:full path' \
                                    '--no-transitive[Skip the one-hop transitive expansion]' \
                                    '--json[Emit each bundle as a JSON object]'
                                ;;
                        esac
                    fi
                    ;;
            esac
            ;;
    esac
}

_f5 "$@"
