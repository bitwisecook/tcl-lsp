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

use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};

use super::*;
use crate::cfg_builder::build_cfg;

/// The Tcl 8.6 catalogue: a fixture that defines a procedure names a release.
fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").expect("Tcl 8.6"))
}

/// The catalogue with no document declarations — what every fixture here
/// analyses against unless it states otherwise.
/// A `# tcl-lsp: stub myexpr {value:expr}` declares an expression word
/// exactly as a shipped command does, and lowering honours that role — so
/// the `[…]` inside `myexpr {[id 9]}` is a call site this scan must see.
///
/// The walk read the bare catalogue, which knows no declared command at
/// all, so the call was invisible and a lone visible `id 7` could seed
/// `id`'s parameter as the constant `7` (#2118, found in review).
#[test]
fn a_declared_expression_role_reaches_the_call_site_walk() {
    use tcl_dialect::model::Provenance;
    use tcl_registry::model::{DeclaredArgument, DeclaredCommand, DeclaredSurface};

    let reg = CommandRegistry::build_default();
    let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {x} {\n myexpr {[id 9]}\n}",
        &reg,
        false,
    );
    let fu = cu.function("::f").expect("proc lowered");
    let tokens = fu.cfg.blocks.values().find_map(|block| {
        block.statements.iter().find_map(|stmt| match stmt {
            Statement::Call {
                command, tokens, ..
            } if command == "myexpr" => tokens.as_ref(),
            _ => None,
        })
    });

    assert!(
        crate::word_subst::lifted_calls_with_surface(tokens, config, &surface(&reg)).is_empty(),
        "undeclared, `myexpr`'s braced word is ordinary literal text"
    );

    let mut declared = DeclaredSurface::new();
    declared.declare(DeclaredCommand::new(
        "myexpr".to_owned(),
        vec![DeclaredArgument {
            name: "value".to_owned(),
            role: tcl_registry::ArgRole::Expr,
            optional: false,
        }],
        Provenance::Document,
    ));
    let document = tcl_registry::model::DocumentCommandSurface::new(&reg, Some(&declared));
    assert_eq!(
        crate::word_subst::lifted_calls_with_surface(tokens, config, &document)
            .iter()
            .map(|lifted| (lifted.command.clone(), lifted.args.clone()))
            .collect::<Vec<_>>(),
        vec![("id".to_owned(), vec!["9".to_owned()])],
        "the declaration says the word is an expression, so its `[…]` runs"
    );
}

fn retained_scope_fixture(source: &str, registry: &CommandRegistry) -> IrModule {
    let profile = registry
        .profile()
        .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl);
    let context = std::sync::Arc::new(
        crate::environment_ingress::context_for_profile(profile)
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(registry, config)
            .with_dialect(Some(profile))
            .with_resolved_analysis_input(crate::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            )),
        source,
    )
}

fn surface(reg: &CommandRegistry) -> tcl_registry::model::DocumentCommandSurface<'_> {
    tcl_registry::model::DocumentCommandSurface::new(reg, None)
}

fn known(names: &[&str]) -> HashSet<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

fn params(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

fn seed(
    params: &[String],
    evidence: &CallSiteEvidence,
    qname: &str,
    linkage: Traits,
    has_cross_file_evidence: bool,
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    let proc_binding_trust = crate::command_binding::ProcBindingTrustProjection::default();
    params_constants_from_call_sites(
        params,
        evidence,
        qname,
        &UnitCallerView {
            linkage,
            has_cross_file_evidence,
            proc_binding_trust: &proc_binding_trust,
            source: None,
            entered: None,
        },
    )
}

fn native_seed(
    params_raw: &str,
    grammar: Option<tcl_dialect::ParameterGrammar>,
    evidence: &CallSiteEvidence,
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    let procedure = crate::ir::Procedure {
        name: "helper".into(),
        qualified_name: "::helper".into(),
        publication: None,
        declaration_site: None,
        body_kind: crate::ir::ProcedureBodyKind::Named,
        params: Vec::new(),
        span: tcl_lexer::Span::new(0, 0),
        body: crate::ir::Script::default(),
        params_raw: params_raw.into(),
        body_source: None,
        body_offset: 0,
        namespace_scoped: false,
        base_priority: 500,
    };
    let mut module = IrModule::default();
    let id = module.retain_procedure(procedure);
    let mut evidence = evidence.clone();
    if let Some(row) = evidence.get("::helper").cloned() {
        evidence.by_body.insert(id, row);
    }
    let trust = crate::command_binding::ProcBindingTrustProjection::default();
    params_constants_from_native_call_sites(
        &module,
        id,
        module.procedures.get(&id).expect("issued fixture body"),
        grammar,
        &evidence,
        &UnitCallerView {
            linkage: Traits::empty(),
            has_cross_file_evidence: false,
            proc_binding_trust: &trust,
            source: None,
            entered: None,
        },
    )
}

fn source_loader_entry(text: &str) -> crate::command_binding::SourceAnalysisEntry {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
    crate::command_binding::SourceAnalysisEntry {
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        trusted_source_modules: vec![crate::command_binding::TrustedSourceModuleLoader::new(
            tcl_dialect::model::Family::Tcl,
            "library.tcl".into(),
            None,
            "workspace/library/revision-one".into(),
            &std::sync::Arc::from(text),
        )],
        ..crate::command_binding::SourceAnalysisEntry::default()
    }
}

fn scan_loaded_caller(
    source: &str,
    entry: &crate::command_binding::SourceAnalysisEntry,
) -> CallSiteEvidence {
    let registry = CommandRegistry::build_default();
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
    let known = HashSet::from(["::helper".to_owned()]);
    let context = std::sync::Arc::new(
        crate::environment_ingress::context_for_profile(profile)
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    scan_source_call_sites_with_source_input(source, None, &known, &[], &input, entry)
}

#[test]
fn independent_caller_ingresses_keep_their_actual_sources_and_loader_identity() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // Actual independent ingress remains available; changing source text
    // starts its own producer rather than querying a mismatched Module.
    let library = "proc helper {mode} {return $mode}";
    let entry = source_loader_entry(library);
    for value in ["FIRST", "SECOND"] {
        let source = format!("source library.tcl; helper {value}");
        let evidence = scan_loaded_caller(&source, &entry);
        let helper = evidence.get("::helper").expect("actual sourced caller");
        assert_eq!(helper.uniform_literal_at(0), Some(value));
        assert_eq!(helper.source_implementations.len(), 1);
        assert!(helper.source_implementations[0].matches_source_declaration(library, 0));
    }
}

#[test]
fn trusted_file_entry_retains_actual_loaded_declaration_identity() {
    let library = "proc helper {mode} {return $mode}";
    let entry = source_loader_entry(library);
    let evidence = scan_loaded_caller("source library.tcl; helper dev", &entry);
    let helper = evidence.get("::helper").expect("actual sourced caller");
    assert_eq!(helper.uniform_literal_at(0), Some("dev"));
    assert_eq!(helper.source_implementations.len(), 1);
    let allocation = &helper.source_implementations[0];
    assert!(allocation.matches_source_declaration(library, 0));
    assert!(!allocation.matches_source_declaration("proc helper {mode} {return WRONG}", 0));
    assert!(
        matches!(allocation.site.source.kind(), crate::command_binding::SourceOriginKind::Loaded {path, ..} if path.as_ref() == "library.tcl")
    );
}

#[test]
fn file_entry_does_not_license_unloaded_shadowed_or_unknown_file_reads() {
    let entry = source_loader_entry("proc helper {mode} {return $mode}");
    for source in [
        "helper dev",
        "source $path; helper dev",
        "source other.tcl; helper dev",
        "proc source {path} {}; source library.tcl; helper dev",
    ] {
        let evidence = scan_loaded_caller(source, &entry);
        assert!(
            evidence
                .get("::helper")
                .is_none_or(|callee| callee.uniform_literal_at(0).is_none()),
            "{source}"
        );
    }
    let absent = crate::command_binding::SourceAnalysisEntry {
        trusted_source_modules: Vec::new(),
        ..entry
    };
    assert!(
        scan_loaded_caller("source library.tcl; helper dev", &absent)
            .get("::helper")
            .is_none_or(|callee| callee.uniform_literal_at(0).is_none())
    );
}

#[test]
fn sourced_partial_errors_and_redefinition_preserve_real_target_identity() {
    let partial =
        "proc helper {mode} {return $mode}; error BOOM; proc helper {mode} {return WRONG}";
    let entry = source_loader_entry(partial);
    let evidence = scan_loaded_caller("catch {source library.tcl}; helper dev", &entry);
    let helper = evidence
        .get("::helper")
        .expect("definition preceding error survives");
    assert_eq!(helper.uniform_literal_at(0), Some("dev"));
    assert!(
        helper
            .source_implementations
            .iter()
            .all(|allocation| allocation.matches_source_declaration(partial, 0))
    );
    let replaced = scan_loaded_caller(
        "source library.tcl; proc helper {mode} {return NEW}; helper dev",
        &source_loader_entry("proc helper {mode} {return OLD}"),
    );
    assert!(
        replaced
            .get("::helper")
            .expect("replacement caller")
            .source_implementations
            .iter()
            .all(|allocation| !allocation
                .matches_source_declaration("proc helper {mode} {return OLD}", 0))
    );
}

#[test]
fn native_formal_seed_distinguishes_zero_arguments_from_an_opaque_caller() {
    let mut evidence = CallSiteEvidence::default();
    evidence.record_call("::helper".into(), &[]);
    let grammar = Some(tcl_dialect::ParameterGrammar::Tcl);
    let seed = native_seed("{mode prod}", grammar, &evidence).expect("default binding");
    assert_eq!(
        seed.get(&("mode".into(), 0)),
        Some(&crate::analyses::LatticeValue::Const(
            crate::analyses::ConstValue::String("prod".into())
        ))
    );
    evidence.record_opaque_caller("::helper");
    assert!(native_seed("{mode prod}", grammar, &evidence).is_none());
}

#[test]
fn native_formal_seed_requires_a_proved_ordinary_activation_layout() {
    let mut evidence = CallSiteEvidence::default();
    evidence.record_call("::helper".into(), &["same".into(), "same".into()]);
    assert!(
        native_seed(
            "mode mode",
            Some(tcl_dialect::ParameterGrammar::Tcl),
            &evidence
        )
        .is_none()
    );
    for params in ["&mode other", "args mode", "{args rest} mode", "mode other"] {
        assert!(native_seed(params, Some(tcl_dialect::ParameterGrammar::Jim), &evidence).is_none());
    }
    assert!(native_seed("mode other", None, &evidence).is_none());
    assert!(
        native_seed(
            "mode other",
            Some(tcl_dialect::ParameterGrammar::Tcl),
            &evidence
        )
        .is_some()
    );
}

#[test]
fn finite_dispatch_evidence_converges_beyond_six_dependencies() {
    let evidence = run_to_fixpoint(|previous| {
        let mut next = CallSiteEvidence {
            consulted_value_sets: true,
            ..Default::default()
        };
        next.record_call("::entry".to_owned(), &["literal".to_owned()]);
        for index in 0..9 {
            let predecessor = if index == 0 {
                "::entry".to_owned()
            } else {
                format!("::dispatch{}", index - 1)
            };
            if let Some(value) = previous
                .get(&predecessor)
                .and_then(|callee| callee.uniform_literal_at(0))
            {
                next.record_call(format!("::dispatch{index}"), &[value.to_owned()]);
            }
        }
        next
    });
    assert_eq!(
        evidence.get("::dispatch8").unwrap().uniform_literal_at(0),
        Some("literal")
    );
}

/// A call that omits a defaulted parameter binds it to its **default**, an
/// unknown value — so a literal at another call site cannot claim the
/// position. `binds_position` is what encodes that.
#[test]
fn an_omitted_argument_stops_a_position_being_uniform() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc helper {a {b x}} { return $a }\nhelper 1 two\nhelper 1\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    let helper = evidence.get("::helper").expect("calls recorded");
    assert_eq!(helper.uniform_literal_at(0), Some("1"));
    assert_eq!(helper.arg_counts, [1, 2].into_iter().collect());
    assert!(!helper.binds_position(1));
    assert_eq!(helper.uniform_literal_at(1), None);
}

/// Merging is monotone: extra evidence widens the value set and the
/// observed argument counts, so a second file can retract a fold but
/// never manufacture one.
#[test]
fn merging_only_ever_widens() {
    let reg = registry();
    let src = "proc helper {mode} { return $mode }\n";
    let mut a = scan_source_call_sites(
        &format!("{src}helper prod\n"),
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    assert_eq!(
        a.get("::helper").unwrap().uniform_literal_at(0),
        Some("prod")
    );
    let b = scan_source_call_sites(
        &format!("{src}helper dev\n"),
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    a.merge_from(&b);
    assert_eq!(a.get("::helper").unwrap().uniform_literal_at(0), None);
}

/// A `TclOO` method's relative command resolves in the receiver object's
/// namespace.  The selected implementation may itself call any procedure
/// reachable from this source with arguments the lexical CFG never sees,
/// so cross-file evidence must withdraw an otherwise-uniform literal.
#[test]
fn runtime_selected_method_command_poisons_cross_file_param_evidence() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "::helper prod\noo::class create C { method run {} { hook } }\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &["::helper".to_owned()],
    );
    let helper = evidence.get("::helper").expect("direct call recorded");
    assert_eq!(
        helper.uniform_literal_at(0),
        None,
        "an object-local hook can call helper with arbitrary arguments: {evidence:?}",
    );
}

/// Absolute method commands do not depend on the receiver namespace and
/// therefore must not discard unrelated exact call-site evidence.
#[test]
fn absolute_only_method_keeps_cross_file_param_evidence_exact() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc ::helper {mode} {return $mode}\n::helper prod\noo::class create C { method run {} { ::puts ok } }\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &["::helper".to_owned()],
    );
    let helper = evidence.get("::helper").expect("direct call recorded");
    assert_eq!(helper.uniform_literal_at(0), Some("prod"));
}

#[test]
fn known_external_name_does_not_prove_a_runtime_procedure_binding() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "helper prod\nhelper prod\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &["::helper".to_owned()],
    );
    assert!(
        evidence
            .get("::helper")
            .is_none_or(|helper| helper.uniform_literal_at(0).is_none())
    );
}

/// A deferred command prefix (`ArgRole::CommandPrefix`) invokes the proc
/// with runtime-supplied words appended, so no position stays uniform.
/// The callback slot comes from the registry, not a command-name list.
#[test]
fn a_command_prefix_callback_is_recorded_as_an_opaque_caller() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc helper {mode} {return $mode}\nhelper prod\nafter 0 helper\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    let helper = evidence.get("::helper").expect("calls recorded");
    assert!(helper.arg_counts.contains(&0), "{helper:?}");
    assert_eq!(helper.uniform_literal_at(0), None);
}

/// A `rename` / `interp alias` naming a known command moves its binding,
/// so a call can reach it under a name no scan attributed to it.
#[test]
fn a_rebinding_call_is_recorded_as_an_opaque_caller() {
    let reg = registry();
    for src in [
        "proc helper {mode} {return $mode}\nhelper prod\nrename helper legacy\n",
        "proc helper {mode} {return $mode}\nhelper prod\ninterp alias {} h {} helper\n",
    ] {
        let evidence = scan_source_call_sites(
            src,
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &known(&["::helper"]),
            &[],
        );
        let helper = evidence.get("::helper").expect("calls recorded");
        assert_eq!(helper.uniform_literal_at(0), None, "{src}");
    }
}

/// Only a **trailing** `args` is Tcl's variadic catch-all
/// (`TclCreateProc`, `generic/tclProc.c`) — in `proc f {args x}` the
/// first word is an ordinary parameter and both positions may be seeded.
#[test]
fn only_a_trailing_args_stops_the_seed() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc helper {args x} {}\nhelper one two\nhelper one two\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    let leading = seed(
        &params(&["args", "x"]),
        &evidence,
        "::helper",
        Traits::empty(),
        false,
    )
    .expect("both positions seeded");
    assert_eq!(leading.len(), 2);
    let trailing = seed(
        &params(&["x", "args"]),
        &evidence,
        "::helper",
        Traits::empty(),
        false,
    )
    .expect("the leading parameter is still seeded");
    assert_eq!(trailing.len(), 1);
    assert!(trailing.contains_key(&("x".to_owned(), 0)));
}

/// A registry-declared boundary declines the seed on its own, and a
/// host-supplied cross-file view re-enables it — but only for the kind of
/// boundary that view can actually bound.
///
/// Publishing this file's commands as an API surface (`PROVIDES_PACKAGE`
/// / `EXPORTS_COMMAND`) admits callers no project scan covers — another
/// checkout can `package require` this one — so it declines outright.
/// Pulling another unit in (`LOADS_EXTERNAL_UNIT`) names a caller the
/// host's project normally *does* contain, so it defers to the evidence.
#[test]
fn boundary_gating_distinguishes_publishing_from_loading() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc helper {mode} {}\nhelper prod\nhelper prod\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    let p = params(&["mode"]);
    assert!(
        seed(&p, &evidence, "::helper", Traits::empty(), false).is_some(),
        "no boundary at all: the visible callers are the whole story"
    );
    for boundary in [Traits::PROVIDES_PACKAGE, Traits::EXPORTS_COMMAND] {
        for has_view in [false, true] {
            assert!(
                seed(&p, &evidence, "::helper", boundary, has_view).is_none(),
                "{boundary:?} publishes to callers no enumeration bounds \
                     (cross-file view: {has_view})"
            );
        }
    }
    assert!(
        seed(
            &p,
            &evidence,
            "::helper",
            Traits::LOADS_EXTERNAL_UNIT,
            false
        )
        .is_none(),
        "a loaded unit is an unenumerated caller without a host view"
    );
    assert!(
        seed(&p, &evidence, "::helper", Traits::LOADS_EXTERNAL_UNIT, true).is_some(),
        "with the project enumerated, the loaded unit's callers are in the evidence"
    );
}

/// `scan_unit_linkage` reads the registry, so it recognises a boundary
/// however it is spelled — and never fires on a mere mention.
#[test]
fn unit_linkage_is_scanned_from_the_lowered_ir_not_the_raw_text() {
    let reg = registry();
    let cases = [
        ("package provide mylib 1.0\n", Traits::PROVIDES_PACKAGE),
        ("::package\tprovide mylib 1.0\n", Traits::PROVIDES_PACKAGE),
        ("if {1} { source other.tcl }\n", Traits::LOADS_EXTERNAL_UNIT),
        (
            "namespace eval ::a { namespace export * }\n",
            Traits::EXPORTS_COMMAND,
        ),
        (
            "# this file does not package provide anything\n",
            Traits::empty(),
        ),
        ("set msg \"package provide\"\n", Traits::empty()),
        ("namespace import ::lib::helper\n", Traits::empty()),
    ];
    for (src, want) in cases {
        let module = retained_scope_fixture(src, &reg);
        assert_eq!(scan_unit_linkage(&module, &reg, None), want, "{src:?}");
    }
}

/// A `namespace eval ::a { helper }` body calls `::a::helper`, never a
/// same-named proc in the *caller's* namespace (tclsh8.6-confirmed).
///
/// The body is scanned once, as the properly-namespaced body unit lowering
/// registers for it. Walking it a second time through the enclosing
/// statement's `ArgRole::Body` — with the caller's namespace — invents a
/// call to whatever `::helper` happens to exist. Within one file that is
/// invisible (a bare global `::helper` is rarely in a single file's own
/// `known` set); across a project it is a false edge into *another file's*
/// procedure.
#[test]
fn a_namespace_eval_body_resolves_against_its_own_namespace() {
    let reg = registry();
    let src =
        "namespace eval ::a {\n    proc helper {} { return 1 }\n    proc run {} { helper }\n}\n";
    // `::helper` is in scope project-wide, as a global proc in some other
    // file would be — the call inside `::a` must not reach it.
    let evidence = scan_source_call_sites(
        src,
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::a::helper", "::a::run", "::helper"]),
        &[],
    );
    assert_eq!(
        evidence.callees().collect::<Vec<_>>(),
        vec!["::a::helper"],
        "the call binds to ::a::helper only"
    );
    // The in-unit scan is the reference: cross-file must not see more.
    let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
    assert_eq!(
        cu.caller_scope.call_sites.callees().collect::<Vec<_>>(),
        vec!["::a::helper"],
    );
}

/// A command substitution nested in a word is a call site (#2134).
///
/// Enumerating only the statements that *are* a command let `[bump m]`
/// go unattributed, so the single visible `bump n` read as the complete
/// caller set and O100 specialised the body to `upvar 1 n v`. Measured on
/// tclsh 9.0.4, the program printed `2 11 11` and the rewritten one
/// `3 10 3`.
#[test]
fn a_nested_substitution_counts_as_a_caller() {
    let reg = registry().project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = "proc bump {name} {\n  upvar 1 $name v\n  incr v\n}\nset n 1\nset m 10\nbump n\nset z [bump m]\n";
    let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
    assert!(
        cu.caller_scope
            .call_sites
            .callees()
            .any(|callee| callee == "::bump"),
        "the nested `[bump m]` must be attributed to ::bump",
    );
    // The two callers disagree on the argument, so nothing may be seeded.
    assert_eq!(
        cu.caller_scope
            .call_sites
            .get("::bump")
            .and_then(|evidence| evidence.uniform_literal_at(0)),
        None,
        "disagreeing callers must not look uniform",
    );
}

/// Callers that agree still seed — the nested site is *counted*, not
/// treated as opaque.
#[test]
fn agreeing_callers_still_seed_through_a_substitution() {
    let reg = registry().project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src =
        "proc bump {name} {\n  upvar 1 $name v\n  incr v\n}\nset n 1\nbump n\nset z [bump n]\n";
    let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
    assert_eq!(
        cu.caller_scope
            .call_sites
            .get("::bump")
            .and_then(|evidence| evidence.uniform_literal_at(0)),
        Some("n"),
        "callers that agree on `n` must still seed",
    );
}

/// An `uplevel` body nested inside another `ArgRole::Body` must not be
/// re-walked with the enclosing unit's namespace.
///
/// The `catch` body is walked correctly — `catch` shifts nothing — and
/// the `uplevel #0` body inside it was then walked again as `::foo`,
/// inventing a call to `::foo::helper` alongside the correct `::helper`
/// that `upframe_scan_bodies` had already recorded with the right
/// frame. Both the phantom callee and the double attribution are wrong:
/// tclsh8.6/9.0 confirm `uplevel #0 { helper b }` inside `::foo::runIt`
/// calls `::helper` and nothing else.
///
/// The `Traits::EVALUATES_IN_SHIFTED_FRAME` skip is what stops it, so
/// no command name appears in the walker.
#[test]
fn retained_upframe_namespaces_do_not_parse_the_procedure_display_label() {
    let registry = registry();
    let module = crate::lowering::lower_to_ir(
        "namespace eval ::N {}\nproc ::N::run {} {uplevel 1 {uplevel 0 {puts $x}}; uplevel 0 {puts $x}; uplevel #0 {puts $x}}",
        &registry,
    );
    let procedures = module
        .procedures
        .values()
        .filter(|procedure| procedure.qualified_name == "::N::run")
        .collect::<Vec<_>>();
    assert_eq!(procedures.len(), 1);
    let mut procedure = (*procedures[0]).clone();
    let parent = procedure
        .body
        .execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected);
    assert!(!matches!(
        parent,
        crate::ir::ExecutionNamespace::RuntimeSelected
    ));
    procedure.qualified_name = "unrelated reporting label".to_owned();
    let mut callers = Vec::new();
    collect_upframes(&procedure.body, &parent, &registry, &mut callers);
    assert_eq!(callers.len(), 4);
    assert!(matches!(
        callers[0].execution_namespace,
        crate::ir::ExecutionNamespace::RuntimeSelected
    ));
    assert!(matches!(
        callers[1].execution_namespace,
        crate::ir::ExecutionNamespace::RuntimeSelected
    ));
    assert_eq!(callers[2].execution_namespace, parent);
    assert!(
        matches!(
            callers[3].execution_namespace,
            crate::ir::ExecutionNamespace::RuntimeSelected
        ),
        "unsupplied root frame cannot borrow the source body's lexical namespace"
    );
}

#[test]
fn an_uplevel_body_inside_a_catch_body_is_not_reattributed() {
    let reg = registry();
    let src = "proc ::helper {mode} {return $mode}\nnamespace eval ::foo {\n    proc helper {mode} { return $mode }\n    proc runIt {} { catch { uplevel #0 { helper b } } }\n}\n";
    let evidence = scan_source_call_sites(
        src,
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::foo::helper", "::foo::runIt", "::helper"]),
        &[],
    );
    let mut callees: Vec<&str> = evidence.callees().collect();
    callees.sort_unstable();
    assert_eq!(
        callees,
        vec!["::helper"],
        "the global helper takes the evidence and ::foo::helper gets no phantom call",
    );
}

/// `catch { … }` does *not* shift namespace, so its body must still be
/// walked with the caller's — the guard above keys on an absolute
/// `ArgRole::Name`, which `catch` has none of.
#[test]
fn a_catch_body_is_still_walked_with_the_callers_namespace() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc helper {mode} { return $mode }\ncatch { helper prod }\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    assert_eq!(
        evidence
            .get("::helper")
            .and_then(|e| e.uniform_literal_at(0)),
        Some("prod"),
        "the call inside catch is still evidence"
    );
}

/// `slice_for` is driven by the *callee* names a file declares, so its
/// cost is that file's procedure count rather than the project's.
#[test]
fn slice_for_keeps_only_the_named_callees() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc a {value} {}\nproc b {value} {}\na 1\nb 2\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::a", "::b"]),
        &[],
    );
    let sliced = evidence.slice_for(["::a"].into_iter());
    assert_eq!(sliced.callees().collect::<Vec<_>>(), vec!["::a"]);
    assert!(evidence.slice_for(["::zzz"].into_iter()).is_empty());
}

/// Build the evidence for `src` under a named dialect's registry, the
/// way `CompilationUnit` does minus the method / body-unit callers
/// (which need a CFG context these unit tests do not exercise).
fn evidence_for_dialect(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> CallSiteEvidence {
    let reg = tcl_registry::model::ingress::static_context_for_profile(dialect).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.grammar);
    // The scan authenticates each statement against the module's own
    // resolved input, so the fixture lowers under one, over the same
    // command store it scans with.
    let input = crate::analyser::ResolvedAnalysisInput::new(
        dialect,
        dialect,
        std::sync::Arc::new(
            crate::environment_ingress::context_for_profile(dialect)
                .with_command_store(reg.snapshot().shared_registry()),
        ),
        config,
    );
    let ir = crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(reg, config)
            .with_dialect(Some(dialect))
            .with_resolved_analysis_input(input),
        src,
    );
    let cfg_module = crate::cfg_builder::build_cfg(&ir, false);
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        src,
        ir.lexer_config,
        reg,
        &ir.source_entry,
    );
    let identities = ir.source_metadata_input.as_ref().map_or_else(
        || identities.clone(),
        |input| {
            identities
                .clone()
                .with_resolved_analysis_input(input.clone())
        },
    );
    collect_call_site_constants(
        &cfg_module,
        &[],
        &ir.procedures,
        &ir.future_call_sites,
        CallSiteSourceContext {
            registry: reg,
            module: &ir,
            declared: None,
            input: ir.source_metadata_input.as_ref(),
            identities: &identities,
        },
    )
}

fn evidence(src: &str) -> CallSiteEvidence {
    evidence_for_dialect(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

#[test]
fn deferred_global_callers_retract_only_their_retained_possible_targets() {
    let evidence = evidence(
        "proc helper {x} {return $x}\nproc other {x} {return $x}\nhelper prod\nother prod\nafter 0 {other $::runtime_value}",
    );
    assert_eq!(
        uniform(&evidence, "::helper", 0).as_deref(),
        Some("prod"),
        "{evidence:#?}"
    );
    assert!(
        evidence
            .get("::other")
            .is_some_and(|site| site.opaque_caller)
    );
    let unknown = self::evidence(
        "proc helper {x} {return $x}\nhelper prod\nafter idle {$::runtime_command $::runtime_value}",
    );
    assert!(
        unknown
            .get("::helper")
            .is_some_and(|site| site.opaque_caller)
    );
}

/// Whether `index` still has a single agreed literal across every
/// recorded caller — the exact predicate the seed reads.  This model
/// records "a caller that never reached this position" in `arg_counts`
/// rather than in the slot, so probing the slot alone would miss it.
fn uniform(ev: &CallSiteEvidence, qname: &str, index: usize) -> Option<String> {
    ev.get(qname)?
        .uniform_literal_at(index)
        .map(ToOwned::to_owned)
}

/// The literal values recorded at `index` for `qname`, sorted, plus
/// whether that position is poisoned.
fn slot(ev: &CallSiteEvidence, qname: &str, index: usize) -> (Vec<String>, bool) {
    let Some(consts) = ev.get(qname).and_then(|e| e.slots.get(&index)) else {
        return (Vec::new(), false);
    };
    let mut values: Vec<String> = consts.values.iter().cloned().collect();
    values.sort();
    (values, consts.unknown)
}

#[test]
fn literal_call_sites_are_recorded_and_nothing_is_withdrawn() {
    let ev = evidence("proc helper {mode} { return $mode }\nhelper a\nhelper b\n");
    assert_eq!(
        slot(&ev, "::helper", 0),
        (vec!["a".into(), "b".into()], false)
    );
    // Two disagreeing literals, so no uniform value — but the position is
    // *bound* by every recorded call, which a withdrawal would undo.
    assert!(ev.get("::helper").unwrap().binds_position(0));
}

#[test]
fn the_unseen_call_marker_is_not_a_zero_argument_caller() {
    let ev = evidence(
        "proc {<unseen-call>} {mode} { return $mode }\n\
             {<unseen-call>} fixed\n\
             set ignored [missing_command]\n",
    );
    assert_eq!(
        uniform(&ev, "::<unseen-call>", 0),
        Some("fixed".into()),
        "the synthetic marker is analysis-only; the real invocation remains evidence",
    );
}

// A module's own `unknown` handler. Tcl dispatches every
// unresolved command word to it, so its direct callers are never its
// complete caller set. tclsh8.6/9.0-confirmed: with `proc unknown {cmd
// args}` in scope, `bogus beta gamma` runs the handler with
// `cmd` = `bogus`, `args` = `beta gamma`.

#[test]
fn an_unresolved_word_is_a_call_site_of_the_modules_unknown_handler_1044() {
    // TP: `bogus` names nothing, so real Tcl calls the handler with
    // `bogus`. Seeing only the two `unknown alpha`
    // calls, the scan would bind `cmd` to the constant `"alpha"` and fold
    // `$cmd eq "alpha"` on a genuinely runtime-varying condition.
    let ev = evidence(
        "proc unknown {cmd args} { if {$cmd eq \"alpha\"} { return 1 } else { return 2 } }\nunknown alpha\nunknown alpha\nbogus beta\n",
    );
    assert_eq!(
        slot(&ev, "::unknown", 0),
        (vec!["alpha".into(), "bogus".into()], false),
        "the unresolved word itself is the handler's first argument",
    );
    assert_eq!(uniform(&ev, "::unknown", 0), None, "must not fold");
}

#[test]
fn the_unresolved_words_own_arguments_follow_it_into_the_handler_1044() {
    // TP: Tcl passes the failed call's arguments after the word, so
    // `args`' positions carry them — evidence the scan really can see,
    // recorded in full rather than merely poisoned.
    let ev = evidence("proc unknown {cmd args} { return $cmd }\nbogus beta gamma\n");
    assert_eq!(slot(&ev, "::unknown", 0), (vec!["bogus".into()], false));
    assert_eq!(slot(&ev, "::unknown", 1), (vec!["beta".into()], false));
    assert_eq!(slot(&ev, "::unknown", 2), (vec!["gamma".into()], false));
}

#[test]
fn a_module_with_no_unknown_handler_is_unaffected_1044() {
    // TN, the common case and the whole regression risk: an unresolved
    // word in a module that defines no handler must change nothing.
    let ev = evidence("proc helper {mode} { return $mode }\nhelper a\nbogus beta\n");
    assert_eq!(slot(&ev, "::helper", 0), (vec!["a".into()], false));
    assert_eq!(uniform(&ev, "::helper", 0).as_deref(), Some("a"));
    assert!(
        ev.get("::unknown").is_none(),
        "no handler defined, so nothing may be attributed to one: {ev:?}",
    );
}

#[test]
fn a_handler_never_seeds_even_when_every_visible_caller_agrees_1044() {
    // The handler's caller set is unenumerable *by construction*, so
    // agreement among the callers a scan can see proves nothing.
    //
    // "With no unresolved word in the file the direct callers really are
    // all of them" is false: real Tcl routes to the handler
    // every word that resolves to nothing at the instant of the call:
    // an auto-loaded name, a name another sourced file introduces, a
    // name built by string arithmetic, a name typed at a prompt. None of
    // those appear in the source for any scan to find.
    //
    // On tclsh8.6, the seeded words are
    // wrong in *both* directions. `Dog new` after `oo::class create Dog`
    // and `worker` after `coroutine worker body` are recorded as
    // dispatches, yet neither ever reaches the handler. And a `bogus
    // beta` written *before* `proc unknown` is handled by the builtin
    // `::unknown` — it errors — so it is not a call site of this
    // handler either.
    let ev = evidence(
        "proc unknown {cmd args} { return $cmd }\nunknown alpha\nunknown alpha\nputs hi\n",
    );
    assert_eq!(
        uniform(&ev, "::unknown", 0),
        None,
        "defining the handler is itself the unenumerable caller: {ev:?}",
    );
}

#[test]
fn a_class_command_never_seeds_the_handler_1044() {
    // On tclsh8.6, `oo::class create Dog` binds `Dog`, so `Dog
    // new` dispatches to the class command and the handler is never
    // called. The scan cannot resolve `Dog` and records it as an
    // unresolved-word dispatch anyway; the unconditional poison is what
    // stops that invented evidence becoming a fold.
    let ev = evidence(
        "proc unknown {cmd args} { if {$cmd eq \"Dog\"} { return 1 } else { return 2 } }\noo::class create Dog {\n    method bark {} { return woof }\n}\nDog new\n",
    );
    assert_eq!(
        uniform(&ev, "::unknown", 0),
        None,
        "a class command is not a handler call site: {ev:?}",
    );
}

#[test]
fn a_coroutine_command_never_seeds_the_handler_1044() {
    // On tclsh8.6, `coroutine worker body` binds `worker`, so
    // calling it resumes the coroutine and the handler is never called.
    let ev = evidence(
        "proc unknown {cmd args} { if {$cmd eq \"worker\"} { return 1 } else { return 2 } }\nproc body {} { yield ; return done }\ncoroutine worker body\nworker\n",
    );
    assert_eq!(
        uniform(&ev, "::unknown", 0),
        None,
        "a coroutine command is not a handler call site: {ev:?}",
    );
}

#[test]
fn a_word_written_before_the_handler_never_seeds_it_1044() {
    // On tclsh8.6, `bogus beta` on line 1, with `proc unknown`
    // defined only afterwards, is handled by the *builtin* `::unknown`
    // and errors with `invalid command name "bogus"`. The scan is
    // definition-order-insensitive, so it records the dispatch anyway.
    let ev = evidence(
        "bogus beta\nproc unknown {cmd args} { if {$cmd eq \"bogus\"} { return 1 } else { return 2 } }\n",
    );
    assert_eq!(
        uniform(&ev, "::unknown", 0),
        None,
        "an order-insensitive scan may not seed an order-sensitive dispatch: {ev:?}",
    );
}

#[test]
fn a_registry_builtin_is_not_an_unresolved_word_1044() {
    // FP guard: a builtin resolves to no *user proc*, but it is not an
    // unresolved word — Tcl never routes `puts`/`set` to the handler.
    let ev = evidence(
        "proc unknown {cmd args} { return $cmd }\nunknown alpha\nunknown alpha\nputs hi\nset x 1\nincr x\n",
    );
    assert_eq!(
        slot(&ev, "::unknown", 0),
        (vec!["alpha".into()], false),
        "builtins must not appear as handler arguments: {ev:?}",
    );
}

#[test]
fn an_unenumerable_dispatch_word_still_poisons_rather_than_naming_the_handler_1044() {
    // FN guard: a dynamic word whose value set is unreadable is not an
    // *unresolved* word — the scan cannot say it resolves to nothing —
    // so it keeps withdrawing every seed, the handler's included.
    let ev = evidence(
        "proc unknown {cmd args} { return $cmd }\nunknown alpha\nunknown alpha\nset c [gets stdin]\n$c beta\n",
    );
    assert_eq!(
        uniform(&ev, "::unknown", 0),
        None,
        "an unreadable dispatch may name anything, handler included: {ev:?}",
    );
}

#[test]
fn a_cross_file_dispatch_never_seeds_another_files_handler_1044() {
    // The cross-file scan resolves against the *project's* names, so a
    // file that defines no handler still attributes its unresolved words
    // to one another file defines. That path needs the same poison, or
    // the miscompile simply moves across the file boundary.
    //
    // Shape: `h.tcl` holds `proc unknown`, `c.tcl` holds
    // `oo::class create Dog` + `Dog new`. The `Dog` dispatch is invented
    // (tclsh8.6: the class command answers, the handler is never called),
    // and it would be the handler's only recorded call site.
    let reg = registry();
    let evidence = scan_source_call_sites(
        "oo::class create Dog { method bark {} { return woof } }\nDog new\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::unknown"]),
        &[],
    );
    assert_eq!(
        evidence
            .get("::unknown")
            .and_then(|e| e.uniform_literal_at(0)),
        None,
        "a sibling file's dispatch may not seed the project's handler: {evidence:?}",
    );
}

#[test]
fn only_one_command_carries_the_unresolved_handler_trait() {
    // `unresolved_command_handler` takes the first match from a hash-map
    // walk, so more than one carrier would make the answer depend on
    // iteration order. Sorting makes it deterministic; this pins the
    // stronger property that there is nothing to choose between.
    let reg = registry();
    let carriers = reg.commands_with_trait(Traits::UNRESOLVED_COMMAND_HANDLER);
    assert_eq!(
        carriers.len(),
        1,
        "a second carrier needs a resolution rule, not an arbitrary pick: {carriers:?}",
    );
}

#[test]
fn a_dispatch_through_a_literal_variable_is_recorded_as_a_call_site() {
    let ev = evidence("proc helper {mode} { return $mode }\nset cmd helper\n$cmd dev\n");
    assert_eq!(slot(&ev, "::helper", 0), (vec!["dev".into()], false));
    assert_eq!(
        uniform(&ev, "::helper", 0).as_deref(),
        Some("dev"),
        "an enumerable dispatch is an ordinary call site, not a withdrawal",
    );
}

/// A registry-declared user-proc invoker (`Traits::INVOKES_USER_PROC`,
/// the iRules `call PROC ?args?` form) names its callee in the first
/// argument and passes the rest — a call site a walk that only ever
/// looked at the command word would miss entirely.
#[test]
fn a_registry_declared_user_proc_invoker_is_a_call_site() {
    let ev = evidence_for_dialect(
        "proc helper {mode} { return $mode }\nwhen RULE_INIT { call helper dev }\n",
        tcl_dialect::DialectProfile::irules(),
    );
    assert_eq!(uniform(&ev, "::helper", 0).as_deref(), Some("dev"));
}

#[test]
fn normal_procedure_invoker_retains_operand_offsets_and_real_uncertainty() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source caller metadata; no appliance or entered-frame claim.
    let profile = tcl_dialect::DialectProfile::irules();
    let exact = evidence_for_dialect(
        "proc helper {mode} {return $mode}\nwhen RULE_INIT {call -debug helper dev}\n",
        profile,
    );
    assert_eq!(uniform(&exact, "::helper", 0).as_deref(), Some("dev"));
    let shadowed = evidence_for_dialect(
        "proc helper {mode} {return $mode}\nproc call {args} {}\nwhen RULE_INIT {call helper dev}\n",
        profile,
    );
    assert_eq!(uniform(&shadowed, "::helper", 0), None);
    let unknown = evidence_for_dialect(
        "proc helper {mode} {return $mode}\nwhen RULE_INIT {call $target dev}\n",
        profile,
    );
    assert_eq!(uniform(&unknown, "::helper", 0), None);
}

#[test]
fn an_unenumerable_dispatch_marks_the_module_non_enumerable() {
    let ev = evidence("proc helper {mode} { return $mode }\nset cmd [gets stdin]\n$cmd dev\n");
    assert_eq!(uniform(&ev, "::helper", 0), None);
}

#[test]
fn a_callback_prefix_poisons_every_parameter_of_its_target() {
    let ev = evidence("proc cmp {a b} { return 0 }\nlsort -command cmp {x y}\n");
    assert_eq!(uniform(&ev, "::cmp", 0), None);
    assert_eq!(uniform(&ev, "::cmp", 1), None);
}

/// The `trace add variable v write cb` shape. The
/// callback's position is registry data (`ArgRole::CommandPrefix` on the
/// `trace add variable` subcommand), and the runtime appends the trace's
/// own three arguments, so every parameter of the named proc is poisoned.
#[test]
fn a_trace_callback_registration_is_a_call_site() {
    for prefix in ["cb", "[list cb]"] {
        let src = format!(
            "proc cb {{name1 name2 op}} {{ return }}\nproc go {{}} {{ trace add variable v write {prefix} }}\n"
        );
        let ev = evidence(&src);
        assert_eq!(
            uniform(&ev, "::cb", 0),
            None,
            "`trace … write {prefix}` invokes cb with runtime-supplied arguments",
        );
    }
}

/// An unenumerable dispatch withdraws every seed in the unit that
/// contains it — expressed per callee, so it survives a merge.
#[test]
fn an_unenumerable_dispatch_withdraws_this_units_seeds() {
    let ev = evidence(
        "proc helper {mode} { return $mode }\nhelper prod\nhelper prod\nset cmd [gets stdin]\n$cmd dev\n",
    );
    assert_eq!(uniform(&ev, "::helper", 0), None);
}

/// ...but only that unit's.  A file declaring no linkage cannot dispatch
/// against a command table it never loaded, so merging its evidence into
/// a project must not disturb an unrelated file's sound seed.  Without
/// the reach bound, one `eval $script` anywhere in a workspace withdrew
/// every interprocedural seed in every file.
#[test]
fn an_unenumerable_dispatch_does_not_reach_an_unlinked_file() {
    let opaque = evidence("proc mine {x} { return $x }\nset cmd [gets stdin]\n$cmd dev\n");
    let mut project = evidence("proc theirs {mode} { return $mode }\ntheirs prod\ntheirs prod\n");
    project.merge_from(&opaque);
    assert_eq!(
        uniform(&project, "::theirs", 0).as_deref(),
        Some("prod"),
        "the dispatching file declares no `source`/`package require`, so it \
             cannot name `theirs` at all",
    );
    assert_eq!(
        uniform(&project, "::mine", 0),
        None,
        "its own procedure is still withdrawn",
    );
}

/// The host supplies the reach, so a file linked to the callee's — in
/// *either* direction — retracts its seed.
///
/// The inbound direction is the one a file-local bound gets wrong: a
/// library declaring no `source` of its own still runs inside its
/// sourcer's interpreter, so its unreadable dispatch can name the
/// sourcer's procedures. Bounding by what the *scanning* file loads
/// missed exactly that.
#[test]
fn an_unenumerable_dispatch_reaches_whatever_the_host_says_it_links_to() {
    let reg = registry();
    let known: HashSet<String> = ["::theirs".to_owned()].into_iter().collect();
    let linked = ["::theirs".to_owned()];
    for src in [
        // Outbound: this file sources the other.
        "source other.tcl\nset cmd [gets stdin]\n$cmd dev\n",
        // Inbound: this file declares no linkage at all, but the host put
        // it in the callee's component because something sources it.
        "set cmd [gets stdin]\n$cmd dev\n",
    ] {
        let opaque = scan_source_call_sites(
            src,
            &reg,
            None,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &known,
            &linked,
        );
        let mut project =
            evidence("proc theirs {mode} { return $mode }\ntheirs prod\ntheirs prod\n");
        project.merge_from(&opaque);
        assert_eq!(
            uniform(&project, "::theirs", 0),
            None,
            "a linked file's unreadable dispatch can name ::theirs: {src}",
        );
    }
}

/// ...and an *unlinked* file cannot: the host leaves it out of the reach,
/// so its `$cmd` does not disturb a sound seed in a file it shares only a
/// workspace folder with.
#[test]
fn an_unenumerable_dispatch_does_not_reach_an_unlinked_project_file() {
    let reg = registry();
    let known: HashSet<String> = ["::theirs".to_owned()].into_iter().collect();
    let opaque = scan_source_call_sites(
        "proc mine {x} { return $x }\nset cmd [gets stdin]\n$cmd dev\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &known,
        // Not in the callee's component: its reach is its own procs.
        &["::mine".to_owned()],
    );
    let mut project = evidence("proc theirs {mode} { return $mode }\ntheirs prod\ntheirs prod\n");
    project.merge_from(&opaque);
    assert_eq!(
        uniform(&project, "::theirs", 0).as_deref(),
        Some("prod"),
        "an unlinked file shares no interpreter, so it cannot name ::theirs",
    );
}

#[test]
fn an_omitted_defaulted_argument_poisons_its_slot() {
    let ev = evidence("proc helper {a {b DEFAULT}} { return $a }\nhelper one\n");
    assert_eq!(slot(&ev, "::helper", 0), (vec!["one".into()], false));
    assert_eq!(
        uniform(&ev, "::helper", 1),
        None,
        "the omitted argument leaves b bound to its default",
    );
}

#[test]
fn a_script_the_scan_cannot_read_makes_the_module_non_enumerable() {
    for src in [
        "proc helper {mode} { return $mode }\nset s [gets stdin]\neval $s\n",
        "proc helper {mode} { return $mode }\ncatch $body\n",
        "proc helper {mode} { return $mode }\napply $fn 1\n",
    ] {
        assert!(
            uniform(&evidence(src), "::helper", 0).is_none(),
            "a script received as a value may call anything: {src}",
        );
    }
}

#[test]
fn a_retained_script_value_records_its_actual_caller() {
    let ev = evidence("proc helper {mode} { return $mode }\nset s {helper dev}\neval $s\n");
    assert_eq!(uniform(&ev, "::helper", 0).as_deref(), Some("dev"));
    assert!(!ev.get("::helper").unwrap().opaque_caller);
}

#[test]
fn a_literal_body_mentioning_a_variable_is_still_walked() {
    let ev = evidence("proc helper {mode} { return $mode }\ncatch {helper $x}\n");
    assert_eq!(slot(&ev, "::helper", 0), (Vec::new(), true));
    // The distinction that matters: the body was *walked*, so the call
    // was recorded at its real arity of one.  A withdrawal would have
    // recorded a zero-argument opaque caller instead.
    assert_eq!(
        ev.get("::helper")
            .unwrap()
            .arg_counts
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![1],
        "`catch {{helper $x}}` carries readable script text",
    );
}

#[test]
fn whole_substitution_recognises_only_a_single_reference() {
    assert!(word_is_whole_substitution(
        "$script",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(word_is_whole_substitution(
        "${script}",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(word_is_whole_substitution(
        "[build]",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(
        !word_is_whole_substitution("puts $v", tcl_lexer::LexerConfig::default()),
        "a braced body that merely mentions a variable is literal script text",
    );
    assert!(!word_is_whole_substitution(
        "helper dev",
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn command_prefix_head_treats_literal_brackets_as_list_data() {
    let reg = registry();
    assert_eq!(
        command_prefix_head(&reg, "cmp -nocase").as_deref(),
        Some("cmp")
    );
    assert_eq!(
        command_prefix_head(&reg, "[list cmp $x]").as_deref(),
        Some("[list")
    );
    assert_eq!(
        command_prefix_head(&reg, "[pickCallback]").as_deref(),
        Some("[pickCallback]")
    );
    assert_eq!(
        command_prefix_head(&reg, "{cmp name} tail").as_deref(),
        Some("cmp name")
    );
}

#[test]
fn original_callback_callers_share_source_horizons_without_parameter_seeds() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    for prefix in ["{cb extra}", "[list cb $x]", "[make $x]"] {
        let source = format!(
            "proc cb {{value args}} {{}}; interp alias {{}} make {{}} list cb; cb FIXED; lsort -command {prefix} {{3 1 2}}"
        );
        let result = evidence_for_dialect(&source, profile);
        assert!(
            result.get("::cb").is_some_and(|site| site.opaque_caller),
            "{source}: {result:?}"
        );
        assert_eq!(uniform(&result, "::cb", 0), None);
    }
    let source =
        "proc cb {value args} {}; cb FIXED; proc list args {}; lsort -command [list cb $x] {3 1 2}";
    let result = evidence_for_dialect(source, profile);
    assert_eq!(
        uniform(&result, "::cb", 0),
        None,
        "a computed prefix's target is unavailable"
    );
}

#[test]
fn scope_var_facts_separate_literal_from_unknown_writes() {
    let reg = registry();
    let ir = retained_scope_fixture(
        "proc p {} {\n set a one\n set b [gets stdin]\n set c two\n set c three\n}\n",
        &reg,
    );
    let cfg_module = build_cfg(&ir, false);
    let cfg = fixture_id_if_present(&cfg_module.procedures, "::p")
        .and_then(|id| cfg_module.procedures.get(&id))
        .expect("p lowered");
    let facts = collect_scope_var_facts(
        cfg,
        &surface(&reg),
        ir.source_metadata_input
            .as_ref()
            .map(crate::analyser::ResolvedAnalysisInput::context_registry)
            .as_deref(),
    );
    assert!(!facts.dynamic_name_write);
    assert_eq!(
        facts.vars["a"].values.iter().cloned().collect::<Vec<_>>(),
        vec!["one".to_owned()],
    );
    assert!(facts.vars["b"].unknown);
    assert_eq!(
        facts.vars["c"].values.iter().cloned().collect::<Vec<_>>(),
        vec!["three".to_owned(), "two".to_owned()],
    );
}

#[test]
fn a_write_through_a_computed_variable_name_poisons_the_whole_scope() {
    let reg = registry();
    let ir = retained_scope_fixture("proc p {n} {\n set a one\n set $n two\n}\n", &reg);
    let cfg_module = build_cfg(&ir, false);
    let cfg = fixture_id_if_present(&cfg_module.procedures, "::p")
        .and_then(|id| cfg_module.procedures.get(&id))
        .expect("p lowered");
    assert!(
        collect_scope_var_facts(
            cfg,
            &surface(&reg),
            ir.source_metadata_input
                .as_ref()
                .map(crate::analyser::ResolvedAnalysisInput::context_registry)
                .as_deref()
        )
        .dynamic_name_write
    );
}

#[test]
fn a_scope_alias_declaration_makes_the_aliased_name_unknown() {
    let reg = registry();
    let ir = retained_scope_fixture("proc p {} {\n global cmd\n set other one\n}\n", &reg);
    let cfg_module = build_cfg(&ir, false);
    let cfg = fixture_id_if_present(&cfg_module.procedures, "::p")
        .and_then(|id| cfg_module.procedures.get(&id))
        .expect("p lowered");
    let facts = collect_scope_var_facts(
        cfg,
        &surface(&reg),
        ir.source_metadata_input
            .as_ref()
            .map(crate::analyser::ResolvedAnalysisInput::context_registry)
            .as_deref(),
    );
    assert!(
        facts.vars["cmd"].unknown,
        "`global cmd` binds an outer variable any other body may write",
    );
}
fn supplied_unit_fixture(
    source: &str,
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> IrModule {
    let registry = context.commands();
    crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(
            registry,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
        )
        .with_dialect(registry.profile())
        .with_context_registry(std::sync::Arc::clone(context)),
        source,
    )
}

fn supplied_unit_input(
    current: &crate::analyser::ResolvedAnalysisInput,
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> crate::analyser::ResolvedAnalysisInput {
    crate::analyser::ResolvedAnalysisInput::new(
        current.analyser_profile(),
        current.unit_profile(),
        context,
        current.lexer_config(),
    )
}

#[test]
fn unit_linkage_keeps_supplied_availability_without_inventing_missing_boundaries() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional Registry trait inventory only; no actual loading or export claim.
    let baseline = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut descriptor = registry.get("dict").unwrap().clone();
    descriptor.traits |= Traits::LOADS_EXTERNAL_UNIT;
    registry.insert(descriptor);
    let context = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
    let mut module = supplied_unit_fixture("dict create key value", &context);
    let registry = context.commands();
    let input = module.source_metadata_input.clone().unwrap();
    assert!(std::sync::Arc::ptr_eq(&input.context_registry(), &context));
    assert_eq!(
        scan_unit_linkage(&module, registry, None),
        Traits::LOADS_EXTERNAL_UNIT
    );
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry)),
    );
    assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
    let original_script = module.top_level.clone();
    for withheld in [
        Some(supplied_unit_input(&input, older)),
        Some(supplied_unit_input(
            &input,
            crate::environment_ingress::context_for_profile(
                tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
            ),
        )),
        None,
    ] {
        module.source_metadata_input = withheld;
        assert_eq!(
            scan_unit_linkage(&module, registry, Some(input.unit_profile())),
            Traits::empty()
        );
        assert_eq!(module.top_level, original_script);
    }
}

#[test]
fn scope_variable_facts_keep_actual_availability_and_refuse_missing_or_foreign_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Original normal-write metadata only; no physical variable or frame claim.
    let context = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    let registry = context.commands();
    let module = supplied_unit_fixture("dict set target key value", &context);
    let cfg = crate::cfg_builder::build_cfg(&module, false);
    let current = collect_scope_var_facts(&cfg.top_level, &surface(registry), Some(&context));
    assert!(!current.dynamic_name_write && !current.cross_frame_write);
    assert!(current.vars["target"].unknown);
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(registry));
    assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
    let foreign = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
    );
    for withheld in [Some(&older), Some(foreign.as_ref()), None] {
        let facts = collect_scope_var_facts(&cfg.top_level, &surface(registry), withheld);
        assert!(facts.dynamic_name_write && facts.cross_frame_write);
    }
}

fn supplied_unit_callers(
    source: &str,
    module: &IrModule,
    input: Option<&crate::analyser::ResolvedAnalysisInput>,
) -> CallSiteEvidence {
    let registry = module.resolved_registry();
    let cfg = crate::cfg_builder::build_cfg(module, false);
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        source,
        module.lexer_config,
        registry,
        &module.source_entry,
    );
    let identities = input.map_or_else(
        || identities.clone(),
        |input| {
            identities
                .clone()
                .with_resolved_analysis_input(input.clone())
        },
    );
    collect_call_site_constants(
        &cfg,
        &[],
        &module.procedures,
        &module.future_call_sites,
        CallSiteSourceContext {
            registry,
            module,
            declared: None,
            input,
            identities: &identities,
        },
    )
}

#[test]
fn unit_caller_seeds_require_the_retained_metadata_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual source-owned caller operands, not a complete Native command world.
    let context = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    let source = "proc helper {mode} {return $mode}; helper FIXED";
    let module = supplied_unit_fixture(source, &context);
    let input = module.source_metadata_input.as_ref().unwrap();
    let current = supplied_unit_callers(source, &module, Some(input));
    assert_eq!(uniform(&current, "::helper", 0).as_deref(), Some("FIXED"));
    let foreign = supplied_unit_input(
        input,
        crate::environment_ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        ),
    );
    for withheld in [Some(&foreign), None] {
        let evidence = supplied_unit_callers(source, &module, withheld);
        assert!(evidence.get("::helper").unwrap().opaque_caller);
        assert_eq!(uniform(&evidence, "::helper", 0), None);
    }
}

#[test]
fn supplied_native_caller_scan_retains_source_input_without_entry_or_frame_donation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let context = std::sync::Arc::new(crate::environment_ingress::context_for_profile(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    let registry = context.commands();
    // Unknown history is an explicit entry; with none supplied the
    // document runs in a fresh interpreter (user decision 2026-10-19T14:20Z).
    let entry = crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input);
    let source = "proc helper {} {}; $command unknown";
    let (module, cfg, _) = lower_source_callers(
        source,
        registry,
        None,
        profile,
        SourceCallSiteEntry {
            entry: Some(&entry),
            input: Some(&input),
        },
    );
    assert_eq!(module.source_metadata_input.as_ref(), Some(&input));
    assert!(
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &module)
            .is_some()
    );
    assert_eq!(
        cfg.top_level.metadata_context.source_analysis_input(),
        Some(&input)
    );
    assert!(module.source_entry.unknown_entry);
    assert!(module.source_entry.native_entry.is_none());
    assert!(
        module.procedures.is_empty(),
        "source cards cannot grant a Native body entry"
    );
    let evidence = scan_source_call_sites_with_source_input(
        source,
        None,
        &known(&["::helper"]),
        &["::helper".to_owned()],
        &input,
        &entry,
    );
    assert!(evidence.get("::helper").unwrap().opaque_caller);
    assert_eq!(uniform(&evidence, "::helper", 0), None);
}

#[test]
fn source_entry_caller_metadata_requires_an_independent_supplied_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Trusted source allocation remains independent of source metadata availability.
    let source = "source library.tcl; helper dev";
    let entry = source_loader_entry("proc helper {mode} {return $mode}");
    let current = scan_loaded_caller(source, &entry);
    assert_eq!(uniform(&current, "::helper", 0).as_deref(), Some("dev"));
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let missing = scan_source_call_sites_with_source_entry(
        source,
        &registry(),
        None,
        profile,
        &known(&["::helper"]),
        &["::helper".to_owned()],
        &entry,
    );
    assert_eq!(uniform(&missing, "::helper", 0), None);
    assert!(missing.get("::helper").unwrap().opaque_caller);
}

fn original_caller_fixture(source: &str, config: tcl_lexer::LexerConfig) -> IrModule {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let registry = CommandRegistry::build_default();
    let context = std::sync::Arc::new(
        tcl_registry::model::context_for_profile(profile)
            .with_command_store(std::sync::Arc::new(registry)),
    );
    let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    assert!(input.has_logical_source_name_context());
    crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(input.borrowed_context_registry().commands(), config)
            .with_dialect(Some(profile))
            .with_resolved_analysis_input(input.clone()),
        source,
    )
}

fn original_caller_cfg_evidence(
    source: &str,
    module: &IrModule,
    cfg: &CfgModule,
) -> CallSiteEvidence {
    let input = module.source_metadata_input.as_ref().unwrap();
    let registry = module.resolved_registry();
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        source,
        module.lexer_config,
        registry,
        &module.source_entry,
    )
    .with_resolved_analysis_input(input.clone());
    collect_call_site_constants(
        cfg,
        &[],
        &module.procedures,
        &module.future_call_sites,
        CallSiteSourceContext {
            registry,
            module,
            declared: None,
            input: Some(input),
            identities: &identities,
        },
    )
}

#[test]
fn original_cfg_caller_inventory_keeps_consumed_expression_source_and_configuration() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Source caller alternatives only; no Native parameter or Normal grant.
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let source =
        "proc helper {mode} {return $mode}\nhelper FIXED\nset result [expr {[helper DIFFERENT]}]";
    let module = original_caller_fixture(source, config);
    assert!(fixture_id_if_present(&module.procedures, "::helper").is_some());
    let cfg = crate::cfg_builder::build_cfg(&module, false);
    assert!(
        cfg.top_level.blocks.iter().any(|(&block, data)| {
            data.statements.iter().enumerate().any(|(index, stmt)| {
                matches!(stmt, Statement::AssignExpr { .. })
                    && stmt.tokens().is_none()
                    && cfg.top_level.source_tokens_at(block, index).is_some()
            })
        }),
        "the consumed expression keeps its genuine sidecar"
    );
    let current = original_caller_cfg_evidence(source, &module, &cfg);
    let caller = current.get("::helper").expect("actual caller alternatives");
    assert_eq!(
        caller.slots[&0].values,
        BTreeSet::from(["FIXED".to_owned(), "DIFFERENT".to_owned()])
    );
    assert_eq!(uniform(&current, "::helper", 0), None);
    assert_eq!(module.lexer_config, config);
    assert!(module.source_entry.native_entry.is_none());
}

#[test]
fn original_cfg_caller_inventory_refuses_missing_foreign_and_stale_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let source = "proc helper {mode} {return $mode}\nhelper FIXED";
    let module = original_caller_fixture(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let cfg = crate::cfg_builder::build_cfg(&module, false);
    let current = original_caller_cfg_evidence(source, &module, &cfg);
    assert_eq!(uniform(&current, "::helper", 0).as_deref(), Some("FIXED"));
    for owner in [
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
        crate::registry_invocation::OwnedInvocationMetadataContext::Standalone,
    ] {
        let mut unavailable = cfg.clone();
        unavailable.top_level.metadata_context = owner;
        let evidence = original_caller_cfg_evidence(source, &module, &unavailable);
        assert!(evidence.get("::helper").unwrap().opaque_caller);
        assert_eq!(uniform(&evidence, "::helper", 0), None);
    }
    let input = module.source_metadata_input.as_ref().unwrap();
    let mut changed_config = input.lexer_config();
    changed_config.leading_bom = if changed_config.leading_bom == tcl_lexer::LeadingBom::Skip {
        tcl_lexer::LeadingBom::Content
    } else {
        tcl_lexer::LeadingBom::Skip
    };
    let changed = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        changed_config,
    );
    let mut foreign_registry = CommandRegistry::build_default();
    foreign_registry.insert(tcl_registry::CommandSpec {
        name: "foreign_caller_source_store",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let foreign_context = std::sync::Arc::new(
        tcl_registry::model::context_for_profile(profile)
            .with_command_store(std::sync::Arc::new(foreign_registry)),
    );
    let foreign = supplied_unit_input(input, foreign_context);
    for replacement in [changed, foreign] {
        let mut unavailable = cfg.clone();
        unavailable.top_level.metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::SuppliedSource(Box::new(
                replacement,
            ));
        let evidence = original_caller_cfg_evidence(source, &module, &unavailable);
        assert!(evidence.get("::helper").unwrap().opaque_caller);
        assert_eq!(uniform(&evidence, "::helper", 0), None);
    }
    let mut changed = module.clone();
    changed.lexer_config = changed_config;
    let evidence = original_caller_cfg_evidence(source, &changed, &cfg);
    assert!(evidence.get("::helper").unwrap().opaque_caller);
    assert_eq!(uniform(&evidence, "::helper", 0), None);
}

fn assert_foreign_caller_sidecar(
    tokens: &crate::ir::CommandTokens,
    command: &tcl_lexer::NativeScriptCommandWords,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    segment: &crate::segmenter::SegmentedCommand,
) {
    let mut foreign = tokens.clone();
    let offset = command.span.start();
    let unrelated = tokens.source_binding.clone().unwrap();
    foreign.nested_bindings.push((offset, unrelated));
    let mut withdrawn = CallSiteEvidence::default();
    record_call_site_evidence(
        &mut withdrawn,
        ctx,
        caller,
        segment.name(),
        segment.args(),
        Some(&foreign),
        0,
    );
    assert!(withdrawn.get("::helper").unwrap().opaque_caller);
    assert_eq!(uniform(&withdrawn, "::helper", 0), None);
}

#[test]
fn original_body_caller_inventory_requires_each_original_child_sidecar() {
    // naming.source.original-script-region-purpose
    // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
    // Whole authored body syntax is separate from dispatch and Native frame entry.
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let source = "proc helper {mode} {return $mode}\nhelper FIXED\ncatch {helper DIFFERENT}";
    let module = original_caller_fixture(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let cfg = crate::cfg_builder::build_cfg(&module, false);
    let input = module.source_metadata_input.as_ref().unwrap();
    let registry = module.resolved_registry();
    let realm = crate::realm::document_realm_bindings_with_source_entry(
        source,
        module.lexer_config,
        registry,
        &module.source_entry,
    )
    .with_resolved_analysis_input(input.clone());
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, module.lexer_config)
            .pop()
            .unwrap();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        module.lexer_config,
        &segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let words =
        crate::registry_invocation::source_structure::original_registry_words_for_supplied_tokens(
            &realm, input, &tokens,
        )
        .unwrap();
    let context = input.context_registry();
    let bodies = words
        .source_script_bodies_for_mutation_coverage(&context)
        .unwrap();
    assert_eq!(bodies.len(), 1);
    let commands = bodies[0].original_commands(&context).unwrap();
    assert_eq!(commands.len(), 1);
    assert!(commands[0].span.start() > tokens.argv[0].start());
    let command = &commands[0];
    assert_eq!(command.words[0].bytes(), b"helper");
    let previous = CallSiteEvidence::default();
    let facts = ModuleVarFacts::default();
    let known = HashSet::from(["::helper".to_owned()]);
    let reach = vec!["::helper".to_owned()];
    let ctx = CallSiteScanCtx {
        known: &known,
        registry,
        module: &module,
        source_input: Some(input),
        identities: &realm,
        declared: None,
        var_facts: &facts,
        previous: &previous,
        procedures: &module.procedures,
        unenumerable_reach: &reach,
        unresolved_handler: None,
    };
    let caller = CallerFrame {
        metadata_owner: &cfg.top_level.metadata_context,
        scope: CallerScopeId::TopLevel,
        params: &[],
        callers_tracked: true,
    };
    let mut current = CallSiteEvidence::default();
    record_call_site_evidence(
        &mut current,
        &ctx,
        &caller,
        segment.name(),
        segment.args(),
        Some(&tokens),
        0,
    );
    assert_eq!(
        uniform(&current, "::helper", 0).as_deref(),
        Some("DIFFERENT")
    );
    let mut missing = tokens.clone();
    missing.nested_bindings.clear();
    let mut withdrawn = CallSiteEvidence::default();
    record_call_site_evidence(
        &mut withdrawn,
        &ctx,
        &caller,
        segment.name(),
        segment.args(),
        Some(&missing),
        0,
    );
    assert!(withdrawn.get("::helper").unwrap().opaque_caller);
    assert_eq!(uniform(&withdrawn, "::helper", 0), None);
    assert_foreign_caller_sidecar(&tokens, command, &ctx, &caller, &segment);
}

fn native_argument_caller_unit(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::environment_ingress::RetainedNativeUnit,
) {
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl9.0").unwrap();
    let context = environment.default_context_registry();
    let profile = environment.unit_profile();
    let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_profile(Some(profile)),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&context),
    );
    (
        context,
        crate::environment_ingress::RetainedNativeUnit::new(unit, owner),
    )
}

#[test]
fn original_argument_caller_phase_keeps_real_return_and_direct_argument_counts() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The actual catch region captures this Return as an argv phase.
    let (_, unit) = native_argument_caller_unit(
        "proc add {b c} {catch {return [expr {$b + $c}]}}; add 2 4; add 3 5",
    );
    let function = unit
        .cfg_module
        .procedures
        .get(&fixture_id(&unit.cfg_module.procedures, "::add"))
        .expect("authored fixture body");
    assert!(
        function.blocks.iter().any(|(&block, data)| {
            data.statements.iter().enumerate().any(|(index, _)| {
                function
                    .source_tokens_at(block, index)
                    .is_some_and(|tokens| {
                        tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments)
                            && tokens
                                .argv_texts
                                .first()
                                .is_some_and(|head| head == "return")
                    })
            })
        }),
        "actual return producer retains its argv boundary"
    );
    let evidence = unit.caller_scope.call_sites.get("::add").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([2]));
    assert_eq!(
        evidence.slots[&0].values,
        BTreeSet::from(["2".into(), "3".into()])
    );
    assert_eq!(
        evidence.slots[&1].values,
        BTreeSet::from(["4".into(), "5".into()])
    );
}

#[test]
fn original_argument_caller_child_keeps_its_own_expression_roles() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (_, unit) = native_argument_caller_unit(
        "proc helper {x} {return $x}; proc run {} {return [expr {[helper SECOND]}]}; helper FIRST; run",
    );
    let evidence = unit.caller_scope.call_sites.get("::helper").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([1]));
    assert_eq!(
        evidence.slots[&0].values,
        BTreeSet::from(["FIRST".into(), "SECOND".into()])
    );
    assert_eq!(evidence.uniform_literal_at(0), None);
    // The enclosing braced argument never inherits the child's Expr role.
    let (_, literal) = native_argument_caller_unit(
        "proc helper {x} {return $x}; proc run {} {return {[helper LITERAL]}}; helper FIRST; run",
    );
    let evidence = literal.caller_scope.call_sites.get("::helper").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.slots[&0].values, BTreeSet::from(["FIRST".into()]));
}

fn assert_original_argv_metadata_has_separate_purpose(
    phase: &crate::ir::CommandTokens,
    owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    module: &IrModule,
    registry: &CommandRegistry,
) {
    let binding = phase.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_invocation_metadata_for_owner(phase, owner, registry)
            .is_err()
    );
    assert!(
        binding
            .original_argument_metadata_for_owner(phase, owner, registry)
            .unwrap()
            .is_some()
    );
    assert!(
        binding
            .original_argument_metadata_for_module(phase, module, registry)
            .is_some()
    );
}

fn assert_original_argv_child_refusals(
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    phase: &crate::ir::CommandTokens,
    owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
) {
    use crate::registry_invocation::OwnedInvocationMetadataContext as Owner;
    fn caller(
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    ) -> CallerFrame<'_> {
        CallerFrame {
            metadata_owner: owner,
            scope: CallerScopeId::Extra(0),
            params: &[],
            callers_tracked: true,
        }
    }
    let mut missing_child = (*phase).clone();
    missing_child.nested_bindings.clear();
    let mut changed_vector = (*phase).clone();
    changed_vector.argv_texts[1] = "changed".into();
    for refused_tokens in [&missing_child, &changed_vector] {
        let mut refused = CallSiteEvidence::default();
        record_original_lifted_call_sites(&mut refused, ctx, &caller(owner), Some(refused_tokens));
        assert!(refused.get("::add").unwrap().opaque_caller);
    }
    // A typed residual producer explicitly performs no argv evaluation.
    let residual = crate::ir::CommandTokens::marker(crate::ir::SyntheticMarker::EvaluatedWrapper);
    let mut empty = CallSiteEvidence::default();
    record_original_lifted_call_sites(
        &mut empty,
        ctx,
        &caller(&Owner::Unavailable),
        Some(&residual),
    );
    assert!(empty.is_empty());
}

#[test]
fn original_argument_caller_owner_refusal_stays_distinct_from_effect_only_empty() {
    fn caller(
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    ) -> CallerFrame<'_> {
        CallerFrame {
            metadata_owner: owner,
            scope: CallerScopeId::Extra(0),
            params: &[],
            callers_tracked: true,
        }
    }
    use crate::registry_invocation::OwnedInvocationMetadataContext as Owner;
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The actual catch region captures this Return as an argv phase.
    let source = "proc add {b c} {catch {return [expr {$b + $c}]}}; add 2 4; add 3 5";
    let (context, unit) = native_argument_caller_unit(source);
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref().unwrap();
    let function = unit
        .cfg_module
        .procedures
        .get(&fixture_id(&unit.cfg_module.procedures, "::add"))
        .expect("authored fixture body");
    let phase = function
        .blocks
        .iter()
        .find_map(|(&block, data)| {
            data.statements.iter().enumerate().find_map(|(index, _)| {
                function.source_tokens_at(block, index).filter(|tokens| {
                    tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments)
                        && tokens
                            .argv_texts
                            .first()
                            .is_some_and(|head| head == "return")
                })
            })
        })
        .expect("actual return argv boundary");
    assert_original_argv_metadata_has_separate_purpose(
        phase,
        &function.metadata_context,
        module,
        context.commands(),
    );
    let identities =
        crate::realm::document_realm_bindings_for_module(module, context.commands()).unwrap();
    let known = HashSet::from(["::add".to_owned()]);
    let reach = vec!["::add".to_owned()];
    let facts = ModuleVarFacts::default();
    let previous = CallSiteEvidence::default();
    let ctx = CallSiteScanCtx {
        known: &known,
        registry: context.commands(),
        module,
        source_input: Some(input),
        identities: &identities,
        declared: None,
        var_facts: &facts,
        previous: &previous,
        procedures: &module.procedures,
        unenumerable_reach: &reach,
        unresolved_handler: None,
    };
    let mut current = CallSiteEvidence::default();
    record_original_lifted_call_sites(
        &mut current,
        &ctx,
        &caller(&function.metadata_context),
        Some(phase),
    );
    assert!(
        current.is_empty(),
        "the actual expr child calls no local procedure"
    );
    let mut changed_config = input.lexer_config();
    changed_config.leading_bom = tcl_lexer::LeadingBom::Skip;
    let changed = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        changed_config,
    );
    let foreign = supplied_unit_input(
        input,
        tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
            .unwrap()
            .default_context_registry(),
    );
    for owner in [
        Owner::Unavailable,
        Owner::Standalone,
        Owner::SuppliedSource(Box::new(changed)),
        Owner::SuppliedSource(Box::new(foreign)),
    ] {
        let mut refused = CallSiteEvidence::default();
        record_original_lifted_call_sites(&mut refused, &ctx, &caller(&owner), Some(phase));
        assert!(refused.get("::add").unwrap().opaque_caller);
        assert!(refused.get("::add").unwrap().arg_counts.contains(&0));
    }
    assert_original_argv_child_refusals(&ctx, phase, &function.metadata_context);
}

#[test]
fn original_argument_caller_metadata_joins_from_the_module_to_its_nested_owner() {
    use crate::registry_invocation::OwnedInvocationMetadataContext as Owner;
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual retained Native source and CFG argv phase; no handler entry or
    // runtime variable result follows from this metadata-only observation.
    let (context, unit) =
        native_argument_caller_unit("proc add {b c} {catch {return [expr {$b + $c}]}}; add 2 4");
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref().unwrap();
    let function = unit
        .cfg_module
        .procedures
        .get(&fixture_id(&unit.cfg_module.procedures, "::add"))
        .expect("authored fixture body");
    let phase = function
        .blocks
        .iter()
        .find_map(|(&block, data)| {
            data.statements.iter().enumerate().find_map(|(index, _)| {
                function.source_tokens_at(block, index).filter(|tokens| {
                    tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments)
                        && tokens
                            .argv_texts
                            .first()
                            .is_some_and(|head| head == "return")
                })
            })
        })
        .expect("actual return argv boundary");
    let identities =
        crate::realm::document_realm_bindings_for_module(module, context.commands()).unwrap();
    let known = HashSet::from(["::add".to_owned()]);
    let reach = vec!["::add".to_owned()];
    let facts = ModuleVarFacts::default();
    let previous = CallSiteEvidence::default();
    let ctx = CallSiteScanCtx {
        known: &known,
        registry: context.commands(),
        module,
        source_input: Some(input),
        identities: &identities,
        declared: None,
        var_facts: &facts,
        previous: &previous,
        procedures: &module.procedures,
        unenumerable_reach: &reach,
        unresolved_handler: None,
    };
    for owner in [
        Owner::for_source_input(Some(input)),
        Owner::for_source_input(Some(&input.for_nested_source())),
        function.metadata_context.clone(),
    ] {
        assert!(
            ctx.original_argument_metadata(phase, &owner)
                .unwrap()
                .is_some()
        );
    }
    let mut changed_config = input.lexer_config();
    changed_config.leading_bom = tcl_lexer::LeadingBom::Skip;
    let changed_file_input = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        changed_config,
    );
    assert_ne!(changed_file_input, *input);
    assert_eq!(
        changed_file_input.for_nested_source(),
        input.for_nested_source()
    );
    let foreign = supplied_unit_input(
        input,
        tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
            .unwrap()
            .default_context_registry(),
    );
    for owner in [
        Owner::SuppliedSource(Box::new(changed_file_input)),
        Owner::SuppliedSource(Box::new(foreign)),
        Owner::Unavailable,
        Owner::Standalone,
    ] {
        assert!(ctx.original_argument_metadata(phase, &owner).is_err());
        // Invocation purpose remains independently refused for an argv phase.
        assert!(ctx.original_metadata(phase, &owner).is_err());
    }
}

#[test]
fn original_native_procedure_argv_roles_do_not_invent_registry_body_callers() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual held Vm entry and source call inventory, not a successful native call.
    let (_, unit) = native_argument_caller_unit(
        "proc selected {value} {}; selected {[inert CHILD]}; selected {[inert CHILD]}",
    );
    let evidence = unit.caller_scope.call_sites.get("::selected").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([1]));
    assert_eq!(evidence.uniform_literal_at(0), Some("[inert CHILD]"));
    let (_, unavailable) =
        native_argument_caller_unit("proc selected {value} {}; unknown; selected FIXED");
    assert!(
        unavailable
            .caller_scope
            .call_sites
            .get("::selected")
            .unwrap()
            .opaque_caller
    );
}

#[test]
fn original_native_procedure_argv_roles_keep_authentic_alias_prefix_counts() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (_, unit) = native_argument_caller_unit(
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded FIXED; selected PREFIX FIXED",
    );
    let evidence = unit.caller_scope.call_sites.get("::selected").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([2]));
    assert_eq!(evidence.uniform_literal_at(0), Some("PREFIX"));
    assert_eq!(evidence.uniform_literal_at(1), Some("FIXED"));
}

fn alias_creation_tokens(module: &IrModule) -> crate::ir::CommandTokens {
    module
        .top_level
        .statements
        .iter()
        .find_map(|statement| {
            statement.tokens().filter(|tokens| {
                tokens
                    .argv_texts
                    .first()
                    .is_some_and(|head| head == "interp")
            })
        })
        .expect("actual original alias declaration")
        .clone()
}

fn alias_binding_transition(
    tokens: &crate::ir::CommandTokens,
    module: &IrModule,
    registry: &CommandRegistry,
) -> tcl_registry::CommandBindingTransition {
    let binding = tokens.source_binding.as_ref().unwrap();
    let metadata = binding
        .original_invocation_metadata_for_module(tokens, module, registry)
        .unwrap();
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry,
        Some(metadata),
        tokens,
    )
    .unwrap();
    invocation
        .facts
        .state_transitions
        .declared()
        .unwrap()
        .command_bindings()
        .find(|transition| {
            matches!(
                transition,
                tcl_registry::CommandBindingTransition::Alias { .. }
            )
        })
        .unwrap()
        .clone()
}

#[test]
fn original_covered_alias_effect_keeps_actual_publication_and_prefix_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The held Vm supplies actual Native source entry, not successful alias execution.
    let (context, unit) = native_argument_caller_unit(
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
    );
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref().unwrap();
    let tokens = alias_creation_tokens(module);
    let transition = alias_binding_transition(&tokens, module, context.commands());
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_covered_alias_transition(
                &tokens,
                module,
                input,
                context.commands(),
                &transition,
            )
            .is_some()
    );
    let evidence = unit.caller_scope.call_sites.get("::selected").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([2]));
    assert_eq!(evidence.uniform_literal_at(0), Some("PREFIX"));
    assert_eq!(evidence.uniform_literal_at(1), Some("FIXED"));
    let tcl_registry::CommandBindingTransition::Alias { arguments, .. } = &transition else {
        unreachable!()
    };
    let mut changed = transition.clone();
    let tcl_registry::CommandBindingTransition::Alias {
        arguments: changed_arguments,
        ..
    } = &mut changed
    else {
        unreachable!()
    };
    assert_ne!(arguments.len(), 0);
    changed_arguments.clear();
    assert!(
            binding
                .original_covered_alias_transition(
                    &tokens,
                    module,
                    input,
                    context.commands(),
                    &changed,
                )
                .is_none()
        );
}

#[test]
fn original_covered_alias_effect_refuses_missing_foreign_source_and_vector_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (context, unit) = native_argument_caller_unit(
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
    );
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref().unwrap();
    let tokens = alias_creation_tokens(module);
    let transition = alias_binding_transition(&tokens, module, context.commands());
    let binding = tokens.source_binding.as_ref().unwrap();
    let mut missing = module.clone();
    missing.retained_source_bindings = None;
    let mut absent = module.clone();
    absent.source_metadata_input = None;
    let mut foreign_source = module.clone();
    foreign_source.source = tcl_lexer::SourceImage::document("proc foreign {} {}");
    let mut config = input.lexer_config();
    config.leading_bom = tcl_lexer::LeadingBom::Skip;
    let changed_input = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    for candidate in [&missing, &absent, &foreign_source] {
        assert!(
            binding
                .original_covered_alias_transition(
                    &tokens,
                    candidate,
                    input,
                    context.commands(),
                    &transition,
                )
                .is_none()
        );
    }
    assert!(
        binding
            .original_covered_alias_transition(
                &tokens,
                module,
                &changed_input,
                context.commands(),
                &transition,
            )
            .is_none()
    );
    let foreign = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
    );
    assert!(
        binding
            .original_covered_alias_transition(
                &tokens,
                module,
                input,
                foreign.commands(),
                &transition,
            )
            .is_none()
    );
    let mut altered = tokens.clone();
    altered.argv_texts.last_mut().unwrap().push_str("foreign");
    assert!(
        binding
            .original_covered_alias_transition(
                &altered,
                module,
                input,
                context.commands(),
                &transition,
            )
            .is_none()
    );
    assert!(
        binding
            .original_covered_alias_transition(
                &tokens,
                module,
                input,
                context.commands(),
                &transition,
            )
            .is_some()
    );
}

#[test]
fn original_covered_alias_effect_keeps_rebinding_unknown_and_callback_opacity() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    for source in [
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; rename forwarded moved; moved FIXED",
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; unknown; forwarded FIXED",
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; after 0 forwarded; forwarded FIXED",
        "proc selected {prefix value} {}; interp alias {} forwarded FOREIGN selected PREFIX; forwarded FIXED",
    ] {
        let (_, unit) = native_argument_caller_unit(source);
        assert!(
            unit.caller_scope
                .call_sites
                .get("::selected")
                .unwrap()
                .opaque_caller,
            "{source}"
        );
    }
}

#[test]
fn original_covered_alias_effect_requires_original_entered_continuation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (context, current) = native_argument_caller_unit(
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
    );
    let tokens = alias_creation_tokens(&current.ir_module);
    let transition = alias_binding_transition(&tokens, &current.ir_module, context.commands());
    for source in [
        "proc selected {prefix value} {}; return DONE; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
        "proc selected {prefix value} {}; unknown; interp alias {} forwarded {} selected PREFIX; forwarded FIXED",
        "proc selected {prefix value} {}; interp alias {} forwarded FOREIGN selected PREFIX; forwarded FIXED",
    ] {
        let (context, unit) = native_argument_caller_unit(source);
        let module = &unit.ir_module;
        let tokens = alias_creation_tokens(module);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_covered_alias_transition(
                    &tokens,
                    module,
                    module.source_metadata_input.as_ref().unwrap(),
                    context.commands(),
                    &transition,
                )
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_direct_return_keeps_terminator_argv_without_captured_phase_marker() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // A procedure exit retains its original argv as a terminator. Captured
    // completions use the distinct EvaluatedArguments statement purpose.
    let (_, unit) =
        native_argument_caller_unit("proc add {b c} {return [expr {$b + $c}]}; add 2 4; add 3 5");
    let function = unit
        .cfg_module
        .procedures
        .get(&fixture_id(&unit.cfg_module.procedures, "::add"))
        .expect("authored fixture body");
    let returns = function
        .blocks
        .iter()
        .filter_map(|(&block, data)| {
            matches!(data.terminator, Some(crate::cfg::Terminator::Return { .. }))
                .then(|| function.source_tokens_at(block, usize::MAX))
                .flatten()
                .filter(|tokens| {
                    tokens
                        .argv_texts
                        .first()
                        .is_some_and(|head| head == "return")
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(returns.len(), 1);
    assert!(returns[0].synthetic.is_none());
    assert_eq!(returns[0].argv_texts, ["return", "[expr {$b + $c}]"]);
    assert!(
        function
            .blocks
            .values()
            .all(|data| data.statements.iter().all(|statement| {
                statement.tokens().is_none_or(|tokens| {
                    tokens.synthetic != Some(crate::ir::SyntheticMarker::EvaluatedArguments)
                        || tokens
                            .argv_texts
                            .first()
                            .is_none_or(|head| head != "return")
                })
            }))
    );
    let evidence = unit.caller_scope.call_sites.get("::add").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([2]));
    assert_eq!(
        evidence.slots[&0].values,
        BTreeSet::from(["2".into(), "3".into()])
    );
    assert_eq!(
        evidence.slots[&1].values,
        BTreeSet::from(["4".into(), "5".into()])
    );
}
