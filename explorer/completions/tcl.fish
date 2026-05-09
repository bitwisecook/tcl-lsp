# fish completion for the `tcl` CLI.
#
# Install:
#   mkdir -p ~/.config/fish/completions
#   tcl completion fish > ~/.config/fish/completions/tcl.fish
#
# Then start a new shell.
#
# iRules-specific verbs (event-order, event-info) live on the `f5` CLI
# under `f5 irule <subverb>` — install `f5 completion fish` for those.

function __tcl_uses_subcommand
    set -l cmd (commandline -opc)
    if test (count $cmd) -ge 2
        if contains -- $cmd[2] $argv
            return 0
        end
    end
    return 1
end

function __tcl_no_subcommand
    set -l cmd (commandline -opc)
    test (count $cmd) -lt 2
end

function __tcl_pkg_no_action
    set -l cmd (commandline -opc)
    test (count $cmd) -eq 2
end

function __tcl_pkg_uses_action
    set -l cmd (commandline -opc)
    if test (count $cmd) -ge 3
        if contains -- $cmd[3] $argv
            return 0
        end
    end
    return 1
end

set -l __tcl_dialects cadence-eda-tcl expect f5-bigip f5-iapps f5-irules \
    f5-tmsh intel-quartus-eda-tcl mentor-eda-tcl synopsys-eda-tcl \
    tcl8.4 tcl8.5 tcl8.6 tcl9.0 xilinx-eda-tcl

# Apply identical completions to every binary name we ship.
for prog in tcl tcl.pyz
    complete -c $prog -f

    # Top-level verbs.
    complete -c $prog -n __tcl_no_subcommand -a opt -d 'Optimise source and emit rewritten Tcl'
    complete -c $prog -n __tcl_no_subcommand -a optimise -d 'Alias for opt'
    complete -c $prog -n __tcl_no_subcommand -a optimize -d 'Alias for opt'
    complete -c $prog -n __tcl_no_subcommand -a diag -d 'Run diagnostics across resolved inputs'
    complete -c $prog -n __tcl_no_subcommand -a diagnostics -d 'Alias for diag'
    complete -c $prog -n __tcl_no_subcommand -a lint -d 'Run lint diagnostics'
    complete -c $prog -n __tcl_no_subcommand -a validate -d 'Validate (errors only)'
    complete -c $prog -n __tcl_no_subcommand -a format -d 'Format source'
    complete -c $prog -n __tcl_no_subcommand -a fmt -d 'Alias for format'
    complete -c $prog -n __tcl_no_subcommand -a minify -d 'Minify source'
    complete -c $prog -n __tcl_no_subcommand -a min -d 'Alias for minify'
    complete -c $prog -n __tcl_no_subcommand -a unminify-error -d 'Translate minified error messages'
    complete -c $prog -n __tcl_no_subcommand -a umerr -d 'Alias for unminify-error'
    complete -c $prog -n __tcl_no_subcommand -a symbols -d 'Emit symbol definitions'
    complete -c $prog -n __tcl_no_subcommand -a syms -d 'Alias for symbols'
    complete -c $prog -n __tcl_no_subcommand -a diagram -d 'Extract control-flow diagram'
    complete -c $prog -n __tcl_no_subcommand -a callgraph -d 'Build call graph'
    complete -c $prog -n __tcl_no_subcommand -a call-graph -d 'Alias for callgraph'
    complete -c $prog -n __tcl_no_subcommand -a symbolgraph -d 'Build symbol relationship graph'
    complete -c $prog -n __tcl_no_subcommand -a symbol-graph -d 'Alias for symbolgraph'
    complete -c $prog -n __tcl_no_subcommand -a dataflow -d 'Build taint/effect data-flow graph'
    complete -c $prog -n __tcl_no_subcommand -a command-info -d 'Look up command registry metadata'
    complete -c $prog -n __tcl_no_subcommand -a commandinfo -d 'Alias for command-info'
    complete -c $prog -n __tcl_no_subcommand -a cmd-info -d 'Alias for command-info'
    complete -c $prog -n __tcl_no_subcommand -a convert -d 'Detect legacy modernisation patterns'
    complete -c $prog -n __tcl_no_subcommand -a dis -d 'Bytecode disassembly'
    complete -c $prog -n __tcl_no_subcommand -a asm -d 'Alias for dis'
    complete -c $prog -n __tcl_no_subcommand -a disassemble -d 'Alias for dis'
    complete -c $prog -n __tcl_no_subcommand -a compwasm -d 'Compile to WebAssembly binary'
    complete -c $prog -n __tcl_no_subcommand -a wasm -d 'Alias for compwasm'
    complete -c $prog -n __tcl_no_subcommand -a highlight -d 'Emit syntax-highlighted source'
    complete -c $prog -n __tcl_no_subcommand -a hl -d 'Alias for highlight'
    complete -c $prog -n __tcl_no_subcommand -a diff -d 'Diff two sources via AST/IR/CFG'
    complete -c $prog -n __tcl_no_subcommand -a explore -d 'Run compiler-explorer views'
    complete -c $prog -n __tcl_no_subcommand -a help -d 'Search bundled KCS help docs'
    complete -c $prog -n __tcl_no_subcommand -a docs -d 'Alias for help'
    complete -c $prog -n __tcl_no_subcommand -a completion -d 'Print shell completion script'
    complete -c $prog -n __tcl_no_subcommand -a pkg -d 'Tcl package management'
    complete -c $prog -n __tcl_no_subcommand -a venv -d 'Tcl virtual environments'
    complete -c $prog -n __tcl_no_subcommand -a docker -d 'Generate Dockerfiles for Tcl projects'

    # Top-level help / version.
    complete -c $prog -n __tcl_no_subcommand -s h -l help -d 'Show brief help and exit'
    complete -c $prog -n __tcl_no_subcommand -l help-all -d 'Show full help for every verb'
    complete -c $prog -n __tcl_no_subcommand -l version -d 'Print build version and exit'

    # ── Common input flags shared by most verbs ────────────────────────
    set -l __tcl_input_verbs opt optimise optimize diag diagnostics lint \
        validate format fmt minify min symbols syms diagram callgraph \
        call-graph symbolgraph symbol-graph dataflow convert dis asm \
        disassemble compwasm wasm highlight hl diff explore

    complete -c $prog -n "__tcl_uses_subcommand $__tcl_input_verbs" -l source -r -d 'Inline Tcl source text (repeatable)'
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_input_verbs" -l package-path -r -F -d 'Add a directory to the package search path'
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_input_verbs" -l no-recursive -d 'Do not recurse into directory inputs'
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_input_verbs" -l dialect -x -a "$__tcl_dialects" -d 'Dialect profile'
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_input_verbs" -s o -l output -r -F -d 'Output path (- for stdout)'
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_input_verbs" -l colour -d 'Force ANSI syntax highlighting'
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_input_verbs" -l color -d 'Force ANSI syntax highlighting'
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_input_verbs" -l no-colour -d 'Disable ANSI syntax highlighting'
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_input_verbs" -l no-color -d 'Disable ANSI syntax highlighting'
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_input_verbs" -l tabs -x -d 'Tab expansion width (default: 4)'

    # ── opt flags ──────────────────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand opt optimise optimize' -l profile \
        -x -a 'off readability standard full aggressive' -d 'Optimisation profile'
    complete -c $prog -n '__tcl_uses_subcommand opt optimise optimize' -l disable -x -d 'Suppress optimisation codes'
    complete -c $prog -n '__tcl_uses_subcommand opt optimise optimize' -l enable -x -d 'Re-enable optimisation codes'

    # ── format flags ───────────────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand format fmt' -l indent-size -x -d 'Spaces per indent level'
    complete -c $prog -n '__tcl_uses_subcommand format fmt' -l indent-style -x -a 'spaces tabs' -d 'Indent style'
    complete -c $prog -n '__tcl_uses_subcommand format fmt' -l max-line-length -x -d 'Hard line length limit'
    complete -c $prog -n '__tcl_uses_subcommand format fmt' -l goal-line-length -x -d 'Soft line length target'
    complete -c $prog -n '__tcl_uses_subcommand format fmt' -l expand-bodies -d 'Expand compact single-line bodies'
    complete -c $prog -n '__tcl_uses_subcommand format fmt' -l no-semicolons -d 'Replace semicolons with newlines'
    complete -c $prog -n '__tcl_uses_subcommand format fmt' -l keep-semicolons -d 'Keep semicolons as-is'

    # ── minify flags ───────────────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand minify min' -l compact -d 'Compact variable and proc names'
    complete -c $prog -n '__tcl_uses_subcommand minify min' -l symbol-map -r -F -d 'Write symbol map to FILE'
    complete -c $prog -n '__tcl_uses_subcommand minify min' -l aggressive -d 'Run all optimisations + name compaction'
    complete -c $prog -n '__tcl_uses_subcommand minify min' -l isolated -d 'Compact global-scope names too'

    # ── unminify-error flags ───────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand unminify-error umerr' -l symbol-map -r -F -d 'Symbol map file'
    complete -c $prog -n '__tcl_uses_subcommand unminify-error umerr' -s e -l error -r -d 'Error text to translate'
    complete -c $prog -n '__tcl_uses_subcommand unminify-error umerr' -l error-file -r -F -d 'Error message file'
    complete -c $prog -n '__tcl_uses_subcommand unminify-error umerr' -l minified -r -F -d 'Minified source file'
    complete -c $prog -n '__tcl_uses_subcommand unminify-error umerr' -l original -r -F -d 'Original source file'
    complete -c $prog -n '__tcl_uses_subcommand unminify-error umerr' -s o -l output -r -F -d 'Output path'

    # ── diag/lint/validate/symbols/etc — JSON + diagnostic toggles ─────
    set -l __tcl_json_verbs diag diagnostics lint validate symbols syms \
        diagram callgraph call-graph symbolgraph symbol-graph dataflow \
        convert diff command-info commandinfo cmd-info help docs
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_json_verbs" -l json -d 'Emit results as JSON'

    set -l __tcl_diag_verbs diag diagnostics lint validate
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_diag_verbs" -l disable -x -d 'Suppress diagnostic codes'
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_diag_verbs" -l enable -x -d 'Re-enable diagnostic codes'

    # ── highlight ──────────────────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand highlight hl' -l format -x -a 'ansi html' -d 'Output format'

    # ── diff ───────────────────────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand diff' -l show -x -a 'ast ir cfg' -d 'Layers to compare (comma-separated)'

    # ── explore ────────────────────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand explore' -l show -x \
        -a 'ir cfg ssa interproc types opt gvn shimmer taint irules callouts asm wasm all compiler optimiser' \
        -d 'Views to render (comma-separated)'

    # ── lookup verbs (command-info, help) ──────────────────────────────
    set -l __tcl_lookup_verbs command-info commandinfo cmd-info help docs
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_lookup_verbs" -l dialect -x -a "$__tcl_dialects" -d 'Dialect profile'
    complete -c $prog -n '__tcl_uses_subcommand help docs' -l limit -x -d 'Maximum number of results'

    # ── completion ─────────────────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand completion' -a 'bash fish zsh' -d 'Shell'
    complete -c $prog -n '__tcl_uses_subcommand completion' -l hint -d 'Print install instructions to stderr'

    # ── pkg group ──────────────────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a init -d 'Create a new tclpkg.tcl manifest'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a install -d 'Resolve, fetch, and materialise packages'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a list -d 'List installed packages'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a tree -d 'Show dependency tree'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a verify -d 'Verify integrity hashes'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a info -d 'Show details for a package'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a add -d 'Add a dependency to the manifest'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a remove -d 'Remove a dependency from the manifest'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a update -d 'Bump dependency minimums'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a sync -d 'Lock-driven install'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a outdated -d 'Show outdated packages'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a why -d 'Explain why a package is in the graph'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a vendor -d 'Copy packages from cache into project'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a run -d 'Run the manifest entry point'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a freeze -d 'Write resolved versions back to manifest'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_no_action' -a search -d 'Search the package registry'
    complete -c $prog -n '__tcl_uses_subcommand pkg' -l json -d 'Emit JSON output'
    complete -c $prog -n '__tcl_uses_subcommand pkg' -l offline -d 'Do not touch the network'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_uses_action install' -l no-dev -d 'Skip dev-require packages'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_uses_action install' -l frozen -d 'Refuse to change lockfile'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_uses_action add' -l dev -d 'Add as dev-require'
    complete -c $prog -n '__tcl_uses_subcommand pkg; and __tcl_pkg_uses_action init' -l force -d 'Overwrite existing manifest'

    # ── venv group ─────────────────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand venv; and __tcl_pkg_no_action' -a create -d 'Create a new virtual environment'
    complete -c $prog -n '__tcl_uses_subcommand venv; and __tcl_pkg_no_action' -a delete -d 'Remove a virtual environment'
    complete -c $prog -n '__tcl_uses_subcommand venv; and __tcl_pkg_no_action' -a info -d 'Show venv details'
    complete -c $prog -n '__tcl_uses_subcommand venv; and __tcl_pkg_no_action' -a activate -d 'Print activation script'
    complete -c $prog -n '__tcl_uses_subcommand venv; and __tcl_pkg_no_action' -a deactivate -d 'Print deactivation script'
    complete -c $prog -n '__tcl_uses_subcommand venv; and __tcl_pkg_no_action' -a list -d 'List discoverable venvs'
    complete -c $prog -n '__tcl_uses_subcommand venv; and __tcl_pkg_no_action' -a update -d 'Re-link a venv to a newer tclsh'
    complete -c $prog -n '__tcl_uses_subcommand venv; and __tcl_pkg_no_action' -a run -d 'Run a command inside a venv'
    complete -c $prog -n '__tcl_uses_subcommand venv' -l json -d 'Emit JSON output'
    complete -c $prog -n '__tcl_uses_subcommand venv' -l force -d 'Force the operation'

    # ── docker group ───────────────────────────────────────────────────
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_no_action' -a create -d 'Generate a Dockerfile'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_no_action' -a recipe -d 'Show Tcl install recipe'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_no_action' -a info -d 'Show base-image families and Tcl versions'
    complete -c $prog -n '__tcl_uses_subcommand docker' -l json -d 'Emit JSON output'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create recipe' -l tcl-version -x -a '8.4 8.5 8.6 9.0' -d 'Tcl version to install'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -s o -l output -r -F -d 'Output Dockerfile path'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -l workdir -x -d 'Container WORKDIR'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -l entrypoint -r -F -d 'Tcl script to run as CMD'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -l venv -d 'Create a Tcl venv inside the container'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -l no-copy -d 'Skip COPY . . layer'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -l no-packages -d 'Skip tclpkg sync step'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -l extra-package -x -d 'Additional OS package (repeatable)'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -l label -x -d 'Docker LABEL key=value'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -l env -x -d 'Docker ENV key=value'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -l cli-version -x -d 'tcl CLI zipapp version'
    complete -c $prog -n '__tcl_uses_subcommand docker; and __tcl_pkg_uses_action create' -l force -d 'Overwrite existing Dockerfile'

    # ── Positional file completion for verbs that take Tcl/iRule files ─
    set -l __tcl_file_verbs opt optimise optimize diag diagnostics lint \
        validate format fmt minify min symbols syms diagram callgraph \
        call-graph symbolgraph symbol-graph dataflow convert dis asm \
        disassemble compwasm wasm highlight hl diff explore
    complete -c $prog -n "__tcl_uses_subcommand $__tcl_file_verbs" -F -k \
        -a "(__fish_complete_suffix .tcl; __fish_complete_suffix .tk; __fish_complete_suffix .itcl; __fish_complete_suffix .tm; __fish_complete_suffix .irul; __fish_complete_suffix .irule; __fish_complete_suffix .iapp; __fish_complete_suffix .iappimpl)"
end
