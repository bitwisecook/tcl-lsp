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
        if contains -- $cmd[2] $argv
            return 0
        end
    end
    return 1
end

function __f5_no_subcommand
    set -l cmd (commandline -opc)
    test (count $cmd) -lt 2
end

# Top-level verbs.
complete -c f5 -f
complete -c f5 -n __f5_no_subcommand -a cleanup -d 'Generate tmsh delete commands for unreferenced objects'
complete -c f5 -n __f5_no_subcommand -a clean -d 'Alias for cleanup'
complete -c f5 -n __f5_no_subcommand -a completion -d 'Print shell completion script'
complete -c f5 -n __f5_no_subcommand -a grep -d 'List every BIG-IP object related to a given object path or regex'
complete -c f5 -n __f5_no_subcommand -a related -d 'Alias for grep'
complete -c f5 -n __f5_no_subcommand -a irule -d 'iRules-specific analysis (event-order, event-info, ...)'

# Top-level help / version.
complete -c f5 -n __f5_no_subcommand -s h -l help -d 'Show brief help and exit'
complete -c f5 -n __f5_no_subcommand -l help-all -d 'Show full help for every verb and exit'
complete -c f5 -n __f5_no_subcommand -l version -d 'Print build version and exit'

# `cleanup` / `clean` flags.
complete -c f5 -n '__f5_uses_subcommand cleanup clean' -l json -d 'Emit cleanup report as JSON'
complete -c f5 -n '__f5_uses_subcommand cleanup clean' -l keep -r -d 'Object full-path or partition prefix to retain'
complete -c f5 -n '__f5_uses_subcommand cleanup clean' -l no-keep-common -d 'Do not auto-keep /Common/*'
complete -c f5 -n '__f5_uses_subcommand cleanup clean' -s o -l output -r -F -d 'Write output here (default: stdout)'
complete -c f5 -n '__f5_uses_subcommand cleanup clean' -s h -l help -d 'Show help'

# Positional arguments for cleanup — restrict to .conf / .scf files.
complete -c f5 -n '__f5_uses_subcommand cleanup clean' -F -k -a "(__fish_complete_suffix .conf; __fish_complete_suffix .scf)"

# `grep` / `related` flags.
complete -c f5 -n '__f5_uses_subcommand grep related' -s e -l regex -d 'Treat PATTERN as a Python regular expression'
complete -c f5 -n '__f5_uses_subcommand grep related' -l direction -r -a 'forward reverse both' -d 'Which edges to traverse'
complete -c f5 -n '__f5_uses_subcommand grep related' -l max-depth -r -d 'Stop BFS after N hops'
complete -c f5 -n '__f5_uses_subcommand grep related' -l max-nodes -r -d 'Cap result at N objects'
complete -c f5 -n '__f5_uses_subcommand grep related' -l full -d 'Print each object full body'
complete -c f5 -n '__f5_uses_subcommand grep related' -l json -d 'Emit grep report as JSON'
complete -c f5 -n '__f5_uses_subcommand grep related' -s o -l output -r -F -d 'Write output here (default: stdout)'
complete -c f5 -n '__f5_uses_subcommand grep related' -s h -l help -d 'Show help'

# Positional config files for grep — restrict to .conf / .scf files.
complete -c f5 -n '__f5_uses_subcommand grep related' -F -k -a "(__fish_complete_suffix .conf; __fish_complete_suffix .scf)"

# `completion` shell argument.
complete -c f5 -n '__f5_uses_subcommand completion' -a 'bash fish zsh' -d 'Shell'

# `irule` sub-actions.
function __f5_irule_no_action
    set -l cmd (commandline -opc)
    test (count $cmd) -eq 2
end

function __f5_irule_uses_action
    set -l cmd (commandline -opc)
    if test (count $cmd) -ge 3
        if contains -- $cmd[3] $argv
            return 0
        end
    end
    return 1
end

complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a event-order -d 'Show iRules events in canonical firing order'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a eventorder -d 'Alias for event-order'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a event-info -d 'Look up iRules event metadata and valid commands'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_no_action' \
    -a eventinfo -d 'Alias for event-info'

# `f5 irule event-order` flags.
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-order eventorder' \
    -l json -d 'Emit event ordering as JSON'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-order eventorder' \
    -l source -r -d 'Inline iRules source text (repeatable)'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-order eventorder' \
    -l package-path -r -F -d 'Add a directory to the package search path'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-order eventorder' \
    -l no-recursive -d 'Do not recurse into directory inputs'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-order eventorder' \
    -l dialect -x -a 'f5-irules' -d 'Dialect profile'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-order eventorder' \
    -s o -l output -r -F -d 'Output path (- for stdout)'

# `f5 irule event-info` flags.
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-info eventinfo' \
    -l json -d 'Emit event metadata as JSON'
complete -c f5 -n '__f5_uses_subcommand irule; and __f5_irule_uses_action event-info eventinfo' \
    -s o -l output -r -F -d 'Output path'
