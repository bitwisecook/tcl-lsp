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

use clap::builder::{PossibleValue, PossibleValuesParser};
use clap::{Args, Parser, Subcommand};
use tcl_dialect::DialectProfile;

/// The enumerated `--dialect` values, projected from the profile catalog: one
/// visible entry per canonical profile carrying its `display_name` as the
/// value help, plus the additive `tk` ingress, with every registered alias
/// (`irules`, `tcl-irule`) accepted but hidden.
///
/// This is exactly the set [`tcl_cli_support::resolve_dialect`] resolves, so
/// enumerating it in `--help` narrows nothing: an unrecognised spelling was
/// already an input error, it is now reported with the list of names.
fn dialect_possible_values() -> Vec<PossibleValue> {
    // T1: the `+ tk` special case is the *payload* ledger row T1 retires
    // (P1) — an environment enumeration has different contents, so
    // re-keying it changes `--help` rather than refactoring it. The `tk`
    // name itself now resolves through the one ingress seam.
    let tk = tcl_cli_support::environment::profile_for_dialect("tk");
    DialectProfile::all()
        .iter()
        .map(|profile| {
            PossibleValue::new(profile.name)
                .help(profile.display_name)
                .aliases(profile.aliases.iter().copied())
        })
        .chain(std::iter::once(
            PossibleValue::new(tk.name).help(tk.display_name),
        ))
        .collect()
}

/// Value parser for a `--dialect` argument naming one profile.
fn dialect_parser() -> PossibleValuesParser {
    PossibleValuesParser::new(dialect_possible_values())
}

/// Value parser for `tcl help --dialect`, which also takes its `all` default —
/// "match every dialect", not a profile.
fn dialect_filter_parser() -> PossibleValuesParser {
    let mut values = vec![PossibleValue::new("all").help("Every dialect (no filtering)")];
    values.extend(dialect_possible_values());
    PossibleValuesParser::new(values)
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
    #[arg(long, value_name = "DIALECT", value_parser = dialect_parser())]
    pub dialect: Option<String>,

    /// Do not recurse into input directories.
    #[arg(long = "no-recursive")]
    pub no_recursive: bool,

    /// Output path ('-' or omitted for stdout).
    #[arg(long, short, value_name = "FILE")]
    pub output: Option<PathBuf>,
}

impl InputArgs {
    /// Resolve the optional CLI spelling once at the ingest boundary.
    ///
    /// Every verb receives the canonical profile, so aliases such as
    /// `irules` cannot leak into string-keyed downstream checks.
    pub fn dialect_profile(
        &self,
    ) -> Result<Option<&'static tcl_dialect::DialectProfile>, tcl_cli_support::CliError> {
        tcl_cli_support::resolve_dialect(self.dialect.as_deref())
    }
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

/// Which semantic/AOT codegen optimisation passes the WASM emitter may use.
///
/// Deliberately **not** spelled `--optimise`: that flag already exists on
/// `tcl dis` and means the *source-rewrite* optimiser
/// (`tcl_compiler::optimiser`), which is a different thing from the
/// target-neutral semantic passes
/// (`tcl_compiler::semantic_optimisation`) this selects.
#[derive(Debug, Args)]
pub struct CodegenPassArgs {
    /// Codegen optimisation passes to enable, comma-separated.
    ///
    /// Individual passes (`native-lowering`, `representation-inference`,
    /// `trace-barrier-elision`, `cell-demotion`, `direct-proc`,
    /// `frame-elision`, `native-integer`, …) or a group: `native-tier` for
    /// the four the native tier enables, `all` for every pass. Omitted, no
    /// pass runs and the emitter produces the generic lowering.
    /// `tcl explore --json` lists them all under `semanticOptimisations`.
    #[arg(long = "codegen-passes", value_name = "PASS[,PASS...]")]
    pub codegen_passes: Option<String>,
}

impl CodegenPassArgs {
    /// Resolve the selection, or an error naming the unrecognised pass.
    ///
    /// # Errors
    ///
    /// Propagates the parse failure from
    /// [`SemanticOptimisationConfig::from_names`](tcl_explorer::SemanticOptimisationConfig::from_names).
    pub fn config(&self) -> anyhow::Result<tcl_explorer::SemanticOptimisationConfig> {
        match self.codegen_passes.as_deref() {
            None => Ok(tcl_explorer::SemanticOptimisationConfig::new()),
            Some(spec) => tcl_explorer::SemanticOptimisationConfig::from_names(spec)
                .map_err(|message| anyhow::anyhow!("--codegen-passes: {message}")),
        }
    }
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
        /// Also write the textual WAT form to this path.
        #[arg(long = "wat-output", value_name = "FILE")]
        wat_output: Option<PathBuf>,
        #[command(flatten)]
        codegen: CodegenPassArgs,
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
        #[arg(long, value_name = "DIALECT", value_parser = dialect_parser())]
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
        #[arg(long, default_value = "tcl8.6", value_name = "DIALECT", value_parser = dialect_parser())]
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
        #[arg(long, default_value = "all", value_name = "DIALECT", value_parser = dialect_filter_parser())]
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
        codegen: CodegenPassArgs,
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
        #[arg(long, default_value = "tcl8.6", value_name = "DIALECT", value_parser = dialect_parser())]
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
        /// Do not emit unique-prefix keyword abbreviations (`string le` for
        /// `string length`, `-noc` for `-nocase`). Abbreviated output is
        /// correct but harder to eyeball-diff. Only affects --aggressive.
        #[arg(long = "no-abbreviations")]
        no_abbreviations: bool,
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

    /// Author `SpecTcl` (.tclspec) command packs.
    Spec {
        #[command(subcommand)]
        action: SpecCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum SpecCommand {
    /// Derive version ranges for a package's commands from several releases.
    ///
    /// Each release is drafted on its own and the drafts are diffed, so
    /// `introduced_version` / `retired_version` say what the releases witness
    /// instead of repeating whatever version the newest sources declare.
    Import(SpecImportArgs),

    /// Rewrite a 1.x pack's source to the newest `SpecTcl` vocabulary.
    ///
    /// Every loader reads all known vocabulary versions in full, so nothing
    /// is *forced* to upgrade — but 2.0 changes meaning by adding words and
    /// translating the legacy ones, so this rewrites statements as well as
    /// the `speclib` version word: `dialects` / `-dialects` rows become
    /// `available` rows at every scope (U2), `ambient_package` and
    /// `file_extension … -dialect` rows rehome into `environment … -extend`
    /// blocks under a cannot-infer rule that leaves ambiguous packs partial
    /// (U4/U5), and the version word moves to 2.0 only when the body rewrite
    /// completed on that file (U1). Edits are content-range replacements
    /// located by the loader's own lexer and applied back-to-front, so
    /// layout, comments and delimiters survive; `--verify` proves the
    /// original and the rewrite load to byte-identical registry snapshots.
    Upgrade(SpecUpgradeArgs),

    /// Render a pack as canonical `SpecTcl` — its expansion, if it is a
    /// program.
    ///
    /// The pack is evaluated (design E) and the registrations it made are
    /// written back as straight-line source: literal `command` / `option` /
    /// `subcommand` declarations, no `proc`, no `foreach`. A pack that is
    /// already canonical round-trips; a templated one is expanded, which is
    /// how its author reads what the loop actually registered. Expansion is
    /// total and contraction is never attempted — a program is not recovered
    /// from its expansion.
    Export(SpecExportArgs),
}

/// Flags of `tcl spec export`.
#[derive(Debug, Args)]
pub struct SpecExportArgs {
    /// The `.tclspec` file to expand.
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Write the canonical pack here instead of stdout.
    #[arg(long = "out", short = 'o', value_name = "FILE")]
    pub out: Option<PathBuf>,

    /// Report the expansion as JSON — the canonical source, whether the pack
    /// is target-dependent, and every load notice.
    #[arg(long)]
    pub json: bool,
}

/// Flags of `tcl spec upgrade`.
#[derive(Debug, Args)]
pub struct SpecUpgradeArgs {
    /// The `.tclspec` files to upgrade.
    #[arg(value_name = "FILE", required = true)]
    pub files: Vec<PathBuf>,

    /// The vocabulary the files are expected to declare.
    #[arg(long, value_name = "VERSION", default_value = "1.0")]
    pub from: String,

    /// The vocabulary to rewrite to. Downgrades are refused.
    #[arg(long, value_name = "VERSION", default_value = "2.0")]
    pub to: String,

    #[command(flatten)]
    pub proof: SpecUpgradeProof,

    #[command(flatten)]
    pub shape: SpecUpgradeShape,
}

/// The `tcl spec upgrade` switches that report or prove instead of writing.
#[derive(Debug, Args)]
pub struct SpecUpgradeProof {
    /// Report what would change without writing anything; exits non-zero if
    /// any file is behind the newest vocabulary.
    #[arg(long)]
    pub check: bool,

    /// Prove the upgrade is behaviour-preserving instead of writing it: the
    /// original and the rewritten pack must produce byte-identical registry
    /// snapshots (upgrade spec U9). Implies --check.
    #[arg(long)]
    pub verify: bool,
}

/// The `tcl spec upgrade` switches that change the rewritten pack's shape
/// rather than its spelling.
#[derive(Debug, Args)]
pub struct SpecUpgradeShape {
    /// Hoist a uniform `required_package` (the pack-level default, or one
    /// identical row in every command) to a pack-level `provides`
    /// declaration (upgrade spec U6). Off by default: it changes the
    /// pack's shape, not just its spelling.
    #[arg(long = "infer-provides")]
    pub infer_provides: bool,

    /// Re-emit the upgraded pack in canonical 2.0 form — straight-line
    /// registration calls at the house layout, through the same renderer
    /// `tcl spec export` uses. Comments and author layout do not survive
    /// it. A programmed pack (one whose `speclib` body runs rather than
    /// registers) is refused whole, never rewritten (design E-R12), and a
    /// partially upgraded file keeps its TODO markers instead.
    #[arg(long)]
    pub restyle: bool,
}

/// Flags of `tcl spec import`.
#[derive(Debug, Args)]
pub struct SpecImportArgs {
    /// One release's sources as VERSION=PATH, where PATH is a directory, a
    /// .zip, or a .tar.gz (repeatable; order does not matter).
    #[arg(long = "snapshot", value_name = "VERSION=PATH")]
    pub snapshot: Vec<String>,

    /// Enumerate a GitHub repository's release tags and fetch each one
    /// instead of reading local snapshots. Honours `GITHUB_TOKEN` (sent as a
    /// bearer token) and the standard proxy environment variables.
    #[arg(
        long = "github",
        value_name = "OWNER/REPO",
        conflicts_with = "snapshot"
    )]
    pub github: Option<String>,

    /// Keep only tags matching this glob: `*` matches any run of characters,
    /// `?` exactly one, and the whole tag must match.
    #[arg(long = "tag-pattern", value_name = "GLOB", requires = "github")]
    pub tag_pattern: Option<String>,

    /// Import only the newest N matching releases.
    #[arg(long = "limit", value_name = "N", requires = "github")]
    pub limit: Option<usize>,

    /// Print the tags that would be fetched and stop (a dry run).
    #[arg(long = "list-tags", requires = "github")]
    pub list_tags: bool,

    /// Dialect profile every snapshot is analysed as.
    #[arg(long, default_value = "tcl8.6", value_name = "DIALECT")]
    pub dialect: String,

    /// Pack name for the rendered `speclib` block (default: the name the
    /// sources `package provide`).
    #[arg(long, value_name = "NAME")]
    pub package: Option<String>,

    /// Write the pack here instead of stdout.
    #[arg(long = "out", short = 'o', value_name = "FILE")]
    pub out: Option<PathBuf>,

    #[command(flatten)]
    pub history: HistoryArgs,

    /// Network timeout in seconds for the GitHub API and codeload fetches.
    #[arg(long, default_value_t = 60, value_name = "SECONDS")]
    pub timeout: u64,

    /// Emit the derivation as JSON (pack source, per-command ranges,
    /// warnings) instead of the pack alone.
    #[arg(long)]
    pub json: bool,
}

/// Paired `--complete-history` / `--partial-history` claim about how much of
/// a package's release history a set of snapshots covers.
///
/// Modelled the way `--colour` / `--no-colour` is: the pair is one decision,
/// and giving both is a contradiction rather than a last-one-wins.
#[derive(Debug, Args)]
pub struct HistoryArgs {
    /// Declare that the snapshots are *every* release of the package, which
    /// makes presence in the earliest one an introduction. Off by default:
    /// unclaimed history is the safe assumption, and a wrongly-claimed
    /// `introduced_version` cannot be told from a derived one afterwards.
    #[arg(long = "complete-history", conflicts_with = "partial_history")]
    pub complete_history: bool,

    /// Declare the snapshots are only *some* releases. This is the default;
    /// the flag exists so a script can say so explicitly.
    #[arg(long = "partial-history")]
    pub partial_history: bool,
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
    /// Discover package requirements in project Tcl source.
    #[command(visible_alias = "scan")]
    Discover {
        /// Files or directories to scan (default: the manifest directory).
        #[arg(value_name = "INPUT")]
        inputs: Vec<PathBuf>,
        /// Add safe, previously undeclared findings to tclpkg.tcl.
        #[arg(long)]
        add: bool,
        /// Do not recurse into input directories.
        #[arg(long = "no-recursive")]
        no_recursive: bool,
        /// Dialect profile override (default: detect per file).
        #[arg(long, value_name = "DIALECT", value_parser = dialect_parser())]
        dialect: Option<String>,
        #[command(flatten)]
        common: PkgCommon,
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
        /// Base Docker image. Defaults to Debian because release binaries
        /// require glibc.
        #[arg(default_value = tcl_pkg::docker::DEFAULT_BASE_IMAGE)]
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
        /// tcl-lsp release to install the native tcl CLI from
        /// (default: this binary's own release; empty resolves the latest).
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
        /// Show the native tcl CLI install recipe instead of the Tcl one.
        #[arg(long)]
        cli: bool,
        /// tcl-lsp release the --cli recipe pins
        /// (default: this binary's own release; empty resolves the latest).
        #[arg(long = "cli-version")]
        cli_version: Option<String>,
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
