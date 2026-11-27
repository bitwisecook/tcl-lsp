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

//! The Tcl surface's hook table: the shipped implementations a pack names
//! with `-native ID`, and the native-compiler and display facts generic core
//! modules ask the surface for
//! (`docs/design/registry/registry-split/02-behaviour-to-surfaces.md`).

use crate::hooks::{ConstFoldFn, VersionedConstFoldFn};
use crate::registry::surfaces::{ConditionalClauseHooks, HookTable};
use crate::state_transition::StateTransitionResolver;

/// The Tcl surface's registrations.
pub(crate) static HOOKS: HookTable = HookTable {
    const_fold: CONST_FOLD,
    const_fold_versioned: CONST_FOLD_VERSIONED,
    state_transition_resolver: STATE_TRANSITION_RESOLVER,
    try_clauses: Some(&super::try_::GRAMMAR),
    conditional: Some(&CONDITIONAL),
    static_display: Some(&super::STATIC_DISPLAY),
    ..HookTable::EMPTY
};

/// `TclCompileIfCmd`'s clause checks, all walks of `if`'s own grammar.
static CONDITIONAL: ConditionalClauseHooks = ConditionalClauseHooks {
    shape_error: super::if_::check_if_shape,
    compile_message: super::if_::native_compile_shape_message,
    condition_bodies: super::if_::bpf_conditional_operands,
};

/// The unversioned constant folders, keyed by the command or subcommand each
/// folds (`docs/design/compiler/value-evaluation.md`'s worked example).
const CONST_FOLD: &[(&str, ConstFoldFn)] = &[
    // `fold_range` itself is `VersionedConstFoldFn`-shaped (it reads the
    // release); the unversioned slot `string range` actually ships is the
    // unanimous-answer wrapper around it.
    ("string::range::const_fold", super::fold_range_unanimous),
    ("string::replace::const_fold", super::fold_replace),
    ("regsub::const_fold", super::fold_regsub),
    ("scan::const_fold", super::fold_scan),
    ("list::const_fold", crate::literal_folds::fold_list),
    ("lindex::const_fold", crate::literal_folds::fold_lindex),
    ("concat::const_fold", |args| {
        Some(crate::literal_folds::fold_concat(args))
    }),
    ("llength::const_fold", crate::literal_folds::fold_llength),
    ("lreverse::const_fold", crate::literal_folds::fold_lreverse),
    ("join::const_fold", crate::literal_folds::fold_join),
    ("split::const_fold", crate::literal_folds::fold_split),
    ("lrepeat::const_fold", crate::literal_folds::fold_lrepeat),
    ("lrange::const_fold", crate::literal_folds::fold_lrange),
    ("lsort::const_fold", crate::literal_folds::fold_lsort),
    ("dict::get::const_fold", crate::literal_folds::fold_dict_get),
    (
        "dict::exists::const_fold",
        crate::literal_folds::fold_dict_exists,
    ),
    (
        "dict::size::const_fold",
        crate::literal_folds::fold_dict_size,
    ),
    (
        "dict::keys::const_fold",
        crate::literal_folds::fold_dict_keys,
    ),
    (
        "dict::values::const_fold",
        crate::literal_folds::fold_dict_values,
    ),
    (
        "dict::create::const_fold",
        crate::literal_folds::fold_dict_create,
    ),
    (
        "dict::merge::const_fold",
        crate::literal_folds::fold_dict_merge,
    ),
    // The `::tcl::dict::` spellings are commands of their own
    // (`qualified_specs()`), each carrying its subcommand's folder.
    (
        "::tcl::dict::get::const_fold",
        crate::literal_folds::fold_dict_get,
    ),
    (
        "::tcl::dict::exists::const_fold",
        crate::literal_folds::fold_dict_exists,
    ),
    (
        "::tcl::dict::size::const_fold",
        crate::literal_folds::fold_dict_size,
    ),
    (
        "::tcl::dict::keys::const_fold",
        crate::literal_folds::fold_dict_keys,
    ),
    (
        "::tcl::dict::values::const_fold",
        crate::literal_folds::fold_dict_values,
    ),
    (
        "::tcl::dict::create::const_fold",
        crate::literal_folds::fold_dict_create,
    ),
    (
        "::tcl::dict::merge::const_fold",
        crate::literal_folds::fold_dict_merge,
    ),
    ("string::cat::const_fold", super::fold_cat),
    ("string::compare::const_fold", super::fold_compare),
    ("string::equal::const_fold", super::fold_equal),
    ("string::first::const_fold", super::fold_first),
    ("string::index::const_fold", super::fold_index),
    ("string::last::const_fold", super::fold_last),
    ("string::length::const_fold", super::fold_length),
    ("string::map::const_fold", super::fold_string_map),
    ("string::match::const_fold", super::fold_match),
    ("string::repeat::const_fold", super::fold_repeat),
    ("string::reverse::const_fold", super::fold_reverse),
    ("string::tolower::const_fold", super::fold_tolower),
    ("string::totitle::const_fold", super::fold_totitle),
    ("string::toupper::const_fold", super::fold_toupper),
    ("string::trim::const_fold", super::fold_trim),
    ("string::trimleft::const_fold", super::fold_trimleft),
    ("string::trimright::const_fold", super::fold_trimright),
    ("namespace::qualifiers::const_fold", super::fold_qualifiers),
    ("namespace::tail::const_fold", super::fold_tail),
    ("subst::const_fold", super::fold_subst),
];

/// The release-aware constant folders.
const CONST_FOLD_VERSIONED: &[(&str, VersionedConstFoldFn)] = &[
    (
        "dict::merge::const_fold_versioned",
        crate::literal_folds::fold_dict_merge_versioned,
    ),
    (
        "string::trimright::const_fold_versioned",
        super::fold_trimright_versioned,
    ),
    (
        "string::trimleft::const_fold_versioned",
        super::fold_trimleft_versioned,
    ),
    (
        "string::trim::const_fold_versioned",
        super::fold_trim_versioned,
    ),
    (
        "string::repeat::const_fold_versioned",
        super::fold_repeat_versioned,
    ),
    (
        "split::const_fold_versioned",
        crate::literal_folds::fold_split_versioned,
    ),
    (
        "string::compare::const_fold_versioned",
        super::fold_compare_versioned,
    ),
    (
        "string::equal::const_fold_versioned",
        super::fold_equal_versioned,
    ),
    ("string::is::const_fold_versioned", super::fold_is),
    (
        "string::match::const_fold_versioned",
        super::fold_match_versioned,
    ),
    ("string::range::const_fold_versioned", super::fold_range),
    ("format::const_fold_versioned", super::fold_format),
    ("regsub::const_fold_versioned", super::fold_regsub_versioned),
];

/// `state_transitions { resolver -native ID }`.
const STATE_TRANSITION_RESOLVER: &[(&str, StateTransitionResolver)] = &[(
    "proc::state_transitions.resolver",
    crate::state_transition::command_binding::DEFINES_PROCEDURE
        .resolver
        .expect("the shipped procedure definition carries its resolver"),
)];
