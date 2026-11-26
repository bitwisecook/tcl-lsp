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

//! Every definition-body member carries an effect that agrees with its roles.

use super::*;

// ── The member-effect descriptor ────────────────────────────────────────────

/// Every definition-body grammar the sweep can reach: the shipped constants,
/// the `SpecTcl` / `SslicTcl` document grammars, and every grammar a
/// loadable dialect's command or subcommand hangs off `definition_body` (a
/// pack's inline grammar included), each once.
fn every_definition_grammar() -> Vec<(
    String,
    &'static tcl_registry::definer::DefinitionBodyGrammar,
)> {
    use tcl_registry::definer::{
        ITCL_GRAMMAR, SNIT_GRAMMAR, SNIT_WIDGET_GRAMMAR, SPECTCL_GRAMMARS, SSLICTCL_GRAMMARS,
        TCLOO_CONFIGURABLE_GRAMMAR, TCLOO_GRAMMAR,
    };
    let mut out: Vec<(
        String,
        &'static tcl_registry::definer::DefinitionBodyGrammar,
    )> = vec![
        ("TCLOO_GRAMMAR".to_owned(), &TCLOO_GRAMMAR),
        (
            "TCLOO_CONFIGURABLE_GRAMMAR".to_owned(),
            &TCLOO_CONFIGURABLE_GRAMMAR,
        ),
        ("SNIT_GRAMMAR".to_owned(), &SNIT_GRAMMAR),
        ("SNIT_WIDGET_GRAMMAR".to_owned(), &SNIT_WIDGET_GRAMMAR),
        ("ITCL_GRAMMAR".to_owned(), &ITCL_GRAMMAR),
    ];
    for (index, grammar) in SPECTCL_GRAMMARS.iter().chain(SSLICTCL_GRAMMARS).enumerate() {
        out.push((format!("document grammar #{index}"), *grammar));
    }
    for &dialect in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dialect);
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            if let Some(grammar) = spec.definition_body {
                out.push((format!("{dialect} {}", spec.name), grammar));
            }
        }
    }
    let mut seen: Vec<*const tcl_registry::definer::DefinitionBodyGrammar> = Vec::new();
    out.retain(|(_, grammar)| {
        let pointer: *const _ = *grammar;
        let fresh = !seen.contains(&pointer);
        seen.push(pointer);
        fresh
    });
    out
}

/// The member-effect descriptor's agreement rules: what a member declares
/// never contradicts the layout that positions it.
///
/// - a `Callable`'s name / params / body slots are the positions its
///   `arg_roles` type `Name` / `ParamList` / `Body` — exactly the first such
///   position, or `None` when there is none, which is the reading the
///   `.tclspec` loader applies to an unwritten slot, so a shipped row
///   renders and reloads as itself;
/// - an `InitScript`'s body slot is a `Body` position;
/// - a `Forward`'s name slot is a `Name` position and its prefix slot the
///   target's `CommandName` (or a `CommandPrefix`) position;
/// - a `Relation` row is a slot — or, like itcl's `inherit`, which takes no
///   slot operation words, a plain list of class references; a
///   `Retraction` row carries its
///   `retraction`, and only it does; a `Visibility` row carries its
///   `visibility_effect`, and only it does;
/// - only a wrapper shifts what it wraps, and a wrapper is `Configuration`.
#[test]
fn every_member_carries_an_effect_that_agrees_with_its_roles() {
    use tcl_registry::definer::{MemberEffect, MemberKind};
    let grammars = every_definition_grammar();
    let mut members = 0usize;
    for (path, grammar) in &grammars {
        for member in grammar.members {
            members += 1;
            let at = format!("{path} member `{}`", member.keyword);
            let first = |role: ArgRole| member.indices_for(role).next();
            let typed = |slot: u8, roles: &[ArgRole]| {
                member
                    .arg_roles
                    .iter()
                    .any(|(index, role)| *index == slot && roles.contains(role))
            };
            match member.effect {
                MemberEffect::Callable {
                    name_slot,
                    params_slot,
                    body_slot,
                    ..
                } => {
                    for (slot, role) in [
                        (name_slot, ArgRole::Name),
                        (params_slot, ArgRole::ParamList),
                        (body_slot, ArgRole::Body),
                    ] {
                        assert_eq!(
                            slot.map(usize::from),
                            first(role),
                            "{at}: a callable's {role:?} slot is its first {role:?} position"
                        );
                    }
                }
                MemberEffect::InitScript { body_slot, .. } => {
                    assert!(
                        typed(body_slot, &[ArgRole::Body]),
                        "{at}: body slot is a Body"
                    );
                }
                MemberEffect::Forward {
                    name_slot,
                    prefix_slot,
                } => {
                    assert!(
                        typed(name_slot, &[ArgRole::Name]),
                        "{at}: name slot is a Name"
                    );
                    assert!(
                        typed(prefix_slot, &[ArgRole::CommandName, ArgRole::CommandPrefix]),
                        "{at}: prefix slot is the target command's position"
                    );
                }
                MemberEffect::Relation { .. } => {
                    assert!(
                        member.slot.is_some()
                            || member.all_args_ref
                                == Some(tcl_registry::definer::MemberRefKind::Class),
                        "{at}: a relation is a slot, or a plain list of class references"
                    );
                }
                MemberEffect::StateDeclaration { .. }
                | MemberEffect::Visibility
                | MemberEffect::Retraction
                | MemberEffect::Configuration => {}
            }
            assert_eq!(
                member.retraction.is_some(),
                member.effect == MemberEffect::Retraction,
                "{at}: a retraction and its `retraction` go together"
            );
            assert_eq!(
                member.visibility_effect.is_some(),
                member.effect == MemberEffect::Visibility,
                "{at}: a visibility change and its `visibility_effect` go together"
            );
            assert!(
                member.slot.is_none()
                    || matches!(
                        member.effect,
                        MemberEffect::Relation { .. } | MemberEffect::StateDeclaration { .. }
                    ),
                "{at}: a slot feeds a relation or declares state"
            );
            assert!(
                member.wrapper_shift.is_none() || member.kind == MemberKind::Wrapper,
                "{at}: only a wrapper shifts what it wraps"
            );
            assert!(
                member.kind != MemberKind::Wrapper || member.effect == MemberEffect::Configuration,
                "{at}: a wrapper configures and declares nothing of its own"
            );
        }
    }
    assert!(
        grammars.len() >= 5 && members > 100,
        "the shipped grammars must be reached ({} grammars, {members} members)",
        grammars.len()
    );
}

/// The vocabulary is family-neutral (negative): no effect, receiver, role,
/// scope, relation or timing spells a class system's name, in its `Debug`
/// form or its `.tclspec` spelling. `DefinerFamily` stays the one place a
/// family is named.
#[test]
fn no_member_effect_names_a_family() {
    use tcl_registry::definer::{
        CallableRole, InitTiming, MemberEffect, MemberReceiver, RelationSlot, StateScope,
    };
    let mut spellings: Vec<String> = MemberEffect::KIND_SPELLINGS
        .iter()
        .map(|spelling| (*spelling).to_owned())
        .collect();
    spellings.extend(
        MemberReceiver::ALL
            .iter()
            .flat_map(|v| [format!("{v:?}"), v.spelling().to_owned()]),
    );
    spellings.extend(
        CallableRole::ALL
            .iter()
            .flat_map(|v| [format!("{v:?}"), v.spelling().to_owned()]),
    );
    spellings.extend(
        StateScope::ALL
            .iter()
            .flat_map(|v| [format!("{v:?}"), v.spelling().to_owned()]),
    );
    spellings.extend(
        RelationSlot::ALL
            .iter()
            .flat_map(|v| [format!("{v:?}"), v.spelling().to_owned()]),
    );
    spellings.extend(
        InitTiming::ALL
            .iter()
            .flat_map(|v| [format!("{v:?}"), v.spelling().to_owned()]),
    );
    for (_, grammar) in every_definition_grammar() {
        spellings.extend(
            grammar
                .members
                .iter()
                .map(|member| format!("{:?}", member.effect)),
        );
    }
    for spelling in &spellings {
        let lower = spelling.to_ascii_lowercase();
        for family in ["tcloo", "snit", "itcl"] {
            assert!(
                !lower.contains(family),
                "`{spelling}` names the {family} family"
            );
        }
    }
}
