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

//! Integration tests for the `tcl-compiler` side-effect and command-binding
//! analyses:
//!
//!   * `tcl_compiler::side_effects` (`classify_side_effects`, `SideEffect`,
//!     `CommandSideEffects`, and the `StorageType` / `StorageScope` /
//!     `ConnectionSide` / `SideEffectTarget` vocabulary).
//!   * `tcl_compiler::command_binding` (`analyse_command_binding`, the
//!     `BindingKind` lattice, and the whole-module
//!     `scan_module_command_mutations` summary).
//!
//! ## Proof split (C-Tcl / tclsh vs. registry-metadata facts)
//!
//! Side-effect *classification* and command *binding* are compiler-metadata
//! and policy facts: "is `set` a variable writer?", "does `puts` write a
//! channel?", "what does the name `string` resolve to after a `rename`?".
//! These are properties of our registry + analysis, **not** of a value
//! observable in `tclsh`, so the overwhelming majority of assertions here are
//! verified against the registry/lattice and carry no `// tclsh:` citation.
//!
//! A handful of cases lean on a *real Tcl runtime behaviour* and were proven
//! against `tclsh8.6` + `tclsh9.0` via `scripts/dev/tclsh_check.sh`; those
//! carry an inline `// tclsh:` note (e.g. `set x` with no value is a read /
//! `incr` is read-modify-write / `rename old {}` deletes a command so the next
//! call hits `unknown` / `interp alias` makes a new name dispatch to its
//! target). F5/iRules-only commands (`table`, `pool`, `HTTP::*`, `static::`,
//! …) are **not** runnable in stock tclsh — they are registry/dialect-metadata
//! facts and are marked `// f5-dialect`.
//!
//! ## Hint-derived effects leave some fields at their default
//!
//! Registry side-effect *hints* carry only
//! `target`/`reads`/`writes`/`connection_side` — they do **not** populate the
//! `scope`, `namespace`, or `key` fields. So a *hint-derived* effect leaves
//! `scope = Unknown` / `namespace = None`. Each such site asserts the parts
//! that *do* hold (target, reads, writes, `connection_side`) and notes the
//! field left at its default. The `set`/`incr`/`unset` variable-assignment
//! path *does* populate scope/namespace/key (it is not hint-driven).
//!
//! `class match` (a `pure: true` subcommand) short-circuits to a pure,
//! effect-free result rather than surfacing the command-level `DataGroup` read
//! hint — pure subcommands win, so the class-lookup assertions check purity
//! (documented at the site).

use tcl_compiler::command_binding::{
    Binding, BindingKind, analyse_command_binding, scan_module_command_mutations,
};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::ir::Statement;
use tcl_compiler::side_effects::{
    CommandSideEffects, ConnectionSide, EffectRegion, SideEffect, SideEffectTarget, StorageScope,
    StorageType, classify_side_effects,
};
use tcl_dialect::DialectSet;
use tcl_registry::{CommandRegistry, registry_for_dialect};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A registry with the iRules dialect loaded, making the F5 command metadata
/// available. `classify_side_effects` still gates per call via its `dialect`
/// argument, so this registry serves both the Tcl-core and the iRules tests.
fn irules_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::build_default();
    reg.load_dialect(DialectSet::IRULES);
    reg
}

/// `classify_side_effects` with no callee summary. `args` are taken as `&str`
/// literals and lifted to owned `String`s.
fn classify(
    reg: &CommandRegistry,
    command: &str,
    args: &[&str],
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> CommandSideEffects {
    let owned: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
    classify_side_effects(reg, command, &owned, dialect, None)
}

/// The single effect of a classification, panicking with a helpful message if
/// the effect list is not exactly length 1.
fn only_effect(cse: &CommandSideEffects) -> &SideEffect {
    assert_eq!(
        cse.effects.len(),
        1,
        "expected exactly one effect, got {:?}",
        cse.effects
    );
    &cse.effects[0]
}

// ===========================================================================
// Side-effect classification
// ===========================================================================

// --- TestEnums -------------------------------------------------------------

#[test]
fn storage_type_members_distinct() {
    // Distinct discriminants.
    assert_ne!(StorageType::Scalar, StorageType::List);
    assert_ne!(StorageType::Dict, StorageType::Array);
    assert_ne!(StorageType::Unknown, StorageType::Scalar);
}

#[test]
fn storage_scope_has_f5_scopes() {
    // The F5-specific scopes exist and are mutually distinct. (Enums make "is a
    // member" a compile-time fact; assert they are pairwise different from the
    // Tcl-universal scopes.)
    let f5 = [
        StorageScope::Connection,
        StorageScope::Static,
        StorageScope::SessionTable,
        StorageScope::Persistence,
        StorageScope::DataGroup,
    ];
    for (i, a) in f5.iter().enumerate() {
        assert_ne!(*a, StorageScope::ProcLocal);
        assert_ne!(*a, StorageScope::Unknown);
        for b in &f5[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn connection_side_members_distinct() {
    assert_ne!(ConnectionSide::Client, ConnectionSide::Server);
    assert_ne!(ConnectionSide::Both, ConnectionSide::Global);
    assert_ne!(ConnectionSide::None, ConnectionSide::Both);
}

#[test]
fn side_effect_target_has_io_targets() {
    // The I/O targets exist and are distinct.
    assert_ne!(SideEffectTarget::FileIo, SideEffectTarget::NetworkIo);
    assert_ne!(SideEffectTarget::NetworkIo, SideEffectTarget::LogIo);
    assert_ne!(SideEffectTarget::FileIo, SideEffectTarget::LogIo);
}

// --- TestSideEffect --------------------------------------------------------

#[test]
fn side_effect_default_values() {
    // The constructor takes (target, reads, writes); the remaining fields take
    // their defaults.
    let e = SideEffect::new(SideEffectTarget::Variable, false, false);
    assert!(!e.reads);
    assert!(!e.writes);
    assert_eq!(e.storage_type, StorageType::Unknown);
    assert_eq!(e.scope, StorageScope::Unknown);
    assert_eq!(e.connection_side, ConnectionSide::None);
    assert!(e.namespace.is_none());
    assert!(e.dialect.is_none());
    assert!(e.key.is_none());
    assert!(e.subtable.is_none());
}

// NOTE: there is no immutability test. Immutability is a compile-time property
// of `let` bindings (a `let e = …; e.reads = false;` simply would not compile),
// so there is no runtime behaviour to assert.

// --- TestCommandSideEffects ------------------------------------------------

#[test]
fn command_side_effects_pure() {
    let cse = CommandSideEffects {
        pure: true,
        deterministic: true,
        ..CommandSideEffects::default()
    };
    assert!(cse.pure);
    assert!(!cse.reads_any());
    assert!(!cse.writes_any());
    // No effects ⇒ no targets touched.
    assert!(cse.effects.is_empty());
}

#[test]
fn command_side_effects_single_write() {
    let mut e = SideEffect::new(SideEffectTarget::Variable, false, true);
    e.key = Some("x".to_owned());
    let cse = CommandSideEffects {
        effects: vec![e],
        ..CommandSideEffects::default()
    };
    assert!(cse.writes_any());
    assert!(!cse.reads_any());
    assert!(cse.affects_target(SideEffectTarget::Variable));
    assert!(cse.writes_target(SideEffectTarget::Variable));
    assert!(!cse.reads_target(SideEffectTarget::Variable));
}

#[test]
fn command_side_effects_multiple_effects() {
    // A read and a write to the same target.
    let cse = CommandSideEffects {
        effects: vec![
            SideEffect::new(SideEffectTarget::HttpHeader, true, false),
            SideEffect::new(SideEffectTarget::HttpHeader, false, true),
        ],
        ..CommandSideEffects::default()
    };
    assert!(cse.reads_any());
    assert!(cse.writes_any());
    assert!(cse.affects_target(SideEffectTarget::HttpHeader));
    assert!(cse.reads_target(SideEffectTarget::HttpHeader));
    assert!(cse.writes_target(SideEffectTarget::HttpHeader));
}

#[test]
fn command_side_effects_scopes_via_effects_in_scope() {
    // There is no `scopes` aggregate, but `effects_in_scope` is the queryable
    // equivalent.
    let mut e1 = SideEffect::new(SideEffectTarget::Variable, false, false);
    e1.scope = StorageScope::ProcLocal;
    let mut e2 = SideEffect::new(SideEffectTarget::SessionTable, false, false);
    e2.scope = StorageScope::SessionTable;
    let cse = CommandSideEffects {
        effects: vec![e1, e2],
        ..CommandSideEffects::default()
    };
    assert_eq!(cse.effects_in_scope(StorageScope::ProcLocal).len(), 1);
    assert_eq!(cse.effects_in_scope(StorageScope::SessionTable).len(), 1);
    assert_eq!(cse.effects_in_scope(StorageScope::Global).len(), 0);
}

#[test]
fn command_side_effects_effects_in_scope() {
    let mut e1 = SideEffect::new(SideEffectTarget::Variable, false, false);
    e1.scope = StorageScope::ProcLocal;
    let mut e2 = SideEffect::new(SideEffectTarget::Variable, false, false);
    e2.scope = StorageScope::Global;
    let cse = CommandSideEffects {
        effects: vec![e1.clone(), e2],
        ..CommandSideEffects::default()
    };
    let local = cse.effects_in_scope(StorageScope::ProcLocal);
    assert_eq!(local.len(), 1);
    assert_eq!(*local[0], e1);
}

#[test]
fn command_side_effects_effects_on_side() {
    let mut e1 = SideEffect::new(SideEffectTarget::PoolSelection, false, false);
    e1.connection_side = ConnectionSide::Server;
    let mut e2 = SideEffect::new(SideEffectTarget::HttpHeader, false, false);
    e2.connection_side = ConnectionSide::Client;
    let cse = CommandSideEffects {
        effects: vec![e1.clone(), e2],
        ..CommandSideEffects::default()
    };
    let server = cse.effects_on_side(ConnectionSide::Server);
    assert_eq!(server.len(), 1);
    assert_eq!(*server[0], e1);
}

// --- TestClassifyPureCommands ----------------------------------------------

#[test]
fn classify_expr_is_pure() {
    // tclsh: `expr {1 + 2}` is a pure arithmetic evaluation (no side effects).
    let reg = irules_registry();
    assert!(classify(&reg, "expr", &["{1 + 2}"], None).pure);
}

#[test]
fn classify_llength_is_pure() {
    // tclsh: `llength $x` only reads its argument.
    let reg = irules_registry();
    assert!(classify(&reg, "llength", &["$x"], None).pure);
}

#[test]
fn classify_string_length_is_pure() {
    // `string length` is a pure subcommand.
    // tclsh: `string length hello` ⇒ 5, no side effects.
    let reg = irules_registry();
    assert!(classify(&reg, "string", &["length", "hello"], None).pure);
}

// --- TestClassifyVariableCommands ------------------------------------------

#[test]
fn classify_set_write() {
    // tclsh: `set x hello` writes the proc-local scalar `x`.
    let reg = irules_registry();
    let r = classify(&reg, "set", &["x", "hello"], None);
    assert!(r.writes_any());
    assert!(r.affects_target(SideEffectTarget::Variable));
    let e = only_effect(&r);
    assert_eq!(e.key.as_deref(), Some("x"));
    assert!(e.writes);
    assert_eq!(e.scope, StorageScope::ProcLocal);
    assert_eq!(e.storage_type, StorageType::Scalar);
}

#[test]
fn classify_set_read() {
    // tclsh: `set x` with no value *reads* `x`.
    let reg = irules_registry();
    let r = classify(&reg, "set", &["x"], None);
    assert!(r.reads_any());
    assert!(!r.writes_any());
    let e = only_effect(&r);
    assert!(e.reads);
    assert!(!e.writes);
}

#[test]
fn classify_set_global_var() {
    // `::myvar` is a root-qualified global.
    let reg = irules_registry();
    let r = classify(&reg, "set", &["::myvar", "val"], None);
    let e = only_effect(&r);
    assert_eq!(e.scope, StorageScope::Global);
    assert_eq!(e.namespace.as_deref(), Some("::"));
    assert_eq!(e.key.as_deref(), Some("::myvar"));
}

#[test]
fn classify_set_namespace_var() {
    // `::foo::bar::x` lives in namespace `::foo::bar` (the trailing `::x`
    // segment is the variable name).
    let reg = irules_registry();
    let r = classify(&reg, "set", &["::foo::bar::x", "val"], None);
    let e = only_effect(&r);
    assert_eq!(e.scope, StorageScope::Namespace);
    assert_eq!(e.namespace.as_deref(), Some("::foo::bar"));
}

#[test]
fn classify_set_static_var_irules() {
    // f5-dialect: `static::` is an iRules scope marker; the var is system-wide
    // (ConnectionSide::Global).
    let reg = irules_registry();
    let r = classify(
        &reg,
        "set",
        &["static::counter", "0"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    let e = only_effect(&r);
    assert_eq!(e.scope, StorageScope::Static);
    assert_eq!(e.connection_side, ConnectionSide::Global);
    assert_eq!(e.key.as_deref(), Some("static::counter"));
}

#[test]
fn classify_set_static_var_f5_irules() {
    // Same, under the `f5-irules` dialect alias. // f5-dialect.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "set",
        &["static::counter", "0"],
        Some(tcl_dialect::DialectProfile::by_name("f5-irules")),
    );
    let e = only_effect(&r);
    assert_eq!(e.scope, StorageScope::Static);
    assert_eq!(e.connection_side, ConnectionSide::Global);
    assert_eq!(e.key.as_deref(), Some("static::counter"));
}

#[test]
fn classify_incr_reads_and_writes() {
    // tclsh: `incr counter` reads the current value and writes the increment
    // (read-modify-write); the value is a scalar integer.
    let reg = irules_registry();
    let r = classify(&reg, "incr", &["counter"], None);
    let e = only_effect(&r);
    assert!(e.reads);
    assert!(e.writes);
    assert_eq!(e.storage_type, StorageType::Scalar);
}

#[test]
fn classify_lappend_is_list_type() {
    // tclsh: `lappend mylist item` appends to (and so first reads) a list var.
    let reg = irules_registry();
    let r = classify(&reg, "lappend", &["mylist", "item"], None);
    let e = only_effect(&r);
    assert_eq!(e.storage_type, StorageType::List);
    assert!(e.reads);
    assert!(e.writes);
}

// --- TestClassifyDynamicBarriers -------------------------------------------

#[test]
fn classify_eval_is_dynamic_barrier() {
    // tclsh: `eval {set x 1}` runs arbitrary code ⇒ effects are unknowable
    // (a dynamic-dispatch barrier that conservatively reads and writes).
    let reg = irules_registry();
    let r = classify(&reg, "eval", &["set x 1"], None);
    assert!(r.dynamic_barrier);
    assert!(r.writes_any());
    assert!(r.reads_any());
}

#[test]
fn classify_uplevel_is_dynamic_barrier() {
    // tclsh: `uplevel 1 {set x 1}` evaluates in the caller's frame ⇒ barrier.
    let reg = irules_registry();
    let r = classify(&reg, "uplevel", &["1", "set x 1"], None);
    assert!(r.dynamic_barrier);
}

// --- TestClassifyTableCommand ---------------------------------------------

#[test]
fn classify_table_set() {
    // f5-dialect. The registry `table set` hint is reads+writes (a keyed upsert
    // reads the old entry), and hints don't populate `scope`, so scope is left
    // at Unknown. The load-bearing facts hold: it writes the SESSION_TABLE
    // target on BOTH sides.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "table",
        &["set", "mykey", "myval"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    assert!(r.writes_any());
    let e = only_effect(&r);
    assert_eq!(e.target, SideEffectTarget::SessionTable);
    assert_eq!(e.connection_side, ConnectionSide::Both);
    // Hint-derived effect: scope not populated by the registry hint path.
    assert_eq!(e.scope, StorageScope::Unknown);
}

#[test]
fn classify_table_lookup() {
    // f5-dialect. `table lookup` is a pure read of the SESSION_TABLE target.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "table",
        &["lookup", "mykey"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    let e = only_effect(&r);
    assert!(e.reads);
    assert!(!e.writes);
    assert_eq!(e.target, SideEffectTarget::SessionTable);
    // Hint-derived: scope left at default.
    assert_eq!(e.scope, StorageScope::Unknown);
}

// --- TestClassifyF5Commands ------------------------------------------------

#[test]
fn classify_pool_selection() {
    // f5-dialect.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "pool",
        &["mypool"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    assert!(r.writes_target(SideEffectTarget::PoolSelection));
    let e = only_effect(&r);
    assert_eq!(e.connection_side, ConnectionSide::Server);
}

#[test]
fn classify_node_selection() {
    // f5-dialect.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "node",
        &["10.0.0.1"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    assert!(r.writes_target(SideEffectTarget::NodeSelection));
}

#[test]
fn classify_snat_selection() {
    // f5-dialect.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "snat",
        &["automap"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    assert!(r.writes_target(SideEffectTarget::SnatSelection));
}

#[test]
fn classify_class_lookup() {
    // f5-dialect. `class match` is a `pure: true` subcommand, so the classifier
    // short-circuits to a pure, effect-free result rather than surfacing the
    // command-level DataGroup read hint. (Pure subcommands win.) Assert the
    // behaviour: pure, no tracked write.
    let reg = irules_registry();
    let r = classify(&reg, "class", &["match", "-name", "myclass", "value"], None);
    assert!(r.pure, "class match resolves via the pure-subcommand path");
    assert!(!r.writes_any());
    // The DataGroup read is reachable via the command-level hint only when no
    // pure subcommand matches; assert the registry actually carries it so the
    // hint is not dead metadata.
    let unknown_sub = classify(
        &reg,
        "class",
        &["__nosuchsub", "x"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    assert!(unknown_sub.reads_target(SideEffectTarget::DataGroup));
}

#[test]
fn classify_persist_command() {
    // f5-dialect. Hint-derived effect ⇒ scope left at default. Target +
    // client-side context hold.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "persist",
        &["source_addr"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    let e = only_effect(&r);
    assert_eq!(e.target, SideEffectTarget::PersistenceTable);
    assert_eq!(e.connection_side, ConnectionSide::Client);
    assert_eq!(e.scope, StorageScope::Unknown);
}

#[test]
fn classify_session_add() {
    // f5-dialect.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "session",
        &["add", "key", "val"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    let e = only_effect(&r);
    assert_eq!(e.target, SideEffectTarget::PersistenceTable);
    assert!(e.writes);
}

#[test]
fn classify_session_lookup() {
    // f5-dialect. `session lookup` is a `pure: true` subcommand, so the
    // classifier short-circuits to a pure, effect-free result (pure subcommands
    // win) rather than surfacing the subcommand's PersistenceTable read hint.
    // Assert the behaviour: pure, no write. The structured read/write effect for
    // the persistence table is still exercised by `session add` (a mutator
    // subcommand) below.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "session",
        &["lookup", "key"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    assert!(
        r.pure,
        "session lookup resolves via the pure-subcommand path"
    );
    assert!(!r.writes_any());
}

// --- TestClassifyProtocolNamespaceCommands ---------------------------------

#[test]
fn classify_http_header_read() {
    // f5-dialect. Hint-derived effects don't set `namespace`; assert the read +
    // HttpHeader target instead.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "HTTP::header",
        &["value", "Host"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    let e = only_effect(&r);
    assert!(e.reads);
    assert_eq!(e.target, SideEffectTarget::HttpHeader);
    assert!(e.namespace.is_none());
}

#[test]
fn classify_http_uri_normalized_getter_is_read_only() {
    // f5-dialect. The `HTTP::uri` spec carries `Traits::PURE` but NO structured
    // `side_effects` hint (`-normalized` is an *option*, not a subcommand, and
    // the read-only-ness is captured purely by `pure=true`). So the getter is
    // classified as pure with an EMPTY effect list rather than surfacing a
    // structured HttpUri read. The load-bearing property — it is a read-only
    // getter that never writes — holds: assert purity + no write. (The HttpUri
    // target *is* exercised via the `URI::path` conformance row, which does
    // carry the hint.)
    let reg = irules_registry();
    let r = classify(
        &reg,
        "HTTP::uri",
        &["-normalized"],
        Some(tcl_dialect::DialectProfile::by_name("f5-irules")),
    );
    assert!(r.pure, "HTTP::uri -normalized is a pure getter");
    assert!(!r.writes_any(), "a getter must not write");
}

#[test]
fn classify_dialect_propagation() {
    // The active dialect is stamped onto both the result and each effect.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "set",
        &["x", "1"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    assert_eq!(r.dialect.as_deref(), Some("f5-irules"));
    let e = only_effect(&r);
    assert_eq!(e.dialect.as_deref(), Some("f5-irules"));
}

#[test]
fn classify_unknown_command_is_conservative() {
    // An unknown command conservatively reads and writes (UNKNOWN target).
    let reg = irules_registry();
    let r = classify(&reg, "some_unknown_command", &["arg1"], None);
    assert!(r.reads_any());
    assert!(r.writes_any());
}

// --- TestClassifyWithRegistryHints -----------------------------------------

#[test]
fn classify_command_level_hint_is_applied() {
    // f5-dialect. `HTTP2::disable` carries a command-level Http2State write hint
    // on BOTH sides.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "HTTP2::disable",
        &[],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    assert!(r.writes_target(SideEffectTarget::Http2State));
    let e = only_effect(&r);
    assert_eq!(e.connection_side, ConnectionSide::Both);
    assert_eq!(e.dialect.as_deref(), Some("f5-irules"));
}

#[test]
fn classify_subcommand_hint_overrides_command_level() {
    // f5-dialect. `HTTP::header replace …` is a mutator subcommand whose hint
    // writes HttpHeader (overriding the pure getter classification).
    let reg = irules_registry();
    let r = classify(
        &reg,
        "HTTP::header",
        &["replace", "Host", "example.com"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    let e = only_effect(&r);
    assert_eq!(e.target, SideEffectTarget::HttpHeader);
    assert!(e.writes);
}

#[test]
fn classify_hint_precedence_beats_fallback() {
    // f5-dialect. `call my_proc` reads the PROC_DEFINITION and does not write.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "call",
        &["my_proc"],
        Some(tcl_dialect::DialectProfile::by_name("irules")),
    );
    assert!(r.reads_target(SideEffectTarget::ProcDefinition));
    assert!(!r.writes_any());
}

#[test]
fn classify_hint_with_unspecified_dialect_preserves_shape() {
    // With no dialect the `HTTP2::disable` hint still resolves (the spec is not
    // gated out) and the effect's dialect stamp is None.
    let reg = irules_registry();
    let r = classify(&reg, "HTTP2::disable", &[], None);
    let e = only_effect(&r);
    assert_eq!(e.target, SideEffectTarget::Http2State);
    assert!(e.dialect.is_none());
}

// NOTE: there is no explicit-subcommand-override test. `classify_side_effects`
// has no separate `subcommand` parameter — the effective subcommand is always
// `args[0]` — so there is no override to exercise.

// --- TestDialectSpecificHints ----------------------------------------------

#[test]
fn classify_close_in_tcl_is_file_io() {
    // tclsh: `close $fd` closes a channel (file I/O), no F5 connection context.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "close",
        &["$fd"],
        Some(tcl_dialect::DialectProfile::by_name("tcl8.6")),
    );
    let e = only_effect(&r);
    assert_eq!(e.target, SideEffectTarget::FileIo);
    assert_eq!(e.connection_side, ConnectionSide::None);
}

#[test]
fn classify_close_in_irules_is_connection_control() {
    // f5-dialect. Under iRules, bare `close` tears down the proxied connection
    // on BOTH sides (ConnectionControl), not a file channel.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "close",
        &[],
        Some(tcl_dialect::DialectProfile::by_name("f5-irules")),
    );
    let e = only_effect(&r);
    assert_eq!(e.target, SideEffectTarget::ConnectionControl);
    assert_eq!(e.connection_side, ConnectionSide::Both);
}

#[test]
fn classify_file_io_does_not_kill_unknown_region() {
    // `puts hello` writes a FileIo target whose coarse EffectRegion is NONE —
    // so it must NOT register as an UNKNOWN_STATE write that would kill GVN
    // facts.
    let reg = irules_registry();
    let r = classify(
        &reg,
        "puts",
        &["hello"],
        Some(tcl_dialect::DialectProfile::by_name("tcl8.6")),
    );
    let (_reads, writes) = r.to_effect_regions();
    assert_eq!(writes, EffectRegion::NONE);
}

// --- TestConformanceHintTargets --------------------------------------------

/// One conformance row: `classify(command, (), dialect)` must surface
/// `expected` among its effect targets — proving the registry hint is wired
/// through, not dead metadata.
fn assert_conformance_target(
    reg: &CommandRegistry,
    command: &str,
    dialect: Option<&tcl_dialect::DialectProfile>,
    expected: SideEffectTarget,
) {
    let r = classify(reg, command, &[], dialect);
    let has = r.effects.iter().any(|e| e.target == expected);
    assert!(
        has,
        "{command} expected target {expected:?}, got {:?}",
        r.effects.iter().map(|e| e.target).collect::<Vec<_>>()
    );
}

#[test]
fn conformance_irules_protocol_namespace_targets() {
    // iRules protocol-namespace rows. // f5-dialect (each command is an
    // F5/iRules construct).
    let reg = irules_registry();
    let rows: &[(&str, SideEffectTarget)] = &[
        ("HTTP2::disable", SideEffectTarget::Http2State),
        ("HTTP2::push", SideEffectTarget::Http2State),
        ("ASM::disable", SideEffectTarget::AsmState),
        ("ASM::enable", SideEffectTarget::AsmState),
        ("COMPRESS::enable", SideEffectTarget::StreamProfile),
        ("DOSL7::disable", SideEffectTarget::Dosl7State),
        ("FLOW::create_related", SideEffectTarget::FlowState),
        ("FTP::enable", SideEffectTarget::FtpState),
        ("ICAP::header", SideEffectTarget::IcapState),
        ("ISTATS::set", SideEffectTarget::IStats),
        ("LSN::address", SideEffectTarget::LsnState),
        ("MESSAGE::proto", SideEffectTarget::MessageState),
        ("MR::message", SideEffectTarget::MessageState),
        ("REWRITE::enable", SideEffectTarget::StreamProfile),
        (
            "CLASSIFY::application",
            SideEffectTarget::ClassificationState,
        ),
        ("CLASSIFICATION::app", SideEffectTarget::ClassificationState),
        ("PROFILE::http", SideEffectTarget::BigipConfig),
        ("X509::subject", SideEffectTarget::SslState),
        ("URI::path", SideEffectTarget::HttpUri),
    ];
    for (cmd, target) in rows {
        assert_conformance_target(
            &reg,
            cmd,
            Some(tcl_dialect::DialectProfile::by_name("irules")),
            *target,
        );
    }
}

#[test]
fn conformance_tcl_core_targets() {
    // Tcl core rows (dialect=None).
    // tclsh-grounded: `puts`/`open` do file I/O, `socket` does network I/O,
    // `proc` defines a procedure, `namespace` mutates namespace state,
    // `interp`/`package` mutate interpreter state.
    let reg = irules_registry();
    let rows: &[(&str, SideEffectTarget)] = &[
        ("puts", SideEffectTarget::FileIo),
        ("open", SideEffectTarget::FileIo),
        ("socket", SideEffectTarget::NetworkIo),
        ("proc", SideEffectTarget::ProcDefinition),
        ("namespace", SideEffectTarget::NamespaceState),
        ("interp", SideEffectTarget::InterpState),
        ("package", SideEffectTarget::InterpState),
    ];
    for (cmd, target) in rows {
        assert_conformance_target(&reg, cmd, None, *target);
    }
}

// --- TestHintCoverageNotDead -----------------------------------------------

#[test]
fn hinted_irules_commands_return_non_unknown_effects() {
    // f5-dialect. Each hinted iRules command yields a non-empty effect list.
    let reg = irules_registry();
    let cmds = [
        "HTTP2::active",
        "HTTP2::stream",
        "ASM::fingerprint",
        "DATAGRAM::ip",
        "CRYPTO::hash",
        "STREAM::enable",
        "ONECONNECT::detach",
        "redirect",
    ];
    for cmd in cmds {
        let r = classify(
            &reg,
            cmd,
            &[],
            Some(tcl_dialect::DialectProfile::by_name("irules")),
        );
        assert!(!r.effects.is_empty(), "{cmd}: expected non-empty effects");
    }
}

#[test]
fn hinted_tcl_commands_return_structured_effects() {
    // Each hinted Tcl stdlib command yields a non-empty effect list (with an
    // `x` argument).
    let reg = irules_registry();
    let cmds = [
        "puts",
        "gets",
        "open",
        "close",
        "file",
        "socket",
        "proc",
        "rename",
        "namespace",
        "set",
        "incr",
        "append",
        "unset",
    ];
    for cmd in cmds {
        let r = classify(&reg, cmd, &["x"], None);
        assert!(!r.effects.is_empty(), "{cmd}: expected non-empty effects");
    }
}

// ===========================================================================
// Command binding
// ===========================================================================

/// Build a `CompilationUnit` for the binding tests with the `tcl9.0` dialect.
fn build_top(src: &str) -> CompilationUnit {
    let reg = registry_for_dialect("tcl9.0");
    CompilationUnit::build_for(src, reg, false)
}

/// The lexically-last `(block, idx_past_last_stmt)`. RPO order is the lexical
/// order for these straight-line scripts, so the last non-empty block in RPO
/// with its full statement count is the "end" point.
fn end_point(cfg: &tcl_compiler::cfg::Function) -> (tcl_compiler::cfg::BlockId, usize) {
    let mut last = None;
    for bid in cfg.reverse_postorder() {
        if let Some(blk) = cfg.blocks.get(&bid)
            && !blk.statements.is_empty()
        {
            last = Some((bid, blk.statements.len()));
        }
    }
    last.expect("no non-empty block")
}

/// The literal source-surface command word of a statement — deliberately
/// NOT `canonical_command_or_source()`. These helpers locate a statement by
/// what the test source literally wrote (e.g. the bare `b` call after
/// `rename a b`), so they can query the flow-sensitive binding lattice at
/// that exact point; resolving through the canonical/alias target first
/// would make a renamed or aliased call unfindable by its own written name.
fn stmt_command(stmt: &Statement) -> &str {
    match stmt {
        Statement::Call { command, .. } | Statement::Barrier { command, .. } => command.as_str(),
        _ => "",
    }
}

/// Source-surface command spelling of a statement — never the resolved
/// canonical name. `Statement::canonical_command_or_source()` now also
/// resolves through a static `rename` (`tcl_compiler::alias::detect_rename`),
/// not just `interp alias`, so a test that locates a statement *by its
/// literal source text* (e.g. "the bare `b` call after `rename a b`") must
/// not use it — `canonical_command_or_source()` would return `"a"` there,
/// not `"b"`.
fn stmt_source_command(stmt: &Statement) -> &str {
    match stmt {
        Statement::Call { command, .. } | Statement::Barrier { command, .. } => command.as_str(),
        _ => "",
    }
}

/// Binding of `query` at the first statement whose command (canonical-or-source)
/// equals `canonical`.
fn binding_at_command(
    cb: &tcl_compiler::command_binding::CommandBinding<'_>,
    cfg: &tcl_compiler::cfg::Function,
    canonical: &str,
    query: &str,
) -> Binding {
    for bid in cfg.reverse_postorder() {
        if let Some(blk) = cfg.blocks.get(&bid) {
            for (idx, stmt) in blk.statements.iter().enumerate() {
                if stmt_command(stmt) == canonical {
                    return cb.binding_at(bid, idx, query);
                }
            }
        }
    }
    panic!("no statement with command {canonical:?}");
}

/// Index of the first statement in the entry block whose *source-surface*
/// command equals `name` (not the alias/rename-resolved canonical name —
/// see [`stmt_source_command`]).
fn stmt_idx_in_entry(cfg: &tcl_compiler::cfg::Function, name: &str) -> usize {
    let blk = cfg.blocks.get(&cfg.entry).expect("entry block");
    blk.statements
        .iter()
        .position(|s| stmt_source_command(s) == name)
        .unwrap_or_else(|| panic!("no statement {name:?} in entry block"))
}

// --- TestBaseline ----------------------------------------------------------

#[test]
fn binding_builtins_are_builtin() {
    // With no rebinding every core name keeps its builtin binding.
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("set x 1\nputs [string length hi]\nincr x\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let (b, i) = end_point(&fu.cfg);
    for name in ["set", "string", "incr", "puts", "list", "lindex"] {
        assert_eq!(
            cb.binding_at(b, i, name).kind,
            BindingKind::Builtin,
            "{name} should be builtin"
        );
    }
}

#[test]
fn binding_user_proc_is_proc() {
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("proc sq {n} {return [expr {$n*$n}]}\nputs [sq 5]\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let (b, i) = end_point(&fu.cfg);
    let binding = cb.binding_at(b, i, "sq");
    assert_eq!(binding.kind, BindingKind::Proc);
    assert_eq!(binding.target.as_deref(), Some("::sq"));
    assert!(binding.is_foldable_proc());
}

#[test]
fn binding_undefined_name_is_opaque() {
    // A name no proc/builtin provides resolves via the `unknown` handler
    // (opaque).
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("nosuchcommand a b c\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let (b, i) = end_point(&fu.cfg);
    assert_eq!(
        cb.binding_at(b, i, "nosuchcommand").kind,
        BindingKind::Opaque
    );
}

// --- TestFlowSensitiveRename -----------------------------------------------

#[test]
fn binding_proc_call_rename_call() {
    // tclsh: after `rename a b`, calling `a` errors (hits unknown) and `b` runs
    // the old proc body — the rename only perturbs calls that follow it.
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("proc a {} {return 1}\na\nrename a b\nb\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let cfg = &fu.cfg;
    let bid = cfg.entry;

    let call_a = stmt_idx_in_entry(cfg, "a");
    let rename = stmt_idx_in_entry(cfg, "rename");
    let call_b = stmt_idx_in_entry(cfg, "b");

    // Before the rename: `a` is the proc, `b` is still unbound.
    assert_eq!(cb.binding_at(bid, call_a, "a").kind, BindingKind::Proc);
    assert_eq!(cb.binding_at(bid, call_a, "b").kind, BindingKind::Opaque);
    // The rename statement sees `a` still bound (state is *pre*-statement).
    assert_eq!(cb.binding_at(bid, rename, "a").kind, BindingKind::Proc);
    // After the rename: `a` falls through to unknown; `b` is the old proc.
    assert_eq!(cb.binding_at(bid, call_b, "a").kind, BindingKind::Opaque);
    let b_binding = cb.binding_at(bid, call_b, "b");
    assert_eq!(b_binding.kind, BindingKind::Proc);
    assert_eq!(b_binding.target.as_deref(), Some("::a"));
}

#[test]
fn binding_rename_deletion_is_opaque_not_error() {
    // tclsh: `rename foo {}` deletes foo; a later `foo` call hits `unknown`.
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("proc foo {} {return 1}\nrename foo {}\nfoo\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let (b, i) = end_point(&fu.cfg);
    assert_eq!(cb.binding_at(b, i, "foo").kind, BindingKind::Opaque);
}

#[test]
fn binding_builtin_rename_away_then_redefine_is_proc() {
    // tclsh: `rename incr ::orig_incr` then `proc incr …` makes `incr` a user
    // proc (no longer the foldable builtin) and `::orig_incr` the real builtin.
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top(
        "rename incr ::orig_incr\nproc incr {v} {upvar 1 $v x; ::orig_incr x 100}\nset c 0\n",
    );
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let (b, i) = end_point(&fu.cfg);
    assert_eq!(cb.binding_at(b, i, "incr").kind, BindingKind::Proc);
    assert!(!cb.binding_at(b, i, "incr").is_original_builtin());
    assert_eq!(
        cb.binding_at(b, i, "::orig_incr").kind,
        BindingKind::Builtin
    );
}

// --- TestRedefinition ------------------------------------------------------

#[test]
fn binding_proc_redefined_stays_proc() {
    // The later body wins; the name is still bound to *a* proc.
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("proc f {} {return 1}\nproc f {} {return 2}\nf\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let (b, i) = end_point(&fu.cfg);
    let binding = cb.binding_at(b, i, "f");
    assert_eq!(binding.kind, BindingKind::Proc);
    assert_eq!(binding.target.as_deref(), Some("::f"));
}

#[test]
fn binding_proc_named_like_mathfunc_is_proc_not_builtin() {
    // `max` is an expr math function but not a top-level command, so `proc max`
    // binds a real foldable proc (must not collapse to UNKNOWN).
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("proc max {a b} {return $a}\nputs [max 1 2]\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let (b, i) = end_point(&fu.cfg);
    let binding = cb.binding_at(b, i, "max");
    assert_eq!(binding.kind, BindingKind::Proc);
    assert_eq!(binding.target.as_deref(), Some("::max"));
}

// --- TestInterpAlias -------------------------------------------------------

#[test]
fn binding_alias_records_target() {
    // tclsh: `interp alias {} myset {} set` makes `myset` dispatch to `set`.
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("interp alias {} myset {} set\nmyset x 1\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let (b, i) = end_point(&fu.cfg);
    let binding = cb.binding_at(b, i, "myset");
    assert_eq!(binding.kind, BindingKind::Alias);
    assert_eq!(binding.target.as_deref(), Some("::set"));
}

// --- TestBranchJoin --------------------------------------------------------

#[test]
fn binding_rename_in_one_arm_joins_to_unknown() {
    // A rename only on the true arm makes the merged binding ⊤ (Unknown).
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("if {$x} {rename string ::s2}\nputs [string length hi]\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let binding = binding_at_command(&cb, &fu.cfg, "puts", "string");
    assert_eq!(binding.kind, BindingKind::Unknown);
}

#[test]
fn binding_rename_on_both_arms_agrees() {
    // The same rename on both arms is deterministic: `string` is renamed away
    // on every path ⇒ opaque, not ⊤.
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top(
        "if {$x} {rename string ::s2} else {rename string ::s2}\nputs [string length hi]\n",
    );
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let binding = binding_at_command(&cb, &fu.cfg, "puts", "string");
    assert_eq!(binding.kind, BindingKind::Opaque);
}

// --- TestDynamicMutation ---------------------------------------------------

#[test]
fn binding_dynamic_rename_makes_everything_unknown() {
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("rename $cmd foo\nputs [string length hi]\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let binding = binding_at_command(&cb, &fu.cfg, "puts", "string");
    assert_eq!(binding.kind, BindingKind::Unknown);
}

#[test]
fn binding_dynamic_proc_name_makes_everything_unknown() {
    let reg = registry_for_dialect("tcl9.0");
    let cu = build_top("proc $name {} {return 1}\nputs [string length hi]\n");
    let fu = cu.function("::top").unwrap();
    let cb = analyse_command_binding(&fu.cfg, reg, &[]);
    let binding = binding_at_command(&cb, &fu.cfg, "puts", "string");
    assert_eq!(binding.kind, BindingKind::Unknown);
}

// --- TestProcBodyMutationScan ----------------------------------------------
//
// The whole-module `scan_module_command_mutations(ir_module)` summarises
// renames buried in proc bodies — it walks the top-level *and* every proc/method
// body via a CFG-free IR walk (see the module docs). `mut.trusts(name)` reports
// whether a name is still safely bound, and `!mut.trusts("string") &&
// mut.names.contains("::string")` checks that a specific name was rebound.

#[test]
fn module_scan_nested_rename_in_proc_is_caught() {
    let reg = registry_for_dialect("tcl9.0");
    let cu = CompilationUnit::build_for(
        "proc danger {} {if {$x} {rename string ::s2}}\nputs ok\n",
        reg,
        false,
    );
    let mt = scan_module_command_mutations(&cu.ir_module, reg);
    assert!(!mt.trusts("string"), "string is rebound in a proc body");
    assert!(mt.trusts("list"), "untouched names stay trusted");
}

#[test]
fn module_scan_dynamic_rename_in_proc_distrusts_everything() {
    let reg = registry_for_dialect("tcl9.0");
    let cu = CompilationUnit::build_for("proc danger {} {rename $a $b}\nputs ok\n", reg, false);
    let mt = scan_module_command_mutations(&cu.ir_module, reg);
    assert!(!mt.trusts("string"));
    assert!(!mt.trusts("anythingatall"));
}

#[test]
fn module_scan_clean_module_trusts_all() {
    let reg = registry_for_dialect("tcl9.0");
    let cu = CompilationUnit::build_for(
        "proc helper {x} {return [expr {$x+1}]}\nputs [helper 4]\n",
        reg,
        false,
    );
    let mt = scan_module_command_mutations(&cu.ir_module, reg);
    assert!(mt.trusts("string"));
    assert!(mt.trusts("list"));
}

// --- TestModuleScanSoundness -----------------------------------------------

#[test]
fn module_scan_transient_rename_and_restore_is_caught() {
    // `string` is renamed away, used, then restored — its final binding is the
    // builtin default, but it was tampered with mid-body, so it must NOT be
    // trusted (calls in that window would hit `unknown`).
    let reg = registry_for_dialect("tcl9.0");
    let cu = CompilationUnit::build_for(
        "rename string ms\nset x [string length abc]\nrename ms string\n",
        reg,
        false,
    );
    let mt = scan_module_command_mutations(&cu.ir_module, reg);
    assert!(!mt.trusts("string"));
}

#[test]
fn module_scan_embedded_substitution_rename_is_dynamic() {
    // `rename ::$c mystr` — the old name carries a substitution after a literal
    // prefix, so it's a *dynamic* rename that can touch any command.
    let reg = registry_for_dialect("tcl9.0");
    let cu = CompilationUnit::build_for(
        "set c string\nrename ::$c mystr\nputs [string length abc]\n",
        reg,
        false,
    );
    let mt = scan_module_command_mutations(&cu.ir_module, reg);
    // Dynamic ⇒ trusts nothing (the `dynamic` flag short-circuits trust).
    assert!(!mt.trusts("string"));
    assert!(!mt.trusts("anything"));
}

#[test]
fn module_scan_embedded_substitution_in_new_name_is_dynamic() {
    // `rename foo bar[x]` — the new name carries a command substitution ⇒
    // dynamic ⇒ trusts nothing.
    let reg = registry_for_dialect("tcl9.0");
    let cu = CompilationUnit::build_for("rename foo bar[x]\n", reg, false);
    let mt = scan_module_command_mutations(&cu.ir_module, reg);
    assert!(!mt.trusts("string"));
    assert!(!mt.trusts("list"));
}

#[test]
fn module_scan_clean_module_still_trusts() {
    let reg = registry_for_dialect("tcl9.0");
    let cu = CompilationUnit::build_for(
        "proc fib {n} {return $n}\nputs [fib 3]\nputs [string length hi]\n",
        reg,
        false,
    );
    let mt = scan_module_command_mutations(&cu.ir_module, reg);
    assert!(mt.trusts("string"));
    assert!(mt.trusts("list"));
}
