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

//! Analysing a nested script in isolation: the synthetic replay procedure, its outer constants and the ambient command table.

use super::super::*;

fn build_isolated_taint_unit<H>(
    source: &str,
    context: &SinkWarningContext<'_, H>,
) -> Option<crate::compilation_unit::CompilationUnit> {
    if context.source.allows_nominal_metadata() {
        return Some(context.dialect.map_or_else(
            || crate::compilation_unit::CompilationUnit::build_for(source, context.registry, true),
            |profile| {
                crate::compilation_unit::CompilationUnit::build_for_profile(
                    source,
                    context.registry,
                    true,
                    profile,
                )
            },
        ));
    }
    let input = context.source.metadata?.source_analysis_input()?;
    Some(
        crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.registry,
                defer_top_level: true,
                config: context.source.config,
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
pub(in super::super) fn emit_isolated_script_warnings<
    S: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    script: &str,
    source_span: Span,
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
    context: &SinkWarningContext<'_, H>,
    command_head_ctx: Option<CommandHeadContext<'_>>,
) {
    let names: FxHashSet<String> = all_argument_candidates(script, context.source.config, ssa)
        .into_iter()
        .collect();
    let mut outer_uses = uses_for_names_at_statement(&names, ssa_stmt, ssa);
    if let Some(head_context) = command_head_ctx {
        expand_constant_outer_uses(&mut outer_uses, ssa_stmt, ssa, head_context);
    }
    let Some(preliminary_cu) = build_isolated_taint_unit(script, context) else {
        context.unavailable_replay(
            source_span,
            UnavailableTaintReplayReason::SourceMetadata,
            command_head_ctx,
        );
        return;
    };
    let entry = nested_script_entry_site(script, source_span, context.source.config);
    let command_setup = command_head_ctx.map_or_else(
        || Ok(String::new()),
        |head_context| {
            ambient_command_setup(
                &preliminary_cu,
                head_context.identities,
                entry,
                context.shadowed_builtins,
                context.registry,
                context.source.metadata,
                context.source.allows_nominal_metadata(),
            )
        },
    );
    let command_setup = match command_setup {
        Ok(setup) => setup,
        Err(reason) => {
            context.unavailable_replay(source_span, reason, command_head_ctx);
            return;
        }
    };
    let constant_setup = command_head_ctx.map_or_else(String::new, |head_context| {
        ambient_constant_setup(&outer_uses, head_context, ssa, context.source.config)
    });
    let (replay, seeds) = nested_script_replay(
        script,
        &command_setup,
        &constant_setup,
        &outer_uses,
        taints,
        ssa,
    );
    let Some(nested_cu) = build_isolated_taint_unit(&replay, context) else {
        context.unavailable_replay(
            source_span,
            UnavailableTaintReplayReason::SourceMetadata,
            command_head_ctx,
        );
        return;
    };
    let nested_report = taint_warning_report_for_cu_base_with_external_variable_seeds(
        &nested_cu,
        context.registry,
        context.dialect,
        Some(&seeds),
    );
    if matches!(
        &nested_report.replays,
        TaintReplayAnalysis::Unavailable { .. }
    ) && nested_report.replays.unavailable_replays().is_empty()
    {
        context.unavailable_replay(
            source_span,
            UnavailableTaintReplayReason::SourceMetadata,
            command_head_ctx,
        );
    }
    for residual in nested_report.replays.unavailable_replays() {
        context.unavailable_replay(source_span, residual.reason, command_head_ctx);
    }
    for mut warning in nested_report.warnings {
        warning.span = source_span;
        warning.replacement = None;
        warning.fixes.clear();
        if !warnings.contains(&warning) {
            warnings.push(warning);
        }
    }
}

/// The outer source offset at which a nested command substitution starts
/// executing: its first command's dispatch site, where the ambient command
/// table is the one the nested script observes. The substitution's own start
/// is the enclosing command's dispatch site instead.
fn nested_script_entry_site(
    script: &str,
    source_span: Span,
    config: tcl_lexer::LexerConfig,
) -> u32 {
    let at = source_span.start();
    let Some(base) = at
        .checked_add(1)
        .filter(|_| usize::try_from(source_span.len()).is_ok_and(|len| len == script.len() + 2))
    else {
        return at;
    };
    crate::segmenter::segment_commands_with_offset_and_config(script, base, config.nested())
        .first()
        .and_then(|command| command.argv.first())
        .map_or(at, |head| head.span.start())
}

/// Close the nested replay's outer bindings over variables referenced by
/// recovered constant values (`script` = `{$x}` needs both `script` and `x`).
/// The walk is bounded and uses the same exact reaching SSA versions as the
/// first-level names; traced or unprovable values simply stop that branch.
fn expand_constant_outer_uses(
    uses: &mut HashMap<Symbol, Version>,
    statement: &SsaStatement,
    ssa: &SsaFunction,
    context: CommandHeadContext<'_>,
) {
    const MAX_NAMES: usize = 128;
    for _ in 0..16 {
        let mut discovered = FxHashSet::default();
        for (&symbol, &version) in uses.iter() {
            let name = ssa.var_name(symbol);
            if context.module_traces.has_dynamic_variable_trace
                || context.module_traces.traced_variables.contains(name)
            {
                continue;
            }
            let Some(contributors) = crate::value_provenance::const_contributors_for_version(
                context.fu,
                name,
                version,
                context.fu.source_lexer_config(),
            ) else {
                continue;
            };
            for contributor in contributors {
                discovered.extend(all_argument_candidates(
                    &contributor.value,
                    context.fu.source_lexer_config(),
                    ssa,
                ));
            }
        }
        let additions = uses_for_names_at_statement(&discovered, statement, ssa);
        let before = uses.len();
        for (symbol, version) in additions {
            if uses.len() >= MAX_NAMES {
                return;
            }
            uses.entry(symbol).or_insert(version);
        }
        if uses.len() == before {
            return;
        }
    }
}

/// Put an isolated command substitution in a synthetic procedure and pass its
/// referenced outer values as ordinary parameters. This is the same proven
/// shape as callback replay: interprocedural entry seeding then reaches
/// structured `eval` bodies, while assignments inside the substitution retain
/// their normal CFG/SSA evaluation order.
fn nested_script_replay<S: std::hash::BuildHasher>(
    inner: &str,
    command_setup: &str,
    constant_setup: &str,
    uses: &HashMap<Symbol, Version>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ssa: &SsaFunction,
) -> (String, HashMap<String, TaintLattice>) {
    let collision_text = format!("{command_setup}{constant_setup}{inner}");
    let proc_name = fresh_nested_name(&collision_text, "__tcl_lsp_nested");
    let mut bindings: Vec<(String, Symbol, Version)> = uses
        .iter()
        .map(|(&symbol, &version)| (ssa.var_name(symbol).to_owned(), symbol, version))
        .collect();
    bindings.sort_by(|left, right| left.0.cmp(&right.0));

    let mut params = Vec::with_capacity(bindings.len());
    let mut invocation = Vec::with_capacity(bindings.len() + 1);
    invocation.push(proc_name.clone());
    let mut assignments = String::new();
    let mut globals = String::new();
    let mut seeds = HashMap::new();
    for (index, (name, symbol, version)) in bindings.into_iter().enumerate() {
        let param = fresh_nested_name(&collision_text, &format!("__tcl_lsp_arg_{index}"));
        let seed = fresh_nested_name(&collision_text, &format!("__tcl_lsp_seed_{index}"));
        params.push(param.clone());
        invocation.push(format!("${{::{seed}}}"));
        // C Tcl rejects reading an unset global, so the closed-world unit
        // would never enter the replay body; the seed carries the taint.
        let _ = writeln!(globals, "::set ::{seed} {{}}");
        let _ = writeln!(
            assignments,
            "set {} ${param}",
            tcl_syntax::list::list_element(&name)
        );
        if let Some(taint) = taints.get(&(symbol, version)).copied() {
            seeds.insert(param, taint);
            seeds.insert(crate::naming::rooted_member_key("::", &seed), taint);
        }
    }
    // Plumbing runs before the ambient table is materialised and uses global
    // spellings, so an outer user command named `set`, `proc`, or `catch`
    // cannot hijack the replay itself. Ambient facts then affect only the user
    // script they are meant to model.
    let body = format!("{assignments}{constant_setup}{command_setup}{inner}");
    let source = format!(
        "{globals}::proc {proc_name} {{{}}} {}\n{}",
        params.join(" "),
        tcl_syntax::list::list_element(&body),
        invocation.join(" "),
    );
    (source, seeds)
}

fn fresh_nested_name(source: &str, base: &str) -> String {
    if !source.contains(base) {
        return base.to_owned();
    }
    (1_u32..=u32::MAX)
        .map(|suffix| format!("{base}_{suffix}"))
        .find(|candidate| !source.contains(candidate))
        .unwrap_or_else(|| format!("{base}_{}", source.len()))
}

/// Materialise phase-correct outer constants referenced by a nested script.
/// This preserves a constant `$command` head and deferred script values such
/// as `set script {$x}; [eval $script]`. Traced variables and non-singleton
/// reaching constants abstain rather than guessing one runtime value.
fn ambient_constant_setup(
    uses: &HashMap<Symbol, Version>,
    context: CommandHeadContext<'_>,
    ssa: &SsaFunction,
    config: tcl_lexer::LexerConfig,
) -> String {
    let mut values = Vec::new();
    for (&symbol, &version) in uses {
        let name = ssa.var_name(symbol);
        if context.module_traces.has_dynamic_variable_trace
            || context.module_traces.traced_variables.contains(name)
        {
            continue;
        }
        let Some(contributors) = crate::value_provenance::const_contributors_for_version(
            context.fu, name, version, config,
        ) else {
            continue;
        };
        let Some(first) = contributors.first() else {
            continue;
        };
        if contributors
            .iter()
            .any(|contributor| contributor.value != first.value)
        {
            continue;
        }
        values.push((name.to_owned(), first.value.clone()));
    }
    values.sort_by(|left, right| left.0.cmp(&right.0));
    let mut setup = String::new();
    for (name, value) in values {
        let _ = writeln!(
            setup,
            "set {} {}",
            tcl_syntax::list::list_element(&name),
            tcl_syntax::list::list_element(&value),
        );
    }
    setup
}

/// Whether two written heads, both resolved from the global namespace, name
/// the same command (`:::puts` and `puts` both name `::puts`).
pub(in super::super) fn global_heads_name_one_command(left: &str, right: &str) -> bool {
    crate::naming::qualify("::", left) == crate::naming::qualify("::", right)
}

/// Materialise the enclosing command table for an isolated nested-script
/// analysis. The offset-keyed identity owner already composed imports,
/// aliases, renames, and user-proc rebindings; this adapter merely spells its
/// answer as analysis-only Tcl setup so the ordinary lowerer applies the same
/// identity to sources, sinks, argument roles, and nested bodies.
fn ambient_command_setup<H: std::hash::BuildHasher>(
    cu: &crate::compilation_unit::CompilationUnit,
    identities: &crate::realm::CommandBindingRealm,
    at: u32,
    shadowed_builtins: &HashSet<String, H>,
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    standalone: bool,
) -> Result<String, UnavailableTaintReplayReason> {
    if identities.is_empty() && shadowed_builtins.is_empty() {
        return Ok(String::new());
    }
    let mut heads = FxHashSet::default();
    for function in cu.analysable_body_function_units() {
        for block in function.ssa.blocks.values() {
            for statement in &block.statements {
                // A synthetic marker is an effect of the lowering, not a command.
                if statement
                    .statement
                    .tokens()
                    .is_some_and(|tokens| tokens.synthetic.is_some())
                {
                    continue;
                }
                match &statement.statement {
                    Statement::Call { command, .. } | Statement::Barrier { command, .. }
                        if !command.contains(['$', '[']) =>
                    {
                        heads.insert(command.clone());
                    }
                    _ => {}
                }
            }
        }
    }
    let mut heads: Vec<String> = heads.into_iter().collect();
    heads.sort();
    let mut aliases = String::new();
    let mut rebindings = Vec::new();
    for head in heads {
        let quoted_head = tcl_syntax::list::list_element(&head);
        if shadowed_builtins.contains(&head) {
            rebindings.push((head, format!("::proc {quoted_head} args {{}}\n")));
            continue;
        }
        match identities.binding_at(&head, at) {
            crate::realm::RealmBindingFact::Unchanged => {}
            crate::realm::RealmBindingFact::Rebound => {
                rebindings.push((head, format!("::proc {quoted_head} args {{}}\n")));
            }
            // Tcl refuses an alias onto itself and deletes the command, so a
            // head that still names its own target needs no ambient alias.
            crate::realm::RealmBindingFact::Command(target)
                if global_heads_name_one_command(target, &head) => {}
            crate::realm::RealmBindingFact::Command(target) => {
                let quoted_target = tcl_syntax::list::list_element(target);
                let _ = writeln!(
                    aliases,
                    "::interp alias {{}} {quoted_head} {{}} {quoted_target}"
                );
            }
        }
    }
    let constructors = ScaffoldConstructors::required(
        registry,
        metadata,
        standalone,
        !rebindings.is_empty(),
        !aliases.is_empty(),
    )?;
    // Keep the constructor that emits each rebinding available until its last use.
    rebindings.sort_by_key(|(head, _)| constructors.rebinding_priority(registry, metadata, head));
    for (_, rebinding) in rebindings {
        aliases.push_str(&rebinding);
    }
    Ok(aliases)
}

pub(in super::super) struct ScaffoldConstructors<'r> {
    procedure: Option<&'r tcl_registry::CommandSpec>,
    alias: Option<&'r tcl_registry::CommandSpec>,
}

impl<'r> ScaffoldConstructors<'r> {
    pub(in super::super) fn required(
        registry: &'r CommandRegistry,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        standalone: bool,
        needs_procedure: bool,
        needs_alias: bool,
    ) -> Result<Self, UnavailableTaintReplayReason> {
        if !needs_procedure && !needs_alias {
            return Ok(Self {
                procedure: None,
                alias: None,
            });
        }
        if metadata.is_some_and(|actual| !actual.matches_registry(registry))
            || (metadata.is_none() && !standalone)
        {
            return Err(UnavailableTaintReplayReason::SourceMetadata);
        }
        let query = metadata.map(|actual| actual.context().authoring_query());
        let procedure = if needs_procedure {
            let selected = registry
                .resolve_invocation("::proc", &["__scaffold_declaration", "args", ""], query)
                .ok_or(UnavailableTaintReplayReason::ProcedureConstructor)?;
            selected
                .state_transitions()
                .facts()
                .iter()
                .any(|fact| {
                    matches!(
                        fact.transition,
                        tcl_registry::StateTransition::CommandBinding(
                            tcl_registry::CommandBindingTransition::Define {
                                kind: tcl_registry::CommandBindingDefinitionKind::Procedure,
                                ..
                            }
                        )
                    )
                })
                .then_some(())
                .ok_or(UnavailableTaintReplayReason::ProcedureConstructor)?;
            Some(selected.authored_source_descriptors().command)
        } else {
            None
        };
        let alias = if needs_alias {
            let selected = registry
                .resolve_invocation(
                    "::interp",
                    &["alias", "", "__scaffold_alias", "", "__scaffold_target"],
                    query,
                )
                .ok_or(UnavailableTaintReplayReason::AliasConstructor)?;
            selected
                .state_transitions()
                .facts()
                .iter()
                .any(|fact| {
                    matches!(
                        fact.transition,
                        tcl_registry::StateTransition::CommandBinding(
                            tcl_registry::CommandBindingTransition::Alias { .. }
                        )
                    )
                })
                .then_some(())
                .ok_or(UnavailableTaintReplayReason::AliasConstructor)?;
            Some(selected.authored_source_descriptors().command)
        } else {
            None
        };
        Ok(Self { procedure, alias })
    }

    pub(in super::super) fn rebinding_priority(
        &self,
        registry: &CommandRegistry,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        head: &str,
    ) -> u8 {
        let query = metadata.map(|actual| actual.context().authoring_query());
        if let Some(selected) =
            registry.resolve_invocation(head, &["__scaffold_declaration", "args", ""], query)
        {
            let descriptor = selected.authored_source_descriptors().command;
            if self
                .procedure
                .is_some_and(|actual| std::ptr::eq(actual, descriptor))
            {
                return 2;
            }
        }
        if let Some(selected) = registry.resolve_invocation(
            head,
            &["alias", "", "__scaffold_alias", "", "__scaffold_target"],
            query,
        ) {
            let descriptor = selected.authored_source_descriptors().command;
            if self
                .alias
                .is_some_and(|actual| std::ptr::eq(actual, descriptor))
            {
                return 1;
            }
        }
        0
    }
}

fn uses_for_names_at_statement(
    names: &FxHashSet<String>,
    statement: &SsaStatement,
    ssa: &SsaFunction,
) -> HashMap<Symbol, Version> {
    let mut uses: HashMap<Symbol, Version> = statement
        .uses
        .iter()
        .filter(|(symbol, _)| names.contains(ssa.var_name(**symbol)))
        .map(|(&symbol, &version)| (symbol, version))
        .collect();
    if uses.len() == names.len() {
        return uses;
    }

    // A variable mentioned only in brace-quoted script text is not necessarily
    // a direct SSA use of the generic outer call. Recover its version at this
    // precise statement by replaying definitions from the containing block's
    // entry. Pointer identity is valid here: callers iterate the statements in
    // this same `SsaFunction`, rather than passing a clone.
    for block in ssa.blocks.values() {
        let mut versions = block.entry_versions.clone();
        for candidate in &block.statements {
            if std::ptr::eq(candidate, statement) {
                for name in names {
                    if let Some(symbol) = ssa.var_symbol(name) {
                        uses.entry(symbol)
                            .or_insert_with(|| versions.get(&symbol).copied().unwrap_or_default());
                    }
                }
                return uses;
            }
            versions.extend(
                candidate
                    .defs
                    .iter()
                    .map(|(&symbol, &version)| (symbol, version)),
            );
        }
    }
    uses
}
