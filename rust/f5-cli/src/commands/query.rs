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

//! The `query` verb — jq-flavoured DSL over BIG-IP configs (read-only).
//!
//! Loads each input to `(uri, source)` via the UCS/stdin-aware loader, runs
//! the pure `tcl_bigip_query::run_query` runner, and renders the per-file
//! values with `tcl_bigip_query::output::render`.
//!
//! Field-value mutations (`=` / `|=` / `+=` / `-=`) are supported: the
//! default is a unified-diff preview, `--write` prints the rewritten config,
//! and `--in-place` overwrites the input. Identity-field writes (`.name = …`)
//! and the `rename*` builtins (`rename` / `rename_partition` / `rename_folder`
//! / `rename_prefix`) are supported too — they route through the
//! `rewrite::rename_object` token-rewrite engine.
//!
//! Side-inputs are supported: `--input-json` / `--input-jsonl` /
//! `--input-csv` / `--input-f5log` and the generic `--input KIND NAME=PATH`
//! bind an external JSON / JSONL / CSV / F5-log file to `$NAME`; the
//! `json_load` / `jsonl_load` / `csv_load` / `f5log_load` builtins load the
//! same shapes ad-hoc.
//!
//! `--merge` treats every loaded config as one logical namespace:
//! `.ltm.virtual[]` enumerates across files, `refs` / `referenced_by` walk
//! references across files, and two sources defining the same
//! `(kind, full-path)` are refused.
//!
//! Help actions are all implemented: `--help-dsl` / `--help-examples` /
//! `--help-renderers` / `--help-inputs` print static catalogues, while
//! `--help-builtins [NAME]` / `--help-manual` are generated from the builtin
//! registry metadata (`BuiltinSpec` carries no per-function prose). Live
//! network probes (`dns` / `ping` / `http` / `tls` / x509 + `cert_load`) are
//! gated behind `--enable-probes`.
//! `ucs_cert` has its UCS reader wired here (read the PEM out of the archive
//! via `read_ucs_member`, then `x509_parse`); reaching it through the DSL also
//! needs the `sys file-ssl-cert` object projection, which the query engine does
//! not yet surface (only the LTM kinds are projected).

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use tcl_bigip_io::{read_path, read_ucs_member, resolve_passphrase};
use tcl_bigip_query::value::Value;
use tcl_bigip_query::{QueryError, QueryOptions, QueryResult, output, run_query};

use crate::cli::FormatArgs;

use tcl_cli_support::difflib;

/// Percent-decode a URI component (the inverse of the `file_uri` encoder in
/// `tcl-bigip-io`), so a `config_uri` round-trips back to the on-disk path.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let hex = |b: u8| -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    };
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Resolve a `config_uri` to a local path, accepting `file://` and bare paths.
/// Returns `None` for a non-file scheme.
fn ucs_uri_to_path(config_uri: &str) -> Option<String> {
    if let Some(rest) = config_uri.strip_prefix("file://") {
        // Strip an optional `//netloc`; the path starts at the first `/`.
        let path = rest.find('/').map_or(rest, |i| &rest[i..]);
        Some(percent_decode(path))
    } else if config_uri.contains("://") {
        None
    } else {
        Some(percent_decode(config_uri))
    }
}

/// Build the `ucs_cert` reader the query engine injects: map a cert object's
/// `config_uri` + filestore `cache-path` to an `x509_parse`-shaped value,
/// reading the PEM out of the (possibly encrypted) UCS.
fn make_ucs_cert_reader() -> tcl_bigip_query::eval::UcsCertReader {
    let opts = crate::cli::PassphraseArgs::default().to_options();
    std::rc::Rc::new(
        move |config_uri: &str, cache_path: &str| -> Result<Value, QueryError> {
            let path = ucs_uri_to_path(config_uri).ok_or_else(|| {
                QueryError::builtin(format!(
                    "ucs_cert: source {config_uri} is not a local file-backed UCS"
                ))
            })?;
            let raw = std::fs::read(&path).map_err(|e| {
                QueryError::builtin(format!("ucs_cert: cannot read UCS {path}: {e}"))
            })?;
            let provider = || resolve_passphrase(&opts);
            let pem = read_ucs_member(&raw, cache_path, &provider, &path).map_err(|e| {
                QueryError::builtin(format!(
                    "ucs_cert: {cache_path} not found in {path} ({e}) \
                 (a plain bigip.conf has no filestore — read the UCS itself)"
                ))
            })?;
            let pem_str = String::from_utf8_lossy(&pem);
            tcl_bigip_query::probes::x509_parse(&pem_str)
        },
    )
}

/// Build the `files` / `file` / `glob` / `grep` reader the query engine injects:
/// enumerate a source's forensic UCS members (metadata) or read one member's
/// content, decrypting the archive as needed with the shared passphrase.
fn make_files_reader() -> tcl_bigip_query::eval::FilesReader {
    use tcl_bigip_io::{MemberScope, list_ucs_members};
    use tcl_bigip_query::eval::FileOp;

    let opts = crate::cli::PassphraseArgs::default().to_options();
    std::rc::Rc::new(
        move |config_uri: &str, op: &FileOp| -> Result<Value, QueryError> {
            let path = ucs_uri_to_path(config_uri).ok_or_else(|| {
                QueryError::builtin(format!(
                    "files: source {config_uri} is not a local file-backed UCS"
                ))
            })?;
            let raw = std::fs::read(&path)
                .map_err(|e| QueryError::builtin(format!("files: cannot read UCS {path}: {e}")))?;
            let provider = || resolve_passphrase(&opts);
            match op {
                FileOp::Inventory => {
                    let entries = list_ucs_members(&raw, &provider, &path, MemberScope::Forensic)
                        .map_err(|e| {
                        QueryError::builtin(format!(
                            "files: {path}: {e} \
                                 (a plain bigip.conf is not an archive — read the UCS itself)"
                        ))
                    })?;
                    let list = entries
                        .into_iter()
                        .map(|e| {
                            Value::object([
                                ("path".to_owned(), Value::Str(e.path)),
                                (
                                    "size".to_owned(),
                                    Value::Int(i64::try_from(e.size).unwrap_or(i64::MAX)),
                                ),
                                ("sha256".to_owned(), Value::Str(e.sha256)),
                                ("is_text".to_owned(), Value::Bool(e.is_text)),
                            ])
                        })
                        .collect();
                    Ok(Value::List(list))
                }
                FileOp::Content(member) => match read_ucs_member(&raw, member, &provider, &path) {
                    Ok(bytes) => Ok(Value::Str(String::from_utf8_lossy(&bytes).into_owned())),
                    Err(_) => Ok(Value::Null),
                },
            }
        },
    )
}

/// Whether *path*'s extension is `.ucs` (case-insensitive).
fn has_ucs_suffix(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ucs"))
}

/// The mutually-exclusive output-mode flags, resolved to an `output::render`
/// mode by [`OutputModeFlags::resolve`]. Corresponds to the `--scf` / `--raw`
/// / `--paths-only` / `--json` / `--table` / `--table-lineart` group.
//
// Each field maps to a distinct user-facing CLI flag (one of a mutually
// exclusive group), so a bool-per-flag is the natural shape — not a state
// machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy)]
pub struct OutputModeFlags {
    pub scf: bool,
    pub raw: bool,
    pub paths_only: bool,
    pub json: bool,
    pub table: bool,
    pub table_lineart: bool,
}

impl OutputModeFlags {
    /// Resolve the selected flag to an `output::render` mode (default `auto`).
    #[must_use]
    pub fn resolve(self) -> &'static str {
        if self.scf {
            "scf"
        } else if self.raw {
            "raw"
        } else if self.paths_only {
            "paths"
        } else if self.json {
            "json"
        } else if self.table {
            "table"
        } else if self.table_lineart {
            "table-lineart"
        } else {
            "auto"
        }
    }
}

/// Parse `--name NAME=PATH` bindings into a `name -> path` map, validating the
/// identifier and that the path was supplied as a positional input.
fn parse_name_bindings(
    raw: &[String],
    paths: &[String],
) -> Result<std::collections::HashMap<String, String>, String> {
    let name_re = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_-]*$").expect("valid regex");
    let mut bindings = std::collections::HashMap::new();
    for entry in raw {
        let Some((nm, pth)) = entry.split_once('=') else {
            return Err(format!("--name expects NAME=PATH (got '{entry}')"));
        };
        if !name_re.is_match(nm) {
            return Err(format!(
                "--name '{entry}': '{nm}' is not a valid DSL identifier \
                 (letters, digits, '_', '-'; cannot start with a digit)"
            ));
        }
        if bindings.contains_key(nm) {
            return Err(format!("--name {nm}: duplicate binding"));
        }
        if !paths.iter().any(|p| p == pth) {
            return Err(format!(
                "--name {nm}={pth}: path was not given as a positional \
                 input (so the runner would have no source text for it)"
            ));
        }
        bindings.insert(nm.to_owned(), pth.to_owned());
    }
    Ok(bindings)
}

/// Parse `--partition PATH=PARTITION` (or bare `--partition PARTITION`)
/// entries.
///
/// Returns ordered `(path, partition)` bindings where an empty `path` is the
/// bare form (applies to every loaded source). Validates that each `PATH=`
/// form names a positional input, that a single source isn't bound twice, and
/// that the bare form is given at most once.
fn parse_partition_bindings(
    raw: &[String],
    paths: &[String],
) -> Result<Vec<(String, String)>, String> {
    let mut bindings: Vec<(String, String)> = Vec::new();
    let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut saw_bare = false;
    for entry in raw {
        if let Some((pth, partition)) = entry.split_once('=') {
            if pth.is_empty() {
                return Err(format!("--partition '{entry}': PATH side cannot be empty"));
            }
            if partition.is_empty() {
                return Err(format!(
                    "--partition '{entry}': PARTITION side cannot be empty"
                ));
            }
            if !paths.iter().any(|p| p == pth) {
                return Err(format!(
                    "--partition {pth}={partition}: path was not given as a \
                     positional input (so the runner would have no source for it)"
                ));
            }
            if seen_paths.contains(pth) {
                return Err(format!("--partition {pth}: duplicate binding"));
            }
            seen_paths.insert(pth.to_owned());
            bindings.push((pth.to_owned(), partition.to_owned()));
        } else {
            // Bare `--partition NAME` applies to every source.
            if saw_bare {
                return Err("--partition: bare PARTITION may only be given once".to_owned());
            }
            saw_bare = true;
            bindings.push((String::new(), entry.clone()));
        }
    }
    Ok(bindings)
}

/// A parsed `--input-<kind> NAME=PATH[:hdr,…]` binding: `(name, (path,
/// csv_headers_or_None))`. Port of `_parse_input_bindings`' return shape.
type InputBinding = (String, (String, Option<Vec<String>>));

/// Parse a `--input-<kind> NAME=PATH[:hdr,…]` flag group.
///
/// Returns the bindings (preserving order). `allow_csv_headers` enables
/// the trailing `:hdr1,hdr2,…` CSV form. Enforces strict identifier rules,
/// duplicate detection, and fixed error wording.
fn parse_input_bindings(
    raw: &[String],
    flag: &str,
    allow_csv_headers: bool,
) -> Result<Vec<InputBinding>, String> {
    let name_re = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_-]*$").expect("valid regex");
    let mut bindings: Vec<InputBinding> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in raw {
        let Some((nm, pth)) = entry.split_once('=') else {
            return Err(format!("{flag} expects NAME=PATH (got '{entry}')"));
        };
        if !name_re.is_match(nm) {
            return Err(format!(
                "{flag} '{entry}': '{nm}' is not a valid DSL identifier \
                 (letters, digits, '_', '-'; cannot start with a digit)"
            ));
        }
        if pth.is_empty() {
            return Err(format!("{flag} '{entry}': PATH side cannot be empty"));
        }
        if seen.contains(nm) {
            return Err(format!("{flag} {nm}: duplicate binding"));
        }
        let (resolved_path, headers) = if allow_csv_headers {
            split_csv_path_headers(pth, flag, entry)?
        } else {
            (pth.to_owned(), None)
        };
        seen.insert(nm.to_owned());
        bindings.push((nm.to_owned(), (resolved_path, headers)));
    }
    Ok(bindings)
}

/// Split `PATH[:hdr1,hdr2]` without treating path colons as headers — port
/// of `query._split_csv_path_headers`.
fn split_csv_path_headers(
    value: &str,
    flag: &str,
    entry: &str,
) -> Result<(String, Option<Vec<String>>), String> {
    if !value.contains(':') {
        return Ok((value.to_owned(), None));
    }
    // Split on the last colon: path before, header list after.
    let (path_part, headers_part) = value.rsplit_once(':').expect("colon present");
    if headers_part.is_empty() {
        return Err(format!(
            "{flag} '{entry}': trailing ``:`` requires one or more header names (hdr1,hdr2,…)"
        ));
    }
    let split_headers: Vec<String> = headers_part
        .split(',')
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if split_headers.is_empty() {
        return Err(format!(
            "{flag} '{entry}': trailing ``:`` requires one or more header names (hdr1,hdr2,…)"
        ));
    }
    let header_re = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_-]*$").expect("valid regex");
    if !split_headers.iter().all(|h| header_re.is_match(h)) {
        // Not a header suffix (e.g. a POSIX / Windows path colon) — treat the
        // whole value as the path.
        return Ok((value.to_owned(), None));
    }
    if path_part.is_empty() {
        return Err(format!("{flag} '{entry}': PATH side cannot be empty"));
    }
    Ok((path_part.to_owned(), Some(split_headers)))
}

/// Parse repeated `--input KIND NAME=PATH` argument pairs. Returns
/// `(kind, name, path)` entries. KIND is validated against the registry by
/// the caller.
fn parse_custom_input_bindings(raw: &[String]) -> Result<Vec<(String, String, String)>, String> {
    let name_re = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_-]*$").expect("valid regex");
    let mut entries: Vec<(String, String, String)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    // clap delivers the two-value `--input` flag as a flat list of pairs.
    for pair in raw.chunks(2) {
        // `num_args = 2` guarantees full pairs, but guard defensively.
        if pair.len() != 2 {
            return Err(format!(
                "--input expects two arguments KIND NAME=PATH (got {pair:?})"
            ));
        }
        let kind = &pair[0];
        let binding = &pair[1];
        if kind.is_empty() {
            return Err(format!("--input '{kind}': KIND cannot be empty"));
        }
        let Some((nm, pth)) = binding.split_once('=') else {
            return Err(format!(
                "--input {kind} expects NAME=PATH (got '{binding}')"
            ));
        };
        if !name_re.is_match(nm) {
            return Err(format!(
                "--input {kind} '{binding}': '{nm}' is not a valid DSL \
                 identifier (letters, digits, '_', '-'; cannot start with a digit)"
            ));
        }
        if pth.is_empty() {
            return Err(format!(
                "--input {kind} '{binding}': PATH side cannot be empty"
            ));
        }
        if seen.contains(nm) {
            return Err(format!("--input {kind} {nm}: duplicate binding"));
        }
        seen.insert(nm.to_owned());
        entries.push((kind.clone(), nm.to_owned(), pth.to_owned()));
    }
    Ok(entries)
}

/// The parsed side-input flag groups — the structured form of [`InputArgs`]
/// after each `--input-*` group is validated. Each entry is a
/// `(name, (path, csv_headers))` binding except the generic `--input`, which
/// also carries its KIND.
struct ParsedInputArgs {
    json: Vec<InputBinding>,
    jsonl: Vec<InputBinding>,
    csv: Vec<InputBinding>,
    f5log: Vec<InputBinding>,
    custom: Vec<(String, String, String)>,
}

/// Validate every side-input flag group, in order (json, jsonl, csv, f5log,
/// generic). Returns the first parse error's message.
fn parse_input_args(args: &InputArgs) -> Result<ParsedInputArgs, String> {
    Ok(ParsedInputArgs {
        json: parse_input_bindings(args.input_json, "--input-json", false)?,
        jsonl: parse_input_bindings(args.input_jsonl, "--input-jsonl", false)?,
        csv: parse_input_bindings(args.input_csv, "--input-csv", true)?,
        f5log: parse_input_bindings(args.input_f5log, "--input-f5log", false)?,
        custom: parse_custom_input_bindings(args.input)?,
    })
}

/// Read each side-input file and build the `$NAME`-bound [`SideInput`]s.
///
/// Each binding reads its file, collides-checks the URI against positional
/// inputs and prior side inputs, and refuses to re-bind a `$NAME` already
/// claimed by an earlier `--input*` flag. Returns `(message, exit_code)` on
/// the first failure. Order: json, jsonl, csv, f5log, generic.
fn load_side_inputs(
    parsed: &ParsedInputArgs,
    path_for_uri: &[(String, String)],
    opts: &tcl_bigip_io::PassphraseOptions,
) -> Result<Vec<tcl_bigip_query::SideInput>, (String, u8)> {
    let mut side_inputs: Vec<tcl_bigip_query::SideInput> = Vec::new();
    let mut side_names: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut side_uris: std::collections::HashSet<String> = std::collections::HashSet::new();

    // One typed group + its `--input-<kind>` flag + its `InputSpec` builder.
    let typed_groups: [(&[InputBinding], &str, &str); 4] = [
        (&parsed.json, "--input-json", "json"),
        (&parsed.jsonl, "--input-jsonl", "jsonl"),
        (&parsed.csv, "--input-csv", "csv"),
        (&parsed.f5log, "--input-f5log", "f5log"),
    ];
    for (group, flag, kind) in typed_groups {
        for (nm, (pth, hdr)) in group {
            let spec = if kind == "csv" {
                tcl_bigip_query::InputSpec::with_csv_headers("csv", hdr.clone())
            } else {
                tcl_bigip_query::InputSpec::new(kind)
            };
            load_one_side_input(
                nm,
                pth,
                flag,
                spec,
                path_for_uri,
                opts,
                &mut side_inputs,
                &mut side_names,
                &mut side_uris,
            )?;
        }
    }

    // Generic `--input KIND NAME=PATH` — KIND validated against the registry.
    for (kind, nm, pth) in &parsed.custom {
        if !tcl_bigip_query::inputs::is_registered(kind) {
            let registered = tcl_bigip_query::inputs::list_input_formats()
                .iter()
                .map(|s| s.name)
                .collect::<Vec<_>>()
                .join(", ");
            return Err((
                format!(
                    "--input {kind} {nm}={pth}: unknown input format '{kind}' \
                     (registered: {registered})"
                ),
                2,
            ));
        }
        load_one_side_input(
            nm,
            pth,
            &format!("--input {kind}"),
            tcl_bigip_query::InputSpec::new(kind.clone()),
            path_for_uri,
            opts,
            &mut side_inputs,
            &mut side_names,
            &mut side_uris,
        )?;
    }
    Ok(side_inputs)
}

/// Load a single side-input file, threading the collision-tracking state.
// Threads the loader's mutable collision-tracking state positionally; a struct
// would just re-wrap these one-shot parameters.
#[allow(clippy::too_many_arguments)]
fn load_one_side_input(
    nm: &str,
    pth: &str,
    flag: &str,
    spec: tcl_bigip_query::InputSpec,
    path_for_uri: &[(String, String)],
    opts: &tcl_bigip_io::PassphraseOptions,
    side_inputs: &mut Vec<tcl_bigip_query::SideInput>,
    side_names: &mut std::collections::HashMap<String, String>,
    side_uris: &mut std::collections::HashSet<String>,
) -> Result<(), (String, u8)> {
    if let Some(prior_uri) = side_names.get(nm) {
        return Err((
            format!(
                "{flag} {nm}={pth}: name ${nm} is already bound by a prior \
                 --input* flag (to {prior_uri}); pick a different name"
            ),
            2,
        ));
    }
    let (uri, src) =
        read_path(pth, false, opts).map_err(|e| (format!("{flag} {nm}={pth}: {e}"), 2))?;
    if path_for_uri.iter().any(|(u, _)| *u == uri) {
        return Err((
            format!("{flag} {nm}={pth}: URI {uri} is already loaded as a positional input"),
            2,
        ));
    }
    if side_uris.contains(&uri) {
        return Err((
            format!("{flag} {nm}={pth}: URI {uri} is already loaded as a side input"),
            2,
        ));
    }
    side_uris.insert(uri.clone());
    side_names.insert(nm.to_owned(), uri.clone());
    side_inputs.push(tcl_bigip_query::SideInput {
        name: nm.to_owned(),
        uri,
        source: src,
        spec,
    });
    Ok(())
}

/// Render the `--help-inputs` catalogue.
#[must_use]
pub fn help_inputs_text() -> String {
    use std::fmt::Write as _;

    let specs = tcl_bigip_query::inputs::list_input_formats();
    if specs.is_empty() {
        return "(no input formats registered)\n".to_string();
    }
    let mut out = String::from("Registered input formats:\n\n");
    for spec in specs {
        let _ = writeln!(out, "  {}", spec.name);
        let _ = writeln!(out, "    summary: {}", spec.summary);
        if !spec.details.is_empty() {
            for line in spec.details.lines() {
                let _ = writeln!(out, "    {line}");
            }
        }
        out.push('\n');
    }
    out.push_str("Use --input KIND NAME=PATH to bind a file via any registered format.\n");
    out
}

/// Parse repeated `--render-opt KEY=VALUE` into a flat map.
///
/// Duplicate keys take the last value. Returns a fixed error string when an
/// entry is malformed.
///
/// # Errors
///
/// Returns an error string for an entry with no `=` or an empty key side.
pub fn parse_render_opts(raw: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut opts: BTreeMap<String, String> = BTreeMap::new();
    for entry in raw {
        let Some((k, v)) = entry.split_once('=') else {
            return Err(format!("--render-opt expects KEY=VALUE (got '{entry}')"));
        };
        if k.is_empty() {
            return Err(format!("--render-opt '{entry}': KEY side cannot be empty"));
        }
        opts.insert(k.to_owned(), v.to_owned());
    }
    Ok(opts)
}

/// Flatten top-level `Stream`s, needed to build the multi-file JSON
/// envelope's inner value arrays.
fn flat(values: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for v in values {
        if let Value::Stream(items) = v {
            out.extend(items.iter().cloned());
        } else {
            out.push(v.clone());
        }
    }
    out
}

/// The non-output behavioural flags for a query run. `merge` treats every
/// loaded config as one logical namespace; `write` / `in_place` pick the
/// mutation emit path; `strict` selects the empty-match exit code.
//
// Each field maps to a distinct user-facing CLI flag, so a bool-per-flag is
// the natural shape here.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy)]
pub struct QueryFlags {
    pub merge: bool,
    pub write: bool,
    pub in_place: bool,
    pub strict: bool,
}

/// The live-probe flags (`--enable-probes` / `--ca-bundle`). Threaded
/// into the runner's [`EvalContext`] so the network builtins can gate on the
/// `--enable-probes` opt-in and the TLS-aware probes can pick up a CA bundle.
pub struct ProbeArgs {
    pub enable_probes: bool,
    pub ca_bundle: Option<String>,
}

/// The raw side-input flag groups (`--input-*` / `--input`). Parsed +
/// validated inside [`run_query_verb`] so the error wording / order is fixed.
pub struct InputArgs<'a> {
    pub input_json: &'a [String],
    pub input_jsonl: &'a [String],
    pub input_csv: &'a [String],
    pub input_f5log: &'a [String],
    pub input: &'a [String],
}

/// `f5 query` (read-only).
//
// The argument count and length track the verb's flag fan-out one-for-one.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn run_query_verb(
    expression: Option<&str>,
    inputs: &[PathBuf],
    names: &[String],
    partition: &[String],
    input_args: &InputArgs,
    mode: &str,
    render_opts: &BTreeMap<String, String>,
    flags: QueryFlags,
    format: &FormatArgs,
    probes: &ProbeArgs,
) -> anyhow::Result<u8> {
    // Validate up-front, in a fixed order with fixed messages.
    let Some(expression) = expression else {
        eprintln!("error: no query expression supplied (positional or --from-file)");
        return Ok(2);
    };
    if inputs.is_empty() {
        eprintln!("error: no input files (pass '-' to read stdin)");
        return Ok(2);
    }

    // `--format tmsh` re-renders the parsed config as a `tmsh modify` script —
    // never a safe replacement for the on-disk SCF source. Reject the
    // `--in-place` + tmsh combination up front.
    if flags.in_place && (format.format == "tmsh" || format.format == "tmsh-delta") {
        eprintln!(
            "error: --in-place is incompatible with --format {} \
             (in-place writes must preserve the SCF source format; \
             use --write or redirect to an explicit output file for \
             tmsh script output)",
            format.format
        );
        return Ok(2);
    }

    // `--in-place` requires a real path, not stdin.
    if flags.in_place && inputs.iter().any(|p| p.as_os_str() == "-") {
        eprintln!("error: --in-place requires a path, not stdin");
        return Ok(2);
    }

    let path_strs: Vec<String> = inputs
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    let name_map = match parse_name_bindings(names, &path_strs) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    let partition_bindings = match parse_partition_bindings(partition, &path_strs) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    // Parse every side-input flag group up front (order: json, jsonl, csv,
    // f5log, then the generic --input). Each surfaces a parse error before
    // any file IO.
    let parsed_inputs = match parse_input_args(input_args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    // Load each input to `(uri, source)` via the UCS/stdin-aware reader, in
    // source order. Reject duplicate URIs.
    let opts = crate::cli::PassphraseArgs::default().to_options();
    let mut sources: Vec<(String, String)> = Vec::with_capacity(path_strs.len());
    let mut path_for_uri: Vec<(String, String)> = Vec::new();
    for path_str in &path_strs {
        // `--in-place` rewriting a UCS archive would mean repacking the
        // gzipped tar and losing other artefacts — refuse.
        if flags.in_place && path_str != "-" && has_ucs_suffix(path_str) {
            eprintln!(
                "error: --in-place not supported for UCS archives ({path_str}); \
                 extract first with `f5 extract` or use --write"
            );
            return Ok(2);
        }
        // Strict UTF-8 for mutating in-place writes: if any byte can't be
        // decoded, raise instead of silently swapping it for U+FFFD —
        // otherwise the read…rewrite…write round-trip would permanently
        // overwrite the unreadable bytes.
        let (uri, src) = match read_path(path_str, flags.in_place, &opts) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("error: {e}");
                return Ok(2);
            }
        };
        if let Some((_, prev)) = path_for_uri.iter().find(|(u, _)| *u == uri) {
            let label = if path_str == "-" {
                "stdin"
            } else {
                path_str.as_str()
            };
            if prev == path_str {
                eprintln!("error: duplicate input {label}");
            } else {
                eprintln!("error: duplicate input {label} (already read as {prev})");
            }
            return Ok(2);
        }
        path_for_uri.push((uri.clone(), path_str.clone()));
        sources.push((uri, src));
    }

    // Translate `--name N=PATH` (path-string) to URI keys for the runner.
    let mut resolved_names = std::collections::HashMap::new();
    for (nm, path_str) in name_map {
        let uri = path_for_uri
            .iter()
            .find(|(_, p)| *p == path_str)
            .map(|(u, _)| u.clone());
        let Some(uri) = uri else {
            eprintln!(
                "error: --name {nm}={path_str}: path was not loaded \
                 (must also appear as a positional argument)"
            );
            return Ok(2);
        };
        resolved_names.insert(nm, uri);
    }

    // Translate `--partition PATH=PARTITION` keys into URI form so the
    // runner's per-URI partition lookup uses the same key shape the source
    // map does. The bare form (empty path) applies to every loaded source.
    let mut resolved_partitions: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (path_str, partition) in &partition_bindings {
        if path_str.is_empty() {
            for (uri, _) in &path_for_uri {
                resolved_partitions
                    .entry(uri.clone())
                    .or_insert_with(|| partition.clone());
            }
            continue;
        }
        let uri = path_for_uri
            .iter()
            .find(|(_, p)| p == path_str)
            .map(|(u, _)| u.clone());
        let Some(uri) = uri else {
            eprintln!(
                "error: --partition {path_str}={partition}: path was not \
                 loaded (must also appear as a positional argument)"
            );
            return Ok(2);
        };
        resolved_partitions.insert(uri, partition.clone());
    }

    // Load every structured side-input into `$NAME`-bound `SideInput`s.
    let side_inputs = match load_side_inputs(&parsed_inputs, &path_for_uri, &opts) {
        Ok(s) => s,
        Err((msg, code)) => {
            eprintln!("error: {msg}");
            return Ok(code);
        }
    };

    // Side inputs participate in the multi-file source count (banner / JSON
    // envelope) but never iterate as the primary `.` input: they live in
    // `sources` yet are skipped by the per-file loop's source check.
    let n_sources = sources.len() + side_inputs.len();

    let query_opts = QueryOptions {
        names: resolved_names,
        partitions: resolved_partitions,
        side_inputs,
        enable_probes: probes.enable_probes,
        ca_bundle: probes.ca_bundle.clone(),
        // `ucs_cert` re-opens a cert from inside a UCS (decrypt + un-tar);
        // inject a reader using the same passphrase resolution the input
        // loader uses.
        ucs_cert_reader: Some(make_ucs_cert_reader()),
        // `files` / `file` / `glob` / `grep` read the OS files inside a UCS;
        // inject a reader over the same passphrase resolution.
        files_reader: Some(make_files_reader()),
        merge: flags.merge,
    };

    let result = match run_query(expression, &sources, &query_opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    if result.has_mutation {
        return emit_mutation(&result, &path_for_uri, flags, format);
    }

    emit_values(&result, n_sources, mode, render_opts, flags.strict)
}

/// Emit the result of a mutating query.
///
/// Default: a unified-diff preview per changed file. `--write`: the rewritten
/// config to stdout. `--in-place`: overwrite each input. A no-op (no file
/// changed) exits 1, or 2 under `--strict`.
fn emit_mutation(
    result: &QueryResult,
    path_for_uri: &[(String, String)],
    flags: QueryFlags,
    format: &FormatArgs,
) -> anyhow::Result<u8> {
    let mut any_changed = false;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for (uri, applied) in &result.edits_per_file {
        // Surface every rename report on stderr as
        // `renamed {old!r} -> {new!r} ({n} occurrence(s))`.
        for rep in &applied.rename_reports {
            eprintln!(
                "renamed {} -> {} ({} occurrence(s))",
                tcl_bigip_query::eval::pyr_pub(&rep.old),
                tcl_bigip_query::eval::pyr_pub(&rep.new),
                rep.occurrences
            );
        }
        // A "mutating" query that produced no actual textual change exits 1
        // (no-op).
        if applied.new_source == applied.original {
            continue;
        }
        any_changed = true;
        let path_str = path_for_uri
            .iter()
            .find(|(u, _)| u == uri)
            .map_or(uri.as_str(), |(_, p)| p.as_str());
        // `--format tmsh` / `tmsh-delta` re-render the rewritten SCF as a
        // `tmsh modify` script for the stdout / explicit-output paths. The
        // unified-diff path stays SCF↔SCF (the on-disk source is SCF), and
        // `--in-place` + tmsh is already rejected up front, so in-place writes
        // always render verbatim SCF.
        let rewritten = super::emit::render_config(
            &applied.new_source,
            &format.format,
            "modify",
            format.transaction,
            &applied.original,
        );
        if flags.in_place && path_str != "-" {
            std::fs::write(Path::new(path_str), &rewritten)?;
            continue;
        }
        if flags.write {
            write!(out, "{rewritten}")?;
            continue;
        }
        let from = path_str.to_owned();
        let to = format!("{path_str} (modified)");
        let a = difflib::splitlines_keepends(&applied.original);
        let b = difflib::splitlines_keepends(&applied.new_source);
        // The shared `difflib` yields the diff as a line list (each line keeps
        // its terminator); the default unified-diff context is 3 — join the
        // lines to reproduce the previous concatenated string.
        let diff = difflib::unified_diff(&a, &b, &from, &to, 3).join("");
        write!(out, "{diff}")?;
    }
    if !any_changed {
        if flags.strict {
            eprintln!(
                "error: --strict: mutating query produced no textual change \
                 (no matches).  Check the path / predicate."
            );
            return Ok(2);
        }
        return Ok(1);
    }
    Ok(0)
}

/// Render the per-file values (read path).
fn emit_values(
    result: &tcl_bigip_query::QueryResult,
    n_sources: usize,
    mode: &str,
    render_opts: &BTreeMap<String, String>,
    strict: bool,
) -> anyhow::Result<u8> {
    let multi = n_sources > 1;
    let use_banner = multi && mode != "json";
    let mut any_matched = false;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    // Multi-file `--json` → one top-level envelope of {uri, values}.
    if multi && mode == "json" {
        let envelope: Vec<Value> = result
            .values_per_file
            .iter()
            .map(|(uri, values)| {
                if !values.is_empty() {
                    any_matched = true;
                }
                Value::object([
                    ("uri".to_owned(), Value::Str(uri.clone())),
                    ("values".to_owned(), Value::List(flat(values))),
                ])
            })
            .collect();
        let rendered = tcl_bigip_query::jsonfmt::to_pretty(&Value::List(envelope));
        writeln!(out, "{rendered}")?;
        return Ok(empty_match_exit_code(any_matched, strict));
    }

    for (uri, values) in &result.values_per_file {
        if !values.is_empty() {
            any_matched = true;
        }
        if use_banner {
            writeln!(out, "# === {uri} ===")?;
        }
        match output::render_with_opts(values, mode, render_opts) {
            Ok(text) => write!(out, "{text}")?,
            Err(e) => {
                eprintln!("error: {e}");
                return Ok(2);
            }
        }
    }
    Ok(empty_match_exit_code(any_matched, strict))
}

/// Decide the read-only exit code.
///
/// jq-style: `0` whether or not the query matched; `--strict` opts into
/// `exit 1 when nothing matched`.
fn empty_match_exit_code(any_matched: bool, strict: bool) -> u8 {
    u8::from(!any_matched && strict)
}

/// Render the `--help-renderers` catalogue: each registered renderer's name,
/// summary, `accepts`, and details, then the usage footer.
#[must_use]
pub fn help_renderers_text() -> String {
    use std::fmt::Write as _;

    let specs = tcl_bigip_query::renderers::list_renderers();
    if specs.is_empty() {
        return "(no renderers registered)\n".to_string();
    }
    let mut out = String::from("Registered renderers:\n\n");
    for spec in specs {
        let _ = writeln!(out, "  {}", spec.name);
        let _ = writeln!(out, "    summary: {}", spec.summary);
        let _ = writeln!(out, "    accepts: {}", spec.accepts);
        if !spec.details.is_empty() {
            for line in spec.details.lines() {
                let _ = writeln!(out, "    {line}");
            }
        }
        out.push('\n');
    }
    out.push_str(
        "Use --render NAME to dispatch, --render-opt KEY=VALUE for per-renderer options.\n",
    );
    out
}

#[cfg(test)]
mod ucs_cert_tests {
    use super::{make_ucs_cert_reader, percent_decode, ucs_uri_to_path};
    use tcl_bigip_query::value::Value;

    // The committed UCS holds a metadata-free `sys file ssl-cert` stanza plus
    // the real PEM in the filestore; `ucs_cert` must recover the cert's identity
    // from the PEM, not the stanza. (End-to-end reachability via the query DSL
    // additionally needs the unimplemented `sys file-ssl-cert` projection.)
    const CACHE_PATH: &str = "/config/filestore/files_d/Common_d/certificate_d/:Common:t.crt_1";
    const EXPECTED_FP: &str = "79F67B000B1E685F3B2EC336A82C37985A1175F17176D60A60CDD6DD7FE874CD";

    #[test]
    fn reader_reads_and_parses_real_cert() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ucs-cert-sample.ucs"
        );
        let reader = make_ucs_cert_reader();
        let value = reader(&format!("file://{fixture}"), CACHE_PATH)
            .expect("reader reads + parses the cert");
        let Value::Object(map) = value else {
            panic!("ucs_cert must return an x509 dict");
        };
        let fp = match map.get("fingerprint_sha256") {
            Some(Value::Str(s)) => s.clone(),
            _ => panic!("x509 dict has no fingerprint_sha256 string"),
        };
        assert_eq!(
            fp, EXPECTED_FP,
            "ucs_cert must parse the real PEM from the filestore",
        );
    }

    #[test]
    fn reader_errors_on_missing_member() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ucs-cert-sample.ucs"
        );
        let reader = make_ucs_cert_reader();
        let err = reader(&format!("file://{fixture}"), "/config/filestore/nope_1").unwrap_err();
        assert!(format!("{err}").contains("ucs_cert"), "got: {err}");
    }

    #[test]
    fn uri_and_percent_decode() {
        assert_eq!(
            ucs_uri_to_path("file:///tmp/a.ucs").as_deref(),
            Some("/tmp/a.ucs")
        );
        assert_eq!(ucs_uri_to_path("/tmp/a.ucs").as_deref(), Some("/tmp/a.ucs"));
        assert_eq!(ucs_uri_to_path("https://x/a"), None);
        assert_eq!(percent_decode("a%20b%2Fc"), "a b/c");
    }
}
