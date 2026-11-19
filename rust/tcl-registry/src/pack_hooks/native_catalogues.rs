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

//! The per-family tables of shipped implementations a pack names with `-native ID`.

use crate::clause_shape::ClauseShapeChecker;
use crate::hooks::{ConstFoldFn, VersionedConstFoldFn};
use crate::hover::OptionValueHook;
use crate::literal_validation::LiteralArgumentValidator;
use crate::spec::{
    ArgRoleResolver, CommandPrefixResolver, ConstraintsHook, ContextGate, ScriptTimingResolver,
};
use crate::state_transition::StateTransitionResolver;
use crate::value_transfer::{DeclaredStructure, EvalRoute};

/// A native implementation, named `SCOPE::FIELD`
/// (`docs/design/compiler/value-evaluation.md` § *`-native ID`, and the
/// per-family catalogues*). `SCOPE` is the command name, `command::subcommand`
/// for a subcommand-scoped field, or `command::subcommand::-option` for an
/// option-scoped one; `FIELD` is the field's own DSL keyword
/// ([`HookFamily::field`], or `semantics` / `evaluate` / `facts`, the two
/// statements no family owns and the one that is also `HookFamily::Evaluate`
/// itself).
///
/// Every family and those two extra fields gets one of these tables below,
/// each keyed by the full id, holding the shipped Rust value the id names.
/// An empty table means nothing this build ships is reachable by id yet —
/// not that the field cannot be declared natively — and every `-native ID`
/// still keeps loading; it only ever fails to resolve.
/// `rust/tcl-spectcl/src/loader.rs`'s `native_hook_tables_cover_their_catalogues`
/// holds each table's id set level with `tcl_spectcl::catalogue`'s picker
/// list of the same name, so a table that gains a shipped entry and a
/// catalogue that does not name it fail the same assertion, from either
/// side.
pub const ARG_ROLE_RESOLVER_NATIVE: &[(&str, ArgRoleResolver)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const COMMAND_PREFIX_RESOLVER_NATIVE: &[(&str, CommandPrefixResolver)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const SCRIPT_TIMING_RESOLVER_NATIVE: &[(&str, ScriptTimingResolver)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. The shipped folders' unversioned
/// constant folders, keyed by the command or subcommand each folds
/// (`docs/design/compiler/value-evaluation.md`'s worked example).
pub const CONST_FOLD_NATIVE: &[(&str, ConstFoldFn)] = &[
    // `fold_range` itself is `VersionedConstFoldFn`-shaped (it reads the
    // release); the unversioned slot `string range` actually ships is the
    // unanimous-answer wrapper around it.
    (
        "string::range::const_fold",
        crate::commands::tcl::fold_range_unanimous,
    ),
    (
        "string::replace::const_fold",
        crate::commands::tcl::fold_replace,
    ),
    ("regsub::const_fold", crate::commands::tcl::fold_regsub),
    ("scan::const_fold", crate::commands::tcl::fold_scan),
    ("list::const_fold", crate::const_fold::fold_list),
    ("lindex::const_fold", crate::const_fold::fold_lindex),
    ("concat::const_fold", |args| {
        Some(crate::const_fold::fold_concat(args))
    }),
    ("llength::const_fold", crate::const_fold::fold_llength),
    ("lreverse::const_fold", crate::const_fold::fold_lreverse),
    ("join::const_fold", crate::const_fold::fold_join),
    ("split::const_fold", crate::const_fold::fold_split),
    ("lrepeat::const_fold", crate::const_fold::fold_lrepeat),
    ("lrange::const_fold", crate::const_fold::fold_lrange),
    ("lsort::const_fold", crate::const_fold::fold_lsort),
    ("dict::get::const_fold", crate::const_fold::fold_dict_get),
    (
        "dict::exists::const_fold",
        crate::const_fold::fold_dict_exists,
    ),
    ("dict::size::const_fold", crate::const_fold::fold_dict_size),
    ("dict::keys::const_fold", crate::const_fold::fold_dict_keys),
    (
        "dict::values::const_fold",
        crate::const_fold::fold_dict_values,
    ),
    (
        "dict::create::const_fold",
        crate::const_fold::fold_dict_create,
    ),
    (
        "dict::merge::const_fold",
        crate::const_fold::fold_dict_merge,
    ),
    // The `::tcl::dict::` spellings are commands of their own
    // (`qualified_specs()`), each carrying its subcommand's folder.
    (
        "::tcl::dict::get::const_fold",
        crate::const_fold::fold_dict_get,
    ),
    (
        "::tcl::dict::exists::const_fold",
        crate::const_fold::fold_dict_exists,
    ),
    (
        "::tcl::dict::size::const_fold",
        crate::const_fold::fold_dict_size,
    ),
    (
        "::tcl::dict::keys::const_fold",
        crate::const_fold::fold_dict_keys,
    ),
    (
        "::tcl::dict::values::const_fold",
        crate::const_fold::fold_dict_values,
    ),
    (
        "::tcl::dict::create::const_fold",
        crate::const_fold::fold_dict_create,
    ),
    (
        "::tcl::dict::merge::const_fold",
        crate::const_fold::fold_dict_merge,
    ),
    ("string::cat::const_fold", crate::commands::tcl::fold_cat),
    (
        "string::compare::const_fold",
        crate::commands::tcl::fold_compare,
    ),
    (
        "string::equal::const_fold",
        crate::commands::tcl::fold_equal,
    ),
    (
        "string::first::const_fold",
        crate::commands::tcl::fold_first,
    ),
    (
        "string::index::const_fold",
        crate::commands::tcl::fold_index,
    ),
    ("string::last::const_fold", crate::commands::tcl::fold_last),
    (
        "string::length::const_fold",
        crate::commands::tcl::fold_length,
    ),
    (
        "string::map::const_fold",
        crate::commands::tcl::fold_string_map,
    ),
    (
        "string::match::const_fold",
        crate::commands::tcl::fold_match,
    ),
    (
        "string::repeat::const_fold",
        crate::commands::tcl::fold_repeat,
    ),
    (
        "string::reverse::const_fold",
        crate::commands::tcl::fold_reverse,
    ),
    (
        "string::tolower::const_fold",
        crate::commands::tcl::fold_tolower,
    ),
    (
        "string::totitle::const_fold",
        crate::commands::tcl::fold_totitle,
    ),
    (
        "string::toupper::const_fold",
        crate::commands::tcl::fold_toupper,
    ),
    ("string::trim::const_fold", crate::commands::tcl::fold_trim),
    (
        "string::trimleft::const_fold",
        crate::commands::tcl::fold_trimleft,
    ),
    (
        "string::trimright::const_fold",
        crate::commands::tcl::fold_trimright,
    ),
    (
        "namespace::qualifiers::const_fold",
        crate::commands::tcl::fold_qualifiers,
    ),
    (
        "namespace::tail::const_fold",
        crate::commands::tcl::fold_tail,
    ),
    ("subst::const_fold", crate::commands::tcl::fold_subst),
];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. The shipped folders' release-aware
/// constant folders.
pub const CONST_FOLD_VERSIONED_NATIVE: &[(&str, VersionedConstFoldFn)] = &[
    (
        "string::repeat::const_fold_versioned",
        crate::commands::tcl::fold_repeat_versioned,
    ),
    (
        "split::const_fold_versioned",
        crate::const_fold::fold_split_versioned,
    ),
    (
        "string::is::const_fold_versioned",
        crate::commands::tcl::fold_is,
    ),
    (
        "string::match::const_fold_versioned",
        crate::commands::tcl::fold_match_versioned,
    ),
    (
        "string::range::const_fold_versioned",
        crate::commands::tcl::fold_range,
    ),
    (
        "format::const_fold_versioned",
        crate::commands::tcl::fold_format,
    ),
    (
        "regsub::const_fold_versioned",
        crate::commands::tcl::fold_regsub_versioned,
    ),
];

/// [`crate::spec::CommandSpec::taint_sink_gate`]'s function-pointer shape,
/// named so [`TAINT_SINK_GATE_NATIVE`]'s element type stays under clippy's
/// `type_complexity` threshold.
pub type TaintSinkGateFn = fn(&[&str]) -> bool;

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const TAINT_SINK_GATE_NATIVE: &[(&str, TaintSinkGateFn)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const CONTEXT_GATE_NATIVE: &[(&str, ContextGate)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const LITERAL_ARGUMENT_VALIDATOR_NATIVE: &[(&str, LiteralArgumentValidator)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const CLAUSE_SHAPE_CHECK_NATIVE: &[(&str, ClauseShapeChecker)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const OPTION_ARITY_NATIVE: &[(&str, OptionValueHook)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const CONSTRAINTS_NATIVE: &[(&str, ConstraintsHook)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. `semantics -native ID`: the shipped
/// structural plan, by name.
pub const SEMANTICS_NATIVE: &[(&str, DeclaredStructure)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. `evaluate -native ID`: a shipped
/// evaluator whose route the catalogue entry itself names. `-direct` and
/// `-expression` are different, already-closed catalogues of their own
/// (`NativeEvalId::ALL`, `LanguageProfileId::ALL`), not `SCOPE::FIELD` ids.
pub const EVALUATE_NATIVE: &[(&str, EvalRoute)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. `state_transitions { resolver -native
/// ID }`.
pub const STATE_TRANSITION_RESOLVER_NATIVE: &[(&str, StateTransitionResolver)] = &[(
    "proc::state_transitions.resolver",
    crate::state_transition::command_binding::DEFINES_PROCEDURE
        .resolver
        .expect("the shipped procedure definition carries its resolver"),
)];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. `facts -native ID`: checked, not
/// stored — nothing reads a pack's facts yet — so the table records only
/// which ids are shipped.
pub const FACTS_NATIVE: &[(&str, ())] = &[];
