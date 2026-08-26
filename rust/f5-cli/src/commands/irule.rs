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

//! The `f5 irule` verb group — iRules-specific sub-subcommands.
//!
//! Every sub-subcommand is implemented, reusing an existing engine rather
//! than adding new analysis machinery:
//!
//! - `event-order` — firing-order event metadata from [`tcl_registry::events`].
//! - `extract` — per-rule bodies from the [`tcl_bigip`] model.
//! - `format` / `minify` — the [`tcl_lsp_core`] formatter / minifier engines,
//!   driven with the `f5-irules` dialect.
//! - `event-info` — event metadata + the `validCommands` cross-product over
//!   the reconciled command registry, `CommandRegistry::event_info`.
//! - `lint` — the [`tcl_bigip::lint`] iRule lint rules.
//! - `context` — [`tcl_bigip::irule_context`]'s cross-reference bundles.
//! - `trace` — a regex/reference extraction over the raw source (no CFG
//!   involved); see [`run_irule_trace`].
//!
//! `pgo` (profile-guided branch-reorder suggestions) is deliberately **not**
//! a sub-subcommand here: it would need a real branch-frequency profile
//! source (an F5 rule-profiler log format this crate has no reader for) and
//! a CFG the compiler crate exposes but `f5-cli` does not currently depend
//! on, to safely reorder `if`/`switch` arms without changing behaviour. That
//! is a standalone compiler feature, out of scope for a CLI-wiring fix — see
//! issue #1315.

use std::path::{Path, PathBuf};

use tcl_bigip::model::ModelObject;
use tcl_bigip::parser::{BigipConfig, Placed, parse_bigip_conf};
use tcl_bigip_io::paths::read_path;
use tcl_cli_support::registry_for_dialect;
use tcl_dialect::DialectProfile;
use tcl_lsp_core::formatting::{FormatterConfig, IndentStyle, formatting_with};
use tcl_lsp_core::minify::{minify_tcl, minify_tcl_aggressive, minify_tcl_compact};

use crate::cli::{IruleColourArgs, IruleCommand, IruleFormatterArgs, IruleInputArgs};

/// A single iRule body resolved from a CLI input.
struct IruleInput {
    /// Display label, used for diagnostics and synthesising output names.
    label: String,
    /// The iRule source body.
    source: String,
    /// Origin URI of the file / inline snippet this body came from (the dict
    /// key shared with [`LoadedIrules::configs`] / [`LoadedIrules::sources`]).
    // Consumed by `irule context`, which is not yet wired up, so it is
    // currently unused by `lint` / `format` / `minify`.
    #[allow(dead_code)]
    origin: String,
    /// BIG-IP full path when extracted from a config; `None` for standalone.
    rule_full_path: Option<String>,
}

/// The resolved iRule inputs plus the per-origin configs and post-decode
/// source text the lint / context verbs consume.
struct LoadedIrules {
    /// One body per `ltm rule` / standalone file / inline snippet, in load
    /// order.
    inputs: Vec<IruleInput>,
    /// One [`BigipConfig`] per input *file* / inline snippet, origin-keyed, in
    /// load order (inline snippets first, then paths). Standalone
    /// `.tcl`/`.irul`/`.irule` files and inline `--source` synthesise a
    /// single-rule config; `.conf`/`.scf`/UCS yield the parsed config.
    configs: Vec<(String, BigipConfig)>,
    /// The post-decode source text per origin (the *extracted* SCF for UCS),
    /// keyed identically to `configs`.
    // `lint` derives its `sources` from `configs` (joined rule bodies); this
    // full-text view is consumed by `irule context`, which is not yet wired up.
    #[allow(dead_code)]
    sources: Vec<(String, String)>,
}

/// The filename suffixes that name a **standalone** iRule file, as opposed to
/// a `bigip.conf` / SCF / UCS the rules are extracted from.
///
/// Projected from the dialect catalog rather than restated: the `f5-irules`
/// profile owns `irul`, `irule` and `irules`, and every editor registers all
/// three, but this list was hand-written with two of them — so `foo.irules`
/// was parsed as a BIG-IP config instead of an iRule (issue #1625). `tcl` is
/// added on top because the catalog deliberately leaves the generic extension
/// unowned (content decides the dialect there), while `f5-query irule` is
/// already in iRules context by the time it reads a file.
fn irule_suffixes() -> &'static [&'static str] {
    static SUFFIXES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    SUFFIXES.get_or_init(|| {
        std::iter::once("tcl")
            .chain(
                tcl_cli_support::environment::profile_for_dialect("f5-irules")
                    .file_extensions
                    .iter()
                    .map(|row| row.extension),
            )
            .collect()
    })
}

/// The container suffixes a standalone iRule is *extracted from* — the other
/// half of the input taxonomy, and not a dialect-catalog fact: `.conf` and
/// `.ucs` are deliberately not owned by any profile (a bare `.conf` belongs
/// to every unrelated config file), and `.scf` is `f5-bigip`'s.
const CONTAINER_SUFFIXES: &[&str] = &["conf", "scf", "ucs"];

fn suffix_lower(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// The final path component of *label* without its last suffix, falling back
/// to `"irule"` when empty.
fn label_stem(label: &str) -> String {
    Path::new(label)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "irule".to_owned())
}

/// Build a synthetic single-rule [`BigipConfig`]: one `ltm rule` at
/// `full_path`, with `default_partition = "Common"`.
fn synth_rule_config(name: &str, full_path: &str, source: &str) -> BigipConfig {
    let rule = tcl_bigip::model::BigipRule {
        name: name.to_owned(),
        full_path: full_path.to_owned(),
        source: source.to_owned(),
        ..Default::default()
    };
    BigipConfig {
        default_partition: "Common".to_owned(),
        objects: vec![Placed {
            table_name: "rules",
            full_path: full_path.to_owned(),
            object: ModelObject::Rule(rule),
        }],
        ..Default::default()
    }
}

/// Read each path's rules / synthesise a single-rule body, plus inline
/// `--source` snippets, returning the inputs alongside the origin-keyed
/// configs + post-decode sources.
fn load_irule_inputs(paths: &[String], inline_sources: &[String]) -> Result<LoadedIrules, String> {
    let mut inputs: Vec<IruleInput> = Vec::new();
    let mut configs: Vec<(String, BigipConfig)> = Vec::new();
    let mut sources: Vec<(String, String)> = Vec::new();

    for (index, source_text) in inline_sources.iter().enumerate() {
        let n = index + 1;
        let synth_name = format!("inline_{n}");
        let synth_path = format!("/{synth_name}");
        let origin = format!("inline://{n}");
        inputs.push(IruleInput {
            label: format!("<inline:{n}>"),
            source: source_text.clone(),
            origin: origin.clone(),
            rule_full_path: None,
        });
        configs.push((
            origin.clone(),
            synth_rule_config(&synth_name, &synth_path, source_text),
        ));
        sources.push((origin, source_text.clone()));
    }

    let opts = tcl_bigip_io::PassphraseOptions::default();
    for path_str in paths {
        let suffix = if path_str == "-" {
            String::new()
        } else {
            suffix_lower(path_str)
        };
        let (origin, text) = read_path(path_str, false, &opts).map_err(|e| e.to_string())?;
        let label = if path_str == "-" {
            "<stdin>".to_owned()
        } else {
            path_str.clone()
        };

        // Standalone iRule file: never parse as a bigip.conf — synthesise a
        // single-rule config at `/{stem}`.
        if irule_suffixes().contains(&suffix.as_str()) {
            let stem = label_stem(&label);
            let synth = synth_rule_config(&stem, &format!("/{stem}"), &text);
            inputs.push(IruleInput {
                label,
                source: text.clone(),
                origin: origin.clone(),
                rule_full_path: None,
            });
            configs.push((origin.clone(), synth));
            sources.push((origin, text));
            continue;
        }

        // bigip.conf / SCF / unknown / stdin text — sniff for `ltm rule`.
        let cfg = parse_bigip_conf(&text, "Common");
        let rules = collect_rules(&cfg);
        if rules.is_empty() {
            // No rules: treat the whole file as a single iRule body.
            let stem = label_stem(&label);
            let synth = synth_rule_config(&stem, &format!("/{stem}"), &text);
            inputs.push(IruleInput {
                label,
                source: text.clone(),
                origin: origin.clone(),
                rule_full_path: None,
            });
            configs.push((origin.clone(), synth));
            sources.push((origin, text));
        } else {
            for (rule_path, src) in rules {
                inputs.push(IruleInput {
                    label: format!("{label}::{rule_path}"),
                    source: src,
                    origin: origin.clone(),
                    rule_full_path: Some(rule_path),
                });
            }
            configs.push((origin.clone(), cfg));
            sources.push((origin, text));
        }
    }

    Ok(LoadedIrules {
        inputs,
        configs,
        sources,
    })
}

/// Collect `(full_path, source)` for every `ltm rule` in a parsed config, in
/// source order.
fn collect_rules(cfg: &tcl_bigip::parser::BigipConfig) -> Vec<(String, String)> {
    cfg.objects
        .iter()
        .filter(|p| p.table_name == "rules")
        .filter_map(|p| match &p.object {
            ModelObject::Rule(r) => Some((p.full_path.clone(), r.source.clone())),
            _ => None,
        })
        .collect()
}

/// Resolve iRule inputs with CLI error handling: prints the error and returns
/// the exit code on failure.
fn resolve_irule_inputs(input: &IruleInputArgs) -> Result<LoadedIrules, u8> {
    if input.paths.is_empty() && input.source.is_empty() {
        eprintln!("error: no input provided; pass files, --source, or `-` for stdin");
        return Err(2);
    }
    match load_irule_inputs(&input.paths, &input.source) {
        Ok(loaded) => Ok(loaded),
        Err(msg) => {
            eprintln!("error: {msg}");
            Err(2)
        }
    }
}

fn flatten_rule_path(rule_full_path: Option<&str>, fallback_label: &str) -> String {
    if let Some(p) = rule_full_path {
        return p.trim_start_matches('/').replace('/', "__");
    }
    let mut stem = Path::new(fallback_label)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if stem.is_empty() || stem == "<stdin>" || stem == "-" {
        "irule".clone_into(&mut stem);
    }
    let known: Vec<String> = irule_suffixes()
        .iter()
        .chain(CONTAINER_SUFFIXES)
        .map(|s| format!(".{s}"))
        .collect();
    if known.iter().any(|s| stem.ends_with(s.as_str()))
        && let Some(base) = Path::new(&stem).file_stem()
    {
        stem = base.to_string_lossy().into_owned();
    }
    stem.replace('/', "__")
}

/// Normalise a POSIX path string: collapse repeated `/`, drop `.` and
/// trailing slashes (keeping `..`); empty → `.`, root stays `/`. Used to
/// canonicalise the `irule context` output-directory message.
fn pathlib_str(s: &str) -> String {
    let absolute = s.starts_with('/');
    let parts: Vec<&str> = s
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != ".")
        .collect();
    if parts.is_empty() {
        return if absolute {
            "/".to_owned()
        } else {
            ".".to_owned()
        };
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

/// Decide whether `output` should be treated as a directory.
fn is_directory_target(output: &str) -> bool {
    if output == "-" {
        return false;
    }
    Path::new(output).is_dir() || output.ends_with('/') || output.ends_with('\\')
}

/// Common output dispatcher for irule format / minify.
fn write_irule_outputs(
    output: &str,
    inputs: &[IruleInput],
    transformed: &[String],
    file_extension: &str,
    colour: Option<&IruleColourArgs>,
) -> Result<u8, String> {
    if is_directory_target(output) {
        let out_dir = Path::new(output);
        std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
        for (entry, text) in inputs.iter().zip(transformed.iter()) {
            let stem = flatten_rule_path(entry.rule_full_path.as_deref(), &entry.label);
            let target = out_dir.join(format!("{stem}{file_extension}"));
            let body = if text.ends_with('\n') {
                text.clone()
            } else {
                format!("{text}\n")
            };
            std::fs::write(&target, body).map_err(|e| e.to_string())?;
        }
        eprintln!("wrote {} iRule(s) to {}", transformed.len(), output);
        return Ok(0);
    }

    // Single-file / stdout: emit the concatenation.
    let text = if transformed.len() <= 1 {
        transformed.first().cloned().unwrap_or_default()
    } else {
        let mut chunks: Vec<String> = Vec::with_capacity(transformed.len());
        for (entry, body) in inputs.iter().zip(transformed.iter()) {
            let label = entry
                .rule_full_path
                .clone()
                .unwrap_or_else(|| entry.label.clone());
            chunks.push(format!("# ===== {label} =====\n{}\n", body.trim_end()));
        }
        chunks.join("\n")
    };
    write_highlighted(output, &text, colour)?;
    Ok(0)
}

/// Resolve the effective tab width.
fn resolve_tab_width(colour: Option<&IruleColourArgs>) -> usize {
    colour.and_then(|c| c.tabs).unwrap_or(4)
}

/// Whether ANSI colour should be applied.
fn resolve_use_colour(output: &str, colour: Option<&IruleColourArgs>) -> bool {
    use tcl_cli_support::OutputTarget;
    let (force, no) = colour.map_or((false, false), |c| (c.colour, c.no_colour));
    let target = if output == "-" {
        OutputTarget::Stdout
    } else {
        OutputTarget::File(PathBuf::from(output))
    };
    tcl_cli_support::resolve_use_colour(force, no, &target)
}

/// Write `text` with optional highlighting / tab expansion to `output`.
fn write_highlighted(
    output: &str,
    text: &str,
    colour: Option<&IruleColourArgs>,
) -> Result<(), String> {
    use tcl_cli_support::OutputTarget;
    let use_colour = resolve_use_colour(output, colour);
    let tab_width = resolve_tab_width(colour);
    let target = if output == "-" {
        OutputTarget::Stdout
    } else {
        OutputTarget::File(PathBuf::from(output))
    };
    tcl_cli_support::write_highlighted_output(
        &target,
        text,
        use_colour,
        tab_width,
        tcl_cli_support::environment::profile_for_dialect("f5-irules"),
    )
    .map_err(|e| e.to_string())
}

/// Write plain text to `output`.
fn write_text(output: &str, text: &str) -> Result<(), String> {
    use tcl_cli_support::OutputTarget;
    let target = if output == "-" {
        OutputTarget::Stdout
    } else {
        OutputTarget::File(PathBuf::from(output))
    };
    tcl_cli_support::write_text_output(&target, text).map_err(|e| e.to_string())
}

/// Dispatch the `irule` verb group.
///
/// Returns `anyhow::Result<u8>` to slot into the top-level `dispatch` match
/// uniformly; the handlers print their own errors and resolve to an exit code,
/// so this never returns `Err`.
#[allow(clippy::unnecessary_wraps)]
pub fn run_irule(action: &IruleCommand) -> anyhow::Result<u8> {
    let rc = match action {
        IruleCommand::EventOrder { input, json } => run_event_order(input, *json),
        IruleCommand::Extract { paths, output } => run_extract(paths, output),
        IruleCommand::Format {
            input,
            colour,
            formatter,
        } => run_format(input, colour, formatter),
        IruleCommand::Minify {
            input,
            compact,
            symbol_map,
            aggressive,
            isolated,
            colour,
        } => run_minify(
            input,
            *compact,
            symbol_map.as_deref(),
            *aggressive,
            *isolated,
            colour,
        ),
        IruleCommand::EventInfo {
            event,
            json,
            output,
            bigip_version,
        } => run_event_info(event, bigip_version.as_deref(), *json, output),
        IruleCommand::Lint {
            input,
            json,
            severity,
        } => run_irule_lint(input, *json, severity.as_deref()),
        IruleCommand::Context {
            input,
            rule,
            no_transitive,
            json,
        } => run_irule_context(input, rule, *no_transitive, *json),
        IruleCommand::Trace { event, input, json } => run_irule_trace(event, input, *json),
    };
    // Both the success path and the printed-error path resolve to a process
    // exit code.
    Ok(rc.unwrap_or_else(|code| code))
}

// event-order

/// Scan `when EVENT` blocks and return events in canonical firing order.
fn order_events_for_file(
    source: &str,
    events: &tcl_registry::events::EventRegistry,
) -> Vec<String> {
    let found: std::collections::BTreeSet<String> = tcl_irules::when_blocks(source)
        .into_iter()
        .map(|block| block.event)
        .collect();

    // order_events: known events by master-order index, unknown sorted after.
    let order: Vec<&str> = events.master_order().iter().map(|o| o.event).collect();
    let mut known: Vec<(usize, String)> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    for evt in &found {
        if let Some(idx) = order.iter().position(|e| *e == evt) {
            known.push((idx, evt.clone()));
        } else {
            unknown.push(evt.clone());
        }
    }
    known.sort();
    unknown.sort();
    known.into_iter().map(|(_, e)| e).chain(unknown).collect()
}

/// Multiplicity category for a single event.
fn event_multiplicity(name: &str, events: &tcl_registry::events::EventRegistry) -> &'static str {
    if name == "RULE_INIT" {
        "init"
    } else if events.is_once_per_connection(name) {
        "once_per_connection"
    } else if events.is_per_request(name) {
        "per_request"
    } else {
        "unknown"
    }
}

fn run_event_order(input: &IruleInputArgs, json: bool) -> Result<u8, u8> {
    let loaded = resolve_irule_inputs(input)?;
    let events = tcl_registry::events::EventRegistry::build();

    let combined: String = loaded
        .inputs
        .iter()
        .map(|e| e.source.trim_end_matches('\n'))
        .collect::<Vec<_>>()
        .join("\n\n");

    let ordered = order_events_for_file(&combined, &events);
    let items: Vec<(usize, String, &str)> = ordered
        .iter()
        .enumerate()
        .map(|(i, name)| (i + 1, name.clone(), event_multiplicity(name, &events)))
        .collect();

    if json {
        use std::fmt::Write as _;
        // Emit 2-space-indented JSON with insertion-ordered keys (no sort).
        let mut out = String::from("{\n");
        let _ = writeln!(out, "  \"count\": {},", items.len());
        out.push_str("  \"dialect\": ");
        out.push_str(&json_string(&input.dialect));
        out.push_str(",\n  \"events\": ");
        if items.is_empty() {
            out.push_str("[]");
        } else {
            out.push_str("[\n");
            for (i, (index, name, mult)) in items.iter().enumerate() {
                out.push_str("    {\n");
                let _ = writeln!(out, "      \"index\": {index},");
                out.push_str("      \"name\": ");
                out.push_str(&json_string(name));
                out.push_str(",\n      \"multiplicity\": ");
                out.push_str(&json_string(mult));
                out.push_str("\n    }");
                if i + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("  ]");
        }
        out.push_str("\n}");
        write_text(&input.output, &out).map_err(|_| 1u8)?;
        return Ok(0);
    }

    let mut lines = vec![format!("event order: {} event(s)", items.len())];
    for (index, name, mult) in &items {
        lines.push(format!("  {index}. {name} ({mult})"));
    }
    write_text(&input.output, &lines.join("\n")).map_err(|_| 1u8)?;
    Ok(0)
}

// event-info

/// `f5 irule event-info EVENT` — look up event metadata + valid commands
/// over the reconciled command-registry cross-product
/// (`CommandRegistry::event_info`). Exit code is `0` for a known event, `1`
/// otherwise (both streams written).
fn run_event_info(
    event: &str,
    bigip_version: Option<&str>,
    json: bool,
    output: &str,
) -> Result<u8, u8> {
    // The profile-stamped registry: the §9 operator-head exclusion applies
    // inside the event/command cross-product, and availability otherwise
    // comes from each spec's own `dialects` group — a raw `build_default`
    // registry would re-admit commands that carry no `IRULES` bit.
    let cmds = tcl_registry::model::static_context_for("f5-irules").commands();
    let events = tcl_registry::events::EventRegistry::build();
    let profiles = tcl_registry::profiles::ProfileRegistry::build();
    let info = cmds.event_info(event, &events, &profiles, bigip_version);
    let rc = u8::from(!info.known);

    if json {
        use std::fmt::Write as _;
        // Emit 2-space-indented JSON with insertion-ordered keys.
        let mut out = String::from("{\n");
        let _ = write!(out, "  \"event\": {}", json_string(&info.event));
        let _ = write!(out, ",\n  \"known\": {}", info.known);
        let _ = write!(
            out,
            ",\n  \"lifecycleState\": {}",
            json_string(info.lifecycle_state.as_str())
        );
        let _ = write!(
            out,
            ",\n  \"multiplicity\": {}",
            json_string(info.multiplicity)
        );
        let _ = write!(
            out,
            ",\n  \"description\": {}",
            json_string(&info.description)
        );
        let _ = write!(out, ",\n  \"side\": {}", json_string(info.side));
        out.push_str(",\n  \"transport\": ");
        match &info.transport {
            Some(t) => out.push_str(&json_string(t)),
            None => out.push_str("null"),
        }
        out.push_str(",\n  \"impliedProfiles\": ");
        push_json_string_array(&mut out, info.implied_profiles.iter().copied());
        // The three lifecycle releases, same names and null semantics as
        // every other registry surface; `retiredVersion` is exclusive.
        for (key, value) in [
            ("introducedVersion", info.lifecycle.introduced),
            ("deprecatedVersion", info.lifecycle.deprecated),
            ("retiredVersion", info.lifecycle.retired),
        ] {
            let _ = write!(out, ",\n  \"{key}\": ");
            match value {
                Some(v) => out.push_str(&json_string(v)),
                None => out.push_str("null"),
            }
        }
        let _ = write!(
            out,
            ",\n  \"validCommandCount\": {}",
            info.valid_command_count()
        );
        out.push_str(",\n  \"validCommands\": ");
        push_json_string_array(&mut out, info.valid_commands.iter().map(String::as_str));
        out.push_str("\n}");
        write_text(output, &out).map_err(|_| 1u8)?;
        return Ok(rc);
    }

    let mut lines = vec![
        format!("event: {}", info.event),
        format!("known: {}", if info.known { "yes" } else { "no" }),
        format!("lifecycle: {}", info.lifecycle_state.as_str()),
        format!("multiplicity: {}", info.multiplicity),
    ];
    if !info.description.is_empty() {
        lines.push(format!("description: {}", info.description));
    }
    lines.push(format!("side: {}", info.side));
    if let Some(t) = info.transport.as_deref().filter(|t| !t.is_empty()) {
        lines.push(format!("transport: {t}"));
    }
    if !info.implied_profiles.is_empty() {
        lines.push(format!("profiles: {}", info.implied_profiles.join(", ")));
    }
    lines.push(format!("valid commands: {}", info.valid_command_count()));
    write_text(output, &lines.join("\n")).map_err(|_| 1u8)?;
    Ok(rc)
}

/// Append a JSON array of strings formatted as 2-space-indented JSON at a
/// top-level key: `[]` when empty, else one item per line indented four
/// spaces with the closing bracket at two.
fn push_json_string_array<'a>(out: &mut String, items: impl Iterator<Item = &'a str>) {
    let rendered: Vec<String> = items.map(json_string).collect();
    if rendered.is_empty() {
        out.push_str("[]");
        return;
    }
    out.push_str("[\n");
    for (i, item) in rendered.iter().enumerate() {
        out.push_str("    ");
        out.push_str(item);
        if i + 1 < rendered.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]");
}

// lint

/// `f5 irule lint` — run only the iRule-category lint rules over iRule sources.
/// Shares the [`tcl_bigip::lint`] engine and the `f5 validate` output
/// formatters ([`crate::commands::validate::to_json`] / [`to_text`]), passing
/// `category="irule"` so only the four irule rules run.
///
/// Exit code: any `error` finding → `2`, any `warning` → `1`, else `0`.
///
/// [`to_text`]: crate::commands::validate::to_text
fn run_irule_lint(input: &IruleInputArgs, json: bool, severity: Option<&str>) -> Result<u8, u8> {
    use tcl_bigip::lint::run_lint;

    use crate::commands::validate;

    let loaded = resolve_irule_inputs(input)?;

    // The iRule lint category only inspects rule bodies, so the `sources`
    // argument is the rule bodies joined back together by origin URI (the
    // built-in rules ignore it, but pass it to satisfy the signature).
    let sources_for_lint: Vec<(String, String)> = loaded
        .configs
        .iter()
        .map(|(origin, cfg)| {
            let joined = collect_rules(cfg)
                .into_iter()
                .map(|(_path, src)| src)
                .collect::<Vec<_>>()
                .join("\n");
            (origin.clone(), joined)
        })
        .collect();

    let config_refs: Vec<(String, &BigipConfig)> = loaded
        .configs
        .iter()
        .map(|(origin, cfg)| (origin.clone(), cfg))
        .collect();
    let source_refs: Vec<(String, &str)> = sources_for_lint
        .iter()
        .map(|(origin, src)| (origin.clone(), src.as_str()))
        .collect();

    let findings = run_lint(&config_refs, &source_refs, Some("irule"), severity);

    // `to_json` / `to_text` already include the trailing newline.
    let out = if json {
        validate::to_json(&findings)
    } else {
        validate::to_text(&findings)
    };
    write_text(&input.output, &out).map_err(|_| 2u8)?;

    let has_error = findings.iter().any(|f| f.severity == "error");
    let has_warning = findings.iter().any(|f| f.severity == "warning");
    if has_error {
        Ok(2)
    } else if has_warning {
        Ok(1)
    } else {
        Ok(0)
    }
}

// context

/// `f5 irule context` — bundle each iRule with the BIG-IP objects it
/// references, using the [`tcl_bigip::irule_context`] engine: resolve inputs,
/// merge the configs once for cross-file resolution, build one bundle per
/// rule, and render to JSON / Tcl-flavoured text (directory → one file per
/// rule; otherwise a single concatenated stream). Exit `0`, or `1` when no
/// iRules were found.
fn run_irule_context(
    input: &IruleInputArgs,
    rule_filter: &[String],
    no_transitive: bool,
    json: bool,
) -> Result<u8, u8> {
    use tcl_bigip::irule_context::{
        build_irule_context, bundles_to_json, context_bundle_to_json, context_bundle_to_text,
        origin_source,
    };

    let loaded = resolve_irule_inputs(input)?;

    // Merge configs once so cross-file references all resolve (empty →
    // default, else the merged union).
    let config_refs: Vec<(String, &tcl_bigip::parser::BigipConfig)> = loaded
        .configs
        .iter()
        .map(|(origin, cfg)| (origin.clone(), cfg))
        .collect();
    let merged = tcl_bigip::lint::merge_configs(&config_refs);

    let registry = tcl_registry::model::static_context_for("f5-irules").commands();

    let filter: std::collections::HashSet<&str> = rule_filter.iter().map(String::as_str).collect();
    let transitive = !no_transitive;

    // (rule_full_path, bundle) per rule, in config-then-source order.
    let mut bundles: Vec<(String, tcl_bigip::irule_context::IruleContextBundle)> = Vec::new();
    for (origin, cfg) in &loaded.configs {
        let src_text = origin_source(&loaded.sources, Some(origin));
        for placed in &cfg.objects {
            if placed.table_name != "rules" {
                continue;
            }
            let ModelObject::Rule(rule) = &placed.object else {
                continue;
            };
            if !filter.is_empty() && !filter.contains(placed.full_path.as_str()) {
                continue;
            }
            let bundle = build_irule_context(rule, &merged, transitive, src_text, registry);
            bundles.push((placed.full_path.clone(), bundle));
        }
    }

    if bundles.is_empty() {
        eprintln!("error: no iRules found in input");
        return Err(1);
    }

    if is_directory_target(&input.output) {
        let out_dir = Path::new(&input.output);
        if let Err(e) = std::fs::create_dir_all(out_dir) {
            eprintln!("error: {e}");
            return Err(2);
        }
        let suffix = if json { ".json" } else { ".tcl" };
        for (rule_path, bundle) in &bundles {
            let flat = rule_path.trim_start_matches('/').replace('/', "__");
            let payload = if json {
                format!("{}\n", context_bundle_to_json(bundle))
            } else {
                context_bundle_to_text(bundle)
            };
            let target = out_dir.join(format!("{flat}{suffix}"));
            if let Err(e) = std::fs::write(&target, payload) {
                eprintln!("error: {e}");
                return Err(2);
            }
        }
        eprintln!(
            "wrote {} iRule context bundle(s) to {}",
            bundles.len(),
            pathlib_str(&input.output)
        );
        return Ok(0);
    }

    // Single-file / stdout: concatenate.
    let payload = if json {
        let only: Vec<tcl_bigip::irule_context::IruleContextBundle> =
            bundles.into_iter().map(|(_, b)| b).collect();
        format!("{}\n", bundles_to_json(&only))
    } else {
        let chunks: Vec<String> = bundles
            .iter()
            .map(|(_, b)| format!("{}\n", context_bundle_to_text(b).trim_end()))
            .collect();
        chunks.join("\n")
    };
    write_text(&input.output, &payload).map_err(|_| 2u8)?;
    Ok(0)
}

// trace

/// One resolved object reference in a trace.
struct TraceRef {
    kind: &'static str,
    name: String,
    command: String,
    resolved_path: Option<String>,
}

/// One event-handler trace: the commands the body runs and the objects it
/// references.
struct Trace {
    rule: String,
    commands: Vec<String>,
    references: Vec<TraceRef>,
}

/// `f5 irule trace EVENT` — static trace of a single event handler: the
/// commands it runs and the BIG-IP objects it references. Purely static — no
/// VM / simulator. Exit `0` when at least one rule has a matching `when EVENT`
/// block, else `1`.
fn run_irule_trace(event: &str, input: &IruleInputArgs, json: bool) -> Result<u8, u8> {
    use tcl_bigip::irule_context::{classify_kind, resolve_reference};
    use tcl_irules::{
        extract_irules_object_references_in_closure, irules_event_executable_closure,
    };

    let loaded = resolve_irule_inputs(input)?;

    let config_refs: Vec<(String, &tcl_bigip::parser::BigipConfig)> = loaded
        .configs
        .iter()
        .map(|(origin, cfg)| (origin.clone(), cfg))
        .collect();
    let merged = tcl_bigip::lint::merge_configs(&config_refs);

    let registry = tcl_registry::model::static_context_for("f5-irules").commands();

    let mut traces: Vec<Trace> = Vec::new();
    for entry in &loaded.inputs {
        if !tcl_irules::when_blocks(&entry.source)
            .into_iter()
            .any(|block| block.event.eq_ignore_ascii_case(event))
        {
            continue;
        }
        let executable = irules_event_executable_closure(&entry.source, event, registry);
        let commands = executable
            .iter()
            .map(|command| command.command.clone())
            .collect();
        let rule_label = entry
            .rule_full_path
            .clone()
            .unwrap_or_else(|| entry.label.clone());

        let mut references: Vec<TraceRef> = Vec::new();
        let mut seen: std::collections::HashSet<(&str, String)> = std::collections::HashSet::new();
        // The closure is the execution owner for both the command inventory
        // and object references.  No physical-body range check can express a
        // reached helper procedure (or distinguish a dormant one) correctly.
        for reference in
            extract_irules_object_references_in_closure(&entry.source, None, registry, &executable)
        {
            let Some(kind) = classify_kind(&reference.kinds) else {
                continue;
            };
            if !seen.insert((kind, reference.name.clone())) {
                continue;
            }
            let resolved_path = resolve_reference(&merged, kind, &reference.name);
            references.push(TraceRef {
                kind,
                name: reference.name,
                command: reference.command,
                resolved_path,
            });
        }

        traces.push(Trace {
            rule: rule_label,
            commands,
            references,
        });
    }

    let out = if json {
        trace_json(event, &traces)
    } else {
        let mut lines = vec![format!("event {event}: {} matching rule(s)", traces.len())];
        for t in &traces {
            lines.push(format!("  {} — {} command(s)", t.rule, t.commands.len()));
            for cmd in &t.commands {
                lines.push(format!("    {cmd}"));
            }
            for r in &t.references {
                let marker = if r.resolved_path.is_some() {
                    "✓"
                } else {
                    "✗"
                };
                let target = r.resolved_path.as_ref().unwrap_or(&r.name);
                lines.push(format!("    {marker} {}: {target}", r.kind));
            }
        }
        format!("{}\n", lines.join("\n"))
    };
    write_text(&input.output, &out).map_err(|_| 2u8)?;
    Ok(u8::from(traces.is_empty()))
}

/// Render the trace payload `{"event": …, "traces": [...]}` as 2-space-indented
/// JSON with insertion-ordered keys and a trailing newline.
fn trace_json(event: &str, traces: &[Trace]) -> String {
    use std::fmt::Write as _;

    let mut out = String::from("{\n");
    let _ = write!(out, "  \"event\": {}", json_string(event));
    out.push_str(",\n  \"traces\": ");
    if traces.is_empty() {
        out.push_str("[]\n}\n");
        return out;
    }
    out.push_str("[\n");
    for (ti, t) in traces.iter().enumerate() {
        out.push_str("    {\n");
        let _ = writeln!(out, "      \"rule\": {},", json_string(&t.rule));
        let _ = writeln!(out, "      \"commandCount\": {},", t.commands.len());
        out.push_str("      \"commands\": ");
        push_str_array(&mut out, &t.commands, 6);
        out.push_str(",\n      \"references\": ");
        if t.references.is_empty() {
            out.push_str("[]");
        } else {
            out.push_str("[\n");
            for (ri, r) in t.references.iter().enumerate() {
                out.push_str("        {\n");
                let _ = writeln!(out, "          \"kind\": {},", json_string(r.kind));
                let _ = writeln!(out, "          \"name\": {},", json_string(&r.name));
                let _ = writeln!(out, "          \"command\": {},", json_string(&r.command));
                let _ = writeln!(
                    out,
                    "          \"resolved\": {},",
                    r.resolved_path.is_some()
                );
                out.push_str("          \"resolvedPath\": ");
                match &r.resolved_path {
                    Some(p) => out.push_str(&json_string(p)),
                    None => out.push_str("null"),
                }
                out.push_str("\n        }");
                if ri + 1 < t.references.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("      ]");
        }
        out.push_str("\n    }");
        if ti + 1 < traces.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

/// A JSON array of strings at `indent` spaces: `[]` when empty, else one item
/// per line (`indent + 2`) with the bracket back at `indent`.
fn push_str_array(out: &mut String, items: &[String], indent: usize) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    let pad = " ".repeat(indent + 2);
    out.push_str("[\n");
    for (i, item) in items.iter().enumerate() {
        out.push_str(&pad);
        out.push_str(&json_string(item));
        if i + 1 < items.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(&" ".repeat(indent));
    out.push(']');
}

// extract

fn run_extract(paths: &[String], output: &Path) -> Result<u8, u8> {
    if paths.is_empty() {
        eprintln!("error: no input provided; pass bigip.conf / SCF / UCS files");
        return Err(2);
    }
    let bad: Vec<&String> = paths
        .iter()
        .filter(|p| *p != "-" && irule_suffixes().contains(&suffix_lower(p).as_str()))
        .collect();
    if !bad.is_empty() {
        let joined = bad
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!(
            "error: extract only accepts bigip.conf / SCF / UCS; refusing standalone \
             iRule file(s): {joined}"
        );
        return Err(2);
    }

    let opts = tcl_bigip_io::PassphraseOptions::default();
    let mut all_rules: Vec<Vec<(String, String)>> = Vec::new();
    for p in paths {
        let (_origin, text) = match read_path(p, false, &opts) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: {e}");
                return Err(2);
            }
        };
        let cfg = parse_bigip_conf(&text, "Common");
        all_rules.push(collect_rules(&cfg));
    }

    if let Err(e) = std::fs::create_dir_all(output) {
        eprintln!("error: {e}");
        return Err(2);
    }

    let mut written = 0;
    for rules in &all_rules {
        for (rule_path, source) in rules {
            let flat = rule_path.trim_start_matches('/').replace('/', "__");
            let target = output.join(format!("{flat}.tcl"));
            if let Err(e) = std::fs::write(&target, format!("{source}\n")) {
                eprintln!("error: {e}");
                return Err(2);
            }
            written += 1;
        }
    }

    eprintln!("extracted {written} iRule(s) to {}", output.display());
    Ok(0)
}

// format / minify

/// The formatter knobs from the command line, aimed at `profile`.
///
/// The profile is the formatter's whole dialect story (issue #1465): with
/// `--dialect f5-irules` (this command's default, and its `irules` /
/// `tcl-irule` alias spellings) the formatter tokenises with the iRules
/// grammar, so an iRule's `}{` re-emits as `} {` and a `{*}` stays the
/// literal braced word TMM's 8.4 core reads it as. Starting from
/// `FormatterConfig::default()` instead formatted every iRule with the
/// modern Tcl 9 lexer.
fn build_formatter_config(
    formatter: &IruleFormatterArgs,
    profile: &'static DialectProfile,
) -> FormatterConfig {
    let mut config = FormatterConfig::for_profile(profile);
    if let Some(size) = formatter.indent_size {
        config.indent_size = size;
    }
    if let Some(style) = formatter.indent_style.as_deref() {
        config.indent_style = if style == "tabs" {
            IndentStyle::Tabs
        } else {
            IndentStyle::Spaces
        };
    }
    if let Some(max) = formatter.max_line_length {
        config.max_line_length = max;
    }
    if let Some(goal) = formatter.goal_line_length {
        config.goal_line_length = goal;
    }
    if formatter.expand_bodies {
        config.expand_single_line_bodies = true;
    }
    if formatter.no_semicolons {
        config.replace_semicolons_with_newlines = true;
    }
    if formatter.keep_semicolons {
        config.replace_semicolons_with_newlines = false;
    }
    config
}

fn run_format(
    input: &IruleInputArgs,
    colour: &IruleColourArgs,
    formatter: &IruleFormatterArgs,
) -> Result<u8, u8> {
    let loaded = resolve_irule_inputs(input)?;
    // One resolved profile drives both the registry and the formatter, so the
    // command table and the lexer can never disagree about the dialect
    // (`registry_for_dialect` resolves the same profile by name).
    let profile = tcl_cli_support::environment::analyser_profile_for_dialect(&input.dialect);
    let registry = registry_for_dialect(&input.dialect);
    let config = build_formatter_config(formatter, profile);

    let rendered: Vec<String> = loaded
        .inputs
        .iter()
        .map(|entry| {
            let edits = formatting_with(&entry.source, &config, &registry);
            edits
                .into_iter()
                .next()
                .map_or_else(|| entry.source.clone(), |edit| edit.new_text)
        })
        .collect();

    write_irule_outputs(
        &input.output,
        &loaded.inputs,
        &rendered,
        ".irule",
        Some(colour),
    )
    .map_err(|_| 2u8)
}

fn run_minify(
    input: &IruleInputArgs,
    compact: bool,
    symbol_map: Option<&Path>,
    aggressive: bool,
    isolated: bool,
    colour: &IruleColourArgs,
) -> Result<u8, u8> {
    let loaded = resolve_irule_inputs(input)?;
    let profile = tcl_lsp_core::profile_for_dialect(&input.dialect);
    let registry = registry_for_dialect(&input.dialect);

    let mut minified: Vec<String> = Vec::with_capacity(loaded.inputs.len());
    let mut symbol_maps: Vec<String> = Vec::new();
    if aggressive {
        for entry in &loaded.inputs {
            let result = minify_tcl_aggressive(&entry.source, profile, isolated, &registry);
            minified.push(result.source);
            symbol_maps.push(result.symbol_map.format());
        }
    } else if compact {
        for entry in &loaded.inputs {
            let (text, sm) = minify_tcl_compact(&entry.source, profile, isolated, &registry);
            minified.push(text);
            symbol_maps.push(sm.format());
        }
    } else {
        for entry in &loaded.inputs {
            minified.push(minify_tcl(&entry.source, profile, &registry));
        }
    }

    let rc = write_irule_outputs(
        &input.output,
        &loaded.inputs,
        &minified,
        ".irule",
        Some(colour),
    )
    .map_err(|_| 2u8)?;
    if rc != 0 {
        return Ok(rc);
    }

    if let Some(map_path) = symbol_map
        && !symbol_maps.is_empty()
    {
        let mut blocks: Vec<String> = Vec::new();
        for (entry, m) in loaded.inputs.iter().zip(symbol_maps.iter()) {
            if m.is_empty() {
                continue;
            }
            let label = entry
                .rule_full_path
                .clone()
                .unwrap_or_else(|| entry.label.clone());
            blocks.push(format!("# {label}\n{m}").trim_end().to_owned());
        }
        let joined = blocks.join("\n\n");
        let text = if joined.is_empty() {
            String::new()
        } else {
            format!("{joined}\n")
        };
        let map_str = map_path.to_string_lossy();
        write_text(&map_str, &text).map_err(|_| 2u8)?;
    }
    Ok(0)
}

/// Render a JSON-quoted string (reuses [`tcl_bigip::jsonfmt`]).
fn json_string(s: &str) -> String {
    tcl_bigip::jsonfmt::json_string(s)
}
