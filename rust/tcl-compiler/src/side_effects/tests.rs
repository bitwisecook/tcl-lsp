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

use super::*;
use crate::var_resolve::VariableCellKey;
use tcl_dialect::model::{Family, SurfaceLayer};

#[test]
fn http_targets_map_to_http_state_region() {
    for target in [
        SideEffectTarget::HttpHeader,
        SideEffectTarget::HttpBody,
        SideEffectTarget::HttpStatus,
        SideEffectTarget::HttpUri,
        SideEffectTarget::HttpCookie,
        SideEffectTarget::HttpMethod,
        SideEffectTarget::Http2State,
    ] {
        assert_eq!(
            target_to_region(target, StorageScope::Unknown),
            EffectRegion::HTTP_STATE,
            "{target:?}"
        );
    }
}

#[test]
fn response_commit_is_both_response_and_http_state() {
    let r = target_to_region(SideEffectTarget::ResponseCommit, StorageScope::Unknown);
    assert!(r.contains(EffectRegion::RESPONSE_LIFECYCLE));
    assert!(r.contains(EffectRegion::HTTP_STATE));
}

#[test]
fn variable_scope_distinguishes_global_vs_local() {
    assert_eq!(
        target_to_region(SideEffectTarget::Variable, StorageScope::Global),
        EffectRegion::GLOBAL_STATE
    );
    assert_eq!(
        target_to_region(SideEffectTarget::Variable, StorageScope::Namespace),
        EffectRegion::GLOBAL_STATE
    );
    assert_eq!(
        target_to_region(SideEffectTarget::Variable, StorageScope::ProcLocal),
        EffectRegion::NONE
    );
}

#[test]
fn external_io_is_not_compiler_tracked() {
    assert_eq!(
        target_to_region(SideEffectTarget::FileIo, StorageScope::FileSystem),
        EffectRegion::NONE
    );
    assert_eq!(
        target_to_region(SideEffectTarget::NetworkIo, StorageScope::NetworkSocket),
        EffectRegion::NONE
    );
    assert_eq!(
        target_to_region(SideEffectTarget::LogIo, StorageScope::LogOutput),
        EffectRegion::NONE
    );
}

#[test]
fn unknown_targets_fall_through() {
    assert_eq!(
        target_to_region(SideEffectTarget::Unknown, StorageScope::Unknown),
        EffectRegion::UNKNOWN_STATE
    );
    assert_eq!(
        target_to_region(SideEffectTarget::SessionTable, StorageScope::Unknown),
        EffectRegion::UNKNOWN_STATE
    );
}

#[test]
fn effect_region_supports_bitwise_combine() {
    let combined = EffectRegion::HTTP_STATE | EffectRegion::GLOBAL_STATE;
    assert!(combined.contains(EffectRegion::HTTP_STATE));
    assert!(combined.contains(EffectRegion::GLOBAL_STATE));
    assert!(!combined.contains(EffectRegion::UNKNOWN_STATE));
}

#[test]
fn http_uri_family_reads_http_state_region() {
    // `HTTP::uri` / `HTTP::path` / `HTTP::query` getters carry a
    // read-only `HttpUri` side effect, so they classify into the
    // `HTTP_STATE` *read* region. Without `side_effects` on their specs,
    // callgraph/dataflow reports `NONE` instead of
    // `HTTP_STATE`.
    let mut reg = tcl_registry::CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    for cmd in ["HTTP::uri", "HTTP::path", "HTTP::query"] {
        let ci = classify_side_effects(&reg, cmd, &[], None, None);
        let (reads, writes) = ci.to_effect_regions();
        assert!(
            reads.contains(EffectRegion::HTTP_STATE),
            "{cmd}: reads should include HTTP_STATE, got {reads:?}"
        );
        assert!(
            !writes.contains(EffectRegion::HTTP_STATE),
            "{cmd}: getter form must not write HTTP_STATE, got {writes:?}"
        );
    }
}

// SideEffect / CommandSideEffects.

#[test]
fn side_effect_new_defaults() {
    let e = SideEffect::new(SideEffectTarget::Variable, true, false);
    assert_eq!(e.target, SideEffectTarget::Variable);
    assert!(e.reads);
    assert!(!e.writes);
    assert_eq!(e.storage_type, StorageType::Unknown);
    assert_eq!(e.scope, StorageScope::Unknown);
    assert_eq!(e.connection_side, ConnectionSide::None);
    assert!(e.namespace.is_none());
    assert!(e.dialect.is_none());
    assert!(e.key.is_none());
    assert!(e.subtable.is_none());
}

#[test]
fn command_side_effects_pure_constant() {
    let cse = CommandSideEffects::pure();
    assert!(cse.pure);
    assert!(cse.deterministic);
    assert_eq!(cse.effects, [] as [crate::side_effects::SideEffect; 0]);
    assert!(!cse.dynamic_barrier);
    assert!(!cse.reads_any());
    assert!(!cse.writes_any());
}

#[test]
fn command_side_effects_unknown_write_constant() {
    let cse = CommandSideEffects::unknown_write();
    assert!(!cse.pure);
    assert_eq!(cse.effects.len(), 1);
    assert_eq!(cse.effects[0].target, SideEffectTarget::Unknown);
    assert!(cse.effects[0].writes);
    assert!(cse.writes_any());
}

#[test]
fn command_side_effects_dynamic_barrier_flag() {
    let cse = CommandSideEffects::dynamic_barrier();
    assert!(cse.dynamic_barrier);
    let (_, w) = cse.to_effect_regions();
    assert!(w.contains(EffectRegion::UNKNOWN_STATE));
}

#[test]
fn affects_reads_writes_target_helpers() {
    let cse = CommandSideEffects {
        effects: vec![
            SideEffect::new(SideEffectTarget::HttpHeader, true, false),
            SideEffect::new(SideEffectTarget::Variable, false, true),
        ],
        ..CommandSideEffects::default()
    };
    assert!(cse.affects_target(SideEffectTarget::HttpHeader));
    assert!(cse.affects_target(SideEffectTarget::Variable));
    assert!(!cse.affects_target(SideEffectTarget::FileIo));
    assert!(cse.reads_target(SideEffectTarget::HttpHeader));
    assert!(!cse.reads_target(SideEffectTarget::Variable));
    assert!(cse.writes_target(SideEffectTarget::Variable));
}

#[test]
fn to_effect_regions_unions_reads_and_writes() {
    let mut global_var = SideEffect::new(SideEffectTarget::Variable, false, true);
    global_var.scope = StorageScope::Global;
    let cse = CommandSideEffects {
        effects: vec![
            SideEffect::new(SideEffectTarget::HttpHeader, true, false),
            global_var,
        ],
        ..CommandSideEffects::default()
    };
    let (r, w) = cse.to_effect_regions();
    assert!(r.contains(EffectRegion::HTTP_STATE));
    assert!(w.contains(EffectRegion::GLOBAL_STATE));
    assert!(!w.contains(EffectRegion::HTTP_STATE));
}

// Scope + storage-type inference.

#[test]
fn scope_proc_local_for_bare_names() {
    let (s, ns) = scope_from_varname("x");
    assert_eq!(s, StorageScope::ProcLocal);
    assert!(ns.is_none());
}

#[test]
fn static_spelling_is_an_ordinary_tcl_namespace() {
    let (s, ns) = scope_from_varname("static::counter");
    assert_eq!(s, StorageScope::Namespace);
    assert_eq!(ns.as_deref(), Some("::static"));
}

#[test]
fn scope_global_for_root_qualified() {
    let (s, ns) = scope_from_varname("::var");
    assert_eq!(s, StorageScope::Global);
    assert_eq!(ns.as_deref(), Some("::"));
}

#[test]
fn scope_namespace_for_qualified_with_segments() {
    let (s, ns) = scope_from_varname("::foo::bar::var");
    assert_eq!(s, StorageScope::Namespace);
    assert_eq!(ns.as_deref(), Some("::foo::bar"));
}

#[test]
fn scope_namespace_single_segment() {
    let (s, ns) = scope_from_varname("::foo::var");
    assert_eq!(s, StorageScope::Namespace);
    assert_eq!(ns.as_deref(), Some("::foo"));
}

#[test]
fn storage_type_default_scalar_for_unknown_command() {
    let registry = CommandRegistry::build_default();
    let args = vec![];
    assert_eq!(
        storage_type_for_command(&registry, "nonexistentcmd", &args),
        StorageType::Scalar
    );
}

#[test]
fn storage_type_registry_backed_list_dict_array() {
    let registry = CommandRegistry::build_default();
    let args = vec![];
    // lappend / dict / array are declared with inferred storage
    // types in the registry and should be mapped through.
    assert_eq!(
        storage_type_for_command(&registry, "lappend", &args),
        StorageType::List
    );
    assert_eq!(
        storage_type_for_command(&registry, "dict", &args),
        StorageType::Dict
    );
    assert_eq!(
        storage_type_for_command(&registry, "array", &args),
        StorageType::Array
    );
}

// classify_side_effects.

#[test]
fn classify_pure_expr() {
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(&registry, "expr", &["{1 + 2}".into()], None, None);
    assert!(cse.pure);
    assert!(cse.deterministic);
    assert_eq!(cse.effects, [] as [crate::side_effects::SideEffect; 0]);
}

#[test]
fn classify_set_write() {
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(&registry, "set", &["x".into(), "42".into()], None, None);
    assert!(!cse.pure);
    assert!(cse.writes_any());
    assert!(!cse.reads_any());
    let eff = &cse.effects[0];
    assert_eq!(eff.target, SideEffectTarget::Variable);
    assert_eq!(eff.scope, StorageScope::ProcLocal);
    assert_eq!(eff.key.as_deref(), Some("x"));
}

#[test]
fn classify_set_read_when_no_value() {
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(&registry, "set", &["x".into()], None, None);
    assert!(cse.reads_any());
    assert!(!cse.writes_any());
}

#[test]
fn classify_incr_is_read_modify_write() {
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(&registry, "incr", &["i".into()], None, None);
    // incr always reads AND writes (READS_BEFORE_WRITE trait).
    assert!(cse.reads_any());
    assert!(cse.writes_any());
}

#[test]
fn classify_unset_keys_the_variable_not_the_option() {
    // `unset x` destroys `x` — it is a WRITE (destroy) of
    // `x`, never a read, and its key is `x` (not modelled as an assignment).
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(&registry, "unset", &["x".into()], None, None);
    assert!(cse.writes_any());
    assert!(!cse.reads_any(), "unset does not read its target");
    assert_eq!(cse.effects.len(), 1);
    assert_eq!(cse.effects[0].target, SideEffectTarget::Variable);
    assert_eq!(cse.effects[0].key.as_deref(), Some("x"));
}

#[test]
fn classify_unset_skips_leading_options() {
    // `unset -nocomplain x` keys `x`, NOT `-nocomplain`.
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(
        &registry,
        "unset",
        &["-nocomplain".into(), "x".into()],
        None,
        None,
    );
    let keys: Vec<&str> = cse
        .effects
        .iter()
        .filter_map(|e| e.key.as_deref())
        .collect();
    assert_eq!(keys, vec!["x"], "must key the variable, not the option");
}

#[test]
fn classify_unset_keys_every_name() {
    // `unset a b c` destroys all three — not just the first.
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(
        &registry,
        "unset",
        &["a".into(), "b".into(), "c".into()],
        None,
        None,
    );
    let mut keys: Vec<&str> = cse
        .effects
        .iter()
        .filter_map(|e| e.key.as_deref())
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["a", "b", "c"]);
    assert!(cse.writes_any());
}

#[test]
fn classify_global_scope_variable() {
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(
        &registry,
        "set",
        &["::foo::bar".into(), "v".into()],
        None,
        None,
    );
    let eff = &cse.effects[0];
    assert_eq!(eff.scope, StorageScope::Namespace);
    assert_eq!(eff.namespace.as_deref(), Some("::foo"));
}

#[test]
fn classify_irules_static_uses_global_connection_side() {
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(
        &registry,
        "set",
        &["static::counter".into(), "0".into()],
        Some(tcl_dialect::DialectProfile::irules()),
        None,
    );
    let eff = &cse.effects[0];
    assert_eq!(eff.scope, StorageScope::Static);
    assert_eq!(eff.connection_side, ConnectionSide::Global);
}

#[test]
fn classify_irules_proc_local_uses_both_side() {
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(
        &registry,
        "set",
        &["local".into(), "0".into()],
        Some(tcl_dialect::DialectProfile::irules()),
        None,
    );
    let eff = &cse.effects[0];
    assert_eq!(eff.connection_side, ConnectionSide::Both);
}

#[test]
fn classify_eval_is_dynamic_barrier() {
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(&registry, "eval", &["script".into()], None, None);
    assert!(cse.dynamic_barrier);
    let (_, w) = cse.to_effect_regions();
    assert!(w.contains(EffectRegion::UNKNOWN_STATE));
}

#[test]
fn classify_proc_is_procedure_definition() {
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(
        &registry,
        "proc",
        &["foo".into(), "{}".into(), "{}".into()],
        None,
        None,
    );
    let eff = &cse.effects[0];
    assert_eq!(eff.target, SideEffectTarget::ProcDefinition);
    assert!(eff.writes);
    assert_eq!(eff.scope, StorageScope::Namespace);
}

#[test]
fn classify_unknown_command_is_conservative() {
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(&registry, "__nonexistent", &[], None, None);
    assert!(cse.reads_any());
    assert!(cse.writes_any());
    assert_eq!(cse.effects[0].target, SideEffectTarget::Unknown);
}

#[test]
fn classify_puts_consults_structured_side_effects() {
    // `puts` carries no purity/assign/proc-def trait, so a trait-only
    // classifier falls through to the conservative UNKNOWN write. It resolves
    // via the spec's structured `side_effects` (a `FileIo` write) —
    // impure (`writes_any`) but region-free: `classify_side_effects`
    // for `puts` yields pure=False, regions=(NONE, NONE).
    let registry = CommandRegistry::build_default();
    let cse = classify_side_effects(&registry, "puts", &["hi".into()], None, None);
    assert!(!cse.pure);
    assert!(cse.writes_any());
    assert!(!cse.reads_any());
    assert_eq!(cse.effects[0].target, SideEffectTarget::FileIo);
    // The whole point: no tracked effect region (NONE, not UNKNOWN_STATE).
    let (r, w) = cse.to_effect_regions();
    assert_eq!(r, EffectRegion::NONE);
    assert_eq!(w, EffectRegion::NONE);
}

#[test]
fn external_tk_selection_reads_are_neither_pure_nor_deterministic() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let profile = tcl_registry::model::ingress::resolve_environment("tk").analyser_profile();
    for command in ["clipboard", "selection"] {
        let effect =
            classify_side_effects(registry, command, &["get".to_owned()], Some(profile), None);
        assert!(!effect.pure, "{command} get reads mutable external state");
        assert!(
            !effect.deterministic,
            "{command} get can change without any Tcl-side write"
        );
        assert!(
            effect.reads_any(),
            "{command} get must retain its read effect"
        );
        assert!(!effect.writes_any(), "{command} get is not itself a writer");
    }
}

#[test]
fn pure_command_mutator_subcommand_downgrades_purity() {
    // `HTTP::header` carries `Traits::PURE` + a structured `HttpHeader`
    // read+write hint + `mutator` subcommands. The getter stays pure with
    // a read-only effect; a mutator subcommand (`insert`) downgrades to
    // impure with a read+write effect. On the pure branch,
    // `HTTP::header` → (HTTP_STATE, NONE) pure; `insert` → (HTTP_STATE,
    // HTTP_STATE) impure.
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));

    let getter = classify_side_effects(
        &registry,
        "HTTP::header",
        &["value".into(), "Host".into()],
        Some(tcl_dialect::DialectProfile::irules()),
        None,
    );
    assert!(getter.pure, "HTTP::header getter should stay pure");
    assert_eq!(
        getter.to_effect_regions(),
        (EffectRegion::HTTP_STATE, EffectRegion::NONE),
        "pure getter reads header state, no write",
    );

    let insert = classify_side_effects(
        &registry,
        "HTTP::header",
        &["insert".into(), "X-Foo".into(), "y".into()],
        Some(tcl_dialect::DialectProfile::irules()),
        None,
    );
    assert!(
        !insert.pure,
        "HTTP::header insert (mutator subcommand) must not be pure",
    );
    assert_eq!(
        insert.to_effect_regions(),
        (EffectRegion::HTTP_STATE, EffectRegion::HTTP_STATE),
        "header mutation reads + writes header state",
    );
}

#[test]
fn classify_hints_are_dialect_gated() {
    // `log` is irules-only. Under a Tcl dialect its irules `LogIo`
    // hint must NOT apply — the spec doesn't support `tcl8.6`, so the
    // classifier falls back to the conservative UNKNOWN write
    // (dialect-mismatched specs are skipped). With no dialect requested
    // the hint applies.
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    // Dialect-agnostic (interproc path): LogIo, region-free, impure.
    let agnostic = classify_side_effects(&registry, "log", &["hi".into()], None, None);
    assert!(!agnostic.pure);
    assert_eq!(agnostic.effects[0].target, SideEffectTarget::LogIo);
    assert_eq!(
        agnostic.to_effect_regions(),
        (EffectRegion::NONE, EffectRegion::NONE)
    );
    // Under a Tcl dialect the irules spec is gated out → UNKNOWN write.
    let tcl = classify_side_effects(
        &registry,
        "log",
        &["hi".into()],
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        None,
    );
    assert!(!tcl.pure);
    assert_eq!(tcl.effects[0].target, SideEffectTarget::Unknown);
    assert_eq!(
        tcl.to_effect_regions(),
        (EffectRegion::UNKNOWN_STATE, EffectRegion::UNKNOWN_STATE)
    );
}

#[test]
fn callee_summary_http_read_emits_http_header_effect() {
    let registry = CommandRegistry::build_default();
    let summary = CalleeSummary {
        effect_reads: EffectRegion::HTTP_STATE,
        effect_writes: EffectRegion::NONE,
        pure: false,
    };
    let cse = classify_side_effects(&registry, "doesnt_matter", &[], None, Some(&summary));
    assert!(cse.affects_target(SideEffectTarget::HttpHeader));
    assert!(cse.reads_target(SideEffectTarget::HttpHeader));
    assert!(!cse.writes_target(SideEffectTarget::HttpHeader));
}

#[test]
fn callee_summary_response_commit_sets_write() {
    let registry = CommandRegistry::build_default();
    let summary = CalleeSummary {
        effect_reads: EffectRegion::NONE,
        effect_writes: EffectRegion::RESPONSE_LIFECYCLE | EffectRegion::HTTP_STATE,
        pure: false,
    };
    let cse = classify_side_effects(&registry, "doesnt_matter", &[], None, Some(&summary));
    assert!(cse.writes_target(SideEffectTarget::ResponseCommit));
}

#[test]
fn callee_summary_pure_bypass_registry() {
    let registry = CommandRegistry::build_default();
    let summary = CalleeSummary {
        effect_reads: EffectRegion::NONE,
        effect_writes: EffectRegion::NONE,
        pure: true,
    };
    let cse = classify_side_effects(&registry, "doesnt_matter", &[], None, Some(&summary));
    assert!(cse.pure);
    assert!(cse.deterministic);
    assert_eq!(cse.effects, [] as [crate::side_effects::SideEffect; 0]);
}

#[test]
fn effects_filtered_by_scope_and_side() {
    let mut client_hdr = SideEffect::new(SideEffectTarget::HttpHeader, true, false);
    client_hdr.connection_side = ConnectionSide::Client;
    let mut server_hdr = SideEffect::new(SideEffectTarget::HttpHeader, true, false);
    server_hdr.connection_side = ConnectionSide::Server;
    let cse = CommandSideEffects {
        effects: vec![client_hdr, server_hdr],
        ..CommandSideEffects::default()
    };
    assert_eq!(cse.effects_on_side(ConnectionSide::Client).len(), 1);
    assert_eq!(cse.effects_on_side(ConnectionSide::Server).len(), 1);
    assert_eq!(cse.effects_in_scope(StorageScope::Global).len(), 0);
}
#[test]
fn static_namespace_effects_share_regions_across_spellings_and_aliases() {
    let registry = CommandRegistry::build_default();
    let dialect = Some(tcl_dialect::DialectProfile::irules());
    let mut context = crate::var_resolve::ResolveContext::default();
    context.known_namespaces.insert("::static".to_owned());
    let target = crate::var_resolve::resolve_place("::static::counter", &context, false, &registry);
    context.alias_bindings.insert("linked".to_owned(), target);
    for name in ["static::counter", "::static::counter", "linked"] {
        let effects = classify_side_effects_with_context(
            &registry,
            "set",
            &[name.to_owned(), "1".to_owned()],
            dialect,
            None,
            &context,
        );
        assert_eq!(effects.effects[0].scope, StorageScope::Static);
        assert_eq!(effects.effects[0].key.as_deref(), Some("::static::counter"));
        assert!(
            effects
                .to_effect_regions()
                .1
                .contains(EffectRegion::GLOBAL_STATE)
        );
    }
    let plain = classify_side_effects(
        &registry,
        "set",
        &["::static::counter".to_owned(), "1".to_owned()],
        Some(tcl_dialect::DialectProfile::find("tcl8.6").expect("profile")),
        None,
    );
    assert_eq!(plain.effects[0].scope, StorageScope::Namespace);
}

fn native_effect_namespace(
    token: u64,
    segments: &[&str],
) -> crate::command_binding::SourceNamespaceKey {
    use tcl_runtime_api::native_compilation::{NativeInterpreterIdentity, NativeNamespaceContext};
    crate::command_binding::SourceNamespaceKey::Native(NativeNamespaceContext {
        interpreter: NativeInterpreterIdentity {
            owner: 31,
            interpreter: 7,
        },
        token,
        path: tcl_core_types::ByteNamespacePath::from_segments(segments.iter().copied()),
    })
}

#[test]
fn namespace_place_storage_scope_requires_the_selected_domain_before_static_reporting() {
    // naming.analysis.resolved-static-variable-patterns
    // docs/design/analysis/name-resolution-proofs/resolved-static-variable-patterns.md
    // Actual resolver/domain software inputs; no physical BIG-IP worker is observed.
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let root = native_effect_namespace(1, &[]);
    let static_namespace = native_effect_namespace(2, &["static"]);
    let mut context = crate::var_resolve::ResolveContext::for_namespace("::static");
    context.retain_namespace_world(
        static_namespace.clone(),
        [root, static_namespace],
        Some(tcl_syntax::naming::NativeNameProtocol::C(
            tcl_dialect::TclVersion::V8_4,
        )),
    );
    context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
        registry.profile().unwrap(),
    ));
    let ordinary = crate::var_resolve::resolve_literal_place("debug", &context, false, registry);
    assert_eq!(
        namespace_storage_scope_for_place(&ordinary, registry.profile()),
        StorageScope::Namespace
    );
    context.hosted_execution_context = Some(tcl_registry::f5::BigIpExecutionContext::TmmIRule);
    let selected = crate::var_resolve::resolve_literal_place("debug", &context, false, registry);
    assert_eq!(
        namespace_storage_scope_for_place(&selected, registry.profile()),
        StorageScope::Static
    );
    let mut displayed = ordinary;
    displayed.ns = "::static".into();
    displayed.name = "static::debug".into();
    assert_eq!(
        namespace_storage_scope_for_place(&displayed, registry.profile()),
        StorageScope::Namespace
    );
    let unknown = crate::place::unknown_top();
    assert_eq!(
        namespace_storage_scope_for_place(&unknown, registry.profile()),
        StorageScope::Unknown
    );
}

/// The static storage policy applies to its own namespace incarnation only.
fn assert_static_policy_scope(
    context: &mut crate::var_resolve::ResolveContext,
    namespace: &crate::command_binding::SourceNamespaceKey,
    registry: &CommandRegistry,
) {
    context.authored_tmm_static = Some(
        tcl_runtime_api::authored_tmm::AuthoredTmmStaticCompilationContext {
            policy: tcl_runtime_api::authored_tmm::AuthoredTmmStaticPolicy::RuleInitPublication,
            context: None,
            namespace: namespace
                .native_context()
                .expect("actual namespace")
                .clone(),
            recipients: Vec::new(),
            outward_observers:
                tcl_runtime_api::native_compilation::NativeVariableObserverPresence::Unknown,
        },
    );
    assert_eq!(
        resolved_variable_scope("counter", registry, None, context).0,
        StorageScope::Static,
    );
    context
        .authored_tmm_static
        .as_mut()
        .expect("selected policy")
        .namespace
        .token += 1;
    assert_eq!(
        resolved_variable_scope("counter", registry, None, context).0,
        StorageScope::Namespace,
    );
}

#[test]
fn native_variable_scope_uses_geometry_and_actual_storage_domain() {
    let registry = CommandRegistry::build_default();
    let root = native_effect_namespace(1, &[]);
    let literal_colons = native_effect_namespace(2, &["::"]);
    let static_namespace = native_effect_namespace(3, &["static"]);
    for (namespace, expected) in [
        (root.clone(), StorageScope::Global),
        (literal_colons.clone(), StorageScope::Namespace),
        (static_namespace.clone(), StorageScope::Namespace),
    ] {
        let mut context = crate::var_resolve::ResolveContext::for_namespace(
            namespace.display().expect("Unicode presentation"),
        );
        context.retain_namespace_world(
            namespace.clone(),
            [
                root.clone(),
                literal_colons.clone(),
                static_namespace.clone(),
            ],
            Some(tcl_syntax::naming::NativeNameProtocol::C(
                tcl_dialect::TclVersion::V8_6,
            )),
        );
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            tcl_dialect::DialectProfile::find("tcl8.6").expect("actual lookup protocol"),
        ));
        if namespace != root {
            assert_eq!(
                resolved_variable_scope(
                    "counter",
                    &registry,
                    Some(tcl_dialect::DialectProfile::irules()),
                    &context,
                )
                .0,
                StorageScope::Unknown,
                "unknown current/global cell presence must remain ambiguous",
            );
        }
        context
            .namespace_cells
            .present
            .insert(VariableCellKey::Namespace {
                identity: namespace.clone(),
                simple: "counter".into(),
            });
        let (scope, displayed_namespace, _) = resolved_variable_scope(
            "counter",
            &registry,
            Some(tcl_dialect::DialectProfile::irules()),
            &context,
        );
        assert_eq!(scope, expected);
        assert_eq!(displayed_namespace, namespace.display());
        context
            .namespace_cells
            .present
            .insert(VariableCellKey::Namespace {
                identity: namespace.clone(),
                simple: "auto_path".into(),
            });
        let special = classify_side_effects_with_context(
            &registry,
            "set",
            &["auto_path".to_owned(), "value".to_owned()],
            Some(tcl_dialect::DialectProfile::find("tcl8.6").expect("profile")),
            None,
            &context,
        );
        assert_eq!(
            special.writes_target(SideEffectTarget::InterpState),
            expected == StorageScope::Global,
        );
        if namespace == static_namespace {
            assert_static_policy_scope(&mut context, &namespace, &registry);
        }
    }
}

#[test]
fn native_namespace_scope_distinguishes_current_global_and_original_alias_cells() {
    use crate::var_resolve::VariableCellKey;

    for (dialect, version, fallback) in [
        ("tcl8.4", tcl_dialect::TclVersion::V8_4, true),
        ("tcl8.5", tcl_dialect::TclVersion::V8_5, true),
        ("tcl8.6", tcl_dialect::TclVersion::V8_6, true),
        ("tcl9.0", tcl_dialect::TclVersion::V9_0, false),
        ("tcl9.1", tcl_dialect::TclVersion::V9_1, false),
    ] {
        let selected = tcl_registry::model::ingress::static_context_for(dialect);
        let registry = selected.commands();
        let root = native_effect_namespace(1, &[]);
        let named = native_effect_namespace(2, &["named"]);
        let mut context = crate::var_resolve::ResolveContext::for_namespace("::named");
        context.retain_namespace_world(
            named.clone(),
            [root.clone(), named.clone()],
            Some(tcl_syntax::naming::NativeNameProtocol::C(version)),
        );
        let profile = Some(tcl_dialect::DialectProfile::find(dialect).expect("native profile"));
        assert_eq!(
            resolved_variable_scope("counter", registry, profile, &context).0,
            if fallback {
                StorageScope::Unknown
            } else {
                StorageScope::Namespace
            },
            "{dialect}: open original cell inventory",
        );
        let root_cell = VariableCellKey::Namespace {
            identity: root.clone(),
            simple: "counter".into(),
        };
        let current_cell = VariableCellKey::Namespace {
            identity: named,
            simple: "counter".into(),
        };
        context.namespace_cells.closed = true;
        context.namespace_cells.present.insert(root_cell);
        assert_eq!(
            resolved_variable_scope("counter", registry, profile, &context).0,
            if fallback {
                StorageScope::Global
            } else {
                StorageScope::Namespace
            },
            "{dialect}: missing current cell and present global cell",
        );
        context.namespace_cells.present.insert(current_cell);
        assert_eq!(
            resolved_variable_scope("counter", registry, profile, &context).0,
            StorageScope::Namespace,
            "{dialect}: present current cell precedes global fallback",
        );
        let original = context.namespace_place_in_identity("counter", &root, false);
        context.alias_bindings.insert("linked", original);
        assert_eq!(
            resolved_variable_scope("linked", registry, profile, &context).0,
            StorageScope::Global,
            "{dialect}: original alias retains its root receiver",
        );
        context.namespace_cells.present.clear();
        assert_eq!(
            resolved_variable_scope("counter", registry, profile, &context).0,
            StorageScope::Namespace,
            "{dialect}: both original cells absent selects current creation table",
        );
    }
}

#[test]
fn jim_namespace_scope_keeps_activation_locals_and_original_variable_links_separate() {
    let selected = tcl_registry::model::ingress::static_context_for("jim");
    let registry = selected.commands();
    let profile = registry.profile();
    let root = native_effect_namespace(1, &[]);
    let named = native_effect_namespace(2, &["named"]);
    let mut context = crate::var_resolve::ResolveContext::for_namespace("::named");
    context.retain_namespace_world(
        named.clone(),
        [root.clone(), named.clone()],
        Some(tcl_syntax::naming::NativeNameProtocol::Jim084),
    );
    context.namespace_objects.insert(root.clone(), "".into());
    context
        .namespace_objects
        .insert(named.clone(), "named".into());
    assert_eq!(
        resolved_variable_scope("counter", registry, profile, &context).0,
        StorageScope::ProcLocal,
        "an unlinked namespace-eval local does not select global fallback",
    );
    let original = context.namespace_place_in_identity("counter", &named, false);
    assert_eq!(
        original
            .cell
            .as_ref()
            .expect("original flat namespace cell")
            .name,
        "named::counter",
    );
    assert_eq!(
        original.cell.as_ref().unwrap().owner,
        crate::place::CellOwner::NamespaceIdentity(Box::new(root.clone())),
        "Jim's stored key is root-flat; the namespace owner is independent",
    );
    context.alias_bindings.insert("counter", original);
    assert_eq!(
        resolved_variable_scope("counter", registry, profile, &context).0,
        StorageScope::Global,
        "variable links retain Jim's actual flat global storage owner",
    );
    let root_original = context.namespace_place_in_identity("counter", &root, false);
    context.alias_bindings.insert("linked", root_original);
    assert_eq!(
        resolved_variable_scope("linked", registry, profile, &context).0,
        StorageScope::Global,
    );
    assert_eq!(
        resolved_variable_scope("::counter", registry, profile, &context).0,
        StorageScope::Global,
        "an original root qualifier selects the real flat global cell",
    );
}

#[test]
fn special_variable_effects_follow_root_cells_and_aliases() {
    let registry = CommandRegistry::build_default();
    let profile = Some(tcl_dialect::DialectProfile::find("tcl8.6").expect("profile"));
    let mut context = crate::var_resolve::ResolveContext::default();
    context.known_namespaces.insert("::named".to_owned());
    for (name, alias) in [("::auto_path", "search_path"), ("::env", "environment")] {
        let target = crate::var_resolve::resolve_place(name, &context, false, &registry);
        context.alias_bindings.insert(alias, target);
    }
    for (name, expected) in [
        ("::auto_path", true),
        ("search_path", true),
        ("::env(KEY)", true),
        ("environment(KEY)", true),
        ("auto_path", false),
        ("env(KEY)", false),
        ("::named::auto_path", false),
        ("::named::env(KEY)", false),
    ] {
        let effects = classify_side_effects_with_context(
            &registry,
            "set",
            &[name.to_owned(), "value".to_owned()],
            profile,
            None,
            &context,
        );
        assert_eq!(
            effects.writes_target(SideEffectTarget::InterpState),
            expected,
            "{name}"
        );
    }
    let read = classify_side_effects_with_context(
        &registry,
        "set",
        &["search_path".to_owned()],
        profile,
        None,
        &context,
    );
    assert!(!read.writes_target(SideEffectTarget::InterpState));
}

/// A command the document declares classifies from the facts its
/// declaration states — nearest wins — and a declaration stating none
/// classifies exactly as an undeclared command does; every other name is
/// the catalogue's.
///
/// The declarations arrive the way a document states them, through the
/// analyser's one ingestion path (`document_declared_surface`), so the
/// facts classified here are the flags' own: `-pure` is `PURE`, and
/// `-mutator` is `READS_BEFORE_WRITE` beside a variable read and write.
#[test]
fn a_declared_command_classifies_from_its_declaration() {
    use tcl_registry::model::DocumentCommandSurface;
    let registry = CommandRegistry::build_default();
    let declared = crate::analyser::utils::document_declared_surface(
        "# tcl-lsp: stubs-begin\n\
             # tcl-lsp: stub my_pure {} -pure\n\
             # tcl-lsp: stub my_mut {v:var} -mutator\n\
             # tcl-lsp: stub my_plain {}\n\
             # tcl-lsp: stubs-end\n",
        None,
        "tcl8.6",
    );
    let surface = DocumentCommandSurface::new(&registry, Some(&declared));
    let classify =
        |name: &str, args: &[String]| classify_side_effects_in(&surface, name, args, None, None);

    let pure = classify("my_pure", &[]);
    assert!(pure.pure && pure.deterministic && pure.effects.is_empty());

    let mutator = classify("my_mut", &[]);
    assert!(!mutator.pure);
    assert!(mutator.reads_target(SideEffectTarget::Variable));
    assert!(mutator.writes_target(SideEffectTarget::Variable));
    assert!(
        !mutator.affects_target(SideEffectTarget::Unknown),
        "the stated effect, not the conservative unknown write"
    );

    assert_eq!(
        classify("my_plain", &[]),
        classify_side_effects(&registry, "no_such_command", &[], None, None),
        "a declaration stating no behaviour is as conservative as no declaration"
    );

    let set_args = ["x".to_owned(), "1".to_owned()];
    assert_eq!(
        classify("set", &set_args),
        classify_side_effects(&registry, "set", &set_args, None, None),
        "an undeclared name is the catalogue's"
    );
}

/// A declaration flagged `-extension` starts from the conservative default
/// for native code: a dynamic barrier with an unknown read and write. A
/// stated purity or effect replaces those, because the declaration said
/// what the command does; a stated barrier survives.
#[test]
fn an_extension_declaration_is_a_barrier_until_it_states_its_effects() {
    use tcl_registry::model::DocumentCommandSurface;
    let registry = CommandRegistry::build_default();
    let declared = crate::analyser::utils::document_declared_surface(
        "# tcl-lsp: stubs-begin\n\
             # tcl-lsp: stub ext_any {} -extension\n\
             # tcl-lsp: stub ext_pure {} -extension -pure\n\
             # tcl-lsp: stub ext_mut {} -extension -mutator\n\
             # tcl-lsp: stub ext_both {} -extension -mutator -barrier\n\
             # tcl-lsp: stub plain {}\n\
             # tcl-lsp: stubs-end\n",
        None,
        "tcl8.6",
    );
    let surface = DocumentCommandSurface::new(&registry, Some(&declared));
    let classify = |name: &str| classify_side_effects_in(&surface, name, &[], None, None);

    let any = classify("ext_any");
    assert!(any.dynamic_barrier && !any.pure);
    assert!(any.reads_target(SideEffectTarget::Unknown));
    assert!(any.writes_target(SideEffectTarget::Unknown));

    let pure = classify("ext_pure");
    assert!(pure.pure && pure.deterministic && pure.effects.is_empty());
    assert!(!pure.dynamic_barrier);

    let mutator = classify("ext_mut");
    assert!(!mutator.dynamic_barrier && !mutator.pure);
    assert!(mutator.writes_target(SideEffectTarget::Variable));
    assert!(!mutator.affects_target(SideEffectTarget::Unknown));

    // `-barrier` is the trait the minifier fences scopes by, not the
    // classifier's `dynamic_barrier`, so it is read off the surface.
    let barrier_trait = |name: &str| {
        surface
            .traits(name)
            .is_some_and(|traits| traits.contains(Traits::CREATES_DYNAMIC_BARRIER))
    };
    assert!(
        barrier_trait("ext_both"),
        "a stated barrier survives a stated effect"
    );
    assert!(
        !barrier_trait("ext_mut"),
        "a stated effect replaces the default's barrier"
    );
    assert!(barrier_trait("ext_any"), "the default is a barrier");

    assert!(
        !classify("plain").dynamic_barrier,
        "without `-extension` a stub that states nothing is no barrier"
    );
}
