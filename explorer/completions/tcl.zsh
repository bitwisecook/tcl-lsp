#compdef tcl tcl.pyz
#
# zsh completion for the `tcl` CLI.
#
# Install (per-user):
#   mkdir -p "${ZDOTDIR:-$HOME}/.zsh/completions"
#   tcl completion zsh > "${ZDOTDIR:-$HOME}/.zsh/completions/_tcl"
#
#   # Then add to .zshrc, before `compinit`:
#   #   fpath=("${ZDOTDIR:-$HOME}/.zsh/completions" $fpath)
#
# Or, with oh-my-zsh:
#   tcl completion zsh > ~/.oh-my-zsh/completions/_tcl
#
# Or system-wide on systems where ``$fpath`` already covers it
# (e.g. /usr/share/zsh/site-functions):
#   sudo tcl completion zsh > /usr/share/zsh/site-functions/_tcl
#
# iRules-specific verbs (event-order, event-info) live on the `f5` CLI
# under `f5 irule <subverb>` — install `f5 completion zsh` for those.

_tcl() {
    local context curcontext="$curcontext" state line
    typeset -A opt_args

    local -a dialects opt_profiles indent_styles highlight_formats \
        diff_layers explore_views completion_shells docker_tcl_versions

    dialects=(cadence-eda-tcl expect f5-bigip f5-iapps f5-irules f5-tmsh \
        intel-quartus-eda-tcl mentor-eda-tcl synopsys-eda-tcl \
        tcl8.4 tcl8.5 tcl8.6 tcl9.0 xilinx-eda-tcl)
    opt_profiles=(off readability standard full aggressive)
    indent_styles=(spaces tabs)
    highlight_formats=(ansi html)
    diff_layers=(ast ir cfg)
    explore_views=(ir cfg ssa interproc types opt gvn shimmer taint \
        irules callouts asm wasm all compiler optimiser)
    completion_shells=(bash fish zsh)
    docker_tcl_versions=(8.4 8.5 8.6 9.0)

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
                'opt:Optimise source and emit rewritten Tcl'
                'optimise:Alias for opt'
                'optimize:Alias for opt'
                'diag:Run diagnostics across all resolved inputs'
                'diagnostics:Alias for diag'
                'lint:Run lint diagnostics across all resolved inputs'
                'validate:Validate source (error-level diagnostics only)'
                'format:Format source and emit rewritten Tcl'
                'fmt:Alias for format'
                'minify:Minify source by stripping comments and joining commands'
                'min:Alias for minify'
                'unminify-error:Translate minified error messages back to original names'
                'umerr:Alias for unminify-error'
                'symbols:Emit symbol definitions for the resolved input'
                'syms:Alias for symbols'
                'diagram:Extract control-flow diagram data from compiler IR'
                'callgraph:Build call graph data for resolved source'
                'call-graph:Alias for callgraph'
                'symbolgraph:Build symbol relationship graph data'
                'symbol-graph:Alias for symbolgraph'
                'dataflow:Build taint/effect data-flow graph data'
                'command-info:Look up command registry metadata'
                'commandinfo:Alias for command-info'
                'cmd-info:Alias for command-info'
                'convert:Detect legacy patterns eligible for modernisation'
                'dis:Compile and emit bytecode disassembly'
                'asm:Alias for dis'
                'disassemble:Alias for dis'
                'compwasm:Compile source to WebAssembly binary'
                'wasm:Alias for compwasm'
                'highlight:Emit syntax-highlighted source output'
                'hl:Alias for highlight'
                'diff:Diff two sources using AST/IR/CFG representations'
                'explore:Run compiler-explorer views on aggregated input'
                'help:Search bundled KCS help docs from the SQLite index'
                'docs:Alias for help'
                'completion:Print shell completion script for bash/fish/zsh'
                'pkg:Tcl package management (init/install/list/...)'
                'venv:Tcl virtual environments (create/activate/...)'
                'docker:Generate Dockerfiles for Tcl projects'
            )
            _describe -t verbs 'verb' verbs
            ;;
        args)
            local common_args=(
                '*--source[Inline Tcl source text (repeatable)]:source text'
                '*--package-path[Directory to scan for pkgIndex.tcl (repeatable)]:dir:_files -/'
                '--no-recursive[Do not recurse into directory inputs]'
                "--dialect[Dialect profile]:dialect:($dialects)"
                '(-o --output)'{-o,--output}'[Output path (- for stdout)]:output file:_files'
                '(--colour --color)'{--colour,--color}'[Force ANSI syntax highlighting]'
                '(--no-colour --no-color)'{--no-colour,--no-color}'[Disable ANSI highlighting]'
                '--tabs[Tab expansion width]:N'
            )
            local toggle_args=(
                '--disable[Suppress codes (comma-separated)]:CODE'
                '--enable[Re-enable codes disabled in config (comma-separated)]:CODE'
            )
            local format_args=(
                '--indent-size[Spaces per indent level]:N'
                "--indent-style[Indent style]:style:($indent_styles)"
                '--max-line-length[Hard line length limit]:N'
                '--goal-line-length[Soft line length target]:N'
                '--expand-bodies[Expand compact single-line bodies]'
                '--no-semicolons[Replace semicolons with newlines]'
                '--keep-semicolons[Keep semicolons as-is]'
            )

            case "$line[1]" in
                opt|optimise|optimize)
                    _arguments \
                        $common_args \
                        $toggle_args \
                        "--profile[Optimisation profile]:profile:($opt_profiles)" \
                        '*:input:_files'
                    ;;
                format|fmt)
                    _arguments $common_args $format_args '*:input:_files'
                    ;;
                minify|min)
                    _arguments $common_args \
                        '--compact[Compact variable and proc names]' \
                        '--symbol-map[Write symbol map to FILE]:symbol-map file:_files' \
                        '--aggressive[Run all optimiser passes plus name compaction]' \
                        '--isolated[Compact global-scope variable names too]' \
                        '*:input:_files'
                    ;;
                unminify-error|umerr)
                    _arguments \
                        '--symbol-map[Path to symbol map produced by minify]:symbol-map file:_files' \
                        '(-e --error)'{-e,--error}'[Error message text to translate]:error text' \
                        '--error-file[File of error messages (- for stdin)]:error file:_files' \
                        '--minified[Minified source file (line remapping)]:minified file:_files' \
                        '--original[Original source file (line remapping)]:original file:_files' \
                        '(-o --output)'{-o,--output}'[Output path]:output file:_files'
                    ;;
                diag|diagnostics|lint|validate)
                    _arguments \
                        $common_args \
                        $toggle_args \
                        '--json[Emit diagnostics as JSON]' \
                        '*:input:_files'
                    ;;
                symbols|syms|diagram|callgraph|call-graph|symbolgraph|symbol-graph|dataflow|convert)
                    _arguments \
                        $common_args \
                        '--json[Emit results as JSON]' \
                        '*:input:_files'
                    ;;
                dis|asm|disassemble)
                    _arguments $common_args '*:input:_files'
                    ;;
                compwasm|wasm)
                    _arguments \
                        $common_args \
                        '--wat-output[Write WAT sidecar to FILE]:wat file:_files' \
                        '*:input:_files'
                    ;;
                highlight|hl)
                    _arguments \
                        $common_args \
                        "--format[Output format]:format:($highlight_formats)" \
                        '*:input:_files'
                    ;;
                diff)
                    _arguments \
                        $common_args \
                        "--show[Layers to compare (comma-separated)]:layers:_values -s , 'layer' $diff_layers" \
                        '--json[Emit diff as JSON]' \
                        '*:input:_files'
                    ;;
                explore)
                    _arguments \
                        $common_args \
                        "--show[Views to render (comma-separated)]:views:_values -s , 'view' $explore_views" \
                        '*:input:_files'
                    ;;
                command-info|commandinfo|cmd-info)
                    _arguments \
                        "--dialect[Dialect profile]:dialect:($dialects)" \
                        '--json[Emit results as JSON]' \
                        '*:query'
                    ;;
                help|docs)
                    _arguments \
                        "--dialect[Dialect profile]:dialect:($dialects)" \
                        '--json[Emit search results as JSON]' \
                        '--limit[Maximum number of results]:N' \
                        '*:query'
                    ;;
                completion)
                    _arguments \
                        '--hint[Print install instructions to stderr]' \
                        "1:shell:($completion_shells)"
                    ;;
                pkg)
                    if (( CURRENT == 2 )); then
                        local -a pkg_actions
                        pkg_actions=(
                            'init:Create a new tclpkg.tcl manifest'
                            'install:Resolve, fetch, and materialise packages'
                            'list:List installed packages'
                            'tree:Show dependency tree'
                            'verify:Verify integrity hashes'
                            'info:Show details for a package'
                            'add:Add a dependency to the manifest'
                            'remove:Remove a dependency from the manifest'
                            'update:Bump dependency minimums'
                            'sync:Lock-driven install (alias for install --frozen)'
                            'outdated:Show packages with newer versions available'
                            'why:Explain why a package is in the dependency graph'
                            'vendor:Copy packages from cache into the project tree'
                            'run:Run the manifest entry point via tclsh'
                            'freeze:Write resolved versions back to the manifest'
                            'search:Search the package registry'
                        )
                        _describe -t pkg_actions 'action' pkg_actions
                    else
                        _arguments \
                            '--json[Emit JSON output]' \
                            '--offline[Do not touch the network]' \
                            '--frozen[Refuse to change lockfile]' \
                            '--no-dev[Skip dev-require packages]' \
                            '--force[Overwrite existing artefacts]' \
                            '--dev[Add as dev-require dependency]' \
                            '*:arg:_files'
                    fi
                    ;;
                venv)
                    if (( CURRENT == 2 )); then
                        local -a venv_actions
                        venv_actions=(
                            'create:Create a new virtual environment'
                            'delete:Remove a virtual environment'
                            'info:Show virtual environment details'
                            'activate:Print activation script to stdout'
                            'deactivate:Print deactivation script to stdout'
                            'list:List discoverable virtual environments'
                            'update:Re-link a venv to a newer tclsh'
                            'run:Run a command inside a venv'
                        )
                        _describe -t venv_actions 'action' venv_actions
                    else
                        _arguments \
                            '--json[Emit JSON output]' \
                            '--force[Force the operation]' \
                            '*:arg:_files'
                    fi
                    ;;
                docker)
                    if (( CURRENT == 2 )); then
                        local -a docker_actions
                        docker_actions=(
                            'create:Generate a Dockerfile for a Tcl project'
                            'recipe:Show the Tcl install recipe for a base image'
                            'info:Show available base-image families and Tcl versions'
                        )
                        _describe -t docker_actions 'action' docker_actions
                    else
                        _arguments \
                            "--tcl-version[Tcl version to install]:version:($docker_tcl_versions)" \
                            '(-o --output)'{-o,--output}'[Output Dockerfile path]:output file:_files' \
                            '--workdir[Container WORKDIR]:path' \
                            '--entrypoint[Tcl script to run as CMD]:script:_files' \
                            '--venv[Create a Tcl virtual environment in the container]' \
                            '--no-copy[Skip COPY . . layer]' \
                            '--no-packages[Skip tclpkg sync step]' \
                            '*--extra-package[Additional OS package (repeatable)]:package' \
                            '*--label[Docker LABEL key=value (repeatable)]:label' \
                            '*--env[Docker ENV key=value (repeatable)]:env' \
                            '--cli-version[tcl CLI zipapp version to download]:version' \
                            '--force[Overwrite existing Dockerfile]' \
                            '--json[Emit JSON output]' \
                            '*:image:'
                    fi
                    ;;
            esac
            ;;
    esac
}

_tcl "$@"
