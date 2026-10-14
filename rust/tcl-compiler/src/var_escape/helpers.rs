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

//! Variable-name predicates and original invocation inventories for escape transfer.

#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use std::sync::OnceLock;

use tcl_registry::CommandRegistry;
use tcl_registry::InvocationFacts;
use tcl_registry::prelude::Traits;

use crate::ir::{CommandTokens, Statement};
use crate::registry_invocation::ResolvedStatementInvocation;

/// Memoised set of commands carrying the registry's
/// [`Traits::FIRST_ARG_VARNAME`] trait — the commands whose first arg is
/// the variable *name* (`set` / `incr` / `append` / `lappend` / `lset` /
/// `unset`). Used to detect dynamic-name forms like `set $n value`. Sourced
/// from the registry; cached once because the set
/// is dialect-agnostic (all entries are core Tcl commands).
#[cfg(test)]
fn name_first_set() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        CommandRegistry::build_default()
            .commands_with_trait(Traits::FIRST_ARG_VARNAME)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

/// Memoised set of commands carrying the registry's
/// [`Traits::FRAMELESS_RUNTIME`] trait — the audited allow-list of
/// commands whose codegen always uses a dedicated runtime helper (or
/// a structured IR node) and never falls back to the interpreter, so
/// a call to one needs no runtime frame in the callee.
///
/// Sourced from the registry (the single source of truth) rather than
/// a hardcoded copy; cached once because the set is dialect-agnostic
/// (all entries are core Tcl commands present in `build_default`).
#[cfg(test)]
fn frameless_runtime_set() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        CommandRegistry::build_default()
            .commands_with_trait(Traits::FRAMELESS_RUNTIME)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

/// Explicit standalone catalogue for raw fixture adapters.
#[cfg(test)]
pub(crate) fn default_registry() -> &'static CommandRegistry {
    tcl_registry::model::ingress::static_context_for("tcl8.6").commands()
}

/// Reborrow one actual store shared by the complete retained source inventory.
fn registry_from_original_sources<'a>(
    sources: impl IntoIterator<Item = Option<&'a CommandTokens>>,
) -> Option<&'a CommandRegistry> {
    let mut selected: Option<&CommandRegistry> = None;
    for source in sources {
        let tokens = source?;
        let registry = tokens
            .source_binding
            .as_ref()?
            .retained_registry_for_tokens(tokens)?;
        if selected.is_some_and(|owner| {
            owner.snapshot().semantic_key() != registry.snapshot().semantic_key()
        }) {
            return None;
        }
        selected = Some(registry);
    }
    selected
}

pub(crate) fn retained_registry_for_script(body: &crate::ir::Script) -> Option<&CommandRegistry> {
    fn sources<'a>(body: &'a crate::ir::Script, result: &mut Vec<Option<&'a CommandTokens>>) {
        for statement in &body.statements {
            if statement.is_executable_invocation() && statement.synthetic_marker().is_none() {
                result.push(body.retained_source_tokens_for_statement(statement));
            }
            for child in statement.child_scripts() {
                sources(child, result);
            }
        }
    }
    let mut inventory = Vec::new();
    sources(body, &mut inventory);
    registry_from_original_sources(inventory)
}

pub(crate) fn retained_registry_for_cfg(cfg: &crate::cfg::Function) -> Option<&CommandRegistry> {
    let statements = cfg.blocks.iter().flat_map(|(block, data)| {
        data.statements
            .iter()
            .enumerate()
            .filter_map(move |(index, statement)| {
                (statement.is_executable_invocation() && statement.synthetic_marker().is_none())
                    .then(|| cfg.source_tokens_at(*block, index))
            })
    });
    let conditions = cfg
        .condition_binding_sites
        .values()
        .map(|site| site.source_tokens.as_deref());
    registry_from_original_sources(statements.chain(conditions))
}

/// Resolve a compiler statement's source-safe word snapshot to the registry's
/// target-neutral invocation facts.
///
/// This deliberately has no fallback through `Statement::{Call,Barrier}`'s
/// lossy `command`/`args` strings: substituted and expanded words must not be
/// reinterpreted as literal Tcl values by an analysis pass.
pub(crate) fn invocation_facts(
    stmt: &Statement,
    registry: &CommandRegistry,
) -> Option<Box<InvocationFacts>> {
    let (Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. }) = stmt else {
        return None;
    };
    let Some(tokens) = tokens else {
        return None;
    };
    invocation_facts_from_tokens(tokens, registry)
}

/// Resolve an ordered source-word snapshot to target-neutral registry facts.
pub(crate) fn invocation_facts_from_tokens(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<Box<InvocationFacts>> {
    invocation_from_tokens(tokens, registry).map(|invocation| invocation.facts)
}

/// Reborrow the unchanged invocation's own supplied metadata; refusal is final.
pub(crate) fn invocation_from_tokens(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<ResolvedStatementInvocation> {
    let metadata = tokens
        .source_binding
        .as_ref()?
        .original_source_metadata_for_advice(tokens, registry)
        .ok()?;
    crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry, metadata, tokens,
    )
}

/// Caller-frame property of this actual independently proved normal handler.
pub(crate) fn normal_handler_is_frameless(
    statement: &Statement,
    registry: &CommandRegistry,
) -> bool {
    let Some(tokens) = statement.tokens() else {
        return false;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return false;
    };
    let Ok(metadata) = binding.original_source_metadata_for_advice(tokens, registry) else {
        return false;
    };
    crate::registry_invocation::normal_statement_representation_with_metadata_context(
        registry, metadata, statement,
    )
    .is_some_and(|invocation| invocation.is_frameless_runtime())
}

/// Whole original substitutions, including the selected expression roles.
/// An unavailable or incomplete source inventory cannot establish frame safety.
pub(crate) struct OriginalSubstitutionInvocation {
    pub(crate) invocation: Option<crate::registry_invocation::SourceNameRoleInventory>,
    pub(crate) frameless: bool,
    pub(crate) name_writes: Option<crate::ir_helpers::VariableWriteEffects>,
}

pub(crate) struct OriginalSubstitutionInventory {
    calls: Vec<OriginalSubstitutionInvocation>,
    pub(crate) unknown_residual: bool,
}
impl std::ops::Deref for OriginalSubstitutionInventory {
    type Target = [OriginalSubstitutionInvocation];
    fn deref(&self) -> &Self::Target {
        &self.calls
    }
}
impl IntoIterator for OriginalSubstitutionInventory {
    type Item = OriginalSubstitutionInvocation;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.calls.into_iter()
    }
}

pub(crate) fn original_substitution_invocations(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<OriginalSubstitutionInventory> {
    let binding = tokens.source_binding.as_ref()?;
    if binding.has_original_authored_source_body_name_role_owner() {
        let scope = binding.original_authored_source_body_scope_for_name_roles(tokens, registry)?;
        let receipts =
            crate::word_subst::checked_authored_source_substitution_words(tokens, registry, scope)?;
        let calls = receipts
            .iter()
            .map(|receipt| OriginalSubstitutionInvocation {
                invocation: crate::registry_invocation::original_authored_substitution_name_roles(
                    receipt, registry,
                ),
                frameless: false,
                name_writes: None,
            })
            .collect();
        return Some(OriginalSubstitutionInventory {
            calls,
            unknown_residual: true,
        });
    }
    let readonly = binding.original_logical_source_input_for_name_roles(tokens, registry);
    let calls = if let Some(input) = readonly {
        crate::word_subst::checked_original_lifted_calls_with_logical_source_input(
            tokens,
            input.lexer_config(),
            registry,
            input,
        )?
    } else {
        let metadata = binding
            .original_source_metadata_for_advice(tokens, registry)
            .ok()?;
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        let site = binding.invocation_site()?;
        crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            config,
        )?;
        let calls = if let Some(metadata) =
            metadata.filter(|metadata| metadata.source_analysis_input().is_some())
        {
            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                tokens, config, registry, metadata,
            )?
        } else {
            let calls = crate::word_subst::checked_argument_lifted_calls(tokens, config)?;
            for parent in
                std::iter::once(tokens).chain(calls.iter().filter_map(|call| call.tokens.as_ref()))
            {
                let invocation =
                    crate::registry_invocation::original_source_name_roles(registry, parent)?;
                if invocation.has_unknown_residual()
                    || !invocation.roles_complete()
                    || invocation
                        .roles()
                        .iter()
                        .any(|(_, role)| *role == tcl_registry::ArgRole::Expr)
                {
                    return None;
                }
            }
            calls
        };
        calls
    };
    let calls = calls
        .into_iter()
        .map(|call| {
            let child = call.tokens.as_ref()?;
            let child_binding = child.source_binding.as_ref()?;
            let frameless = child_binding.original_source_metadata_for_advice(child, registry)
                .ok().and_then(|metadata| {
                    crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                        registry, metadata, child,
                    )
                }).is_some_and(|invocation| invocation.is_frameless_runtime());
            Some(OriginalSubstitutionInvocation {
                invocation: crate::registry_invocation::original_source_name_roles(registry, child),
                frameless,
                name_writes: crate::ir_helpers::original_invocation_possible_name_writes(child, registry),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(OriginalSubstitutionInventory {
        calls,
        unknown_residual: false,
    })
}

pub(crate) fn statement_needs_original_substitution_inventory(statement: &Statement) -> bool {
    statement.is_executable_invocation()
        && statement.synthetic_marker().is_none()
        && !matches!(
            statement,
            Statement::AssignConst { .. }
                | Statement::NativeCall { .. }
                | Statement::Incr { amount: None, .. }
                | Statement::Return {
                    value: None,
                    expr: None,
                    ..
                }
        )
}

/// True if *arg* is a plain identifier, not a substituted ref.
/// Applies the same "starts with `$` or `[`" filter used throughout
/// the memory-SSA alias detectors.
#[must_use]
#[cfg(test)]
pub fn is_literal_name(arg: &str) -> bool {
    if arg.is_empty() {
        return false;
    }
    !arg.starts_with('$') && !arg.starts_with('[')
}

/// Conservative marker scan of an unattributed source presentation.
/// Original words retain their own kind and selected source owner.
#[must_use]
pub fn is_dynamic_token(arg: &str) -> bool {
    tcl_syntax::naming::is_dynamic_word(arg)
}

/// Dynamic-level presentation policy for independent compatibility fixtures.
/// Actual invocations use the selected alias and frame-level owners.
#[must_use]
#[cfg(test)]
pub fn is_dynamic_upvar_level(head: &str) -> bool {
    head.starts_with('$')
        || head.starts_with('[')
        || (head.starts_with('"') && (head.contains('$') || head.contains('[')))
        || (head.starts_with('#') && (head.contains('$') || head.contains('[')))
}

/// An unattributed name presentation that cannot close a scalar target.
/// Literal empty and sigil names require their independent operand owner.
#[must_use]
pub fn is_dynamic_name(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    is_dynamic_token(name)
}

/// Independent compatibility fixture roster; no original handler or frame proof.
#[must_use]
#[cfg(test)]
pub fn is_frameless_runtime_command(cmd: &str) -> bool {
    frameless_runtime_set().contains(cmd)
}

/// Catalogue metadata classification for independent compatibility fixtures.
/// Production eligibility requires the retained original Normal handler.
#[must_use]
#[cfg(test)]
pub fn is_frameless_runtime_command_in(cmd: &str, registry: &CommandRegistry) -> bool {
    registry.get(cmd).is_some_and(|spec| {
        registry
            .effective_semantics()
            .command(spec.descriptor_key().label())
            .is_some_and(|command| command.has_traits(Traits::FRAMELESS_RUNTIME))
    })
}

/// True if *cmd* is one of the five `set` / `incr` / `append` /
/// `lappend` / `unset` commands whose first arg is a variable
/// name.
#[must_use]
#[cfg(test)]
pub fn is_name_first_command(cmd: &str) -> bool {
    name_first_set().contains(cmd)
}

#[cfg(test)]
fn scan_value_for_info_hazards(value: &str) -> (bool, Vec<String>) {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let source = format!("set result \"{value}\"");
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        &source,
        registry,
        false,
        context
            .commands()
            .profile()
            .expect("fixture context retains its selected Tcl profile"),
    );
    let module = &unit.ir_module;
    let Some(tokens) = module
        .top_level
        .statements
        .first()
        .and_then(Statement::tokens)
    else {
        return (true, Vec::new());
    };
    let Some(calls) = original_substitution_invocations(tokens, registry) else {
        return (true, Vec::new());
    };
    let mut state = crate::var_escape::state::EscapeState::new(std::iter::empty::<String>());
    for invocation in calls.into_iter().filter_map(|call| call.invocation) {
        if invocation.facts().canonical_command == "info" {
            crate::var_escape::handlers::handle_source_introspection(&invocation, &mut state);
        }
    }
    (
        state.dynamic_barrier(),
        state.tags.keys().cloned().collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn original_tokens(source: &str) -> CommandTokens {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            context.commands(),
            false,
            context
                .commands()
                .profile()
                .expect("fixture context retains its selected Tcl profile"),
        );
        let script = &unit.ir_module.top_level;
        script
            .statements
            .iter()
            .find_map(|statement| script.retained_source_tokens_for_statement(statement))
            .expect("actual original invocation")
            .clone()
    }

    #[test]
    fn authored_substitution_expansions_cannot_relabel_original_name_words() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(
            "proc p {} {puts [info {*}{exists {$expanded}}]; puts [info {*}$dynamic]}",
            &registry,
        );
        let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
        for statement in &procedure.body.statements {
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let calls = original_substitution_invocations(tokens, &registry).unwrap();
            assert!(calls.unknown_residual);
            assert_eq!(calls.len(), 1);
            assert!(!calls[0].frameless);
            assert!(calls[0].invocation.is_none());
        }
    }

    #[test]
    fn authored_body_scope_substitutions_keep_actual_expr_names_and_unknown_execution() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(
            "proc p {} {puts [info exists {$lexical}]; if {[info exists {$expression}]} {puts yes}; puts {[info exists {$inert}]}}",
            &registry,
        );
        let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
        for (statement, expected) in [(0, Some("$lexical")), (1, Some("$expression")), (2, None)] {
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(&procedure.body.statements[statement])
                .unwrap();
            let calls = original_substitution_invocations(tokens, &registry)
                .expect("actual Authored source substitution owner");
            assert!(calls.unknown_residual);
            match expected {
                Some(expected) => {
                    assert_eq!(calls.len(), 1);
                    assert!(!calls[0].frameless);
                    let names = calls[0].invocation.as_ref().unwrap();
                    assert_eq!(names.argument_literal(1).as_deref(), Some(expected));
                    assert!(names.has_unknown_residual());
                }
                None => assert!(calls.is_empty()),
            }
            let mut changed = tokens.clone();
            changed.argv_texts[1].push_str(" changed");
            assert!(original_substitution_invocations(&changed, &registry).is_none());
            let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
            assert!(original_substitution_invocations(tokens, foreign).is_none());
        }
    }

    #[test]
    fn original_substitutions_keep_inert_data_and_literal_operand_values() {
        let registry = default_registry();
        let inert = original_tokens("set observed {[info exists target]}");
        assert!(
            original_substitution_invocations(&inert, registry)
                .expect("original inert argument")
                .is_empty()
        );
        let tokens =
            original_tokens(r#"set observed "prefix[info exists {$literal[tail]}]suffix""#);
        let calls = original_substitution_invocations(&tokens, registry)
            .expect("complete original quoted substitutions");
        assert_eq!(calls.len(), 1);
        let invocation = calls[0]
            .invocation
            .as_ref()
            .expect("original selected info descriptor");
        assert_eq!(
            invocation
                .argument_literal(invocation.facts().argument_offset)
                .as_deref(),
            Some("$literal[tail]")
        );
        let mut state = crate::var_escape::state::EscapeState::new(std::iter::empty::<String>());
        crate::var_escape::handlers::handle_source_introspection(invocation, &mut state);
        assert!(!state.dynamic_barrier());
        assert!(state.tags.contains_key("$literal[tail]"));
    }

    #[test]
    fn original_substitutions_include_expression_roles_and_refuse_changed_ownership() {
        let registry = default_registry();
        let tokens = original_tokens("expr {[info exists target]}");
        let calls =
            original_substitution_invocations(&tokens, registry).expect("original expression role");
        assert_eq!(calls.len(), 1);
        let invocation = calls[0]
            .invocation
            .as_ref()
            .expect("actual nested descriptor");
        assert_eq!(
            invocation
                .argument_literal(invocation.facts().argument_offset)
                .as_deref(),
            Some("target")
        );
        let mut changed = tokens.clone();
        changed.source_binding = None;
        assert!(original_substitution_invocations(&changed, registry).is_none());
        changed = tokens;
        changed.argv_texts.push("foreign".to_owned());
        assert!(original_substitution_invocations(&changed, registry).is_none());
        let other = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(
            original_substitution_invocations(
                &original_tokens("set observed [info exists target]"),
                other
            )
            .is_none()
        );
    }

    #[test]
    fn literal_name_rejects_substituted_tokens() {
        assert!(is_literal_name("foo"));
        assert!(is_literal_name("foo_bar"));
        assert!(!is_literal_name("$foo"));
        assert!(!is_literal_name("[cmd]"));
        assert!(!is_literal_name(""));
    }

    #[test]
    fn dynamic_token_detects_dollar_or_bracket() {
        assert!(is_dynamic_token("$x"));
        assert!(is_dynamic_token("[expr 1+1]"));
        assert!(is_dynamic_token("foo$bar"));
        assert!(is_dynamic_token("foo[cmd]"));
        assert!(!is_dynamic_token("foo"));
        assert!(!is_dynamic_token(""));
    }

    #[test]
    fn dynamic_upvar_level_recognises_all_substitution_shapes() {
        // ``$var`` and ``${var}``.
        assert!(is_dynamic_upvar_level("$lvl"));
        assert!(is_dynamic_upvar_level("${lvl}"));
        assert!(is_dynamic_upvar_level("$::ns::level"));
        // ``[cmd subst]``.
        assert!(is_dynamic_upvar_level("[expr {$n}]"));
        assert!(is_dynamic_upvar_level("[foo]"));
        // Double-quoted strings carrying ``$`` / ``[``.
        assert!(is_dynamic_upvar_level("\"#${n}\""));
        assert!(is_dynamic_upvar_level("\"[expr {$n}]\""));
        // ``#``-absolute forms with embedded substitution.
        assert!(is_dynamic_upvar_level("#$n"));
        assert!(is_dynamic_upvar_level("#${n}"));
        assert!(is_dynamic_upvar_level("#[foo]"));
        // Non-substituted shapes — not flagged here. Callers gate
        // literal integers / ``#N`` separately via
        // ``is_level_literal``.
        assert!(!is_dynamic_upvar_level("1"));
        assert!(!is_dynamic_upvar_level("#0"));
        assert!(!is_dynamic_upvar_level("\"plain\""));
        assert!(!is_dynamic_upvar_level(""));
    }

    #[test]
    fn dynamic_name_includes_empty() {
        assert!(is_dynamic_name(""));
        assert!(is_dynamic_name("$x"));
        assert!(!is_dynamic_name("plain_name"));
    }

    #[test]
    fn frameless_metadata_uses_the_selected_descriptor_before_optional_root_fallback() {
        let mut registry = CommandRegistry::build_default();
        assert!(is_frameless_runtime_command_in("list", &registry));
        assert!(is_frameless_runtime_command_in("::list", &registry));
        for label in [
            "::::list",
            ":list",
            "list::",
            "list:::member",
            "missing",
            "",
        ] {
            assert!(!is_frameless_runtime_command_in(label, &registry));
        }
        let mut rooted = registry.get_exact("list").unwrap().clone();
        rooted.name = "::list";
        rooted.traits = Traits::empty();
        registry.insert(rooted);
        assert!(is_frameless_runtime_command_in("list", &registry));
        assert!(!is_frameless_runtime_command_in("::list", &registry));
    }

    #[test]
    fn frameless_runtime_command_membership() {
        assert!(is_frameless_runtime_command("list"));
        assert!(is_frameless_runtime_command("string"));
        assert!(is_frameless_runtime_command("expr"));
        assert!(is_frameless_runtime_command("set"));
        assert!(!is_frameless_runtime_command("foreach"));
        assert!(!is_frameless_runtime_command("proc"));
    }

    #[test]
    fn name_first_command_is_the_documented_six() {
        for cmd in ["set", "incr", "append", "lappend", "lset", "unset"] {
            assert!(is_name_first_command(cmd), "{cmd}");
        }
        assert!(!is_name_first_command("string"));
        assert!(!is_name_first_command("list"));
    }

    #[test]
    fn scan_value_finds_info_exists_literal() {
        let (pess, names) = scan_value_for_info_hazards("foo [info exists bar] baz");
        assert!(!pess);
        assert_eq!(names, vec!["bar".to_string()]);
    }

    #[test]
    fn scan_value_dynamic_info_exists_is_pessimistic() {
        let (pess, names) = scan_value_for_info_hazards("[info exists $dyn]");
        assert!(pess);
        assert_eq!(names, [] as [std::string::String; 0]);
    }

    #[test]
    fn scan_value_frame_inspecting_is_pessimistic() {
        let (pess, _) = scan_value_for_info_hazards("[info level]");
        assert!(pess);
        let (pess, _) = scan_value_for_info_hazards("[info frame]");
        assert!(pess);
        let (pess, _) = scan_value_for_info_hazards("[info vars]");
        assert!(pess);
    }

    #[test]
    fn scan_value_safe_info_does_not_escape() {
        let (pess, names) = scan_value_for_info_hazards("[info patchlevel]");
        assert!(!pess);
        assert_eq!(names, [] as [std::string::String; 0]);
    }

    #[test]
    fn scan_value_no_info_means_no_hazard() {
        let (pess, names) = scan_value_for_info_hazards("plain text");
        assert!(!pess);
        assert_eq!(names, [] as [std::string::String; 0]);
        let (pess, names) = scan_value_for_info_hazards("[other_cmd arg]");
        assert!(!pess);
        assert_eq!(names, [] as [std::string::String; 0]);
    }

    #[test]
    fn scan_value_unknown_info_subcommand_is_pessimistic() {
        // Unknown ``info`` subcommands are conservative — pessimistic.
        let (pess, _) = scan_value_for_info_hazards("[info totally_made_up]");
        assert!(pess);
    }
}
