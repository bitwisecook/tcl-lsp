//! Proc argument trait inference — Rust port of
//! ``core/analysis/proc_arg_traits.py::infer_param_traits`` and
//! ``infer_param_traits_deep``.
//!
//! Walks a proc body to determine how each parameter is used:
//!
//! - `Eval` — passed to ``eval`` / ``uplevel`` / ``subst``
//! - `Body` — used as a loop / control body
//! - `VarWrite` — names a variable the proc writes (upvar +
//!   ``set`` / ``incr`` / ``append`` / ``lappend``, or a
//!   registry-marked variable-write site)
//! - `VarRead` — names a variable the proc reads via ``upvar``
//! - `Expr` — evaluated as an expression
//! - `LoopList` — used as the list arg in ``foreach`` / ``lmap``
//!
//! Two passes are exposed:
//!
//! * [`infer_param_traits`] — shallow, top-level command scan
//!   only.  Fast enough for synchronous use during analysis;
//!   detects direct patterns like ``eval $param``, ``upvar 1
//!   $param local``, ``foreach x $list body``.  Does not
//!   recurse into braced body arguments.
//! * [`infer_param_traits_deep`] — recursive descent into
//!   braced body args, catching traits hidden one or more
//!   levels deep (`foreach item $items { uplevel 1 $body }`
//!   surfaces the `$body` Eval trait via the recursion).  More
//!   expensive than the shallow pass; intended for asynchronous
//!   analysis (call-graph / symbol-graph / dataflow-graph /
//!   semantic-graph builders).  Bounded by [`MAX_DEPTH`]
//!   (8 levels) to prevent runaway recursion on pathological
//!   input.
//!
//! [`merge_traits`] unions the two passes' results when
//! callers want both.

use std::collections::{HashMap, HashSet};

use tcl_registry::arg_role::ArgRole;
use tcl_registry::stub_overlay::StubOverlay;
use tcl_registry::CommandRegistry;

use super::types::ProcArgTrait;
use crate::segmenter::segment_commands;

/// Top-level shallow trait inference.  Returns a map from
/// parameter name to a set of inferred traits.  Empty entries
/// (parameters with no detected trait) are dropped from the
/// returned map.
///
/// `registry` is the **dialect-aware** command registry the
/// caller already built (typically `Analyser::registry`).
/// Building a fresh `CommandRegistry::build_default()` on every
/// proc would both be expensive and miss dialect-specific
/// `arg_role_resolver` / `arg_roles` (e.g. iRules `when` body
/// detection) that the caller's registry already loaded.
///
/// `stub_overlay`, when `Some`, lets user-declared
/// `# tcl-lsp: stub` commands participate in role-driven
/// trait inference.  The overlay's
/// [`StubOverlay::arg_indices_for_role`] return is unioned
/// with the registry's at each call site, so a stub like
/// `# tcl-lsp: stub my_eval {script:body}` causes a
/// `my_eval $param` invocation to mark the parameter as
/// `ProcArgTrait::Body`.
#[must_use]
pub fn infer_param_traits(
    params: &[&str],
    body_source: &str,
    registry: &CommandRegistry,
    stub_overlay: Option<&StubOverlay>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    if params.is_empty() || body_source.trim().is_empty() {
        return HashMap::new();
    }
    let param_set: HashSet<&str> = params.iter().copied().collect();
    let mut traits: HashMap<&str, HashSet<ProcArgTrait>> =
        params.iter().map(|p| (*p, HashSet::new())).collect();
    let mut upvar_aliases: HashMap<String, &str> = HashMap::new();

    scan_commands(
        body_source,
        &param_set,
        &mut traits,
        &mut upvar_aliases,
        registry,
        stub_overlay,
    );

    finalise_traits(traits)
}

/// Maximum recursion depth for [`infer_param_traits_deep`] —
/// matches Python's ``_MAX_DEPTH = 8`` in
/// ``core/analysis/proc_arg_traits.py``.  Pathological input
/// (deeply-nested braced bodies) stops descending past this
/// bound rather than blowing the stack.
pub const MAX_DEPTH: u8 = 8;

/// Recursive deep trait inference.  Same return shape as
/// [`infer_param_traits`] but additionally descends into braced
/// body arguments to surface traits hidden one or more levels
/// in.  More expensive than the shallow pass — intended for
/// asynchronous use behind the `S*` call-graph / symbol-graph /
/// dataflow-graph / semantic-graph builders.
///
/// Mirrors Python's ``infer_param_traits_deep``
/// (``core/analysis/proc_arg_traits.py:216-241``).  Recursion
/// is bounded by [`MAX_DEPTH`] and only enters braced body args
/// — `$var` or `[cmd]` references at the head of a body arg
/// are treated as opaque (their `Eval` trait is already
/// captured at the top level by the same call-site's role
/// scan).
#[must_use]
pub fn infer_param_traits_deep(
    params: &[&str],
    body_source: &str,
    registry: &CommandRegistry,
    stub_overlay: Option<&StubOverlay>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    if params.is_empty() || body_source.trim().is_empty() {
        return HashMap::new();
    }
    let param_set: HashSet<&str> = params.iter().copied().collect();
    let mut traits: HashMap<&str, HashSet<ProcArgTrait>> =
        params.iter().map(|p| (*p, HashSet::new())).collect();
    let mut upvar_aliases: HashMap<String, &str> = HashMap::new();

    scan_deep(
        body_source,
        &param_set,
        &mut traits,
        &mut upvar_aliases,
        0,
        registry,
        stub_overlay,
    );

    finalise_traits(traits)
}

/// Union shallow + deep trait results per parameter.  Mirrors
/// Python's ``merge_traits``.  Useful when callers want to run
/// the shallow pass synchronously for an initial result and
/// then upgrade with the deep pass once it completes.
//
// `implicit_hasher` allowed: this helper is paired with
// [`infer_param_traits`] / [`infer_param_traits_deep`], both
// of which return the default-hasher [`HashMap`].  Generalising
// the hasher here would force every caller (today and future)
// to declare the same type parameter for no practical gain —
// the call sites unconditionally feed the helper their results.
#[allow(clippy::implicit_hasher)]
#[must_use]
pub fn merge_traits(
    shallow: HashMap<String, HashSet<ProcArgTrait>>,
    deep: HashMap<String, HashSet<ProcArgTrait>>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    let mut merged = shallow;
    for (param, deep_traits) in deep {
        merged.entry(param).or_default().extend(deep_traits);
    }
    merged
}

/// Drop parameters with no detected trait and convert the
/// borrowed keys back to owned `String`s.  Shared between
/// `infer_param_traits` and `infer_param_traits_deep` so both
/// pass shapes return the same kind of map.
fn finalise_traits(
    traits: HashMap<&str, HashSet<ProcArgTrait>>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    traits
        .into_iter()
        .filter(|(_, set)| !set.is_empty())
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

/// Single-level command scan shared by both passes.  Extracts
/// segmented commands from `source` and dispatches each through
/// [`scan_command`].
///
/// `'p` is the params' lifetime (the param-name slices borrowed
/// from the caller's `params` argument); `source` has an
/// independent lifetime so callers can re-enter with body-arg
/// slices that don't outlive the enclosing source.
fn scan_commands<'p>(
    source: &str,
    param_set: &HashSet<&'p str>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    upvar_aliases: &mut HashMap<String, &'p str>,
    registry: &CommandRegistry,
    stub_overlay: Option<&StubOverlay>,
) {
    let commands = extract_commands(source);
    for (cmd_name, cmd_args) in &commands {
        scan_command(
            cmd_name,
            cmd_args,
            param_set,
            traits,
            upvar_aliases,
            registry,
            stub_overlay,
        );
    }
}

/// Recursive scan with depth tracking.  Walks every command at
/// the current level, then descends into the braced body
/// arguments each command declares via the registry's
/// ``ArgRole::Body`` role assignments.  Mirrors Python's
/// ``_scan_deep`` (``core/analysis/proc_arg_traits.py:247-288``).
///
/// `$var` / `[cmd]` body args are skipped — they aren't
/// braced bodies, and any `Eval` trait they carry is recorded
/// at the top level by the same call-site's role scan.
fn scan_deep<'p>(
    source: &str,
    param_set: &HashSet<&'p str>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    upvar_aliases: &mut HashMap<String, &'p str>,
    depth: u8,
    registry: &CommandRegistry,
    stub_overlay: Option<&StubOverlay>,
) {
    if depth > MAX_DEPTH {
        return;
    }

    scan_commands(
        source,
        param_set,
        traits,
        upvar_aliases,
        registry,
        stub_overlay,
    );

    // The recursion only walks braced bodies, so we re-segment
    // here rather than threading the segmented commands through
    // `scan_commands`.  The segmented slices have a lifetime
    // tied to this stack frame; each recursion needs its own.
    let segments = segment_commands(source);
    for seg in segments {
        if seg.texts.is_empty() {
            continue;
        }
        let cmd_name = &seg.texts[0];
        let cmd_args: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();
        // Look up body args from both the registry (for built-in
        // commands) and the stub overlay (for user-declared
        // `# tcl-lsp: stub` commands).  Union so a stub-defined
        // body arg recurses just like a registry-defined one.
        let mut body_indices: HashSet<usize> = registry
            .arg_indices_for_role(cmd_name, &cmd_args, ArgRole::Body)
            .into_iter()
            .collect();
        if let Some(overlay) = stub_overlay {
            body_indices.extend(overlay.arg_indices_for_role(cmd_name, &cmd_args, ArgRole::Body));
        }
        for idx in body_indices {
            let Some(body_text) = cmd_args.get(idx) else {
                continue;
            };
            if body_text.trim().is_empty() {
                continue;
            }
            // Skip non-braced bodies — `$var` / `[cmd]` heads
            // are already handled at the top-level role scan
            // (their `Eval` trait is recorded by
            // `apply_arg_role_traits` /
            // `apply_eval_traits`).  Match Python's check by
            // peeking at the first two bytes for cheap detection.
            let head = body_text.as_bytes();
            if head.first().is_some_and(|&b| b == b'$' || b == b'[') {
                continue;
            }
            if head.len() >= 2 && (head[1] == b'$' || head[1] == b'[') {
                continue;
            }
            scan_deep(
                body_text,
                param_set,
                traits,
                upvar_aliases,
                depth + 1,
                registry,
                stub_overlay,
            );
        }
    }
}

/// Extract `(command, args)` pairs from `source` via the
/// segmenter.  Mirrors ``_extract_commands`` in
/// ``core/analysis/proc_arg_traits.py:78-95``.
fn extract_commands(source: &str) -> Vec<(String, Vec<String>)> {
    let mut commands = Vec::new();
    let segments = segment_commands(source);
    for seg in segments {
        if seg.texts.is_empty() {
            continue;
        }
        let cmd_name = seg.texts[0].clone();
        let cmd_args: Vec<String> = seg.texts[1..].to_vec();
        commands.push((cmd_name, cmd_args));
    }
    commands
}

/// Extract a bare variable name from ``$var`` or ``${var}``.
/// Returns `None` when the text isn't a simple variable
/// reference.  Mirrors ``_extract_var_name`` in Python.
fn extract_var_name(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'$' {
        return None;
    }
    let (name_start, name_end) = if bytes[1] == b'{' {
        // ``${name}`` — find the closing ``}``.
        let close = text[2..].find('}')?;
        (2, 2 + close)
    } else {
        (1, bytes.len())
    };
    let name = &text[name_start..name_end];
    if name.is_empty() {
        return None;
    }
    // Verify identifier-like content (alphanumerics, underscore,
    // colons for namespace-qualified names — matches the Python
    // ``_SIMPLE_VAR_RE`` regex shape).
    let mut iter = name.chars();
    let first = iter.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }
    if !iter.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') {
        return None;
    }
    // Reject anything past the closing ``}`` for the braced form.
    if bytes[1] == b'{' && name_end + 1 < bytes.len() {
        return None;
    }
    Some(name)
}

/// Resolve a command's per-arg roles via the registry, unioned
/// with any matching `# tcl-lsp: stub` overlay entry.  Mirrors
/// ``_resolve_arg_roles`` in Python — picks the
/// `arg_role_resolver` callback first, then static
/// `arg_roles`, then sub-command-level roles.  When
/// `stub_overlay` is `Some`, user-declared stub commands
/// contribute their declared roles on top of the registry's;
/// a stub-defined role for a given arg index overrides the
/// registry's (the overlay is later-write-wins).
fn resolve_arg_roles(
    command: &str,
    args: &[String],
    registry: &CommandRegistry,
    stub_overlay: Option<&StubOverlay>,
) -> HashMap<u8, ArgRole> {
    let mut roles: HashMap<u8, ArgRole> = HashMap::new();
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    for role in [
        ArgRole::Body,
        ArgRole::Expr,
        ArgRole::VarWrite,
        ArgRole::VarRead,
    ] {
        for idx in registry.arg_indices_for_role(command, &arg_strs, role) {
            if let Ok(idx_u8) = u8::try_from(idx) {
                roles.insert(idx_u8, role);
            }
        }
        if let Some(overlay) = stub_overlay {
            for idx in overlay.arg_indices_for_role(command, &arg_strs, role) {
                if let Ok(idx_u8) = u8::try_from(idx) {
                    roles.insert(idx_u8, role);
                }
            }
        }
    }
    roles
}

fn scan_command<'p>(
    cmd_name: &str,
    cmd_args: &[String],
    param_set: &HashSet<&'p str>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    upvar_aliases: &mut HashMap<String, &'p str>,
    registry: &CommandRegistry,
    stub_overlay: Option<&StubOverlay>,
) {
    apply_arg_role_traits(
        cmd_name,
        cmd_args,
        param_set,
        traits,
        upvar_aliases,
        registry,
        stub_overlay,
    );
    apply_eval_traits(cmd_name, cmd_args, param_set, traits);

    // Per-command structural handlers — mirror the Python
    // ``_handle_*`` functions.
    match cmd_name {
        "upvar" => handle_upvar(cmd_args, param_set, traits, upvar_aliases),
        "foreach" | "lmap" => handle_foreach(cmd_args, param_set, traits),
        "while" => handle_while(cmd_args, param_set, traits),
        "for" => handle_for(cmd_args, param_set, traits),
        "after" => handle_after(cmd_args, param_set, traits),
        "scan" => handle_variadic_var_write(cmd_args, param_set, traits, 2),
        "lassign" => handle_variadic_var_write(cmd_args, param_set, traits, 1),
        "regexp" => handle_regexp_vars(cmd_args, param_set, traits),
        "regsub" => handle_regsub_var(cmd_args, param_set, traits),
        _ => {}
    }

    // (Variable-writing commands where a param is used directly as
    // the var name — `set`/`incr`/`append`/`lappend`/`global`/
    // `variable` etc. — are already covered by
    // `apply_arg_role_traits` above, which marks `ProcArgTrait::VarWrite`
    // for any arg whose registry `ArgRole` is `VarWrite`.  The old
    // hardcoded `var_write_index` name list was a redundant duplicate
    // of that registry query and has been removed.)

    // Track writes through upvar aliases — ``set local …`` where
    // ``local`` was registered as an alias for some param.
    if matches!(cmd_name, "set" | "incr" | "append" | "lappend") && !cmd_args.is_empty() {
        if let Some(target) = upvar_aliases.get(cmd_args[0].as_str()) {
            if let Some(set) = traits.get_mut(target) {
                set.insert(ProcArgTrait::VarWrite);
            }
        }
    }

    // foreach / lmap loop variables write through aliases.
    if matches!(cmd_name, "foreach" | "lmap") && cmd_args.len() >= 3 {
        let remaining = &cmd_args[..cmd_args.len() - 1];
        let mut i = 0;
        while i < remaining.len() {
            if let Some(target) = upvar_aliases.get(remaining[i].as_str()) {
                if let Some(set) = traits.get_mut(target) {
                    set.insert(ProcArgTrait::VarWrite);
                }
            }
            i += 2;
        }
    }
}

/// Per-arg role-driven trait recording — apply
/// ``ArgRole::Body`` / ``Expr`` / ``VarWrite`` / ``VarRead`` to
/// the matching parameter trait set when an arg is a simple
/// ``$param`` reference (or aliases an upvar'd one).
fn apply_arg_role_traits<'p>(
    cmd_name: &str,
    cmd_args: &[String],
    param_set: &HashSet<&'p str>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    upvar_aliases: &HashMap<String, &'p str>,
    registry: &CommandRegistry,
    stub_overlay: Option<&StubOverlay>,
) {
    let arg_roles = resolve_arg_roles(cmd_name, cmd_args, registry, stub_overlay);
    for (idx, arg) in cmd_args.iter().enumerate() {
        let Some(var_name) = extract_var_name(arg) else {
            continue;
        };
        let source_param = if let Some(p) = param_set.get(var_name) {
            *p
        } else if let Some(alias) = upvar_aliases.get(var_name) {
            *alias
        } else {
            continue;
        };
        let Ok(idx_u8) = u8::try_from(idx) else {
            continue;
        };
        let trait_to_add = match arg_roles.get(&idx_u8) {
            Some(ArgRole::Body) => ProcArgTrait::Body,
            Some(ArgRole::Expr) => ProcArgTrait::Expr,
            Some(ArgRole::VarWrite) => ProcArgTrait::VarWrite,
            Some(ArgRole::VarRead) => ProcArgTrait::VarRead,
            _ => continue,
        };
        if let Some(set) = traits.get_mut(source_param) {
            set.insert(trait_to_add);
        }
    }
}

/// Code-evaluating commands — ``eval`` / ``subst`` mark every
/// ``$param`` arg as ``Eval``; ``uplevel ?level? script`` marks
/// only the last arg.  Mirrors Python's ``spec.evaluates_code``
/// / ``spec.performs_substitution`` branch.
fn apply_eval_traits<'a>(
    cmd_name: &str,
    cmd_args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    let mark_as_eval = |vn: &str, traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>| {
        if let Some(p) = param_set.get(vn) {
            if let Some(set) = traits.get_mut(p) {
                set.insert(ProcArgTrait::Eval);
            }
        }
    };
    match cmd_name {
        "eval" | "subst" => {
            for arg in cmd_args {
                if let Some(vn) = extract_var_name(arg) {
                    mark_as_eval(vn, traits);
                }
            }
        }
        "uplevel" => {
            // ``uplevel ?level? script`` — last arg is the script.
            if let Some(last) = cmd_args.last() {
                if let Some(vn) = extract_var_name(last) {
                    mark_as_eval(vn, traits);
                }
            }
        }
        _ => {}
    }
}

fn handle_upvar<'p>(
    args: &[String],
    param_set: &HashSet<&'p str>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    upvar_aliases: &mut HashMap<String, &'p str>,
) {
    let mut start = 0usize;
    if !args.is_empty() {
        let head = args[0].as_str();
        if head.chars().all(|c| c.is_ascii_digit()) || head.starts_with('#') {
            start = 1;
        }
    }
    let mut i = start;
    while i + 1 < args.len() {
        let other_var = &args[i];
        let my_var = &args[i + 1];
        i += 2;

        if let Some(other_vn) = extract_var_name(other_var) {
            if let Some(p) = param_set.get(other_vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::VarRead);
                }
                upvar_aliases.insert(my_var.clone(), p);
            }
        }
        if let Some(my_vn) = extract_var_name(my_var) {
            if let Some(p) = param_set.get(my_vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::VarWrite);
                }
            }
        }
    }
}

fn handle_foreach<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 3 {
        return;
    }
    if let Some(body_vn) = extract_var_name(args.last().unwrap()) {
        if let Some(p) = param_set.get(body_vn).copied() {
            if let Some(set) = traits.get_mut(p) {
                set.insert(ProcArgTrait::Body);
            }
        }
    }
    let remaining = &args[..args.len() - 1];
    let mut i = 0;
    while i + 1 < remaining.len() {
        if let Some(list_vn) = extract_var_name(&remaining[i + 1]) {
            if let Some(p) = param_set.get(list_vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::LoopList);
                }
            }
        }
        i += 2;
    }
}

fn handle_while<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 2 {
        return;
    }
    if let Some(vn) = extract_var_name(&args[0]) {
        if let Some(p) = param_set.get(vn).copied() {
            if let Some(set) = traits.get_mut(p) {
                set.insert(ProcArgTrait::Expr);
            }
        }
    }
    if let Some(vn) = extract_var_name(&args[1]) {
        if let Some(p) = param_set.get(vn).copied() {
            if let Some(set) = traits.get_mut(p) {
                set.insert(ProcArgTrait::Body);
            }
        }
    }
}

fn handle_for<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 4 {
        return;
    }
    let pairs = [
        (&args[0], ProcArgTrait::Body),
        (&args[1], ProcArgTrait::Expr),
        (&args[2], ProcArgTrait::Body),
        (&args[3], ProcArgTrait::Body),
    ];
    for (arg, trait_) in pairs {
        if let Some(vn) = extract_var_name(arg) {
            if let Some(p) = param_set.get(vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(trait_);
                }
            }
        }
    }
}

fn handle_after<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 2 {
        return;
    }
    if matches!(args[0].as_str(), "cancel" | "info") {
        return;
    }
    let mut start = 1usize;
    if start < args.len() && args[start] == "-periodic" {
        start += 1;
    }
    for arg in &args[start..] {
        if let Some(vn) = extract_var_name(arg) {
            if let Some(p) = param_set.get(vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::Eval);
                }
            }
        }
    }
}

fn handle_variadic_var_write<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
    start: usize,
) {
    for arg in &args[start.min(args.len())..] {
        if let Some(vn) = extract_var_name(arg) {
            if let Some(p) = param_set.get(vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::VarWrite);
                }
            }
        }
    }
}

const REGEXP_SWITCHES: &[&str] = &[
    "-nocase",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-all",
    "-inline",
    "-indices",
    "--",
];
const REGEXP_VALUE_SWITCHES: &[&str] = &["-start"];

fn skip_regexp_switches(args: &[String]) -> usize {
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == "--" {
            return i + 1;
        }
        if REGEXP_SWITCHES.iter().any(|s| *s == args[i].as_str()) {
            i += 1;
        } else if REGEXP_VALUE_SWITCHES.iter().any(|s| *s == args[i].as_str()) {
            i += 2;
        } else {
            break;
        }
    }
    i
}

fn handle_regexp_vars<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    let pos = skip_regexp_switches(args);
    let var_start = pos + 2;
    handle_variadic_var_write(args, param_set, traits, var_start);
}

fn handle_regsub_var<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    let pos = skip_regexp_switches(args);
    let var_idx = pos + 3;
    if var_idx < args.len() {
        if let Some(vn) = extract_var_name(&args[var_idx]) {
            if let Some(p) = param_set.get(vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::VarWrite);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_trait(
        traits: &HashMap<String, HashSet<ProcArgTrait>>,
        param: &str,
        expected: ProcArgTrait,
    ) {
        let set = traits
            .get(param)
            .unwrap_or_else(|| panic!("no traits for {param}"));
        assert!(
            set.contains(&expected),
            "{param}: expected {expected:?}, got {set:?}",
        );
    }

    /// Test helper — builds the registry once, since the public
    /// API now requires the caller to thread one through.  No
    /// stub overlay; tests that need one construct it inline.
    fn infer(params: &[&str], body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
        let registry = CommandRegistry::build_default();
        infer_param_traits(params, body, &registry, None)
    }

    #[test]
    fn extract_var_name_simple() {
        assert_eq!(extract_var_name("$foo"), Some("foo"));
        assert_eq!(extract_var_name("${foo}"), Some("foo"));
        assert_eq!(extract_var_name("foo"), None);
        assert_eq!(extract_var_name("$"), None);
        assert_eq!(extract_var_name("$1abc"), None);
    }

    #[test]
    fn eval_param_records_eval_trait() {
        let traits = infer(&["body"], "eval $body");
        assert_trait(&traits, "body", ProcArgTrait::Eval);
    }

    #[test]
    fn uplevel_records_eval_on_last_arg_only() {
        let traits = infer(&["lvl", "script"], "uplevel $lvl $script");
        assert_trait(&traits, "script", ProcArgTrait::Eval);
        assert!(!traits
            .get("lvl")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval)));
    }

    #[test]
    fn foreach_records_loop_list_and_body() {
        let traits = infer(&["items", "body"], "foreach x $items $body");
        assert_trait(&traits, "items", ProcArgTrait::LoopList);
        assert_trait(&traits, "body", ProcArgTrait::Body);
    }

    #[test]
    fn while_records_expr_and_body() {
        let traits = infer(&["cond", "body"], "while $cond $body");
        assert_trait(&traits, "cond", ProcArgTrait::Expr);
        assert_trait(&traits, "body", ProcArgTrait::Body);
    }

    #[test]
    fn for_records_init_cond_next_body() {
        let traits = infer(&["i", "c", "n", "b"], "for $i $c $n $b");
        assert_trait(&traits, "i", ProcArgTrait::Body);
        assert_trait(&traits, "c", ProcArgTrait::Expr);
        assert_trait(&traits, "n", ProcArgTrait::Body);
        assert_trait(&traits, "b", ProcArgTrait::Body);
    }

    #[test]
    fn upvar_records_var_read_and_aliases_writes() {
        let traits = infer(&["var"], "upvar 1 $var local\nset local 1");
        assert_trait(&traits, "var", ProcArgTrait::VarRead);
        // Write through the alias upgrades to VarWrite.
        assert_trait(&traits, "var", ProcArgTrait::VarWrite);
    }

    #[test]
    fn lassign_records_var_writes() {
        let traits = infer(&["a", "b"], "lassign {1 2} $a $b");
        assert_trait(&traits, "a", ProcArgTrait::VarWrite);
        assert_trait(&traits, "b", ProcArgTrait::VarWrite);
    }

    #[test]
    fn after_records_eval_skipping_cancel_info() {
        let traits = infer(&["body"], "after 100 $body");
        assert_trait(&traits, "body", ProcArgTrait::Eval);
        let traits = infer(&["x"], "after cancel $x");
        // ``after cancel`` doesn't take a script, so $x is not eval.
        assert!(traits
            .get("x")
            .is_none_or(|s| !s.contains(&ProcArgTrait::Eval)));
    }

    #[test]
    fn regsub_records_var_write() {
        let traits = infer(&["out"], "regsub -all foo $line bar $out");
        assert_trait(&traits, "out", ProcArgTrait::VarWrite);
    }

    #[test]
    fn empty_body_returns_empty_map() {
        let traits = infer(&["a"], "");
        assert!(traits.is_empty());
    }

    /// Deep-pass helper that mirrors [`infer`] for ergonomics.
    fn infer_deep(params: &[&str], body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
        let registry = CommandRegistry::build_default();
        infer_param_traits_deep(params, body, &registry, None)
    }

    #[test]
    fn deep_pass_surfaces_eval_trait_inside_braced_body() {
        // `$body` is used inside a nested `foreach` body — the
        // shallow pass walks only top-level commands, so it
        // misses the trait.  The deep pass descends into the
        // braced `foreach` body and surfaces `Eval`.
        let body = "foreach item $items {\n  uplevel 1 $body\n}";
        let shallow = infer(&["items", "body"], body);
        let deep = infer_deep(&["items", "body"], body);
        // Shallow catches `items` (LoopList) but misses `body`.
        assert_trait(&shallow, "items", ProcArgTrait::LoopList);
        assert!(
            !shallow
                .get("body")
                .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
            "shallow pass should not surface nested Eval, got {shallow:?}",
        );
        // Deep catches both.
        assert_trait(&deep, "items", ProcArgTrait::LoopList);
        assert_trait(&deep, "body", ProcArgTrait::Eval);
    }

    #[test]
    fn deep_pass_descends_through_multiple_levels() {
        // `$inner` is buried two levels deep: `if` → `while` →
        // `eval $inner`.  Shallow misses it; deep finds it.
        let body = "if {1} {\n  while {1} {\n    eval $inner\n  }\n}";
        let deep = infer_deep(&["inner"], body);
        assert_trait(&deep, "inner", ProcArgTrait::Eval);
    }

    #[test]
    fn deep_pass_respects_max_depth() {
        // Build a body nested past `MAX_DEPTH` (8 levels of
        // `if {1} { ... }`) with `eval $deep_var` at the
        // innermost level.  The recursion should stop before
        // reaching the innermost level and the trait should not
        // be surfaced.  Using `MAX_DEPTH + 2` (10) levels of
        // nesting puts the eval below the recursion bound.
        let depth_to_nest = usize::from(MAX_DEPTH) + 2;
        let mut body = String::from("eval $deep_var");
        for _ in 0..depth_to_nest {
            body = format!("if {{1}} {{ {body} }}");
        }
        let deep = infer_deep(&["deep_var"], &body);
        assert!(
            !deep
                .get("deep_var")
                .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
            "MAX_DEPTH bound should keep deeply-nested eval from being surfaced, got {deep:?}",
        );
    }

    #[test]
    fn deep_pass_skips_dynamic_body_args() {
        // `if {1} $body` — the body is a `$var` reference,
        // not a braced literal.  The deep pass shouldn't try
        // to descend into it (we have no body text to scan);
        // the shallow pass already surfaces the `Body` trait
        // via the registry's role scan.  This pins that
        // contract: dynamic body args don't double-count.
        let body = "if {1} $body";
        let deep = infer_deep(&["body"], body);
        assert_trait(&deep, "body", ProcArgTrait::Body);
    }

    #[test]
    fn merge_traits_unions_shallow_and_deep() {
        let mut shallow: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
        shallow
            .entry("p1".into())
            .or_default()
            .insert(ProcArgTrait::VarRead);
        let mut deep: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
        deep.entry("p1".into())
            .or_default()
            .insert(ProcArgTrait::Eval);
        deep.entry("p2".into())
            .or_default()
            .insert(ProcArgTrait::Body);

        let merged = merge_traits(shallow, deep);
        // p1 gains Eval from deep without losing VarRead from shallow.
        assert!(merged.get("p1").unwrap().contains(&ProcArgTrait::VarRead));
        assert!(merged.get("p1").unwrap().contains(&ProcArgTrait::Eval));
        // p2 (deep-only) lands in the merged map.
        assert!(merged.get("p2").unwrap().contains(&ProcArgTrait::Body));
    }

    #[test]
    fn merge_traits_with_empty_deep_returns_shallow_unchanged() {
        let mut shallow: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
        shallow
            .entry("p1".into())
            .or_default()
            .insert(ProcArgTrait::VarWrite);
        let merged = merge_traits(shallow.clone(), HashMap::new());
        assert_eq!(merged.get("p1"), shallow.get("p1"));
    }

    #[test]
    fn deep_pass_matches_shallow_for_top_level_only_bodies() {
        // When there are no nested bodies, the deep pass
        // should return exactly what the shallow pass does.
        let body = "set $x 1\nupvar 1 $var local";
        let shallow = infer(&["x", "var"], body);
        let deep = infer_deep(&["x", "var"], body);
        assert_eq!(shallow, deep);
    }

    #[test]
    fn deep_pass_empty_params_returns_empty_map() {
        let deep = infer_deep(&[], "foreach item $items { uplevel 1 $body }");
        assert!(deep.is_empty());
    }

    // -- stub-overlay integration ------------------------------------
    //
    // These tests pin the contract that a non-empty
    // [`StubOverlay`] threaded through `infer_param_traits` /
    // `infer_param_traits_deep` lets user-declared
    // `# tcl-lsp: stub` commands participate in role-driven
    // trait inference alongside the built-in registry.

    use tcl_registry::stub_overlay::{StubArg, StubSig};

    fn make_overlay(sigs: Vec<StubSig>) -> StubOverlay {
        let mut o = StubOverlay::new();
        for s in sigs {
            o.insert(s);
        }
        o
    }

    fn stub_sig(name: &str, args: &[(&str, ArgRole)]) -> StubSig {
        use tcl_registry::stub_overlay::StubSigFlags;
        StubSig {
            name: name.to_string(),
            args: args
                .iter()
                .map(|(n, r)| StubArg {
                    name: (*n).to_string(),
                    role: *r,
                    optional: false,
                })
                .collect(),
            flags: StubSigFlags::empty(),
        }
    }

    #[test]
    fn overlay_shallow_surfaces_stub_declared_body_role() {
        // `my_eval` isn't in the built-in registry, but the
        // overlay declares its arg-0 as `body`.  An invocation
        // `my_eval $script` should therefore surface `Body` on
        // the `script` param.
        let overlay = make_overlay(vec![stub_sig("my_eval", &[("script", ArgRole::Body)])]);
        let registry = CommandRegistry::build_default();
        let traits = infer_param_traits(&["script"], "my_eval $script", &registry, Some(&overlay));
        assert_trait(&traits, "script", ProcArgTrait::Body);
    }

    #[test]
    fn overlay_shallow_surfaces_stub_declared_var_write_role() {
        // Stub-declared `VarWrite` on a parameter should
        // surface `VarWrite` on the matching param.
        let overlay = make_overlay(vec![stub_sig(
            "with_var",
            &[("varName", ArgRole::VarWrite), ("value", ArgRole::Value)],
        )]);
        let registry = CommandRegistry::build_default();
        let traits = infer_param_traits(&["v"], "with_var $v 42", &registry, Some(&overlay));
        assert_trait(&traits, "v", ProcArgTrait::VarWrite);
    }

    #[test]
    fn overlay_deep_recurses_through_stub_body_args() {
        // The overlay declares `my_loop`'s arg-1 as a body.
        // Without the overlay the deep pass can't see that
        // `my_loop { uplevel 1 $body }` carries an Eval inside.
        // With the overlay it should descend into the brace
        // and surface the Eval.
        let overlay = make_overlay(vec![stub_sig(
            "my_loop",
            &[("count", ArgRole::Value), ("body", ArgRole::Body)],
        )]);
        let registry = CommandRegistry::build_default();
        let body = "my_loop 5 { uplevel 1 $script }";
        // Sanity: without the overlay, the deep pass misses
        // the nested Eval because `my_loop` isn't recognised.
        let no_overlay = infer_param_traits_deep(&["script"], body, &registry, None);
        assert!(
            !no_overlay
                .get("script")
                .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
            "without overlay, my_loop body shouldn't be recognised, got {no_overlay:?}",
        );
        // With the overlay, the recursion fires.
        let with_overlay = infer_param_traits_deep(&["script"], body, &registry, Some(&overlay));
        assert_trait(&with_overlay, "script", ProcArgTrait::Eval);
    }

    #[test]
    fn overlay_does_not_disturb_registry_resolution() {
        // An overlay covering `my_thing` mustn't shadow any
        // built-in command.  A built-in `foreach` invocation
        // still records its `LoopList` / `Body` traits via the
        // registry path even when an unrelated stub overlay is
        // active.
        let overlay = make_overlay(vec![stub_sig("my_thing", &[("a", ArgRole::Body)])]);
        let registry = CommandRegistry::build_default();
        let traits = infer_param_traits(
            &["items", "body"],
            "foreach x $items $body",
            &registry,
            Some(&overlay),
        );
        assert_trait(&traits, "items", ProcArgTrait::LoopList);
        assert_trait(&traits, "body", ProcArgTrait::Body);
    }

    #[test]
    fn overlay_none_matches_overlay_empty() {
        // An empty overlay should produce the same result as
        // `None` — the overlay is a no-op when it has no
        // entries.
        let registry = CommandRegistry::build_default();
        let body = "foreach x {1 2 3} $body";
        let none = infer_param_traits(&["body"], body, &registry, None);
        let empty = infer_param_traits(&["body"], body, &registry, Some(&StubOverlay::new()));
        assert_eq!(none, empty);
    }
}
