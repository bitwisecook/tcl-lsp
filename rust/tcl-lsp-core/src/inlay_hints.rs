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

use tcl_compiler::analyser::{AnalysisResult, Scope, ScopeKind};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::ir::ProcedureId;
use tcl_compiler::types::{TypeKind, TypeLattice};
use tcl_lexer::LineIndex;
use tcl_registry::{CommandRegistry, TclType};

use crate::definition::LspRange;
use crate::original_invocation::{
    OriginalOperandSource as HintOperand, OriginalRegistryWords as OriginalHintWords,
    source_registry_words,
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
            return out;
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
                    emit_original_procedure_hints(source, &prototype, &line_index, range, &mut out);
                    continue;
                }
            } else if let Some(prototype) = tcl_compiler::registry_invocation::source_structure::original_logical_procedure_arguments(source, analysis, seg) {
                emit_original_logical_procedure_hints(source, &prototype, &line_index, range, &mut out);
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
                    &line_index,
                    range,
                    &mut out,
                );
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
        let mut hints = std::collections::BTreeMap::new();
        for function in cu.all_body_function_units() {
            let metadata = function.invocation_metadata_context_for_module(registry, &cu.ir_module);
            for (&block, body) in &function.ssa.blocks {
                for (index, statement) in body.statements.iter().enumerate() {
                    let view =
                        tcl_compiler::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
                    for (&symbol, &version) in &statement.defs {
                        let definition = view
                            .original_definition_name_with_metadata_context(
                                symbol, registry, metadata,
                            )
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

/// Registry format operands retain effective ordinals. An alias prefix or list
/// expansion never shifts a written template onto another argument's source.
fn format_args(
    source: &str,
    analysis: &AnalysisResult,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
) -> Vec<(FormatHintOperand, tcl_registry::FormatType)> {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let Some(selected) = source_registry_words(source, analysis, seg) else {
        return Vec::new();
    };
    let Some(input) = analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let context = input.context_registry();
    // The closure is required: the method path is not general enough
    // over the schema borrow's higher-ranked lifetime.
    #[allow(clippy::redundant_closure_for_method_calls)]
    let Some(formats) = selected
        .with_source_schema(&context, |schema| schema.authored_source_format_arguments())
        .flatten()
    else {
        return Vec::new();
    };
    formats
        .into_iter()
        .filter_map(|format| {
            let operand = selected.operands.get(format.index)?.as_ref()?;
            let actual = selected.arguments.get(format.index)?.literal_bytes()?;
            let logical = if let Some(native) = operand.input.as_ref() {
                (native.bytes() == actual).then_some(())?;
                None
            } else {
                let crate::original_invocation::OriginalRegistrySource::SourceTransitions(advice) =
                    &selected.source
                else {
                    return None;
                };
                (advice.logical_source_input() == Some(input)).then_some(())?;
                let word = operand.word.as_ref()?;
                (tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?.as_slice()
                    == actual)
                    .then_some(())?;
                Some(std::sync::Arc::clone(advice))
            };
            Some((
                FormatHintOperand {
                    original: operand.clone(),
                    logical,
                },
                format.kind,
            ))
        })
        .collect()
}

struct FormatHintOperand {
    original: HintOperand,
    logical: Option<
        std::sync::Arc<tcl_compiler::command_binding::OriginalSourceCommandTransitionAdvice>,
    >,
}

enum FormatTemplateMapping {
    Native {
        channel: tcl_lexer::SourceChannel,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        escapes: Option<tcl_dialect::EscapeSyntax>,
    },
    Logical(Box<tcl_lexer::NativeWord>),
}

struct FormatTemplate<'a> {
    content: std::borrow::Cow<'a, str>,
    raw: &'a [u8],
    span: tcl_lexer::Span,
    mapping: FormatTemplateMapping,
}

impl<'a> FormatTemplate<'a> {
    fn from_operand(source: &'a str, operand: &FormatHintOperand) -> Option<Self> {
        let original = &operand.original;
        let Some(input) = &original.input else {
            let logical = operand.logical.as_ref()?.logical_source_input()?;
            let word = original.word.as_ref()?;
            if word.image() != &tcl_lexer::SourceImage::document(source)
                || word.config() != logical.lexer_config()
            {
                return None;
            }
            let value = tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?;
            return Some(Self {
                content: std::borrow::Cow::Owned(String::from_utf8(value).ok()?),
                raw: source.as_bytes().get(word.span().as_range())?,
                span: word.span(),
                mapping: FormatTemplateMapping::Logical(Box::new(word.clone())),
            });
        };
        let protocol = input.policy().string_protocol();
        let (span, channel, escapes) = if let Some(key) = input.original_word_key() {
            let word = key.original_word();
            let escapes =
                (word.group().kind != tcl_lexer::WordKind::Braced).then_some(word.config().escapes);
            (word.content_span().ok()?, word.image().channel(), escapes)
        } else {
            // Readonly list children can have exact literal extents. Produced
            // values without such an extent cannot supply a template coordinate.
            input.original_static_list_container()?;
            (original.span, tcl_lexer::SourceChannel::Document, None)
        };
        let raw = source.as_bytes().get(span.as_range())?;
        if escapes.is_none()
            && tcl_syntax::backslash::native_source_literal_bytes(raw, channel, protocol)
                .ok()?
                .as_ref()
                != input.bytes()
        {
            return None;
        }
        Some(Self {
            content: std::borrow::Cow::Owned(std::str::from_utf8(input.bytes()).ok()?.to_owned()),
            raw,
            span,
            mapping: FormatTemplateMapping::Native {
                channel,
                protocol,
                escapes,
            },
        })
    }

    fn original_offset(&self, offset: usize) -> Option<usize> {
        let (channel, protocol, escapes) = match &self.mapping {
            FormatTemplateMapping::Native {
                channel,
                protocol,
                escapes,
            } => (channel, protocol, escapes),
            FormatTemplateMapping::Logical(word) => {
                return usize::try_from(
                    tcl_syntax::word_rules::original_static_word_ascii_source_offset(word, offset)?,
                )
                .ok();
            }
        };
        let end = if let Some(escapes) = escapes {
            tcl_syntax::backslash::native_source_string_extent(
                self.raw,
                *channel,
                *escapes,
                *protocol,
                0..offset,
            )?
            .end
        } else {
            tcl_syntax::backslash::native_source_literal_extent(
                self.raw,
                *channel,
                *protocol,
                0..offset,
            )?
            .end
        };
        (self.span.start() as usize).checked_add(end)
    }
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
        for (operand, kind) in format_args(source, analysis, seg) {
            let Some(template) = FormatTemplate::from_operand(source, &operand) else {
                let Some(word) = operand.original.word.as_ref() else {
                    continue;
                };
                let Some(computed_contents) =
                    crate::original_invocation::known_word_contents(cu, registry, word.span())
                else {
                    continue;
                };
                for (_, label) in format_specifier_labels(&computed_contents, kind, profile) {
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
            for (offset, label) in format_specifier_labels(&template.content, kind, profile) {
                if let Some(original) = template.original_offset(offset) {
                    push_format_hint(label, original, range, source, line_index, out);
                }
            }
        }
    }
}

fn format_specifier_labels(
    content: &str,
    kind: tcl_registry::FormatType,
    profile: &tcl_dialect::DialectProfile,
) -> Vec<(usize, &'static str)> {
    let mut labels = Vec::new();
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
                    labels.push((end, label));
                }
                i = end;
            }
        }
        tcl_registry::FormatType::Clock => {
            for spec in tcl_cmd_core::clock::specifiers(content) {
                if let Some(label) = clock_short(char::from(spec.letter)) {
                    labels.push((spec.end, label));
                }
            }
        }
        tcl_registry::FormatType::Binary => {
            for spec in tcl_cmd_core::binary::specifiers(
                content.as_bytes(),
                tcl_cmd_core::binary::signedness_available(profile),
            ) {
                if let Some(label) = binary_short(char::from(spec.letter)) {
                    labels.push((spec.end, label));
                }
            }
        }
        tcl_registry::FormatType::Regsub => {
            for capture in REGSUB_RE.captures_iter(content) {
                let whole = capture.get(0).expect("group 0");
                let ch = capture
                    .get(1)
                    .and_then(|group| group.as_str().chars().next())
                    .unwrap_or(' ');
                if let Some(label) = regsub_short(ch) {
                    labels.push((whole.end(), label));
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
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    pub(super) fn whole_document_range(source: &str) -> LspRange {
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
        let hints = inlay_hints(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            range,
            Some(&analysis),
            None,
            false,
            true,
        );
        assert_eq!(hints.len(), 1, "{hints:?}");
        assert_eq!(hints[0].position_line, 2);
    }

    /// A command whose span falls entirely outside the
    /// requested range must never reach original procedure selection or the registry
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
        let full_hints = inlay_hints(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            full,
            Some(&analysis),
            Some(&reg),
            false,
            true,
        );
        let narrow_hints = inlay_hints(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            narrow,
            Some(&analysis),
            Some(&reg),
            false,
            true,
        );
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

    /// `walk_scope_type_hints` recurses once per nested namespace/proc
    /// scope, capped by `MAX_SCOPE_WALK_DEPTH` (`crate::lib`). 80 nested
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
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
        type_labels_for_dialect(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        )
    }

    fn type_labels_for_dialect(
        src: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> Vec<(u32, String)> {
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
    fn inlay_hints_label_a_computed_format_string() {
        // A literal `format "%d" 1` already labels its specifier;
        // the lattice proves the same text when it reaches the word
        // through a variable, so the computed spelling must not go blind
        // (there is no in-place span of its own, so the label anchors at
        // the `$fmt` token's own end instead of a content offset).
        let labels = type_labels("set fmt \"%d\"\nformat $fmt 1\n");
        let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
        assert!(names.contains(&"int"), "{labels:?}");

        // Negative: an unproven `$fmt` (a bare parameter, never assigned a
        // literal) proves nothing, so no hint is fabricated.
        let labels = type_labels("proc p {fmt} {\n    format $fmt 1\n}\n");
        assert!(labels.is_empty(), "{labels:?}");
    }

    #[test]
    fn binary_q_and_q_hints_are_owned_by_shared_spec_table() {
        let labels = type_labels_for_dialect(
            "binary format \"q Q\" 1 2\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
        assert!(names.contains(&"f64le"), "{labels:?}");
        assert!(names.contains(&"f64be"), "{labels:?}");
    }

    // `ss` is two short-integer fields on every release (tclsh 8.4.20 through
    // 9.0.4 all pack four bytes), so it earns two hints — the shared owner
    // used to swallow the second `s` as a signedness modifier and emit one.
    #[test]
    fn short_specifier_pair_gets_a_hint_each() {
        let labels = type_labels_for_dialect(
            "binary format ss 1 2\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(
            names.iter().filter(|n| **n == "i16le").count(),
            2,
            "{labels:?}"
        );
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

    /// Format hints follow the head's *effective command
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

#[cfg(test)]
mod original_hint_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn parameter_hints(source: &str, analysis: &AnalysisResult) -> Vec<InlayHint> {
        parameter_hints_from(source, analysis, 0)
    }

    fn parameter_hints_from(
        source: &str,
        analysis: &AnalysisResult,
        start: usize,
    ) -> Vec<InlayHint> {
        assert!(source.is_ascii() && !source.contains('\n'));
        inlay_hints(
            source,
            crate::profile_for_dialect("tcl8.6"),
            LspRange {
                start_line: 0,
                start_character: u32::try_from(start).unwrap(),
                end_line: 100,
                end_character: 0,
            },
            Some(analysis),
            None,
            false,
            true,
        )
    }

    fn builtin_hints(source: &str, analysis: &AnalysisResult) -> Vec<InlayHint> {
        inlay_hints(
            source,
            crate::profile_for_dialect("tcl8.6"),
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 100,
                end_character: 0,
            },
            Some(analysis),
            Some(crate::registry_for_dialect("tcl8.6")),
            false,
            true,
        )
    }

    #[test]
    fn original_parameter_hints_preserve_opaque_call_identity_and_source_currency() {
        let source = "proc p\\uD800 {first} {}; proc p\\uD801 {second} {}; p\\uD800 1; p\\uD801 2";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        for invocation in &mut analysis.command_invocations {
            invocation.name = "counterfactual".to_owned();
        }
        // The requested viewport includes both tested calls. Definition
        // operands outside it have their own valid builtin synopsis hints.
        let start = source.find("; p\\uD800 1").unwrap() + 2;
        let hints = parameter_hints_from(source, &analysis, start);
        assert_eq!(
            hints
                .iter()
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            ["first:", "second:"]
        );
        assert!(parameter_hints(&format!("#{source}"), &analysis).is_empty());
        for invocation in &mut analysis.command_invocations {
            invocation.original_lookup = None;
        }
        assert!(parameter_hints_from(source, &analysis, start).is_empty());
    }

    #[test]
    fn original_alias_parameter_hints_map_captured_prefix_before_written_arguments() {
        let source =
            "proc target {captured written} {}; interp alias {} alias {} target FIXED; alias VALUE";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        analysis.all_procs.clear();
        let start = source.rfind("; alias VALUE").unwrap() + 2;
        let hints = parameter_hints_from(source, &analysis, start);
        assert_eq!(
            hints
                .iter()
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            ["written:"]
        );
    }

    #[test]
    fn original_builtin_alias_and_expansion_hints_keep_effective_slots_and_original_extents() {
        // Implementation contract: naming.core.original-hint-effective-origins
        // docs/design/analysis/name-resolution-proofs/core-original-hint-effective-origins.md
        let source = "interp alias {} slice {} string range CAPTURED; slice 1 2";
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let hints = builtin_hints(source, &analysis);
        let start = source.rfind("slice 1 2").unwrap();
        let hints: Vec<_> = hints
            .iter()
            .filter(|hint| hint.position_character >= u32::try_from(start).unwrap())
            .collect();
        assert_eq!(
            hints
                .iter()
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            ["first:", "last:"]
        );
        assert_eq!(
            hints[0].position_character as usize,
            source.rfind("1 2").unwrap()
        );

        let source = "string range {*}{CAPTURED 1} 2";
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let hints = builtin_hints(source, &analysis);
        assert_eq!(
            hints
                .iter()
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            ["string:", "first:", "last:"]
        );
        assert_eq!(
            hints
                .iter()
                .map(|hint| hint.position_character as usize)
                .collect::<Vec<_>>(),
            [
                source.find("CAPTURED").unwrap(),
                source.find("1}").unwrap(),
                source.rfind('2').unwrap()
            ]
        );
        let source = "set prefix {CAPTURED 1}; string range {*}$prefix 2";
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let hints = builtin_hints(source, &analysis);
        let hints: Vec<_> = hints
            .iter()
            .filter(|hint| hint.position_character as usize >= source.find("string range").unwrap())
            .collect();
        assert_eq!(
            hints
                .iter()
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            ["last:"]
        );
        let source = "string range {*}$unknown 2";
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        assert!(builtin_hints(source, &analysis).is_empty());
    }

    #[test]
    fn original_format_alias_expansion_and_escape_hints_use_the_template_producer() {
        // Implementation contract: naming.core.original-format-hint-producer
        // docs/design/analysis/name-resolution-proofs/core-original-format-hint-producer.md
        let hints = |source: &str| {
            let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
            let registry = analysis.resolved_registry().unwrap();
            let unit =
                crate::original_invocation::source_compilation_unit(source, &analysis, registry)
                    .unwrap();
            let mut out = Vec::new();
            collect_format_string_hints(
                source,
                &analysis,
                FormatHintContext {
                    dialect: crate::profile_for_dialect("tcl8.6"),
                    registry,
                    unit: &unit,
                },
                LspRange {
                    start_line: 0,
                    start_character: 0,
                    end_line: 100,
                    end_character: 0,
                },
                &LineIndex::new(source),
                &mut out,
            );
            out
        };
        let source = r#"interp alias {} fmt {} format; fmt "%d" 3"#;
        let out = hints(source);
        assert_eq!(
            out.iter()
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            ["int"]
        );
        assert_eq!(
            out[0].position_character as usize,
            source.find("%d").unwrap() + 2
        );
        let source = "format {*}{%d 3}";
        let out = hints(source);
        assert_eq!(
            out.iter()
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            ["int"]
        );
        assert_eq!(
            out[0].position_character as usize,
            source.find("%d").unwrap() + 2
        );
        let source = r"format \x25d 3";
        let out = hints(source);
        assert_eq!(
            out.iter()
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            ["int"]
        );
        assert_eq!(
            out[0].position_character as usize,
            source.find(r"\x25d").unwrap() + 5
        );
        assert!(hints(r#"interp alias {} fmt {} format "%d"; fmt 3"#).is_empty());
        assert!(hints("format {*}$unknown 3").is_empty());
    }

    #[test]
    fn original_builtin_hints_use_retained_descriptor_and_do_not_borrow_shadowed_prototype() {
        // Implementation contract: naming.core.original-hint-effective-origins
        // docs/design/analysis/name-resolution-proofs/core-original-hint-effective-origins.md
        let source = "set value 1";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let hints = builtin_hints(source, &analysis);
        assert!(
            hints.iter().any(|hint| hint.label == "varName:"),
            "{hints:?}"
        );
        for invocation in &mut analysis.command_invocations {
            invocation.name = "puts".to_owned();
        }
        assert_eq!(builtin_hints(source, &analysis), hints);
        let shadow = "proc set {other} {}; set value";
        let analysis = Analyser::new().analyse(shadow, "tcl8.6").clone();
        assert_eq!(
            parameter_hints_from(shadow, &analysis, shadow.rfind("; set value").unwrap() + 2)
                .iter()
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            ["other:"]
        );
    }
}

#[cfg(test)]
mod original_variable_type_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn hints(source: &str, analysis: &AnalysisResult) -> Vec<InlayHint> {
        inlay_hints(
            source,
            crate::profile_for_dialect("tcl8.6"),
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 100,
                end_character: 0,
            },
            Some(analysis),
            Some(crate::registry_for_dialect("tcl8.6")),
            true,
            false,
        )
    }

    #[test]
    // Implementation contract: naming.variable.original-ssa-definition-operand
    // docs/design/analysis/name-resolution-proofs/original-ssa-definition-operand.md
    fn original_type_hints_match_exact_ssa_cells_without_reporting_variable_maps() {
        let source = "set v\\uD800 42\nset v\\uD801 TEXT\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        analysis.global_scope.variables.clear();
        analysis.all_procs.clear();
        let hints = hints(source, &analysis);
        assert!(
            hints
                .iter()
                .any(|hint| hint.position_line == 0 && hint.label == ": int"),
            "{hints:?}"
        );
        assert!(
            hints
                .iter()
                .any(|hint| hint.position_line == 1 && hint.label == ": str"),
            "{hints:?}"
        );
        assert!(hints.iter().all(
            |hint| (hint.position_line, hint.label.as_str()) != (0, ": str")
                && (hint.position_line, hint.label.as_str()) != (1, ": int")
        ));
        assert!(
            super::original_variable_type_tests::hints(
                &format!("{source}# changed elsewhere"),
                &analysis
            )
            .is_empty()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(super::original_variable_type_tests::hints(source, &missing).is_empty());
        analysis.original_variable_symbols.clear();
        assert!(super::original_variable_type_tests::hints(source, &analysis).is_empty());
    }

    #[test]
    fn original_type_hints_keep_local_frames_and_alias_targets_separate() {
        let source = "set shared 7\nproc first {} {set value 42; global shared; set shared 8}\nproc second {} {set value TEXT}\nfirst; second\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        fn clear(scope: &mut Scope) {
            scope.variables.clear();
            for child in &mut scope.children {
                clear(child);
            }
        }
        clear(&mut analysis.global_scope);
        analysis.all_procs.clear();
        let hints = hints(source, &analysis);
        assert!(
            hints
                .iter()
                .any(|hint| hint.position_line == 1 && hint.label == ": int"),
            "{hints:?}"
        );
        assert!(
            hints
                .iter()
                .any(|hint| hint.position_line == 2 && hint.label == ": str"),
            "{hints:?}"
        );
        assert!(
            hints
                .iter()
                .all(|hint| hint.position_line != 2 || hint.label != ": int")
        );
    }
}

#[cfg(test)]
mod original_procedure_topology_hint_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_procedure_hints_share_expanded_children_and_formal_binding_topology() {
        // naming.source.original-procedure-argument-topology
        // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
        for source in [
            "proc target {captured written} {}; target {*}{FIXED VALUE}",
            "proc target {captured written} {}; interp alias {} alias {} target FIXED; alias {*}{VALUE}",
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl8.6");
            analysis.all_procs.clear();
            analysis.global_scope.procs.clear();
            analysis.command_invocations.clear();
            let hints = inlay_hints(
                source,
                analysis.resolved_profile().unwrap(),
                LspRange {
                    start_line: 0,
                    start_character: 0,
                    end_line: 100,
                    end_character: 0,
                },
                Some(&analysis),
                None,
                false,
                true,
            );
            let hint = hints
                .iter()
                .find(|hint| hint.label == "written:")
                .expect("actual child hint");
            let expected = LineIndex::new(source).position_at_utf16(
                u32::try_from(source.rfind("VALUE").unwrap()).unwrap(),
                source,
            );
            assert_eq!(hint.position_line, expected.line);
            assert_eq!(hint.position_character, expected.character.get());
            if source.contains("interp alias") {
                assert!(!hints.iter().any(|hint| hint.label == "captured:"));
            }
        }
        let source = "proc target {first {second DEFAULT}} {}; target ONLY";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let hints = inlay_hints(
            source,
            analysis.resolved_profile().unwrap(),
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 100,
                end_character: 0,
            },
            Some(&analysis),
            None,
            false,
            true,
        );
        let written = LineIndex::new(source).position_at_utf16(
            u32::try_from(source.rfind("ONLY").unwrap()).unwrap(),
            source,
        );
        assert_eq!(
            hints
                .iter()
                .filter(|hint| hint.position_line == written.line
                    && hint.position_character == written.character.get())
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            ["first:"]
        );
        assert!(!hints.iter().any(|hint| hint.label == "second:"));
        assert!(
            inlay_hints(
                &format!("#{source}"),
                analysis.resolved_profile().unwrap(),
                LspRange {
                    start_line: 0,
                    start_character: 0,
                    end_line: 100,
                    end_character: 0,
                },
                Some(&analysis),
                None,
                false,
                true
            )
            .is_empty()
        );
    }
}

#[cfg(test)]
mod retained_hint_context_tests {
    use super::tests::whole_document_range;
    use super::*;
    use crate::original_invocation::selected_registry_words as selected_original_words;
    use tcl_compiler::analyser::Analyser;

    fn hints(
        source: &str,
        analysis: &AnalysisResult,
        profile: &'static tcl_dialect::DialectProfile,
        registry: &CommandRegistry,
    ) -> Vec<InlayHint> {
        inlay_hints_in_program(
            source,
            profile,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 100,
                end_character: 0,
            },
            Some(analysis),
            crate::definition::CallResolution {
                registry: Some(registry),
                program: None,
            },
            true,
            true,
        )
    }

    fn analysis_with_availability(source: &str, environment: &str) -> AnalysisResult {
        let profile = crate::profile_for_dialect("tcl9.1");
        let driver =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    fn logical_format_input(environment: &str) -> tcl_compiler::analyser::ResolvedAnalysisInput {
        let mut profile = crate::profile_for_dialect("f5-irules").clone();
        profile.name = "explicit-logical-format-hints";
        let profile = profile.intern();
        let driver =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        );
        let config = tcl_lexer::LexerConfig {
            escapes: crate::profile_for_dialect("tcl8.6").grammar.escapes,
            ..tcl_lexer::LexerConfig::for_profile(Some(profile))
        };
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config)
    }

    #[test]
    fn logical_format_hints_keep_actual_context_and_full_source_currency() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "clock scan 2020 -format {%Y}";
        for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
            let input = logical_format_input(environment);
            let context = input.context_registry();
            let mut analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            assert!(analysis.allows_lexical_declaration_advice());
            analysis.command_invocations.clear();
            let out = hints(source, &analysis, input.unit_profile(), context.commands());
            assert_eq!(
                out.iter().any(|hint| hint.label == "year"),
                expected,
                "{environment}: {out:?}"
            );
            assert!(
                hints(
                    &source.replace("%Y", "%m"),
                    &analysis,
                    input.unit_profile(),
                    context.commands()
                )
                .is_empty()
            );
            analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                std::sync::Arc::new(
                    context.with_command_store(context.commands().snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
            assert!(hints(source, &analysis, input.unit_profile(), context.commands()).is_empty());
        }
    }

    #[test]
    fn logical_format_hints_keep_nested_escaped_words_and_refuse_captured_anchors() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let input = logical_format_input("tcl9.1");
        let context = input.context_registry();
        for source in [
            "format {%d} 7",
            r"format \x25d 7",
            r#"format "\x25d" 7"#,
            "proc p {} {format {%d} 7}",
            "interp alias {} fmt {} format; fmt {%d} 7",
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            let out = hints(source, &analysis, input.unit_profile(), context.commands());
            let formats = out
                .iter()
                .filter(|hint| hint.label == "int")
                .collect::<Vec<_>>();
            assert_eq!(formats.len(), 1, "{source}: {out:?}");
            let end = source.find('d').unwrap() + 1;
            let expected =
                LineIndex::new(source).position_at_utf16(u32::try_from(end).unwrap(), source);
            assert_eq!(
                (formats[0].position_line, formats[0].position_character),
                (expected.line, expected.character.get()),
                "{source}"
            );
        }
        for source in [
            "interp alias {} fmt {} format {%d}; fmt 7",
            "format $template 7",
            "proc format args {}; format {%d} 7",
            "format {a\\\n%d} 7",
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            assert!(
                hints(source, &analysis, input.unit_profile(), context.commands())
                    .iter()
                    .all(|hint| hint.label != "int"),
                "{source}"
            );
        }
    }

    #[test]
    fn logical_format_escape_hints_use_the_complete_selected_grammar() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = r"format \x25d 7";
        let modern = logical_format_input("tcl9.1");
        let config = tcl_lexer::LexerConfig {
            escapes: crate::profile_for_dialect("tcl8.5").grammar.escapes,
            ..modern.lexer_config()
        };
        let legacy = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            modern.analyser_profile(),
            modern.unit_profile(),
            modern.context_registry(),
            config,
        );
        for (input, expected) in [(modern, true), (legacy, false)] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            assert_eq!(
                hints(
                    source,
                    &analysis,
                    input.unit_profile(),
                    input.context_registry().commands()
                )
                .iter()
                .any(|hint| hint.label == "int"),
                expected
            );
        }
    }

    #[test]
    fn logical_procedure_parameter_hints_use_original_allocation_and_effective_topology() {
        for (source, expected) in [
            (
                "proc target {first second} {}; target ONE TWO",
                vec!["first:", "second:"],
            ),
            (
                "proc target {captured written} {}; interp alias {} alias {} target FIXED; alias VALUE",
                vec!["written:"],
            ),
            (
                "proc target {first {second DEFAULT} args} {}; target ONE",
                vec!["first:"],
            ),
            (
                "proc target {first {second DEFAULT} args} {}; target ONE TWO THREE",
                vec!["first:", "second:"],
            ),
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl");
            analysis.all_procs.clear();
            analysis.global_scope.procs.clear();
            analysis.command_invocations.clear();
            let start = u32::try_from(source.rfind(';').unwrap() + 1).unwrap();
            let out = hints(
                source,
                &analysis,
                analysis.resolved_profile().unwrap(),
                analysis.resolved_registry().unwrap(),
            );
            let labels = out
                .iter()
                .filter(|hint| {
                    hint.kind == InlayHintKind::Parameter && hint.position_character >= start
                })
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>();
            assert_eq!(labels, expected, "{source}: {out:?}");
            if source.contains("alias VALUE") {
                let actual = out.iter().find(|hint| hint.label == "written:").unwrap();
                let expected = LineIndex::new(source).position_at_utf16(
                    u32::try_from(source.rfind("VALUE").unwrap()).unwrap(),
                    source,
                );
                assert_eq!(
                    (actual.position_line, actual.position_character),
                    (expected.line, expected.character.get())
                );
                assert!(!out.iter().any(|hint| hint.label == "captured:"));
            }
        }
    }

    #[test]
    fn logical_procedure_parameter_hints_keep_unknown_formals_and_source_currency_terminal() {
        for source in [
            "proc target {value} {}; unknown_mutation; target VALUE",
            "proc target {{optional DEFAULT} required} {}; target ONLY",
            "proc target {&value} {}; target VALUE",
            "proc target {args tail} {}; target FIRST SECOND",
            "proc target {value} {}; rename target {}; target VALUE",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let start = u32::try_from(source.rfind(';').unwrap() + 1).unwrap();
            let out = hints(
                source,
                &analysis,
                analysis.resolved_profile().unwrap(),
                analysis.resolved_registry().unwrap(),
            );
            assert!(
                out.iter()
                    .all(|hint| hint.kind != InlayHintKind::Parameter
                        || hint.position_character < start),
                "{source}: {out:?}"
            );
        }
        let source = "proc target {value} {}; target VALUE";
        let original = Analyser::new().analyse(source, "tcl");
        let profile = original.resolved_profile().unwrap();
        let registry = original.resolved_registry().unwrap();
        assert!(
            hints(source, &original, profile, registry)
                .iter()
                .any(|hint| hint.label == "value:")
        );
        assert!(
            hints(
                &source.replace("VALUE", "OTHER"),
                &original,
                profile,
                registry
            )
            .is_empty()
        );
        let mut changed = original.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        assert!(hints(source, &changed, profile, registry).is_empty());
        changed = original.clone();
        changed.resolved_input = None;
        assert!(hints(source, &changed, profile, registry).is_empty());
    }

    #[test]
    fn logical_parameter_hints_keep_actual_context_and_original_source_currency() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "string cat A B";
        for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
            let input = logical_format_input(environment);
            let context = input.context_registry();
            let mut analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            analysis.command_invocations.clear();
            let out = hints(source, &analysis, input.unit_profile(), context.commands());
            assert_eq!(
                out.iter().any(|hint| hint.kind == InlayHintKind::Parameter),
                expected,
                "{environment}: {out:?}"
            );
            assert!(
                hints(
                    "string cat C D",
                    &analysis,
                    input.unit_profile(),
                    context.commands()
                )
                .is_empty()
            );
            analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                std::sync::Arc::new(
                    context.with_command_store(context.commands().snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
            assert!(hints(source, &analysis, input.unit_profile(), context.commands()).is_empty());
        }
    }

    #[test]
    fn logical_parameter_hints_keep_nested_and_prefix_ordinals_without_captured_anchors() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let input = logical_format_input("tcl9.1");
        let context = input.context_registry();
        for (source, needle) in [
            ("string index held 1", "1"),
            ("proc p {} {string index held 1}", "1"),
            ("interp alias {} idx {} string index held; idx 1", "1"),
            (r"string \u0069ndex held 1", "1"),
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            let out = hints(source, &analysis, input.unit_profile(), context.commands());
            let actual = out
                .iter()
                .filter(|hint| hint.label == "charIndex:")
                .collect::<Vec<_>>();
            assert_eq!(actual.len(), 1, "{source}: {out:?}");
            let position = LineIndex::new(source).position_at_utf16(
                u32::try_from(source.rfind(needle).unwrap()).unwrap(),
                source,
            );
            assert_eq!(
                (actual[0].position_line, actual[0].position_character),
                (position.line, position.character.get())
            );
        }
        for source in [
            "interp alias {} len {} string length held; len",
            "proc string args {}; string index held 1",
            "string $selector held 1",
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "presentation-only");
            let offset = u32::try_from(source.rfind(';').map_or(0, |offset| offset + 1)).unwrap();
            let position = LineIndex::new(source).position_at_utf16(offset, source);
            assert!(
                hints(source, &analysis, input.unit_profile(), context.commands())
                    .iter()
                    .all(|hint| hint.kind != InlayHintKind::Parameter
                        || hint.position_line < position.line
                        || hint.position_line == position.line
                            && hint.position_character < position.character.get()),
                "{source}"
            );
        }
    }

    #[test]
    fn original_builtin_hints_use_the_selected_availability_not_registry_profile() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "string cat A B";
        let current = analysis_with_availability(source, "tcl9.1");
        let profile = current.resolved_profile().unwrap();
        let registry = current.resolved_registry().unwrap();
        let positive = hints(source, &current, profile, registry);
        let positions = positive
            .iter()
            .filter(|hint| hint.kind == InlayHintKind::Parameter)
            .map(|hint| hint.position_character)
            .collect::<Vec<_>>();
        // The synopsis names its first positional string; its variadic tail
        // contributes no additional parameter label.
        assert_eq!(positions, [11], "{positive:?}");
        assert!(positive.iter().any(|hint| hint.label == "string1:"));

        let older = analysis_with_availability(source, "tcl8.4");
        assert_eq!(
            older.resolved_registry().unwrap().profile().unwrap().name,
            "tcl9.1"
        );
        let segment = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            older.body_lexer_config.unwrap(),
        )
        .remove(0);
        let selected =
            selected_original_words(source, &older, &segment, older.resolved_registry().unwrap())
                .expect("same genuine Native source words and lookup still exist");
        let context = older.resolved_input.as_ref().unwrap().context_registry();
        assert_eq!(
            selected.with_source_schema(&context, |schema| schema.subcommand.kind()),
            Some(Some(
                tcl_registry::resolved_invocation::SubcommandResolutionKind::Unknown
            ))
        );
        assert!(
            hints(source, &older, profile, registry)
                .iter()
                .all(|hint| hint.kind != InlayHintKind::Parameter)
        );
    }

    #[test]
    fn original_format_hints_use_selected_option_roles_under_the_actual_context() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "clock scan 2020 -format {%Y}";
        let current = analysis_with_availability(source, "tcl9.1");
        let profile = current.resolved_profile().unwrap();
        let registry = current.resolved_registry().unwrap();
        let positive = hints(source, &current, profile, registry);
        assert_eq!(
            positive.iter().filter(|hint| hint.label == "year").count(),
            1,
            "{positive:?}"
        );
        let older = analysis_with_availability(source, "tcl8.4");
        let out = hints(source, &older, profile, registry);
        assert!(out.iter().all(|hint| hint.label != "year"), "{out:?}");
        let segment = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            older.body_lexer_config.unwrap(),
        )
        .remove(0);
        let selected =
            selected_original_words(source, &older, &segment, older.resolved_registry().unwrap())
                .expect("original words exist independently of unavailable format option");
        let context = older.resolved_input.as_ref().unwrap().context_registry();
        // The closure is required: the method path is not general enough
        // over the schema borrow's higher-ranked lifetime.
        #[allow(clippy::redundant_closure_for_method_calls)]
        let formats = selected
            .with_source_schema(&context, |schema| schema.authored_source_format_arguments())
            .expect("the actual context still admits clock scan source metadata");
        assert!(
            formats.is_none_or(|formats| formats.is_empty()),
            "the actual option grammar supplies no profile-derived format operand"
        );
    }

    #[test]
    fn original_hint_metadata_keeps_the_generation_after_reporting_maps_are_cleared() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "format {%d} 3";
        let mut analysis = analysis_with_availability(source, "tcl9.1");
        let profile = analysis.resolved_profile().unwrap();
        let retained_context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let registry = retained_context.commands();
        let expected = hints(source, &analysis, profile, registry);
        assert!(
            expected.iter().any(|hint| hint.label == "int"),
            "{expected:?}"
        );
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        analysis.global_scope.variables.clear();
        analysis.dialect = "foreign presentation label".to_owned();
        assert_eq!(hints(source, &analysis, profile, registry), expected);
        assert!(hints("format {%x} 3", &analysis, profile, registry).is_empty());

        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let foreign = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        );
        analysis.resolved_input = Some(foreign);
        assert!(hints(source, &analysis, profile, registry).is_empty());
    }

    #[test]
    fn original_inlay_hints_keep_retained_profile_registry_and_whole_source() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "set value 42\nformat {%d} 3";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.global_scope.variables.clear();
        let profile = analysis.resolved_profile().unwrap();
        let registry = analysis.resolved_registry().unwrap();
        let expected = hints(source, &analysis, profile, registry);
        assert!(
            expected.iter().any(|hint| hint.label == ": int"),
            "{expected:?}"
        );
        assert!(
            expected.iter().any(|hint| hint.label == "int"),
            "{expected:?}"
        );
        assert_eq!(
            hints(
                source,
                &analysis,
                crate::profile_for_dialect("jim"),
                crate::registry_for_dialect("jim")
            ),
            expected,
        );
        assert!(hints(&format!("{source} "), &analysis, profile, registry).is_empty());
        let config = analysis.body_lexer_config.unwrap();
        let mut different = config;
        different.strict_quoting = !different.strict_quoting;
        analysis.body_lexer_config = Some(different);
        assert!(
            hints(
                source,
                &analysis,
                profile,
                crate::registry_for_dialect("tcl8.6")
            )
            .is_empty()
        );
    }

    #[test]
    fn original_format_hints_follow_actual_nested_roles_and_known_shadow() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "proc p {} {format {%d} 3}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let out = hints(
            source,
            &analysis,
            crate::profile_for_dialect("jim"),
            crate::registry_for_dialect("jim"),
        );
        assert!(out.iter().any(|hint| hint.label == "int"), "{out:?}");
        let source = "proc format args {return ignored}\nproc p {} {format {%d} 3}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let out = hints(
            source,
            &analysis,
            crate::profile_for_dialect("tcl8.6"),
            crate::registry_for_dialect("tcl8.6"),
        );
        assert!(!out.iter().any(|hint| hint.label == "int"), "{out:?}");
    }

    #[test]
    fn logical_type_hints_select_the_original_definition_version() {
        let source = "set value 42\nset value TEXT\nputs $value\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let hints = inlay_hints(
            source,
            analysis.resolved_profile().unwrap(),
            whole_document_range(source),
            Some(&analysis),
            analysis.resolved_registry(),
            true,
            false,
        );
        assert!(
            hints.iter().any(|hint| hint.kind == InlayHintKind::Type
                && hint.position_line == 0
                && hint.label == ": int"),
            "{hints:?}"
        );
        assert!(
            hints
                .iter()
                .all(|hint| hint.position_line != 0 || hint.label != ": str"),
            "{hints:?}"
        );
    }

    #[test]
    fn logical_type_hints_refuse_a_same_named_definition_from_another_body() {
        let source = "proc first {} {set value 42}\nproc second {} {set value TEXT}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let scopes = &mut analysis.global_scope.children;
        assert_eq!(scopes.len(), 2);
        let foreign_span = scopes[1].variables["value"].definition_span;
        scopes[0]
            .variables
            .get_mut("value")
            .unwrap()
            .definition_span = foreign_span;
        let hints = inlay_hints(
            source,
            analysis.resolved_profile().unwrap(),
            whole_document_range(source),
            Some(&analysis),
            analysis.resolved_registry(),
            true,
            false,
        );
        assert!(
            hints
                .iter()
                .all(|hint| hint.kind != InlayHintKind::Type || hint.position_line != 0),
            "{hints:?}"
        );
        assert_eq!(
            hints
                .iter()
                .filter(|hint| hint.kind == InlayHintKind::Type && hint.position_line == 1)
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>(),
            vec![": str"],
            "{hints:?}"
        );
    }

    #[test]
    fn logical_type_hints_keep_repeated_declarations_on_their_original_body_ids() {
        let source = "proc repeated {} {set value 42}\nproc repeated {} {set value TEXT}\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let registry = analysis.resolved_registry().unwrap();
        let hints = inlay_hints(
            source,
            analysis.resolved_profile().unwrap(),
            whole_document_range(source),
            Some(&analysis),
            Some(registry),
            true,
            false,
        );
        let labels = |line| {
            hints
                .iter()
                .filter(|hint| hint.kind == InlayHintKind::Type && hint.position_line == line)
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>()
        };
        assert!(labels(0).contains(&": int"), "{hints:?}");
        assert!(!labels(0).contains(&": str"), "{hints:?}");
        assert!(labels(1).contains(&": str"), "{hints:?}");
        assert!(!labels(1).contains(&": int"), "{hints:?}");
    }

    #[test]
    fn logical_type_hints_keep_constructed_colon_scopes_and_original_source() {
        // naming.analysis.constructed-key-consumer-projections
        // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
        let source = "namespace eval Ω {\nproc :pair {} {set x 42}\nproc pair {} {set x 3.5}\n}\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let registry = analysis.resolved_registry().unwrap();
        let hints = inlay_hints(
            source,
            analysis.resolved_profile().unwrap(),
            whole_document_range(source),
            Some(&analysis),
            Some(registry),
            true,
            false,
        );
        let labels = |line| {
            hints
                .iter()
                .filter(|hint| hint.kind == InlayHintKind::Type && hint.position_line == line)
                .map(|hint| hint.label.as_str())
                .collect::<Vec<_>>()
        };
        assert!(labels(1).contains(&": int"), "{hints:?}");
        assert!(!labels(1).contains(&": double"), "{hints:?}");
        assert!(labels(2).contains(&": double"), "{hints:?}");
        assert!(!labels(2).contains(&": int"), "{hints:?}");
        let changed = format!("# changed\n{source}");
        assert!(
            inlay_hints(
                &changed,
                analysis.resolved_profile().unwrap(),
                whole_document_range(&changed),
                Some(&analysis),
                Some(registry),
                true,
                false,
            )
            .is_empty()
        );
        assert!(
            inlay_hints(
                source,
                analysis.resolved_profile().unwrap(),
                whole_document_range(source),
                None,
                Some(registry),
                true,
                false,
            )
            .is_empty()
        );
    }
}
