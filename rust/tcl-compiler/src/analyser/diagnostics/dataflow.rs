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

//! Control- and data-flow diagnostics derived from the per-function CFG and
//! SSA form.
//!
//! These checks run from the per-function dispatcher over the compilation
//! unit: dead stores (W220), unused variables (W211) and parameters (W214),
//! read-before-set (W210) including the phi-from-undef and provably-unset
//! variants, unset-on-possibly-undefined (W213), a paste-error fingerprint
//! (H300), constant branches and switch arms (I230/I231), channel-argument
//! validation (W126), divide-by-zero (W233), interval-bounds violations, an
//! invalid IP-address literal at its def site (W124), the racy `static::`
//! cross-event flow (IRULE4005), and the flow-sensitive renamed-command
//! check (W128).

mod constant_conditions;
mod dead_stores;
mod declared_reads;
mod irules_cells;
mod no_match_outputs;
mod read_before_set;
mod return_reads;
mod signed_scans;
mod unused_bindings;
mod value_domains;

#[cfg(test)]
use dead_stores::original_store_metadata;
#[cfg(test)]
pub(crate) use dead_stores::report_original_store_diagnostic_gates;
#[cfg(test)]
use no_match_outputs::selected_matcher_no_match_outputs;
#[cfg(test)]
use read_before_set::barrier_body_locally_sets;
#[cfg(test)]
pub(super) use unused_bindings::body_references_param;
#[cfg(test)]
use value_domains::collect_expr_command_texts;
#[cfg(test)]
pub(super) use value_domains::find_ipv6_candidates;

fn rbs_read_option<T>(stage: &str, value: Option<T>) -> Option<T> {
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some() {
        eprintln!("RBS_READ_OWNER stage={stage} present={}", value.is_some());
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
    value
}

fn rbs_trace_enabled() -> bool {
    cfg!(debug_assertions)
        && tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some()
}

fn rbs_trace_read(stage: &str, name: &str, span: tcl_lexer::Span) {
    if rbs_trace_enabled() {
        eprintln!("RBS_READ_OWNER stage={stage} name={name} span={span:?}");
    }
}

fn rbs_trace_count(stage: &str, count: usize) {
    if rbs_trace_enabled() {
        eprintln!("RBS_READ_OWNER stage={stage} count={count}");
    }
}

use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;
use tcl_registry::value_transfer::{DomainFact, Existence, FactView};

use super::helpers::{UndefSuppression, source_slice};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
#[cfg(test)]
use crate::expr_ast::ExprNode;

/// The read-only scope and suppression facts for the version-0 / statement
/// W210 pass. Keeping this alongside [`ReturnUndefCtx`] prevents the two W210
/// emitters from acquiring separate, subtly divergent entry-scope contracts.
pub(super) struct ReadBeforeSetCtx<'a> {
    pub initial_global: bool,
    pub original_unit: Option<&'a crate::compilation_unit::CompilationUnit>,
    pub global_aliases: &'a HashSet<String>,
    pub defined_vars: &'a HashSet<String>,
    pub scope_aliases: &'a HashSet<String>,
    pub extra_known_defined: &'a HashSet<String>,
    pub cell_facts: &'a super::helpers::DiagnosticCellFacts,
    pub supp: &'a UndefSuppression,
}

/// Existing absence proofs and guards used to classify declared reads.
#[derive(Clone, Copy)]
struct DeclaredReadProofs<'a> {
    exists_guards: &'a [super::helpers::ExistenceGuard],
    proved: &'a std::collections::HashMap<String, tcl_lexer::Span>,
}

pub(super) struct ReturnUndefCtx<'a> {
    pub registry: std::sync::Arc<tcl_registry::CommandRegistry>,
    /// The variables the read pass already reported: W210 is one per
    /// variable, so a `return` read of one of them adds nothing.
    pub already_reported: &'a HashSet<String>,
    pub initial_global: bool,
    pub global_aliases: &'a HashSet<String>,
    pub dialect: Option<SurfaceQuery<'a>>,
    pub params: &'a HashSet<&'a str>,
    pub exists_guards: &'a [super::helpers::ExistenceGuard],
    pub scope_aliases: &'a HashSet<String>,
    pub extra_known_defined: &'a HashSet<String>,
    pub cell_facts: &'a super::helpers::DiagnosticCellFacts,
    pub defined_vars: &'a HashSet<String>,
    pub considered: &'a HashSet<crate::cfg::BlockId>,
    pub supp: &'a UndefSuppression,
}

/// Registry-owned startup lifecycle facts for one SSA variable version.
#[derive(Clone, Copy)]
struct StartupReadFacts {
    readable: bool,
    initially_bound: bool,
    lazy_read: bool,
}

fn startup_read_facts(
    cell: &crate::var_resolve::VariableCellKey,
    version: crate::ssa::Version,
    killed: bool,
    initial_global: bool,
    global_aliases: &HashSet<String>,
    dialect: Option<SurfaceQuery<'_>>,
    registry: &tcl_registry::CommandRegistry,
) -> StartupReadFacts {
    let (startup_name, global_binding) =
        super::helpers::startup_cell_binding(cell, initial_global, global_aliases);
    let startup = registry.startup_existence(startup_name, dialect);
    StartupReadFacts {
        readable: global_binding && version == 0 && !killed && startup.readable(),
        initially_bound: global_binding && version == 0 && startup.initially_bound,
        lazy_read: global_binding && startup.lazily_readable,
    }
}

/// The existence fact a W210 or W213 read reports on: an unbound or a
/// may-bound place. A bound one, one the run never reached, and one it
/// computed nothing for — `Unavailable` below the deep tier or past the
/// complexity ceiling — report nothing.
fn reportable(fact: &FactView) -> Option<Existence> {
    match fact {
        FactView::Domain(DomainFact::Existence(
            fact @ (Existence::Unbound | Existence::MayBound),
        )) => Some(*fact),
        _ => None,
    }
}

/// Bind source-relative advice to this exact current SSA function. A source
/// receipt from another body cannot use an equal display name or numeric slot.
struct StaticReadUseScope<'a> {
    ssa: &'a crate::ssa::SsaFunction,
    source: crate::command_binding::OriginalProcedureStaticReadAlternatives,
}
impl StaticReadUseScope<'_> {
    fn may_select_static(&self, name: &str, span: tcl_lexer::Span) -> bool {
        self.source.may_select_static(name, span)
    }
    fn may_select_static_in_declared_frame(
        &self,
        name: &str,
        span: tcl_lexer::Span,
        context: &crate::var_resolve::ResolveContext,
    ) -> bool {
        self.source
            .may_select_static_in_declared_frame(name, span, context)
    }
    fn may_select_direct_source_read(
        &self,
        view: crate::ssa::SsaSourceView<'_>,
        symbol: crate::ssa::Symbol,
        name: &str,
        span: tcl_lexer::Span,
    ) -> bool {
        std::ptr::eq(view.function(), self.ssa)
            && self
                .source
                .may_select_direct_source_read(view, symbol, name, span)
    }
}

impl Analyser {
    /// W128 describes original table absence at a slot this Module mutates.
    /// Dynamic or ambiguous source ownership suppresses the diagnostic.
    pub(super) fn emit_w128_renamed_command(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::ir::Statement;

        let Some(context) = crate::command_binding::OriginalMissingCommandContext::for_module(
            &cu.ir_module,
            registry,
        ) else {
            return;
        };
        let cfg = &cu.top_level.cfg;
        // Reverse-postorder for deterministic diagnostic ordering.
        for block_id in cfg.reverse_postorder() {
            let Some(block) = cfg.blocks.get(&block_id) else {
                continue;
            };
            for stmt in &block.statements {
                let Statement::Call { command, span, .. } = stmt else {
                    continue;
                };
                let Some(tokens) = stmt.tokens() else {
                    continue;
                };
                // An auto-loadable sibling class is absent here and created
                // by the stock `unknown` handler's load, not renamed.
                if !context.missing_rebound_command(tokens) || self.is_workspace_autoloaded(command)
                {
                    continue;
                }
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W128,
                        *span,
                        format!(
                            "Command '{command}' has no binding at this call, and its name \
is renamed or deleted in this file."
                        ),
                        Severity::Warning,
                    ));
            }
        }
    }

    /// Narrow a whole-command span to its assignment-target token (the
    /// second word, `argv[1]`), returning that token's absolute span — or
    /// `None` when it can't be located, so callers fall back to the command
    /// span.  W211 / W220 anchor at the variable-name column, not the command
    /// start.  Re-lexes the command's own source slice (token-based) and
    /// takes the first non-separator word after the command name.
    fn narrow_to_assigned_name(&self, stmt_span: tcl_lexer::Span) -> Option<tcl_lexer::Span> {
        let base = stmt_span.start();
        let slice = source_slice(&self.source, stmt_span)?;
        let toks = tcl_lexer::Lexer::with_source_map(
            tcl_lexer::SourceMap::new(&slice),
            self.file_lexer_config().nested(),
        )
        .tokenise_all()
        .ok()?;
        let name = toks
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    tcl_lexer::TokenType::Sep
                        | tcl_lexer::TokenType::Eol
                        | tcl_lexer::TokenType::Comment
                )
            })
            .nth(1)?;
        Some(tcl_lexer::Span::new(
            name.span.start() + base,
            name.span.end() + base,
        ))
    }

    /// Narrow a whole-command span to the `$var` read token for *var*,
    /// returning that token's absolute span — or `None` when no matching
    /// `Var` token is found anywhere in the statement (the caller falls
    /// back to the command span).  W210 anchors at the variable read, not
    /// the command-start column.
    ///
    /// The read is frequently nested inside a command substitution
    /// (`set x [cmd $var]`, `if {[llength $var]} …`), so the scan descends
    /// into `Cmd` tokens' inner scripts rather than stopping at the
    /// top-level word walk.  Braced (`Str`) words are literal text — a
    /// `$var` inside one is not a read — so they are never descended.
    fn narrow_to_read_var(&self, stmt_span: tcl_lexer::Span, var: &str) -> Option<tcl_lexer::Span> {
        // The SSA name is already resolved: a leading dollar is literal.
        // Source sigils and the selected release's lexical extent belong to
        // the shared scanner, separately from the combined-name split owner.
        // naming.diagnostics.original-variable-name-anchor
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-variable-name-anchor.md
        /// First `$target` `Var` token within `slice` (whose absolute start
        /// is `abs_base`), descending into command-substitution contents.
        /// `depth` bounds pathological nesting.
        fn find_var(
            slice: &str,
            abs_base: u32,
            target: &str,
            config: tcl_lexer::LexerConfig,
            depth: u8,
        ) -> Option<tcl_lexer::Span> {
            if depth > 4 {
                return None;
            }
            let config = config.nested();
            let toks = tcl_lexer::Lexer::with_source_map(tcl_lexer::SourceMap::new(slice), config)
                .tokenise_all()
                .ok()?;
            for t in &toks {
                match t.kind {
                    tcl_lexer::TokenType::Var => {
                        let Ok(Some(reference)) = tcl_lexer::scan_var_ref(
                            slice.as_bytes(),
                            t.span.start() as usize,
                            config,
                        ) else {
                            continue;
                        };
                        let name = std::str::from_utf8(reference.name).ok()?;
                        let root =
                            crate::naming::split_element_ref(name).map_or(name, |(root, _)| root);
                        if root == target {
                            return Some(tcl_lexer::Span::new(
                                t.span.start() + abs_base,
                                u32::try_from(reference.next).ok()? + abs_base,
                            ));
                        }
                    }
                    // A `[…]` word: recurse into its inner script (the span
                    // covers `[inner` and excludes the closing `]`; the
                    // content starts past the opening bracket).
                    tcl_lexer::TokenType::Cmd => {
                        let content_start = t.span.start() as usize + usize::from(t.content_offset);
                        let content_end = t.span.end() as usize;
                        if let Some(inner) = slice.get(content_start..content_end)
                            && inner.contains('$')
                            && let Some(found) = find_var(
                                inner,
                                abs_base + t.span.start() + u32::from(t.content_offset),
                                target,
                                config,
                                depth + 1,
                            )
                        {
                            return Some(found);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        let target = crate::naming::split_element_ref(var).map_or(var, |(root, _)| root);
        let slice = source_slice(&self.source, stmt_span)?;
        find_var(&slice, stmt_span.start(), target, self.lexer_config(), 0)
    }
}

/// The suggestion name for an undefined-variable "; did you mean 'X'?"
/// suffix (W210): a case-insensitive twin among `defined_vars` wins at
/// any edit distance ([`find_case_mismatch`] — the established W210/W211/
/// W220 behaviour), otherwise the closest *other* defined name within
/// the length-scaled edit budget ([`crate::text::scaled_max_distance`],
/// so a short typo can't fish an unrelated short name). `None` when
/// nothing is close — the message then stays suffix-free.
fn undefined_var_suggestion<'a>(
    variable: &str,
    defined_vars: &'a HashSet<String>,
) -> Option<&'a str> {
    if let Some(similar) = find_case_mismatch(variable, defined_vars) {
        return Some(similar);
    }
    // The read variable can itself appear in `defined_vars` when it is
    // assigned later in the function (`puts $x; set x 1`) — never
    // suggest the typo as its own correction.
    crate::text::suggest_similar(
        variable,
        defined_vars
            .iter()
            .map(String::as_str)
            .filter(|name| *name != variable),
        1,
        crate::text::scaled_max_distance_strict(variable),
    )
    .first()
    .copied()
}

/// Find a defined variable that differs from `variable` only in case.
/// Returns the lexicographically smallest other-cased variant —
/// deterministic across runs.
fn find_case_mismatch<'a>(variable: &str, defined_vars: &'a HashSet<String>) -> Option<&'a str> {
    let lower = variable.to_lowercase();
    let mut matches: Vec<&str> = defined_vars
        .iter()
        .filter(|n| n.as_str() != variable && n.to_lowercase() == lower)
        .map(String::as_str)
        .collect();
    matches.sort_unstable();
    matches.into_iter().next()
}

fn function_declaration_flow(
    fu: &crate::compilation_unit::FunctionUnit,
    procedure: &crate::ir::Procedure,
    image: &tcl_lexer::SourceImage,
    registry: &tcl_registry::CommandRegistry,
) -> Option<std::sync::Arc<crate::command_binding::DeclarationFlowReport>> {
    fu.ssa.blocks.iter().find_map(|(&block, data)| {
        (0..data.statements.len())
            .chain(std::iter::once(usize::MAX))
            .find_map(|index| {
                let tokens = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index)
                    .source_tokens()?;
                let report = tokens
                    .source_binding
                    .as_ref()?
                    .declaration_flow_report(registry)?;
                report
                    .owns_original_procedure(procedure, image)
                    .then_some(report)
            })
    })
}

#[cfg(test)]
fn namespace_of(qualified_name: &str) -> String {
    tcl_syntax::naming::reported_qname_namespace(
        qualified_name,
        tcl_syntax::naming::ReportedQNameNamespacePurpose::LiteralPrefix,
    )
}

#[cfg(test)]
mod issue996_tests;

#[cfg(test)]
mod source_body_purpose_tests;

#[cfg(test)]
mod original_variable_anchor_tests;

#[cfg(test)]
mod original_matcher_admission_tests;

#[cfg(test)]
mod original_store_context_tests;

#[cfg(test)]
mod original_body_parser_config_tests;

#[cfg(test)]
mod original_condition_child_purpose_tests;

#[cfg(test)]
mod original_condition_extent_tests;

#[cfg(test)]
mod constructed_projection_tests;

#[cfg(test)]
mod original_formal_diagnostic_source_tests;
