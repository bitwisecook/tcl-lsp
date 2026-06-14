//! `clap` derive definitions for the `tcl` CLI command tree.
//!
//! Mirrors `tooling/tcl/main.py` + `tooling/tcl/verbs/*`. Verb names map to the
//! kebab-cased enum-variant names; Python aliases map to `visible_aliases`.
//!
//! Flag coverage is intentionally pragmatic for the scaffolding phase: the
//! common input/output/dialect surface every verb shares is modelled precisely
//! (it is the bulk of the parity contract), while verb-specific flags are added
//! as each verb's behaviour is ported. New flags slot into the existing structs
//! without reshaping the tree.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Unified Tcl toolchain CLI.
#[derive(Debug, Parser)]
#[command(
    name = "tcl",
    version,
    about = "Unified Tcl toolchain CLI.",
    disable_help_subcommand = true,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Input-resolution flags shared by most verbs (`tooling/cli/_utils.py`
/// `_add_input_arguments`).
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

    /// Dialect profile for parsing and registry lookups.
    #[arg(long, default_value = "tcl8.6", value_name = "DIALECT")]
    pub dialect: String,

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
    #[command(visible_alias = "wasm")]
    Compwasm {
        #[command(flatten)]
        input: InputArgs,
        /// Also write the textual WAT form to this path.
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
        /// Dialect profile.
        #[arg(long, default_value = "tcl8.6", value_name = "DIALECT")]
        dialect: String,
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
        #[arg(long, default_value = "tcl8.6", value_name = "DIALECT")]
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
    #[command(visible_aliases = ["minimise", "repro"])]
    Minimize {
        #[command(flatten)]
        input: InputArgs,
        /// Diagnostic code to reproduce (for example: E101).
        #[arg(value_name = "CODE")]
        code: String,
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
        #[arg(long, default_value = "full", value_name = "PROFILE")]
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
        #[arg(long = "indent-style", value_name = "STYLE", value_parser = ["space", "tab"])]
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
        /// Also compact identifier names.
        #[arg(long)]
        compact: bool,
        /// Write the symbol map to this path (with --compact).
        #[arg(long = "symbol-map", value_name = "FILE")]
        symbol_map: Option<PathBuf>,
    },

    /// Translate a minified-code error message back to original names.
    #[command(visible_alias = "umerr")]
    UnminifyError {
        /// Symbol map written by `minify --compact`.
        #[arg(long = "symbol-map", value_name = "FILE")]
        symbol_map: PathBuf,
        /// Error message text to translate.
        #[arg(long)]
        error: Option<String>,
        /// Read the error message from a file.
        #[arg(long = "error-file", value_name = "FILE")]
        error_file: Option<PathBuf>,
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

impl Command {
    /// The canonical verb name, used in not-yet-implemented messages and logs.
    pub fn verb_name(&self) -> &'static str {
        match self {
            Command::Dis { .. } => "dis",
            Command::Compwasm { .. } => "compwasm",
            Command::Diag { .. } => "diag",
            Command::Lint { .. } => "lint",
            Command::Validate { .. } => "validate",
            Command::Diff { .. } => "diff",
            Command::Symbols { .. } => "symbols",
            Command::Diagram { .. } => "diagram",
            Command::Callgraph { .. } => "callgraph",
            Command::Symbolgraph { .. } => "symbolgraph",
            Command::Dataflow { .. } => "dataflow",
            Command::Highlight { .. } => "highlight",
            Command::CmdInfo { .. } => "command-info",
            Command::Help { .. } => "help",
            Command::Minimize { .. } => "minimize",
            Command::Explore { .. } => "explore",
            Command::FindLegacy { .. } => "find-legacy",
            Command::RegistryDump { .. } => "registry-dump",
            Command::Opt { .. } => "opt",
            Command::Format { .. } => "format",
            Command::Minify { .. } => "minify",
            Command::UnminifyError { .. } => "unminify-error",
            Command::Completion { .. } => "completion",
            Command::Pkg { .. } => "pkg",
            Command::Venv { .. } => "venv",
            Command::Docker { .. } => "docker",
        }
    }
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

#[derive(Debug, Subcommand)]
pub enum PkgCommand {
    /// Create a new tclpkg.tcl manifest.
    Init,
    /// Resolve + fetch + materialise packages.
    Install,
    /// List installed packages.
    List,
    /// Show the dependency tree.
    Tree,
    /// Verify integrity hashes.
    Verify,
    /// Show details for a package.
    Info,
    /// Add a dependency to the manifest.
    Add,
    /// Remove a dependency from the manifest.
    Remove,
    /// Bump dependency minimums.
    Update,
    /// Lock-driven install (alias for install --frozen).
    Sync,
    /// Show packages with newer versions available.
    Outdated,
    /// Explain why a package is in the dependency graph.
    Why,
    /// Copy packages from cache into the project tree.
    Vendor,
    /// Run the manifest entry point via tclsh.
    Run,
    /// Output locked versions as manifest directives.
    Freeze,
    /// Search the package registry.
    Search,
}

#[derive(Debug, Subcommand)]
pub enum VenvCommand {
    /// Create a new virtual environment.
    Create,
    /// Remove a virtual environment.
    Delete,
    /// Show virtual environment details.
    Info,
    /// Print activation script to stdout.
    Activate,
    /// Print deactivation script to stdout.
    Deactivate,
    /// List discoverable virtual environments.
    List,
    /// Re-link a venv to a newer tclsh.
    Update,
    /// Run a command inside a venv.
    Run,
}

#[derive(Debug, Subcommand)]
pub enum DockerCommand {
    /// Generate a Dockerfile for a Tcl project.
    Create,
    /// Show the Tcl install recipe for a base image.
    Recipe,
    /// List available base-image families and Tcl versions.
    Info,
}
