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

//! W210 on the output variables of a matcher that provably does not match.

use super::super::helpers::block_dominated_by;
use super::{ReadBeforeSetCtx, rbs_trace_count, startup_read_facts};
use crate::analyser::state::Analyser;
use crate::expr_ast::{ExprNode, UnaryOp};
use std::collections::HashSet;

impl Analyser {
    /// W210 from a selected original matcher that leaves its targets unchanged.
    /// Logical source advice follows the same represented SSA value; Native
    /// emission additionally requires independent closed read-absence proof.
    pub(in super::super) fn emit_provably_unset_w210(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        considered: &HashSet<crate::cfg::BlockId>,
        ctx: &ReadBeforeSetCtx<'_>,
        params: &HashSet<&str>,
    ) {
        // naming.diagnostics.original-matcher-output-retention
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
        let Some(input) = self.result.resolved_input.as_ref() else {
            return;
        };
        let context = input.context_registry();
        let logical = self.result.allows_retained_logical_declaration_advice();
        let origins = self.collect_no_match_origins(fu, considered, &context, logical);
        let mut spans = std::collections::HashMap::new();
        for &block in considered {
            let Some(data) = fu.ssa.blocks.get(&block) else {
                continue;
            };
            for (index, statement) in data.statements.iter().enumerate() {
                let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
                for (&symbol, &version) in &statement.uses {
                    if statement.quoted_uses.contains(&symbol)
                        || statement.name_only_uses.contains(&symbol)
                        || !origins.get(&symbol).is_some_and(|candidates| {
                            candidates.iter().any(|origin| {
                                origin.version == version && origin.reaches(fu, block, index)
                            })
                        })
                    {
                        continue;
                    }
                    let name = fu.ssa.var_name(symbol);
                    let span = if logical {
                        let Some(cell) = fu.ssa.cell_key(symbol) else {
                            continue;
                        };
                        if params.contains(name)
                            || ctx.scope_aliases.contains(name)
                            || ctx.extra_known_defined.contains(name)
                            || ctx.supp.suppresses(name)
                            || ctx.cell_facts.known_defined.contains(cell)
                            || fu.dynamic_names.writes
                        {
                            continue;
                        }
                        let startup = startup_read_facts(
                            cell,
                            0,
                            false,
                            ctx.initial_global,
                            ctx.global_aliases,
                            Some(context.context().authoring_query()),
                            context.commands(),
                        );
                        if startup.readable {
                            continue;
                        }
                        let Some(statement) = fu
                            .cfg
                            .blocks
                            .get(&block)
                            .and_then(|data| data.statements.get(index))
                        else {
                            continue;
                        };
                        let Some(span) =
                            self.narrow_to_read_var(fu.abs_span(statement.span()), name)
                        else {
                            continue;
                        };
                        span
                    } else {
                        let Some(span) = original_missing_matcher_read_span(
                            fu,
                            view,
                            symbol,
                            context.commands(),
                        ) else {
                            continue;
                        };
                        span
                    };
                    spans
                        .entry(name.to_owned())
                        .and_modify(|earlier: &mut tcl_lexer::Span| {
                            if span.start() < earlier.start() {
                                *earlier = span;
                            }
                        })
                        .or_insert(span);
                }
            }
        }
        rbs_trace_count("map_provably_unset", spans.len());
        self.emit_w210_read_spans(spans, ctx);
    }

    fn collect_no_match_origins(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        considered: &HashSet<crate::cfg::BlockId>,
        context: &tcl_registry::model::ContextRegistry,
        logical: bool,
    ) -> std::collections::HashMap<crate::ssa::Symbol, Vec<NoMatchOutputOrigin>> {
        use crate::registry_invocation::source_structure::original_registry_words_for_tokens;
        let mut origins = std::collections::HashMap::new();
        for &block in considered {
            let Some(data) = fu.cfg.blocks.get(&block) else {
                continue;
            };
            for index in 0..data.statements.len() {
                let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let Some(words) =
                    original_registry_words_for_tokens(&self.source, &self.result, tokens)
                else {
                    continue;
                };
                record_no_match_outputs(
                    fu,
                    NoMatchCallSite {
                        producer: block,
                        index,
                        dominator: block,
                        branch_entry: false,
                    },
                    &selected_matcher_no_match_outputs(&words, context),
                    logical,
                    &mut origins,
                );
            }
            if let Some(crate::cfg::Terminator::Branch {
                condition,
                condition_base,
                true_target,
                false_target,
                ..
            }) = &data.terminator
                && let Some((words, negated)) =
                    self.original_condition_matcher(condition, *condition_base)
            {
                let outputs = selected_matcher_no_match_outputs(&words, context);
                let head = words
                    .head_source()
                    .and_then(|head| head.word())
                    .map(tcl_lexer::NativeWord::span);
                let index = (0..data.statements.len())
                    .find(|&index| {
                        crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index)
                            .source_tokens()
                            .and_then(|tokens| tokens.words().first())
                            .is_some_and(|word| Some(word.source().span) == head)
                    })
                    .unwrap_or(data.statements.len());
                record_no_match_outputs(
                    fu,
                    NoMatchCallSite {
                        producer: block,
                        index,
                        dominator: if negated { *true_target } else { *false_target },
                        branch_entry: true,
                    },
                    &outputs,
                    logical,
                    &mut origins,
                );
            }
        }
        origins
    }

    /// The condition's retained absolute source base owns expression offsets.
    /// Detached text, transformed expressions and multiple commands decline.
    pub(super) fn original_condition_matcher(
        &self,
        condition: &ExprNode,
        base: Option<u32>,
    ) -> Option<(
        crate::registry_invocation::source_structure::OriginalRegistryWords,
        bool,
    )> {
        let (command, negated) = match condition {
            ExprNode::Command { .. } => (condition, false),
            ExprNode::Unary {
                op: UnaryOp::Not | UnaryOp::WordNot,
                operand,
            } if matches!(operand.as_ref(), ExprNode::Command { .. }) => (operand.as_ref(), true),
            _ => return None,
        };
        let ExprNode::Command { text, start, end } = command else {
            return None;
        };
        let span = tcl_syntax::expr::expression_span(
            &self.source,
            base?.checked_add(*start)?,
            base?.checked_add(*end)?,
        )?;
        let start = span.start();
        let end = span.end();
        if self
            .source
            .get(usize::try_from(start).ok()?..usize::try_from(end).ok()?)?
            != text
        {
            return None;
        }
        let inner = text.strip_prefix('[')?.strip_suffix(']')?;
        let config = self.result.body_lexer_config?;
        let commands = tcl_source::segment::segment_commands_with_offset_and_config(
            inner,
            start.checked_add(1)?,
            tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
        );
        let [command] = commands.as_slice() else {
            return None;
        };
        crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            command,
        )
        .map(|words| (words, negated))
    }
}

/// Tcl ARE metacharacters: a pattern free of these reduces to a literal
/// substring search.
const TCL_REGEX_METACHARS: &str = r"\^$.|?*+()[]{}";

/// `regexp` switches that don't change match-vs-no-match for a pure-literal
/// pattern.
fn is_regexp_literal_safe_switch(opt: &str) -> bool {
    matches!(
        opt,
        "-indices" | "-inline" | "-all" | "-line" | "-lineanchor" | "-linestop" | "-start" | "--"
    )
    // `-expanded` is handled separately (whitespace/comment-gated) by the
    // caller, so it is intentionally not listed here.
}

/// True iff `regexp PATTERN INPUT` provably returns 0.  Sound only when
/// `pat` is a pure-literal pattern (no ARE metacharacters), reducing the
/// match to substring search.  Unknown / unsafe switches bail (return
/// `false` = cannot prove no-match).
fn regexp_literal_no_match(pat: &str, inp: &str, options: &[String]) -> bool {
    if pat.chars().any(|c| TCL_REGEX_METACHARS.contains(c)) {
        return false;
    }
    let mut nocase = false;
    let mut expanded = false;
    for opt in options {
        if !opt.starts_with('-') {
            continue; // an option value (e.g. after `-start`)
        }
        if opt == "-nocase" {
            nocase = true;
            continue;
        }
        if opt == "-expanded" {
            expanded = true;
            continue;
        }
        if is_regexp_literal_safe_switch(opt) {
            continue;
        }
        return false; // unknown / unsafe switch
    }
    // `-expanded` makes Tcl ignore unescaped whitespace and `#`-comments in
    // the pattern, so a pattern containing either is NOT a plain substring
    // (`regexp -expanded {a b} {ab}` matches).  Bail in that case so the
    // no-match proof stays sound — a whitespace/comment-free literal is
    // still safe.
    if expanded && pat.chars().any(|c| c.is_whitespace() || c == '#') {
        return false;
    }
    if nocase {
        !inp.to_lowercase().contains(&pat.to_lowercase())
    } else {
        !inp.contains(pat)
    }
}

/// One unchanged output in the represented source-value projection.
struct NoMatchOutputOrigin {
    dominator: crate::cfg::BlockId,
    pub(super) index: usize,
    pub(super) version: crate::ssa::Version,
    branch_entry: bool,
}

impl NoMatchOutputOrigin {
    pub(super) fn reaches(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        block: crate::cfg::BlockId,
        index: usize,
    ) -> bool {
        if block == self.dominator {
            self.branch_entry || index > self.index
        } else {
            block_dominated_by(&fu.ssa, block, self.dominator)
        }
    }
}

#[derive(Clone, Copy)]
struct NoMatchCallSite {
    producer: crate::cfg::BlockId,
    pub(super) index: usize,
    dominator: crate::cfg::BlockId,
    branch_entry: bool,
}

fn record_no_match_outputs(
    fu: &crate::compilation_unit::FunctionUnit,
    site: NoMatchCallSite,
    outputs: &[String],
    logical: bool,
    origins: &mut std::collections::HashMap<crate::ssa::Symbol, Vec<NoMatchOutputOrigin>>,
) {
    let Some(data) = fu.ssa.blocks.get(&site.producer) else {
        return;
    };
    let point = if site.index == data.statements.len() {
        usize::MAX
    } else {
        site.index
    };
    let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, site.producer, point);
    for name in outputs {
        let symbol = view.symbol(name).or_else(|| {
            logical
                .then(|| {
                    fu.ssa
                        .cell_symbol(&tcl_names::variable::VariableCellKey::Authored(
                            name.clone(),
                        ))
                })
                .flatten()
        });
        let Some(symbol) = symbol else {
            continue;
        };
        let before = data.statements[..site.index]
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, statement)| {
                fu.ssa
                    .value_clobbers
                    .get(&site.producer)
                    .and_then(|markers| markers.get(&index))
                    .and_then(|versions| versions.get(&symbol))
                    .map(|&(_, fresh)| fresh)
                    .or_else(|| statement.defs.get(&symbol).copied())
            })
            .or_else(|| data.entry_versions.get(&symbol).copied())
            .unwrap_or(0);
        // Failure retains a preceding value, and any later version is separate.
        if before != 0 {
            continue;
        }
        let version = data
            .statements
            .get(site.index)
            .and_then(|statement| statement.defs.get(&symbol))
            .copied()
            .unwrap_or(before);
        origins
            .entry(symbol)
            .or_default()
            .push(NoMatchOutputOrigin {
                dominator: site.dominator,
                index: site.index,
                version,
                branch_entry: site.branch_entry,
            });
    }
}

fn original_missing_matcher_read_span(
    fu: &crate::compilation_unit::FunctionUnit,
    view: crate::ssa::SsaSourceView<'_>,
    symbol: crate::ssa::Symbol,
    registry: &tcl_registry::CommandRegistry,
) -> Option<tcl_lexer::Span> {
    view.source_tokens()?
        .variable_accesses
        .iter()
        .filter_map(|access| {
            let read = view.read_reference(&access.source, &access.original_spelling)?;
            if read.symbol != symbol
                || !view
                    .read_contents_presence_alternatives_at(
                        &access.source,
                        &access.original_spelling,
                        registry,
                    )
                    .is_some_and(crate::ssa::SsaReadPresenceAlternatives::may_be_undefined)
                || super::super::helpers::original_read_is_live_array_scalar(access, registry)
            {
                return None;
            }
            Some(fu.abs_span(access.source.span))
        })
        .min_by_key(|span| span.start())
}

/// Complete selected source matcher layout and decoded effective literal argv.
/// A handler descriptor identifies the authored protocol; it never proves an
/// installed handler, physical target, normal completion or current absence.
pub(super) fn selected_matcher_no_match_outputs(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<String> {
    use tcl_registry::variable_output::NativeVariableOutputSpec;
    // naming.diagnostics.original-matcher-output-retention
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
    let Some(layout) = SourceMatcherLayout::from_original(words, context) else {
        return Vec::new();
    };
    let pattern = layout.arguments[layout.pattern];
    let input = layout.arguments[layout.input];
    if !pattern.is_ascii()
        || !input.is_ascii()
        || pattern.contains('\0')
        || input.contains('\0')
        || pattern.len() > 1024
        || input.len() > 8192
    {
        return Vec::new();
    }
    let no_match = match layout.protocol {
        NativeVariableOutputSpec::Regexp => {
            let options = layout.arguments[..layout.pattern]
                .iter()
                .map(|option| (*option).to_owned())
                .collect::<Vec<_>>();
            !options.iter().any(|option| option == "-start")
                && regexp_literal_no_match(pattern, input, &options)
        }
        NativeVariableOutputSpec::Scan => {
            let binary = words
                .dialect()
                .and_then(|dialect| dialect.tcl_version)
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_6);
            scan_literal_no_match(pattern, input, layout.outputs.len(), binary)
        }
    };
    if !no_match {
        return Vec::new();
    }
    layout
        .outputs
        .into_iter()
        .filter_map(|index| {
            let name = *layout.arguments.get(index)?;
            (!name.is_empty()).then(|| name.to_owned())
        })
        .collect()
}

struct SourceMatcherLayout<'a> {
    pub(super) protocol: tcl_registry::variable_output::NativeVariableOutputSpec,
    arguments: Vec<&'a str>,
    outputs: Vec<usize>,
    pattern: usize,
    input: usize,
}

impl<'a> SourceMatcherLayout<'a> {
    fn from_original(
        words: &'a crate::registry_invocation::source_structure::OriginalRegistryWords,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<Self> {
        use tcl_registry::{
            ArgRole, native_compilation::SuccessfulHandlerSpec,
            variable_output::NativeVariableOutputSpec,
        };
        let facts = words.with_source_schema(
            context,
            crate::registry_invocation::source_schema_queries::facts,
        )?;
        let SuccessfulHandlerSpec::ConditionalVariableOperands(protocol) =
            facts.successful_handler?
        else {
            return None;
        };
        if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        let arguments = words
            .arguments()
            .iter()
            .map(|word| std::str::from_utf8(word.literal_bytes()?).ok())
            .collect::<Option<Vec<_>>>()?;
        if facts.frozen_argument_count != Some(arguments.len()) {
            return None;
        }
        let indices = |role| {
            facts.arg_roles.iter().filter_map(move |&(index, found)| {
                (role == found).then_some(facts.argument_offset + usize::from(index))
            })
        };
        let outputs = indices(ArgRole::VarWrite).collect::<Vec<_>>();
        if outputs.is_empty() {
            return None;
        }
        let (pattern, input) = match protocol {
            NativeVariableOutputSpec::Regexp => {
                let pattern = indices(ArgRole::Pattern).next()?;
                (pattern, pattern.checked_add(1)?)
            }
            NativeVariableOutputSpec::Scan => {
                let pattern = indices(ArgRole::ScanFormat).next()?;
                (pattern, pattern.checked_sub(1)?)
            }
        };
        arguments.get(pattern)?;
        arguments.get(input)?;
        Some(Self {
            protocol,
            arguments,
            outputs,
            pattern,
            input,
        })
    }
}

/// The shared parser validates the complete dialect-common plain conversion
/// subset, and `%b` where the unit's release has it (8.6 and later), before
/// the existing first-conversion no-match kernel is consulted.
fn scan_literal_no_match(format: &str, input: &str, output_count: usize, binary: bool) -> bool {
    let chars = format.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut conversions = 0;
    while index < chars.len() {
        if chars[index] != '%' {
            index += 1;
            continue;
        }
        index += 1;
        let Ok(conversion) = tcl_syntax::scan::parse_conversion(&chars, &mut index) else {
            return false;
        };
        if conversion.suppress
            || conversion.width.is_some()
            || conversion.size.is_some()
            || conversion.xpg_index.is_some()
            || !(matches!(conversion.verb, '%' | 'd' | 'o' | 'x' | 'X' | 's' | 'c')
                || binary && conversion.verb == 'b')
        {
            return false;
        }
        conversions += usize::from(conversion.verb != '%');
    }
    conversions == output_count && crate::scan_predicate::scan_provably_no_match(format, input)
}
