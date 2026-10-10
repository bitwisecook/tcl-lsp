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

//! Inline variable — replace a single-use variable with its value.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::types::{Scope, VarDef};
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{LexerConfig, LineIndex, Token, TokenType};
use tcl_registry::CommandRegistry;

use super::{
    MAX_COMMAND_SEARCH_DEPTH, RefactorEdit, Refactoring, find_command_at, token_end_offset,
};
use crate::code_actions::ActionKind;

/// Inline the variable defined by the `set` command at byte offset
/// `cursor`.
///
/// Only inlines when the cursor is on a `set var value` command and the
/// variable has exactly one reference after the definition; returns
/// `None` otherwise.
#[must_use]
pub fn inline_variable(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    line_index: &LineIndex,
) -> Option<Refactoring> {
    let config = analysis.body_lexer_config?;
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return None;
    }
    let cmd = if analysis.allows_lexical_declaration_advice() {
        find_command_at(source, cursor, None, registry, config)?
    } else {
        super::find_original_command_at(source, cursor, analysis)?
    };
    let (var_name, ref_span, value_index) = if analysis.allows_lexical_declaration_advice() {
        if cmd.name() != "set" || cmd.texts.len() != 3 {
            return None;
        }
        let var_name = cmd.texts[1].clone();
        let cmd_line = line_index.line_at(cmd.span.start());
        let definition = walk_scopes(&analysis.global_scope)
            .into_iter()
            .find(|definition| {
                definition.name == var_name
                    && line_index.line_at(definition.definition_span.start()) == cmd_line
            })?;
        let [reference] = definition.references.as_slice() else {
            return None;
        };
        (var_name, *reference, 2)
    } else {
        original_inline_binding(source, analysis, &cmd)?
    };

    // The value word, verbatim from source (widened to include any
    // closing quote / brace / bracket).
    let value_tok = *cmd.argv.get(value_index)?;
    let value_start = value_tok.span.start() as usize;
    let value_end = token_end_offset(source, value_tok) as usize;
    let value_text = source.get(value_start..value_end)?;

    // Delete the `set` command, and with it the whole line when the
    // command is alone on one: the leading indentation belongs to the
    // deleted line, so leaving it behind would push the following line
    // out by that much.
    let (mut cmd_start, mut cmd_end) = super::command_span_offsets(source, &cmd);
    let line_start = source[..cmd_start as usize]
        .rfind('\n')
        .map_or(0, |nl| nl + 1);
    if source
        .get(line_start..cmd_start as usize)?
        .trim()
        .is_empty()
    {
        cmd_start = u32::try_from(line_start).ok()?;
    }
    if source.as_bytes().get(cmd_end as usize) == Some(&b'\n') {
        cmd_end += 1;
    }
    let delete = RefactorEdit {
        start: cmd_start,
        end: cmd_end,
        new_text: String::new(),
    };

    // Resolve the reference's VAR token + enclosing word so we know,
    // from the tokens alone, whether the reference is a standalone word
    // or interpolated inside a larger word.
    let ctx = reference_token(source, ref_span.start(), registry, config, analysis)?;

    // The `$var` / `${var}` span to replace.  The VAR token starts at
    // `$`; its end omits the braced form's `}`, so re-add it.
    let ref_start = ctx.var_tok.span.start();
    let mut ref_end = token_end_offset(source, ctx.var_tok);
    if source
        .get(ref_start as usize..ref_start as usize + 2)
        .is_some_and(|s| s == "${")
        && source.as_bytes().get(ref_end as usize) == Some(&b'}')
    {
        ref_end += 1;
    }

    // A standalone word keeps the value verbatim (preserving quotes /
    // braces so it stays one word).  Interpolated inside a larger word,
    // splice the unquoted content instead — keeping the delimiters would
    // yield `"hello "world""`.  A bare (unquoted) concatenation can only
    // take a value with no whitespace.
    let new_text = if ctx.word_is_single {
        value_text.to_owned()
    } else {
        let inner = dequote(value_text);
        // Splicing a brace-quoted (literal) value into an interpolated context
        // (a bare concatenation or inside `"…"`) would ACTIVATE any `$`/`[`/`\`
        // that were literal inside the braces: `set x {a $b}` inlined into
        // `puts "v: $x"` must not turn `$b` into a live substitution.
        // Decline the refactor rather than change the value.
        let was_braced = value_text.trim_start().starts_with('{');
        if was_braced && inner.bytes().any(|b| matches!(b, b'$' | b'[' | b'\\')) {
            return None;
        }
        let in_quoted_string = source.as_bytes().get(ctx.word_start as usize) == Some(&b'"');
        if !in_quoted_string && inner.chars().any(char::is_whitespace) {
            return None;
        }
        inner.to_owned()
    };

    let replace = RefactorEdit {
        start: ref_start,
        end: ref_end,
        new_text,
    };

    Some(Refactoring {
        title: format!("Inline variable '${var_name}'"),
        edits: vec![delete, replace],
        kind: ActionKind::RefactorInline,
        data_group: None,
        disabled: None,
    })
}

/// Source selection and rendering data, without a runtime name lookup grant.
struct OriginalInlineContext<'a> {
    image: tcl_lexer::SourceImage,
    config: LexerConfig,
    registry: &'a CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
    original: tcl_compiler::ir::CommandTokens,
    offset: u32,
}

type OriginalInlineBinding = (String, tcl_lexer::Span, usize);

/// A literal setter and its single physical value read. Name geometry alone
/// cannot establish store preservation, observers or reaching contents.
fn original_inline_binding(
    source: &str,
    analysis: &AnalysisResult,
    command: &SegmentedCommand,
) -> Option<OriginalInlineBinding> {
    use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
    let context = OriginalInlineContext {
        image: tcl_lexer::SourceImage::document(source),
        config: analysis.body_lexer_config?,
        registry: analysis.resolved_registry()?,
        profile: analysis.resolved_profile()?,
        original: tcl_compiler::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            analysis.body_lexer_config?,
            command,
        ),
        offset: command.span.start(),
    };
    let unit = CompilationUnit::build_with_options(
        source,
        UnitBuildOptions {
            registry: context.registry,
            config: context.config,
            dialect: Some(context.profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
    );
    let mut selected = None;
    for function in unit.analysable_body_function_units() {
        let opaque = function.cfg.has_opaque_native_accesses();
        let barrier = function.dynamic_barrier_blocks_value_motion();
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_INLINE_FUNCTION offset={} opaque={opaque} barrier={barrier} blocks={}",
                context.offset,
                function.ssa.blocks.len()
            );
        }
        if opaque || barrier {
            continue;
        }
        for (&block, body) in &function.ssa.blocks {
            for index in 0..body.statements.len() {
                if let Some(candidate) = point_inline_binding(&context, function, block, index) {
                    if selected.is_some() {
                        return None;
                    }
                    selected = Some(candidate);
                }
            }
        }
    }
    selected
}

fn point_inline_binding(
    context: &OriginalInlineContext<'_>,
    function: &tcl_compiler::compilation_unit::FunctionUnit,
    block: tcl_compiler::cfg::BlockId,
    index: usize,
) -> Option<OriginalInlineBinding> {
    use tcl_compiler::ir::{WordExpr, WordPart};
    use tcl_compiler::ssa::SsaSourceView;
    let view = SsaSourceView::at_statement(&function.ssa, block, index);
    let tokens = view.source_tokens()?;
    if function
        .abs_span(tokens.words().first()?.source().span)
        .start()
        != context.offset
    {
        return None;
    }
    let normal = tcl_compiler::registry_invocation::normal_transfer_invocation(
        context.registry,
        Some(tcl_registry::model::semantic::SemanticContext::for_profile(
            context.profile,
        )),
        tokens,
    );
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
        eprintln!(
            "ORIGINAL_INLINE_POINT offset={} normal={} binding={} handler={} continuation={}",
            context.offset,
            normal.is_some(),
            tokens.source_binding.is_some(),
            tokens
                .source_binding
                .as_ref()
                .is_some_and(|binding| binding.proved_handler_target().is_some()),
            tokens
                .source_binding
                .as_ref()
                .is_some_and(|binding| binding.normal_variable_continuation().is_some())
        );
    }
    let normal = normal?;
    let value_index = normal
        .written_argument(normal.stored_value_argument()?)?
        .checked_add(1)?;
    let value = context.original.words().get(value_index)?;
    if !matches!(
        value,
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
    ) && !matches!(value, WordExpr::Template { parts, .. } if parts.iter().all(|part| matches!(part, WordPart::Text { .. })))
    {
        return None;
    }
    normal.stored_value_word(
        &tokens.source_binding.as_ref()?.variable_context,
        context.registry,
    )?;
    let statement = function.ssa.blocks.get(&block)?.statements.get(index)?;
    let mut selected = None;
    for (&symbol, &version) in &statement.defs {
        if let Some((label, read_span)) =
            value_read_binding(context, function, view, block, index, symbol, version)
        {
            if selected.is_some() {
                return None;
            }
            selected = Some((label, read_span, value_index));
        }
    }
    selected
}

fn value_read_binding(
    context: &OriginalInlineContext<'_>,
    function: &tcl_compiler::compilation_unit::FunctionUnit,
    view: tcl_compiler::ssa::SsaSourceView<'_>,
    block: tcl_compiler::cfg::BlockId,
    index: usize,
    symbol: tcl_compiler::ssa::Symbol,
    version: tcl_compiler::ssa::Version,
) -> Option<(String, tcl_lexer::Span)> {
    use tcl_compiler::def_use::{DefKind, UseKind};
    use tcl_compiler::ssa::SsaSourceView;
    let definition = view.original_definition_name(symbol, context.registry)?;
    let input = definition.original_name_input();
    let key = input.original_word_key()?;
    if key.source_image() != &context.image
        || key.lexer_config() != context.config
        || !view.normal_store_contents_preserved(symbol, context.registry)
    {
        return None;
    }
    let chain = function
        .def_use
        .chains
        .get(&(definition.cell().clone(), version))?;
    let [use_site] = chain.uses.as_slice() else {
        return None;
    };
    if chain.definition.kind != DefKind::Statement
        || use_site.kind != UseKind::Operand
        || use_site.block != chain.definition.block
    {
        return None;
    }
    let use_index = usize::try_from(use_site.statement_index)
        .ok()
        .filter(|use_index| *use_index > index)?;
    let use_view = SsaSourceView::at_statement(&function.ssa, block, use_index);
    let use_tokens = use_view.source_tokens()?;
    let mut reads = use_tokens.variable_accesses.iter().filter(|access| {
        use_view
            .replaceable_read_at(&access.source, &access.original_spelling, context.registry)
            .is_some_and(|reference| {
                reference.symbol == symbol && reference.version == Some(version)
            })
    });
    let read = reads.next()?;
    if reads.next().is_some() {
        return None;
    }
    let label = tcl_syntax::native_string::resident_name_label(input.bytes());
    Some((label, function.abs_span(read.source.span)))
}

/// Resolved reference context — the VAR token, whether it is its own
/// word, and the enclosing word's start offset.
struct RefContext {
    var_tok: Token,
    word_is_single: bool,
    word_start: u32,
}

/// Resolve `ref_off` through the segmenter
/// to its VAR token and enclosing word.
fn reference_token(
    source: &str,
    ref_off: u32,
    registry: &CommandRegistry,
    config: LexerConfig,
    analysis: &AnalysisResult,
) -> Option<RefContext> {
    let cmd = if analysis.allows_lexical_declaration_advice() {
        find_command_at(source, ref_off, None, registry, config)?
    } else {
        super::find_original_command_at(source, ref_off, analysis)?
    };
    resolve_in_command(source, &cmd, ref_off, config, 0)
}

/// Resolve `ref_off` against `cmd`'s own tokens, descending into the
/// command substitution that holds it when it is not one of them.
///
/// `set r [f $x]` reaches `$x` only through the `[f $x]` word, and it is
/// the *inner* command's words that decide how the value may be spliced:
/// `$x` is a whole word of `f`, so the value keeps its own quoting.
fn resolve_in_command(
    source: &str,
    cmd: &SegmentedCommand,
    ref_off: u32,
    config: LexerConfig,
    depth: u32,
) -> Option<RefContext> {
    let var_tok = cmd.all_tokens.iter().copied().find(|tok| {
        tok.kind == TokenType::Var
            && tok.span.start() <= ref_off
            && ref_off <= token_end_offset(source, *tok)
    });

    let Some(var_tok) = var_tok else {
        if MAX_COMMAND_SEARCH_DEPTH.exceeded(depth) {
            return None;
        }
        let (start, end) = substitution_interior(cmd, ref_off)?;
        let interior = source.get(start as usize..end as usize)?;
        let inner = segment_commands_with_offset_and_config(interior, start, config)
            .into_iter()
            .find(|c| c.span.start() <= ref_off && ref_off <= c.span.end())?;
        return resolve_in_command(source, &inner, ref_off, config, depth + 1);
    };

    for (word_tok, &is_single) in cmd.argv.iter().zip(cmd.single_token_word.iter()) {
        let word_start = word_tok.span.start();
        let word_end = token_end_offset(source, *word_tok);
        if word_start <= var_tok.span.start() && var_tok.span.start() < word_end {
            return Some(RefContext {
                var_tok,
                word_is_single: is_single,
                word_start,
            });
        }
    }
    Some(RefContext {
        var_tok,
        word_is_single: true,
        word_start: var_tok.span.start(),
    })
}

/// Byte range of the interior (brackets excluded) of `cmd`'s command
/// substitution containing `ref_off`.
fn substitution_interior(cmd: &SegmentedCommand, ref_off: u32) -> Option<(u32, u32)> {
    cmd.all_tokens
        .iter()
        .filter(|tok| tok.kind == TokenType::Cmd)
        .map(|tok| {
            (
                tok.span.start() + u32::from(tok.content_offset),
                tok.span.end(),
            )
        })
        .find(|&(start, end)| start <= ref_off && ref_off < end)
}

/// Strip one layer of matching `"…"` or `{…}` delimiters.
fn dequote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        if bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
            return &value[1..value.len() - 1];
        }
        if bytes[0] == b'{' && bytes[bytes.len() - 1] == b'}' {
            return &value[1..value.len() - 1];
        }
    }
    value
}

/// Yield every `VarDef` in `scope` and its descendants.
fn walk_scopes(scope: &Scope) -> Vec<&VarDef> {
    walk_scopes_at_depth(scope, 0)
}

fn walk_scopes_at_depth(scope: &Scope, depth: u32) -> Vec<&VarDef> {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return Vec::new();
    }
    let mut out: Vec<&VarDef> = scope.variables.values().collect();
    for child in &scope.children {
        out.extend(walk_scopes_at_depth(child, depth + 1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use tcl_compiler::analyser::Analyser;

    fn run(source: &str, cursor: u32) -> Option<String> {
        let reg = super::super::test_registry();
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let li = LineIndex::new(source);
        inline_variable(source, cursor, &analysis, &reg, &li).map(|r| r.apply(source))
    }

    /// `walk_scopes_at_depth` recurses once per nested namespace / proc
    /// scope, and `MAX_SCOPE_WALK_DEPTH` (`crate::lib`) is what stops that
    /// recursion from overflowing `cargo test`'s bare ~2 MiB per-test
    /// stack: unguarded, it goes over at around 100 levels. The assertion
    /// is that 80 levels return at all, not what they return.
    #[test]
    fn deeply_nested_namespaces_survive_scope_walk() {
        // Implementation contract: naming.source.recursive-driver-state-transport
        // docs/design/analysis/name-resolution-proofs/recursive-driver-state-transport.md
        const DEPTH: usize = 80;
        let mut source = String::new();
        for i in 0..DEPTH {
            let _ = writeln!(source, "namespace eval ns{i} {{");
        }
        source.push_str("set x 1\nputs $x\n");
        for _ in 0..DEPTH {
            source.push_str("}\n");
        }
        let analysis = Analyser::new().analyse(&source, "tcl8.6").clone();
        let _ = walk_scopes(&analysis.global_scope);
    }

    #[test]
    fn inline_single_use() {
        let source = "set name \"hello\"\nputs $name";
        let applied = run(source, 0).expect("result");
        assert!(!applied.contains("set name"), "{applied:?}");
        assert!(applied.contains("\"hello\""), "{applied:?}");
    }

    #[test]
    fn original_inline_variable_requires_exact_definition_and_unobserved_read_without_ui_maps() {
        // Implementation contract: naming.refactor.original-variable-inline-permission
        // docs/design/analysis/name-resolution-proofs/original-variable-inline-permission.md
        // Native proof: naming.refactor.variable-inline-literal-source-values
        // docs/design/analysis/name-resolution-proofs/inline-variable-literal-source-values.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let source = "set café 7\nputs ${café}";
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.global_scope.variables.clear();
            let registry = analysis.resolved_registry().unwrap();
            let index = LineIndex::new(source);
            let change = inline_variable(source, 0, &analysis, registry, &index).expect(dialect);
            assert_eq!(change.apply(source), "puts 7", "{dialect}");
            assert!(inline_variable("# stale source", 0, &analysis, registry, &index).is_none());
        }
    }

    #[test]
    fn original_inline_variable_rejects_reassigned_cells_observers_and_value_effect_movement() {
        // Implementation contract: naming.refactor.original-variable-inline-permission
        // docs/design/analysis/name-resolution-proofs/original-variable-inline-permission.md
        // Native proof: naming.refactor.variable-inline-reassignment-read
        // docs/design/analysis/name-resolution-proofs/inline-variable-reassignment-read.md
        // Native proof: naming.refactor.variable-inline-callee-upvar-read
        // docs/design/analysis/name-resolution-proofs/inline-variable-callee-upvar-read.md
        // Native proof: naming.refactor.variable-inline-modern-read-observer
        // docs/design/analysis/name-resolution-proofs/inline-variable-modern-read-observer.md
        // Native proof: naming.refactor.variable-inline-legacy-read-observer
        // docs/design/analysis/name-resolution-proofs/inline-variable-legacy-read-observer.md
        for source in [
            "set x 1; set x 2; puts $x",
            "set x [incr counter]; puts $x",
            "set x 1; unset x; set x 2; puts $x",
            "proc observer {args} {puts observed}; trace variable x r observer; set x 1; puts $x",
            "set x 1; upvar #0 x alias; set alias 2; puts $x",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let cursor = u32::try_from(source.find("set x").unwrap()).unwrap();
            assert!(
                inline_variable(
                    source,
                    cursor,
                    &analysis,
                    analysis.resolved_registry().unwrap(),
                    &LineIndex::new(source)
                )
                .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn inline_braced_reference() {
        let source = "set x 1\nputs ${x}";
        assert_eq!(run(source, 0).as_deref(), Some("puts 1"));
    }

    #[test]
    fn does_not_inline_braced_literal_with_subst_into_quotes() {
        // `set x {a $b}` holds the LITERAL `a $b`. Inlining it
        // into `"v: $x"` would turn `$b` into a live substitution, so the
        // refactor must decline rather than corrupt the value.
        let source = "set x {a $b}\nputs \"v: $x\"";
        assert_eq!(run(source, 0), None);
        // A `[cmd]` in a braced literal is equally unsafe.
        let source2 = "set x {a [cmd]}\nputs \"v: $x\"";
        assert_eq!(run(source2, 0), None);
        // FP-guard: a braced literal WITHOUT substitution chars still inlines.
        let source3 = "set x {a b}\nputs \"v: $x\"";
        assert_eq!(run(source3, 0).as_deref(), Some("puts \"v: a b\""));
    }

    #[test]
    fn inline_preserves_following_line_without_trailing_newline() {
        let source = "set x 1\nset y $x";
        assert_eq!(run(source, 0).as_deref(), Some("set y 1"));
    }

    #[test]
    fn inline_into_command_substitution() {
        // The only use sits inside `[…]`, so the reference resolves through
        // the substitution's own command.
        let source = "set timeout 30\nset result [list URL -timeout $timeout]";
        assert_eq!(
            run(source, 0).as_deref(),
            Some("set result [list URL -timeout 30]")
        );
        let unproved = "set timeout 30\nset result [http::geturl $url -timeout $timeout]";
        assert!(run(unproved, 0).is_none());
    }

    #[test]
    fn inline_into_nested_command_substitution() {
        let source = "set n 2\nputs [format %d [string repeat x $n]]";
        assert_eq!(
            run(source, 0).as_deref(),
            Some("puts [format %d [string repeat x 2]]")
        );
    }

    #[test]
    fn no_inline_of_a_reference_inside_a_braced_word() {
        // `expr {$n + 1}` holds `$n` in a braced literal, which `expr`
        // substitutes itself; the segmenter has no VAR token there, so
        // there is no reference span to rewrite.
        let source = "set n 2\nputs [expr {$n + 1}]";
        assert_eq!(run(source, 0), None);
    }

    #[test]
    fn inline_keeps_the_following_line_indentation() {
        // Deleting the `set` line must take its indentation with it.
        let source = "proc p {} {\n    set timeout 30\n    puts $timeout\n}\np";
        assert_eq!(
            run(source, 16).as_deref(),
            Some("proc p {} {\n    puts 30\n}\np")
        );
    }

    #[test]
    fn no_inline_multiple_uses() {
        assert!(run("set x 42\nputs $x\nputs $x", 0).is_none());
    }

    #[test]
    fn no_inline_read_form() {
        assert!(run("set x", 0).is_none());
    }

    #[test]
    fn no_inline_non_set() {
        assert!(run("puts \"hello\"", 0).is_none());
    }
}
