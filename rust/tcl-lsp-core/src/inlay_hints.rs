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

//! Inlay hints under the document's retained source and analysis input.
//!
//! Parameter hints label original call-site arguments (`name:`). Type hints
//! annotate inferred variable types (`: int`) and format conversions (`int`).
//! Each family has an independent enable flag.
//!
//! Complete source-image/configuration checks precede advice. Parameter
//! positions come from source structure and original operand mappings;
//! captured values do not supply written source anchors. Procedure prototypes
//! and selected registry synopsis metadata supply parameter names.
//!
//! Type hints use a checked [`CompilationUnit`]. Exact original SSA
//! definitions must match declaration inputs/spans; the explicitly admitted
//! Logical branch selects each written definition's actual body, symbol and version.
//! Unknown or overdefined types do not produce annotations.
//!
//! Method-call parameter hints inside class bodies are not provided.

use crate::format_template::{FormatOperand, FormatTemplate};
use crate::original_invocation::schema_format_arguments;
use tcl_compiler::analyser::{AnalysisResult, Scope, ScopeKind};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::ir::ProcedureId;
use tcl_compiler::types::{TypeKind, TypeLattice};
use tcl_lexer::LineIndex;
use tcl_registry::{CommandRegistry, TclType};

use crate::definition::LspRange;
use crate::original_invocation::{
    OriginalRegistryWords as OriginalHintWords, source_registry_words,
};

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
/// `analysis` supplies the retained source/configuration and declaration facts;
/// missing analysis returns an empty vector. The retained registry supplies
/// selected built-in metadata and the checked [`CompilationUnit`] for type
/// hints. `registry` is a compatibility fallback under admitted lexical advice.
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
/// lives in another file changes which parameter names are correct here.
/// Hinting the shadowed local proc's parameters over a
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
    if !type_hints && !parameter_hints {
        return Vec::new();
    }
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return Vec::new();
    }
    let lexical = analysis.allows_lexical_declaration_advice();
    let Some(dialect) = analysis
        .resolved_profile()
        .or_else(|| lexical.then_some(dialect))
    else {
        return Vec::new();
    };
    let registry = analysis
        .resolved_registry()
        .or_else(|| lexical.then_some(resolution.registry).flatten());
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();

    if type_hints && let Some(registry) = registry {
        let Some(cu) =
            crate::original_invocation::source_compilation_unit(source, analysis, registry)
        else {
            return out;
        };
        collect_type_hints(
            source,
            analysis,
            registry,
            &cu,
            range,
            &line_index,
            &mut out,
        );
        // Format-string specifier labels are registry-driven too (which
        // words carry a conversion string, and in which mini-language), so
        // they need the registry the same way the type hints do.
        collect_format_string_hints(
            source,
            analysis,
            FormatHintContext {
                dialect,
                registry,
                unit: &cu,
            },
            range,
            &line_index,
            &mut out,
        );
    }

    if parameter_hints {
        collect_parameter_hints(
            source,
            analysis,
            resolution,
            config,
            range,
            &line_index,
            &mut out,
        );
    }

    out
}

fn collect_parameter_hints(
    source: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    config: tcl_lexer::LexerConfig,
    range: LspRange,
    line_index: &LineIndex,
    out: &mut Vec<InlayHint>,
) {
    // The segmenter has no sound way to start mid-file (a command
    // boundary requires tracking brace/bracket nesting from the top), so
    // it still walks the whole document. What a requested viewport buys
    // is skipping original procedure/argument selection and the registry
    // synopsis query for a command
    // that falls entirely outside the range: cheap span arithmetic on
    // data the segmenter already produced, applied before any of that
    // work runs, not after (the `position_within_range` filter inside
    // original procedure/builtin hint emitters only ever trimmed the
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
    let Some(structure) =
        crate::source_structure::SourceStructure::capture(source, Some(analysis), config)
    else {
        return;
    };
    let segments = structure.commands;
    for seg in &segments {
        if seg.texts.is_empty() || seg.argv.is_empty() {
            continue;
        }
        if seg.span.end() <= range_start_off || seg.span.start() >= range_end_off {
            continue;
        }
        if crate::definition::program_forced_import_abstains(
            analysis,
            resolution.program,
            seg.argv[0].span.start(),
        ) {
            continue;
        }
        if !analysis.allows_lexical_declaration_advice() {
            if let Some(prototype) = tcl_compiler::registry_invocation::source_structure::original_procedure_arguments(source, analysis, seg) {
                emit_original_procedure_hints(source, &prototype, line_index, range, out);
                continue;
            }
        } else if let Some(prototype) = tcl_compiler::registry_invocation::source_structure::original_logical_procedure_arguments(source, analysis, seg) {
            emit_original_logical_procedure_hints(source, &prototype, line_index, range, out);
            continue;
        }
        // Built-in command — parse the registry synopsis for
        // positional parameter names.  User procs take
        // precedence (handled above).
        if let Some(selected) = source_registry_words(source, analysis, seg)
            && let Some(input) = analysis.resolved_input.as_ref()
        {
            emit_original_builtin_hints(
                source,
                &selected,
                &input.context_registry(),
                line_index,
                range,
                out,
            );
        }
    }
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

/// Collect readonly type hints from current physical or conditional source definitions.
/// Each projection retains its own original operands and body ownership.
fn collect_type_hints(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    cu: &CompilationUnit,
    range: LspRange,
    line_index: &LineIndex,
    out: &mut Vec<InlayHint>,
) {
    let Some((image, config)) = tcl_compiler::source_graph::current_analysis(source, analysis)
    else {
        return;
    };
    if cu.ir_module.source != image || cu.ir_module.lexer_config != config {
        return;
    }
    if !analysis.allows_lexical_declaration_advice() {
        collect_original_type_hints(source, analysis, registry, cu, range, line_index, out);
        return;
    }

    if analysis.resolved_input.as_ref() != cu.ir_module.source_metadata_input.as_ref() {
        return;
    }
    walk_scope_type_hints(
        &analysis.global_scope,
        analysis,
        cu,
        registry,
        cu.function_for_body(None),
        range,
        source,
        line_index,
        out,
        0,
    );
}

fn collect_original_type_hints(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    cu: &CompilationUnit,
    range: LspRange,
    line_index: &LineIndex,
    out: &mut Vec<InlayHint>,
) {
    let mut hints = std::collections::BTreeMap::new();
    for function in cu.all_body_function_units() {
        let metadata = function.invocation_metadata_context_for_module(registry, &cu.ir_module);
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let view =
                    tcl_compiler::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
                for (&symbol, &version) in &statement.defs {
                    let definition = view
                        .original_definition_name_with_metadata_context(symbol, registry, metadata)
                        .and_then(|definition| {
                            let span = function.abs_span(definition.span());
                            analysis
                                .original_variable_symbols
                                .iter()
                                .any(|occurrence| {
                                    occurrence.is_declaration()
                                        && occurrence.span() == span
                                        && occurrence.original_name_input()
                                            == definition.original_name_input()
                                })
                                .then(|| (span, definition.original_name_input().clone()))
                        })
                        .or_else(|| {
                            if analysis.resolved_input.as_ref()
                                != cu.ir_module.source_metadata_input.as_ref()
                            {
                                return None;
                            }
                            let definition = view.original_source_definition_for_body(
                                symbol,
                                cu,
                                function.body_id?,
                                registry,
                            )?;
                            Some((definition.span(), definition.original_name_input().clone()))
                        });
                    let Some((span, input)) = definition else {
                        continue;
                    };
                    let display = function
                        .types
                        .get(&(symbol, version))
                        .and_then(type_display);
                    let value = (input, display);
                    hints
                        .entry((span.start(), span.end()))
                        .and_modify(|previous: &mut (_, Option<String>)| {
                            if previous != &value {
                                previous.1 = None;
                            }
                        })
                        .or_insert(value);
                }
            }
        }
    }
    for ((span_start, span_end), (_, display)) in hints {
        let Some(display) = display else {
            continue;
        };
        let start = line_index.position_at_utf16(span_start, source);
        let end = line_index.position_at_utf16(span_end, source);
        if end.line < range.start_line || start.line > range.end_line {
            continue;
        }
        out.push(InlayHint {
            position_line: end.line,
            position_character: end.character.get(),
            label: format!(": {display}"),
            kind: InlayHintKind::Type,
            padding_left: true,
        });
    }
}

/// Match an original scope body to one actual declaration and issued body.
fn scope_body_id(
    source: &str,
    analysis: &AnalysisResult,
    cu: &CompilationUnit,
    scope: &Scope,
) -> Option<ProcedureId> {
    if scope.kind != ScopeKind::Proc {
        return None;
    }
    let body_span = scope.body_span?;
    let mut bodies = analysis
        .original_procedure_declarations()
        .filter(|declaration| declaration.metadata().body_span == body_span)
        .filter_map(|declaration| {
            tcl_compiler::source_graph::procedure_body_for_declaration(
                source,
                analysis,
                &cu.ir_module,
                declaration,
            )
            .map(|(body_id, _)| body_id)
        });
    let selected = bodies.next()?;
    bodies.next().is_none().then_some(selected)
}

/// Project each declaration's exact represented version in its original body.
#[allow(clippy::too_many_arguments)]
fn walk_scope_type_hints(
    scope: &Scope,
    analysis: &AnalysisResult,
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    function: Option<&tcl_compiler::compilation_unit::FunctionUnit>,
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
        let Some(function) = function else {
            continue;
        };
        let Some(definition) = tcl_compiler::ssa::original_analysis_definition_at_unit(
            cu,
            var_def.definition_span,
            registry,
        ) else {
            continue;
        };
        if !std::ptr::eq(definition.function(), function) {
            continue;
        }
        let Some(type_str) = definition
            .function()
            .types
            .get(&(definition.symbol(), definition.version()))
            .and_then(type_display)
        else {
            continue;
        };
        let start = line_index.position_at_utf16(definition.span().start(), source);
        let end = line_index.position_at_utf16(definition.span().end(), source);
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
    for child in &scope.children {
        let child_function = if matches!(child.kind, ScopeKind::Proc | ScopeKind::Method) {
            scope_body_id(source, analysis, cu, child)
                .and_then(|body| cu.function_for_body(Some(body)))
        } else {
            function
        };
        walk_scope_type_hints(
            child,
            analysis,
            cu,
            registry,
            child_function,
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

fn format_args(
    source: &str,
    analysis: &AnalysisResult,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(FormatOperand, tcl_registry::FormatStringArg)> {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let Some(selected) = source_registry_words(source, analysis, seg) else {
        return Vec::new();
    };
    let Some(input) = analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let context = input.context_registry();
    let Some(formats) = selected
        .with_source_schema(&context, schema_format_arguments)
        .flatten()
    else {
        return Vec::new();
    };
    formats
        .into_iter()
        .filter_map(|format| {
            Some((
                FormatOperand::capture(&selected, input, format.index)?,
                format,
            ))
        })
        .collect()
}

/// Push a `Type`-kind specifier hint at the byte offset `abs_byte`
/// (converted to a UTF-16 position) when it lies within `range`.
fn push_format_hint(
    label: String,
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
        label,
        kind: InlayHintKind::Type,
        padding_left: true,
    });
}

#[derive(Clone, Copy)]
struct FormatHintContext<'a> {
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &'a CommandRegistry,
    unit: &'a CompilationUnit,
}

/// Collect format-string specifier hints for the whole document. `cu` is the
/// same [`CompilationUnit`] the caller built for the type hints: a
/// format word with no literal content of its own falls back to the
/// lattice's proven value for it, read off `cu`.
fn collect_format_string_hints(
    source: &str,
    analysis: &AnalysisResult,
    context: FormatHintContext<'_>,
    range: LspRange,
    line_index: &LineIndex,
    out: &mut Vec<InlayHint>,
) {
    let FormatHintContext {
        dialect,
        registry,
        unit: cu,
    } = context;
    let profile = dialect;
    let Some(config) = analysis.body_lexer_config else {
        return;
    };
    let Some(structure) =
        crate::source_structure::SourceStructure::capture(source, Some(analysis), config)
    else {
        return;
    };
    let segments = structure.commands;
    for seg in &segments {
        for (operand, format) in format_args(source, analysis, seg) {
            let Some(template) = FormatTemplate::from_operand(source, &operand) else {
                let Some(word) = operand.original.word.as_ref() else {
                    continue;
                };
                let Some(computed_contents) =
                    crate::original_invocation::known_word_contents(cu, registry, word.span())
                else {
                    continue;
                };
                for (_, label) in format_specifier_labels(&computed_contents, format, profile) {
                    // Computed contents have no per-character source mapping.
                    push_format_hint(
                        label,
                        word.span().end() as usize,
                        range,
                        source,
                        line_index,
                        out,
                    );
                }
                continue;
            };
            for (offset, label) in format_specifier_labels(&template.content, format, profile) {
                if let Some(original) = template.original_offset(offset) {
                    push_format_hint(label, original, range, source, line_index, out);
                }
            }
        }
    }
}

fn format_specifier_labels(
    content: &str,
    format: tcl_registry::FormatStringArg,
    profile: &tcl_dialect::DialectProfile,
) -> Vec<(usize, String)> {
    let mut labels = Vec::new();
    match format.kind {
        tcl_registry::FormatType::Sprintf if format.scan => {
            for field in tcl_syntax::scan::conversion_spans(content, profile) {
                let label = match field.spec.verb {
                    'n' => Some("count"),
                    '[' => Some("chars"),
                    verb => sprintf_short(verb),
                };
                if let Some(label) = label {
                    labels.push((field.conversion.end, label.to_owned()));
                }
            }
        }
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
                    labels.push((end, label.to_owned()));
                }
                i = end;
            }
        }
        tcl_registry::FormatType::Clock => {
            for spec in tcl_cmd_core::clock::specifiers(content) {
                if let Some(label) = clock_short(char::from(spec.letter)) {
                    labels.push((spec.end, label.to_owned()));
                }
            }
        }
        tcl_registry::FormatType::Binary => {
            for spec in tcl_cmd_core::binary::specifiers(
                content.as_bytes(),
                tcl_cmd_core::binary::signedness_available(profile),
            ) {
                if let Some(label) = binary_short(char::from(spec.letter)) {
                    labels.push((spec.end, label.to_owned()));
                }
            }
        }
        tcl_registry::FormatType::Regsub => {
            for (span, part) in tcl_cmd_core::regex::replacement_parts(content.as_bytes()) {
                if let tcl_cmd_core::regex::ReplacementPart::Capture(group) = part {
                    let label = if group == 0 {
                        "match".to_owned()
                    } else {
                        format!("grp{group}")
                    };
                    labels.push((span.end, label));
                }
            }
        }
    }
    labels
}

fn emit_original_builtin_hints(
    source: &str,
    selected: &OriginalHintWords,
    context: &tcl_registry::model::ContextRegistry,
    line_index: &LineIndex,
    range: LspRange,
    out: &mut Vec<InlayHint>,
) {
    let Some((synopsis, skip_words, positions)) = selected
        .with_source_schema(context, |schema| {
            let selected = schema.authored_source_descriptors();
            let synopsis = match selected.subcommand {
                Some(sub) => sub.synopsis,
                None if selected.command.subcommands.is_empty() => {
                    *selected.command.hover.as_ref()?.synopsis.first()?
                }
                None => return None,
            };
            let arguments = schema.words.arguments();
            let first = schema.semantics.argument_offset;
            let options = schema
                .semantics
                .options
                .prefix_occurrences(arguments.slice_from(first))?;
            let positions = (first..arguments.exact_argv_len()?)
                .filter(|&ordinal| {
                    !options.iter().any(|option| {
                        (option.argument_index..option.values.end).contains(&(ordinal - first))
                    })
                })
                .collect::<Vec<_>>();
            Some((synopsis, first + 1, positions))
        })
        .flatten()
    else {
        return;
    };
    let params = param_names_from_synopsis(synopsis, skip_words);
    let names = select_param_names(&params, positions.len());
    for (&ordinal, name) in positions.iter().zip(names) {
        let Some(operand) = selected.operands.get(ordinal).and_then(Option::as_ref) else {
            continue;
        };
        let pos = line_index.position_at_utf16(operand.span.start(), source);
        if position_within_range(pos.line, pos.character.get(), range) {
            out.push(InlayHint {
                position_line: pos.line,
                position_character: pos.character.get(),
                label: format!("{name}:"),
                kind: InlayHintKind::Parameter,
                padding_left: false,
            });
        }
    }
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
/// `emit_original_builtin_hints`).
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

/// Readonly Logical labels use the selected allocation's exact original formal
/// topology. Captured values occupy effective slots but have no written anchor.
fn emit_original_logical_procedure_hints(
    source: &str,
    prototype: &tcl_compiler::registry_invocation::source_structure::OriginalLogicalProcedureArguments<'_>,
    line_index: &LineIndex,
    range: LspRange,
    out: &mut Vec<InlayHint>,
) {
    use tcl_syntax::formal_params::FormalByteArgumentBinding;
    for binding in prototype.bindings() {
        let (parameter, argument) = match binding {
            FormalByteArgumentBinding::Value {
                parameter,
                argument,
            }
            | FormalByteArgumentBinding::CallerLink {
                parameter,
                argument,
                ..
            } => (*parameter, *argument),
            FormalByteArgumentBinding::Default { .. } | FormalByteArgumentBinding::Rest { .. } => {
                continue;
            }
        };
        // This is display data from the same current original declaration card,
        // never a reporting-name lookup or an inverse to declaration identity.
        let Some(parameter) = prototype.declaration().metadata().params.get(parameter) else {
            continue;
        };
        let Some(operand) = prototype.operands().get(argument).and_then(Option::as_ref) else {
            continue;
        };
        let position = line_index.position_at_utf16(operand.span().start(), source);
        if position_within_range(position.line, position.character.get(), range) {
            out.push(InlayHint {
                position_line: position.line,
                position_character: position.character.get(),
                label: format!("{}:", parameter.name),
                kind: InlayHintKind::Parameter,
                padding_left: false,
            });
        }
    }
}

/// Labels and positions come from the same original formal/argv topology.
/// Defaults/rest aggregates have no individual supplied-field hint; captured
/// operands keep a parameter slot without borrowing a written source anchor.
fn emit_original_procedure_hints(
    source: &str,
    prototype: &tcl_compiler::registry_invocation::source_structure::OriginalProcedureArguments<'_>,
    line_index: &LineIndex,
    range: LspRange,
    out: &mut Vec<InlayHint>,
) {
    use tcl_syntax::formal_params::FormalByteArgumentBinding;
    for binding in prototype.bindings() {
        let (parameter, argument) = match binding {
            FormalByteArgumentBinding::Value {
                parameter,
                argument,
            }
            | FormalByteArgumentBinding::CallerLink {
                parameter,
                argument,
                ..
            } => (*parameter, *argument),
            FormalByteArgumentBinding::Default { .. } | FormalByteArgumentBinding::Rest { .. } => {
                continue;
            }
        };
        let Some(field) = prototype.formals().name_field(parameter) else {
            continue;
        };
        let Some(operand) = prototype.operands().get(argument).and_then(Option::as_ref) else {
            continue;
        };
        let pos = line_index.position_at_utf16(operand.span().start(), source);
        if !position_within_range(pos.line, pos.character.get(), range) {
            continue;
        }
        out.push(InlayHint {
            position_line: pos.line,
            position_character: pos.character.get(),
            label: format!(
                "{}:",
                tcl_syntax::native_string::resident_name_label(field.original_input().bytes())
            ),
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
mod tests;

#[cfg(test)]
mod original_hint_tests;

#[cfg(test)]
mod original_variable_type_tests;

#[cfg(test)]
mod original_procedure_topology_hint_tests;

#[cfg(test)]
mod retained_hint_context_tests;
