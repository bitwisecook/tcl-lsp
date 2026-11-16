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

//! Argument-role resolvers: the closed native count and layout descriptors and the capability set a resolver must state.

use super::super::statement_reader::Stmt;
use super::super::value_parsers::{by_name, enum_by_name, list_words};
use super::super::vocabulary_class::VocabularyClass;
use super::super::{Log, leak_slice};
use std::fmt;
use tcl_registry::arg_role::ArgRole;

pub(super) fn index_list(text: &str) -> Vec<u8> {
    list_words(text)
        .iter()
        .filter_map(|word| word.parse().ok())
        .collect()
}

/// Count hooks use a closed native descriptor; arbitrary Tcl hooks require a
/// typed cardinality evaluator and are rejected rather than fed dummy words.
pub(in super::super) fn native_count_role_resolver(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<tcl_registry::spec::ArgRoleCountResolver> {
    let selected =
        (stmt.words.len() == 3 && stmt.word_text(1) == "-native").then(|| stmt.word_text(2));
    let resolver = selected.and_then(|id| {
        match tcl_registry::native_argument_role_hook::shipped_native_argument_role_hook(id) {
            Some(tcl_registry::native_argument_role_hook::NativeArgumentRoleHook::Count(
                resolver,
            )) => Some(resolver),
            _ => None,
        }
    });
    if resolver.is_none() {
        log.say_classified(
            stmt.line,
            VocabularyClass::Semantic,
            "`arg_role_count_resolver` requires a known native cardinality descriptor; Tcl count hooks are unsupported",
        );
    }
    resolver
}

/// Structured hooks use a closed native descriptor; arbitrary Tcl hooks require a
/// typed structured-operand evaluator and are rejected rather than fed dummy words.
pub(in super::super) fn native_layout_role_resolver(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<tcl_registry::spec::ArgRoleLayoutResolver> {
    let selected =
        (stmt.words.len() == 3 && stmt.word_text(1) == "-native").then(|| stmt.word_text(2));
    let resolver = selected.and_then(|id| {
        match tcl_registry::native_argument_role_hook::shipped_native_argument_role_hook(id) {
            Some(tcl_registry::native_argument_role_hook::NativeArgumentRoleHook::Layout(
                resolver,
            )) => Some(resolver),
            _ => None,
        }
    });
    if resolver.is_none() {
        log.say_classified(
            stmt.line,
            VocabularyClass::Semantic,
            "`arg_role_layout_resolver` requires a known native structured descriptor; Tcl layout hooks are unsupported",
        );
    }
    resolver
}

pub(in super::super) fn arg_role_capabilities(
    text: &str,
    line: u32,
    log: &mut Log,
) -> &'static [ArgRole] {
    let names = list_words(text);
    if names.is_empty() {
        log.say_classified(
            line,
            VocabularyClass::Assistance,
            "`arg_role_resolver_roles` must name at least one argument role",
        );
        return &[];
    }

    let mut roles = Vec::new();
    let mut valid = true;
    for name in names {
        match by_name(ArgRole::ALL, &name) {
            Some(role) if !roles.contains(&role) => roles.push(role),
            Some(_) => {}
            None => {
                valid = false;
                log.say_classified(
                    line,
                    VocabularyClass::Assistance,
                    format!(
                        "unknown argument role `{name}` makes the resolver capability set incomplete"
                    ),
                );
            }
        }
    }
    if valid { leak_slice(roles) } else { &[] }
}

/// A resolver must state its closed capability set; a capability set must
/// describe a role source. A clause grammar is one — its declared set is the
/// closed set its walk emits — but needs none, since the registry reads the
/// grammar's own slots when a call cannot be walked.
pub(in super::super) fn validate_arg_role_capabilities(
    has_resolver: bool,
    has_grammar: bool,
    roles: &[ArgRole],
    owner: &str,
    line: u32,
    log: &mut Log,
) {
    match (has_resolver, roles.is_empty()) {
        (false, false) if has_grammar => {}
        (true, true) => log.say_classified(
            line,
            VocabularyClass::Assistance,
            format!(
                "{owner} has an `arg_role_resolver` without a non-empty \
                 `arg_role_resolver_roles` capability set"
            ),
        ),
        (false, false) => log.say_classified(
            line,
            VocabularyClass::Assistance,
            format!(
                "{owner} declares `arg_role_resolver_roles` without an \
                 `arg_role_resolver`"
            ),
        ),
        _ => {}
    }
}

pub(in super::super) fn native_id<T: Copy + fmt::Debug>(
    stmt: &Stmt,
    all: &[T],
    what: &str,
    log: &mut Log,
) -> Option<T> {
    if stmt.word_text(1) != "-native" {
        log.say(stmt.line, format!("`{what}` takes `-native ID`"));
        return None;
    }
    enum_by_name(all, stmt.word_text(2), what, stmt.line, log)
}
