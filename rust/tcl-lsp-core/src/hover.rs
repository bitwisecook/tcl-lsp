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

//! Hover provider.
//!
//! Resolves the word or `$var` reference at a given LSP position
//! and produces a [`Hover`] with markdown-formatted content for
//! one of:
//!
//! * a user-defined `proc` whose name (or fully-qualified name)
//!   matches the cursor word — formats the signature plus the
//!   harvested doc-comment;
//! * a `TclOO` class whose name matches — formats the
//!   metaclass-qualified declaration plus method / property
//!   summaries;
//! * a `$var` reference whose name resolves through the
//!   enclosing-scope chain to a [`VarDef`] — formats the
//!   reference count.
//!
//! Inferred-interp / taint annotations on `$var` hovers: when a
//! registry is supplied the provider builds a compiler
//! [`tcl_compiler::compilation_unit::CompilationUnit`] and reads
//! the per-variable type / taint lattices off it.
//!
//! Cache + debounce + `spawn_blocking` + `Ok(None)`-on-no-cached-
//! analysis ride on top of this provider in
//! `tcl-lsp-server::Backend::hover`; this module is the pure-CPU
//! computation, no I/O, no async.

use rustc_hash::FxHashSet;
use tcl_compiler::analyser::{AnalysisResult, ClassDef, ProcDef, VarDef};
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::taint::{TaintColour, TaintLattice};
use tcl_compiler::types::{TclType, TypeKind, TypeLattice};
use tcl_registry::CommandRegistry;

use crate::definition::utf16_col_to_char_col;

/// LSP markup-content kind for a hover body.
///
/// We only emit Markdown today; the variant exists so the lift in
/// `tcl-lsp-server` is exhaustive when we add `PlainText` support
/// later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverKind {
    /// GitHub-flavoured Markdown, suitable for VS Code rendering.
    Markdown,
}

/// A single hover result — markdown-formatted body.
///
/// The subset this provider emits today carries no `range` and no
/// `PlainText`.  The lift in `tcl-lsp-server` materialises this
/// onto `ls_types::Hover`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    /// Markdown body of the hover.
    pub value: String,
    /// Markup kind. Always `Markdown` for the minimal port.
    pub kind: HoverKind,
}

impl Hover {
    fn markdown(value: String) -> Self {
        Self {
            value,
            kind: HoverKind::Markdown,
        }
    }
}

/// Word-delimiter set used by `find_word_span_at_position`.
const WORD_DELIMS: &[char] = &[' ', '\t', '\n', ';', '{', '}', '[', ']', '"', '$'];

/// Scan a bare `$name` variable name in `chars` starting at `start`, returning
/// the end index (exclusive).
///
/// A name char is an alphanumeric or `_`.  A `:` is part of the name **only**
/// as a namespace qualifier `::` — matching C Tcl's `Tcl_ParseVarName`, which
/// consumes the whole colon run once a `::` starts it (`$a:::b` → `a:::b`) but
/// stops at a *lone* `:`.  So `$host:$port` resolves `host`, not `host:`
/// (issue 183).
fn scan_var_name_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() {
        let c = chars[end];
        if c.is_alphanumeric() || c == '_' {
            end += 1;
        } else if c == ':' && chars.get(end + 1) == Some(&':') {
            end += 2;
            while chars.get(end) == Some(&':') {
                end += 1;
            }
        } else {
            break;
        }
    }
    end
}

/// Compute hover text for a position in `source`.
///
/// `analysis` is the pre-computed analyser result; the caller is
/// expected to cache it. Returns `None` when:
///
/// * `line` / `character` falls outside the source extents,
/// * the cursor isn't on any recognisable identifier or `$var`,
/// * no proc / class / var matches the resolved word.
///
/// The character index is interpreted as UTF-16 code units per
/// the LSP spec, but this provider treats it as a char-count
/// index.  Multi-byte BMP code points round-trip correctly;
/// supplementary-plane characters can drift by one position
/// (rare in Tcl source).
#[must_use]
pub fn hover(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Option<Hover> {
    // Dialect-agnostic entry point (every subcommand is a resolution
    // candidate).  Production callers that know the document's Tcl version
    // should prefer [`hover_with_dialect`] so a prefix's uniqueness matches the
    // version (see [`tcl_registry::CommandSpec::resolve_subcommand_for_dialect`]).
    hover_with_dialect(
        source,
        line,
        character,
        analysis,
        registry,
        tcl_registry::dialects::DialectSet::ALL_TCL,
    )
}

/// [`hover`] resolving prefix-abbreviated subcommands against a specific Tcl
/// `dialect`, so e.g. `info class def` hovers `definition` under 8.6 but
/// nothing (ambiguous with `definitionnamespace`) under 9.0.
pub fn hover_with_dialect(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    dialect: tcl_registry::dialects::DialectSet,
) -> Option<Hover> {
    // One index shared by the position conversions below.
    let line_index = tcl_lexer::LineIndex::new(source);

    // Variable hover takes precedence — `$var` resolution sits
    // at a position where `find_word_span_at_position` would
    // also match the unqualified name, but a `$`-led ref should
    // surface the [`VarDef`] not the (typically absent) proc of
    // the same name.
    if let Some(var_name) = find_var_at_position(source, line, character) {
        let var_byte_offset =
            crate::definition::byte_offset_at(&line_index, source, line, character);
        // Use the byte-offset scope-chain lookup (the local line-based helper
        // mis-resolves namespace/proc-scoped vars).
        if let Some(var_def) = crate::definition::lookup_var_in_scope_chain(
            &analysis.global_scope,
            var_byte_offset,
            &var_name,
        ) {
            // Inferred-intrep / taint annotations need the compiler
            // pipeline (`CompilationUnit`), which requires a
            // registry; without one we surface just the reference
            // count.
            let (type_info, taint_info) = match registry {
                Some(reg) => infer_var_type_and_taint(source, reg, &var_name),
                None => (None, None),
            };
            return Some(Hover::markdown(var_hover_text(
                var_def,
                type_info.as_deref(),
                taint_info.as_deref(),
            )));
        }
        // No user definition: an interpreter-provided special variable
        // (`auto_path`, `env`, `tcl_platform`, the iRules `static::` namespace)
        // still has documentation, sourced from the dialect-aware
        // special-variable registry.  The `(idx)` array index is already
        // stripped by `find_var_at_position`, so `$tcl_platform(os)` resolves
        // to the `tcl_platform` spec.
        if let Some(spec) = tcl_registry::special_var(&var_name).filter(|s| s.available_in(dialect))
        {
            return Some(Hover::markdown(special_var_hover_text(spec, dialect)));
        }
    }

    // Format-string hover: when the cursor
    // sits on the format-string argument of a known
    // format-bearing command, surface a markdown table of the
    // specifiers it contains.  Currently covers `clock format`
    // / `clock scan` and `format` / `scan`.
    if let Some(text) = clock_format_string_at_position(source, line, character) {
        return Some(Hover::markdown(clock_format_hover_text(&text)));
    }
    if let Some(text) = sprintf_format_string_at_position(source, line, character) {
        return Some(Hover::markdown(sprintf_format_hover_text(&text)));
    }
    if let Some(ctx) = binary_format_context_at_position(source, line, character) {
        return Some(Hover::markdown(binary_format_hover_text(&ctx)));
    }
    if let Some(text) = regsub_subspec_at_position(source, line, character) {
        return Some(Hover::markdown(regsub_hover_text(&text)));
    }
    if let Some(text) = glob_pattern_at_position(source, line, character) {
        return Some(Hover::markdown(glob_hover_text(&text)));
    }
    if let Some(text) = regex_pattern_at_position(source, line, character) {
        return Some(Hover::markdown(regex_hover_text(&text)));
    }

    let (word, _start, _end) = find_word_span_at_position(source, line, character)?;

    // `$obj method` dispatch — when the cursor sits on the
    // method-name token of an instance-method call and the
    // instance's class is known, render the method summary.
    // Checked before the proc lookup so a method call wins over
    // a same-named proc.
    if let Some((inst, method, is_dollar)) =
        crate::definition::instance_method_at_cursor(source, line, character)
        && let Some(class_q) =
            crate::definition::receiver_instance_class(analysis, &inst, is_dollar)
        && let Some(text) = obj_method_hover_text(analysis, class_q, &method, registry)
    {
        return Some(Hover::markdown(text));
    }

    // Command alias (`interp alias {} = {} expr`) — show the resolved target.
    if let Some(text) = alias_hover_text(analysis, &word) {
        return Some(Hover::markdown(text));
    }

    if let Some(proc_def) = lookup_proc(analysis, &word) {
        return Some(Hover::markdown(proc_hover_text(proc_def)));
    }

    if let Some(class_def) = lookup_class(analysis, &word) {
        return Some(Hover::markdown(class_hover_text(analysis, class_def)));
    }

    // Class-member hover — same dispatch as
    // [`crate::definition::lookup_class_member`], rendered as
    // a one-line method / property summary.  Fires when the
    // cursor sits inside a class body and `word` matches one
    // of that class's members.
    let cursor_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some(text) = class_member_hover_text(analysis, &word, cursor_offset) {
        return Some(Hover::markdown(text));
    }
    // Scoped command environments (a `report::defstyle` style script) win over
    // a same-named global command inside their body — resolved from the
    // analyser's recorded body regions, not the registry.
    if let Some(text) =
        scoped_command_hover_text(source, line, character, analysis, &word, cursor_offset)
    {
        return Some(Hover::markdown(text));
    }

    // Registry-driven hovers — built-in command name, plus
    // `cmd subcommand` lookups when the cursor sits on the
    // subcommand word.
    if let Some(registry) = registry {
        if let Some(text) = option_hover_text(source, line, character, registry, &word, dialect) {
            return Some(Hover::markdown(text));
        }
        if let Some(text) =
            sub_subcommand_hover_text(source, line, character, registry, &word, dialect)
        {
            return Some(Hover::markdown(text));
        }
        if let Some(text) = subcommand_hover_text(source, line, character, registry, &word, dialect)
        {
            return Some(Hover::markdown(text));
        }
        if let Some(text) = builtin_command_hover_text(registry, &word, analysis, cursor_offset) {
            return Some(Hover::markdown(text));
        }
    }

    if let Some(text) = ip_address_hover_text(&word) {
        return Some(Hover::markdown(text));
    }

    None
}

/// Resolve an unqualified command `name` at byte offset `cursor_offset` to its
/// qualified registry spec via the document's `namespace import`s
/// (`namespace import ::tcltest::*` → `test` resolves to `tcltest::test`).
/// Mirrors the analyser's imported-command body resolution, respecting the
/// import's **scope** and **source order**: only an import made at global scope
/// (`ns == "::"`, in effect everywhere via Tcl's unqualified-name fallback) and
/// positioned *before* the hovered word applies — a nested-namespace import
/// (e.g. inside `namespace eval ns { … }`) or one appearing after the cursor
/// must not retroactively resolve a bare name here.  Returns the qualified name
/// and its spec, or `None`.  Only unqualified names are resolved.
fn resolve_imported_command<'r>(
    registry: &'r CommandRegistry,
    name: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<(String, &'r tcl_registry::CommandSpec)> {
    if name.contains("::") {
        return None;
    }
    // Iterate imports newest-first (`namespace_imports` is in source order): when
    // several in-scope imports could provide the same bare name, Tcl's later
    // `namespace import` (notably `-force`) wins, so the most recent one before
    // the cursor is the effective binding.
    for imp in analysis.namespace_imports.iter().rev() {
        if imp.ns != "::" || imp.range.end() > cursor_offset {
            continue;
        }
        let candidate = if let Some(prefix) = imp.pattern.strip_suffix('*') {
            format!("{prefix}{name}")
        } else if imp.pattern.rsplit("::").next() == Some(name) {
            imp.pattern.clone()
        } else {
            continue;
        };
        if let Some(spec) = registry.get(&candidate) {
            return Some((candidate, spec));
        }
    }
    None
}

/// Render a hover snippet for a built-in command name.
/// Looks up `name` in the registry, uses the matched spec's
/// `hover.summary` / `synopsis` to produce a markdown block.
fn builtin_command_hover_text(
    registry: &CommandRegistry,
    name: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<String> {
    use std::borrow::Cow;
    use std::fmt::Write;
    // Resolve the head through the document's `namespace import`s: a bare
    // `test` under `namespace import ::tcltest::*` hovers as `tcltest::test`
    // (mirrors the analyser's imported-command body resolution, scoped + ordered
    // to imports actually in effect at the cursor).
    let (name, spec): (Cow<'_, str>, _) = if let Some(spec) = registry.get(name) {
        (Cow::Borrowed(name), spec)
    } else {
        let (qual, spec) = resolve_imported_command(registry, name, analysis, cursor_offset)?;
        (Cow::Owned(qual), spec)
    };
    let name = name.as_ref();
    let hover = spec.hover.as_ref()?;
    let mut out = format!("**`{name}`** — built-in command\n");
    if !hover.summary.is_empty() {
        let _ = write!(out, "\n{}\n", hover.summary);
    }
    if let Some(synopsis) = hover.synopsis.first() {
        let _ = write!(out, "\n```tcl\n{synopsis}\n```\n");
    }
    if !spec.subcommands.is_empty() {
        let mut names: Vec<&str> = spec.subcommands.iter().map(|s| s.name).collect();
        names.sort_unstable();
        let joined = names.join(", ");
        let _ = write!(out, "\nSubcommands: {joined}\n");
    }
    // Import hint: when the command needs a `package require` the
    // document hasn't imported, append a `**Requires**` line.  Gated on
    // the dialect supporting `package require` at all (the `package`
    // command must exist — e.g. iRules has no package system).
    if let Some(pkg) = spec.required_package {
        let pkg_available = registry.get("package").is_some();
        let imported = analysis.package_requires.iter().any(|pr| pr.name == pkg);
        if pkg_available && !imported {
            let _ = write!(out, "\n**Requires**: `package require {pkg}`");
        }
    }
    // iRules: append the **Valid events** list + event **Requires**.
    // Only iRules commands carry `event_requires`, so its presence is the
    // dialect gate.
    if let Some(requires) = spec.event_requires.as_ref() {
        let effective = effective_event_requires(name, requires);
        append_valid_events(&mut out, &effective);
    } else if let Some((prefix, _)) = name.split_once("::") {
        // Namespace-only command (no `event_requires`): if its protocol
        // namespace declares profiles (e.g. `ACCESS::log` → ACCESS), surface
        // a `**Requires**` profile line.
        let profile_reg = tcl_registry::profiles::ProfileRegistry::build();
        if let Some(ns) = profile_reg.get_namespace(prefix)
            && !ns.profiles.is_empty()
        {
            let _ = write!(out, "\n\n**Requires**: {} profile", ns.profiles.join(", "));
        }
    }
    Some(out)
}

/// Augment a command's `EventRequires` with namespace-backed profile
/// metadata.  For an iRules `NS::cmd` whose own `profiles` are empty, the
/// profiles declared by the `NS` protocol namespace are substituted, so
/// the **Valid events** list narrows and a profile **Requires** line is
/// shown (e.g. `DIAMETER::*`, `MQTT::*`, `LDAP::*`, `IMAP::*`).  The
/// f5-irules gate is implicit — only iRules specs carry `event_requires`.
fn effective_event_requires(
    command_name: &str,
    requires: &tcl_registry::events::EventRequires,
) -> tcl_registry::events::EventRequires {
    // A command with its own profiles, or no `NS::` qualifier, is unchanged.
    if !requires.profiles.is_empty() {
        return requires.clone();
    }
    let Some((prefix, _)) = command_name.split_once("::") else {
        return requires.clone();
    };
    let profile_reg = tcl_registry::profiles::ProfileRegistry::build();
    match profile_reg.get_namespace(prefix) {
        Some(ns) if !ns.profiles.is_empty() => tcl_registry::events::EventRequires {
            profiles: ns.profiles,
            ..*requires
        },
        _ => requires.clone(),
    }
}

/// Append the iRules "**Valid events**" list (first 8 + total) and the
/// event "**Requires**" line (sides / transport / profiles) for a
/// command's `EventRequires`.
fn append_valid_events(out: &mut String, requires: &tcl_registry::events::EventRequires) {
    use std::fmt::Write;
    let matching = valid_events(requires);
    if !matching.is_empty() {
        let mut list = matching
            .iter()
            .take(8)
            .map(|e| format!("`{e}`"))
            .collect::<Vec<_>>()
            .join(", ");
        if matching.len() > 8 {
            let _ = write!(list, ", ... ({} total)", matching.len());
        }
        let _ = write!(out, "\n\n**Valid events**: {list}");
    }
    let mut reqs: Vec<String> = Vec::new();
    if requires.client_side {
        reqs.push("client-side".to_owned());
    }
    if requires.server_side {
        reqs.push("server-side".to_owned());
    }
    if let Some(t) = requires.transport {
        reqs.push(t.to_uppercase());
    }
    if !requires.profiles.is_empty() {
        let mut profs: Vec<&str> = requires.profiles.to_vec();
        profs.sort_unstable();
        reqs.push(format!("profile {}", profs.join(" or ")));
    }
    if !reqs.is_empty() {
        let _ = write!(out, "\n\n**Requires**: {}", reqs.join(", "));
    }
}

/// Sorted event names whose properties satisfy `requires`.
fn valid_events(requires: &tcl_registry::events::EventRequires) -> Vec<String> {
    let event_reg = tcl_registry::events::EventRegistry::build();
    let profile_reg = tcl_registry::profiles::ProfileRegistry::build();
    let mut out: Vec<String> = event_reg
        .all_event_names()
        .into_iter()
        .filter(|name| {
            event_reg.get_props(name).is_some_and(|p| {
                tcl_registry::events::event_satisfies(p, requires, name, &profile_reg)
            })
        })
        .map(ToOwned::to_owned)
        .collect();
    out.sort_unstable();
    out
}

/// Render a hover snippet for a `cmd subcommand` pair when
/// the cursor sits on the subcommand word.  Detects the
/// surrounding command segment via single-line tokenisation
/// (mirrors the `command_context_on_line` helper used by
/// completion / signature-help).
fn subcommand_hover_text(
    source: &str,
    line: u32,
    character: u32,
    registry: &CommandRegistry,
    cursor_word: &str,
    dialect: tcl_registry::dialects::DialectSet,
) -> Option<String> {
    use std::fmt::Write;
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let prefix: String = chars[..col].iter().collect();
    let tokens: Vec<&str> = prefix.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let cmd_name = tokens[0];
    // The cursor word IS the subcommand — use it directly as
    // the lookup key.  The prefix-tokenised second token might
    // be a partial (if cursor is mid-word).
    let sub_name = cursor_word;
    if cmd_name == sub_name {
        // Cursor sits on the command word itself, not on a
        // subcommand.  Fall through to the built-in-command
        // hover instead.
        return None;
    }
    let spec = registry.get(cmd_name)?;
    // Resolve unique-prefix abbreviations (`string le` ⇒ `length`) like Tcl,
    // honouring the active dialect for the prefix's uniqueness.
    let sub = spec.resolve_subcommand_for_dialect(sub_name, dialect)?;
    let mut out = format!("**`{cmd_name} {}`** — subcommand\n", sub.name);
    if let Some(hover) = sub.hover.as_ref() {
        if !hover.summary.is_empty() {
            let _ = write!(out, "\n{}\n", hover.summary);
        }
        if let Some(synopsis) = hover.synopsis.first() {
            let _ = write!(out, "\n```tcl\n{synopsis}\n```\n");
        }
    } else {
        let _ = write!(out, "\nSubcommand of `{cmd_name}`.\n");
    }
    Some(out)
}

/// Render a hover for a command inside a scoped command environment — a
/// `report::defstyle` style script exposing the report configuration methods
/// (`top`, `data`, `columns`, …) and their operations (`top set`, `top
/// enable`).  Resolves against the analyser-recorded
/// [`ScopedBodyRegion`](tcl_compiler::analyser::ScopedBodyRegion)s active at the
/// cursor; the scoped command set is registry data, so no command name is
/// matched here.
fn scoped_command_hover_text(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    cursor_word: &str,
    cursor_offset: u32,
) -> Option<String> {
    use std::fmt::Write;
    let env = analysis
        .scoped_command_regions
        .iter()
        .find(|r| r.contains(cursor_offset))
        .map(|r| r.env)?;
    // Head-command hover — the cursor sits on a scoped command head.
    if let Some(cmd) = env.command(cursor_word) {
        let mut out = format!("**`{cursor_word}`** — {} command\n", env.name);
        if let Some(hover) = cmd.hover.as_ref() {
            if !hover.summary.is_empty() {
                let _ = write!(out, "\n{}\n", hover.summary);
            }
            if !hover.snippet.is_empty() {
                let _ = write!(out, "\n```tcl\n{}\n```\n", hover.snippet);
            }
        } else if !cmd.detail.is_empty() {
            let _ = write!(out, "\n{}\n", cmd.detail);
        }
        if !cmd.subcommands.is_empty() {
            let names: Vec<&str> = cmd.subcommands.iter().map(|s| s.name).collect();
            let _ = write!(out, "\nOperations: {}\n", names.join(", "));
        }
        return Some(out);
    }
    // Ensemble-operation hover — the cursor sits on the operation word
    // (`set` / `enable`), whose head is the line's first token.
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let prefix: String = chars[..col].iter().collect();
    let head = prefix.split_whitespace().next()?;
    if head == cursor_word {
        return None;
    }
    let cmd = env.command(head)?;
    let sub = cmd.subcommand(cursor_word)?;
    let mut out = format!("**`{head} {}`** — {} operation\n", sub.name, env.name);
    if !sub.detail.is_empty() {
        let _ = write!(out, "\n{}\n", sub.detail);
    }
    if !sub.synopsis.is_empty() {
        let _ = write!(out, "\n```tcl\n{}\n```\n", sub.synopsis);
    }
    Some(out)
}

/// Render a hover for the third word of a two-level ensemble — the
/// second-level subcommand of `info object <op>` / `info class <op>` (issue
/// #798) — when the cursor sits on it.  Accepts a unique prefix (`info object
/// cl` ⇒ `class`), matching Tcl's ensemble dispatch.
fn sub_subcommand_hover_text(
    source: &str,
    line: u32,
    character: u32,
    registry: &CommandRegistry,
    cursor_word: &str,
    dialect: tcl_registry::dialects::DialectSet,
) -> Option<String> {
    use std::fmt::Write;
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let prefix: String = chars[..col].iter().collect();
    let tokens: Vec<&str> = prefix.split_whitespace().collect();
    // Need at least the command and its first-level subcommand before the
    // cursor word (`info object …`).
    if tokens.len() < 2 {
        return None;
    }
    let cmd_name = tokens[0];
    let sub_name = tokens[1];
    // The cursor must be on the *third* word, not the command or the
    // first-level subcommand.
    if cursor_word == cmd_name || cursor_word == sub_name {
        return None;
    }
    let spec = registry.get(cmd_name)?;
    let sub = spec.resolve_subcommand_for_dialect(sub_name, dialect)?;
    let ss = sub.resolve_sub_subcommand_for_dialect(cursor_word, dialect)?;
    let mut out = format!("**`{cmd_name} {} {}`** — subcommand\n", sub.name, ss.name);
    if !ss.detail.is_empty() {
        let _ = write!(out, "\n{}\n", ss.detail);
    }
    if !ss.synopsis.is_empty() {
        let _ = write!(out, "\n```tcl\n{}\n```\n", ss.synopsis);
    }
    Some(out)
}

/// Render a hover snippet for a `-option` when the cursor sits on an option
/// word of the surrounding command.  `cursor_word` is the identifier under
/// the cursor (no leading `-`); the dash is detected on the line.
fn option_hover_text(
    source: &str,
    line: u32,
    character: u32,
    registry: &CommandRegistry,
    _cursor_word: &str,
    dialect: tcl_registry::dialects::DialectSet,
) -> Option<String> {
    use std::fmt::Write;
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    // The option word run (dash-led identifier) containing the cursor.
    let is_opt_char = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    let mut start = col.min(chars.len());
    while start > 0 && is_opt_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_opt_char(chars[end]) {
        end += 1;
    }
    // The run must begin with a `-` to be an option.
    if start >= end || chars[start] != '-' {
        return None;
    }
    let option: String = chars[start..end].iter().collect();
    // The surrounding command (and, if present, its subcommand) are the
    // whitespace-delimited tokens before the option run.
    let prefix: String = chars[..start].iter().collect();
    let mut words = prefix.split_whitespace();
    let cmd_name = words.next()?;
    let spec = registry.get(cmd_name)?;
    // Resolve the subcommand-scoped option table (`chan configure
    // -inputmode`) before falling back to the command's own top-level
    // table — an ensemble's real options live on the subcommand, and only
    // that table is dialect-correct for a subcommand-specific option.
    let (options, parent_dialects, owner) = match words
        .next()
        .and_then(|sub_name| spec.resolve_subcommand_for_dialect(sub_name, dialect))
    {
        Some(sub) => (
            sub.options,
            sub.dialects.or(spec.dialects),
            format!("{cmd_name} {}", sub.name),
        ),
        None => (spec.options, spec.dialects, cmd_name.to_owned()),
    };
    let opt = options.iter().find(|o| o.matches(option.as_str()))?;
    let mut out = format!("**`{}`** — option of `{owner}`\n", opt.name);
    if !opt.detail.is_empty() {
        let _ = write!(out, "\n{}\n", opt.detail);
    }
    if opt.takes_value() && !opt.value_hint().is_empty() {
        let _ = write!(out, "\nTakes a `{}` value.\n", opt.value_hint());
    }
    if !opt.supports_dialect(Some(dialect), parent_dialects) {
        let _ = write!(out, "\n_Not available in the active dialect._\n");
    }
    Some(out)
}

/// Strftime specifier descriptions for clock-format hover.
const CLOCK_SPEC_DESC: &[(char, &str)] = &[
    ('a', "Abbreviated weekday name"),
    ('A', "Full weekday name"),
    ('b', "Abbreviated month name"),
    ('B', "Full month name"),
    ('c', "Locale date and time"),
    ('C', "Century (00–99)"),
    ('d', "Day of month (01–31)"),
    ('D', "Date as %m/%d/%Y"),
    ('e', "Day of month (1–31, no leading zero)"),
    ('g', "ISO 8601 2-digit year"),
    ('G', "ISO 8601 4-digit year"),
    ('h', "Abbreviated month name (same as %b)"),
    ('H', "Hour (00–23)"),
    ('I', "Hour (01–12)"),
    ('j', "Day of year (001–366)"),
    ('J', "Julian day number"),
    ('k', "Hour (0–23, no leading zero)"),
    ('l', "Hour (1–12, no leading zero)"),
    ('m', "Month (01–12)"),
    ('M', "Minute (00–59)"),
    ('N', "Month number (1–12, no leading zero)"),
    ('p', "AM/PM indicator (uppercase)"),
    ('P', "AM/PM indicator (lowercase)"),
    ('s', "Seconds since Unix epoch"),
    ('S', "Second (00–59)"),
    ('u', "Day of week (1=Monday–7=Sunday)"),
    ('U', "Week number (Sunday start, 00–53)"),
    ('V', "ISO 8601 week number (01–53)"),
    ('w', "Day of week (0=Sunday–6=Saturday)"),
    ('W', "Week number (Monday start, 00–53)"),
    ('x', "Locale date representation"),
    ('X', "Locale time representation"),
    ('y', "2-digit year (00–99)"),
    ('Y', "4-digit year"),
    ('z', "Timezone offset (+hhmm)"),
    ('Z', "Timezone abbreviation"),
    ('%', "Literal percent sign"),
];

/// Look up a clock-format specifier letter's description.
fn clock_spec_desc(letter: char) -> Option<&'static str> {
    CLOCK_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Find every clock-format specifier in `text` —
/// `%[EO]?[a-zA-Z%]`.  Returns each specifier as its source
/// text (including any `%E` / `%O` locale prefix).
fn scan_clock_specifiers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        // `%`...
        let start = i;
        i += 1;
        if i < chars.len() && (chars[i] == 'E' || chars[i] == 'O') {
            i += 1;
        }
        if i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '%') {
            i += 1;
            out.push(chars[start..i].iter().collect());
        }
    }
    out
}

/// Render a markdown table of clock-format specifiers found
/// in `text`.
fn clock_format_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Clock format string** (strftime-style)\n".to_string()];
    let specs = scan_clock_specifiers(text);
    if specs.is_empty() {
        parts.push("No specifiers found.".to_string());
    } else {
        parts.push("| Specifier | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for spec in specs {
            let last = spec.chars().last().unwrap_or(' ');
            let desc = clock_spec_desc(last).unwrap_or("Unknown");
            let display = if spec.chars().count() == 3 {
                format!("{desc} (locale-modified)")
            } else {
                desc.to_string()
            };
            parts.push(format!("| `{spec}` | {display} |"));
        }
    }
    parts.join("\n")
}

/// `printf`-style format-specifier descriptions for
/// sprintf-hover.
const SPRINTF_SPEC_DESC: &[(char, &str)] = &[
    ('d', "Signed decimal integer"),
    ('i', "Signed decimal integer"),
    ('u', "Unsigned decimal integer"),
    ('o', "Unsigned octal integer"),
    ('x', "Unsigned hexadecimal (lowercase)"),
    ('X', "Unsigned hexadecimal (uppercase)"),
    ('f', "Floating-point (fixed notation)"),
    ('e', "Floating-point (scientific, lowercase)"),
    ('E', "Floating-point (scientific, uppercase)"),
    ('g', "Shorter of %e or %f"),
    ('G', "Shorter of %E or %f"),
    ('s', "String"),
    ('c', "Character (by Unicode code point)"),
    ('%', "Literal percent sign"),
    ('b', "Unsigned binary integer"),
    ('B', "Unsigned binary integer (alternate form)"),
    ('a', "Double hex fraction (lowercase)"),
    ('A', "Double hex fraction (uppercase)"),
];

fn sprintf_spec_desc(letter: char) -> Option<&'static str> {
    SPRINTF_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Scan `text` for sprintf-style format specifiers.  Captures
/// the full specifier as written, e.g. `%05d` or `%-10s`.
/// Scan for `printf`-style specifiers
/// (`%[positional$]?[flags]*[width]?[.prec]?[type]`).
fn scan_sprintf_specifiers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        // Positional argument `<digit>+$`.
        let digits_start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i < chars.len() && chars[i] == '$' && i > digits_start {
            i += 1;
        } else {
            // Roll back — those digits were flags / width.
            i = digits_start;
        }
        // Flags: `-` / `+` / ` ` / `#` / `0`.
        while i < chars.len() && matches!(chars[i], '-' | '+' | ' ' | '#' | '0') {
            i += 1;
        }
        // Width.
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        // Precision.
        if i < chars.len() && chars[i] == '.' {
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
        // Type character.
        if i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '%') {
            i += 1;
            out.push(chars[start..i].iter().collect());
        }
    }
    out
}

/// Render a markdown table of sprintf-style specifiers in
/// `text`.
fn sprintf_format_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Format string** (sprintf-style)\n".to_string()];
    let specs = scan_sprintf_specifiers(text);
    if specs.is_empty() {
        parts.push("No specifiers found.".to_string());
    } else {
        parts.push("| Specifier | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for spec in specs {
            let type_char = spec.chars().last().unwrap_or(' ');
            let desc = sprintf_spec_desc(type_char).unwrap_or("Unknown");
            parts.push(format!("| `{spec}` | {desc} |"));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on a `format` / `scan`
/// format-string argument and return the literal text.
/// `format <fmtString> ?arg arg ...?` — the first arg is the
/// format.  Single-line context only.
fn sprintf_format_string_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    if tokens[0] != "format" && tokens[0] != "scan" {
        return None;
    }
    string_literal_with_percent_at(line_text, character)
}

/// Find a `"..."` or `{...}` literal that contains `character`
/// AND has at least one `%` in it.  Helper shared between
/// `clock_format_string_at_position` and
/// `sprintf_format_string_at_position`.
fn string_literal_with_percent_at(line_text: &str, character: u32) -> Option<String> {
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let opener = chars[i];
        let closer = match opener {
            '"' => '"',
            '{' => '}',
            _ => {
                i += 1;
                continue;
            }
        };
        let start = i + 1;
        let mut end = start;
        while end < chars.len() && chars[end] != closer {
            end += 1;
        }
        if start <= col && col <= end {
            let literal: String = chars[start..end].iter().collect();
            if literal.contains('%') {
                return Some(literal);
            }
            return None;
        }
        i = end + 1;
    }
    None
}

/// `binary format` / `binary scan` specifier table.
const BINARY_SPEC_DESC: &[(char, &str)] = &[
    ('a', "Byte string, padded with nulls"),
    ('A', "Byte string, padded with spaces"),
    ('b', "Binary digits (low-to-high order)"),
    ('B', "Binary digits (high-to-low order)"),
    ('h', "Hexadecimal digits (low-to-high nibble)"),
    ('H', "Hexadecimal digits (high-to-low nibble)"),
    ('c', "8-bit signed integer"),
    ('s', "16-bit signed integer (little-endian)"),
    ('S', "16-bit signed integer (big-endian)"),
    ('i', "32-bit signed integer (little-endian)"),
    ('I', "32-bit signed integer (big-endian)"),
    ('n', "32-bit integer (native byte order)"),
    ('w', "64-bit signed integer (little-endian)"),
    ('W', "64-bit signed integer (big-endian)"),
    ('m', "64-bit integer (native byte order)"),
    ('r', "32-bit float (little-endian)"),
    ('R', "32-bit float (big-endian)"),
    ('f', "32-bit float (native byte order)"),
    ('d', "64-bit double (native byte order)"),
    ('x', "Null padding byte (format) / skip byte (scan)"),
    ('X', "Move cursor back one byte"),
    ('@', "Move cursor to absolute position"),
    ('t', "Reserved (Tcl 8.5+)"),
];

fn binary_spec_desc(letter: char) -> Option<&'static str> {
    BINARY_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Compact type label for the detail table.
fn binary_short_type(letter: char) -> &'static str {
    match letter {
        'a' => "str (null-pad)",
        'A' => "str (space-pad)",
        'b' => "bits lo→hi",
        'B' => "bits hi→lo",
        'h' => "hex lo→hi",
        'H' => "hex hi→lo",
        'c' => "int8",
        's' => "int16 LE",
        'S' => "int16 BE",
        'i' => "int32 LE",
        'I' => "int32 BE",
        'n' => "int32 native",
        'w' => "int64 LE",
        'W' => "int64 BE",
        'm' => "int64 native",
        'r' => "float32 LE",
        'R' => "float32 BE",
        'f' => "float32 native",
        'd' => "float64 native",
        'x' => "pad/skip",
        'X' => "back",
        '@' => "seek",
        't' => "reserved",
        _ => "?",
    }
}

/// Unit byte size per element for fixed-width binary types.
fn binary_unit_bytes(letter: char) -> Option<u32> {
    match letter {
        'c' => Some(1),
        's' | 'S' => Some(2),
        'i' | 'I' | 'n' | 'r' | 'R' | 'f' => Some(4),
        'w' | 'W' | 'm' | 'd' => Some(8),
        _ => None,
    }
}

/// Specifiers that don't consume a variable / value argument.
fn binary_no_var(letter: char) -> bool {
    matches!(letter, 'x' | 'X' | '@')
}

/// Total byte size for one binary format field, or `None` if
/// unknown (`*` count, `X` move-back, …).
fn binary_field_bytes(letter: char, count: u32, star: bool) -> Option<u32> {
    if star {
        return None;
    }
    if let Some(unit) = binary_unit_bytes(letter) {
        // A large-but-parseable count (`d600000000` ⇒ 8 × 600000000) overflows
        // u32; report an unknown size rather than panicking in debug or
        // wrapping to a bogus value in release (issue 182).
        return unit.checked_mul(count);
    }
    match letter {
        'a' | 'A' | 'x' => Some(count),
        'b' | 'B' => Some(count.div_ceil(8)),
        'h' | 'H' => Some(count.div_ceil(2)),
        _ => None,
    }
}

/// One parsed `binary format` / `binary scan` field.
#[derive(Debug, Clone)]
struct BinaryField {
    /// The full spec token as written (e.g. `"i4"`, `"a*"`).
    full: String,
    /// Type character (e.g. `'i'`).
    letter: char,
    /// Numeric count (defaults to `1` when omitted).
    count: u32,
    /// `u` / `s` size-modifier (Tcl 8.5+), or empty string.
    modifier: String,
    /// `true` when the spec used `*` for the count.
    star: bool,
    /// Per-unit byte size before seek/skip adjustment.
    byte_size: Option<u32>,
    /// `true` when this field consumes a value/variable argument.
    consumes_var: bool,
}

/// Scan a `binary format` / `binary scan` spec string into
/// structured fields.  Tcl grammar: `type [modifier] [count|*]`,
/// repeated.
fn scan_binary_fields(text: &str) -> Vec<BinaryField> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let letter = chars[i];
        if binary_spec_desc(letter).is_none() {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        let mut modifier = String::new();
        if i < chars.len() && (chars[i] == 'u' || chars[i] == 's') {
            modifier.push(chars[i]);
            i += 1;
        }
        let mut star = false;
        let mut count_str = String::new();
        if i < chars.len() && chars[i] == '*' {
            star = true;
            i += 1;
        } else {
            while i < chars.len() && chars[i].is_ascii_digit() {
                count_str.push(chars[i]);
                i += 1;
            }
        }
        let count: u32 = count_str.parse().unwrap_or(1);
        let full: String = chars[start..i].iter().collect();
        let byte_size = binary_field_bytes(letter, count, star);
        let consumes_var = !binary_no_var(letter);
        out.push(BinaryField {
            full,
            letter,
            count,
            modifier,
            star,
            byte_size,
            consumes_var,
        });
    }
    out
}

/// Surrounding-command context the binary-hover renderer uses
/// to label fields with variable / value argument names.
#[derive(Debug, Clone)]
struct BinaryContext {
    /// Format-string content (between the surrounding quotes
    /// / braces).
    text: String,
    /// `"format"` or `"scan"`.
    subcmd: String,
    /// Trailing argument tokens (variable names for `scan`,
    /// value expressions for `format`).  Filled best-effort
    /// from the line tokenisation — may be empty.
    args: Vec<String>,
}

/// Render the binary format-spec hover markdown, including
/// the byte-ruler diagram when every field has a known byte
/// size, no field uses `X` (move-back), and the total fits in
/// 32 bytes.
fn binary_format_hover_text(ctx: &BinaryContext) -> String {
    let fields = scan_binary_fields(&ctx.text);
    if fields.is_empty() {
        return "**Binary format string**\n\nNo specifiers found.".to_string();
    }

    // Map each consuming field → arg name (variable for scan,
    // value expr for format).  Fields without a corresponding
    // arg fall back to the spec text as their label.
    let mut field_labels: Vec<String> = Vec::with_capacity(fields.len());
    let mut var_idx = 0;
    for field in &fields {
        if field.consumes_var && var_idx < ctx.args.len() {
            field_labels.push(ctx.args[var_idx].clone());
            var_idx += 1;
        } else {
            field_labels.push(field.full.clone());
        }
    }

    // Resolve effective byte deltas, including absolute seek
    // (`@N` jumps to absolute offset N — count the gap from the
    // current cursor).  A backward seek (target < cursor)
    // disables the diagram entirely.
    let mut effective_bytes: Vec<Option<u32>> = Vec::with_capacity(fields.len());
    let mut cursor: u32 = 0;
    let mut has_backward_seek = false;
    for field in &fields {
        if field.letter == '@' {
            if field.star {
                effective_bytes.push(None);
                continue;
            }
            let target = field.count;
            if target < cursor {
                effective_bytes.push(Some(0));
                has_backward_seek = true;
            } else {
                effective_bytes.push(Some(target - cursor));
            }
            cursor = target;
            continue;
        }
        match field.byte_size {
            Some(bs) => {
                effective_bytes.push(Some(bs));
                cursor += bs;
            }
            None => effective_bytes.push(None),
        }
    }

    let n_vars = fields.iter().filter(|f| f.consumes_var).count();
    let total_known: u32 = effective_bytes
        .iter()
        .filter_map(|bs| bs.filter(|n| *n > 0))
        .sum();
    let has_unknown = effective_bytes.iter().any(Option::is_none);
    let plural = if n_vars == 1 { "" } else { "s" };
    let size_suffix = if has_unknown { "+ " } else { "" };

    let mut parts: Vec<String> = vec![format!(
        "**binary {}** — {n_vars} field{plural}, {total_known}{size_suffix} bytes\n",
        ctx.subcmd
    )];

    // Byte-ruler diagram — skipped when any field has unknown
    // size, when a backward seek scrambled the offsets, or when
    // the total exceeds the 32-byte rendering budget.
    let can_diagram = !has_backward_seek
        && !effective_bytes.is_empty()
        && effective_bytes
            .iter()
            .all(|bs| matches!(bs, Some(n) if *n > 0));
    if can_diagram && (1..=32).contains(&total_known) {
        parts.push("```".to_string());
        parts.extend(render_byte_ruler(
            &fields,
            &effective_bytes,
            &field_labels,
            total_known,
        ));
        parts.push("```\n".to_string());
    }

    // Detail table — Spec / Variable / Type / Bytes.
    parts.push("| Spec | Variable | Type | Bytes |".to_string());
    parts.push("|------|----------|------|------:|".to_string());
    for (j, field) in fields.iter().enumerate() {
        let var = if field.consumes_var {
            field_labels[j].as_str()
        } else {
            "—"
        };
        let mut typ = binary_short_type(field.letter).to_string();
        if field.modifier == "u" {
            typ = typ.replace("int", "uint");
        }
        if field.count > 1 && binary_unit_bytes(field.letter).is_some() {
            typ = format!("{typ} ×{}", field.count);
        }
        let bs_str = if field.star {
            "…".to_string()
        } else {
            effective_bytes[j].map_or_else(|| "?".to_string(), |n| n.to_string())
        };
        parts.push(format!("| `{}` | {var} | {typ} | {bs_str} |", field.full));
    }
    parts.join("\n")
}

/// Render the four-line byte-ruler diagram: a numeric ruler
/// across the byte axis, then top / middle / bottom rows of
/// box-drawing characters labelling each field.  `total_known`
/// is guaranteed to be in `1..=32` (gated by the caller).
fn render_byte_ruler(
    fields: &[BinaryField],
    effective_bytes: &[Option<u32>],
    field_labels: &[String],
    total_known: u32,
) -> Vec<String> {
    use std::fmt::Write;
    const CPB: u32 = 4; // chars per byte
    let indent = "      ";
    let mut ruler = String::from(indent);
    for b in 0..total_known {
        let _ = write!(ruler, "{b:<width$}", width = CPB as usize);
    }
    let mut top = String::from(indent);
    let mut mid = String::from(indent);
    let mut bot = String::from(indent);
    for j in 0..fields.len() {
        let bs = effective_bytes[j].expect("caller gates on all-Some");
        let w = (CPB * bs).saturating_sub(1) as usize;
        let label = field_labels[j].chars().take(w).collect::<String>();
        let sep_t = if j == 0 { '┌' } else { '┬' };
        let sep_b = if j == 0 { '└' } else { '┴' };
        top.push(sep_t);
        top.push_str(&"─".repeat(w));
        mid.push('│');
        mid.push_str(&center(&label, w));
        bot.push(sep_b);
        bot.push_str(&"─".repeat(w));
    }
    top.push('┐');
    mid.push('│');
    bot.push('┘');
    vec![ruler, top, mid, bot]
}

/// Center `s` within a `width`-character cell using spaces (the
/// byte-ruler labels).
fn center(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_string();
    }
    let extra = width - len;
    let left = extra / 2;
    let right = extra - left;
    let mut out = String::with_capacity(width);
    for _ in 0..left {
        out.push(' ');
    }
    out.push_str(s);
    for _ in 0..right {
        out.push(' ');
    }
    out
}

/// Detect when the cursor sits on a `binary format` /
/// `binary scan` format-string argument and capture the
/// surrounding command's argument list.  Returns the format
/// text plus the `format`/`scan` subcommand and the trailing
/// argument tokens (best-effort, single-line).
fn binary_format_context_at_position(
    source: &str,
    line: u32,
    character: u32,
) -> Option<BinaryContext> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }
    if tokens[0] != "binary" || (tokens[1] != "format" && tokens[1] != "scan") {
        return None;
    }
    let subcmd = tokens[1].to_string();
    // The format string is argv[2] for `format`, argv[3] for `scan`.  It is
    // usually a braced/quoted literal, but a bare word (`binary format c2s …`)
    // is just as valid — fall back to the whitespace-token at the cursor when
    // it sits at the format-arg index.
    let fmt_idx = if subcmd == "scan" { 3 } else { 2 };
    let text = string_literal_at(line_text, character).or_else(|| {
        word_token_at(line_text, character)
            .filter(|(idx, _)| *idx == fmt_idx)
            .map(|(_, w)| w)
    })?;
    // `binary format FORMAT VAL ...`   — format is argv[2]
    // `binary scan STRING FORMAT VAR ...` — format is argv[3]
    let skip = if subcmd == "scan" { 4 } else { 3 };
    let args = binary_trailing_args(line_text, skip);
    Some(BinaryContext { text, subcmd, args })
}

/// Return `(token_index, token_text)` for the whitespace-delimited token that
/// contains `character` (UTF-16 column), or `None` when the cursor is on
/// whitespace / past the end.
fn word_token_at(line_text: &str, character: u32) -> Option<(usize, String)> {
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut idx = 0;
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        if start <= col && col <= i {
            return Some((idx, chars[start..i].iter().collect()));
        }
        idx += 1;
    }
    None
}

/// Recover the trailing argument tokens (variable names for
/// `scan`, value expressions for `format`) that follow the
/// format-string argument.  Skips over braced / quoted literal
/// groupings so the format string itself doesn't bleed into
/// the args list — `binary format {a4 i} val` correctly yields
/// `["val"]` rather than `["{a4", "i}", "val"]`.  The first
/// `skip` argv positions (incl. the format string itself) are
/// dropped.
fn binary_trailing_args(line_text: &str, skip: usize) -> Vec<String> {
    let chars: Vec<char> = line_text.chars().collect();
    let mut tokens: Vec<String> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let mut token = String::new();
        match chars[i] {
            '{' => {
                let mut depth = 1;
                i += 1;
                token.push('{');
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        depth -= 1;
                    }
                    token.push(chars[i]);
                    i += 1;
                }
            }
            '"' => {
                let mut escaped = false;
                i += 1;
                token.push('"');
                while i < chars.len() {
                    let c = chars[i];
                    token.push(c);
                    i += 1;
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        break;
                    }
                }
            }
            _ => {
                while i < chars.len() && !chars[i].is_whitespace() {
                    token.push(chars[i]);
                    i += 1;
                }
            }
        }
        tokens.push(token);
    }
    tokens.into_iter().skip(skip).collect()
}

/// `regsub` backref description table.
fn regsub_backref_desc(c: char) -> Option<&'static str> {
    match c {
        '&' | '0' => Some("Entire matched string"),
        '1' => Some("First capture group"),
        '2' => Some("Second capture group"),
        '3' => Some("Third capture group"),
        '4' => Some("Fourth capture group"),
        '5' => Some("Fifth capture group"),
        '6' => Some("Sixth capture group"),
        '7' => Some("Seventh capture group"),
        '8' => Some("Eighth capture group"),
        '9' => Some("Ninth capture group"),
        _ => None,
    }
}

/// Scan a `regsub` substitution spec for `\0` … `\9` / `\&`
/// backreferences.  Returns each match as written (e.g.
/// `\\1`).
fn scan_regsub_backrefs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' {
            i += 1;
            continue;
        }
        if i + 1 >= chars.len() {
            break;
        }
        let next = chars[i + 1];
        if next == '&' || next.is_ascii_digit() {
            out.push(chars[i..=i + 1].iter().collect());
        }
        i += 2;
    }
    out
}

/// Render the regsub substitution-spec hover markdown.
fn regsub_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Substitution spec** (regsub)\n".to_string()];
    let refs = scan_regsub_backrefs(text);
    if refs.is_empty() {
        parts.push("No backreferences found.".to_string());
    } else {
        parts.push("| Reference | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for r in refs {
            let backref_char = r.chars().nth(1).unwrap_or(' ');
            let desc = regsub_backref_desc(backref_char).unwrap_or("Unknown");
            // Escape the backslash for display (`\\` in
            // markdown renders as `\`).
            parts.push(format!("| `{r}` | {desc} |"));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on the substitution-spec
/// argument of a `regsub` invocation and return the literal
/// text.  `regsub ?switches? exp string subSpec ?varName?`
/// — `subSpec` is the 4th positional arg (after switches).
/// Single-line only.
fn regsub_subspec_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() || tokens[0] != "regsub" {
        return None;
    }
    // The substitution spec contains backslash sequences,
    // typically as a quoted or braced literal.  Any literal
    // string containing `\\<digit-or-&>` overlapping the cursor
    // counts as the subspec — a loose detection that does not
    // do precise arg-position resolution.
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let opener = chars[i];
        let closer = match opener {
            '"' => '"',
            '{' => '}',
            _ => {
                i += 1;
                continue;
            }
        };
        let start = i + 1;
        let mut end = start;
        while end < chars.len() && chars[end] != closer {
            end += 1;
        }
        if start <= col && col <= end {
            let literal: String = chars[start..end].iter().collect();
            if scan_regsub_backrefs(&literal).is_empty() {
                return None;
            }
            return Some(literal);
        }
        i = end + 1;
    }
    None
}

/// Helper: find any `"..."` / `{...}` literal containing the
/// cursor.  Shared between hover providers that need
/// literal-context detection but don't care whether the
/// literal contains `%`.
fn string_literal_at(line_text: &str, character: u32) -> Option<String> {
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let opener = chars[i];
        let closer = match opener {
            '"' => '"',
            '{' => '}',
            _ => {
                i += 1;
                continue;
            }
        };
        let start = i + 1;
        let mut end = start;
        while end < chars.len() && chars[end] != closer {
            end += 1;
        }
        if start <= col && col <= end {
            return Some(chars[start..end].iter().collect());
        }
        i = end + 1;
    }
    None
}

/// Glob metacharacter descriptions.
fn glob_meta_desc(c: char) -> Option<&'static str> {
    match c {
        '*' => Some("Matches any sequence of characters"),
        '?' => Some("Matches any single character"),
        '[' => Some("Character class — matches any character inside brackets"),
        _ => None,
    }
}

/// Scan a glob pattern for metacharacters.  Returns a list of
/// `(token, description)` tuples — `*`, `?`, character class
/// `[abc]`, and escape sequences. +
/// `_glob_hover`'s metacharacter walk.
fn scan_glob_metachars(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            let key = "escape".to_string();
            if !seen.contains(&key) {
                out.push((format!("\\{next}"), format!("Escaped character `{next}`")));
                seen.insert(key);
            }
            i += 2;
            continue;
        }
        if chars[i] == '[' {
            let start = i;
            let mut end = i + 1;
            while end < chars.len() && chars[end] != ']' {
                end += 1;
            }
            let token: String = if end < chars.len() {
                chars[start..=end].iter().collect()
            } else {
                chars[start..end].iter().collect()
            };
            if !seen.contains(&token) {
                let inner: String = chars[start + 1..end].iter().collect();
                out.push((
                    token.clone(),
                    format!("Character class: matches any of `{inner}`"),
                ));
                seen.insert(token);
            }
            i = end + 1;
            continue;
        }
        if let Some(desc) = glob_meta_desc(chars[i]) {
            let key = chars[i].to_string();
            if !seen.contains(&key) {
                out.push((key.clone(), desc.to_string()));
                seen.insert(key);
            }
        }
        i += 1;
    }
    out
}

/// Escape a value for a GFM table cell: a literal `|` closes the cell even
/// inside an inline code span, so it must be backslash-escaped (`\|`, which
/// GFM renders back as `|`).  Applied to the regex/glob hover token and
/// meaning cells, whose content is user-pattern-derived and may contain `|`
/// (e.g. an alternation `foo|bar`) that would otherwise break the row in
/// strict GFM renderers (issue 187).
fn gfm_table_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// Render the glob-pattern hover markdown.
fn glob_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Glob pattern**\n".to_string()];
    let metas = scan_glob_metachars(text);
    if metas.is_empty() {
        parts.push("Literal string (no metacharacters).".to_string());
    } else {
        parts.push("| Pattern | Meaning |".to_string());
        parts.push("|---------|---------|".to_string());
        for (tok, desc) in metas {
            parts.push(format!(
                "| `{}` | {} |",
                gfm_table_cell(&tok),
                gfm_table_cell(&desc)
            ));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on a glob pattern.  Recognises
/// `string match <pat> ...`, `glob <pat>...`, and `lsearch
/// -glob <pat> ...` — three common entry points for glob
/// matching in Tcl.  Single-line only.
fn glob_pattern_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let is_glob_command = matches!(tokens[0], "glob")
        || (tokens.len() >= 2 && tokens[0] == "string" && tokens[1] == "match")
        || (tokens.len() >= 2 && tokens[0] == "lsearch" && tokens.contains(&"-glob"));
    if !is_glob_command {
        return None;
    }
    let literal = string_literal_at(line_text, character)?;
    // Require at least one glob metacharacter or `\` escape so
    // we don't fire on literal strings.
    if !literal.chars().any(|c| matches!(c, '*' | '?' | '[' | '\\')) {
        return None;
    }
    Some(literal)
}

/// Regex metacharacter descriptions.
fn regex_meta_desc(token: &str) -> Option<&'static str> {
    match token {
        "^" => Some("Start of line/string anchor"),
        "$" => Some("End of line/string anchor"),
        "." => Some("Match any single character"),
        "*" => Some("Zero or more (greedy)"),
        "+" => Some("One or more (greedy)"),
        "?" => Some("Zero or one (greedy)"),
        "*?" => Some("Zero or more (lazy)"),
        "+?" => Some("One or more (lazy)"),
        "??" => Some("Zero or one (lazy)"),
        "|" => Some("Alternation (OR)"),
        _ => None,
    }
}

/// Regex escape descriptions for common shorthand classes.
fn regex_escape_desc(token: &str) -> Option<&'static str> {
    match token {
        "\\d" => Some("Digit `[0-9]`"),
        "\\D" => Some("Non-digit"),
        "\\s" => Some("Whitespace"),
        "\\S" => Some("Non-whitespace"),
        "\\w" => Some("Word character `[a-zA-Z0-9_]`"),
        "\\W" => Some("Non-word character"),
        "\\b" => Some("Word boundary"),
        "\\B" => Some("Non-word boundary"),
        "\\A" => Some("Start of string"),
        "\\Z" => Some("End of string"),
        "\\n" => Some("Newline"),
        "\\t" => Some("Tab"),
        "\\r" => Some("Carriage return"),
        _ => None,
    }
}

/// One emitted regex-token entry — `(consumed, key, tok, desc)`.
/// `consumed` is the number of source chars the token covers;
/// `key` is the dedup key; `tok` is what the table renders;
/// `desc` is the explanation.  Returned by each sub-scanner so
/// the outer loop in [`scan_regex_components`] stays readable.
type RegexComp = (usize, String, String, String);

/// Scan a regex pattern for metacharacters / classes /
/// escapes.  Handles common cases:
/// anchors, quantifiers, alternation, character classes,
/// shorthand escapes, capture-group parens, lazy
/// quantifiers.
fn scan_regex_components(text: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let mut i = 0;
    while i < chars.len() {
        // The sub-scanners are tried in order: escape and char
        // class are eager (they consume multi-char windows);
        // lazy-quantifier has to run before single-char meta so
        // `*?` doesn't get split.  Group-open also runs before
        // single-char meta so `(?:` / `(` get attributed
        // correctly.
        let token = scan_regex_escape(&chars, i)
            .or_else(|| scan_regex_char_class(&chars, i))
            .or_else(|| scan_regex_lazy_quantifier(&chars, i))
            .or_else(|| scan_regex_group(&chars, i))
            .or_else(|| scan_regex_single_meta(chars[i]));
        match token {
            Some((consumed, key, tok, desc)) => {
                if seen.insert(key) {
                    out.push((tok, desc));
                }
                i += consumed.max(1);
            }
            None => i += 1,
        }
    }
    out
}

/// `\<char>` escape sequences — shorthand classes (`\d`, `\w`),
/// numbered backreferences (`\1`-`\9`), and escaped literals
/// (`\.`, `\*`, …).  Falls back to a generic "Escape sequence"
/// label for unknown payloads.
fn scan_regex_escape(chars: &[char], i: usize) -> Option<RegexComp> {
    if chars.get(i)? != &'\\' {
        return None;
    }
    let next = *chars.get(i + 1)?;
    let tok = format!("\\{next}");
    let desc = if next.is_ascii_digit() {
        format!("Backreference to group {next}")
    } else if let Some(d) = regex_escape_desc(&tok) {
        d.to_string()
    } else if ".*+?(){}[]|^$\\".contains(next) {
        format!("Escaped literal `{next}`")
    } else {
        format!("Escape sequence `{tok}`")
    };
    Some((2, tok.clone(), tok, desc))
}

/// `[...]` character classes, including leading `^` negation
/// and a literal `]` as first char per regex grammar.
/// Consumes the entire class including the closing `]` (or to
/// EOL when the pattern is malformed).
fn scan_regex_char_class(chars: &[char], i: usize) -> Option<RegexComp> {
    if chars.get(i)? != &'[' {
        return None;
    }
    let start = i;
    let mut end = i + 1;
    if chars.get(end) == Some(&'^') {
        end += 1;
    }
    if chars.get(end) == Some(&']') {
        end += 1;
    }
    while end < chars.len() && chars[end] != ']' {
        if chars[end] == '\\' && end + 1 < chars.len() {
            end += 2;
        } else {
            end += 1;
        }
    }
    let (tok_slice, consumed) = if end < chars.len() {
        (&chars[start..=end], end + 1 - start)
    } else {
        (&chars[start..end], end - start)
    };
    let tok: String = tok_slice.iter().collect();
    let inner: String = if tok.starts_with('[') && tok.ends_with(']') {
        tok[1..tok.len() - 1].to_string()
    } else {
        tok[1..].to_string()
    };
    let desc = format!("Character class: matches any of `{inner}`");
    Some((consumed, tok.clone(), tok, desc))
}

/// Lazy quantifiers — `*?`, `+?`, `??`.  Must run before
/// [`scan_regex_single_meta`] so `*` alone doesn't claim the
/// pair.
fn scan_regex_lazy_quantifier(chars: &[char], i: usize) -> Option<RegexComp> {
    let c = *chars.get(i)?;
    if !matches!(c, '*' | '+' | '?') {
        return None;
    }
    if chars.get(i + 1) != Some(&'?') {
        return None;
    }
    let tok = format!("{c}?");
    let desc = regex_meta_desc(&tok)?.to_string();
    Some((2, tok.clone(), tok, desc))
}

/// Grouping — `(?:`, `(?=`, `(?!`, `(?>`, and bare `(` / `)`.
fn scan_regex_group(chars: &[char], i: usize) -> Option<RegexComp> {
    let c = *chars.get(i)?;
    if c == ')' {
        return Some((1, ")".into(), ")".into(), "Group close".into()));
    }
    if c != '(' {
        return None;
    }
    if chars.get(i + 1) == Some(&'?')
        && let Some(trail) = chars.get(i + 2)
    {
        let pair = match trail {
            ':' => Some(("(?:", "Non-capturing group")),
            '=' => Some(("(?=", "Positive lookahead")),
            '!' => Some(("(?!", "Negative lookahead")),
            '>' => Some(("(?>", "Atomic (possessive) group")),
            _ => None,
        };
        if let Some((tok, desc)) = pair {
            return Some((3, tok.to_string(), tok.to_string(), desc.to_string()));
        }
    }
    Some((1, "(".into(), "(".into(), "Capture group open".into()))
}

/// Single-char metacharacters — `^`, `$`, `.`, `*`, `+`, `?`,
/// `|`.  Anything [`regex_meta_desc`] knows about.
fn scan_regex_single_meta(c: char) -> Option<RegexComp> {
    let key = c.to_string();
    let desc = regex_meta_desc(&key)?.to_string();
    Some((1, key.clone(), key, desc))
}

/// Render a hover for a command alias (`interp alias {} ALIAS {} TARGET …`)
/// when `word` names a recorded alias.
fn alias_hover_text(analysis: &AnalysisResult, word: &str) -> Option<String> {
    for alias in analysis.command_aliases.values() {
        let simple = alias.qualified_name.trim_start_matches("::");
        if simple == word || alias.qualified_name == word {
            let mut target = alias.target.clone();
            if !alias.extras.is_empty() {
                target.push(' ');
                target.push_str(&alias.extras.join(" "));
            }
            return Some(format!("**Alias** \u{2192} `{target}`"));
        }
    }
    None
}

/// Render a regex-pattern hover markdown.
fn regex_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Regex pattern**\n".to_string()];
    let comps = scan_regex_components(text);
    if comps.is_empty() {
        parts.push("Literal string (no metacharacters).".to_string());
    } else {
        parts.push("| Component | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for (tok, desc) in comps {
            parts.push(format!(
                "| `{}` | {} |",
                gfm_table_cell(&tok),
                gfm_table_cell(&desc)
            ));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on a regex pattern.
/// Recognises `regexp <pat> ...`, `regsub <pat> ...` (the
/// pattern arg, not the subspec), and `lsearch -regexp <pat>
/// ...`.  Single-line only.
fn regex_pattern_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let is_regex_command = matches!(tokens[0], "regexp" | "regsub")
        || (tokens.len() >= 2 && tokens[0] == "lsearch" && tokens.contains(&"-regexp"));
    if !is_regex_command {
        return None;
    }
    let literal = string_literal_at(line_text, character)?;
    // An explicit `regexp` / `regsub` / `lsearch -regexp` pattern argument
    // hovers even with no metacharacters — `regex_hover_text` renders the
    // "Literal string (no metacharacters)" note.  (The `regsub` replacement
    // spec is handled by the earlier `regsub_subspec_at_position` branch.)
    Some(literal)
}

/// Format hover markdown for an IPv4 / IPv6 literal at the
/// cursor's word.
///
/// Returns `None` when `word` isn't a valid IP literal.  An
/// optional `/prefix` suffix is supported; the prefix is
/// rendered as a CIDR network in the result.
fn ip_address_hover_text(word: &str) -> Option<String> {
    use std::fmt::Write;
    if !word.contains('.') && !word.contains(':') {
        return None;
    }
    // Strip the optional `/prefix` suffix before parsing.
    let (addr, prefix) = match word.split_once('/') {
        Some((a, p)) => (a, p.parse::<u8>().ok()),
        None => (word, None),
    };
    if let Ok(v4) = addr.parse::<std::net::Ipv4Addr>() {
        let class = classify_ipv4(v4);
        let mut out = format!("**IPv4 address** `{addr}`\n\n* Classification: {class}\n");
        if let Some(p) = prefix
            && p <= 32
        {
            let _ = writeln!(out, "* CIDR network: `{addr}/{p}`");
        }
        return Some(out);
    }
    if let Ok(v6) = addr.parse::<std::net::Ipv6Addr>() {
        let class = classify_ipv6(v6);
        let mut out = format!("**IPv6 address** `{addr}`\n\n* Classification: {class}\n");
        if let Some(p) = prefix
            && p <= 128
        {
            let _ = writeln!(out, "* CIDR network: `{addr}/{p}`");
        }
        // IPv4-mapped form (`::ffff:x.x.x.x`).
        if let Some(mapped) = v6.to_ipv4_mapped() {
            let _ = writeln!(out, "* IPv4-mapped form: `{mapped}`");
        }
        return Some(out);
    }
    None
}

/// Classify an IPv4 address by RFC category — loopback,
/// private, multicast, broadcast, link-local, unspecified,
/// or public.
fn classify_ipv4(addr: std::net::Ipv4Addr) -> &'static str {
    if addr.is_unspecified() {
        "Unspecified (`0.0.0.0`)"
    } else if addr.is_loopback() {
        "Loopback (RFC 1122)"
    } else if addr.is_private() {
        "Private (RFC 1918)"
    } else if addr.is_link_local() {
        "Link-local (RFC 3927)"
    } else if addr.is_multicast() {
        "Multicast (RFC 5771)"
    } else if addr.is_broadcast() {
        "Broadcast"
    } else if addr.is_documentation() {
        "Documentation (RFC 5737)"
    } else {
        "Public / global"
    }
}

/// Classify an IPv6 address by RFC category.
fn classify_ipv6(addr: std::net::Ipv6Addr) -> &'static str {
    if addr.is_unspecified() {
        "Unspecified (`::`)"
    } else if addr.is_loopback() {
        "Loopback (`::1`)"
    } else if addr.is_multicast() {
        "Multicast (RFC 4291)"
    } else if addr.to_ipv4_mapped().is_some() {
        "IPv4-mapped (RFC 4291)"
    } else if addr.segments()[0] & 0xffc0 == 0xfe80 {
        "Link-local (RFC 4291)"
    } else if addr.segments()[0] & 0xfe00 == 0xfc00 {
        "Unique local (RFC 4193)"
    } else {
        "Global unicast"
    }
}

/// Detect when the cursor sits on a `clock format` /
/// `clock scan` format-string argument and return the
/// literal text.  Single-line only — multi-line literals
/// are not handled.
fn clock_format_string_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    // Tokenise the line on whitespace — the first two tokens
    // must be `clock format` or `clock scan` for the hover to
    // fire.  This is the same single-line context detection
    // used by `signature_help` / `completion`; multi-line
    // command segments are not handled.
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }
    if tokens[0] != "clock" || (tokens[1] != "format" && tokens[1] != "scan") {
        return None;
    }
    string_literal_with_percent_at(line_text, character)
}

/// Find the word and its `[start, end)` columns at the given
/// position, using Tcl's word delimiters.
///
/// Returns `None` when
/// `line` / `character` is out of bounds or the cursor sits on a
/// delimiter run.
#[must_use]
pub fn find_word_span_at_position(
    source: &str,
    line: u32,
    character: u32,
) -> Option<(String, u32, u32)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, character);
    if col >= chars.len() {
        return None;
    }

    let mut start = col;
    while start > 0 && !WORD_DELIMS.contains(&chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && !WORD_DELIMS.contains(&chars[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    let word: String = chars[start..end].iter().collect();
    let prefix: String = chars[..start].iter().collect();
    let start_u32 = crate::definition::utf16_len(&prefix);
    let end_u32 = start_u32.saturating_add(crate::definition::utf16_len(&word));
    Some((word, start_u32, end_u32))
}

/// Check whether the cursor sits on a `$var` reference and
/// return the variable name (without the leading `$`).
#[must_use]
pub fn find_var_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();

    let cursor = utf16_col_to_char_col(line_text, character).min(chars.len());

    // `${name}` braced form first: scan left looking for the
    // most recent `${` whose matching `}` lies at or to the
    // right of the cursor.  This handles cursors anywhere
    // inside the braces, including on the closing `}`.
    if let Some(name) = braced_var_around(&chars, cursor) {
        return Some(name);
    }

    let mut pos = cursor;
    // `$` is a delimiter: in a `$a$b` concatenation the left-scan must stop at
    // the inner `$` so a cursor on `b` resolves `b`, not `a`. Omitting it walked
    // left across the whole concatenation to the first `$` and always returned
    // the first variable (RUST_ISSUE_108). Mirrors `WORD_DELIMS`.
    let stop_chars: &[char] = &[' ', '\t', '\n', ';', '{', '}', '[', ']', '"', '$'];
    while pos > 0 && !stop_chars.contains(&chars[pos - 1]) {
        pos -= 1;
    }
    if pos > 0 && chars[pos - 1] == '$' {
        pos -= 1;
    }

    if pos < chars.len() && chars[pos] == '$' {
        let start = pos + 1;
        let end = scan_var_name_end(&chars, start);
        if end > start {
            let name: String = chars[start..end].iter().collect();
            return Some(name);
        }
    }
    None
}

/// Find a `${name}` braced variable reference containing `cursor`.
/// Walks left from `cursor` to find a `${`, then matches it with
/// the next `}` to its right.  Returns the inner name when the
/// cursor sits inside the braces.
fn braced_var_around(chars: &[char], cursor: usize) -> Option<String> {
    let mut i = cursor.min(chars.len());
    while i > 0 {
        let c = chars[i - 1];
        if c == '{' {
            if i >= 2 && chars[i - 2] == '$' {
                let inner_start = i;
                let mut end = inner_start;
                while end < chars.len() && chars[end] != '}' {
                    end += 1;
                }
                if end < chars.len() && cursor <= end {
                    let name: String = chars[inner_start..end].iter().collect();
                    // `${arr(idx)}` resolves to the base array variable
                    // `arr` (matching the analyser's `normalise_var_name`,
                    // which strips the index for the braced form too), so
                    // a cursor anywhere inside the braces — including on
                    // the index — finds the same symbol the unbraced
                    // `$arr(idx)` path does.
                    let base = match name.find('(') {
                        Some(i) => &name[..i],
                        None => name.as_str(),
                    };
                    if !base.is_empty() {
                        return Some(base.to_owned());
                    }
                }
            }
            return None;
        }
        if c == '}' || c == '"' || c == '[' || c == ']' || c == ';' || c == '\n' {
            return None;
        }
        i -= 1;
    }
    None
}

fn lookup_proc<'a>(analysis: &'a AnalysisResult, word: &str) -> Option<&'a ProcDef> {
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == word || qname == &format!("::{word}") {
            return Some(proc_def);
        }
    }
    None
}

fn lookup_class<'a>(analysis: &'a AnalysisResult, word: &str) -> Option<&'a ClassDef> {
    for class_def in analysis.all_classes.values() {
        if class_def.name == word
            || class_def.qualified_name == word
            || class_def.qualified_name == format!("::{word}")
        {
            return Some(class_def);
        }
    }
    None
}

fn proc_hover_text(proc_def: &ProcDef) -> String {
    let params: Vec<String> = proc_def
        .params
        .iter()
        .map(|p| {
            if p.has_default {
                let default = p.default_value.as_deref().unwrap_or("");
                format!("{{{} {}}}", p.name, default)
            } else {
                p.name.clone()
            }
        })
        .collect();
    let sig = format!(
        "proc {} {{{}}} {{...}}",
        proc_def.qualified_name,
        params.join(" ")
    );
    let mut parts = vec![format!("```tcl\n{sig}\n```")];
    if !proc_def.doc.is_empty() {
        // Render `@param` / `@return` /
        // `@brief` tagged docstrings as structured Markdown
        // sections rather than the raw harvested text.  Lines
        // that don't carry a tag fall through into the
        // description block, preserving free-form comments
        // for procs that don't use Doxygen-style tags.
        parts.push(format_docstring(&proc_def.doc));
    }
    parts.join("\n\n")
}

/// Parse a raw docstring and render it as Markdown for LSP
/// hover.
///
/// Recognised tags:
///
/// * `@brief <text>` — short summary surfaced before the
///   description block.
/// * `@param <name> <text>` — parameter docs rendered as a
///   bulleted **Parameters** list.
/// * `@return <text>` / `@returns <text>` — return-value
///   description surfaced as a **Returns** line.
///
/// Other lines accumulate into the description block.  Pure-
/// decoration lines (a run of `.`, `-`, `=`, `*`, `~`, `#`)
/// are dropped.
fn format_docstring(text: &str) -> String {
    let mut brief = String::new();
    let mut description_lines: Vec<String> = Vec::new();
    let mut params: Vec<(String, String)> = Vec::new();
    let mut returns_parts: Vec<String> = Vec::new();

    for line in text.lines() {
        let stripped = line.trim();
        let low = stripped.to_ascii_lowercase();
        if let Some(rest) = low
            .strip_prefix("@param ")
            .or_else(|| low.strip_prefix("@param\t"))
        {
            // Use the original `stripped` slice for body extract
            // so we preserve case on the parameter name and
            // description.  Find the offset of `rest` within
            // `low` (always 7 — `@param ` length).
            let body = &stripped[7..].trim();
            let mut iter = body.splitn(2, char::is_whitespace);
            let Some(name) = iter.next() else {
                continue;
            };
            let name = name.trim_end_matches(['-', ' ']);
            let desc = iter
                .next()
                .map(|s| s.trim_start_matches(['-', ' ']).to_string())
                .unwrap_or_default();
            params.push((name.to_string(), desc));
            let _ = rest;
            continue;
        }
        if low.starts_with("@return ")
            || low.starts_with("@return\t")
            || low.starts_with("@returns ")
            || low.starts_with("@returns\t")
        {
            let body = stripped
                .split_once(char::is_whitespace)
                .map_or("", |x| x.1)
                .trim();
            returns_parts.push(body.trim_start_matches(['-', ' ']).to_string());
            continue;
        }
        if let Some(rest) = low
            .strip_prefix("@brief ")
            .or_else(|| low.strip_prefix("@brief\t"))
        {
            brief = stripped[7..].trim().to_string();
            let _ = rest;
            continue;
        }
        // Drop decoration-only lines.
        if !stripped.is_empty()
            && stripped
                .chars()
                .all(|c| matches!(c, '.' | '-' | '=' | '*' | '~' | '#'))
        {
            continue;
        }
        description_lines.push(stripped.to_string());
    }

    let description = description_lines.join("\n");
    let description = description.trim().to_string();
    let returns_text = returns_parts.join(" ");

    let mut parts: Vec<String> = Vec::new();
    if !brief.is_empty() {
        parts.push(brief);
    }
    if !description.is_empty() {
        parts.push(description);
    }
    if !params.is_empty() {
        let mut lines = vec!["**Parameters:**".to_string()];
        for (name, desc) in &params {
            if desc.is_empty() {
                lines.push(format!("- **{name}**"));
            } else {
                lines.push(format!("- **{name}** \u{2014} {desc}"));
            }
        }
        parts.push(lines.join("\n"));
    }
    if !returns_text.is_empty() {
        parts.push(format!("**Returns:** {returns_text}"));
    }
    parts.join("\n\n")
}

fn class_hover_text(analysis: &AnalysisResult, class_def: &ClassDef) -> String {
    let mut sig = format!(
        "{} create {}",
        class_def.metaclass, class_def.qualified_name
    );
    if !class_def.superclasses.is_empty() {
        use std::fmt::Write as _;
        let _ = write!(sig, " (superclass: {})", class_def.superclasses.join(", "));
    }
    if !class_def.mixins.is_empty() {
        use std::fmt::Write as _;
        let _ = write!(sig, " (mixin: {})", class_def.mixins.join(", "));
    }
    let mut parts = vec![format!("```tcl\n{sig}\n```")];
    let mut details: Vec<String> = Vec::new();
    if !class_def.methods.is_empty() {
        let mut names: Vec<&str> = class_def.methods.keys().map(String::as_str).collect();
        names.sort_unstable();
        details.push(format!("**Methods**: {}", names.join(", ")));
    }
    if !class_def.class_methods.is_empty() {
        let mut names: Vec<&str> = class_def.class_methods.keys().map(String::as_str).collect();
        names.sort_unstable();
        details.push(format!("**Class methods**: {}", names.join(", ")));
    }
    if !class_def.variables.is_empty() {
        details.push(format!(
            "**Instance variables**: {}",
            class_def.variables.join(", ")
        ));
    }
    // MRO chain + direct subclasses from the class hierarchy — surfaces
    // the inheritance shape inline. Only shown when non-trivial.
    let hierarchy = analysis.class_hierarchy();
    let qname = &class_def.qualified_name;
    if let Some(mro) = hierarchy.mro_map.get(qname)
        && mro.len() > 1
    {
        details.push(format!("**MRO**: {}", mro.join(" → ")));
    }
    if let Some(subs) = hierarchy.subclasses.get(qname)
        && !subs.is_empty()
    {
        let mut names: Vec<&str> = subs.iter().map(String::as_str).collect();
        names.sort_unstable();
        details.push(format!("**Subclasses**: {}", names.join(", ")));
    }
    if !details.is_empty() {
        parts.push(details.join("  \n"));
    }
    if !class_def.doc.is_empty() {
        parts.push(class_def.doc.clone());
    }
    parts.join("\n\n")
}

fn var_hover_text(var_def: &VarDef, type_info: Option<&str>, taint_info: Option<&str>) -> String {
    use std::fmt::Write as _;
    let ref_count = var_def.references.len();
    let mut text = format!(
        "**Variable** `{}`\n\n{} reference(s)",
        var_def.name, ref_count
    );
    if let Some(t) = type_info {
        let _ = write!(text, "\n\n**Inferred intrep**: {t}");
    }
    if let Some(t) = taint_info {
        let _ = write!(text, "\n\n**Taint**: {t}");
    }
    text
}

/// Render hover markdown for an interpreter-provided special variable
/// ([`tcl_registry::SpecialVarSpec`]), showing its shape, summary, provenance,
/// and — for arrays — the keys available in the active `dialect`.
fn special_var_hover_text(
    spec: &tcl_registry::SpecialVarSpec,
    dialect: tcl_registry::dialects::DialectSet,
) -> String {
    use std::fmt::Write as _;
    use tcl_registry::{SpecialVarKind, VarAccess, VarOrigin};

    let (shape, sigil) = match spec.kind {
        SpecialVarKind::Scalar => ("special variable", format!("${}", spec.name)),
        SpecialVarKind::Array => ("special array", format!("${}(…)", spec.name)),
        SpecialVarKind::Namespace => ("special namespace", format!("{}::…", spec.name)),
    };
    let access = match spec.access {
        VarAccess::ReadOnly => "read-only",
        VarAccess::ReadWrite => "read/write",
    };
    let origin = match spec.origin {
        VarOrigin::Interpreter => "the Tcl interpreter",
        VarOrigin::AutoLoader => "the auto-loader (`init.tcl`)",
        VarOrigin::Platform => "the platform / build",
        VarOrigin::Environment => "the process environment",
        VarOrigin::Dialect => "the dialect runtime",
    };

    let mut text = format!(
        "**`{sigil}`** — {shape} ({access})\n\nProvided by {origin}.\n\n{}",
        spec.summary
    );

    // Array keys available in this dialect (skip open-keyed arrays like `env`).
    let keys: Vec<&tcl_registry::SpecialVarKey> = spec.keys_in(dialect).collect();
    if !keys.is_empty() {
        text.push_str("\n\n**Keys**:\n");
        for k in keys {
            let _ = write!(text, "\n- `{}` — {}", k.key, k.summary);
        }
    }

    // CMP-safety note only matters under iRules.
    if spec.cmp_unsafe && dialect.intersects(tcl_registry::dialects::DialectSet::IRULES) {
        let _ = write!(
            text,
            "\n\n⚠️ Accessing `{}` as a plain global demotes the virtual server \
             from CMP. Use the CMP-safe `static::{}` alias instead.",
            spec.name, spec.name
        );
    }

    text
}

/// Lower-case label for a Tcl intrep type (e.g. `ByteArray` →
/// `bytearray`).
fn tcl_type_label(t: TclType) -> String {
    format!("{t:?}").to_lowercase()
}

/// Build the compiler [`CompilationUnit`] and extract the
/// inferred-intrep and taint annotations for `var_name`.  Returns
/// `(type_label, taint_label)`; either may be `None`.
fn infer_var_type_and_taint(
    source: &str,
    registry: &CommandRegistry,
    var_name: &str,
) -> (Option<String>, Option<String>) {
    let unit = CompilationUnit::build_for(source, registry, false);
    (
        infer_var_type(&unit, var_name),
        infer_var_taint(&unit, var_name),
    )
}

/// The top-level function unit followed by every procedure unit
/// in deterministic (name-sorted) order, so the per-variable
/// inference picks a stable first match regardless of the
/// procedure map's hashing order.
fn function_units_in_order(unit: &CompilationUnit) -> Vec<&FunctionUnit> {
    let mut out: Vec<&FunctionUnit> = Vec::with_capacity(unit.procedures.len() + 1);
    out.push(&unit.top_level);
    let mut procs: Vec<&FunctionUnit> = unit.procedures.values().collect();
    procs.sort_by(|a, b| a.name.cmp(&b.name));
    out.extend(procs);
    out
}

/// Infer a dominant intrep type for `var_name`, mirroring
/// `_infer_var_type`: returns the first function (top-level, then
/// procs in name order) that has type entries for the variable
/// and resolves them to a single label.
fn infer_var_type(unit: &CompilationUnit, var_name: &str) -> Option<String> {
    for func in function_units_in_order(unit) {
        let entries: Vec<&TypeLattice> = func
            .types
            .iter()
            .filter(|((name, _ver), _)| func.ssa.var_name(*name) == var_name)
            .map(|(_, t)| t)
            .collect();
        if entries.is_empty() {
            continue;
        }
        // Every version agrees on the same KNOWN type.
        let known: Vec<&TypeLattice> = entries
            .iter()
            .copied()
            .filter(|t| t.kind == TypeKind::Known && t.tcl_type.is_some())
            .collect();
        if !known.is_empty()
            && known.iter().all(|t| t.tcl_type == known[0].tcl_type)
            && known.len() == entries.len()
        {
            return Some(tcl_type_label(known[0].tcl_type.unwrap()));
        }
        // Every version agrees on the same SHIMMERED pair.
        let shimmered: Vec<&TypeLattice> = entries
            .iter()
            .copied()
            .filter(|t| {
                t.kind == TypeKind::Shimmered && t.from_type.is_some() && t.tcl_type.is_some()
            })
            .collect();
        if !shimmered.is_empty() && shimmered.len() == entries.len() {
            let s = shimmered[0];
            if shimmered.iter().all(|t| *t == s) {
                return Some(format!(
                    "shimmered ({} / {})",
                    tcl_type_label(s.from_type.unwrap()),
                    tcl_type_label(s.tcl_type.unwrap()),
                ));
            }
        }
        // Mixed, but a dominant KNOWN type exists.  Pick the
        // smallest by `Ord` so the choice is deterministic.
        if let Some(t) = known
            .iter()
            .filter_map(|t| t.tcl_type)
            .min()
            .map(tcl_type_label)
        {
            return Some(t);
        }
    }
    None
}

/// Human-readable mitigation-colour labels present in `taint`,
/// in display-priority order.
fn taint_colour_labels(taint: TaintLattice) -> Vec<&'static str> {
    let flag_labels: [(TaintColour, &str); 13] = [
        (TaintColour::PATH_PREFIXED, "path-prefixed"),
        (TaintColour::NON_DASH_PREFIXED, "non-dash-prefixed"),
        (TaintColour::CRLF_FREE, "CRLF-free"),
        (TaintColour::SHELL_ATOM, "shell-atom"),
        (TaintColour::LIST_CANONICAL, "list-canonical"),
        (TaintColour::REGEX_LITERAL, "regex-literal"),
        (TaintColour::PATH_NORMALISED, "path-normalised"),
        (TaintColour::HEADER_TOKEN_SAFE, "header-token-safe"),
        (TaintColour::HTML_ESCAPED, "HTML-escaped"),
        (TaintColour::URL_ENCODED, "URL-encoded"),
        (TaintColour::IP_ADDRESS, "IP-address"),
        (TaintColour::PORT, "port"),
        (TaintColour::FQDN, "FQDN"),
    ];
    flag_labels
        .into_iter()
        .filter(|(flag, _)| taint.colours.contains(*flag))
        .map(|(_, label)| label)
        .collect()
}

/// Infer taint for `var_name`, mirroring `_infer_var_taint`:
/// returns the first function with taint entries for the
/// variable, joining the tainted versions to the most
/// conservative colour set.
fn infer_var_taint(unit: &CompilationUnit, var_name: &str) -> Option<String> {
    for func in function_units_in_order(unit) {
        let entries: Vec<&TaintLattice> = func
            .taints
            .iter()
            .filter(|((name, _ver), _)| func.ssa.var_name(*name) == var_name)
            .map(|(_, t)| t)
            .collect();
        if entries.is_empty() {
            continue;
        }
        let tainted: Vec<&TaintLattice> =
            entries.iter().copied().filter(|t| t.is_tainted()).collect();
        if let Some((first, rest)) = tainted.split_first() {
            let combined = rest.iter().fold(**first, |acc, t| acc.join(**t));
            let labels = taint_colour_labels(combined);
            if labels.is_empty() {
                return Some("tainted (from I/O)".to_owned());
            }
            return Some(format!("tainted (from I/O); {}", labels.join(", ")));
        }
    }
    None
}

/// Hover text for a class member at the cursor's byte
/// offset.  Walks every class whose body span contains the
/// cursor and looks `word` up against `methods`,
/// `class_methods`, `properties`, plus the `constructor` /
/// `destructor` keywords.  Returns a one-line markdown
/// summary on hit, `None` otherwise.
fn class_member_hover_text(
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<String> {
    for class_def in analysis.all_classes.values() {
        let body = class_def.body_span;
        if !(body.start() < cursor_offset && cursor_offset < body.end()) {
            continue;
        }
        let qname = &class_def.qualified_name;
        if let Some(m) = class_def.methods.get(word) {
            let note = oo_method_resolution_note(analysis, qname, word)
                .map_or(String::new(), |n| format!("  \n{n}"));
            return Some(format!(
                "**method** `{qname}::{name}` ({nparam} param(s)){note}",
                name = m.name,
                nparam = m.params.len(),
            ));
        }
        if let Some(m) = class_def.class_methods.get(word) {
            let note = oo_method_resolution_note(analysis, qname, word)
                .map_or(String::new(), |n| format!("  \n{n}"));
            return Some(format!(
                "**classmethod** `{qname}::{name}` ({nparam} param(s)){note}",
                name = m.name,
                nparam = m.params.len(),
            ));
        }
        if let Some(p) = class_def.properties.get(word) {
            return Some(format!("**property** `{qname}::{name}`", name = p.name));
        }
        if word == "constructor" && !class_def.constructors.is_empty() {
            let nparam = class_def.constructors.first().map_or(0, |c| c.params.len());
            return Some(format!("**constructor** of `{qname}` ({nparam} param(s))"));
        }
        if word == "destructor" && class_def.destructor.is_some() {
            return Some(format!("**destructor** of `{qname}`"));
        }
    }
    None
}

/// Hover text for a `$obj method` call — `method` resolved
/// against the class identified by `class_q`.  Searches
/// `methods` then `class_methods`, rendering a one-line
/// summary that names the receiver class plus an MRO note
/// (inherited-from / overrides).
///
/// `class_q` may name either a *user*-defined class (`analysis.all_classes`
/// — `oo::class`/`oo::define`/snit/itcl bodies the analyser parsed) or a
/// *registry*-modelled one (a `tcl-registry` `ObjectClassSpec` — tcllib
/// factories, or a Tk/ttk widget's self-referential class, issue #927).
/// User classes are tried first (richer: params, MRO note); the registry is
/// the fallback so e.g. `.t instate` still hovers even though `ttk::treeview`
/// is never a user-defined class.
fn obj_method_hover_text(
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    registry: Option<&CommandRegistry>,
) -> Option<String> {
    if let Some(class_def) = analysis.all_classes.get(class_q) {
        let note = oo_method_resolution_note(analysis, class_q, method);
        let suffix = note.map_or(String::new(), |n| format!("  \n{n}"));
        if let Some(m) = class_def.methods.get(method) {
            return Some(format!(
                "**method** `{class_q}::{name}` ({nparam} param(s)){suffix}",
                name = m.name,
                nparam = m.params.len(),
            ));
        }
        if let Some(m) = class_def.class_methods.get(method) {
            return Some(format!(
                "**classmethod** `{class_q}::{name}` ({nparam} param(s)){suffix}",
                name = m.name,
                nparam = m.params.len(),
            ));
        }
        return None;
    }
    let sub = registry?.instance_method(class_q, method)?;
    Some(format!(
        "**method** `{class_q} {method}`  \n{detail}\n\n`{synopsis}`",
        detail = sub.detail,
        synopsis = sub.synopsis,
    ))
}

/// MRO note for `method` on `class_q`: `inherited from ::Provider` when the
/// method resolves to an ancestor, or `overrides ::Super::method` when it
/// is defined on `class_q` but a superclass also provides it.  `None` for a
/// method defined only here (no note needed) or an unresolvable one.
fn oo_method_resolution_note(
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
) -> Option<String> {
    let hierarchy = analysis.class_hierarchy();
    let provider = hierarchy.method_target(class_q, method)?;
    if provider == class_q {
        // Defined here — does a superclass further down the MRO also
        // provide it (i.e. this is an override)?
        hierarchy
            .next_provider(class_q, method, class_q, None)
            .map(|sup| format!("_overrides `{sup}::{method}`_"))
    } else {
        Some(format!("_inherited from `{provider}`_"))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use tcl_compiler::analyser::Analyser;

    /// Dialect-agnostic default for the subcommand-hover helper tests.
    const ALL: tcl_registry::dialects::DialectSet = tcl_registry::dialects::DialectSet::ALL_TCL;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn find_word_span_returns_none_at_eol() {
        // Position one past the line's last char yields None.
        let src = "proc foo {} {}\n";
        let line = src.split('\n').next().unwrap();
        let len = u32::try_from(line.chars().count()).expect("len fits u32");
        assert!(find_word_span_at_position(src, 0, len).is_none());
    }

    #[test]
    fn find_word_span_extracts_word_under_cursor() {
        // Cursor on the 'r' of `proc`.
        let src = "proc greet {} {}\n";
        let (word, start, end) = find_word_span_at_position(src, 0, 1).unwrap();
        assert_eq!(word, "proc");
        assert_eq!(start, 0);
        assert_eq!(end, 4);
    }

    #[test]
    fn find_word_span_stops_at_dollar_sign() {
        // `$var` — `$` is in `_WORD_DELIMS`, so a cursor inside
        // `var` should yield just `var`.
        let src = "set x $var\n";
        let (word, start, end) = find_word_span_at_position(src, 0, 8).unwrap();
        assert_eq!(word, "var");
        assert_eq!(start, 7);
        assert_eq!(end, 10);
    }

    #[test]
    fn find_var_at_position_recognises_dollar_ref() {
        // Cursor inside `$var`.
        let src = "set x $var\n";
        assert_eq!(find_var_at_position(src, 0, 8), Some("var".to_owned()));
    }

    #[test]
    fn find_var_at_position_returns_none_for_bare_word() {
        let src = "set x 1\n";
        assert!(find_var_at_position(src, 0, 4).is_none());
    }

    #[test]
    fn find_var_at_position_resolves_second_var_of_concatenation() {
        // RUST_ISSUE_108: `set z $x$y` — a cursor on `y` must resolve `y`, not
        // walk left across the `$` to the first variable `x`.
        let src = "set z $x$y\n";
        // Columns: s(0)e(1)t(2) (3)z(4) (5)$(6)x(7)$(8)y(9)
        assert_eq!(find_var_at_position(src, 0, 9), Some("y".to_owned())); // on `y`
        assert_eq!(find_var_at_position(src, 0, 7), Some("x".to_owned())); // on `x`
        // Three-way concatenation resolves the middle var too.
        let src3 = "set w $a$b$c\n";
        assert_eq!(find_var_at_position(src3, 0, 9), Some("b".to_owned()));
    }

    #[test]
    fn find_var_at_position_stops_at_lone_colon() {
        // `$host:$port` — a lone `:` is not part of the variable name (Tcl
        // substitutes only `$host`); it must not be pulled in as `host:`
        // (issue 183).
        let src = "puts $host:$port\n";
        // Columns: p0 u1 t2 s3 (4) $5 h6 o7 s8 t9 :10 $11 ...
        assert_eq!(find_var_at_position(src, 0, 7), Some("host".to_owned()));
        // The `::` qualifier IS part of the name, and a full colon run stays.
        let src2 = "puts $a::b:$c\n";
        assert_eq!(find_var_at_position(src2, 0, 7), Some("a::b".to_owned()));
        let src3 = "puts $a:::b\n";
        assert_eq!(find_var_at_position(src3, 0, 7), Some("a:::b".to_owned()));
    }

    #[test]
    fn binary_field_bytes_saturates_instead_of_overflowing() {
        // `d600000000` ⇒ 8 × 600000000 overflows u32; report unknown size,
        // never panic (debug) or wrap (release) (issue 182).
        assert_eq!(super::binary_field_bytes('d', 600_000_000, false), None);
        assert_eq!(super::binary_field_bytes('i', u32::MAX, false), None);
        // Ordinary sizes still compute.
        assert_eq!(super::binary_field_bytes('i', 4, false), Some(16));
    }

    #[test]
    fn regex_and_glob_hover_escape_table_pipes() {
        // A `|` inside a code-span table cell must be `\|` or it splits the GFM
        // row (issue 187).
        let rx = regex_hover_text("foo|bar");
        assert!(
            rx.lines().all(|l| !l.starts_with("| `|`")),
            "raw pipe token would break the row: {rx}"
        );
        assert!(
            rx.contains("\\|"),
            "alternation `|` should be escaped: {rx}"
        );
        // Glob alternation `{a,b}` has no pipe, but a bracketed pipe would; a
        // char-class-style pattern with `|` escapes too.
        let rxcc = regex_hover_text("[a|b]");
        assert!(rxcc.contains("\\|"));
        // Every non-separator row keeps a balanced cell count (3 pipes → 2
        // cells) after escaping.
        for line in rx.lines().filter(|l| l.starts_with("| ")) {
            let unescaped = line.replace("\\|", "");
            assert_eq!(
                unescaped.matches('|').count(),
                3,
                "row must have exactly two cells: {line}"
            );
        }
    }

    #[test]
    fn find_var_at_position_recognises_braced_form() {
        // Cursor on the `r` inside `${var}`.  The braced form
        // should still resolve to `"var"` so rename and hover
        // can find the symbol.
        let src = "set x ${var}\n";
        assert_eq!(find_var_at_position(src, 0, 9), Some("var".to_owned()));
        // Cursor immediately after the opening `${` (start of name).
        assert_eq!(find_var_at_position(src, 0, 8), Some("var".to_owned()));
        // Cursor on the closing `}` itself — the inner name is
        // still resolvable as long as the cursor sits inside the
        // braces inclusive of the close brace.
        assert_eq!(find_var_at_position(src, 0, 11), Some("var".to_owned()));
    }

    #[test]
    fn hover_on_proc_name_returns_signature() {
        let src = "proc greet {name} { puts $name }\n";
        let analysis = analyse(src);
        let h = hover(src, 0, 6, &analysis, None).expect("expected hover for proc name");
        assert_eq!(h.kind, HoverKind::Markdown);
        assert!(h.value.contains("proc ::greet"), "{}", h.value);
        assert!(h.value.contains("name"), "{}", h.value);
    }

    #[test]
    fn hover_on_proc_qualified_name() {
        let src = "namespace eval ::ns { proc helper {} { return } }\n";
        let analysis = analyse(src);
        // Cursor on `helper` token at column ~28
        let h = hover(src, 0, 28, &analysis, None);
        // Either matches via simple name or qualified name; the
        // contract is that hover surfaces the proc when present.
        if let Some(h) = h {
            assert!(h.value.contains("helper"), "{}", h.value);
        }
    }

    #[test]
    fn hover_on_unknown_word_returns_none() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        // Cursor on "hello" — not a proc / class / var, so None.
        // (`puts` is a builtin and isn't in `all_procs` either.)
        assert!(hover(src, 0, 6, &analysis, None).is_none());
    }

    #[test]
    fn hover_on_class_name_returns_metaclass_signature() {
        let src = "oo::class create Greeter {}\n";
        let analysis = analyse(src);
        let h = hover(src, 0, 18, &analysis, None);
        if let Some(h) = h {
            assert!(h.value.contains("Greeter"), "{}", h.value);
            assert!(
                h.value.contains("oo::class create"),
                "expected metaclass declaration, got {}",
                h.value,
            );
        }
    }

    #[test]
    fn hover_on_dollar_var_returns_var_text() {
        // Variable defined at top level, referenced via `$x`.
        let src = "set x 1\nset y $x\n";
        let analysis = analyse(src);
        let h = hover(src, 1, 7, &analysis, None);
        if let Some(h) = h {
            assert!(h.value.contains("Variable"), "{}", h.value);
            assert!(h.value.contains("`x`"), "{}", h.value);
        }
    }

    #[test]
    fn hover_on_var_surfaces_inferred_intrep() {
        // `x` is assigned an integer literal; the compiler's
        // type-propagation pass should infer `int`, and the hover
        // (given a registry) should surface it.
        let src = "set x 1\nset y $x\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 1, 7, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("**Variable** `x`"), "{}", h.value);
        assert!(h.value.contains("**Inferred intrep**: int"), "{}", h.value);
    }

    #[test]
    fn hover_returns_none_for_out_of_range_line() {
        let src = "proc foo {} {}\n";
        let analysis = analyse(src);
        assert!(hover(src, 99, 0, &analysis, None).is_none());
    }

    #[test]
    fn builtin_hover_shows_requires_when_package_not_imported() {
        // `http::geturl` needs `package require http`; without the
        // import the hover appends the **Requires** line.
        let src = "http::geturl $url\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 6, &analysis, Some(&registry)).expect("hover");
        assert!(
            h.value.contains("**Requires**: `package require http`"),
            "{}",
            h.value
        );
    }

    #[test]
    fn builtin_hover_omits_requires_when_package_imported() {
        // With `package require http` present, the hint is suppressed.
        let src = "package require http\nhttp::geturl $url\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 1, 6, &analysis, Some(&registry)).expect("hover");
        assert!(!h.value.contains("**Requires**"), "{}", h.value);
    }

    #[test]
    fn builtin_hover_resolves_imported_command() {
        // Peer of #776: a bare command imported into the global scope resolves
        // to its qualified spec — hovering `test` after `namespace import
        // ::tcltest::*` surfaces the `tcltest::test` documentation.
        let src = "namespace import ::tcltest::*\ntest t-1 {desc} -body { set x 1 } -result 1\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 1, 2, &analysis, Some(&registry)).expect("hover on bare `test`");
        assert!(
            h.value.contains("tcltest::test"),
            "bare imported `test` must hover as tcltest::test: {}",
            h.value
        );
    }

    #[test]
    fn builtin_hover_bare_test_without_import_is_none() {
        // Control: without an import, a bare `test` is not a known command.
        let src = "test t-1 {desc}\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        assert!(hover(src, 0, 0, &analysis, Some(&registry)).is_none());
    }

    #[test]
    fn builtin_hover_ignores_nested_namespace_import() {
        // An import made inside `namespace eval ns { … }` is not in scope at top
        // level, so hovering a bare top-level `test` must NOT resolve to
        // tcltest::test (mirrors the analyser's scope rule).
        let src = "namespace eval ns { namespace import ::tcltest::* }\ntest t-1 {desc}\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        // `test` is on line 1 (top level) — out of the `ns` import's scope.
        assert!(
            hover(src, 1, 2, &analysis, Some(&registry)).is_none(),
            "a nested-namespace import must not resolve a top-level bare `test`",
        );
    }

    #[test]
    fn builtin_hover_ignores_import_after_cursor() {
        // Source order: hovering a `test` that appears *before* the
        // `namespace import` must not resolve — the import is not yet in effect.
        let src = "test t-1 {desc}\nnamespace import ::tcltest::*\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        assert!(
            hover(src, 0, 2, &analysis, Some(&registry)).is_none(),
            "an import after the cursor must not retroactively resolve `test`",
        );
    }

    #[test]
    fn valid_events_for_asm_profile_requirement() {
        use tcl_registry::events::EventRequires;
        // A command requiring the ASM profile is valid only in ASM events.
        let requires = EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        };
        let events = valid_events(&requires);
        assert_eq!(
            events,
            vec![
                "ASM_REQUEST_BLOCKING",
                "ASM_REQUEST_DONE",
                "ASM_REQUEST_VIOLATION",
                "ASM_RESPONSE_LOGIN",
                "ASM_RESPONSE_VIOLATION",
            ],
            "ASM-profile events"
        );
    }

    #[test]
    fn valid_events_uses_transitive_profile_expansion() {
        use tcl_registry::events::EventRequires;
        // `HTTP`-profile requirement matches HTTP events directly *and*
        // events whose implied profile transitively requires HTTP (e.g.
        // `ADAPT_REQUEST_HEADERS` via `REQUESTADAPT` → `HTTP`).  This
        // exercises `expand_profile_stack`.
        let requires = EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        };
        let events = valid_events(&requires);
        assert!(events.iter().any(|e| e == "HTTP_REQUEST"), "{events:?}");
        assert!(
            events.iter().any(|e| e == "ADAPT_REQUEST_HEADERS"),
            "transitive-profile event missing (expansion broken): {events:?}"
        );
    }

    #[test]
    fn expand_profile_stack_includes_parents() {
        let reg = tcl_registry::profiles::ProfileRegistry::build();
        let stack = reg.expand_profile_stack(&["HTTP"]);
        assert!(stack.contains("HTTP"), "{stack:?}");
        assert!(stack.contains("TCP"), "HTTP should require TCP: {stack:?}");
    }

    #[test]
    fn irules_command_hover_lists_valid_events() {
        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(tcl_registry::dialects::DialectSet::IRULES);
        if let Some(text) =
            builtin_command_hover_text(&registry, "ASM::is_authenticated", &analyse(""), u32::MAX)
        {
            assert!(text.contains("**Valid events**"), "{text}");
            assert!(text.contains("ASM_REQUEST_BLOCKING"), "{text}");
            assert!(text.contains("**Requires**: profile ASM"), "{text}");
        }
    }

    #[test]
    fn irules_namespace_backed_profiles_injected_into_hover() {
        // `DIAMETER::retransmission_default` carries `event_requires` with
        // *empty* profiles; `effective_event_requires` substitutes the
        // `DIAMETER` namespace's profiles, so the hover shows a profile
        // **Requires** line (none would appear without the injection).
        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(tcl_registry::dialects::DialectSet::IRULES);
        let text = builtin_command_hover_text(
            &registry,
            "DIAMETER::retransmission_default",
            &analyse(""),
            u32::MAX,
        )
        .expect("hover");
        assert!(
            text.contains("**Requires**: profile DIAMETER or DIAMETERSESSION or DIAMETER_ENDPOINT"),
            "{text}"
        );
    }

    #[test]
    fn effective_event_requires_injects_namespace_profiles() {
        // Unit check: empty own-profiles + a known `NS::` prefix → the
        // namespace's profiles; a non-namespaced name is unchanged.
        let base = tcl_registry::events::EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        };
        let eff = super::effective_event_requires("DIAMETER::foo", &base);
        assert_eq!(
            eff.profiles,
            &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"]
        );
        // No `::` qualifier → unchanged (still empty).
        assert!(
            super::effective_event_requires("puts", &base)
                .profiles
                .is_empty()
        );
        // Already-populated profiles are left as-is.
        let with = tcl_registry::events::EventRequires {
            profiles: &["HTTP"],
            ..base
        };
        assert_eq!(
            super::effective_event_requires("HTTP::uri", &with).profiles,
            &["HTTP"]
        );
    }

    #[test]
    fn builtin_hover_omits_requires_for_core_command() {
        // A core built-in (`lindex`) needs no package — no hint.
        let src = "lindex $l 0\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 2, &analysis, Some(&registry)).expect("hover");
        assert!(!h.value.contains("**Requires**"), "{}", h.value);
    }

    #[test]
    fn proc_hover_text_formats_default_param() {
        let src = "proc greet {{name world}} { puts $name }\n";
        let analysis = analyse(src);
        let proc_def = analysis.all_procs.values().next().unwrap();
        let text = proc_hover_text(proc_def);
        assert!(text.contains("{name world}"), "got: {text}");
    }

    #[test]
    fn class_hover_text_lists_methods_alphabetically() {
        let src = concat!(
            "oo::class create Foo {\n",
            "    method beta {} {}\n",
            "    method alpha {} {}\n",
            "}\n",
        );
        let analysis = analyse(src);
        let class_def = analysis
            .all_classes
            .values()
            .next()
            .expect("class recorded")
            .clone();
        let text = class_hover_text(&analysis, &class_def);
        // Methods listed in sorted order.
        let alpha_pos = text.find("alpha");
        let beta_pos = text.find("beta");
        if let (Some(a), Some(b)) = (alpha_pos, beta_pos) {
            assert!(a < b, "expected alpha before beta in: {text}");
        }
    }

    #[test]
    fn class_hover_shows_mro_and_subclasses() {
        let src = "oo::class create A {}\noo::class create B {\n    superclass A\n}\noo::class create C {\n    superclass B\n}\n";
        let analysis = analyse(src);
        let b = analysis.all_classes.get("::B").expect("B").clone();
        let text = class_hover_text(&analysis, &b);
        assert!(text.contains("**MRO**"), "MRO missing: {text}");
        assert!(text.contains("::B → ::A"), "MRO chain wrong: {text}");
        assert!(
            text.contains("**Subclasses**") && text.contains("::C"),
            "subclasses missing: {text}"
        );
    }

    #[test]
    fn method_hover_notes_inheritance_and_override() {
        let src = "oo::class create A {\n    method greet {} {}\n}\noo::class create B {\n    superclass A\n    method greet {} {}\n}\noo::class create C {\n    superclass A\n}\n";
        let analysis = analyse(src);
        // B::greet overrides A::greet.
        let over = oo_method_resolution_note(&analysis, "::B", "greet").unwrap_or_default();
        assert!(
            over.contains("overrides") && over.contains("::A::greet"),
            "{over}"
        );
        // C inherits greet from A.
        let inh = oo_method_resolution_note(&analysis, "::C", "greet").unwrap_or_default();
        assert!(
            inh.contains("inherited from") && inh.contains("::A"),
            "{inh}"
        );
        // A::greet defined only here — no note.
        assert!(oo_method_resolution_note(&analysis, "::A", "greet").is_none());
    }

    #[test]
    fn var_hover_text_renders_reference_count() {
        let src = "set x 1\nset y $x\nset z $x\n";
        let analysis = analyse(src);
        let var_def = analysis
            .global_scope
            .variables
            .get("x")
            .expect("x recorded");
        let text = var_hover_text(var_def, None, None);
        assert!(text.contains("**Variable** `x`"), "{}", text);
        assert!(text.contains("reference"), "{}", text);
    }

    #[test]
    fn var_hover_text_appends_intrep_and_taint() {
        let var_def = VarDef {
            name: "x".to_owned(),
            definition_span: tcl_lexer::Span::new(0, 1),
            references: Vec::new(),
            warn_if_unused: false,
            array_indices: std::collections::BTreeSet::new(),
        };
        let text = var_hover_text(&var_def, Some("int"), Some("tainted (from I/O)"));
        assert!(text.contains("**Inferred intrep**: int"), "{text}");
        assert!(text.contains("**Taint**: tainted (from I/O)"), "{text}");
    }

    #[test]
    fn special_var_hover_documents_auto_path() {
        // `$auto_path` has no user definition, but the special-variable
        // registry provides documentation on hover (issue #831).
        let src = "puts $auto_path\n";
        let analysis = analyse(src);
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 8, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("special variable"), "{}", h.value);
        assert!(h.value.contains("auto-loader"), "{}", h.value);
    }

    #[test]
    fn special_var_hover_is_dialect_aware() {
        use tcl_registry::dialects::DialectSet;
        let analysis = analyse("puts $auto_path\n");
        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(DialectSet::IRULES);
        // iRules provides no `auto_path`, so no special-var hover fires there.
        assert!(
            hover_with_dialect(
                "puts $auto_path\n",
                0,
                8,
                &analysis,
                Some(&registry),
                DialectSet::IRULES,
            )
            .is_none()
        );

        // `tcl_platform` exists under iRules and is CMP-unsafe as a plain
        // global — the hover surfaces the `static::` guidance.
        let src = "set x $tcl_platform(os)\n";
        let analysis = analyse(src);
        let h = hover_with_dialect(src, 0, 10, &analysis, Some(&registry), DialectSet::IRULES)
            .expect("hover");
        assert!(h.value.contains("special array"), "{}", h.value);
        assert!(h.value.contains("CMP"), "{}", h.value);
        assert!(h.value.contains("static::tcl_platform"), "{}", h.value);
    }

    // clock format hover

    #[test]
    fn scan_clock_specifiers_finds_each_specifier() {
        let s = scan_clock_specifiers("%Y-%m-%d %H:%M:%S");
        assert_eq!(s, vec!["%Y", "%m", "%d", "%H", "%M", "%S"]);
    }

    #[test]
    fn scan_clock_specifiers_handles_locale_prefix() {
        let s = scan_clock_specifiers("%EY-%Om");
        assert_eq!(s, vec!["%EY", "%Om"]);
    }

    #[test]
    fn scan_clock_specifiers_handles_literal_percent() {
        let s = scan_clock_specifiers("100%% complete");
        assert_eq!(s, vec!["%%"]);
    }

    #[test]
    fn clock_format_hover_renders_specifier_table() {
        let text = clock_format_hover_text("%Y-%m-%d");
        assert!(text.contains("**Clock format string**"), "{text}");
        assert!(text.contains("| `%Y` | 4-digit year |"), "{text}");
        assert!(text.contains("| `%m` | Month (01–12) |"), "{text}");
        assert!(text.contains("| `%d` | Day of month (01–31) |"), "{text}");
    }

    #[test]
    fn clock_format_hover_marks_locale_modified_specifiers() {
        let text = clock_format_hover_text("%EY");
        assert!(text.contains("(locale-modified)"), "{text}");
    }

    #[test]
    fn clock_format_hover_handles_empty_format() {
        let text = clock_format_hover_text("no specifiers here");
        assert!(text.contains("No specifiers found"), "{text}");
    }

    #[test]
    fn clock_format_string_at_position_detects_braced_literal() {
        let src = "clock format $time {%Y-%m-%d}\n";
        // Cursor inside the `{...}` literal.
        let found = clock_format_string_at_position(src, 0, 22);
        assert_eq!(found.as_deref(), Some("%Y-%m-%d"));
    }

    #[test]
    fn clock_format_string_at_position_detects_quoted_literal() {
        let src = "clock format $time \"%Y\"\n";
        // Cursor inside the `"..."` literal.
        let found = clock_format_string_at_position(src, 0, 22);
        assert_eq!(found.as_deref(), Some("%Y"));
    }

    #[test]
    fn clock_format_string_at_position_skips_non_clock_commands() {
        let src = "puts \"%Y\"\n";
        let found = clock_format_string_at_position(src, 0, 7);
        assert!(found.is_none(), "{found:?}");
    }

    #[test]
    fn hover_fires_for_clock_format_specifier() {
        let src = "clock format $time {%Y-%m-%d}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor inside the format literal.
        let h = hover(src, 0, 22, &analysis, None).expect("hover");
        assert!(
            h.value.contains("Clock format string"),
            "expected clock hover, got: {value}",
            value = h.value,
        );
    }

    // sprintf format hover

    #[test]
    fn scan_sprintf_specifiers_finds_basic_types() {
        let s = scan_sprintf_specifiers("%d - %s : %x");
        assert_eq!(s, vec!["%d", "%s", "%x"]);
    }

    #[test]
    fn scan_sprintf_specifiers_captures_width_and_precision() {
        let s = scan_sprintf_specifiers("%05d %-10s %.3f");
        assert_eq!(s, vec!["%05d", "%-10s", "%.3f"]);
    }

    #[test]
    fn scan_sprintf_specifiers_captures_positional() {
        let s = scan_sprintf_specifiers("%1$s %2$d");
        assert_eq!(s, vec!["%1$s", "%2$d"]);
    }

    #[test]
    fn scan_sprintf_specifiers_handles_literal_percent() {
        let s = scan_sprintf_specifiers("%% done");
        assert_eq!(s, vec!["%%"]);
    }

    #[test]
    fn sprintf_format_hover_renders_specifier_table() {
        let text = sprintf_format_hover_text("%d - %s");
        assert!(text.contains("**Format string** (sprintf-style)"), "{text}");
        assert!(text.contains("| `%d` | Signed decimal integer |"), "{text}");
        assert!(text.contains("| `%s` | String |"), "{text}");
    }

    #[test]
    fn sprintf_format_hover_handles_empty_format() {
        let text = sprintf_format_hover_text("no specifiers here");
        assert!(text.contains("No specifiers found"), "{text}");
    }

    #[test]
    fn sprintf_format_string_at_position_detects_braced_literal() {
        let src = "format {%d items} $count\n";
        let found = sprintf_format_string_at_position(src, 0, 10);
        assert_eq!(found.as_deref(), Some("%d items"));
    }

    #[test]
    fn sprintf_format_string_at_position_detects_quoted_literal() {
        let src = "format \"%d\" 42\n";
        let found = sprintf_format_string_at_position(src, 0, 9);
        assert_eq!(found.as_deref(), Some("%d"));
    }

    #[test]
    fn sprintf_format_string_at_position_skips_non_format_commands() {
        let src = "puts \"%d\"\n";
        let found = sprintf_format_string_at_position(src, 0, 7);
        assert!(found.is_none(), "{found:?}");
    }

    #[test]
    fn hover_fires_for_sprintf_specifier() {
        let src = "format {%d items} $count\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 10, &analysis, None).expect("hover");
        assert!(
            h.value.contains("Format string"),
            "expected sprintf hover, got: {value}",
            value = h.value,
        );
    }

    // binary format hover

    fn binary_ctx(text: &str) -> BinaryContext {
        BinaryContext {
            text: text.to_string(),
            subcmd: "format".to_string(),
            args: Vec::new(),
        }
    }

    #[test]
    fn scan_binary_fields_finds_basic_types() {
        let fields = scan_binary_fields("a4 H2 i");
        let fulls: Vec<&str> = fields.iter().map(|f| f.full.as_str()).collect();
        assert_eq!(fulls, vec!["a4", "H2", "i"]);
        assert_eq!(fields[0].byte_size, Some(4));
        assert_eq!(fields[1].byte_size, Some(1));
        assert_eq!(fields[2].byte_size, Some(4));
    }

    #[test]
    fn scan_binary_fields_handles_star_count() {
        let fields = scan_binary_fields("a* I*");
        assert!(fields[0].star);
        assert!(fields[1].star);
        assert_eq!(fields[0].byte_size, None);
        assert_eq!(fields[1].byte_size, None);
    }

    #[test]
    fn binary_format_hover_renders_summary_and_detail_table() {
        let text = binary_format_hover_text(&binary_ctx("a4 i"));
        assert!(text.contains("**binary format**"), "{text}");
        assert!(text.contains("2 fields"), "{text}");
        assert!(text.contains("8 bytes"), "{text}");
        // Detail table now has 4 columns and a Bytes column.
        assert!(
            text.contains("| Spec | Variable | Type | Bytes |"),
            "{text}"
        );
        assert!(
            text.contains("| `a4` | a4 | str (null-pad) | 4 |"),
            "{text}"
        );
        assert!(text.contains("| `i` | i | int32 LE | 4 |"), "{text}");
    }

    #[test]
    fn binary_format_hover_renders_byte_ruler_diagram() {
        let text = binary_format_hover_text(&binary_ctx("c s i"));
        // Diagram fenced in a code block.
        assert!(text.contains("```"), "{text}");
        // Box-drawing characters for the field boundaries.
        assert!(text.contains('┌'), "{text}");
        assert!(text.contains('┬'), "{text}");
        assert!(text.contains('┐'), "{text}");
        // Numeric ruler — 7 bytes total (1 + 2 + 4).
        assert!(text.contains("0   1"), "{text}");
    }

    #[test]
    fn binary_format_hover_omits_diagram_when_total_exceeds_32_bytes() {
        // `d` is 8 bytes; five of them = 40 bytes — over the
        // 32-byte diagram budget.
        let text = binary_format_hover_text(&binary_ctx("d5"));
        assert!(text.contains("**binary format**"), "{text}");
        assert!(!text.contains('┌'), "diagram should be skipped: {text}");
    }

    #[test]
    fn binary_format_hover_omits_diagram_when_size_unknown() {
        // `a*` has unknown byte count.
        let text = binary_format_hover_text(&binary_ctx("a*"));
        assert!(!text.contains('┌'), "{text}");
        // The Bytes column still renders `…` for star fields.
        assert!(
            text.contains("| `a*` | a* | str (null-pad) | … |"),
            "{text}"
        );
    }

    #[test]
    fn binary_format_hover_labels_fields_with_arg_names() {
        let ctx = BinaryContext {
            text: "c i".to_string(),
            subcmd: "scan".to_string(),
            args: vec!["byte".to_string(), "word".to_string()],
        };
        let text = binary_format_hover_text(&ctx);
        // Detail-table Variable column gets the real names.
        assert!(text.contains("| `c` | byte |"), "{text}");
        assert!(text.contains("| `i` | word |"), "{text}");
        // Ruler diagram labels also pick up the names.
        assert!(text.contains("byte"), "{text}");
        assert!(text.contains("word"), "{text}");
    }

    #[test]
    fn binary_format_hover_renders_uint_modifier() {
        let text = binary_format_hover_text(&binary_ctx("iu"));
        assert!(text.contains("uint32"), "{text}");
    }

    #[test]
    fn binary_format_hover_no_specifiers_returns_friendly_message() {
        let text = binary_format_hover_text(&binary_ctx("ZZZ"));
        assert!(text.contains("No specifiers found"), "{text}");
    }

    #[test]
    fn binary_format_context_at_position_detects_braced_literal() {
        let src = "binary format {a4 i} val\n";
        let ctx = binary_format_context_at_position(src, 0, 17).expect("found ctx");
        assert_eq!(ctx.text, "a4 i");
        assert_eq!(ctx.subcmd, "format");
        assert_eq!(ctx.args, vec!["val"]);
    }

    #[test]
    fn binary_format_context_extracts_scan_var_names() {
        // Quoted format string with two trailing var names — the
        // hover should pick up both as the scan target labels.
        let src = "binary scan $buf \"cI\" byte word\n";
        let ctx = binary_format_context_at_position(src, 0, 19).expect("found ctx");
        assert_eq!(ctx.text, "cI");
        assert_eq!(ctx.subcmd, "scan");
        assert_eq!(ctx.args, vec!["byte", "word"]);
    }

    #[test]
    fn hover_fires_for_binary_specifier() {
        let src = "binary format {a4 i} val\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 17, &analysis, None).expect("hover");
        assert!(h.value.contains("binary format"), "{}", h.value);
    }

    // regsub substitution-spec hover

    #[test]
    fn scan_regsub_backrefs_finds_each_backref() {
        let r = scan_regsub_backrefs("\\1-\\2 (\\& and \\0)");
        assert_eq!(r, vec!["\\1", "\\2", "\\&", "\\0"]);
    }

    #[test]
    fn regsub_hover_renders_backref_table() {
        let text = regsub_hover_text("prefix \\1 suffix");
        assert!(text.contains("**Substitution spec**"), "{text}");
        assert!(text.contains("| `\\1` | First capture group |"), "{text}");
    }

    #[test]
    fn regsub_hover_handles_no_backrefs() {
        let text = regsub_hover_text("plain text");
        assert!(text.contains("No backreferences found"), "{text}");
    }

    #[test]
    fn regsub_subspec_at_position_finds_subspec_literal() {
        let src = "regsub foo bar {\\1-baz} out\n";
        // Cursor inside the subspec literal.
        let found = regsub_subspec_at_position(src, 0, 18);
        assert_eq!(found.as_deref(), Some("\\1-baz"));
    }

    #[test]
    fn hover_fires_for_regsub_backref() {
        let src = "regsub foo bar {\\1-baz} out\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 18, &analysis, None).expect("hover");
        assert!(h.value.contains("Substitution spec"), "{}", h.value);
    }

    // glob pattern hover

    #[test]
    fn scan_glob_metachars_finds_star_and_question() {
        let m = scan_glob_metachars("*.tcl");
        let toks: Vec<&str> = m.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"*"), "{m:?}");
    }

    #[test]
    fn scan_glob_metachars_finds_character_class() {
        let m = scan_glob_metachars("[abc]*.tcl");
        let toks: Vec<&str> = m.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"[abc]"), "{m:?}");
        assert!(toks.contains(&"*"), "{m:?}");
    }

    #[test]
    fn glob_hover_renders_table() {
        let text = glob_hover_text("*.tcl");
        assert!(text.contains("**Glob pattern**"), "{text}");
        assert!(text.contains("| `*` |"), "{text}");
    }

    #[test]
    fn glob_hover_for_literal_string() {
        let text = glob_hover_text("plain");
        assert!(text.contains("Literal string"), "{text}");
    }

    #[test]
    fn hover_fires_for_glob_pattern() {
        // Braced glob pattern — single-line literal detection
        // requires `"..."` or `{...}` delimiters.  Bare globs
        // (`glob *.tcl`) fall through to the proc / word
        // lookup; their support lives in the same multi-line
        // / arg-position machinery that other `*-rich`
        // sub-strips defer.
        let src = "glob {*.tcl}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor inside the braced pattern.
        let h = hover(src, 0, 8, &analysis, None).expect("hover");
        assert!(h.value.contains("Glob pattern"), "{}", h.value);
    }

    // regex pattern hover

    #[test]
    fn scan_regex_components_finds_anchors_and_quantifiers() {
        let r = scan_regex_components("^foo.*$");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"^"), "{r:?}");
        assert!(toks.contains(&"."), "{r:?}");
        assert!(toks.contains(&"*"), "{r:?}");
        assert!(toks.contains(&"$"), "{r:?}");
    }

    #[test]
    fn scan_regex_components_finds_character_class() {
        let r = scan_regex_components("[a-z]+");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"[a-z]"), "{r:?}");
        assert!(toks.contains(&"+"), "{r:?}");
    }

    #[test]
    fn scan_regex_components_finds_escapes() {
        let r = scan_regex_components("\\d+");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"\\d"), "{r:?}");
    }

    #[test]
    fn scan_regex_components_finds_groups_and_lookahead() {
        let r = scan_regex_components("(?:foo)(?=bar)");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"(?:"), "{r:?}");
        assert!(toks.contains(&"(?="), "{r:?}");
    }

    #[test]
    fn regex_hover_renders_table() {
        let text = regex_hover_text("^foo$");
        assert!(text.contains("**Regex pattern**"), "{text}");
        assert!(text.contains("| `^` |"), "{text}");
    }

    #[test]
    fn hover_fires_for_regex_pattern() {
        let src = "regexp {^foo.*$} $line\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor inside the pattern literal.
        let h = hover(src, 0, 10, &analysis, None).expect("hover");
        assert!(h.value.contains("Regex pattern"), "{}", h.value);
    }

    // IP address hover

    #[test]
    fn ip_hover_classifies_private_ipv4() {
        let t = ip_address_hover_text("10.0.0.1").expect("hover");
        assert!(t.contains("IPv4 address"), "{t}");
        assert!(t.contains("Private (RFC 1918)"), "{t}");
    }

    #[test]
    fn ip_hover_classifies_loopback() {
        let t = ip_address_hover_text("127.0.0.1").expect("hover");
        assert!(t.contains("Loopback"), "{t}");
    }

    #[test]
    fn ip_hover_classifies_public_ipv4() {
        let t = ip_address_hover_text("8.8.8.8").expect("hover");
        assert!(t.contains("Public"), "{t}");
    }

    #[test]
    fn ip_hover_renders_cidr_prefix() {
        let t = ip_address_hover_text("10.0.0.0/8").expect("hover");
        assert!(t.contains("CIDR network: `10.0.0.0/8`"), "{t}");
    }

    #[test]
    fn ip_hover_classifies_ipv6_loopback() {
        let t = ip_address_hover_text("::1").expect("hover");
        assert!(t.contains("IPv6 address"), "{t}");
        assert!(t.contains("Loopback"), "{t}");
    }

    #[test]
    fn ip_hover_detects_ipv4_mapped_ipv6() {
        let t = ip_address_hover_text("::ffff:192.0.2.1").expect("hover");
        assert!(t.contains("IPv4-mapped"), "{t}");
    }

    #[test]
    fn ip_hover_rejects_non_ip_strings() {
        assert!(ip_address_hover_text("hello").is_none());
        assert!(ip_address_hover_text("256.256.256.256").is_none());
        assert!(ip_address_hover_text("not.an.ip.address").is_none());
    }

    #[test]
    fn hover_fires_for_ip_address_word() {
        let src = "set host 10.0.0.1\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor on `10.0.0.1`.
        let h = hover(src, 0, 11, &analysis, None).expect("hover");
        assert!(h.value.contains("IPv4 address"), "{}", h.value);
    }

    // registry-driven hovers

    #[test]
    fn builtin_command_hover_surfaces_summary_from_registry() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let t =
            builtin_command_hover_text(&registry, "puts", &analyse(""), u32::MAX).expect("hover");
        assert!(t.contains("built-in command"), "{t}");
        assert!(t.contains("`puts`"), "{t}");
    }

    #[test]
    fn builtin_command_hover_lists_subcommands() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let t =
            builtin_command_hover_text(&registry, "string", &analyse(""), u32::MAX).expect("hover");
        assert!(t.contains("Subcommands:"), "{t}");
        assert!(t.contains("length"), "{t}");
    }

    #[test]
    fn builtin_command_hover_returns_none_for_unknown() {
        let registry = tcl_registry::CommandRegistry::build_default();
        assert!(
            builtin_command_hover_text(&registry, "totallyMadeUpCommand", &analyse(""), u32::MAX)
                .is_none()
        );
    }

    #[test]
    fn subcommand_hover_surfaces_for_string_length() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "string length $name\n";
        let t =
            subcommand_hover_text(src, 0, 10, &registry, "length", ALL).expect("subcommand hover");
        assert!(t.contains("`string length`"), "{t}");
        assert!(t.contains("subcommand"), "{t}");
    }

    #[test]
    fn subcommand_hover_resolves_unique_prefix_abbreviation() {
        // `string le` abbreviates `string length`; hover resolves it and shows
        // the canonical name.
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "string le $name\n";
        let t = subcommand_hover_text(src, 0, 8, &registry, "le", ALL).expect("prefix hover");
        assert!(t.contains("`string length`"), "{t}");
    }

    #[test]
    fn subcommand_hover_prefix_is_dialect_aware() {
        use tcl_registry::dialects::DialectSet;
        let registry = tcl_registry::CommandRegistry::build_default();
        // `info class def` is `definition` in 8.6 (unique) but ambiguous with
        // `definitionnamespace` in 9.0 (verified against tclsh).
        let src = "info class def ::C\n";
        let t86 = sub_subcommand_hover_text(src, 0, 11, &registry, "def", DialectSet::TCL86);
        assert!(
            t86.is_some_and(|t| t.contains("`info class definition`")),
            "8.6 should resolve `def` to definition",
        );
        assert!(
            sub_subcommand_hover_text(src, 0, 11, &registry, "def", DialectSet::TCL90).is_none(),
            "9.0 `def` is ambiguous — no hover",
        );
        // `string rev` (reverse, 8.5+) hovers in 8.6 but not in 8.4.
        let src = "string rev abc\n";
        assert!(subcommand_hover_text(src, 0, 8, &registry, "rev", DialectSet::TCL86).is_some(),);
        assert!(
            subcommand_hover_text(src, 0, 8, &registry, "rev", DialectSet::TCL84).is_none(),
            "`string rev` is unknown in 8.4",
        );
    }

    #[test]
    fn subcommand_hover_skips_unknown_subcommand() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "string bogusSubcommand\n";
        assert!(subcommand_hover_text(src, 0, 12, &registry, "bogusSubcommand", ALL).is_none());
    }

    #[test]
    fn option_hover_resolves_subcommand_scoped_option() {
        use tcl_registry::dialects::DialectSet;
        // `-inputmode` lives only on `chan`'s `configure` SubCommand table —
        // absent from `chan`'s own top-level option table entirely — so
        // hover must resolve the typed subcommand to find it at all.
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "chan configure $chan -inputmode raw\n";
        let t = option_hover_text(src, 0, 30, &registry, "inputmode", DialectSet::TCL90)
            .expect("hover should resolve the configure-scoped option");
        assert!(t.contains("`-inputmode`"), "{t}");
        assert!(t.contains("chan configure"), "{t}");
    }

    #[test]
    fn option_hover_notes_dialect_unavailability() {
        use tcl_registry::dialects::DialectSet;
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "chan configure $chan -inputmode raw\n";
        let old = option_hover_text(src, 0, 30, &registry, "inputmode", DialectSet::TCL86)
            .expect("hover should still resolve the option under an older dialect");
        assert!(old.contains("Not available in the active dialect"), "{old}");
        let new = option_hover_text(src, 0, 30, &registry, "inputmode", DialectSet::TCL90)
            .expect("hover should resolve under tcl9.0");
        assert!(
            !new.contains("Not available in the active dialect"),
            "{new}"
        );
    }

    #[test]
    fn option_hover_falls_back_to_top_level_options_for_simple_command() {
        use tcl_registry::dialects::DialectSet;
        // `lsearch` has no subcommands at all — hover must still resolve its
        // own top-level option table (the pre-existing, non-ensemble path).
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "lsearch -exact {a b} x\n";
        let t = option_hover_text(src, 0, 14, &registry, "exact", DialectSet::TCL86)
            .expect("hover should resolve a simple command's own option");
        assert!(t.contains("`-exact`"), "{t}");
        assert!(t.contains("of `lsearch`"), "{t}");
    }

    #[test]
    fn sub_subcommand_hover_surfaces_for_info_object_class() {
        // Issue #798 fix 3: hovering the third word of `info object class`
        // returns the second-level subcommand's doc.
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "info object class $obj\n";
        let t =
            sub_subcommand_hover_text(src, 0, 12, &registry, "class", ALL).expect("sub-sub hover");
        assert!(t.contains("`info object class`"), "{t}");
        assert!(t.contains("subcommand"), "{t}");
        // Unique-prefix abbreviation resolves to the canonical op.
        let src = "info class super $cls\n";
        let t =
            sub_subcommand_hover_text(src, 0, 11, &registry, "super", ALL).expect("prefix hover");
        assert!(t.contains("`info class superclasses`"), "{t}");
        // The first-level subcommand word itself is not a sub-subcommand.
        assert!(
            sub_subcommand_hover_text("info object class\n", 0, 5, &registry, "object", ALL)
                .is_none()
        );
    }

    #[test]
    fn hover_fires_for_builtin_command_with_registry() {
        let src = "puts hello\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 2, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("built-in command"), "{}", h.value);
    }

    #[test]
    fn hover_fires_for_subcommand_with_registry() {
        let src = "string length $name\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 10, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("subcommand"), "{}", h.value);
    }

    // docstring formatting

    #[test]
    fn format_docstring_renders_brief_param_return_tags() {
        let raw = concat!(
            "@brief Greet someone\n",
            "Free-form description\n",
            "spanning two lines.\n",
            "@param name the person's name\n",
            "@param greeting optional greeting prefix\n",
            "@return the formatted greeting\n",
        );
        let rendered = format_docstring(raw);
        assert!(rendered.contains("Greet someone"), "{rendered}");
        assert!(rendered.contains("Free-form description"), "{rendered}");
        assert!(rendered.contains("**Parameters:**"), "{rendered}");
        assert!(
            rendered.contains("- **name** \u{2014} the person's name"),
            "{rendered}",
        );
        assert!(
            rendered.contains("- **greeting** \u{2014} optional greeting prefix"),
            "{rendered}",
        );
        assert!(
            rendered.contains("**Returns:** the formatted greeting"),
            "{rendered}",
        );
    }

    #[test]
    fn format_docstring_drops_decoration_lines() {
        // Pure-decoration lines (`.....`, `-----`) shouldn't
        // pollute the description block.
        let raw = "..........\nA description.\n..........\n";
        let rendered = format_docstring(raw);
        assert_eq!(rendered, "A description.");
    }

    #[test]
    fn format_docstring_passes_through_plain_text() {
        let raw = "Just a free-form description.\nNo tags here.\n";
        let rendered = format_docstring(raw);
        assert!(rendered.contains("Just a free-form description"));
        assert!(rendered.contains("No tags here"));
    }

    #[test]
    fn format_docstring_handles_param_without_description() {
        let raw = "@param naked\n";
        let rendered = format_docstring(raw);
        assert!(
            rendered.contains("- **naked**"),
            "expected bare param entry; got {rendered}",
        );
        // No trailing em-dash since there's no description.
        assert!(!rendered.contains("**naked** \u{2014}"), "{rendered}");
    }

    // class-member hover

    #[test]
    fn class_member_hover_fires_for_method_inside_body() {
        let src = "oo::class create C {\n    method greet {who} {}\n    method twice {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` invocation (line 2,
        // col 22).
        let h = hover(src, 2, 22, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("C::greet"), "{}", h.value);
        assert!(h.value.contains("1 param"), "{}", h.value);
    }

    #[test]
    fn class_member_hover_fires_for_classmethod() {
        let src = "oo::class create C {\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
        let analysis = analyse(src);
        let h = hover(src, 2, 20, &analysis, None).expect("hover");
        assert!(h.value.contains("**classmethod**"), "{}", h.value);
        assert!(h.value.contains("C::factory"), "{}", h.value);
    }

    #[test]
    fn class_member_hover_fires_for_constructor_keyword() {
        let src = "oo::class create C {\n    constructor {arg} {}\n    method touch_ctor {} { constructor }\n}\n";
        let analysis = analyse(src);
        let h = hover(src, 2, 27, &analysis, None).expect("hover");
        assert!(h.value.contains("constructor"), "{}", h.value);
        // Class qualified name is `::C`.
        assert!(h.value.contains("::C"), "{}", h.value);
    }

    #[test]
    fn class_member_hover_skipped_outside_class_body() {
        let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the bare `greet` outside the class body.
        // No proc / class / method match — should return None.
        assert!(hover(src, 3, 2, &analysis, None).is_none());
    }

    // $obj method dispatch

    #[test]
    fn obj_method_hover_fires_for_known_instance() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
        let analysis = analyse(src);
        // Line 4 `$d bark` — cursor on `bark` (col 3).
        let h = hover(src, 4, 3, &analysis, None).expect("hover");
        assert!(h.value.contains("**method**"), "{}", h.value);
        assert!(h.value.contains("::Dog::bark"), "{}", h.value);
    }

    #[test]
    fn obj_method_hover_none_for_unknown_instance() {
        let src = "oo::class create Dog {\n    method bark {} {}\n}\n$x bark\n";
        let analysis = analyse(src);
        // `x` has no recorded class — no hover.
        assert!(hover(src, 3, 3, &analysis, None).is_none());
    }

    /// A Tk widget's instance command (a registry-modelled, self-referential
    /// `object_class`, not a user-defined one — issue #927) needs the
    /// registry passed to resolve hover text at all.
    #[test]
    fn obj_method_hover_fires_for_bareword_widget() {
        let reg = tcl_registry::CommandRegistry::build_default();
        let src = "ttk::treeview .t\n.t instate {selected} {}\n";
        let analysis = analyse(src);
        // Line 1 `.t instate …` — cursor on `instate` (col 3).
        let h = hover(src, 1, 3, &analysis, Some(&reg)).expect("hover");
        assert!(h.value.contains("ttk::treeview instate"), "{}", h.value);
    }

    /// Without a registry passed, the same widget dispatch has nowhere to
    /// look up the method — `obj_method_hover_text`'s registry fallback is
    /// skipped, not a panic.
    #[test]
    fn obj_method_hover_none_for_widget_without_registry() {
        let src = "ttk::treeview .t\n.t instate {selected} {}\n";
        let analysis = analyse(src);
        assert!(hover(src, 1, 3, &analysis, None).is_none());
    }

    #[test]
    fn obj_method_hover_fires_for_var_captured_widget() {
        let reg = tcl_registry::CommandRegistry::build_default();
        let src = "set lb [listbox .l]\n$lb curselection\n";
        let analysis = analyse(src);
        // Line 1 `$lb curselection` — cursor on `curselection` (col 4).
        let h = hover(src, 1, 4, &analysis, Some(&reg)).expect("hover");
        assert!(h.value.contains("listbox curselection"), "{}", h.value);
    }
}
