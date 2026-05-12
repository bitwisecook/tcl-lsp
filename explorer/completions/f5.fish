# fish completion for the f5 BIG-IP CLI.
#
# Install:
#   mkdir -p ~/.config/fish/completions
#   f5 completion fish > ~/.config/fish/completions/f5.fish
#
# Then start a new shell.

function __f5_uses_subcommand
    set -l cmd (commandline -opc)
    if test (count $cmd) -ge 2
        # Normalise documented aliases so the *primary* verb name fires
        # the right branch even when the user typed an alias.
        set -l verb $cmd[2]
        switch $verb
            case clean
                set verb cleanup
            case related
                set verb grep
            case changes
                set verb diff
            case get
                set verb fetch
            case deps
                set verb graph
            case describe
                set verb explain
            case ucs2scf
                set verb extract
            case summary
                set verb stats
            case scf2tmsh
                set verb tmsh
            case mv
                set verb rename
            case sanitize
                set verb redact
            case unmap
                set verb unredact
            case enrich
                set verb enrich-pcapng
            case ws-profile
                set verb enrich-wireshark
            case pcapmap
                set verb pcap-remap
            case lint
                set verb validate
        end
        if contains -- $verb $argv
            return 0
        end
    end
    return 1
end

function __f5_no_subcommand
    set -l cmd (commandline -opc)
    test (count $cmd) -lt 2
end

# Top-level verbs (primary names only — aliases are still accepted by
# the CLI but intentionally omitted from the completion list to avoid
# cluttering the picker).
complete -c f5 -f
complete -c f5 -n __f5_no_subcommand -a cleanup -d 'Generate tmsh delete commands for unreferenced objects'
complete -c f5 -n __f5_no_subcommand -a completion -d 'Print shell completion script'
complete -c f5 -n __f5_no_subcommand -a convert -d 'Convert between UCS / SCF / AS3 declaration formats'
complete -c f5 -n __f5_no_subcommand -a diff -d 'Object-aware diff between two bigip.conf / SCF files'
complete -c f5 -n __f5_no_subcommand -a enrich-pcapng -d 'Annotate pcapng with BIG-IP context'
complete -c f5 -n __f5_no_subcommand -a enrich-wireshark -d 'Generate a Wireshark profile for BIG-IP captures'
complete -c f5 -n __f5_no_subcommand -a explain -d 'Describe the resolved profile/iRule/pool plan for a virtual'
complete -c f5 -n __f5_no_subcommand -a explain-flow -d 'Trace PCAP flows through the BIG-IP config'
complete -c f5 -n __f5_no_subcommand -a extract -d 'Convert a local UCS archive to an SCF text file'
complete -c f5 -n __f5_no_subcommand -a fetch -d 'Pull SCF or UCS from a live BIG-IP device'
complete -c f5 -n __f5_no_subcommand -a graph -d 'Emit the object reference graph as DOT/JSON/Mermaid'
complete -c f5 -n __f5_no_subcommand -a grep -d 'List every BIG-IP object related to a given object path or regex'
complete -c f5 -n __f5_no_subcommand -a irule -d 'iRules-specific analysis (event-order, lint, trace, ...)'
complete -c f5 -n __f5_no_subcommand -a merge -d 'Concatenate split per-partition SCFs back into a single bigip.conf'
complete -c f5 -n __f5_no_subcommand -a pcap-remap -d 'Apply a redact map to a PCAP capture'
complete -c f5 -n __f5_no_subcommand -a pull -d 'Fetch a single object (virtual/pool/node/rule) from a live BIG-IP'
complete -c f5 -n __f5_no_subcommand -a push -d 'Replace or create a single object on a live BIG-IP via iControl REST'
complete -c f5 -n __f5_no_subcommand -a redact -d 'Strip secrets and remap public IPs into a configurable pool'
complete -c f5 -n __f5_no_subcommand -a rename -d 'Rename a BIG-IP object full-path and update every reference'
complete -c f5 -n __f5_no_subcommand -a split -d 'Split an SCF into per-partition files under a directory'
complete -c f5 -n __f5_no_subcommand -a stats -d 'Print object counts, partition breakdown, and top-references'
complete -c f5 -n __f5_no_subcommand -a tmsh -d 'Emit a tmsh script that creates every object in the input'
complete -c f5 -n __f5_no_subcommand -a unredact -d 'Reverse a previous redact using its sidecar map file'
complete -c f5 -n __f5_no_subcommand -a validate -d 'Run BIG-IP best-practice / structural checks'

# Top-level help / version.
complete -c f5 -n __f5_no_subcommand -s h -l help -d 'Show brief help and exit'
complete -c f5 -n __f5_no_subcommand -l help-all -d 'Show full help for every verb and exit'
complete -c f5 -n __f5_no_subcommand -l version -d 'Print build version and exit'

# Helper: every verb that accepts bigip.conf / SCF positional inputs.
set -l __f5_conf_verbs cleanup convert diff explain extract graph grep merge \
    redact rename split stats tmsh unredact validate

# Positional .conf / .scf / .ucs file completion for those verbs.
complete -c f5 -n "__f5_uses_subcommand $__f5_conf_verbs" -F -k \
    -a "(__fish_complete_suffix .conf; __fish_complete_suffix .scf; __fish_complete_suffix .ucs)"

# ── cleanup ────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand cleanup' -l json -d 'Emit cleanup report as JSON'
complete -c f5 -n '__f5_uses_subcommand cleanup' -l keep -r -d 'Object full-path or partition prefix to retain'
complete -c f5 -n '__f5_uses_subcommand cleanup' -l no-keep-common -d 'Do not auto-keep /Common/*'
complete -c f5 -n '__f5_uses_subcommand cleanup' -s o -l output -r -F -d 'Write output here (default: stdout)'
complete -c f5 -n '__f5_uses_subcommand cleanup' -s h -l help -d 'Show help'

# ── convert ────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand convert' -l application -r -d 'AS3 application name'
complete -c f5 -n '__f5_uses_subcommand convert' -l tenant -r -d 'AS3 tenant name'
complete -c f5 -n '__f5_uses_subcommand convert' -l report -r -F -d 'Write a conversion report'
complete -c f5 -n '__f5_uses_subcommand convert' -s o -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand convert' -s h -l help -d 'Show help'

# ── diff ───────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand diff' -l json -d 'Emit diff as JSON'
complete -c f5 -n '__f5_uses_subcommand diff' -s o -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand diff' -s h -l help -d 'Show help'

# ── enrich-pcapng ──────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand enrich-pcapng' -l all -d 'Annotate every packet'
complete -c f5 -n '__f5_uses_subcommand enrich-pcapng' -l config -r -F -d 'BIG-IP config (.conf/.scf/.ucs)'
complete -c f5 -n '__f5_uses_subcommand enrich-pcapng' -l dry-run -d 'Do not write output'
complete -c f5 -n '__f5_uses_subcommand enrich-pcapng' -l keylog -r -F -d 'TLS key log file'
complete -c f5 -n '__f5_uses_subcommand enrich-pcapng' -s h -l help -d 'Show help'

# ── enrich-wireshark ───────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand enrich-wireshark' -l config -r -F -d 'BIG-IP config'
complete -c f5 -n '__f5_uses_subcommand enrich-wireshark' -l force -d 'Overwrite existing profile directory'
complete -c f5 -n '__f5_uses_subcommand enrich-wireshark' -s h -l help -d 'Show help'

# ── explain ────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand explain' -l json -d 'Emit plan as JSON'
complete -c f5 -n '__f5_uses_subcommand explain' -s o -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand explain' -s h -l help -d 'Show help'

# ── explain-flow ───────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand explain-flow' -l json -d 'Emit flows as JSON'
complete -c f5 -n '__f5_uses_subcommand explain-flow' -l keylog -r -F -d 'TLS key log file'
complete -c f5 -n '__f5_uses_subcommand explain-flow' -l max-event-lines -r -d 'Cap each event body at N lines'
complete -c f5 -n '__f5_uses_subcommand explain-flow' -l no-event-bodies -d 'Do not show iRule event bodies'
complete -c f5 -n '__f5_uses_subcommand explain-flow' -s o -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand explain-flow' -l simulate -d 'Simulate flow execution'
complete -c f5 -n '__f5_uses_subcommand explain-flow' -l tshark -r -F -d 'Path to tshark binary'
complete -c f5 -n '__f5_uses_subcommand explain-flow' -l tshark-filter -r -d 'Display filter passed to tshark'
complete -c f5 -n '__f5_uses_subcommand explain-flow' -s h -l help -d 'Show help'

# ── extract ────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand extract' -l include-extras -d 'Also extract sidecar files (certs, keys, ...)'
complete -c f5 -n '__f5_uses_subcommand extract' -l output -r -F -d 'Output SCF path'
complete -c f5 -n '__f5_uses_subcommand extract' -s h -l help -d 'Show help'

# ── fetch ──────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand fetch' -l format -r -a 'scf ucs' -d 'Output format'
complete -c f5 -n '__f5_uses_subcommand fetch' -l host -r -d 'BIG-IP host (IP or hostname)'
complete -c f5 -n '__f5_uses_subcommand fetch' -l insecure -d 'Skip TLS certificate verification'
complete -c f5 -n '__f5_uses_subcommand fetch' -l no-prompt -d 'Fail rather than prompt for credentials'
complete -c f5 -n '__f5_uses_subcommand fetch' -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand fetch' -l password -r -d 'BIG-IP admin password'
complete -c f5 -n '__f5_uses_subcommand fetch' -l port -r -d 'iControl REST TCP port'
complete -c f5 -n '__f5_uses_subcommand fetch' -l print-path -d 'Print the saved path to stdout'
complete -c f5 -n '__f5_uses_subcommand fetch' -l ssh-port -r -d 'SSH TCP port'
complete -c f5 -n '__f5_uses_subcommand fetch' -l timeout -r -d 'Request timeout (seconds)'
complete -c f5 -n '__f5_uses_subcommand fetch' -l transport -r -a 'rest ssh auto' -d 'Transport mechanism'
complete -c f5 -n '__f5_uses_subcommand fetch' -l user -r -d 'BIG-IP admin user'
complete -c f5 -n '__f5_uses_subcommand fetch' -s h -l help -d 'Show help'

# ── graph ──────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand graph' -l format -r -a 'dot json mermaid' -d 'Output format'
complete -c f5 -n '__f5_uses_subcommand graph' -l max-depth -r -d 'Stop BFS after N hops'
complete -c f5 -n '__f5_uses_subcommand graph' -s o -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand graph' -l reverse -d 'Traverse reverse edges (referenced-by)'
complete -c f5 -n '__f5_uses_subcommand graph' -l seed -r -d 'Restrict graph to objects reachable from seed'
complete -c f5 -n '__f5_uses_subcommand graph' -s h -l help -d 'Show help'

# ── grep ───────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand grep' -s e -l regex -d 'Treat PATTERN as a Python regular expression'
complete -c f5 -n '__f5_uses_subcommand grep' -l direction -r -a 'forward reverse both' -d 'Which edges to traverse'
complete -c f5 -n '__f5_uses_subcommand grep' -l max-depth -r -d 'Stop BFS after N hops'
complete -c f5 -n '__f5_uses_subcommand grep' -l max-nodes -r -d 'Cap result at N objects'
complete -c f5 -n '__f5_uses_subcommand grep' -l full -d 'Print each object full body'
complete -c f5 -n '__f5_uses_subcommand grep' -l cidr -d 'Treat PATTERN as a CIDR range to match IP/node objects'
complete -c f5 -n '__f5_uses_subcommand grep' -l recurse -d 'Recurse through reference edges'
complete -c f5 -n '__f5_uses_subcommand grep' -l json -d 'Emit grep report as JSON'
complete -c f5 -n '__f5_uses_subcommand grep' -s o -l output -r -F -d 'Write output here (default: stdout)'
complete -c f5 -n '__f5_uses_subcommand grep' -s h -l help -d 'Show help'

# ── merge ──────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand merge' -s o -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand merge' -s h -l help -d 'Show help'

# ── pcap-remap ─────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand pcap-remap' -l list-schemas -d 'List built-in mapping schemas'
complete -c f5 -n '__f5_uses_subcommand pcap-remap' -l on-unknown -r -d 'Behaviour for unmapped addresses'
complete -c f5 -n '__f5_uses_subcommand pcap-remap' -l reverse -d 'Apply the map in reverse'
complete -c f5 -n '__f5_uses_subcommand pcap-remap' -l schema -r -d 'Use a built-in schema'
complete -c f5 -n '__f5_uses_subcommand pcap-remap' -s h -l help -d 'Show help'

# ── pull ───────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand pull' -l host -r -d 'BIG-IP host'
complete -c f5 -n '__f5_uses_subcommand pull' -l insecure -d 'Skip TLS certificate verification'
complete -c f5 -n '__f5_uses_subcommand pull' -l json -d 'Emit object as JSON'
complete -c f5 -n '__f5_uses_subcommand pull' -l no-prompt -d 'Fail rather than prompt for credentials'
complete -c f5 -n '__f5_uses_subcommand pull' -l password -r -d 'BIG-IP admin password'
complete -c f5 -n '__f5_uses_subcommand pull' -l port -r -d 'iControl REST TCP port'
complete -c f5 -n '__f5_uses_subcommand pull' -l timeout -r -d 'Request timeout (seconds)'
complete -c f5 -n '__f5_uses_subcommand pull' -l user -r -d 'BIG-IP admin user'
complete -c f5 -n '__f5_uses_subcommand pull' -s h -l help -d 'Show help'

# ── push ───────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand push' -l create -d 'Create if missing (POST instead of PATCH/PUT)'
complete -c f5 -n '__f5_uses_subcommand push' -l dry-run -d 'Print the API call but do not send'
complete -c f5 -n '__f5_uses_subcommand push' -l host -r -d 'BIG-IP host'
complete -c f5 -n '__f5_uses_subcommand push' -l insecure -d 'Skip TLS certificate verification'
complete -c f5 -n '__f5_uses_subcommand push' -l no-prompt -d 'Fail rather than prompt for credentials'
complete -c f5 -n '__f5_uses_subcommand push' -l password -r -d 'BIG-IP admin password'
complete -c f5 -n '__f5_uses_subcommand push' -l port -r -d 'iControl REST TCP port'
complete -c f5 -n '__f5_uses_subcommand push' -l timeout -r -d 'Request timeout (seconds)'
complete -c f5 -n '__f5_uses_subcommand push' -l user -r -d 'BIG-IP admin user'
complete -c f5 -n '__f5_uses_subcommand push' -s h -l help -d 'Show help'

# ── redact ─────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand redact' -l keep-ips -d 'Do not remap any IPs'
complete -c f5 -n '__f5_uses_subcommand redact' -l map-file -r -F -d 'Write the redaction map sidecar here'
complete -c f5 -n '__f5_uses_subcommand redact' -l remap-private -d 'Remap RFC1918 addresses too'
complete -c f5 -n '__f5_uses_subcommand redact' -l seed -r -d 'Deterministic remap seed'
complete -c f5 -n '__f5_uses_subcommand redact' -l shuffle -d 'Randomly permute target IP allocations'
complete -c f5 -n '__f5_uses_subcommand redact' -l source-cidr -r -d 'CIDR(s) to remap (repeatable)'
complete -c f5 -n '__f5_uses_subcommand redact' -l target-cidr -r -d 'CIDR pool to allocate replacements from'
complete -c f5 -n '__f5_uses_subcommand redact' -s h -l help -d 'Show help'

# ── rename ─────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand rename' -l in-place -d 'Rewrite each input file in place'
complete -c f5 -n '__f5_uses_subcommand rename' -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand rename' -l write -d 'Persist changes (default is preview)'
complete -c f5 -n '__f5_uses_subcommand rename' -s h -l help -d 'Show help'

# ── split ──────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand split' -s h -l help -d 'Show help'

# ── stats ──────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand stats' -l json -d 'Emit stats as JSON'
complete -c f5 -n '__f5_uses_subcommand stats' -s o -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand stats' -l top -r -d 'Show top N referenced objects'
complete -c f5 -n '__f5_uses_subcommand stats' -s h -l help -d 'Show help'

# ── tmsh ───────────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand tmsh' -l include -r -d 'Include only matching object kinds'
complete -c f5 -n '__f5_uses_subcommand tmsh' -l modify -d 'Emit `modify` commands for existing objects'
complete -c f5 -n '__f5_uses_subcommand tmsh' -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand tmsh' -s h -l help -d 'Show help'

# ── unredact ───────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand unredact' -s h -l help -d 'Show help'

# ── validate ───────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand validate' -l category -r -d 'Filter to a lint-rule category'
complete -c f5 -n '__f5_uses_subcommand validate' -l format -r -a 'text json sarif' -d 'Output format'
complete -c f5 -n '__f5_uses_subcommand validate' -s o -l output -r -F -d 'Output path'
complete -c f5 -n '__f5_uses_subcommand validate' -l severity -r -a 'error warning info' -d 'Minimum severity to report'
complete -c f5 -n '__f5_uses_subcommand validate' -s h -l help -d 'Show help'

# ── completion ─────────────────────────────────────────────────────────
complete -c f5 -n '__f5_uses_subcommand completion' -a 'bash fish zsh' -d 'Shell'
complete -c f5 -n '__f5_uses_subcommand completion' -l hint -d 'Print install instructions to stderr'

# ── irule sub-actions ──────────────────────────────────────────────────
function __f5_irule_no_action
    set -l cmd (commandline -opc)
    test (count $cmd) -eq 2
end

function __f5_irule_uses_action
    set -l cmd (commandline -opc)
    if test (count $cmd) -ge 3
        set -l action $cmd[3]
        switch $action
            case eventorder
                set action event-order
            case eventinfo
                set action event-info
            case fmt
                set action format
            case min
                set action minify
        end
        if contains -- $action $argv
            return 0
        end
    end
    return 1
end

complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a event-order -d 'Show iRules events in canonical firing order'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a event-info -d 'Look up iRules event metadata and valid commands'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a lint -d 'Apply iRule-only lint rules'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a trace -d 'Static event-flow trace from a starting event'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a extract -d 'Split a config into per-rule .tcl files'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a format -d 'Pretty-print iRule source'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a minify -d 'Strip whitespace/comments from iRule source'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a context -d 'Bundle each iRule with the BIG-IP objects it references'

# Shared irule sub-action input flags.
set -l __f5_irule_input_actions event-order lint trace format minify context
complete -c f5 -n "__f5_uses_subcommand irule; and __f5_irule_uses_action $__f5_irule_input_actions" \
    -l source -r -d 'Inline iRules source text (repeatable)'
complete -c f5 -n "__f5_uses_subcommand irule; and __f5_irule_uses_action $__f5_irule_input_actions" \
    -l dialect -x -a 'f5-irules' -d 'Dialect profile'
complete -c f5 -n "__f5_uses_subcommand irule; and __f5_irule_uses_action $__f5_irule_input_actions" \
    -s o -l output -r -F -d 'Output path (- for stdout)'

# event-order
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-order' \
    -l json -d 'Emit event ordering as JSON'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-order' \
    -l package-path -r -F -d 'Add a directory to the package search path'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-order' \
    -l no-recursive -d 'Do not recurse into directory inputs'

# event-info
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-info' \
    -l json -d 'Emit event metadata as JSON'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-info' \
    -s o -l output -r -F -d 'Output path'

# lint
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action lint' \
    -l json -d 'Emit lint results as JSON'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action lint' \
    -l severity -x -a 'error warning info' -d 'Filter to a severity level'

# trace
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action trace' \
    -l json -d 'Emit trace as JSON'

# minify
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action minify' \
    -l compact -d 'Compact variable and proc names'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action minify' \
    -l aggressive -d 'Run all optimisations + name compaction'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action minify' \
    -l isolated -d 'Compact global-scope names too'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action minify' \
    -l symbol-map -r -F -d 'Write symbol map to FILE'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action minify' \
    -l json -d 'Emit minify report as JSON'

# context
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action context' \
    -l rule -r -d 'Limit output to rules with this full path (repeatable)'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action context' \
    -l no-transitive -d 'Skip the one-hop transitive expansion'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action context' \
    -l json -d 'Emit each bundle as a JSON object'
