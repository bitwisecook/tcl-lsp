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

//! Command-alias detection and resolution.
//!
//! Used by the lowerer to resolve command names through the alias table,
//! and the compiler's bridge from **source words** to the registry's one
//! command-table transition vocabulary.
//!
//! ## The one vocabulary (centralisation ledger C8)
//!
//! What a call did to the command table is registry data, stated once as
//! [`tcl_registry::CommandBindingTransition`] facts and read through
//! [`command_table_transitions`]. This module used to carry the *other*
//! half of a second vocabulary: `detect_interp_alias`, `detect_rename` and
//! `detect_interp_alias_delete` re-destructured `interp alias`'s and
//! `rename`'s argument layouts here, after each consumer had dispatched on
//! the coarse `CommandTableEffect` word — a layout the registry's own
//! resolvers already knew, and a dynamic-operand rule
//! ([`is_dynamic_word`]) each consumer re-applied for itself. The layout
//! now lives with the resolver
//! (`tcl_registry::state_transition::command_binding`), and a dynamic
//! operand arrives as a typed [`TransitionSubject::Unknown`] instead.
//!
//! [`TransitionSubject::Unknown`]: tcl_registry::TransitionSubject::Unknown

use std::collections::{HashMap, HashSet};

use tcl_registry::{
    CommandBindingTransition, CommandRegistry, InvocationWord, InvocationWords, StateTransitions,
    TransitionSubject,
};

use crate::naming::{is_dynamic_word, normalise_qualified_name};

/// Classify one reconstructed source word for the registry's structured
/// invocation view.
///
/// A word carrying a `$` substitution or a `[…]` command substitution has
/// no statically known Tcl value, so it becomes
/// [`InvocationWord::Dynamic`] and every subject the registry derives from
/// it is a typed unknown. This is the compiler's **one** dynamic-word rule
/// reaching the registry; before ledger C8 each command-table consumer
/// applied its own copy of it after the fact.
#[must_use]
pub fn source_word(text: &str) -> InvocationWord<'_> {
    if is_dynamic_word(text) {
        InvocationWord::Dynamic
    } else {
        InvocationWord::Literal(text)
    }
}

/// The command-table transitions `head args…` establishes, read from the
/// registry under its own profile's command surface.
///
/// This is the compiler's door onto
/// [`CommandRegistry::command_binding_transitions`] for a call whose words
/// are reconstructed source text rather than IR word expressions (the IR
/// path resolves its own structured words through
/// [`crate::registry_invocation`]).
#[must_use]
pub fn command_table_transitions(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
) -> StateTransitions {
    let words: Vec<InvocationWord<'_>> = args.iter().map(|arg| source_word(arg)).collect();
    registry.command_binding_transitions(InvocationWords::structured(
        InvocationWord::Literal(head),
        &words,
    ))
}

/// Whether a transition's interpreter-path subject names the **current**
/// interpreter — the empty path.
///
/// A brace-quoted empty word's Tcl value is the empty string; a source-word
/// reconstruction that kept the braces spells the same value `{}`, so both
/// are the current interpreter. A dynamic path is not: the caller must
/// widen rather than assume.
#[must_use]
pub fn is_current_interpreter(subject: &TransitionSubject) -> bool {
    matches!(subject.literal(), Some("" | "{}"))
}

/// The source word a transition subject came from.
///
/// A literal subject *is* its word. A typed unknown carries the post-head
/// argument index it came from, so a consumer that must still inspect the
/// **written** word — a dynamic-name overlap test such as
/// `${ns}::define::$method` versus `::string` — recovers it here rather
/// than re-deriving the argument layout for itself.
#[must_use]
pub fn subject_word<'a>(subject: &'a TransitionSubject, args: &'a [String]) -> Option<&'a str> {
    match subject {
        TransitionSubject::Literal(value) => Some(value.as_str()),
        TransitionSubject::Unknown { argument_index, .. } => {
            args.get(*argument_index).map(String::as_str)
        }
    }
}

/// Every command name a binding transition names, as written.
///
/// A consumer that must distrust *both* ends of a rebinding — the source
/// and the destination — reads this rather than re-walking the argument
/// list. Dynamic subjects contribute nothing; a consumer that must widen
/// on them reads the transition itself.
#[must_use]
pub fn transition_names(transition: &CommandBindingTransition) -> Vec<&str> {
    match transition {
        CommandBindingTransition::Define { name, .. } => name.literal().into_iter().collect(),
        CommandBindingTransition::Move { from, to } => [from.literal(), to.literal()]
            .into_iter()
            .flatten()
            .collect(),
        CommandBindingTransition::Delete { name, .. } => name.literal().into_iter().collect(),
        CommandBindingTransition::Alias { alias, target, .. } => {
            [alias.literal(), target.literal()]
                .into_iter()
                .flatten()
                .collect()
        }
        CommandBindingTransition::Unknown { operands } => operands
            .iter()
            .filter_map(TransitionSubject::literal)
            .collect(),
    }
}

/// Alias store: qualified name → (target command, prepended args).
pub type CommandAliasMap = HashMap<String, (String, Vec<String>)>;

/// Look up a command alias, namespace-aware.
///
/// Returns `(target_cmd, prepended_args)` or `None`.
/// If `cmd_name` starts with `::`, looks up directly.
/// Otherwise tries the current `namespace` first, then global.
#[must_use]
pub fn resolve_alias(
    cmd_name: &str,
    aliases: &CommandAliasMap,
    namespace: &str,
) -> Option<(String, Vec<String>)> {
    if cmd_name.starts_with("::") {
        return aliases.get(&normalise_qualified_name(cmd_name)).cloned();
    }

    if namespace != "::" {
        let candidate = normalise_qualified_name(&format!("{namespace}::{cmd_name}"));
        if let Some(entry) = aliases.get(&candidate) {
            return Some(entry.clone());
        }
    }

    aliases
        .get(&normalise_qualified_name(&format!("::{cmd_name}")))
        .cloned()
}

/// Return names that are aliases for `expr` (no prepended args).
///
/// Returns both the qualified keys (`::=`) and stripped short names
/// (`=`) so callers matching against bare command words get a hit.
///
/// The literal `"expr"` target match stays name-keyed for now: the
/// consumers are the registry-less lowering fast paths
/// (`extract_single_expr_arg` behind `LoweringHookId::Return` / `Set`),
/// whose shared `dispatch_lowering_hook` signature deliberately carries
/// no registry.  Routing this through `EXPR_CONCATENATES_ARGS` means
/// threading a registry through that public dispatch table — tracked
/// migration debt rather than a semantic choice.
#[must_use]
pub fn expr_alias_names(aliases: &CommandAliasMap) -> HashSet<String> {
    let mut result = HashSet::new();
    for (name, (target, prepended)) in aliases {
        if target == "expr" && prepended.is_empty() {
            result.insert(name.clone());
            if let Some(short) = name.rsplit("::").next()
                && !short.is_empty()
                && name.starts_with("::")
            {
                result.insert(short.to_owned());
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: the transitions a `head args…` source call establishes.
    fn transitions(head: &str, args: &[&str]) -> Vec<CommandBindingTransition> {
        let registry = CommandRegistry::build_default();
        let owned: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
        command_table_transitions(&registry, head, &owned)
            .command_bindings()
            .cloned()
            .collect()
    }

    fn literal(subject: &TransitionSubject) -> Option<&str> {
        subject.literal()
    }

    #[test]
    fn interp_alias_creation_states_the_alias_with_its_baked_arguments() {
        let facts = transitions(
            "interp",
            &["alias", "", "myalias", "", "puts", "-nonewline"],
        );
        let [
            CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
            },
        ] = facts.as_slice()
        else {
            panic!("one alias fact, got {facts:?}");
        };
        assert!(is_current_interpreter(source_interpreter));
        assert!(is_current_interpreter(target_interpreter));
        assert_eq!(literal(alias), Some("myalias"));
        assert_eq!(literal(target), Some("puts"));
        assert_eq!(arguments.len(), 1);
        assert_eq!(literal(&arguments[0]), Some("-nonewline"));
    }

    #[test]
    fn a_dynamic_alias_target_is_a_typed_unknown_not_a_source_spelling() {
        // `interp alias {} myEval {} $target` — the true target isn't known
        // statically; the subject must never carry the literal string
        // `"$target"`, which would silently map `myEval` onto a
        // never-registered command name.
        let facts = transitions("interp", &["alias", "", "myEval", "", "$target"]);
        let [CommandBindingTransition::Alias { alias, target, .. }] = facts.as_slice() else {
            panic!("one alias fact, got {facts:?}");
        };
        assert_eq!(literal(alias), Some("myEval"));
        assert_eq!(literal(target), None);
    }

    #[test]
    fn a_dynamic_alias_name_is_a_typed_unknown() {
        let facts = transitions("interp", &["alias", "", "$name", "", "eval"]);
        let [CommandBindingTransition::Alias { alias, .. }] = facts.as_slice() else {
            panic!("one alias fact, got {facts:?}");
        };
        assert_eq!(literal(alias), None);
    }

    #[test]
    fn rename_states_a_move_of_both_names() {
        let facts = transitions("rename", &["eval", "myEval"]);
        let [CommandBindingTransition::Move { from, to }] = facts.as_slice() else {
            panic!("one move, got {facts:?}");
        };
        assert_eq!(literal(from), Some("eval"));
        assert_eq!(literal(to), Some("myEval"));
    }

    #[test]
    fn a_dynamic_rename_operand_is_a_typed_unknown_at_either_end() {
        // `rename $old eval` / `rename eval myEval[x]` — the true name isn't
        // known statically, so no consumer may read the source spelling as a
        // command name. A dynamic *source* still states the move (the
        // destination is known); a dynamic *destination* makes the whole
        // shape unknown, because an empty destination would have been a
        // deletion instead.
        let facts = transitions("rename", &["$old", "eval"]);
        let [CommandBindingTransition::Move { from, to }] = facts.as_slice() else {
            panic!("one move, got {facts:?}");
        };
        assert_eq!(literal(from), None);
        assert_eq!(literal(to), Some("eval"));

        let facts = transitions("rename", &["eval", "myEval[x]"]);
        let [CommandBindingTransition::Unknown { operands }] = facts.as_slice() else {
            panic!("one unknown, got {facts:?}");
        };
        assert_eq!(literal(&operands[0]), Some("eval"));
        assert_eq!(literal(&operands[1]), None);
    }

    #[test]
    fn renaming_to_the_empty_name_is_a_deletion_not_a_move() {
        let facts = transitions("rename", &["eval", ""]);
        let [
            CommandBindingTransition::Delete {
                interpreter: None,
                name,
            },
        ] = facts.as_slice()
        else {
            panic!("one delete, got {facts:?}");
        };
        assert_eq!(literal(name), Some("eval"));
    }

    #[test]
    fn a_wrong_arity_rename_states_nothing() {
        // Any arity but two is `wrong # args`, which moves nothing.
        assert!(transitions("rename", &["eval"]).is_empty());
        assert!(transitions("rename", &[]).is_empty());
        assert!(transitions("rename", &["a", "b", "c"]).is_empty());
    }

    #[test]
    fn a_qualified_rename_target_keeps_its_global_root() {
        let facts = transitions("rename", &["eval", "::ns::myEval"]);
        let [CommandBindingTransition::Move { to, .. }] = facts.as_slice() else {
            panic!("one move, got {facts:?}");
        };
        assert_eq!(literal(to), Some("::ns::myEval"));
    }

    #[test]
    fn a_foreign_source_interpreter_is_stated_as_such() {
        // `interp alias slave x {} y` binds a name in a child interpreter;
        // the fact records which, and the consumer decides.
        let facts = transitions("interp", &["alias", "slave", "x", "", "y"]);
        let [
            CommandBindingTransition::Alias {
                source_interpreter, ..
            },
        ] = facts.as_slice()
        else {
            panic!("one alias fact, got {facts:?}");
        };
        assert!(!is_current_interpreter(source_interpreter));
    }

    #[test]
    fn the_four_word_alias_form_deletes_and_the_three_word_form_queries() {
        let facts = transitions("interp", &["alias", "", "bar", ""]);
        let [CommandBindingTransition::Delete { interpreter, name }] = facts.as_slice() else {
            panic!("one delete, got {facts:?}");
        };
        assert!(is_current_interpreter(
            interpreter
                .as_ref()
                .expect("an alias delete names its interpreter")
        ));
        assert_eq!(literal(name), Some("bar"));

        // Only two words after `alias` — a query, which deletes nothing.
        assert!(transitions("interp", &["alias", "", "bar"]).is_empty());
        // Five words — a creation, not a deletion.
        assert!(matches!(
            transitions("interp", &["alias", "", "bar", "", "foo"]).as_slice(),
            [CommandBindingTransition::Alias { .. }]
        ));
    }

    /// A `SpecTcl` pack may stamp `CommandTableEffect::CreatesAliases` on a
    /// command whose words are nothing like `interp alias` — the tcllib draft
    /// does it on `struct::tree` and `struct::graph`, which build their handle
    /// through `interp alias` internally. The stock resolver must read that as
    /// "no alias stated" rather than inventing one out of the command's own
    /// arguments; before the shape guard the consumer-side detector aborted a
    /// debug build outright (`tcl-spectcl/tests/spec_corpus.rs` caught it).
    #[test]
    fn a_call_that_is_not_interp_alias_shaped_states_no_alias() {
        let registry = CommandRegistry::build_default();
        for args in [
            vec![
                "myTree".to_string(),
                String::new(),
                "x".into(),
                String::new(),
            ],
            vec![
                "myGraph".to_string(),
                String::new(),
                "g".into(),
                String::new(),
                "puts".into(),
            ],
        ] {
            let stated = tcl_registry::CommandTableEffect::CreatesAliases
                .transitions()
                .resolve(tcl_registry::InvocationArguments::literals(
                    &args.iter().map(String::as_str).collect::<Vec<_>>(),
                ));
            assert!(
                stated.command_bindings().next().is_none(),
                "a non-`interp alias`-shaped call states no alias: {args:?}",
            );
        }
        // And a genuinely unknown head states nothing at all.
        assert!(
            command_table_transitions(&registry, "myTree", &[String::new()])
                .command_bindings()
                .next()
                .is_none()
        );
    }

    #[test]
    fn a_deleting_alias_in_a_foreign_interpreter_is_stated_as_foreign() {
        let facts = transitions("interp", &["alias", "slave", "bar", ""]);
        let [CommandBindingTransition::Delete { interpreter, name }] = facts.as_slice() else {
            panic!("one delete, got {facts:?}");
        };
        assert!(!is_current_interpreter(
            interpreter
                .as_ref()
                .expect("an alias delete names its interpreter")
        ));
        assert_eq!(literal(name), Some("bar"));
    }

    #[test]
    fn resolve_global() {
        let mut aliases = CommandAliasMap::new();
        aliases.insert("::myalias".into(), ("puts".into(), vec![]));
        let result = resolve_alias("myalias", &aliases, "::");
        assert_eq!(result, Some(("puts".into(), vec![])));
    }

    #[test]
    fn resolve_qualified() {
        let mut aliases = CommandAliasMap::new();
        aliases.insert("::ns::cmd".into(), ("target".into(), vec!["a".into()]));
        let result = resolve_alias("::ns::cmd", &aliases, "::");
        assert_eq!(result, Some(("target".into(), vec!["a".into()])));
    }

    #[test]
    fn resolve_namespace_local() {
        let mut aliases = CommandAliasMap::new();
        aliases.insert("::ns::cmd".into(), ("target".into(), vec![]));
        // When in ::ns namespace, unqualified "cmd" should resolve.
        let result = resolve_alias("cmd", &aliases, "::ns");
        assert_eq!(result, Some(("target".into(), vec![])));
    }

    #[test]
    fn resolve_not_found() {
        let aliases = CommandAliasMap::new();
        assert!(resolve_alias("nope", &aliases, "::").is_none());
    }

    #[test]
    fn expr_aliases() {
        let mut aliases = CommandAliasMap::new();
        aliases.insert("::=".into(), ("expr".into(), vec![]));
        aliases.insert("::notexpr".into(), ("puts".into(), vec![]));
        aliases.insert("::exprwithargs".into(), ("expr".into(), vec!["1".into()]));
        let names = expr_alias_names(&aliases);
        assert!(names.contains("::="));
        assert!(names.contains("="));
        assert!(!names.contains("::notexpr"));
        assert!(!names.contains("::exprwithargs"));
    }
}
