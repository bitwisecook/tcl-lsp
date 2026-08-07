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

//! `clap` derive definitions for the `f5-query` CLI command tree.
//!
//! Covers every verb (including the `irule` verb group). Verb names map to
//! kebab-cased variant names; aliases map to `visible_aliases`.
//!
//! Shared surfaces (`--format` scf/tmsh/tmsh-delta, remote credentials,
//! passphrase handling) are modelled as reusable `Args` structs; verb-specific
//! flags are defined per verb.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// jq-flavoured BIG-IP / iRules query and rewrite CLI.
#[derive(Debug, Parser)]
#[command(
    name = "f5-query",
    bin_name = "f5-query",
    // See tcl-cli: the release version comes from the tag, not the manifest.
    version = tcl_version::VERSION,
    about = "Inspect, transform, and query F5 BIG-IP configs and iRules.",
    disable_help_subcommand = true,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Output-format flag shared by config-producing verbs.
#[derive(Debug, Args)]
pub struct FormatArgs {
    /// Output rendering: SCF text, a tmsh script, or a tmsh delta.
    #[arg(long, default_value = "scf", value_parser = ["scf", "tmsh", "tmsh-delta"], value_name = "FORMAT")]
    pub format: String,

    /// Wrap the tmsh script in a cli transaction (requires a tmsh format).
    #[arg(long)]
    pub transaction: bool,
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

/// F5 unit master-key (`f5mku`) source flags shared by `encrypt-secrets` /
/// `decrypt-secrets` (the `_add_key_args` group
#[derive(Debug, Default, Args)]
pub struct MasterKeyArgs {
    /// base64 unit master key (the value `f5mku -K` prints on the device).
    /// Falls back to $F5MKU, then a secure terminal prompt.
    #[arg(short = 'k', long = "f5mku", value_name = "KEY")]
    pub f5mku: Option<String>,
    /// read the base64 master key from FILE (e.g. `f5mku -K > key.txt`).
    #[arg(long = "f5mku-file", value_name = "FILE")]
    pub f5mku_file: Option<PathBuf>,
    /// never prompt on the terminal for the master key; require a flag or
    /// $F5MKU instead.
    #[arg(long = "no-key-prompt")]
    pub no_key_prompt: bool,
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
        /// bigip.conf / SCF files (one or more). Pass `-` to read stdin.
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Treat the pattern as a regular expression.
        #[arg(long = "regex", short = 'e', group = "grep_mode")]
        regex: bool,
        /// Treat the pattern as a CIDR / IP match.
        #[arg(long = "cidr", short = 'c', group = "grep_mode")]
        cidr: bool,
        /// Traversal direction.
        #[arg(long, default_value = "both", value_parser = ["forward", "reverse", "both"])]
        direction: String,
        /// Stop the BFS after N hops from each search match.
        #[arg(long = "max-depth", value_name = "N")]
        max_depth: Option<usize>,
        /// Walk the related-object BFS from each search match (default).
        #[arg(long, short = 'r', overrides_with = "no_recurse")]
        recurse: bool,
        /// Skip the related-object BFS; report only the matched objects.
        #[arg(long = "no-recurse", overrides_with = "recurse")]
        no_recurse: bool,
        /// Cap the result at N objects (default: 1000).
        #[arg(long = "max-nodes", value_name = "N", default_value_t = 1000)]
        max_nodes: usize,
        /// Include each object's full body in the output.
        #[arg(long)]
        full: bool,
        /// Emit the grep report as JSON instead of the text report.
        #[arg(long)]
        json: bool,
        /// Write the report here (default: stdout).
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
        #[command(flatten)]
        format: FormatArgs,
        #[command(flatten)]
        passphrase: PassphraseArgs,
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
    // The `old` / `new` help strings are clap-visible plain text (no Markdown
    // backticks).
    #[allow(clippy::doc_markdown)]
    #[command(visible_alias = "mv")]
    Rename {
        /// Old full-path (e.g. /Common/old_pool).
        old: String,
        /// New full-path (e.g. /Common/new_pool).
        new: String,
        /// bigip.conf / SCF file (`-` for stdin).
        path: String,
        /// Write rewritten config here (default: stdout, dry-run).
        #[arg(long, short, value_name = "FILE", group = "rename_output")]
        output: Option<PathBuf>,
        /// Overwrite the input file with the rewritten config.
        #[arg(long = "in-place", group = "rename_output")]
        in_place: bool,
        /// Print rewritten config to stdout instead of a unified-diff preview.
        #[arg(long)]
        write: bool,
        #[command(flatten)]
        format: FormatArgs,
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
        /// bigip.conf / SCF file (`-` for stdin).
        path: PathBuf,
        /// Write the tmsh script here (default: stdout).
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
        /// Emit `modify` commands instead of `create`.
        #[arg(long)]
        modify: bool,
        /// Restrict to objects of this kind (repeatable).
        #[arg(long = "include", value_name = "KIND")]
        include: Vec<String>,
    },

    /// Convert between UCS / SCF / AS3 declaration formats.
    Convert {
        /// Conversion to perform.
        #[arg(value_parser = ["ucs2scf", "scf2as3"])]
        format: String,
        /// Input file (`-` for stdin; UCS must be a real file).
        path: String,
        /// Write here (default: stdout).
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
        /// (scf2as3) Tenant name in the AS3 declaration (default: Common).
        #[arg(long, default_value = "Common", value_name = "NAME")]
        tenant: String,
        /// (scf2as3) Application name in the AS3 declaration (default: app).
        #[arg(long, default_value = "app", value_name = "NAME")]
        application: String,
        /// (scf2as3) Print a coverage report on stderr listing unmapped objects.
        #[arg(long)]
        report: bool,
        #[command(flatten)]
        passphrase: PassphraseArgs,
    },

    /// jq-flavoured DSL for inspecting and rewriting BIG-IP configs.
    #[command(visible_alias = "q")]
    Query {
        /// The query expression. Use -f/--from-file to read it from a file.
        expression: Option<String>,
        inputs: Vec<PathBuf>,
        /// Read the query expression from FILE (mutually exclusive with passing
        /// the expression as the first positional argument).
        #[arg(short = 'f', long = "from-file", value_name = "FILE")]
        from_file: Option<String>,
        /// Bind a named config (NAME=PATH, repeatable).
        #[arg(long = "name", value_name = "NAME=PATH")]
        name: Vec<String>,
        /// Tell the loader which BIG-IP partition a source belongs to
        /// (PATH=PARTITION, or a bare PARTITION applied to every source).
        /// Repeatable; defaults to `Common`.
        #[arg(long = "partition", value_name = "PATH=PARTITION")]
        partition: Vec<String>,
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
        /// exit. Not yet available: the builtins prose catalogue is not
        /// implemented.
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
        /// bigip.conf / SCF file (`-` for stdin).
        path: String,
        /// Preserve original IP addresses (only redact secrets and PEM blocks).
        #[arg(long = "keep-ips")]
        keep_ips: bool,
        /// Target CIDR for remapped addresses (repeatable; mix v4 and v6).
        #[arg(long = "target-cidr", value_name = "CIDR")]
        target_cidr: Vec<String>,
        /// Permute host bits within each source CIDR via a deterministic key.
        #[arg(long)]
        shuffle: bool,
        /// Seed for --shuffle (deterministic; recorded in the map file).
        #[arg(long, value_name = "SEED", default_value = "")]
        seed: String,
        /// Also remap RFC1918 / `fc00::/7` (private) addresses.
        #[arg(long = "remap-private")]
        remap_private: bool,
        /// Write the redaction map TOML here.
        #[arg(long = "map-file", value_name = "FILE")]
        map_file: Option<PathBuf>,
        /// Pre-declare a source CIDR (repeatable).
        #[arg(long = "source-cidr", value_name = "CIDR")]
        source_cidr: Vec<String>,
        #[command(flatten)]
        format: FormatArgs,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Reverse a previous redact using its sidecar map file.
    #[command(visible_alias = "unmap")]
    Unredact {
        /// The TOML map file produced by `f5 redact` (`-` for stdin).
        map_file: String,
        /// Text containing redacted IPs (`-` for stdin).
        path: String,
        #[command(flatten)]
        format: FormatArgs,
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Encrypt clear-text secrets in a bigip.conf under the f5mku master key.
    #[command(name = "encrypt-secrets", visible_alias = "encrypt")]
    EncryptSecrets {
        /// bigip.conf / SCF file (`-` for stdin).
        path: String,
        /// Write the result here (default: stdout).
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
        #[command(flatten)]
        key: MasterKeyArgs,
        /// Force this two-character salt instead of a random one (deterministic
        /// output; mainly for testing).
        #[arg(long, value_name = "XX")]
        salt: Option<String>,
        #[command(flatten)]
        passphrase: PassphraseArgs,
        #[command(flatten)]
        format: FormatArgs,
    },

    /// Decrypt `$M$...` secrets in a bigip.conf with the f5mku master key.
    #[command(name = "decrypt-secrets", visible_alias = "decrypt")]
    DecryptSecrets {
        /// bigip.conf / SCF file (`-` for stdin).
        path: String,
        /// Write the result here (default: stdout).
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
        #[command(flatten)]
        key: MasterKeyArgs,
        #[command(flatten)]
        passphrase: PassphraseArgs,
        #[command(flatten)]
        format: FormatArgs,
    },

    /// Pull SCF or UCS from a live BIG-IP device (REST or SSH).
    // Help strings are clap-visible plain text (the env-var names carry
    // underscores), so no Markdown backticks.
    #[allow(clippy::doc_markdown)]
    #[command(visible_alias = "get")]
    Fetch {
        /// Device hostname / IP / alias (or set F5_HOST).
        #[arg(long, value_name = "HOST")]
        host: Option<String>,
        /// Username (or set F5_USER).
        #[arg(long, value_name = "USER")]
        user: Option<String>,
        /// Password (or set F5_PASSWORD).  Prompted interactively if absent.
        #[arg(long, value_name = "PASSWORD")]
        password: Option<String>,
        /// iControl REST HTTPS port (default 443).
        #[arg(long, value_name = "PORT")]
        port: Option<u16>,
        /// SSH port for SSH transport (default 22).
        #[arg(long = "ssh-port", value_name = "SSH_PORT")]
        ssh_port: Option<u16>,
        /// Transport: REST, SSH, or auto-fallback (default).
        #[arg(long, default_value = "auto", value_parser = ["auto", "rest", "ssh"])]
        transport: String,
        /// What to download (default: scf).
        #[arg(long = "format", default_value = "scf", value_parser = ["scf", "ucs", "both"], value_name = "FORMAT")]
        fmt: String,
        /// Skip TLS certificate verification (default: yes — BIG-IP self-signs).
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
        insecure: bool,
        /// Per-request timeout in seconds (default: 120).
        #[arg(long, default_value_t = 120.0, value_name = "SECS")]
        timeout: f64,
        /// Write artefacts to DIR.  Pass `-` to stream the SCF to stdout.
        #[arg(long, short = 'o', value_name = "DIR")]
        output: Option<String>,
        /// Fail rather than prompt for missing credentials.
        #[arg(long = "no-prompt")]
        no_prompt: bool,
        /// Print the cache directory path on stdout after a successful fetch.
        #[arg(long = "print-path")]
        print_path: bool,
    },

    /// Replace or create a single object on a live BIG-IP via iControl REST.
    Push {
        /// Object kind to push.
        #[arg(value_parser = ["virtual", "pool", "node", "rule"])]
        kind: String,
        /// Path to a JSON file (`-` for stdin) holding the iControl REST payload.
        payload: String,
        /// Device hostname / IP / alias.
        #[arg(long, value_name = "HOST")]
        host: Option<String>,
        /// Username.
        #[arg(long, value_name = "USER")]
        user: Option<String>,
        /// Password.
        #[arg(long, value_name = "PASSWORD")]
        password: Option<String>,
        /// HTTPS port (default 443).
        #[arg(long, value_name = "PORT")]
        port: Option<u16>,
        /// Fail rather than prompt.
        #[arg(long = "no-prompt")]
        no_prompt: bool,
        /// Skip TLS certificate verification (default: yes).
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
        insecure: bool,
        /// POST as a new object rather than PUT-replacing an existing one.
        #[arg(long)]
        create: bool,
        /// Show the payload and target endpoint without sending.
        #[arg(long = "dry-run")]
        dry_run: bool,
        /// Per-request timeout (default: 60s).
        #[arg(long, default_value_t = 60.0, value_name = "SECS")]
        timeout: f64,
    },

    /// Fetch a single object from a live BIG-IP.
    // The `full_path` example carries a slash path; keep it backtick-free so
    // the clap help stays plain text.
    #[allow(clippy::doc_markdown)]
    Pull {
        /// Object kind to fetch.
        #[arg(value_parser = ["virtual", "pool", "node", "rule"])]
        kind: String,
        /// Full path of the object (e.g. /Common/vs_app).
        full_path: String,
        /// Device hostname / IP / alias.
        #[arg(long, value_name = "HOST")]
        host: Option<String>,
        /// Username.
        #[arg(long, value_name = "USER")]
        user: Option<String>,
        /// Password.
        #[arg(long, value_name = "PASSWORD")]
        password: Option<String>,
        /// HTTPS port (default 443).
        #[arg(long, value_name = "PORT")]
        port: Option<u16>,
        /// Fail rather than prompt.
        #[arg(long = "no-prompt")]
        no_prompt: bool,
        /// Skip TLS certificate verification (default: yes).
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
        insecure: bool,
        /// Emit raw iControl REST JSON.
        #[arg(long)]
        json: bool,
        /// Per-request timeout (default: 60s).
        #[arg(long, default_value_t = 60.0, value_name = "SECS")]
        timeout: f64,
        #[command(flatten)]
        format: FormatArgs,
    },

    /// Trace each flow in a PCAP through the BIG-IP config.
    ExplainFlow {
        /// PCAP file to analyse (libpcap or pcapng).
        pcap: PathBuf,
        /// bigip.conf / SCF files (`-` for stdin).
        #[arg(required = true, value_name = "PATHS")]
        paths: Vec<PathBuf>,
        /// Enrich flows via tshark (HTTP method/Host/URI, TLS SNI/version/cipher).
        /// Requires `tshark` on PATH; silently no-ops otherwise.
        #[arg(long)]
        tshark: bool,
        /// NSS-format TLS keylog file (SSLKEYLOGFILE). Implies --tshark.
        #[arg(long, value_name = "FILE")]
        keylog: Option<PathBuf>,
        /// tshark display filter applied via -Y. Implies --tshark.
        #[arg(long = "tshark-filter", value_name = "FILTER")]
        tshark_filter: Option<String>,
        /// Run each matched session's iRule under the C-tcl orchestrator.
        #[arg(long)]
        simulate: bool,
        /// Suppress the verbatim `when EVENT { ... }` body for each fired event.
        #[arg(long = "no-event-bodies")]
        no_event_bodies: bool,
        /// Truncate each event body after N lines (default 40).
        #[arg(long = "max-event-lines", value_name = "N", default_value_t = 40)]
        max_event_lines: usize,
        /// Emit JSON instead of text.
        #[arg(long)]
        json: bool,
        /// Write report here (default: stdout).
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,
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

    /// Apply a `f5 redact` map to a PCAP capture (rewrites IP layer + parsed F5 trailer).
    #[command(visible_alias = "pcapmap")]
    PcapRemap {
        /// The TOML map file produced by `f5 redact`.
        map_file: PathBuf,
        /// Input PCAP file.
        input: PathBuf,
        /// Output PCAP file.
        output: PathBuf,
        /// Apply the map in reverse (recover originals from a redacted capture).
        #[arg(long)]
        reverse: bool,
        /// What to do when an F5 trailer TLV's (type, version) has no registered IP-field schema.
        #[arg(long = "on-unknown", value_parser = ["error", "preserve", "sweep"], default_value = "error")]
        on_unknown: String,
        /// Additional F5 trailer schema TOML to overlay on top of the built-ins (repeatable).
        #[arg(long, value_name = "FILE")]
        schema: Vec<PathBuf>,
        /// Print every registered trailer schema and exit.
        #[arg(long = "list-schemas")]
        list_schemas: bool,
    },

    /// Dump the F5 command registry and event/profile/object graphs as JSON.
    #[command(visible_aliases = ["registrydump", "dump-registry"])]
    RegistryDump {
        /// Registry section to dump.
        #[arg(long, default_value = "all", value_parser = ["commands", "events", "profiles", "objects", "all"])]
        section: String,
        /// Emit JSON (default and only format).
        #[arg(long)]
        json: bool,
        /// Output path ('-' for stdout).
        #[arg(long, short, value_name = "FILE", default_value = "-")]
        output: String,
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

/// Shared iRule input flags (`_add_irule_input_arguments`): a flexible mix of
/// `.tcl`/`.irul`/`.irule` files, bigip.conf/SCF, UCS, `--source` snippets, or
/// `-` for stdin, plus `--dialect` (default `f5-irules`) and `-o/--output`.
#[derive(Debug, Args)]
pub struct IruleInputArgs {
    /// Input files: .tcl/.irul/.irule, bigip.conf/SCF, or .ucs.  Pass `-` to read stdin.
    #[arg(value_name = "PATHS")]
    pub paths: Vec<String>,

    /// Inline iRule source text (can be repeated).
    #[arg(long = "source", value_name = "SOURCE")]
    pub source: Vec<String>,

    /// Dialect profile (default: f5-irules).
    #[arg(long, default_value = "f5-irules", value_name = "DIALECT")]
    pub dialect: String,

    /// Output path: '-' for stdout, a directory for one file per iRule, or any
    /// other path for a single concatenated file.
    #[arg(long, short, default_value = "-", value_name = "OUTPUT")]
    pub output: String,
}

/// Paired `--colour` / `--no-colour` toggle plus `--tabs`
/// (`_add_colour_arguments`).
#[derive(Debug, Args)]
pub struct IruleColourArgs {
    /// Disable syntax highlighting on stdout.
    #[arg(long = "no-colour", visible_alias = "no-color")]
    pub no_colour: bool,
    /// Force syntax highlighting even when stdout is not a TTY.
    #[arg(long = "colour", visible_alias = "color", overrides_with = "no_colour")]
    pub colour: bool,
    /// Expand tabs to N spaces on stdout (default: 4, 0 to keep tabs).
    #[arg(long, value_name = "N")]
    pub tabs: Option<usize>,
}

/// Formatter knobs (`_add_formatter_arguments`).
#[derive(Debug, Args)]
pub struct IruleFormatterArgs {
    /// Spaces per indent level (default: 4).
    #[arg(long = "indent-size", value_name = "N")]
    pub indent_size: Option<usize>,
    /// Indent using spaces or tabs (default: spaces).
    #[arg(long = "indent-style", value_parser = ["spaces", "tabs"])]
    pub indent_style: Option<String>,
    /// Hard line-length limit (default: 120).
    #[arg(long = "max-line-length", value_name = "N")]
    pub max_line_length: Option<usize>,
    /// Soft target line length (default: 100).
    #[arg(long = "goal-line-length", value_name = "N")]
    pub goal_line_length: Option<usize>,
    /// Expand compact single-line bodies.
    #[arg(long = "expand-bodies")]
    pub expand_bodies: bool,
    /// Replace semicolons with newlines (enabled by default).
    #[arg(long = "no-semicolons")]
    pub no_semicolons: bool,
    /// Keep semicolons as-is (do not replace with newlines).
    #[arg(long = "keep-semicolons")]
    pub keep_semicolons: bool,
}

#[derive(Debug, Subcommand)]
pub enum IruleCommand {
    /// Show iRules events in canonical firing order.
    #[command(visible_alias = "eventorder")]
    EventOrder {
        #[command(flatten)]
        input: IruleInputArgs,
        /// Emit event ordering as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Look up iRules event metadata and valid commands.
    // Help strings are clap-visible plain text (event names carry
    // underscores), so no Markdown backticks.
    #[allow(clippy::doc_markdown)]
    #[command(visible_alias = "eventinfo")]
    EventInfo {
        /// iRules event name (for example: HTTP_REQUEST).
        event: String,
        /// Target BIG-IP (TMOS) release for version-aware availability
        /// (defaults to the oldest supported release, 16.1.0).
        #[arg(long, value_name = "VERSION")]
        bigip_version: Option<String>,
        /// Emit event metadata as JSON.
        #[arg(long)]
        json: bool,
        /// Output path ('-' for stdout).
        #[arg(long, short, default_value = "-", value_name = "OUTPUT")]
        output: String,
    },
    /// Run iRule-only lint rules over iRule sources.
    Lint {
        #[command(flatten)]
        input: IruleInputArgs,
        /// Emit JSON instead of text.
        #[arg(long)]
        json: bool,
        /// Filter to one severity level.
        #[arg(long, value_parser = ["error", "warning", "info"])]
        severity: Option<String>,
    },
    /// Static event-flow trace from a starting event.
    // Help strings are clap-visible plain text (event names like HTTP_REQUEST
    // carry underscores), so keep them backtick-free.
    #[allow(clippy::doc_markdown)]
    Trace {
        /// Starting event name (e.g. HTTP_REQUEST).
        event: String,
        #[command(flatten)]
        input: IruleInputArgs,
        /// Emit JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Write each iRule body in a config to a standalone .tcl file.
    Extract {
        /// bigip.conf / SCF / UCS files (`-` for stdin).
        #[arg(required = true, value_name = "PATHS")]
        paths: Vec<String>,
        /// Output directory (created if needed).
        output: PathBuf,
    },
    /// Format iRule source and emit rewritten Tcl.
    #[command(visible_alias = "fmt")]
    Format {
        #[command(flatten)]
        input: IruleInputArgs,
        #[command(flatten)]
        colour: IruleColourArgs,
        #[command(flatten)]
        formatter: IruleFormatterArgs,
    },
    /// Minify iRule source: strip comments, collapse whitespace.
    #[command(visible_alias = "min")]
    Minify {
        #[command(flatten)]
        input: IruleInputArgs,
        /// Compact variable and proc names to short identifiers.
        #[arg(long)]
        compact: bool,
        /// Write symbol map (original -> compacted names) to FILE.
        #[arg(long = "symbol-map", value_name = "FILE")]
        symbol_map: Option<PathBuf>,
        /// Maximum compression: run all optimiser passes, then compact names and minify.
        #[arg(long)]
        aggressive: bool,
        /// Treat script as self-contained — also compact global-scope variable names.
        #[arg(long)]
        isolated: bool,
        #[command(flatten)]
        colour: IruleColourArgs,
    },
    /// Bundle each iRule with the BIG-IP objects it references.
    Context {
        #[command(flatten)]
        input: IruleInputArgs,
        /// Limit output to rules with these full paths (repeatable).
        #[arg(long, value_name = "FULL_PATH")]
        rule: Vec<String>,
        /// Skip the one-hop transitive expansion (pool->node, pool->monitor).
        #[arg(long = "no-transitive")]
        no_transitive: bool,
        /// Emit each bundle as a JSON object (default: Tcl-flavoured text).
        #[arg(long)]
        json: bool,
    },
}
