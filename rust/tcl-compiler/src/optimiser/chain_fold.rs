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

//! O104 / O130 — write-only build-chain folding.
//!
//! Collapses a run of consecutive static writes to one variable into a
//! single `set`:
//!
//! ```tcl
//! set s ""        ;# →  set s "foobar"
//! append s foo
//! append s bar
//! ```
//! ```tcl
//! set l {}        ;# →  set l {a b c}
//! lappend l a
//! lappend l b c
//! ```
//!
//! `O104` folds string-concat (`append`) chains; `O130` folds list
//! (`lappend`) chains. The fold emits a `set` rewrite over the *last*
//! write and a paired deletion over each earlier one, all sharing one
//! group so they apply atomically.
//!
//! A chain may also anchor at an `append` / `lappend` whose own target the
//! existence rung proves `Unbound` immediately before it: the release
//! rule creates the cell in every release for both
//! commands, so the absent start folds through the value at the last write
//! exactly as an explicit `set var ""` would —
//! `lappend l a; lappend l b` folds to `set l {a b}`.
//!
//! ## Soundness gates
//!
//! - The writes must be **strictly consecutive** — no statement runs
//!   between them, so no intermediate value can be observed (the
//!   `var_observability` flow-sensitive read check is
//!   subsumed: a read between writes would be a non-write statement and
//!   ends the run).
//! - Every value word must be a static literal (`Esc`/`Str` single-token
//!   word), or a `$var` word the function's lattice proves constant at that
//!   statement — the chain then folds through the lattice value (`set s
//!   hello; set p again; append s $p` folds to `helloagain`); a `[cmd]`
//!   operand or an unproven `$var` ends the run.
//! - Which call extends the string and which the list is the registry's
//!   declaration — the resolved cell update — not a command's spelling.
//! - The variable must not **escape** (be aliased via
//!   `global`/`upvar`/`variable` or be under a `trace`) and must not be a
//!   cross-event iRules state variable — folding would drop a trace
//!   callback or a value a later scope / event observes.
//!
//! The actual function metadata and typed selected operation own the source
//! recipe. Executable grouped rewrites require retained Logical compatibility
//! and a separate setter-binding guard. Native layouts remain hints because
//! source pattern and quiet hazards do not certify Native erasure equivalence.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{CommandTokens, Script, Statement};
use crate::registry_invocation::InvocationMetadataContext;
use crate::var_observability::analyse_var_observability_with_metadata_context;

use super::helpers::literals::render_static_string_word;
use super::helpers::spans::{full_rewrite_span, statement_delete_rewrite_range};
use super::{Optimisation, PassContext};

/// Run the chain-fold pass over the whole compilation unit.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    // A dynamic variable-trace target (`trace add variable $n …`) means
    // *every* name is potentially traced, so no intermediate write is
    // provably unobserved anywhere in the module.
    if cu.ir_module.has_dynamic_variable_trace {
        return;
    }
    let mut cross = ctx.cross_event_vars.clone();
    // The whole-module trace fact stores the canonical (`::`-stripped)
    // spelling, so it also protects a chain whose target is spelled
    // unqualified while the trace names `::var` — the same
    // fact SCCP and O102 already consult.
    cross.extend(cu.ir_module.traced_variables.iter().cloned());
    let Some(registry) = ctx.registry else {
        return;
    };
    if ctx.source != cu.source
        || ctx.retained_metadata_context().is_none()
        || ctx.ir_module.is_none_or(|module| {
            module.source != cu.ir_module.source
                || module.source_metadata_input != cu.ir_module.source_metadata_input
                || module.lexer_config != cu.ir_module.lexer_config
        })
    {
        return;
    }
    fold_function(
        ctx,
        &cu.top_level,
        &cu.ir_module.top_level,
        &cross,
        registry,
        &cu.command_mutations,
    );
    for (qname, procedure) in &cu.ir_module.procedures {
        let Some(function) = cu.procedures.get(qname) else {
            continue;
        };
        fold_function(
            ctx,
            function,
            &procedure.body,
            &cross,
            registry,
            &cu.command_mutations,
        );
    }
}

/// A function's SSA statements by span, with where each sits, over its
/// shared lattice, so a `$var` value word resolves to the constant the
/// lattice proves at that statement.
#[derive(Default)]
struct FunctionLattice<'a> {
    unit: Option<&'a FunctionUnit>,
    statements: HashMap<(u32, u32), Option<LocatedStatement>>,
}

/// One SSA statement and its place in the function: its block and its
/// index there.
#[derive(Clone, Copy)]
struct LocatedStatement {
    block: crate::cfg::BlockId,
    index: usize,
}

impl<'a> FunctionLattice<'a> {
    fn of(unit: &'a FunctionUnit) -> Self {
        // A synthetic call the CFG builder emitted beside a host statement
        // shares the host's span; the host is the statement the chain reads.
        let mut statements = HashMap::new();
        for (&block, ssa_block) in &unit.ssa.blocks {
            for (index, statement) in ssa_block.statements.iter().enumerate() {
                if statement
                    .statement
                    .tokens()
                    .is_some_and(|tokens| tokens.synthetic.is_some())
                {
                    continue;
                }
                if unit.cfg.source_tokens_at(block, index).is_none() {
                    continue;
                }
                let span = unit.abs_span(statement.statement.span());
                statements
                    .entry((span.start(), span.end()))
                    .and_modify(|candidate| *candidate = None)
                    .or_insert(Some(LocatedStatement { block, index }));
            }
        }
        Self {
            unit: Some(unit),
            statements,
        }
    }

    /// The constant `name` holds at the statement spanning `span`, when
    /// the lattice proves one.
    fn constant_at(&self, span: tcl_lexer::Span, word: &crate::ir::WordExpr) -> Option<String> {
        let unit = self.unit?;
        let located = self.statements.get(&(span.start(), span.end()))?.as_ref()?;
        let crate::ir::WordExpr::Variable { spelling, source } = word else {
            return None;
        };
        let read = crate::ssa::SsaSourceView::at_statement(&unit.ssa, located.block, located.index)
            .read_reference(source, spelling)?;
        let key = (read.symbol, read.version?);
        if !unit.sccp.materialises(key) {
            return None;
        }
        let crate::analyses::LatticeValue::Const(value) = unit.sccp.values.get(&key)? else {
            return None;
        };
        super::helpers::literals::format_constant_with_policy(
            value,
            crate::tcl_expr_eval::FoldPolicy::for_profile(
                crate::tcl_expr_eval::leading_zero_is_octal(
                    unit.source_metadata_input()?.unit_profile(),
                ),
                Some(unit.source_metadata_input()?.unit_profile()),
            ),
        )
    }

    /// The existence fact `name`'s place holds where the statement
    /// spanning `span` reads it — the state its own read-modify-write
    /// observes (`incr` / `append` / `lappend` all read their target's
    /// existence before they write it), after every clobber since the
    /// version's definition (a non-lowered `switch` arm's clobber reaches
    /// the statement, not the version) — or
    /// `None` when the run computed none.
    fn existence_at_statement(
        &self,
        span: tcl_lexer::Span,
        name: &str,
    ) -> Option<tcl_registry::value_transfer::Existence> {
        let unit = self.unit?;
        let located = self.statements.get(&(span.start(), span.end()))?.as_ref()?;
        let symbol = unit.ssa.var_symbol_at(located.block, located.index, name)?;
        unit.sccp
            .existence_before(located.block, located.index, symbol)
    }
}

/// One function's actual command generation and complete source grammar.
#[derive(Clone, Copy)]
struct ChainSourceContext<'a> {
    registry: &'a CommandRegistry,
    metadata: InvocationMetadataContext<'a>,
    config: LexerConfig,
    mutations: &'a crate::command_binding::ModuleCommandMutations,
    lattice: &'a FunctionLattice<'a>,
}

fn fold_function(
    ctx: &mut PassContext<'_>,
    function: &FunctionUnit,
    script: &Script,
    cross_event: &HashSet<String>,
    registry: &CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
) {
    if function.dynamic_barrier_blocks_value_motion() {
        return;
    }
    let Some(module) = ctx.ir_module else {
        return;
    };
    let Some(metadata) = function.invocation_metadata_context_for_module(registry, module) else {
        return;
    };
    let mut protected =
        analyse_var_observability_with_metadata_context(&function.cfg, registry, Some(metadata))
            .escaping_var_names();
    protected.extend(cross_event.iter().cloned());
    let lattice = FunctionLattice::of(function);
    let semantics = ChainSourceContext {
        registry,
        metadata,
        config: function.source_lexer_config(),
        mutations,
        lattice: &lattice,
    };
    fold_script(ctx, script, &protected, semantics, 0);
}

/// Fold chains in `script`, then recurse into control-flow bodies (a
/// chain never crosses a control-flow boundary, so each body is folded
/// independently). `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn fold_script(
    ctx: &mut PassContext<'_>,
    script: &Script,
    protected: &HashSet<String>,
    semantics: ChainSourceContext<'_>,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return;
    }
    let stmts = &script.statements;
    let mut i = 0;
    while i < stmts.len() {
        if let Some(consumed) = try_fold_chain_at(ctx, script, i, protected, semantics) {
            i += consumed;
        } else {
            i += 1;
        }
    }
    for stmt in stmts {
        match stmt {
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    fold_script(ctx, &c.body, protected, semantics, depth + 1);
                }
                if let Some(b) = else_body {
                    fold_script(ctx, b, protected, semantics, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                fold_script(ctx, init, protected, semantics, depth + 1);
                fold_script(ctx, next, protected, semantics, depth + 1);
                fold_script(ctx, body, protected, semantics, depth + 1);
            }
            Statement::While { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Foreach { body, .. } => {
                fold_script(ctx, body, protected, semantics, depth + 1)
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                fold_script(ctx, body, protected, semantics, depth + 1);
                for h in handlers {
                    fold_script(ctx, &h.body, protected, semantics, depth + 1);
                }
                if let Some(fb) = finally_body {
                    fold_script(ctx, fb, protected, semantics, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        fold_script(ctx, b, protected, semantics, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    fold_script(ctx, b, protected, semantics, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// Static source operands of one selected variable operation.
enum Write {
    Set {
        var: String,
        value: String,
        setter: String,
    },
    Append {
        var: String,
        pieces: Vec<String>,
    },
    Lappend {
        var: String,
        elements: Vec<String>,
    },
}

fn write_var(write: &Write) -> &str {
    match write {
        Write::Set { var, .. } | Write::Append { var, .. } | Write::Lappend { var, .. } => var,
    }
}

#[derive(Clone, Copy)]
enum WriteKind {
    Set,
    Append,
    Lappend,
}

struct WriteLayout {
    kind: WriteKind,
    variable: usize,
    values: std::ops::Range<usize>,
}

/// The selected schema owns effective ordinals, including compound selectors
/// and captured alias operands. This layout is not an erasure or cell proof.
fn source_write_layout(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    semantics: ChainSourceContext<'_>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Option<WriteLayout> {
    use tcl_registry::SemanticOperationId::StructuredLowering;
    use tcl_registry::hooks::LoweringHookId;

    invocation.with_metadata_schema(semantics.registry, semantics.metadata, realm, |schema| {
        match invocation.facts.operation {
            StructuredLowering(LoweringHookId::Set) => {
                let assignments = schema.authored_source_assignment_arguments()?;
                let [(variable, Some(value))] = assignments.as_slice() else {
                    return None;
                };
                // The emitted setter has no compound-prefix recipe. A source
                // assignment layout cannot invent that independent edit input.
                if *variable != 0 || *value != 1 || invocation.arguments.len() != 2 {
                    return None;
                }
                Some(WriteLayout {
                    kind: WriteKind::Set,
                    variable: *variable,
                    values: *value..value.checked_add(1)?,
                })
            }
            StructuredLowering(LoweringHookId::AppendOrLappend) => {
                let (kind, layout) = schema.authored_source_append_arguments().map_or_else(
                    || {
                        schema
                            .authored_source_list_append_arguments()
                            .map(|layout| (WriteKind::Lappend, layout))
                    },
                    |layout| Some((WriteKind::Append, layout)),
                )?;
                (!layout.values.is_empty()).then_some(WriteLayout {
                    kind,
                    variable: layout.variable,
                    values: layout.values,
                })
            }
            _ => None,
        }
    })
}

/// Typed source recipe only. Frozen values of substitutions cannot stand in
/// for static operands because removing a read could remove an observer.
fn classify_write(
    tokens: &CommandTokens,
    semantics: ChainSourceContext<'_>,
    span: tcl_lexer::Span,
) -> Option<Write> {
    let binding = tokens.source_binding.as_ref()?;
    if binding
        .original_lexer_config_for_tokens(tokens)?
        .normalized()
        != semantics.config.normalized()
    {
        return None;
    }
    let invocation = if semantics.metadata.permits_logical_source_names() {
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            semantics.registry,
            semantics.metadata,
            tokens,
        )?
    } else {
        crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
            semantics.registry,
            Some(semantics.metadata),
            tokens,
        )?
    };
    if !invocation.facts.arg_roles_complete
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    let layout = source_write_layout(&invocation, semantics, binding.invocation_realm()?)?;
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&semantics.config);
    let var =
        invocation
            .effective
            .argument_literal(layout.variable, semantics.config.escapes, rules)?;
    // Complete array operands need their own index/cell proof. A malformed
    // final parenthesis is an unchanged scalar, including any literal '$'.
    if tcl_syntax::naming::split_element_ref(&var).is_some() {
        return None;
    }
    let values = layout
        .values
        .map(|index| {
            invocation
                .effective
                .argument_literal(index, semantics.config.escapes, rules)
                .or_else(|| {
                    semantics
                        .metadata
                        .permits_logical_source_names()
                        .then_some(())?;
                    semantics
                        .lattice
                        .constant_at(span, invocation.effective.words.get(index + 1)?)
                })
        })
        .collect::<Option<Vec<_>>>()?;
    match layout.kind {
        WriteKind::Set => Some(Write::Set {
            var,
            value: values.into_iter().next()?,
            setter: invocation.facts.canonical_command.clone(),
        }),
        WriteKind::Append => Some(Write::Append {
            var,
            pieces: values,
        }),
        WriteKind::Lappend => Some(Write::Lappend {
            var,
            elements: values,
        }),
    }
}

fn replacement_setter(semantics: ChainSourceContext<'_>) -> Option<String> {
    semantics
        .registry
        .command_names_for_semantic_operation(
            tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            ),
        )
        .find(|name| {
            semantics.mutations.trusts(name)
                && semantics
                    .metadata
                    .context()
                    .resolve_spec(semantics.registry, name)
                    .is_some()
        })
        .map(str::to_owned)
}

/// Attempt to fold a write-chain starting at `script.statements[start]`. Returns the
/// number of statements consumed (the run length) when a fold fires, else
/// `None`.
fn try_fold_chain_at(
    ctx: &mut PassContext<'_>,
    script: &Script,
    start: usize,
    protected: &HashSet<String>,
    semantics: ChainSourceContext<'_>,
) -> Option<usize> {
    let stmts = &script.statements;
    let original = script.retained_source_tokens_for_statement(&stmts[start])?;
    let write = classify_write(original, semantics, stmts[start].span())?;
    let absent = !matches!(write, Write::Set { .. })
        && semantics
            .lattice
            .existence_at_statement(stmts[start].span(), write_var(&write))
            == Some(tcl_registry::value_transfer::Existence::Unbound);
    let (var, mut chain_value, mut elements, setter) = match write {
        Write::Set { var, value, setter } => (var, value, None, setter),
        Write::Append { var, pieces } if absent => {
            (var, pieces.concat(), None, replacement_setter(semantics)?)
        }
        Write::Lappend { var, elements } if absent => (
            var,
            String::new(),
            Some(elements),
            replacement_setter(semantics)?,
        ),
        Write::Append { .. } | Write::Lappend { .. } => return None,
    };
    if !semantics.mutations.trusts(&setter) {
        return None;
    }
    let replacement_head = tcl_syntax::naming::qualify("::", &setter);
    let mut writes = vec![start];

    let mut j = start + 1;
    while j < stmts.len() {
        match script
            .retained_source_tokens_for_statement(&stmts[j])
            .and_then(|tokens| classify_write(tokens, semantics, stmts[j].span()))
        {
            Some(Write::Append { var: v, pieces }) if v == var && elements.is_none() => {
                for p in pieces {
                    chain_value.push_str(&p);
                }
                writes.push(j);
                j += 1;
            }
            Some(Write::Lappend {
                var: v,
                elements: els,
            }) if v == var => {
                if elements.is_none() {
                    // First lappend after the set — reinterpret the current
                    // string value as a list (bail if it is not one).
                    let rules =
                        tcl_syntax::word_rules::WordValueRules::from_config(&semantics.config);
                    let Ok(base) = rules.split_list(&chain_value) else {
                        break;
                    };
                    elements = Some(base.into_iter().map(std::borrow::Cow::into_owned).collect());
                }
                if let Some(list) = elements.as_mut() {
                    list.extend(els);
                }
                writes.push(j);
                j += 1;
            }
            // A separate unobserved literal write stays in place. An
            // escaped/traced interleaved target can observe the accumulator
            // through its callback, so it ends this source chain.
            Some(other)
                if write_var(&other) != var
                    && !protected.contains(write_var(&other))
                    && !protected.contains(write_var(&other).trim_start_matches("::")) =>
            {
                j += 1;
            }
            _ => break,
        }
    }

    // The whole-module traced-variable fact is stored `::`-stripped (see
    // `populate_variable_trace_facts`), so a `::`-qualified chain target is
    // checked under that canonical spelling too.
    if writes.len() < 2
        || protected.contains(&var)
        || protected.contains(var.trim_start_matches("::"))
    {
        return None;
    }
    let var_word = render_static_string_word(&var).or_else(|| {
        tcl_syntax::backslash::literal_quoted_source_fragment(&var, semantics.config.escapes)
            .map(|fragment| format!("\"{fragment}\""))
    })?;

    let (code, fold_msg, dead_msg, rendered) = if let Some(els) = &elements {
        (
            DiagCode::O130,
            "Fold write-only list build chain",
            "Remove dead intermediate list write",
            render_list_word(els, ctx.dialect)?,
        )
    } else {
        (
            DiagCode::O104,
            "Fold write-only string build chain",
            "Remove dead intermediate string write",
            render_static_string_word(&chain_value)?,
        )
    };

    let source = ctx.source;
    let group = ctx.alloc_group();

    let last = *writes.last().unwrap();
    let last_span = full_rewrite_span(source, stmts[last].span());
    let mut fold = Optimisation::new(
        code,
        fold_msg,
        last_span,
        format!("{replacement_head} {var_word} {rendered}"),
    );
    // A selected Native layout is source advice; no chain result/erasure
    // owner has certified changing its original objects or intermediate writes.
    fold.hint_only = !semantics.metadata.permits_logical_source_names();
    fold.group = Some(group);
    ctx.report(fold);

    for &w in &writes[..writes.len() - 1] {
        let full = full_rewrite_span(source, stmts[w].span());
        let next_start = stmts.get(w + 1).map(|s| s.span().start() as usize);
        let del_span = statement_delete_rewrite_range(source, full, next_start);
        let mut del = Optimisation::new(code, dead_msg, del_span, "");
        del.hint_only = !semantics.metadata.permits_logical_source_names();
        del.group = Some(group);
        ctx.report(del);
    }

    Some(writes.len())
}

/// Render `elements` as the single `set` value-word that recreates the
/// list the target builds — join into its canonical list, then quote that
/// as one word — or `None` where the rendering is release-dependent (a
/// first element that starts with `#`, under a profile naming no release).
/// Under 8.4 `lappend l # b` builds `# b`, so the word is `{# b}`; from 8.5
/// it builds `{#} b`.
fn render_list_word(
    elements: &[String],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<String> {
    let list = tcl_registry::value_transfer::TargetSemantics::of(dialect).render_list(elements)?;
    Some(tcl_syntax::list::list_element(&list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interprocedural::InterproceduralAnalysis;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let registry = registry();
        let cu = CompilationUnit::build_for_profile(
            source,
            &registry,
            false,
            tcl_dialect::DialectProfile::plain_tcl(),
        );
        run_unit(&cu, &registry)
    }

    fn run_unit(cu: &CompilationUnit, registry: &CommandRegistry) -> Vec<Optimisation> {
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.registry = Some(registry);
        ctx.ir_module = Some(&cu.ir_module);
        ctx.command_mutations.clone_from(&cu.command_mutations);
        run(&mut ctx, cu);
        ctx.optimisations
    }

    /// Apply every grouped O104/O130 rewrite to `source` (reverse offset
    /// order so earlier edits don't shift later spans) and return the
    /// rewritten text.
    fn apply(source: &str) -> String {
        let mut opts: Vec<Optimisation> = run_pass(source)
            .into_iter()
            .filter(|o| o.code == DiagCode::O104 || o.code == DiagCode::O130)
            .collect();
        opts.sort_by_key(|o| std::cmp::Reverse(o.span.start()));
        let mut out = source.to_owned();
        for o in opts {
            out.replace_range(
                o.span.start() as usize..o.span.end() as usize,
                &o.replacement,
            );
        }
        out
    }

    /// The original 1000-level source must return without changing its depth.
    /// The larger test stack accommodates full lexical/source traversal.
    #[test]
    fn deeply_nested_if_survives_fold_script() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("set s \"\"\nappend s foo\nappend s bar\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = run_pass(&src);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    fn logical_unit_at(
        source: &str,
        registry: &CommandRegistry,
        availability: &str,
    ) -> CompilationUnit {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for(availability)
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let unit = CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: LexerConfig::from_grammar(profile.grammar),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            context,
        );
        assert!(
            unit.top_level
                .source_metadata_input()
                .unwrap()
                .has_logical_source_name_context()
        );
        unit
    }

    #[test]
    fn chain_uses_authentic_alias_captures_and_moved_append() {
        // naming.optimiser.original-chain-write-metadata
        // docs/design/analysis/name-resolution-proofs/optimiser-original-chain-write-metadata.md
        for (source, code, expected) in [
            (
                "interp alias {} extend {} append s PRE; set s {}; extend A; extend B",
                DiagCode::O104,
                "::set s PREAPREB",
            ),
            (
                "rename append moved; set s {}; moved s A; moved s B",
                DiagCode::O104,
                "::set s AB",
            ),
            (
                "interp alias {} extend {} lappend l {PRE VALUE}; set l {}; extend A; extend B",
                DiagCode::O130,
                "::set l {{PRE VALUE} A {PRE VALUE} B}",
            ),
        ] {
            let opts = run_pass(source);
            assert!(
                opts.iter()
                    .any(|opt| opt.code == code && opt.replacement == expected && !opt.hint_only),
                "{source}: {opts:?}"
            );
        }
        for source in [
            "proc append args {return CUSTOM}; set s {}; append s A; append s B",
            "interp alias {} extend {} append s; rename append {}; proc append args {return CUSTOM}; set s {}; extend A; extend B",
            "rename set moved_set; moved_set s {}; append s A; append s B",
            "namespace eval N {proc append args {return CUSTOM}; set s {}; append s A; append s B}",
        ] {
            assert!(
                run_pass(source)
                    .iter()
                    .all(|opt| opt.code != DiagCode::O104),
                "{source}"
            );
        }
    }

    #[test]
    fn chain_preserves_literal_scalar_names_and_closed_array_refusal() {
        // naming.optimiser.original-chain-write-metadata
        // docs/design/analysis/name-resolution-proofs/optimiser-original-chain-write-metadata.md
        for (name, expected) in [
            ("$scalar(open", "{$scalar(open}"),
            ("café", "{café}"),
            ("a b", "{a b}"),
            (r"a\b", r#""a\\b""#),
        ] {
            let source = format!("set {{{name}}} {{}}; append {{{name}}} A; append {{{name}}} B");
            let opts = run_pass(&source);
            assert!(
                opts.iter().any(|opt| opt.code == DiagCode::O104
                    && opt.replacement == format!("::set {expected} AB")),
                "{source}: {opts:?}"
            );
        }
        for source in [
            "set {a(k)} {}; append {a(k)} A; append {a(k)} B",
            "set {$s} {}; append s A; append s B",
            "set {scalar(open} {}; append scalar A; append scalar B",
            // A known value of the receiver's variable is not an erasure
            // receipt for its original read while forming that receiver.
            "set receiver s; set $receiver {}; append $receiver A; append $receiver B",
        ] {
            assert!(
                run_pass(source)
                    .iter()
                    .all(|opt| opt.code != DiagCode::O104),
                "{source}"
            );
        }
    }

    #[test]
    fn chain_requires_supplied_function_and_module_metadata() {
        // naming.optimiser.original-chain-write-metadata
        // docs/design/analysis/name-resolution-proofs/optimiser-original-chain-write-metadata.md
        let registry = registry();
        let source = "set s {}; append s A; append s B";
        let original = logical_unit_at(source, &registry, "tcl9.0");
        assert!(
            run_unit(&original, &registry)
                .iter()
                .any(|opt| opt.code == DiagCode::O104)
        );
        let mut missing_function = original.clone();
        missing_function.top_level.source_metadata_input = None;
        assert!(run_unit(&missing_function, &registry).is_empty());
        let mut stale_grammar = original.clone();
        stale_grammar.top_level.source_config.expand_syntax =
            !stale_grammar.top_level.source_config.expand_syntax;
        assert!(run_unit(&stale_grammar, &registry).is_empty());
        let mut missing_module = original.clone();
        missing_module.ir_module.source_metadata_input = None;
        assert!(run_unit(&missing_module, &registry).is_empty());
        let mut foreign = original.clone();
        let input = original.top_level.source_metadata_input().unwrap();
        foreign.top_level.source_metadata_input =
            Some(crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                tcl_registry::model::ingress::resolve_environment("tcl9.0")
                    .default_context_registry(),
                input.lexer_config(),
            ));
        assert!(run_unit(&foreign, &registry).is_empty());
        // Both inputs individually belong to this command store. A different
        // availability owner must still not be borrowed across the function/module join.
        let mut different_owner = original.clone();
        different_owner.top_level.source_metadata_input =
            Some(crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                std::sync::Arc::new(
                    tcl_registry::model::ingress::static_context_for("tcl8.4")
                        .with_command_store(registry.snapshot().shared_registry()),
                ),
                input.lexer_config(),
            ));
        assert!(
            different_owner
                .top_level
                .invocation_metadata_context(&registry)
                .is_some()
        );
        assert!(run_unit(&different_owner, &registry).is_empty());
        let mut detached = PassContext::new(&original.source, InterproceduralAnalysis::default());
        run(&mut detached, &original);
        assert!(detached.optimisations.is_empty());
    }

    #[test]
    fn chain_requires_original_statement_tape_after_structured_lowering() {
        // naming.optimiser.original-chain-write-metadata
        // docs/design/analysis/name-resolution-proofs/optimiser-original-chain-write-metadata.md
        // The source sidecar survives typed lowering without becoming a Native receipt.
        let registry = registry();
        let source = "set s {}; append s A; append s B";
        let original = logical_unit_at(source, &registry, "tcl9.0");
        let script = &original.ir_module.top_level;
        assert!(script.statements[0].tokens().is_none());
        assert!(
            script
                .retained_source_tokens_for_statement(&script.statements[0])
                .is_some()
        );
        assert!(
            run_unit(&original, &registry)
                .iter()
                .any(|opt| opt.code == DiagCode::O104)
        );
        let mut missing = original;
        missing.ir_module.top_level.command_binding_sites = Default::default();
        assert!(run_unit(&missing, &registry).is_empty());
    }

    #[test]
    fn chain_uses_retained_availability_over_catalogue_generation() {
        // naming.optimiser.original-chain-write-metadata
        // docs/design/analysis/name-resolution-proofs/optimiser-original-chain-write-metadata.md
        let mut registry = registry();
        let append = registry.get("append").unwrap().clone();
        registry.insert(tcl_registry::CommandSpec {
            surface: registry.get("dict").unwrap().surface,
            ..append
        });
        let source = "set s {}; append s A; append s B";
        let available = logical_unit_at(source, &registry, "tcl9.0");
        let unavailable = logical_unit_at(source, &registry, "tcl8.4");
        assert!(
            run_unit(&available, &registry)
                .iter()
                .any(|opt| opt.code == DiagCode::O104)
        );
        assert!(run_unit(&unavailable, &registry).is_empty());
    }

    #[test]
    fn native_chain_layout_never_grants_erasure() {
        // naming.optimiser.original-chain-write-metadata
        // docs/design/analysis/name-resolution-proofs/optimiser-original-chain-write-metadata.md
        // Original API receipts only; this is not a native execution comparison.
        let source = "set s {}; append s A; append s B";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = tcl_registry::model::ingress::static_context_for(dialect);
            let registry = context.commands();
            let profile = registry.profile().unwrap();
            let entry = crate::command_binding::SourceAnalysisEntry {
                native_entry: Some(std::sync::Arc::new(
                    crate::environment_ingress::captured_native_entry(profile),
                )),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            };
            let config = LexerConfig::from_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::resolve_environment(dialect)
                    .default_context_registry(),
                config,
            );
            let unit = CompilationUnit::build_with_analysis_input(
                source,
                crate::compilation_unit::UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config,
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                Some(&entry),
                &input,
            );
            assert!(
                !unit
                    .top_level
                    .source_metadata_input()
                    .unwrap()
                    .has_logical_source_name_context()
            );
            if dialect == "tcl8.6" {
                let function = &unit.top_level;
                let statement = &unit.ir_module.top_level.statements[0];
                let lattice = FunctionLattice::of(function);
                let semantics = ChainSourceContext {
                    registry,
                    metadata: unit
                        .top_level
                        .invocation_metadata_context(registry)
                        .unwrap(),
                    config: unit.top_level.source_lexer_config(),
                    mutations: &unit.command_mutations,
                    lattice: &lattice,
                };
                assert!(
                    classify_write(
                        unit.ir_module
                            .top_level
                            .retained_source_tokens_for_statement(statement)
                            .unwrap(),
                        semantics,
                        statement.span(),
                    )
                    .is_some(),
                    "the positive Native API layout remains distinct from erasure admission"
                );
            }
            let opts = run_unit(&unit, registry);
            assert!(opts.iter().all(|opt| opt.hint_only), "{dialect}: {opts:?}");
            assert_eq!(super::super::apply_optimisations(source, &opts), source);
        }
    }

    #[test]
    fn traced_interleaved_write_ends_chain() {
        // naming.optimiser.original-chain-write-metadata
        // docs/design/analysis/name-resolution-proofs/optimiser-original-chain-write-metadata.md
        let source = "proc observe args {puts $::s}; trace add variable ::t write observe; set s {}; set t A; append s B";
        assert!(
            run_pass(source)
                .iter()
                .all(|opt| opt.code != DiagCode::O104)
        );
    }

    #[test]
    fn string_chain_folds_to_single_set() {
        let opts = run_pass("set s \"\"\nappend s foo\nappend s bar");
        let fold = opts
            .iter()
            .find(|o| o.code == DiagCode::O104 && o.replacement.starts_with("::set"))
            .expect("expected an O104 fold");
        assert_eq!(fold.replacement, "::set s foobar");
        // One fold + two deletions, all in one group.
        let o104: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O104).collect();
        assert_eq!(o104.len(), 3);
        let groups: HashSet<_> = o104.iter().filter_map(|o| o.group).collect();
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn string_chain_rewrite_applies_cleanly() {
        assert_eq!(
            apply("set s \"\"\nappend s foo\nappend s bar"),
            "::set s foobar"
        );
        assert_eq!(
            apply("set s start\nappend s _mid\nappend s _end"),
            "::set s start_mid_end",
        );
    }

    #[test]
    fn chain_continues_past_interleaved_literal_set() {
        // `set t 1` writes a *different* variable with a static literal, so
        // the build chain folds across it (precise-flow); the interleaved
        // statement stays in place.
        assert_eq!(
            apply("set s \"\"\nset t 1\nappend s foo\nappend s bar"),
            "set t 1\n::set s foobar",
        );
    }

    #[test]
    fn chain_breaks_on_interleaved_dynamic_statement() {
        // A `puts $s` reads the accumulator — `classify_write` returns
        // None for it, so the chain must NOT fold across it (only the
        // trailing two appends, which is a fresh sub-chain of length < 2).
        let opts = run_pass("set s \"\"\nputs $s\nappend s foo\nappend s bar");
        let fold = opts
            .iter()
            .find(|o| o.code == DiagCode::O104 && o.replacement.starts_with("::set s"));
        // The `set s ""; puts $s` prefix breaks; the two trailing appends
        // have no anchoring `set`, so no fold fires.
        assert!(
            fold.is_none(),
            "must not fold across a reader, got {opts:?}"
        );
    }

    #[test]
    fn list_chain_folds_with_lappend() {
        assert_eq!(
            apply("set l {}\nlappend l a\nlappend l b c"),
            "::set l {a b c}"
        );
    }

    #[test]
    fn list_chain_quotes_spacey_elements() {
        // An element containing a space must be re-quoted as a list word.
        assert_eq!(
            apply("set l {}\nlappend l {a b}\nlappend l c"),
            "::set l {{a b} c}"
        );
    }

    /// A `$var` piece the lattice proves constant folds through the value
    /// at that statement: the non-consecutive O104 chain, and its
    /// exact-value twin (#2052).
    #[test]
    fn var_piece_proven_by_the_lattice_folds_the_chain() {
        let out = apply("set s hello\nset p again\nappend s $p\nputs $s\n");
        assert_eq!(out, "set p again\nset s helloagain\nputs $s\n");
        let out = apply("set s hello\nset p { again}\nappend s $p\nputs $s\n");
        assert!(
            out.contains("hello again") && !out.contains("append"),
            "the leading space is kept: {out:?}"
        );
        let out = apply("set l {}\nset e {b c}\nlappend l a $e\nputs $l\n");
        assert_eq!(out, "set e {b c}\nset l {a {b c}}\nputs $l\n");
    }

    /// An unproven `$var` piece still ends the run.
    #[test]
    fn unproven_var_piece_ends_the_run() {
        let src = "set s hello\nappend s $p\nappend s x\nputs $s\n";
        let opts = run_pass(src);
        assert!(
            !opts.iter().any(|o| o.code == DiagCode::O104),
            "an unproven `$p` cannot be folded: {opts:?}"
        );
    }

    /// The write chain is classified by the resolved cell update, so the
    /// qualified spelling of the same command extends it too.
    /// [`run_pass`] under the module's own command-trust fact, as the
    /// optimiser entry points install it.
    fn run_pass_under_the_module_trust(source: &str) -> Vec<Optimisation> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.command_mutations.clone_from(&cu.command_mutations);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    /// A typed assignment keeps no head of its own, so the chain asks every
    /// registry spelling of the assignment operation: with `proc set` in
    /// scope, `set s foo` never assigns (tclsh 8.4.20 – 9.1b0: the chain's
    /// `puts $s` prints `bar`, where the fold would store `foobar`). A
    /// shadowed call head declines the same way, and the controls fold.
    #[test]
    fn a_shadowed_head_anchors_or_extends_no_chain() {
        let chain = "set s foo\nappend s bar\nputs $s\n";
        let folds = |src: &str| {
            run_pass_under_the_module_trust(src)
                .iter()
                .any(|o| o.code == DiagCode::O104 || o.code == DiagCode::O130)
        };
        assert!(folds(chain), "the unshadowed chain folds");
        assert!(
            !folds(&format!("proc set {{args}} {{return ZZZ}}\n{chain}")),
            "a shadowed `set` anchors no chain"
        );
        assert!(
            !folds(&format!("proc append {{args}} {{return ZZZ}}\n{chain}")),
            "a shadowed `append` extends no chain"
        );
    }

    #[test]
    fn qualified_spelling_of_the_cell_update_extends_the_chain() {
        let out = apply("set s foo\n::append s bar\nputs $s\n");
        assert_eq!(out, "set s foobar\nputs $s\n");
    }

    #[test]
    fn single_write_does_not_fold() {
        let opts = run_pass("set s \"\"\nappend s foo");
        // set + one append = 2 writes → folds (the chain needs >= 2 writes).
        assert!(opts.iter().any(|o| o.code == DiagCode::O104));
        // But a lone set is not a chain.
        let opts = run_pass("::set s foo");
        assert!(opts.iter().all(|o| o.code != DiagCode::O104));
    }

    #[test]
    fn dynamic_value_ends_the_run() {
        // `append s $x` is dynamic — the chain stops before it, and the
        // `set; append foo` prefix (2 writes) still folds.
        let opts = run_pass("set s \"\"\nappend s foo\nappend s $x");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O104 && o.replacement == "::set s foo")
        );
    }

    #[test]
    fn intervening_read_ends_the_run_but_folds_prefix() {
        // `puts $s` reads the accumulator, ending the run after `append s
        // foo`. The consecutive prefix still folds to `set s foo` (the same
        // value `puts` observes); the trailing `append s bar` is not folded
        // into it. Matches `finish_chain`-on-read behaviour.
        let opts = run_pass("set s \"\"\nappend s foo\nputs $s\nappend s bar");
        let folds: Vec<&str> = opts
            .iter()
            .filter(|o| o.code == DiagCode::O104 && o.replacement.starts_with("::set"))
            .map(|o| o.replacement.as_str())
            .collect();
        assert_eq!(folds, ["::set s foo"], "got {opts:?}");
    }

    #[test]
    fn escaping_global_var_not_folded() {
        // `s` is global — every write is visible to other scopes.
        let opts = run_pass("proc ::f {} { global s\nset s \"\"\nappend s foo\nappend s bar }");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O104),
            "global var must not fold, got {opts:?}",
        );
    }

    #[test]
    fn cross_event_var_not_folded() {
        let src = "set s \"\"\nappend s foo\nappend s bar";
        let registry = registry();
        let cu = CompilationUnit::build_for_profile(
            src,
            &registry,
            false,
            tcl_dialect::DialectProfile::plain_tcl(),
        );
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.registry = Some(&registry);
        ctx.ir_module = Some(&cu.ir_module);
        ctx.command_mutations.clone_from(&cu.command_mutations);
        ctx.cross_event_vars.insert("s".to_owned());
        run(&mut ctx, &cu);
        assert!(ctx.optimisations.iter().all(|o| o.code != DiagCode::O104));
    }

    #[test]
    fn folds_inside_proc_body() {
        assert_eq!(
            apply("proc ::f {} {\n    set s \"\"\n    append s a\n    append s b\n}"),
            "proc ::f {} {\n    ::set s ab\n}",
        );
    }

    /// A computed variable name between the writes can hit the
    /// accumulator under a spelling `classify_write` cannot see (`f acc`
    /// returns `zzz b` in tclsh; the fold's `a b` would be a miscompile), so
    /// the whole proc abstains from O104 / O130.
    #[test]
    fn dynamic_name_write_blocks_chain_fold() {
        let src = "proc ::f {name} { set acc {}; lappend acc a; set $name zzz; lappend acc b; return $acc }";
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .all(|o| o.code != DiagCode::O130 && o.code != DiagCode::O104),
            "dynamic-name proc must not chain-fold, got {opts:?}",
        );
    }

    /// A write trace observes every intermediate store. The
    /// module fact records the trace target `::acc` canonically as `acc`, so
    /// the unqualified chain over `acc` must be protected too.
    #[test]
    fn traced_variable_blocks_chain_fold() {
        let src = "proc onw {a b c} { puts trace }\ntrace add variable ::acc write ::onw\nset acc {}\nlappend acc a\nlappend acc b";
        let opts = run_pass(src);
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O130),
            "traced accumulator must not chain-fold, got {opts:?}",
        );
    }

    /// A dynamic trace target (`trace add variable $n …`)
    /// makes every name potentially traced, so no chain folds at all.
    #[test]
    fn dynamic_trace_target_blocks_chain_fold() {
        let src = "proc onw {a b c} { puts trace }\ntrace add variable $n write ::onw\nset acc {}\nlappend acc a\nlappend acc b";
        let opts = run_pass(src);
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O130),
            "dynamic trace target must block every chain fold, got {opts:?}",
        );
    }
}
