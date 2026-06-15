//! `clap` derive definitions for the `f5-query` CLI command tree.
//!
//! Mirrors `tooling/f5/main.py` + `tooling/f5/verbs/*` (including the `irule`
//! verb group). Verb names map to kebab-cased variant names; Python aliases map
//! to `visible_aliases`.
//!
//! Flag coverage is pragmatic for scaffolding: shared surfaces (`--format`
//! scf/tmsh/tmsh-delta, remote credentials, passphrase handling) are modelled
//! as reusable `Args` structs; verb-specific flags are filled in as each verb
//! is ported.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// jq-flavoured BIG-IP / iRules query and rewrite CLI.
#[derive(Debug, Parser)]
#[command(
    name = "f5-query",
    bin_name = "f5-query",
    version,
    about = "Inspect, transform, and query F5 BIG-IP configs and iRules.",
    disable_help_subcommand = true,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Output-format flag shared by config-producing verbs (`verbs/_emit.py`).
#[derive(Debug, Args)]
pub struct FormatArgs {
    /// Output rendering: SCF text, a tmsh script, or a tmsh delta.
    #[arg(long, default_value = "scf", value_parser = ["scf", "tmsh", "tmsh-delta"], value_name = "FORMAT")]
    pub format: String,

    /// Wrap the tmsh script in a cli transaction (requires a tmsh format).
    #[arg(long)]
    pub transaction: bool,
}

/// Remote BIG-IP credential flags shared by `fetch` / `push` / `pull`.
#[derive(Debug, Args)]
pub struct RemoteArgs {
    /// BIG-IP management host or IP.
    #[arg(long, value_name = "HOST")]
    pub host: Option<String>,
    /// Username (falls back to env / config / prompt).
    #[arg(long, value_name = "USER")]
    pub user: Option<String>,
    /// Password (falls back to env / config / prompt).
    #[arg(long, value_name = "PASSWORD")]
    pub password: Option<String>,
    /// Management TCP port.
    #[arg(long, value_name = "PORT")]
    pub port: Option<u16>,
    /// Skip TLS certificate verification.
    #[arg(long)]
    pub insecure: bool,
    /// Per-request timeout in seconds.
    #[arg(long, value_name = "SECS")]
    pub timeout: Option<u64>,
}

/// UCS passphrase flags shared by verbs that read encrypted archives.
#[derive(Debug, Default, Args)]
pub struct PassphraseArgs {
    /// Read the UCS passphrase from this environment variable.
    #[arg(long = "passphrase-env", value_name = "VAR")]
    pub passphrase_env: Option<String>,
    /// Never prompt interactively for a passphrase.
    #[arg(long = "no-passphrase-prompt")]
    pub no_passphrase_prompt: bool,
}

impl PassphraseArgs {
    /// Build [`tcl_bigip_io::PassphraseOptions`], wiring the secure TTY prompt
    /// (honouring `--passphrase-env` / `--no-passphrase-prompt`). The pure
    /// library calls back into `tcl-cli-support` only when an encrypted UCS
    /// actually needs a passphrase and none was supplied non-interactively.
    #[must_use]
    pub fn to_options(&self) -> tcl_bigip_io::PassphraseOptions {
        tcl_bigip_io::PassphraseOptions {
            explicit: None,
            env_var: self
                .passphrase_env
                .clone()
                .unwrap_or_else(|| tcl_bigip_io::DEFAULT_PASSPHRASE_ENV.to_owned()),
            allow_prompt: !self.no_passphrase_prompt,
            prompt: Some(tcl_cli_support::prompt::read_ucs_passphrase),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print object counts, partition breakdown, and top-references.
    #[command(visible_alias = "summary")]
    Stats {
        /// Config inputs (.conf/.scf/.ucs, or '-' for stdin).
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Show the top N most-referenced objects.
        #[arg(long, value_name = "N")]
        top: Option<usize>,
        #[arg(long)]
        json: bool,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
        #[command(flatten)]
        passphrase: PassphraseArgs,
    },

    /// Generate `tmsh delete` commands for unreferenced objects.
    #[command(visible_alias = "clean")]
    Cleanup {
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Keep this object path even if unreferenced (repeatable).
        #[arg(long = "keep", value_name = "PATH")]
        keep: Vec<String>,
        /// Do not implicitly keep /Common objects.
        #[arg(long = "no-keep-common")]
        no_keep_common: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
        #[command(flatten)]
        passphrase: PassphraseArgs,
    },

    /// List every BIG-IP object related to a given path or pattern.
    #[command(visible_alias = "related")]
    Grep {
        /// Object path or pattern to seed from.
        pattern: String,
        inputs: Vec<PathBuf>,
        /// Treat the pattern as a regular expression.
        #[arg(long = "regex", short = 'e')]
        regex: bool,
        /// Treat the pattern as a CIDR / IP match.
        #[arg(long = "cidr", short = 'c')]
        cidr: bool,
        /// Traversal direction.
        #[arg(long, default_value = "both", value_parser = ["forward", "reverse", "both"])]
        direction: String,
        /// Maximum traversal depth.
        #[arg(long = "max-depth", value_name = "N")]
        max_depth: Option<usize>,
        /// Maximum number of nodes to visit.
        #[arg(long = "max-nodes", value_name = "N")]
        max_nodes: Option<usize>,
        /// Emit the full object stanzas, not just paths.
        #[arg(long)]
        full: bool,
        #[arg(long)]
        json: bool,
    },

    /// Convert a local UCS archive to an SCF text file.
    #[command(visible_alias = "ucs2scf")]
    Extract {
        /// UCS archive path.
        ucs: PathBuf,
        /// Include non-config extras from the archive.
        #[arg(long = "include-extras")]
        include_extras: bool,
        #[command(flatten)]
        format: FormatArgs,
        #[command(flatten)]
        passphrase: PassphraseArgs,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Object-aware diff between two SCF or tmsh-output files.
    #[command(visible_alias = "changes")]
    Diff {
        /// The "before" config.
        before: PathBuf,
        /// The "after" config.
        after: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Split an SCF into per-partition files under a directory.
    Split {
        input: PathBuf,
        /// Output directory.
        output: PathBuf,
        #[command(flatten)]
        format: FormatArgs,
    },

    /// Concatenate split per-partition SCFs into a single bigip.conf.
    Merge {
        /// Per-partition `.conf` files (or one directory).
        #[arg(required = true, value_name = "PATH")]
        paths: Vec<PathBuf>,
        #[command(flatten)]
        format: FormatArgs,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Rename a BIG-IP object full-path and update every reference.
    #[command(visible_alias = "mv")]
    Rename {
        inputs: Vec<PathBuf>,
        /// Existing object full-path.
        #[arg(long, value_name = "PATH")]
        old: String,
        /// New object full-path.
        #[arg(long, value_name = "PATH")]
        new: String,
        /// Rewrite the input file in place.
        #[arg(long = "in-place")]
        in_place: bool,
        /// Emit the rewritten config (not a dry-run diff).
        #[arg(long)]
        write: bool,
        #[command(flatten)]
        format: FormatArgs,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Run BIG-IP best-practice / structural checks.
    #[command(visible_alias = "lint")]
    Validate {
        inputs: Vec<PathBuf>,
        /// Restrict to a check category.
        #[arg(long, value_parser = ["config", "irule"])]
        category: Option<String>,
        /// Minimum severity to report.
        #[arg(long)]
        severity: Option<String>,
        /// Report format.
        #[arg(long, default_value = "text", value_parser = ["text", "json", "sarif"])]
        format: String,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Describe the resolved plan for a virtual or pool.
    #[command(visible_alias = "describe")]
    Explain {
        /// Object kind.
        #[arg(value_parser = ["virtual", "pool", "auto"])]
        kind: String,
        /// Target object full-path or short name.
        target: String,
        /// bigip.conf / SCF files.
        #[arg(required = true, value_name = "PATH")]
        inputs: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Emit the object reference graph as DOT, JSON, or Mermaid.
    #[command(visible_alias = "deps")]
    Graph {
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        #[arg(long, default_value = "dot", value_parser = ["dot", "json", "mermaid"])]
        format: String,
        /// Seed traversal from this object path (repeatable).
        #[arg(long = "seed", value_name = "PATH")]
        seed: Vec<String>,
        /// Walk references in reverse.
        #[arg(long)]
        reverse: bool,
        #[arg(long = "max-depth", value_name = "N")]
        max_depth: Option<usize>,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
        #[command(flatten)]
        passphrase: PassphraseArgs,
    },

    /// Emit a tmsh script that creates every object in the input config.
    #[command(visible_alias = "scf2tmsh")]
    Tmsh {
        inputs: Vec<PathBuf>,
        /// Emit `modify` commands instead of `create`.
        #[arg(long)]
        modify: bool,
        /// Restrict to objects of this kind (repeatable).
        #[arg(long = "include", value_name = "KIND")]
        include: Vec<String>,
    },

    /// Convert between UCS / SCF / AS3 declaration formats.
    Convert {
        /// Conversion direction.
        #[arg(value_parser = ["ucs2scf", "scf2as3"])]
        format: String,
        inputs: Vec<PathBuf>,
        /// AS3 tenant name (scf2as3).
        #[arg(long, value_name = "NAME")]
        tenant: Option<String>,
        /// AS3 application name (scf2as3).
        #[arg(long, value_name = "NAME")]
        application: Option<String>,
        /// Emit a coverage report instead of the conversion.
        #[arg(long)]
        report: bool,
    },

    /// jq-flavoured DSL for inspecting and rewriting BIG-IP configs.
    #[command(visible_alias = "q")]
    Query {
        /// The query expression.
        expression: Option<String>,
        inputs: Vec<PathBuf>,
        /// Bind a named config (NAME=PATH, repeatable).
        #[arg(long = "name", value_name = "NAME=PATH")]
        name: Vec<String>,
        /// Bind an external JSON file to $NAME as a side input (repeatable).
        #[arg(long = "input-json", value_name = "NAME=PATH")]
        input_json: Vec<String>,
        /// Bind a JSON Lines (NDJSON) file to $NAME as a side input.
        #[arg(long = "input-jsonl", value_name = "NAME=PATH")]
        input_jsonl: Vec<String>,
        /// Bind a CSV file to $NAME as a side input (optional :hdr1,hdr2,… ).
        #[arg(long = "input-csv", value_name = "NAME=PATH[:hdr1,hdr2,…]")]
        input_csv: Vec<String>,
        /// Bind a BIG-IP log file to $NAME as a side input (structured events).
        #[arg(long = "input-f5log", value_name = "NAME=PATH")]
        input_f5log: Vec<String>,
        /// Bind a file via a registered input format (KIND NAME=PATH, repeatable).
        #[arg(long = "input", value_names = ["KIND", "NAME=PATH"], num_args = 2)]
        input: Vec<String>,
        /// Merge all configs into one namespace.
        #[arg(long)]
        merge: bool,
        /// Apply edits to stdout.
        #[arg(long)]
        write: bool,
        /// Apply edits in place.
        #[arg(long = "in-place")]
        in_place: bool,
        // Output-mode flags (mutually exclusive). Each sets `output_mode`
        // to the matching `output::render` mode; the default is `auto`.
        /// Render every selected value as an SCF stanza when possible.
        #[arg(long = "scf", group = "query_output_mode")]
        scf: bool,
        /// Render scalar values one per line, no quoting.
        #[arg(long = "raw", group = "query_output_mode")]
        raw: bool,
        /// Print only the full-path of each object / reference produced.
        #[arg(long = "paths-only", group = "query_output_mode")]
        paths_only: bool,
        /// Render the result as a JSON array.
        #[arg(long = "json", group = "query_output_mode")]
        json: bool,
        /// Render the result as an ASCII grid.
        #[arg(long = "table", group = "query_output_mode")]
        table: bool,
        /// Like --table but with Unicode box-drawing borders.
        #[arg(long = "table-lineart", group = "query_output_mode")]
        table_lineart: bool,
        /// Exit non-zero when a read-only query matched nothing.
        #[arg(long)]
        strict: bool,
        /// Opt the query in to live network probes (the ping / portping /
        /// traceroute / HTTP / socket / TLS-handshake builtins). Without this
        /// flag those builtins raise rather than touching the network — keeps
        /// the default invocation offline-safe.
        #[arg(long = "enable-probes")]
        enable_probes: bool,
        /// CA bundle to trust for TLS-aware probes (the HTTP and TLS-handshake
        /// builtins). Defaults to the platform trust store. Only used when a
        /// query runs a TLS probe.
        #[arg(long = "ca-bundle", value_name = "PATH")]
        ca_bundle: Option<String>,
        /// Dispatch output through a renderer plugin (mermaid / gantt /
        /// ascii-blocks). Overrides the output-mode flags.
        #[arg(long = "render", short = 'R', value_name = "NAME")]
        render_name: Option<String>,
        /// Pass an option to --render NAME (repeatable), e.g.
        /// `--render-opt direction=TB`.
        #[arg(long = "render-opt", value_name = "KEY=VALUE")]
        render_opt: Vec<String>,
        #[command(flatten)]
        format: FormatArgs,
        /// Show the DSL grammar reference and exit.
        #[arg(long = "help-dsl")]
        help_dsl: bool,
        /// Show the builtin catalogue (optionally one function) and exit.
        // `Option<Option<T>>` is clap's idiom for "flag present, value
        // optional": outer None = flag absent, Some(None) = `--help-builtins`
        // with no name (list all), Some(Some(n)) = a specific function.
        #[allow(clippy::option_option)]
        #[arg(long = "help-builtins", value_name = "NAME", num_args = 0..=1)]
        help_builtins: Option<Option<String>>,
        /// Show the worked-example cookbook and exit.
        #[arg(long = "help-examples")]
        help_examples: bool,
        /// Show the comprehensive manual (grammar + builtins + examples) and
        /// exit. Deferred: the builtins prose catalogue is not yet ported.
        #[arg(long = "help-manual")]
        help_manual: bool,
        /// List the registered renderer plugins and exit.
        #[arg(long = "help-renderers")]
        help_renderers: bool,
        /// List the registered input formats and exit.
        #[arg(long = "help-inputs")]
        help_inputs: bool,
    },

    /// Strip secrets and remap public IPs into a configurable CIDR pool.
    #[command(visible_alias = "sanitize")]
    Redact {
        inputs: Vec<PathBuf>,
        /// Do not remap IP addresses.
        #[arg(long = "keep-ips")]
        keep_ips: bool,
        /// CIDR pool to remap public IPs into.
        #[arg(long = "target-cidr", value_name = "CIDR")]
        target_cidr: Option<String>,
        /// Shuffle the IP assignment.
        #[arg(long)]
        shuffle: bool,
        /// Deterministic shuffle seed.
        #[arg(long, value_name = "SEED")]
        seed: Option<u64>,
        /// Also remap RFC1918 private ranges.
        #[arg(long = "remap-private")]
        remap_private: bool,
        /// Write the sidecar map to this path.
        #[arg(long = "map-file", value_name = "FILE")]
        map_file: Option<PathBuf>,
        #[command(flatten)]
        format: FormatArgs,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Reverse a previous redact using its sidecar map file.
    #[command(visible_alias = "unmap")]
    Unredact {
        /// Sidecar map file from `f5 redact`.
        map_file: PathBuf,
        /// Redacted config to restore.
        path: PathBuf,
        #[command(flatten)]
        format: FormatArgs,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Pull SCF or UCS from a live BIG-IP device (REST or SSH).
    #[command(visible_alias = "get")]
    Fetch {
        #[command(flatten)]
        remote: RemoteArgs,
        /// Transport to use.
        #[arg(long, default_value = "auto", value_parser = ["auto", "rest", "ssh"])]
        transport: String,
        /// Artifact(s) to retrieve.
        #[arg(long, default_value = "scf", value_parser = ["scf", "ucs", "both"])]
        format: String,
    },

    /// Replace or create a single object on a live BIG-IP via iControl REST.
    Push {
        /// Object kind (virtual/pool/node/rule).
        kind: String,
        /// JSON payload file.
        payload: PathBuf,
        #[command(flatten)]
        remote: RemoteArgs,
        /// Create instead of replace.
        #[arg(long)]
        create: bool,
        /// Show the request without sending it.
        #[arg(long = "dry-run")]
        dry_run: bool,
    },

    /// Fetch a single object from a live BIG-IP.
    Pull {
        /// Object kind (virtual/pool/node/rule).
        kind: String,
        /// Object full-path.
        full_path: String,
        #[command(flatten)]
        remote: RemoteArgs,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "scf", value_parser = ["scf", "json"])]
        format: String,
    },

    /// Trace each flow in a PCAP through the BIG-IP config.
    ExplainFlow {
        /// PCAP / PCAPNG capture file.
        pcap: PathBuf,
        /// Config inputs (repeatable).
        #[arg(long = "config", short = 'c', value_name = "FILE")]
        config: Vec<PathBuf>,
        /// Path to the tshark binary.
        #[arg(long, value_name = "BIN")]
        tshark: Option<PathBuf>,
        /// TLS keylog file for decryption.
        #[arg(long, value_name = "FILE")]
        keylog: Option<PathBuf>,
        /// Extra tshark display filter.
        #[arg(long = "tshark-filter", value_name = "EXPR")]
        tshark_filter: Option<String>,
        /// Simulate iRule event bodies per session.
        #[arg(long)]
        simulate: bool,
        /// Omit iRule event bodies from the report.
        #[arg(long = "no-event-bodies")]
        no_event_bodies: bool,
        #[arg(long)]
        json: bool,
    },

    /// Inject name-resolution (and optional keylog) blocks into a PCAPNG.
    #[command(visible_alias = "enrich")]
    EnrichPcapng {
        /// Config inputs (repeatable).
        #[arg(long = "config", short = 'c', value_name = "FILE")]
        config: Vec<PathBuf>,
        /// Input PCAP / PCAPNG.
        input: PathBuf,
        /// Output PCAPNG.
        output: PathBuf,
        /// TLS keylog file to embed as a DSB.
        #[arg(long, value_name = "FILE")]
        keylog: Option<PathBuf>,
        /// Include every config IP, not just those seen in the capture.
        #[arg(long)]
        all: bool,
        /// Report what would change without writing.
        #[arg(long = "dry-run")]
        dry_run: bool,
    },

    /// Generate a Wireshark profile directory from configs.
    #[command(visible_alias = "ws-profile")]
    EnrichWireshark {
        /// Config inputs (repeatable).
        #[arg(long = "config", short = 'c', value_name = "FILE")]
        config: Vec<PathBuf>,
        /// Output profile directory.
        output: PathBuf,
        /// Overwrite an existing profile directory.
        #[arg(long)]
        force: bool,
    },

    /// Apply an `f5 redact` map to a PCAP capture.
    #[command(visible_alias = "pcapmap")]
    PcapRemap {
        /// Sidecar map file.
        map_file: PathBuf,
        /// Input capture.
        input: PathBuf,
        /// Output capture.
        output: PathBuf,
        /// Reverse the mapping.
        #[arg(long)]
        reverse: bool,
        /// Behaviour for IPs not in the map.
        #[arg(long = "on-unknown", value_name = "MODE")]
        on_unknown: Option<String>,
        /// F5 trailer schema name.
        #[arg(long, value_name = "NAME")]
        schema: Option<String>,
        /// List known F5 trailer schemas and exit.
        #[arg(long = "list-schemas")]
        list_schemas: bool,
    },

    /// Dump the F5 command registry and event/profile/object graphs as JSON.
    #[command(visible_aliases = ["registrydump", "dump-registry"])]
    RegistryDump {
        /// Registry section to dump.
        #[arg(long, default_value = "all", value_parser = ["commands", "events", "profiles", "objects", "all"])]
        section: String,
    },

    /// Print a bash / fish / zsh completion script for the f5 CLI.
    Completion {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// iRule-specific subcommands.
    Irule {
        #[command(subcommand)]
        action: IruleCommand,
    },
}

impl Command {
    /// The canonical verb name, used in not-yet-implemented messages and logs.
    pub fn verb_name(&self) -> &'static str {
        match self {
            Command::Stats { .. } => "stats",
            Command::Cleanup { .. } => "cleanup",
            Command::Grep { .. } => "grep",
            Command::Extract { .. } => "extract",
            Command::Diff { .. } => "diff",
            Command::Split { .. } => "split",
            Command::Merge { .. } => "merge",
            Command::Rename { .. } => "rename",
            Command::Validate { .. } => "validate",
            Command::Explain { .. } => "explain",
            Command::Graph { .. } => "graph",
            Command::Tmsh { .. } => "tmsh",
            Command::Convert { .. } => "convert",
            Command::Query { .. } => "query",
            Command::Redact { .. } => "redact",
            Command::Unredact { .. } => "unredact",
            Command::Fetch { .. } => "fetch",
            Command::Push { .. } => "push",
            Command::Pull { .. } => "pull",
            Command::ExplainFlow { .. } => "explain-flow",
            Command::EnrichPcapng { .. } => "enrich-pcapng",
            Command::EnrichWireshark { .. } => "enrich-wireshark",
            Command::PcapRemap { .. } => "pcap-remap",
            Command::RegistryDump { .. } => "registry-dump",
            Command::Completion { .. } => "completion",
            Command::Irule { .. } => "irule",
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum IruleCommand {
    /// Show iRule events in canonical firing order.
    #[command(visible_alias = "eventorder")]
    EventOrder {
        inputs: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Look up event metadata and valid commands.
    #[command(visible_alias = "eventinfo")]
    EventInfo {
        /// Event name to query.
        event: String,
        #[arg(long)]
        json: bool,
    },
    /// Apply iRule-only lint rules.
    Lint {
        inputs: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Static event-flow trace from a starting event.
    Trace {
        inputs: Vec<PathBuf>,
        /// Starting event.
        #[arg(long, value_name = "EVENT")]
        start: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Profile-guided event-order report.
    Pgo {
        inputs: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Split a config / UCS into one file per iRule.
    Extract {
        inputs: Vec<PathBuf>,
        /// Output directory.
        #[arg(long, short, value_name = "DIR")]
        output: Option<PathBuf>,
    },
    /// Pretty-print iRule source.
    #[command(visible_alias = "fmt")]
    Format {
        inputs: Vec<PathBuf>,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Strip whitespace and comments from iRule source.
    #[command(visible_alias = "min")]
    Minify {
        inputs: Vec<PathBuf>,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Show the resolved command context for an iRule.
    Context {
        inputs: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}
