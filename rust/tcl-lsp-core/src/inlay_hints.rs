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

//! Inlay-hints provider (parameter-name hints).
//!
//! Surfaces `param_name:` hints at each positional argument
//! of every user-proc call site within the requested document
//! range.  The hints make it easy to see which argument goes
//! where without having to hover or look up the proc
//! signature.
//!
//! Example: for `proc greet {name greeting} { ... }` and a
//! call site `greet alice hello`, the provider emits:
//!
//! ```text
//! greet (name:)alice (greeting:)hello
//! ```
//!
//! Implementation: re-segments the source on each request so
//! per-argument token spans are available (the analyser
//! records the command-head span on `command_invocations` but
//! not per-arg spans).  Re-segmenting is cheap relative to
//! the LSP request rate.
//!
//! Built-in command hints: when a registry is
//! provided and the call's head matches a built-in command (or
//! `cmd subcommand`), the provider parses the spec's synopsis
//! for positional parameter names and labels the matching
//! call-site args.  Synopsis flags (`?-nocase?`,
//! `?-length length?`) are skipped on both sides — synopsis
//! parsing drops flag tokens, and call-site args that look
//! like flags (start with `-`) don't consume a positional
//! slot.  Varargs (`?name ...?`) stop the parse.
//!
//! Limitations:
//!
//! * Type / inferred-trait annotations on hints (e.g.
//!   `name:string`, `count:int`) are not shown.
//! * Method-call hints inside class bodies — needs the
//!   analyser's method-resolution machinery — are not shown.

use rustc_hash::FxHashMap;
use tcl_compiler::analyser::{AnalysisResult, ProcDef, Scope, ScopeKind};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::registry_invocation::segmented_command_arguments;
use tcl_compiler::types::{TypeKind, TypeLattice};
use tcl_lexer::LineIndex;
use tcl_registry::{CommandRegistry, InvocationArguments, TclType};

use crate::definition::LspRange;

/// Which inlay-hint family a hint belongs to.
///
/// Mirrors the LSP `InlayHintKind` split the server lifts to: inferred
/// variable types and format-string specifier labels are [`Self::Type`];
/// the proc / built-in parameter-name labels at call sites are
/// [`Self::Parameter`].  Each family is gated independently
/// (`inlayTypeHints` / `inlayParameterHints`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayHintKind {
    /// Inferred variable type or format-specifier annotation (`: int`).
    Type,
    /// Call-site parameter-name label (`name:`).
    Parameter,
}

/// One inlay-hint entry — position plus label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlayHint {
    /// Anchor position for the hint (typically the start of
    /// an argument token).
    pub position_line: u32,
    /// Anchor character.
    pub position_character: u32,
    /// Hint label (e.g. `name:`).
    pub label: String,
    /// Which family this hint belongs to (Type vs Parameter).
    pub kind: InlayHintKind,
    /// Whether the editor should pad a space to the left of the label
    /// (the inferred-type / format hints render as ` : int`).
    pub padding_left: bool,
}

/// Compute inlay hints for `range` in `source`.
///
/// `analysis` provides the proc-name → parameter-list lookup
/// the hints need.  When `analysis` is `None` (a stub-only
/// caller from the minimal port), returns an empty vector.
/// `registry`, when `Some`, additionally surfaces parameter-
/// name hints for built-in commands via their synopsis (and is
/// required for the inferred-type hints, which build a
/// [`CompilationUnit`] from it).
///
/// `type_hints` gates the inferred-variable-type annotations and the
/// format-string specifier labels ([`InlayHintKind::Type`]);
/// `parameter_hints` gates the call-site parameter-name labels
/// ([`InlayHintKind::Parameter`]).  Each family is requested
/// independently so an editor can opt into one without the other.
#[must_use]
pub fn inlay_hints(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: LspRange,
    analysis: Option<&AnalysisResult>,
    registry: Option<&CommandRegistry>,
    type_hints: bool,
    parameter_hints: bool,
) -> Vec<InlayHint> {
    inlay_hints_in_program(
        source,
        dialect,
        range,
        analysis,
        crate::definition::CallResolution {
            registry,
            program: None,
        },
        type_hints,
        parameter_hints,
    )
}

/// [`inlay_hints`] with the caller's whole-program export view attached — the
/// entry point a host with a workspace index should call.
///
/// The parameter-name hints are labelled from *the proc the call actually
/// reaches*, so a `namespace import -force` whose covering `namespace export`
/// lives in another file changes which parameter names are correct here
/// (issue #1116 item 1).  Hinting the shadowed local proc's parameters over a
/// call that runs the imported one is a wrong answer, not a missing one.
///
/// `resolution` carries the registry and the oracle together rather than as
/// two parameters, which also keeps this entry point at the same arity as
/// [`inlay_hints`].
#[must_use]
pub fn inlay_hints_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: LspRange,
    analysis: Option<&AnalysisResult>,
    resolution: crate::definition::CallResolution<'_>,
    type_hints: bool,
    parameter_hints: bool,
) -> Vec<InlayHint> {
    let registry = resolution.registry;
    if !type_hints && !parameter_hints {
        return Vec::new();
    }
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();

    if type_hints && let Some(registry) = registry {
        collect_type_hints(
            source,
            dialect,
            analysis,
            registry,
            range,
            &line_index,
            &mut out,
        );
        // Format-string specifier labels are registry-driven too (which
        // words carry a conversion string, and in which mini-language), so
        // they need the registry the same way the type hints do.
        collect_format_string_hints(source, dialect, registry, range, &line_index, &mut out);
    }

    if parameter_hints {
        // The segmenter has no sound way to start mid-file (a command
        // boundary requires tracking brace/bracket nesting from the top), so
        // it still walks the whole document. What a requested viewport buys
        // is skipping the *resolution* work per segment — the namespace walk
        // (`lookup_proc`) and the registry synopsis lookup — for a command
        // that falls entirely outside the range: cheap span arithmetic on
        // data the segmenter already produced, applied before any of that
        // work runs, not after (the `position_within_range` filter inside
        // `emit_hints_for_call`/`emit_builtin_hints` only ever trimmed the
        // *output*).
        let range_start_off = line_index.offset_at_utf16(
            range.start_line,
            tcl_lexer::Utf16Col::new(range.start_character),
            source,
        );
        let range_end_off = line_index.offset_at_utf16(
            range.end_line,
            tcl_lexer::Utf16Col::new(range.end_character),
            source,
        );
        let segments = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
        );
        for seg in &segments {
            if seg.texts.is_empty() || seg.argv.is_empty() {
                continue;
            }
            if seg.span.end() <= range_start_off || seg.span.start() >= range_end_off {
                continue;
            }
            let cmd_name = &seg.texts[0];
            // Resolve the call from the namespace its command token sits in, the
            // way C Tcl would (caller namespace, then global): a proc buried in
            // an unrelated namespace never captures the argument hints, and a
            // same-named builtin keeps its own hints unless a proc is visible.
            let cmd_off = seg.argv[0].span.start();
            if let Some(proc_def) = lookup_proc(analysis, source, cmd_off, cmd_name, resolution) {
                emit_hints_for_call(source, seg, proc_def, &line_index, range, &mut out);
                continue;
            }
            // Built-in command — parse the registry synopsis for
            // positional parameter names.  User procs take
            // precedence (handled above).
            if let Some(registry) = registry
                && let Some(spec) = registry.get(cmd_name)
            {
                emit_builtin_hints(source, seg, spec, &line_index, range, &mut out);
            }
        }
    }

    out
}

/// Short display name for a Tcl intrep type.
fn short_type(t: TclType) -> &'static str {
    match t {
        TclType::String => "str",
        TclType::Int => "int",
        TclType::Double => "double",
        TclType::Boolean => "bool",
        TclType::List => "list",
        TclType::Dict => "dict",
        TclType::ByteArray => "bytes",
        TclType::Numeric => "num",
        TclType::Object => "object",
        TclType::Channel => "channel",
    }
}

/// Render a [`TypeLattice`] as its inlay display string, or `None`
/// when the lattice carries no concrete type (Unknown / Overdefined).
/// A `Known` type shows `int`; a shimmer union chains every member
/// (`int → str`, or `int → list → str` for the 3+-way merges the union
/// lattice tracks).
fn type_display(tl: &TypeLattice) -> Option<String> {
    match tl.kind() {
        TypeKind::Known => tl.tcl_type().map(|t| short_type(t).to_owned()),
        TypeKind::Shimmered => {
            let members: Vec<&str> = tl
                .shapes()
                .iter()
                .map(|shape| short_type(shape.coarse()))
                .collect();
            Some(members.join(" \u{2192} "))
        }
        TypeKind::Unknown | TypeKind::Overdefined => None,
    }
}

/// Collect inferred-variable-type hints (`: int`) for variable
/// definitions in `range`.  Builds a name → display-type map from the
/// type-propagation pass
/// (over every function in a fresh [`CompilationUnit`]) and walks the
/// analyser scope tree, annotating each variable definition whose type
/// is known.
fn collect_type_hints(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    range: LspRange,
    line_index: &LineIndex,
    out: &mut Vec<InlayHint>,
) {
    let cu = CompilationUnit::build_for_profile(source, registry, false, dialect);

    // Build a *per-function* name → display map, keyed by the function's
    // qualified name (leading `::` stripped so it matches the analyser's
    // scope names). A single flat map was last-writer-wins across functions,
    // so a var named `x` typed Int in one proc bled its type onto an unrelated
    // `x` typed String in another. Keeping the maps separate
    // and selecting by the owning scope keeps each scope's types local.
    let mut by_function: FxHashMap<String, FxHashMap<String, String>> = FxHashMap::default();
    for fu in cu.functions() {
        let mut m: FxHashMap<String, String> = FxHashMap::default();
        for ((name, _ver), tl) in fu.types.iter() {
            if let Some(display) = type_display(tl) {
                m.insert(fu.ssa.var_name(*name).to_owned(), display);
            }
        }
        if !m.is_empty() {
            by_function.insert(normalise_fn_name(&fu.name), m);
        }
    }
    if by_function.is_empty() {
        return;
    }

    let empty = FxHashMap::default();
    let top = by_function
        .get(&normalise_fn_name(&cu.top_level.name))
        .unwrap_or(&empty);
    walk_scope_type_hints(
        &analysis.global_scope,
        &by_function,
        top,
        range,
        source,
        line_index,
        out,
        0,
    );
}

/// Normalise a function / scope name to a comparable key: the analyser's proc
/// scope names are unqualified (`a`) while a `FunctionUnit`'s name is fully
/// qualified (`::a`), so strip a single leading `::`.
fn normalise_fn_name(name: &str) -> String {
    name.strip_prefix("::").unwrap_or(name).to_owned()
}

/// Recursively emit type hints for every variable definition in `scope`
/// (and its children) whose name carries a known type and whose
/// definition falls within `range`.
#[allow(clippy::too_many_arguments)]
fn walk_scope_type_hints(
    scope: &Scope,
    by_function: &FxHashMap<String, FxHashMap<String, String>>,
    type_map: &FxHashMap<String, String>,
    range: LspRange,
    source: &str,
    line_index: &LineIndex,
    out: &mut Vec<InlayHint>,
    depth: u32,
) {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for var_def in scope.variables.values() {
        let Some(type_str) = type_map.get(&var_def.name) else {
            continue;
        };
        let start = line_index.position_at_utf16(var_def.definition_span.start(), source);
        let end = line_index.position_at_utf16(var_def.definition_span.end(), source);
        // The hint sits immediately after the variable name; skip when the
        // definition falls entirely outside the requested range.
        if end.line < range.start_line || start.line > range.end_line {
            continue;
        }
        out.push(InlayHint {
            position_line: end.line,
            position_character: end.character.get(),
            label: format!(": {type_str}"),
            kind: InlayHintKind::Type,
            padding_left: true,
        });
    }
    let empty = FxHashMap::default();
    for child in &scope.children {
        // A proc OR TclOO method body has its own function unit, so switch to
        // that function's type map (never fall back to the enclosing one, which
        // would re-introduce the cross-scope bleeding — a method's locals must
        // not inherit an outer `x`). Namespace / uplevel-0 bodies share the
        // enclosing function's locals, so they keep `type_map`.
        let child_map = if matches!(child.kind, ScopeKind::Proc | ScopeKind::Method) {
            by_function
                .get(&normalise_fn_name(&child.name))
                .unwrap_or(&empty)
        } else {
            type_map
        };
        walk_scope_type_hints(
            child,
            by_function,
            child_map,
            range,
            source,
            line_index,
            out,
            depth + 1,
        );
    }
}

// format-string specifier hints
//
// `Type`-kind hints that annotate the conversion specifiers inside the
// format string
// of `format`/`scan` (`%d` → `int`), `clock format`/`scan` (`%Y` →
// `year`), `binary format`/`scan` (`i` → `i32le`), and the substitution
// backreferences of `regsub` (`\1` → `grp1`).

/// `regsub` substitution backreference (`\0`-`\9`, `\&`).
/// Capture group 1 is the back-reference char.
static REGSUB_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"\\([0-9&])").expect("static regsub regex"));

/// Short label for a `format`/`scan` conversion type.  `%` (literal
/// percent) yields `None` so no hint is emitted for `%%`.
fn sprintf_short(c: char) -> Option<&'static str> {
    Some(match c {
        's' => "str",
        'd' | 'i' => "int",
        'u' => "uint",
        'o' => "oct",
        'x' => "hex",
        'X' => "HEX",
        'f' => "float",
        'e' => "exp",
        'E' => "EXP",
        'g' => "num",
        'G' => "NUM",
        'c' => "char",
        'b' => "bin",
        _ => return None,
    })
}

/// Short label for a `clock` field letter.
fn clock_short(c: char) -> Option<&'static str> {
    Some(match c {
        'Y' => "year",
        'y' => "yr",
        'm' | 'B' | 'n' => "month",
        'd' | 'e' => "day",
        'H' | 'k' => "hour",
        'I' | 'l' => "hour12",
        'M' => "min",
        'S' => "sec",
        's' => "epoch",
        'p' => "AM/PM",
        'a' => "wday",
        'A' => "weekday",
        'b' => "mon",
        'D' | 'x' => "date",
        'j' => "yday",
        'u' | 'w' => "wday#",
        'z' | 'Z' => "tz",
        'C' => "century",
        _ => return None,
    })
}

/// Short label for a `binary format`/`scan` specifier letter.
fn binary_short(c: char) -> Option<&'static str> {
    Some(match c {
        'a' => "strN",
        'A' => "strS",
        'b' => "bits",
        'B' => "BITS",
        'h' => "hex",
        'H' => "HEX",
        'c' => "i8",
        's' => "i16le",
        'S' => "i16be",
        'i' => "i32le",
        'I' => "i32be",
        'n' => "i32",
        'w' => "i64le",
        'W' => "i64be",
        'm' => "i64",
        'r' => "f32le",
        'R' => "f32be",
        'f' => "f32",
        'd' => "f64",
        'q' => "f64le",
        'Q' => "f64be",
        'x' => "pad",
        'X' => "back",
        '@' => "seek",
        't' => "rsv",
        _ => return None,
    })
}

/// Short label for a `regsub` substitution backreference.
fn regsub_short(c: char) -> Option<&'static str> {
    Some(match c {
        '&' | '0' => "match",
        '1' => "grp1",
        '2' => "grp2",
        '3' => "grp3",
        '4' => "grp4",
        '5' => "grp5",
        '6' => "grp6",
        '7' => "grp7",
        '8' => "grp8",
        '9' => "grp9",
        _ => return None,
    })
}

/// The format-string words of one segmented command, as `(argv index,
/// family)` pairs, resolved entirely from the registry.
///
/// This used to be a second, independent copy of the format-family dispatch
/// the semantic-token walk carried — `match head { "format" => …, "scan" =>
/// …, "binary" => …, "clock" => …, "regsub" => … }`, each re-deriving its own
/// argument layout, and neither firing for the explicitly global spellings
/// C Tcl resolves to the same commands (issue #1185). Both now read one
/// registry answer, so they cannot drift.
fn format_args(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    identities: &tcl_compiler::head_identity::HeadIdentityMap,
) -> Vec<(usize, tcl_registry::FormatType)> {
    let Some(head) = seg.texts.first() else {
        return Vec::new();
    };
    let Some(tok) = seg.argv.first() else {
        return Vec::new();
    };
    // Resolve the head's *effective command identity* first, exactly as the
    // semantic-token walk does, so a call through a proven `interp alias` /
    // `rename` gets the target's format family and a `rename`d-away or
    // `proc`-shadowed spelling gets none (issue #1185).
    let resolved = identities.resolve(head, tok.span.start()).spec_name();
    let source_args = segmented_command_arguments(seg);
    registry
        .format_string_args_words(resolved, InvocationArguments::structured(&source_args))
        .into_iter()
        // `+ 1` converts a post-head argument index into an `argv` index
        // (`argv[0]` is the command word).
        .map(|f| (f.index + 1, f.kind))
        .collect()
}

/// Byte range of a format word's *content* — the token span with one
/// layer of `"…"` / `{…}` delimiters stripped.
fn format_content_range(source: &str, span: tcl_lexer::Span) -> Option<(usize, usize)> {
    let mut start = span.start() as usize;
    let mut end = span.end() as usize;
    if start >= end || end > source.len() {
        return None;
    }
    let bytes = source.as_bytes();
    if matches!(bytes[start], b'"' | b'{') {
        start += 1;
    }
    if end > start && matches!(bytes[end - 1], b'"' | b'}') {
        end -= 1;
    }
    (start < end).then_some((start, end))
}

/// Push a `Type`-kind specifier hint at the byte offset `abs_byte`
/// (converted to a UTF-16 position) when it lies within `range`.
fn push_format_hint(
    label: &str,
    abs_byte: usize,
    range: LspRange,
    source: &str,
    line_index: &LineIndex,
    out: &mut Vec<InlayHint>,
) {
    let pos = line_index.position_at_utf16(u32::try_from(abs_byte).unwrap_or(u32::MAX), source);
    if pos.line < range.start_line || pos.line > range.end_line {
        return;
    }
    out.push(InlayHint {
        position_line: pos.line,
        position_character: pos.character.get(),
        label: label.to_owned(),
        kind: InlayHintKind::Type,
        padding_left: true,
    });
}

/// Collect format-string specifier hints for the whole document.
fn collect_format_string_hints(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    range: LspRange,
    line_index: &LineIndex,
    out: &mut Vec<InlayHint>,
) {
    let profile = dialect;
    let segments = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
    );
    // The document's proven command-identity facts, computed once for the
    // whole file (empty, and lookup-free, unless it binds something).
    let identities =
        tcl_compiler::head_identity::command_head_identities(source, dialect, registry);
    for seg in &segments {
        for (idx, kind) in format_args(seg, registry, &identities) {
            let Some(tok) = seg.argv.get(idx) else {
                continue;
            };
            let Some((cstart, cend)) = format_content_range(source, tok.span) else {
                continue;
            };
            let content = &source[cstart..cend];
            match kind {
                tcl_registry::FormatType::Sprintf => {
                    let bytes = content.as_bytes();
                    let mut i = 0;
                    while i < bytes.len() {
                        if bytes[i] != b'%' {
                            i += 1;
                            continue;
                        }
                        let start = i;
                        i += 1;
                        if bytes.get(i) == Some(&b'%') {
                            i += 1;
                            continue;
                        }
                        let mut end = i;
                        let Some(spec) = tcl_syntax::format::parse_spec(bytes, &mut end) else {
                            i = start + 1;
                            continue;
                        };
                        if tcl_cmd_core::format::is_verb(spec.verb)
                            && tcl_cmd_core::format::is_available(&spec, profile)
                            && let Some(label) = sprintf_short(char::from(spec.verb))
                        {
                            push_format_hint(label, cstart + end, range, source, line_index, out);
                        }
                        i = end;
                    }
                }
                tcl_registry::FormatType::Clock => {
                    for spec in tcl_cmd_core::clock::specifiers(content) {
                        let letter = char::from(spec.letter);
                        if let Some(label) = clock_short(letter) {
                            push_format_hint(
                                label,
                                cstart + spec.end,
                                range,
                                source,
                                line_index,
                                out,
                            );
                        }
                    }
                }
                tcl_registry::FormatType::Binary => {
                    collect_binary_hints(content, cstart, range, source, line_index, profile, out);
                }
                tcl_registry::FormatType::Regsub => {
                    for m in REGSUB_RE.captures_iter(content) {
                        let whole = m.get(0).expect("group 0");
                        let ch = m
                            .get(1)
                            .and_then(|g| g.as_str().chars().next())
                            .unwrap_or(' ');
                        if let Some(label) = regsub_short(ch) {
                            push_format_hint(
                                label,
                                cstart + whole.end(),
                                range,
                                source,
                                line_index,
                                out,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Scan a `binary format`/`scan` template using the shared field grammar and
/// emit a hint after each recognised specifier.
fn collect_binary_hints(
    content: &str,
    cstart: usize,
    range: LspRange,
    source: &str,
    line_index: &LineIndex,
    profile: &tcl_dialect::DialectProfile,
    out: &mut Vec<InlayHint>,
) {
    let allow_modifier = tcl_cmd_core::binary::signedness_available(profile);
    for spec in tcl_cmd_core::binary::specifiers(content.as_bytes(), allow_modifier) {
        if let Some(label) = binary_short(char::from(spec.letter)) {
            push_format_hint(label, cstart + spec.end, range, source, line_index, out);
        }
    }
}

/// Emit parameter-name hints for a built-in command call by
/// parsing the spec's synopsis.  Handles the `cmd subcommand`
/// shape: when the spec declares subcommands and the call's
/// first arg names one, the subcommand's synopsis drives the
/// hints (and the subcommand keyword itself isn't labelled).
fn emit_builtin_hints(
    source: &str,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    spec: &tcl_registry::CommandSpec,
    line_index: &LineIndex,
    range: LspRange,
    out: &mut Vec<InlayHint>,
) {
    // Resolve synopsis, the call-arg index at which positional
    // arguments begin, and the option set the registry declares
    // for this command / subcommand.  argv[0] is the command head.
    let (synopsis, skip_words, first_arg_idx, options): (
        &str,
        usize,
        usize,
        &[tcl_registry::prelude::OptionSpec],
    ) = if spec.subcommands.is_empty() {
        let Some(hover) = spec.hover.as_ref() else {
            return;
        };
        let Some(line) = hover.synopsis.first() else {
            return;
        };
        // Synopsis like `set varName ?value?` — skip the
        // command word (1); positional call args start at
        // argv[1].
        (*line, 1, 1, spec.options)
    } else {
        // Subcommand shape: argv[1] should name a subcommand.
        let Some(sub_name) = seg.texts.get(1) else {
            return;
        };
        let Some(sub) = spec.resolve_subcommand(sub_name) else {
            return;
        };
        // Synopsis like `string length string` — skip the
        // command + subcommand words (2); positional call args
        // start at argv[2].
        (sub.synopsis, 2, 2, sub.options)
    };

    let params = param_names_from_synopsis(synopsis, skip_words);
    if params.is_empty() {
        return;
    }

    // Collect the call's positional argument tokens (skipping
    // options).  Whether a `-`-prefixed token is an option is decided
    // by the registry, not its spelling: only tokens matching a
    // declared `OptionSpec` are skipped (and their value too, when the
    // option `takes_value`).  This keeps real positionals like the
    // `-1` in `string index $s -1` — which is no command's option —
    // labelled correctly.  `argv` and `texts` are parallel, indexed by
    // `arg_idx`.
    let positional_args = collect_positional_args(seg, first_arg_idx, options);

    // Pick which synopsis params bind to the supplied positionals.
    // When the call supplies fewer positionals than the synopsis
    // lists, required trailing params win over leading optionals:
    // `puts ?-nonewline? ?channelId? string` called with one arg
    // labels it `string:`, not `channelId:`.
    let selected = select_param_names(&params, positional_args.len());

    for (&arg_idx, name) in positional_args.iter().zip(selected.iter()) {
        let arg_tok = &seg.argv[arg_idx];
        let pos = line_index.position_at_utf16(arg_tok.span.start(), source);
        if !position_within_range(pos.line, pos.character.get(), range) {
            continue;
        }
        out.push(InlayHint {
            position_line: pos.line,
            position_character: pos.character.get(),
            label: format!("{name}:"),
            kind: InlayHintKind::Parameter,
            padding_left: false,
        });
    }
}

/// Walk a call's arguments and return the `argv` indices of the
/// positional (non-option) arguments, in order.  Option tokens
/// matching a declared `OptionSpec` are skipped along with their
/// value when the option `takes_value`; `--` ends option parsing.
fn collect_positional_args(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    first_arg_idx: usize,
    options: &[tcl_registry::prelude::OptionSpec],
) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut arg_idx = first_arg_idx;
    let mut options_ended = false;
    while arg_idx < seg.argv.len() {
        let arg_text = seg.texts.get(arg_idx).map_or("", String::as_str);
        if !options_ended && arg_text.starts_with('-') && arg_text != "-" {
            // `--` ends option parsing; everything after is positional.
            if arg_text == "--" {
                options_ended = true;
                arg_idx += 1;
                continue;
            }
            if let Some(opt) = options.iter().find(|o| o.matches(arg_text)) {
                // Skip the option and the value word(s) it consumes.
                arg_idx += 1 + opt.value_word_count(&seg.texts, arg_idx);
                continue;
            }
        }
        positions.push(arg_idx);
        arg_idx += 1;
    }
    positions
}

/// Choose the ordered param names to label `n_positional` supplied
/// arguments.  Required params are always kept; optional params are
/// filled only up to the slack `max(0, n_positional - n_required)`,
/// so when fewer args are supplied than the synopsis lists the
/// trailing required positionals are preferred over leading optionals.
fn select_param_names(params: &[(String, bool)], n_positional: usize) -> Vec<&str> {
    let n_required = params.iter().filter(|(_, is_opt)| !*is_opt).count();
    let mut optional_budget = n_positional.saturating_sub(n_required);
    let mut selected = Vec::new();
    for (name, is_opt) in params {
        if *is_opt {
            if optional_budget == 0 {
                continue;
            }
            optional_budget -= 1;
        }
        selected.push(name.as_str());
    }
    selected
}

/// Parse positional parameter names out of a command synopsis,
/// dropping the leading `skip_words` command/subcommand tokens.
/// Each entry is `(name, is_optional)` — `?name?` groups are
/// optional, bare `name` tokens are required.  The optional flag
/// lets the emitter prefer required trailing positionals when a
/// call supplies fewer arguments than the synopsis lists (see
/// `emit_builtin_hints`).
///
/// Token grammar (best-effort):
/// * `name` — required positional → `(name, false)`.
/// * `?name?` — optional positional → `(name, true)` (stripped).
/// * `?-flag?` / `-flag` / `?-flag value?` — flag → skipped.
/// * `?options?` / `?switches?` — flag-group placeholders → skipped
///   (the real flags are handled per-call by the registry option
///   table, so these aren't positional params).
/// * `?name ...?` / `...` — varargs → stops the parse.
fn param_names_from_synopsis(synopsis: &str, skip_words: usize) -> Vec<(String, bool)> {
    let groups = synopsis_groups(synopsis);
    let mut names = Vec::new();
    for group in groups.into_iter().skip(skip_words) {
        // Optional group `?...?` — handled *before* the bare-varargs stop
        // so an optional flag/repeat placeholder that happens to carry
        // `...` (e.g. `?option ...?` in `lsearch ?option ...? list pattern`)
        // is skipped, not mistaken for a hard varargs terminator that would
        // drop the real trailing positionals (`list` / `pattern`) after it.
        if let Some(inner) = group.strip_prefix('?').and_then(|g| g.strip_suffix('?')) {
            let inner = inner.trim();
            if inner.starts_with('-') {
                // Optional flag (possibly `-flag value`) — skip.
                continue;
            }
            if inner.is_empty() || inner.contains(char::is_whitespace) {
                // Multi-word optional that isn't a plain name — a flag-group
                // or repeat placeholder (`option ...`, `arg ...`, `-length
                // length`).  Skip conservatively; the real flags are handled
                // per-call by the registry option table.
                continue;
            }
            // Flag-group documentation placeholders — not real
            // positional params; the registry option table handles
            // the actual flags per-call.
            if inner == "options" || inner == "switches" {
                continue;
            }
            names.push((inner.to_string(), true));
            continue;
        }
        // Non-optional varargs marker (`...` or `name ...`) → stop; a plain
        // positional preceding it is already labelled.
        if group.contains("...") {
            break;
        }
        // Bare flag.
        if group.starts_with('-') {
            continue;
        }
        // Plain required positional.
        if !group.is_empty() {
            names.push((group, false));
        }
    }
    names
}

/// Split a synopsis into whitespace tokens, re-joining
/// `?...?` optional groups that span multiple tokens (e.g.
/// `?-length length?` → one group).
fn synopsis_groups(synopsis: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let mut current: Option<String> = None;
    for tok in synopsis.split_whitespace() {
        match &mut current {
            Some(buf) => {
                buf.push(' ');
                buf.push_str(tok);
                if tok.ends_with('?') {
                    groups.push(current.take().unwrap());
                }
            }
            None => {
                if tok.starts_with('?') && !tok.ends_with('?') {
                    current = Some(tok.to_string());
                } else {
                    groups.push(tok.to_string());
                }
            }
        }
    }
    // Unterminated optional group — keep what we have.
    if let Some(buf) = current {
        groups.push(buf);
    }
    groups
}

/// Resolve the proc a call `name` at byte offset `cmd_off` denotes, using C
/// Tcl's namespace-aware command resolution (the caller's namespace first, then
/// global; `registry`, when `Some`, gates a same-named builtin so a proc in an
/// unrelated namespace can't shadow it).  Replaces a namespace-blind
/// simple-name scan that emitted the wrong proc's parameter hints whenever two
/// namespaces shared a proc name.
fn lookup_proc<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cmd_off: u32,
    name: &str,
    resolution: crate::definition::CallResolution<'_>,
) -> Option<&'a ProcDef> {
    let ns = crate::definition::namespace_context_at(
        &analysis.global_scope,
        cmd_off,
        &analysis.namespace_overrides,
    );
    crate::definition::resolve_called_proc(analysis, source, &ns, name, cmd_off, resolution)
}

/// Walk a single segmented command, emit a hint per argument
/// that falls inside `range`.  Stops at the proc's parameter
/// count — extra arguments (e.g. an `args`-tail proc) don't
/// produce hints.
fn emit_hints_for_call(
    source: &str,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    proc_def: &ProcDef,
    line_index: &LineIndex,
    range: LspRange,
    out: &mut Vec<InlayHint>,
) {
    // argv[0] is the command head; positional args start at
    // index 1.
    for (arg_idx, arg_tok) in seg.argv.iter().enumerate().skip(1) {
        let param_idx = arg_idx - 1;
        let Some(param) = proc_def.params.get(param_idx) else {
            // Past the declared parameter count — proc may
            // have an `args` tail, but we don't emit hints
            // for those (no individual name).
            break;
        };
        // `args` is the conventional tail-collector; skip it
        // even when present.
        if param.name == "args" {
            continue;
        }
        let pos = line_index.position_at_utf16(arg_tok.span.start(), source);
        if !position_within_range(pos.line, pos.character.get(), range) {
            continue;
        }
        out.push(InlayHint {
            position_line: pos.line,
            position_character: pos.character.get(),
            label: format!("{}:", param.name),
            kind: InlayHintKind::Parameter,
            padding_left: false,
        });
    }
}

fn position_within_range(line: u32, character: u32, range: LspRange) -> bool {
    if line < range.start_line {
        return false;
    }
    if line > range.end_line {
        return false;
    }
    if line == range.start_line && character < range.start_character {
        return false;
    }
    if line == range.end_line && character > range.end_character {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    fn whole_document_range(source: &str) -> LspRange {
        let line_count = source.lines().count().max(1);
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: u32::try_from(line_count - 1).unwrap_or(0),
            end_character: u32::MAX,
        }
    }

    #[test]
    fn empty_hints_when_analysis_is_none() {
        let hints = inlay_hints(
            "set x 1\n",
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range("set x 1\n"),
            None,
            None,
            false,
            true,
        );
        assert!(hints.is_empty());
    }

    #[test]
    fn hints_emitted_for_user_proc_call() {
        let src = "proc greet {name greeting} {}\ngreet alice hello\n";
        let analysis = analyse(src);
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            None,
            false,
            true,
        );
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(
            labels.contains(&"name:"),
            "expected `name:` hint; got {labels:?}",
        );
        assert!(
            labels.contains(&"greeting:"),
            "expected `greeting:` hint; got {labels:?}",
        );
    }

    #[test]
    fn hints_anchored_at_argument_start() {
        let src = "proc greet {name} {}\ngreet alice\n";
        let analysis = analyse(src);
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            None,
            false,
            true,
        );
        assert_eq!(hints.len(), 1);
        let h = &hints[0];
        assert_eq!(h.position_line, 1);
        // `greet ` is 6 chars; `alice` starts at column 6.
        assert_eq!(h.position_character, 6);
        assert_eq!(h.label, "name:");
    }

    #[test]
    fn no_hints_for_unknown_command() {
        let src = "unknown_cmd a b c\n";
        let analysis = analyse(src);
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            None,
            false,
            true,
        );
        assert!(hints.is_empty(), "{hints:?}");
    }

    #[test]
    fn no_hints_for_args_tail_parameter() {
        // `args` is the variadic-tail collector — we skip it
        // because there's no individual name to surface.
        let src = "proc many {first args} {}\nmany 1 2 3 4\n";
        let analysis = analyse(src);
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            None,
            false,
            true,
        );
        // Only the `first` arg gets a hint.
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].label, "first:");
    }

    #[test]
    fn qualified_call_hints_from_namespaced_proc() {
        // A qualified `A::greet` call at global scope resolves to the proc in
        // `::A`, so its parameter name (`alpha`) drives the hint — the
        // qualified name is honoured, not tail-matched to a same-named proc.
        let src = "namespace eval A {\n    proc greet {alpha} {}\n}\nA::greet x\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&registry),
            false,
            true,
        );
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(labels.contains(&"alpha:"), "{labels:?}");
    }

    #[test]
    fn namespaced_proc_does_not_hijack_builtin_hints_from_global() {
        // A `proc incr` buried in `::foo` must not supply argument hints for an
        // `incr` call in the global namespace — C Tcl resolves the builtin
        // there, so the builtin's `varName` hint surfaces, never the proc's
        // `custom` parameter.
        let src = "namespace eval foo {\n    proc incr {custom} {}\n}\nincr y\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&registry),
            false,
            true,
        );
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(
            !labels.contains(&"custom:"),
            "proc hijacked builtin: {labels:?}"
        );
    }

    #[test]
    fn extra_args_past_param_count_not_hinted() {
        // `proc one {a} {}` then `one 1 2 3` — only `a` has
        // a corresponding parameter; the extra args produce
        // no hints (no name to attach).
        let src = "proc one {a} {}\none 1 2 3\n";
        let analysis = analyse(src);
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            None,
            false,
            true,
        );
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].label, "a:");
    }

    #[test]
    fn hints_filtered_by_range() {
        // Three lines, each with a proc call.  Range covers
        // only line 2.
        let src = "proc greet {name} {}\ngreet alice\ngreet bob\ngreet charlie\n";
        let analysis = analyse(src);
        let range = LspRange {
            start_line: 2,
            start_character: 0,
            end_line: 2,
            end_character: u32::MAX,
        };
        let hints = inlay_hints(src, tcl_dialect::DialectProfile::by_name("tcl"), range, Some(&analysis), None, false, true);
        assert_eq!(hints.len(), 1, "{hints:?}");
        assert_eq!(hints[0].position_line, 2);
    }

    /// Issue #1160: a command whose span falls entirely outside the
    /// requested range must never reach `lookup_proc` or the registry
    /// synopsis lookup. There is no call counter to assert against through
    /// the public API, so this is parity: the narrow-range result must equal
    /// the in-range subset of the full-range result, for a document whose
    /// out-of-range lines exercise *both* resolution branches the skip has to
    /// short-circuit — an unresolvable call (line 2) and a real builtin call
    /// that would otherwise emit its own hints (line 3, `puts`, two
    /// positionals).
    #[test]
    fn out_of_range_segments_are_skipped_without_changing_in_range_results() {
        let src =
            "proc greet {name} {}\ngreet alice\nunknownproc foo bar\nputs hello world\ngreet zed\n";
        let analysis = analyse(src);
        let reg = registry();
        let full = LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 10,
            end_character: 0,
        };
        let narrow = LspRange {
            start_line: 4,
            start_character: 0,
            end_line: 4,
            end_character: u32::MAX,
        };
        let full_hints = inlay_hints(src, tcl_dialect::DialectProfile::by_name("tcl"), full, Some(&analysis), Some(&reg), false, true);
        let narrow_hints =
            inlay_hints(src, tcl_dialect::DialectProfile::by_name("tcl"), narrow, Some(&analysis), Some(&reg), false, true);
        let expected: Vec<_> = full_hints
            .into_iter()
            .filter(|h| h.position_line == 4)
            .collect();
        assert_eq!(narrow_hints, expected);
        assert_eq!(narrow_hints.len(), 1, "{narrow_hints:?}");
    }

    // built-in command synopsis hints

    fn registry() -> tcl_registry::CommandRegistry {
        tcl_registry::CommandRegistry::build_default()
    }

    /// Regression coverage for issue #996: `walk_scope_type_hints`
    /// recurses once per nested namespace/proc scope, with no depth cap
    /// before this fix (`MAX_SCOPE_WALK_DEPTH`, `crate::lib`). 80 nested
    /// `namespace eval` levels is past the point (confirmed empirically:
    /// 100+) where unguarded namespace-scope recursion overflows `cargo
    /// test`'s bare ~2 MiB per-test default. The assertion is that
    /// `inlay_hints` returns at all, not what it returns.
    #[test]
    fn deeply_nested_namespaces_survive_type_hints() {
        const DEPTH: usize = 80;
        let mut src = String::new();
        for i in 0..DEPTH {
            let _ = writeln!(src, "namespace eval ns{i} {{");
        }
        src.push_str("proc leaf {} { set x 1 }\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        let analysis = analyse(&src);
        let reg = registry();
        let _ = inlay_hints(
            &src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(&src),
            Some(&analysis),
            Some(&reg),
            true,
            false,
        );
    }

    #[test]
    fn synopsis_groups_rejoins_optional_flag_value() {
        let g = synopsis_groups("string compare ?-nocase? ?-length length? a b");
        assert_eq!(
            g,
            vec![
                "string",
                "compare",
                "?-nocase?",
                "?-length length?",
                "a",
                "b"
            ],
        );
    }

    #[test]
    fn param_names_skips_flags_and_keeps_positionals() {
        // After `string compare`, the flags drop out leaving the
        // two required positionals.
        let names = param_names_from_synopsis(
            "string compare ?-nocase? ?-length length? string1 string2",
            2,
        );
        assert_eq!(
            names,
            vec![
                ("string1".to_string(), false),
                ("string2".to_string(), false)
            ],
        );
    }

    #[test]
    fn param_names_stops_at_varargs() {
        let names = param_names_from_synopsis("string cat ?string1? ?string2 ...?", 2);
        // `?string1?` is an optional positional; `?string2 ...?`
        // is varargs → stop.
        assert_eq!(names, vec![("string1".to_string(), true)]);
    }

    #[test]
    fn param_names_tags_optional_and_required() {
        // `puts ?-nonewline? ?channelId? string` — the flag drops,
        // `channelId` is optional, `string` is required.
        let names = param_names_from_synopsis("puts ?-nonewline? ?channelId? string", 1);
        assert_eq!(
            names,
            vec![
                ("channelId".to_string(), true),
                ("string".to_string(), false),
            ],
        );
    }

    #[test]
    fn param_names_drops_flag_group_placeholders() {
        // `?options?` / `?switches?` are flag-group docs, not params.
        let names = param_names_from_synopsis("cmd ?options? path", 1);
        assert_eq!(names, vec![("path".to_string(), false)]);
        let names = param_names_from_synopsis("cmd ?switches? path", 1);
        assert_eq!(names, vec![("path".to_string(), false)]);
    }

    #[test]
    fn param_names_skips_optional_varargs_flag_group_then_labels_positionals() {
        // `lsearch ?option ...? list pattern` (the actual registry hover
        // synopsis) — the `?option ...?` flag-group placeholder must be
        // skipped, NOT treated as a hard varargs stop that drops the real
        // `list` / `pattern` positionals after it.
        let names = param_names_from_synopsis("lsearch ?option ...? list pattern", 1);
        assert_eq!(
            names,
            vec![("list".to_string(), false), ("pattern".to_string(), false)],
        );
        // A *non-optional* `...` repeat marker still stops the parse, after
        // the preceding plain positional is labelled.
        let names = param_names_from_synopsis("cmd first ...", 1);
        assert_eq!(names, vec![("first".to_string(), false)]);
    }

    #[test]
    fn select_prefers_required_trailing_positional() {
        // One supplied arg against `?channelId? string` binds to the
        // required `string`, not the leading optional `channelId`.
        let params = vec![
            ("channelId".to_string(), true),
            ("string".to_string(), false),
        ];
        assert_eq!(select_param_names(&params, 1), vec!["string"]);
        // Two args fill the optional then the required.
        assert_eq!(select_param_names(&params, 2), vec!["channelId", "string"]);
    }

    #[test]
    fn builtin_hint_one_arg_labels_required_trailing_positional() {
        // `puts hello` — one positional arg against
        // `puts ?-nonewline? ?channelId? string` labels it `string:`,
        // not the leading optional `channelId:`.
        let src = "puts hello\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            false,
            true,
        );
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert_eq!(labels, vec!["string:"], "{hints:?}");
    }

    #[test]
    fn builtin_hint_two_args_fill_optional_then_required() {
        // `puts stderr hello` — two positionals fill `channelId` then
        // `string`.
        let src = "puts stderr hello\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            false,
            true,
        );
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert_eq!(labels, vec!["channelId:", "string:"], "{hints:?}");
    }

    #[test]
    fn builtin_hint_for_subcommand_positional() {
        // `string index $s 3` — subcommand `index`, synopsis
        // `string index string charIndex`.
        let src = "string index $s 3\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            false,
            true,
        );
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(labels.contains(&"string:"), "{hints:?}");
        assert!(labels.contains(&"charIndex:"), "{hints:?}");
    }

    #[test]
    fn builtin_hint_skips_call_site_flags() {
        // `string compare -nocase $a $b` — `-nocase` is a flag,
        // so $a→string1, $b→string2.
        let src = "string compare -nocase $a $b\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            false,
            true,
        );
        // The flag token shouldn't be labelled.
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(labels.contains(&"string1:"), "{hints:?}");
        assert!(labels.contains(&"string2:"), "{hints:?}");
        // No hint anchored on the `-nocase` flag.
        for h in &hints {
            assert_ne!(h.position_character, 15, "flag should not be hinted: {h:?}");
        }
    }

    #[test]
    fn builtin_hint_treats_negative_number_as_positional() {
        // `string index $s -1` — the `index` subcommand declares no
        // `-1` option, so the registry-driven walk keeps `-1` as the
        // `charIndex` positional rather than skipping it as a flag.
        let src = "string index $s -1\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            false,
            true,
        );
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(labels.contains(&"string:"), "{hints:?}");
        assert!(labels.contains(&"charIndex:"), "{hints:?}");
    }

    #[test]
    fn builtin_hint_consumes_value_taking_option() {
        // `string compare -length 3 $a $b` — `-length` takes a value,
        // so both `-length` and `3` are skipped; $a→string1, $b→string2.
        let src = "string compare -length 3 $a $b\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            false,
            true,
        );
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        // Only the two positionals are labelled — `-length` and its
        // value `3` are both consumed.
        assert_eq!(labels, vec!["string1:", "string2:"], "{hints:?}");
    }

    #[test]
    fn builtin_hint_not_emitted_without_registry() {
        // Same source, no registry — no built-in hints.
        let src = "string index $s 3\n";
        let analysis = analyse(src);
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            None,
            false,
            true,
        );
        assert!(hints.is_empty(), "{hints:?}");
    }

    #[test]
    fn user_proc_takes_precedence_over_builtin() {
        // A user proc named `string` (contrived) wins over the
        // built-in.  Here we just confirm a user proc still
        // gets its param hints when a registry is also present.
        let src = "proc greet {name} {}\ngreet alice\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            false,
            true,
        );
        let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
        assert!(labels.contains(&"name:"), "{hints:?}");
    }

    // inferred type hints (InlayHintKind::Type)

    #[test]
    fn same_named_vars_in_different_procs_do_not_bleed_types() {
        // `x` is Int in `a` and a String in `b`. The type hint
        // for each `x` must reflect its own proc, not last-writer-wins across a
        // flat name→type map.
        let src = "proc a {} { set x 42 }\nproc b {} { set x \"hi\" }\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            true,
            false,
        );
        let labels_on_line = |line: u32| -> Vec<String> {
            hints
                .iter()
                .filter(|h| h.kind == InlayHintKind::Type && h.position_line == line)
                .map(|h| h.label.clone())
                .collect()
        };
        // Line 0: proc a's `x` is int. Line 1: proc b's `x` is str.
        assert!(
            labels_on_line(0).iter().any(|l| l == ": int"),
            "proc a's x should be int: {:?}",
            labels_on_line(0),
        );
        assert!(
            labels_on_line(0).iter().all(|l| l != ": str"),
            "proc b's str type must not bleed onto proc a: {:?}",
            labels_on_line(0),
        );
        assert!(
            labels_on_line(1).iter().all(|l| l != ": int"),
            "proc a's int type must not bleed onto proc b: {:?}",
            labels_on_line(1),
        );
    }

    #[test]
    fn type_hint_for_integer_set() {
        // `set x 42` → a `: int` Type-kind hint immediately after `x`
        // (character 5).
        let src = "set x 42\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            true,
            false,
        );
        let type_hints: Vec<&InlayHint> = hints
            .iter()
            .filter(|h| h.kind == InlayHintKind::Type)
            .collect();
        assert!(
            type_hints
                .iter()
                .any(|h| h.label == ": int" && h.position_character == 5),
            "{type_hints:?}",
        );
        // No parameter hints leak in when only type hints are requested.
        assert!(
            hints.iter().all(|h| h.kind == InlayHintKind::Type),
            "{hints:?}"
        );
    }

    #[test]
    fn type_hints_require_registry_for_inference() {
        // Without a registry the type map can't be built; no type hints.
        let src = "set x 42\n";
        let analysis = analyse(src);
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            None,
            true,
            false,
        );
        assert!(
            hints.iter().all(|h| h.kind != InlayHintKind::Type),
            "{hints:?}",
        );
    }

    #[test]
    fn type_and_parameter_families_gate_independently() {
        // `set x 42` carries a type hint; `add 1 2` carries parameter hints.
        let src = "proc add {a b} {}\nset x 42\nadd 1 2\n";
        let analysis = analyse(src);
        let reg = registry();
        let type_only = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            true,
            false,
        );
        assert!(!type_only.is_empty(), "expected type hints");
        assert!(
            type_only.iter().all(|h| h.kind == InlayHintKind::Type),
            "{type_only:?}",
        );
        let param_only = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            false,
            true,
        );
        assert!(!param_only.is_empty(), "expected parameter hints");
        assert!(
            param_only
                .iter()
                .all(|h| h.kind == InlayHintKind::Parameter),
            "{param_only:?}",
        );
    }

    #[test]
    fn both_families_disabled_yields_nothing() {
        let src = "set x 42\nadd 1 2\n";
        let analysis = analyse(src);
        let reg = registry();
        let hints = inlay_hints(
            src,
            tcl_dialect::DialectProfile::by_name("tcl"),
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            false,
            false,
        );
        assert!(hints.is_empty(), "{hints:?}");
    }

    // format-string specifier hints (InlayHintKind::Type)

    fn type_labels(src: &str) -> Vec<(u32, String)> {
        type_labels_for_dialect(src, tcl_dialect::DialectProfile::by_name("tcl"))
    }

    fn type_labels_for_dialect(src: &str, dialect: &'static tcl_dialect::DialectProfile) -> Vec<(u32, String)> {
        let analysis = analyse(src);
        let reg = registry();
        inlay_hints(
            src,
            dialect,
            whole_document_range(src),
            Some(&analysis),
            Some(&reg),
            true,
            false,
        )
        .into_iter()
        .filter(|h| h.kind == InlayHintKind::Type)
        .map(|h| (h.position_character, h.label))
        .collect()
    }

    #[test]
    fn format_sprintf_specifier_hints() {
        let labels = type_labels("format \"%d-%s\" 1 two\n");
        let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
        assert!(names.contains(&"int"), "{labels:?}");
        assert!(names.contains(&"str"), "{labels:?}");
    }

    #[test]
    fn format_literal_percent_not_hinted() {
        // `%%` is a literal percent — no specifier hint.
        let labels = type_labels("format \"100%%\"\n");
        assert!(labels.is_empty(), "{labels:?}");
    }

    #[test]
    fn clock_format_specifier_hints() {
        let labels = type_labels("clock format $t -format \"%Y-%m-%d\"\n");
        let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
        assert!(names.contains(&"year"), "{labels:?}");
        assert!(names.contains(&"month"), "{labels:?}");
        assert!(names.contains(&"day"), "{labels:?}");
    }

    #[test]
    fn binary_format_specifier_hints() {
        let labels = type_labels("binary format \"a3 i\" foo 1\n");
        let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
        assert!(names.contains(&"strN"), "{labels:?}");
        assert!(names.contains(&"i32le"), "{labels:?}");
    }

    #[test]
    fn binary_q_and_q_hints_are_owned_by_shared_spec_table() {
        let labels = type_labels_for_dialect("binary format \"q Q\" 1 2\n", tcl_dialect::DialectProfile::by_name("tcl8.6"));
        let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
        assert!(names.contains(&"f64le"), "{labels:?}");
        assert!(names.contains(&"f64be"), "{labels:?}");
    }

    #[test]
    fn regsub_subspec_backreference_hints() {
        let labels = type_labels("regsub {(a)(b)} $s {\\1-\\2} out\n");
        let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
        assert!(names.contains(&"grp1"), "{labels:?}");
        assert!(names.contains(&"grp2"), "{labels:?}");
    }

    #[test]
    fn dynamic_leading_option_does_not_claim_regsub_subspec_hints() {
        let dynamic = type_labels("regsub $mode {a} $value {\\1}\n");
        assert!(
            !dynamic.iter().any(|(_, label)| label == "grp1"),
            "a dynamic leading word can shift regsub's replacement: {dynamic:?}"
        );

        for src in [
            "regsub -c {a} $value {\\1}\n",
            "regsub -command {a} $value callback\n",
        ] {
            let labels = type_labels(src);
            assert!(
                !labels.iter().any(|(_, label)| label == "grp1"),
                "{src:?}: only a valid replacement template has capture hints: {labels:?}"
            );
        }

        let fixed = type_labels("regsub -start $start {a} $value {\\1}\n");
        assert!(
            fixed.iter().any(|(_, label)| label == "grp1"),
            "a declared -start value keeps the replacement layout fixed: {fixed:?}"
        );
    }

    /// Issue #1185: format hints follow the head's *effective command
    /// identity*, so a proven `interp alias` / `rename` of a format-family
    /// command hints like the original — and a spelling whose binding was
    /// taken over hints not at all.
    ///
    /// tclsh-proof (9.0.4 / 8.6.16): `interp alias {} myfmt {} format;
    /// myfmt "%d-%s" 1 two` -> `1-two`; after `proc format {args} {return
    /// USER}`, `format "%d"` -> `USER`.
    #[test]
    fn format_hints_follow_the_effective_command_identity() {
        let names_of =
            |src: &str| -> Vec<String> { type_labels(src).into_iter().map(|(_, l)| l).collect() };
        // TP — an aliased and a renamed head both hint like the original.
        for src in [
            "interp alias {} myfmt {} format\nmyfmt \"%d-%s\" 1 two\n",
            "rename format myfmt\nmyfmt \"%d-%s\" 1 two\n",
        ] {
            let names = names_of(src);
            assert!(
                names.iter().any(|n| n == "int") && names.iter().any(|n| n == "str"),
                "`{}` must hint like a direct format call, got {names:?}",
                src.lines().next().unwrap_or_default()
            );
        }
        // FP — a top-level `proc` that shadows the built-in takes the name
        // over, so its `%d` is an ordinary string.
        assert!(
            names_of("proc format {args} { return USER }\nformat \"%d-%s\" 1 two\n").is_empty(),
            "a shadowed `format` must not be hinted"
        );
        // TN — an unprovable binding states nothing, so the name stays an
        // ordinary unknown command.
        assert!(
            names_of("interp alias {} myfmt {} $target\nmyfmt \"%d-%s\" 1 two\n").is_empty(),
            "a dynamic alias target must not confer format hints"
        );
    }
}
