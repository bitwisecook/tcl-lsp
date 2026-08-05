// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `clap` derive definitions for the `tcl` CLI command tree.
//!
//! Verb names map to the kebab-cased enum-variant names; aliases map to
//! `visible_aliases`.
//!
//! The common input/output/dialect surface every verb shares is modelled
//! precisely (it is the bulk of the CLI contract), with verb-specific flags
//! added alongside. New flags slot into the existing structs without reshaping
//! the tree.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// The WebAssembly backend `tcl compwasm` emits (`RUST_ISSUE_008`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum WasmBackend {
    /// The self-contained bytecode-VM runner (`vm.wasm`): the VM + compiler
    /// statically linked, running any script (coroutines included) with no host
    /// imports and no WASI. The primary target.
    #[default]
    Vm,
    /// The legacy eval-fallback emitter: a bare module importing the tree-walker
    /// runtime C-ABI (`tcl_*`) over a shared `runtime.wasm`. Kept for its WAT
    /// disassembly view.
    TreeWalker,
}

/// Unified Tcl toolchain CLI.
#[derive(Debug, Parser)]
#[command(
    name = "tcl",
    // Not clap's default (CARGO_PKG_VERSION): the workspace manifest carries
    // 0.1.0 and is never bumped, because releases are tag-only.
    version = tcl_version::VERSION,
    about = "Unified Tcl toolchain CLI.",
    disable_help_subcommand = true,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Input-resolution flags shared by most verbs.
#[derive(Debug, Args)]
pub struct InputArgs {
    /// Input files, directories, or package names.
    #[arg(value_name = "INPUT")]
    pub inputs: Vec<PathBuf>,

    /// Append inline Tcl source to the inputs (repeatable).
    #[arg(long = "source", value_name = "CODE")]
    pub source: Vec<String>,

    /// Additional directory to search for packages (repeatable).
    #[arg(long = "package-path", value_name = "DIR")]
    pub package_path: Vec<PathBuf>,

    /// Dialect profile for parsing and registry lookups. Defaults to
    /// auto-detection (a `# tcl-dialect:` directive, shebang, content
    /// signals, then the file extension), falling back to tcl8.6.
    ///
    /// Resolved per document by the diagnostics verbs (`diag` / `lint` /
    /// `validate` / `minimize`) via
    /// [`tcl_cli_support::InputDocument::effective_dialect`], and once for
    /// the whole invocation by the verbs that combine their inputs into one
    /// source (transforms, graphs, explore, compile) via
    /// [`tcl_cli_support::combined_effective_dialect`].
    #[arg(long, value_name = "DIALECT")]
    pub dialect: Option<String>,

    /// Do not recurse into input directories.
    #[arg(long = "no-recursive")]
    pub no_recursive: bool,

    /// Output path ('-' or omitted for stdout).
    #[arg(long, short, value_name = "FILE")]
    pub output: Option<PathBuf>,
}

/// Paired `--colour` / `--no-colour` toggle (resolved against config + TTY).
#[derive(Debug, Args)]
pub struct ColourArgs {
    /// Force-enable ANSI colour in the output.
    #[arg(long = "colour", overrides_with = "no_colour")]
    pub colour: bool,

    /// Force-disable ANSI colour in the output.
    #[arg(long = "no-colour")]
    pub no_colour: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Compile source and emit human-readable bytecode disassembly.
    #[command(visible_aliases = ["asm", "disassemble"])]
    Dis {
        #[command(flatten)]
        input: InputArgs,
        /// Apply optimisation passes before disassembling.
        #[arg(long)]
        optimise: bool,
    },

    /// Compile source to a WebAssembly binary.
    ///
    /// Writes to `out.wasm` when `--output` is omitted (use `-o -` for stdout).
    #[command(visible_alias = "wasm")]
    Compwasm {
        #[command(flatten)]
        input: InputArgs,
        /// Which wasm backend to emit. `vm` (default) is the self-contained
        /// bytecode-VM runner (`vm.wasm` — runs any script incl. coroutines, no
        /// imports/WASI); `tree-walker` is the legacy eval-fallback module that
        /// imports the runtime ABI.
        #[arg(long, value_enum, default_value_t = WasmBackend::Vm)]
        backend: WasmBackend,
        /// Also write the textual WAT form to this path (`tree-walker` only).
        #[arg(long = "wat-output", value_name = "FILE")]
        wat_output: Option<PathBuf>,
    },

    /// Run diagnostics across all resolved inputs.
    #[command(visible_alias = "diagnostics")]
    Diag {
        #[command(flatten)]
        input: InputArgs,
        #[command(flatten)]
        diag: DiagArgs,
    },

    /// Run lint diagnostics across all resolved inputs.
    Lint {
        #[command(flatten)]
        input: InputArgs,
        #[command(flatten)]
        diag: DiagArgs,
    },

    /// Validate source (error-level diagnostics only).
    Validate {
        #[command(flatten)]
        input: InputArgs,
        #[command(flatten)]
        diag: DiagArgs,
    },

    /// Diff two sources using AST, IR, and CFG representations.
    Diff {
        /// Left-hand input file.
        left: Option<PathBuf>,
        /// Right-hand input file.
        right: Option<PathBuf>,
        /// Inline left-hand source.
        #[arg(long = "left-source", value_name = "CODE")]
        left_source: Option<String>,
        /// Inline right-hand source.
        #[arg(long = "right-source", value_name = "CODE")]
        right_source: Option<String>,
        /// Dialect profile. Defaults to auto-detection over the left-hand
        /// input (directive, shebang, content signals, then extension),
        /// falling back to tcl8.6.
        #[arg(long, value_name = "DIALECT")]
        dialect: Option<String>,
        /// Layers to show.
        #[arg(
            long,
            value_name = "LAYER",
            value_delimiter = ',',
            default_value = "all"
        )]
        show: Vec<String>,
        /// Emit JSON instead of a unified diff.
        #[arg(long)]
        json: bool,
        /// Output path ('-' for stdout).
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Emit symbol definitions (procs, namespaces, variables, events).
    #[command(visible_alias = "syms")]
    Symbols {
        #[command(flatten)]
        input: InputArgs,
        #[arg(long)]
        json: bool,
    },

    /// Extract control-flow diagram data from compiler IR.
    Diagram {
        #[command(flatten)]
        input: InputArgs,
        #[arg(long)]
        json: bool,
    },

    /// Build the procedure call graph.
    #[command(visible_alias = "call-graph")]
    Callgraph {
        #[command(flatten)]
        input: InputArgs,
        #[arg(long)]
        json: bool,
    },

    /// Build the symbol relationship graph (proc/variable references).
    #[command(visible_alias = "symbol-graph")]
    Symbolgraph {
        #[command(flatten)]
        input: InputArgs,
        #[arg(long)]
        json: bool,
    },

    /// Build the taint / effect data-flow graph.
    #[command(visible_alias = "dataflow-graph")]
    Dataflow {
        #[command(flatten)]
        input: InputArgs,
        #[arg(long)]
        json: bool,
    },

    /// Emit syntax-highlighted source output.
    #[command(visible_alias = "hl")]
    Highlight {
        #[command(flatten)]
        input: InputArgs,
        /// Output rendering format.
        #[arg(long, value_name = "FORMAT", default_value = "ansi", value_parser = ["ansi", "html"])]
        format: String,
        #[command(flatten)]
        colour: ColourArgs,
    },

    /// Look up command registry metadata (arity, args, side effects).
    #[command(name = "command-info", visible_aliases = ["commandinfo", "cmd-info"])]
    CmdInfo {
        /// Command name to query (for example: `HTTP::uri` or string).
        #[arg(value_name = "COMMAND")]
        command: String,
        /// Dialect profile for command metadata lookup.
        #[arg(long, default_value = "tcl8.6", value_name = "DIALECT")]
        dialect: String,
        #[arg(long)]
        json: bool,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Search KCS help docs from the bundled `SQLite` index.
    #[command(visible_alias = "docs")]
    Help {
        /// Search terms (omit to list available help sections).
        #[arg(value_name = "QUERY")]
        query: Vec<String>,
        /// Filter help matches by dialect context.
        #[arg(long, default_value = "all", value_name = "DIALECT")]
        dialect: String,
        /// Maximum number of help search matches.
        #[arg(long, default_value_t = 20, value_name = "N")]
        limit: usize,
        #[arg(long)]
        json: bool,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Reduce a diagnostic to a minimal reproducer for bug reports.
    ///
    /// The diagnostic CODE is the **last** positional argument, with any
    /// inputs before it — `tcl minimize script.tcl W220`. This is a variadic
    /// `inputs` list followed by a required trailing `code`, which clap cannot
    /// express as two separate positionals (a required positional may not follow
    /// a variadic one), so CODE is split off the trailing input in the handler.
    #[command(visible_aliases = ["minimise", "repro"])]
    Minimize {
        #[command(flatten)]
        input: InputArgs,
        /// Do not rename identifiers in the reproducer.
        #[arg(long = "no-rename")]
        no_rename: bool,
        #[arg(long)]
        json: bool,
    },

    /// Run compiler-explorer views on aggregated input.
    Explore {
        #[command(flatten)]
        input: InputArgs,
        /// Views to show (ir, cfg, ssa, types, opt, taint, asm, wasm, ...).
        #[arg(long, value_name = "VIEW", value_delimiter = ',')]
        show: Vec<String>,
        /// Emit the full machine-readable JSON (the explorer contract shape).
        #[arg(long)]
        json: bool,
        /// Render the views as box-drawing text trees (ANSI colour by default).
        #[arg(long)]
        text: bool,
        /// Launch the interactive terminal UI (requires the `tui` feature).
        #[arg(long)]
        tui: bool,
        /// Serve the interactive web GUI (the compiler explorer) locally.
        #[arg(long, conflicts_with_all = ["json", "text", "tui"])]
        serve: bool,
        /// Bind address for `--serve` (default: 127.0.0.1).
        #[arg(long, value_name = "ADDR", default_value = "127.0.0.1")]
        bind: String,
        /// Port for `--serve` (default: 8080; 0 picks a free port).
        #[arg(long, value_name = "PORT", default_value_t = 8080)]
        port: u16,
        /// Open the GUI in the default browser after `--serve` starts.
        #[arg(long)]
        open: bool,
        #[command(flatten)]
        colour: ColourArgs,
    },

    /// Report legacy patterns eligible for modernisation (detection only).
    FindLegacy {
        #[command(flatten)]
        input: InputArgs,
        #[arg(long)]
        json: bool,
    },

    /// Dump the full command registry (arities, traits, subcommands) as JSON.
    #[command(visible_aliases = ["registrydump", "dump-registry"])]
    RegistryDump {
        /// Dialect profile to snapshot.
        #[arg(long, default_value = "tcl8.6", value_name = "DIALECT")]
        dialect: String,
        /// Snapshot every dialect instead of one.
        #[arg(long = "all-dialects", conflicts_with = "dialect")]
        all_dialects: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Optimise source and emit rewritten Tcl.
    #[command(visible_aliases = ["optimise", "optimize"])]
    Opt {
        #[command(flatten)]
        input: InputArgs,
        /// Optimisation profile.
        #[arg(long, default_value = "full", value_name = "PROFILE",
              value_parser = ["off", "readability", "standard", "full", "aggressive"])]
        profile: String,
        /// Disable specific optimisation codes (repeatable).
        #[arg(long = "disable", value_name = "CODE")]
        disable: Vec<String>,
        /// Enable specific optimisation codes (repeatable).
        #[arg(long = "enable", value_name = "CODE")]
        enable: Vec<String>,
        #[command(flatten)]
        colour: ColourArgs,
    },

    /// Format source and emit canonical rewritten Tcl.
    #[command(visible_alias = "fmt")]
    Format {
        #[command(flatten)]
        input: InputArgs,
        /// Indentation width.
        #[arg(long = "indent-size", value_name = "N")]
        indent_size: Option<usize>,
        /// Indentation style.
        #[arg(long = "indent-style", value_name = "STYLE", value_parser = ["spaces", "tabs"])]
        indent_style: Option<String>,
        /// Maximum line length before wrapping.
        #[arg(long = "max-line-length", value_name = "N")]
        max_line_length: Option<usize>,
        #[command(flatten)]
        colour: ColourArgs,
    },

    /// Minify source: strip comments, collapse whitespace, join commands.
    #[command(visible_alias = "min")]
    Minify {
        #[command(flatten)]
        input: InputArgs,
        /// Compact proc-local variable and parameter names to short
        /// identifiers (procedure names only with --isolated — they are
        /// public command identities).
        #[arg(long)]
        compact: bool,
        /// Write the symbol map (original -> compacted names) to FILE.
        #[arg(long = "symbol-map", value_name = "FILE")]
        symbol_map: Option<PathBuf>,
        /// Maximum compression: run all optimiser passes, then compact +
        /// alias + minify. NOT frame-transparent: injects helper variables
        /// observable via `info vars` and variable traces.
        #[arg(long)]
        aggressive: bool,
        /// Assert the script is self-contained (no external callers or
        /// reflection over it) — also compact procedure names and
        /// global-scope variables.
        #[arg(long)]
        isolated: bool,
        #[command(flatten)]
        colour: ColourArgs,
    },

    /// Translate a minified-code error message back to original names.
    #[command(visible_alias = "umerr")]
    UnminifyError {
        /// Symbol map written by `minify --symbol-map`.
        #[arg(long = "symbol-map", value_name = "FILE")]
        symbol_map: PathBuf,
        /// Error message text to translate (inline).
        #[arg(long, short = 'e', value_name = "TEXT")]
        error: Option<String>,
        /// File containing error messages to translate ('-' for stdin).
        #[arg(long = "error-file", value_name = "FILE")]
        error_file: Option<PathBuf>,
        /// The minified source file (for line-number remapping).
        #[arg(long, value_name = "FILE")]
        minified: Option<PathBuf>,
        /// The original source file (for line-number remapping).
        #[arg(long, value_name = "FILE")]
        original: Option<PathBuf>,
        /// Output path ('-' for stdout).
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Print a bash / fish / zsh completion script for the tcl CLI.
    Completion {
        /// Target shell.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Manage Tcl packages (tclpkg manifests, lockfiles, registry).
    Pkg {
        #[command(subcommand)]
        action: PkgCommand,
    },

    /// Manage isolated Tcl package virtual environments.
    Venv {
        #[command(subcommand)]
        action: VenvCommand,
    },

    /// Generate Dockerfiles and install recipes for Tcl projects.
    Docker {
        #[command(subcommand)]
        action: DockerCommand,
    },
}

/// Diagnostic-filtering flags shared by `diag` / `lint` / `validate`.
#[derive(Debug, Args)]
pub struct DiagArgs {
    /// Emit diagnostics as JSON.
    #[arg(long)]
    pub json: bool,
    /// Disable specific diagnostic codes (repeatable).
    #[arg(long = "disable", value_name = "CODE")]
    pub disable: Vec<String>,
    /// Enable specific diagnostic codes (repeatable).
    #[arg(long = "enable", value_name = "CODE")]
    pub enable: Vec<String>,
}

/// Flags shared by most `pkg` sub-actions.
#[derive(Debug, Args)]
pub struct PkgCommon {
    /// Emit JSON output.
    #[arg(long)]
    pub json: bool,
    /// Override tclpkg.tcl location.
    #[arg(long, value_name = "PATH")]
    pub manifest: Option<PathBuf>,
    /// Never touch the network.
    #[arg(long)]
    pub offline: bool,
}

#[derive(Debug, Subcommand)]
pub enum PkgCommand {
    /// Create a new tclpkg.tcl manifest.
    // `--version` here is the manifest's initial version, not the CLI version,
    // so suppress clap's auto-generated propagated `--version` flag.
    #[command(disable_version_flag = true)]
    Init {
        /// Package name (default: directory name).
        #[arg(long)]
        name: Option<String>,
        /// Initial version.
        #[arg(long = "version")]
        init_version: Option<String>,
        /// SPDX licence identifier.
        #[arg(long = "license")]
        init_license: Option<String>,
        /// Tcl version constraint (default: >=8.6).
        #[arg(long)]
        tcl: Option<String>,
        /// Overwrite existing manifest.
        #[arg(long)]
        force: bool,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Resolve + fetch + materialise packages.
    Install {
        #[command(flatten)]
        common: PkgCommon,
        /// Skip dev-require packages.
        #[arg(long = "no-dev")]
        no_dev: bool,
        /// Refuse to change lockfile.
        #[arg(long)]
        frozen: bool,
    },
    /// List installed packages.
    List {
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Show the dependency tree.
    Tree {
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Verify integrity hashes.
    Verify {
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Show details for a package.
    Info {
        /// Package name.
        package: String,
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Add a dependency to the manifest.
    Add {
        /// Package name.
        package: String,
        /// Minimum version (default: 0.0.1).
        min_version: Option<String>,
        /// Explicit source URL.
        #[arg(long)]
        source: Option<String>,
        /// Add as dev-require.
        #[arg(long)]
        dev: bool,
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Remove a dependency from the manifest.
    Remove {
        /// Package name to remove.
        package: String,
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Bump dependency minimums.
    Update {
        /// Packages to update (default: all).
        packages: Vec<String>,
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Lock-driven install (alias for install --frozen).
    Sync {
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Show packages with newer versions available.
    Outdated {
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Explain why a package is in the dependency graph.
    Why {
        /// Package name.
        package: String,
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Copy packages from cache into the project tree.
    Vendor {
        /// Vendor directory.
        #[arg(long, default_value = "vendor")]
        dir: PathBuf,
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Run the manifest entry point via tclsh.
    Run {
        /// Extra arguments passed to tclsh.
        extra: Vec<String>,
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Output locked versions as manifest directives.
    Freeze {
        #[command(flatten)]
        common: PkgCommon,
    },
    /// Search the package registry.
    Search {
        /// Search query.
        query: String,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
        /// Use cached registry only.
        #[arg(long)]
        offline: bool,
    },
    /// Inspect the effective sandbox / hooks / registry policy.
    Policy {
        #[command(subcommand)]
        action: PolicyAction,
    },
    /// List the operator hooks bound to lifecycle stages.
    Hooks {
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Show recent sandboxed-execution audit records.
    Audit {
        /// Number of trailing records to show.
        #[arg(long, default_value_t = 20)]
        lines: usize,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Trust a package's build script so it may run (when build scripts are
    /// enabled by policy). Writes to the per-user policy layer.
    Trust {
        /// Package name.
        package: String,
        /// Remove the package from the trusted list instead.
        #[arg(long)]
        remove: bool,
    },
    /// Run the manifest's declared build script in a deprivileged sandbox.
    /// Requires `[build] allow-build-scripts` and the package to be trusted.
    Build {
        #[command(flatten)]
        common: PkgCommon,
    },
}

/// Sub-actions for `tcl pkg policy`.
#[derive(Debug, Subcommand)]
pub enum PolicyAction {
    /// Print the merged, effective policy and which keys are admin-locked.
    Show {
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Validate policy files and report any problems (non-zero on warnings).
    Verify {
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum VenvCommand {
    /// Create a new virtual environment.
    Create {
        /// Venv directory.
        #[arg(default_value = ".venv")]
        path: PathBuf,
        /// Pin a specific Tcl version (e.g. 8.6, 9.0).
        #[arg(long)]
        tcl: Option<String>,
        /// Allow fallback to host `auto_path`.
        #[arg(long = "system-site-packages")]
        system_site_packages: bool,
        /// Custom shell prompt label.
        #[arg(long)]
        prompt: Option<String>,
        /// Overwrite existing directory.
        #[arg(long)]
        force: bool,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Remove a virtual environment.
    Delete {
        /// Venv directory.
        #[arg(default_value = ".venv")]
        path: PathBuf,
        /// Force deletion even if active.
        #[arg(long)]
        force: bool,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Show virtual environment details.
    Info {
        /// Venv directory.
        #[arg(default_value = ".venv")]
        path: PathBuf,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Print activation script to stdout.
    Activate {
        /// Venv directory.
        #[arg(default_value = ".venv")]
        path: PathBuf,
        /// Shell flavour (default: auto-detect).
        #[arg(long, value_parser = ["bash", "zsh", "fish", "csh", "powershell"])]
        shell: Option<String>,
    },
    /// Print deactivation script to stdout.
    Deactivate {
        /// Shell flavour (default: auto-detect).
        #[arg(long, value_parser = ["bash", "zsh", "fish", "csh", "powershell"])]
        shell: Option<String>,
    },
    /// List discoverable virtual environments.
    List {
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Re-link a venv to a newer tclsh.
    Update {
        /// Venv directory.
        #[arg(default_value = ".venv")]
        path: PathBuf,
        /// New Tcl version to pin.
        #[arg(long, required = true)]
        tcl: String,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Run a command inside a venv.
    Run {
        /// Venv directory.
        #[arg(default_value = ".venv")]
        path: PathBuf,
        /// Command to run (after --).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum DockerCommand {
    /// Generate a Dockerfile for a Tcl project.
    Create {
        /// Base Docker image (e.g. debian:bookworm-slim, alpine:3.19).
        image: String,
        /// Tcl version to install.
        #[arg(long = "tcl-version", default_value = "8.6", value_parser = ["8.4", "8.5", "8.6", "9.0"])]
        tcl_version: String,
        /// Output file path.
        #[arg(long, short = 'o', default_value = "Dockerfile")]
        output: PathBuf,
        /// Container WORKDIR.
        #[arg(long, default_value = "/app")]
        workdir: String,
        /// Tcl script to run as CMD (e.g. main.tcl).
        #[arg(long)]
        entrypoint: Option<String>,
        /// Create a Tcl virtual environment inside the container.
        #[arg(long)]
        venv: bool,
        /// Skip COPY . . (useful for multi-stage builds).
        #[arg(long = "no-copy")]
        no_copy: bool,
        /// Skip tclpkg sync step.
        #[arg(long = "no-packages")]
        no_packages: bool,
        /// Additional OS package to install (repeatable).
        #[arg(long = "extra-package")]
        extra_package: Vec<String>,
        /// Docker LABEL as key=value (repeatable).
        #[arg(long)]
        label: Vec<String>,
        /// Docker ENV as key=value (repeatable).
        #[arg(long)]
        env: Vec<String>,
        /// tcl CLI zipapp version to download (default: latest known).
        #[arg(long = "cli-version")]
        cli_version: Option<String>,
        /// Overwrite existing Dockerfile.
        #[arg(long)]
        force: bool,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Show the Tcl install recipe for a base image.
    Recipe {
        /// Base Docker image (e.g. alpine:3.19, ubuntu:22.04).
        image: String,
        /// Tcl version.
        #[arg(long = "tcl-version", default_value = "8.6", value_parser = ["8.4", "8.5", "8.6", "9.0"])]
        tcl_version: String,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// List available base-image families and Tcl versions.
    Info {
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
}
