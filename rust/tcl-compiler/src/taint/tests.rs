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

mod callback_replay;
mod command_heads;
mod definition_taint;
mod destructive_files;
mod irules_sinks;
mod lattice;
mod numeric_coercion;
mod option_injection;
mod propagation;
mod reports;
mod sanitisers;
mod setter_constraints;
mod sink_classification;
mod sinks;
mod source_reads;
mod word_taint;

use super::*;
use crate::sccp::SccpResult;
use tcl_dialect::model::{Family, SurfaceLayer};

fn retained_warning_unit(
    source: &str,
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> (tcl_vm::Vm, crate::compilation_unit::CompilationUnit) {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
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
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(context),
    );
    (owner, unit)
}

fn simple_sccp(blocks: &[BlockId]) -> SccpResult {
    SccpResult {
        required_math_invocations: Vec::new(),
        required_expression_preparations: Vec::new(),
        explanations: Vec::new(),
        route_tally: crate::value_transfer::RouteTally::default(),
        folded_types: HashMap::new(),
        preserved: HashMap::new(),
        raised: HashSet::default(),
        template_plans: Vec::new(),
        selections: Vec::new(),
        existence: HashMap::new(),
        existence_reads: HashMap::new(),
        existence_exits: HashMap::new(),
        existence_entries: HashMap::new(),
        refinements: Vec::new(),
        refinements_at: HashMap::new(),
        value_entries: HashMap::new(),
        query_places: Vec::new(),
        existence_guards: Vec::new(),
        loop_enumerations: Vec::new(),
        reads_module: false,
        completion: crate::sccp::RunCompletion::default(),
        values: HashMap::new(),
        executable_blocks: blocks.iter().copied().collect(),
        executable_edges: HashSet::new(),
        constant_branches: Vec::new(),
    }
}

/// Bare intra-procedural propagation with no optional inputs.
fn plain_taints(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    registry: &CommandRegistry,
) -> HashMap<ValueKey, TaintLattice> {
    let instance_classes = local_instance_classes(cfg, registry);
    propagate_taints(
        &TaintGraph::new(cfg, ssa, sccp),
        TaintPropagationInputs {
            registry,
            rendered_props: None,
            interproc: None,
            source: TaintSourceContext::standalone(registry, None),
            param_taints: None,
            taint_summaries: None,
            instance_classes: &instance_classes,
        },
    )
}

/// Wire `set x [gets stdin]` (a taint source) followed by `sink`
/// (which uses `$x`) and return the taint warnings.  Shared by the
/// T104 / T105 sink tests.
fn warnings_for_tainted_sink(sink: Statement, sink_uses: &[(&str, u32)]) -> Vec<TaintWarning> {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let registry = CommandRegistry::build_default();
    let assign = Statement::AssignValue {
        span: Span::new(0, 12),
        name: "x".into(),
        name_braced: false,
        value: "[gets stdin]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    {
        let b = cfg.blocks.get_mut(&entry).unwrap();
        b.statements.push(assign.clone());
        b.statements.push(sink.clone());
        b.terminator = Some(Terminator::Return {
            expr_base: None,
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
    }
    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let ssa_assign = ssa_stmt(
        assign,
        HashMap::new(),
        [(ssa.intern_var("x"), 1u32)].into_iter().collect(),
    );
    let ssa_sink = ssa_stmt(
        sink,
        sink_uses
            .iter()
            .map(|&(n, v)| (ssa.intern_var(n), v))
            .collect(),
        HashMap::new(),
    );
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_assign, ssa_sink],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    find_taint_warnings(
        &cfg,
        &ssa,
        &taints,
        &sccp.executable_blocks,
        &registry,
        None,
        &HashSet::new(),
    )
}

fn call_stmt(command: &str, args: &[&str]) -> Statement {
    Statement::Call {
        span: tcl_lexer::Span::new(13, 30),
        command: command.into(),
        canonical_command: None,
        args: args.iter().map(|a| (*a).to_string()).collect(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

/// A hand-built SSA statement with no may-defs, quoted uses or name-only
/// uses — the shape every fixture in this module wants.
fn ssa_stmt(
    statement: Statement,
    uses: HashMap<crate::ssa::Symbol, u32>,
    defs: HashMap<crate::ssa::Symbol, u32>,
) -> crate::ssa::SsaStatement {
    crate::ssa::SsaStatement {
        statement,
        uses,
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

/// Shared substitution inventories preserve name and lexical boundaries.
mod arg_var_names_vs_vars_in_word {
    use super::*;

    /// TP: a bare `$name` reference — both scanners agree.
    #[test]
    fn agree_on_bare_var() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            arg_var_names("$p", tcl_dialect::BracedVarStyle::default()),
            HashSet::from(["p".to_owned()])
        );
        assert_eq!(
            crate::var_refs::vars_in_word("$p", &registry)
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from(["p".to_owned()])
        );
    }

    /// TP: `${braced}` next to a bare `$var` in the same word — both
    /// scanners agree, including across a non-variable separator (`/`).
    #[test]
    fn agree_on_braced_and_bare_in_one_word() {
        let registry = CommandRegistry::build_default();
        let want = HashSet::from(["dir".to_owned(), "file".to_owned()]);
        assert_eq!(
            arg_var_names("${dir}/$file", tcl_dialect::BracedVarStyle::default()),
            want
        );
        assert_eq!(
            crate::var_refs::vars_in_word("${dir}/$file", &registry)
                .into_iter()
                .collect::<HashSet<_>>(),
            want
        );
    }

    /// TP (command substitution): a `$var` nested inside `[cmd …]` —
    /// both scanners recurse into the substitution and agree.
    #[test]
    fn agree_on_var_inside_command_substitution() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            arg_var_names("[foo $x]", tcl_dialect::BracedVarStyle::default()),
            HashSet::from(["x".to_owned()])
        );
        assert_eq!(
            crate::var_refs::vars_in_word("[foo $x]", &registry)
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from(["x".to_owned()])
        );
    }

    /// The array root and its substituted index are separate dependencies.
    #[test]
    fn agree_on_var_nested_in_array_index() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            arg_var_names("$arr($cmd)", tcl_dialect::BracedVarStyle::default()),
            HashSet::from(["arr".to_owned(), "cmd".to_owned()]),
            "arg_var_names must keep catching the nested index variable"
        );
        assert_eq!(
            crate::var_refs::vars_in_word("$arr($cmd)", &registry)
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from(["arr".to_owned(), "cmd".to_owned()]),
            "the shared inventory includes the substituted index dependency"
        );
    }

    /// FP guard: a backslash-escaped `\$name` is literal text (Tcl
    /// backslash substitution turns `\$` into a literal `$`, so no
    /// variable is referenced). A byte-scan that does not know about
    /// backslash escaping reads it as a reference to `notavar`;
    /// `arg_var_names` must agree with `vars_in_word`, which goes through
    /// the real lexer.
    #[test]
    fn escaped_dollar_is_not_a_reference() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            arg_var_names("\\$notavar", tcl_dialect::BracedVarStyle::default()).len(),
            0,
            "an escaped \\$ must not be read as a variable reference"
        );
        assert_eq!(
            crate::var_refs::vars_in_word("\\$notavar", &registry).len(),
            0
        );
    }

    /// FP fixed, doubled escape: `\\$foo` is an escaped backslash
    /// (literal `\`) followed by a live, unescaped `$foo` — the
    /// variable reference still must be found.
    #[test]
    fn double_backslash_then_live_var_is_still_found() {
        assert_eq!(
            arg_var_names("\\\\$foo", tcl_dialect::BracedVarStyle::default()),
            HashSet::from(["foo".to_owned()])
        );
    }

    /// TN: plain literal text with no `$` sigil at all.
    #[test]
    fn no_dollar_sigil_is_empty() {
        assert_eq!(
            arg_var_names("hello world", tcl_dialect::BracedVarStyle::default()).len(),
            0
        );
    }
}

/// `interp alias` / `rename` command-name resolution feeding sink
/// classification. Sink checks key off
/// `Statement::canonical_command_or_source()`, so a call through a
/// renamed or aliased name resolves to whatever it now denotes.
mod rename_and_alias_taint {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn taint_warnings_for(source: &str) -> Vec<TaintWarning> {
        let registry = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        find_taint_warnings_for_cu(&cu, &registry, None)
    }

    /// TP (regression guard): tainted data reaching the real `puts` sink
    /// through an `interp alias` name. This was silently missed before
    /// sink classification started reading `canonical_command` — the
    /// alias table was populated but never consulted here.
    #[test]
    fn tainted_data_through_interp_alias_is_flagged() {
        let w = taint_warnings_for("set u [gets stdin]\ninterp alias {} myputs {} puts\nmyputs $u");
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 through the `myputs` alias, got {w:?}"
        );
    }

    /// TP: tainted data reaching the real `puts` sink through a renamed
    /// command name (`rename puts myputs`) — the scenario this
    /// investigation was specifically scoped to cover.
    #[test]
    fn tainted_data_through_renamed_sink_is_flagged() {
        let w = taint_warnings_for("set u [gets stdin]\nrename puts myputs\nmyputs $u");
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 through the renamed `myputs`, got {w:?}"
        );
    }

    /// TN: `rename someSafeProc puts` shadows the builtin — the bare
    /// name `puts` now denotes the user's own proc, not the real
    /// output sink, so a subsequent `puts $tainted` must NOT be flagged.
    /// Guards the false-positive direction of the same fix (a renamed
    /// binding can point *away* from a builtin just as easily as onto
    /// one).
    #[test]
    fn rename_shadowing_a_builtin_suppresses_the_sink() {
        let w = taint_warnings_for(
            "proc mysafeputs {x} { return }\n\
                 set u [gets stdin]\n\
                 rename mysafeputs puts\n\
                 puts $u",
        );
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "puts is shadowed by a user proc after the rename, got {w:?}"
        );
    }

    /// A substituted word whose value is known names the missing command;
    /// its failed rename stops this script before the later sink.
    #[test]
    fn known_missing_rename_stops_before_the_builtin_sink() {
        let w =
            taint_warnings_for("set old somecmd\nset u [gets stdin]\nrename $old puts\nputs $u");
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "a failed rename must not invent a reached puts sink, got {w:?}"
        );
    }

    /// TN control: an *unrelated* rename must not disturb ordinary
    /// `puts` sink detection.
    #[test]
    fn unrelated_rename_does_not_affect_puts_sink() {
        let w = taint_warnings_for("rename string mystr\nset u [gets stdin]\nputs $u");
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 for a plain puts call, got {w:?}"
        );
    }

    /// TN: the deletion form `rename puts {}` creates no new binding —
    /// nothing to alias — so it must not be mistaken for a rename that
    /// resolves some other name to `puts`.
    #[test]
    fn rename_deletion_form_creates_no_alias() {
        let w = taint_warnings_for("set u [gets stdin]\nrename puts {}\nputs $u");
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "a deleted command cannot regain stock puts metadata: {w:?}"
        );
    }
}

/// Unit tests for the pure [`shadowed_builtin_names`] helper, isolated
/// from the end-to-end taint-warning plumbing tested by
/// `namespace_shadowed_builtin_taint` below.
mod shadowed_builtin_names_tests {
    use super::*;

    #[test]
    fn retained_body_scope_ignores_display_and_refuses_detached_units() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let mut unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            "namespace eval ns {proc puts {args} {}; proc run {value} {puts $value}}",
            &registry,
            false,
            profile,
        );
        let id =
            crate::interprocedural::test_body_identity::fixture_id(&unit.procedures, "::ns::run");
        let function = unit.procedures.get_mut(&id).unwrap();
        function.name = "::counterfactual::run".to_owned();
        assert_eq!(
            shadowed_builtin_names_for_module_function(
                unit.procedures.get(&id).unwrap(),
                &unit,
                &registry
            ),
            Some(HashSet::from(["puts".to_owned()]))
        );
        let detached_top = unit.top_level.clone();
        assert!(
            shadowed_builtin_names_for_module_function(&detached_top, &unit, &registry).is_none()
        );
        let detached_body = unit.procedures.get(&id).unwrap().clone();
        assert!(
            shadowed_builtin_names_for_module_function(&detached_body, &unit, &registry).is_none()
        );
        let foreign_unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            "proc foreign {} {}",
            &registry,
            false,
            profile,
        );
        let foreign = crate::interprocedural::test_body_identity::fixture_id(
            &foreign_unit.procedures,
            "::foreign",
        );
        unit.procedures.get_mut(&id).unwrap().body_id = Some(foreign);
        assert!(
            shadowed_builtin_names_for_module_function(
                unit.procedures.get(&id).unwrap(),
                &unit,
                &registry
            )
            .is_none()
        );
    }

    #[test]
    fn namespace_scoped_proc_shadows_only_its_own_namespace() {
        let registry = CommandRegistry::build_default();
        let procs = ["::myns::puts", "::myns::helper"];
        let shadowed = shadowed_builtin_names("::myns", procs.iter().copied(), &registry);
        assert_eq!(shadowed, HashSet::from(["puts".to_owned()]));
        // A different namespace sees no shadow at all.
        assert_eq!(
            shadowed_builtin_names("::other", procs.iter().copied(), &registry).len(),
            0
        );
    }

    #[test]
    fn global_proc_shadows_every_namespace() {
        let registry = CommandRegistry::build_default();
        let procs = ["::puts"];
        for ns in ["::", "::myns", "::a::b"] {
            assert_eq!(
                shadowed_builtin_names(ns, procs.iter().copied(), &registry),
                HashSet::from(["puts".to_owned()]),
                "namespace {ns} must see the global shadow"
            );
        }
    }

    #[test]
    fn nested_child_proc_does_not_shadow_its_parent() {
        let registry = CommandRegistry::build_default();
        let procs = ["::a::b::puts"];
        assert_eq!(
            shadowed_builtin_names("::a", procs.iter().copied(), &registry).len(),
            0
        );
    }

    #[test]
    fn non_registry_proc_name_is_not_shadowing() {
        // A proc named after something the registry doesn't know is not a
        // "shadow" in this sense — there's no builtin sink to suppress.
        let registry = CommandRegistry::build_default();
        let procs = ["::myns::totallyMadeUpName"];
        assert_eq!(
            shadowed_builtin_names("::myns", procs.iter().copied(), &registry).len(),
            0
        );
    }
}

/// A user proc shadowing a builtin's bare name: `proc ::myns::puts {args}
/// {…}` (or a plain top-level `proc puts {…}`) makes a bare `puts` call
/// from the shadowing scope dispatch to the user's own proc, not the
/// output-sink builtin.
mod namespace_shadowed_builtin_taint {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn taint_warnings_for(source: &str) -> Vec<TaintWarning> {
        // These fixtures mean C Tcl 8.6: its own profile, not the lenient
        // all-release `tcl` profile, beside its 8.6 command context.
        let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let context = environment.default_context_registry();
        let profile = environment.analyser_profile();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            environment.unit_profile(),
            std::sync::Arc::clone(&context),
            config,
        );
        let registry = context.commands();
        let cu = CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
        .with_interprocedural(registry, Some(profile));
        find_taint_warnings_for_cu(&cu, registry, Some(profile))
    }

    /// FP fixed: a namespace-scoped proc of the same name as a builtin
    /// must suppress the builtin sink classification for bare calls made
    /// from that namespace — this was the false positive the
    /// investigation's example (`proc ::myns::puts`) called out.
    #[test]
    fn namespace_scoped_proc_shadowing_builtin_suppresses_sink() {
        let w = taint_warnings_for(
            "namespace eval ::myns {\n\
                 \x20 proc puts {x} { return }\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
        );
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "puts is shadowed by ::myns::puts, must not be a sink, got {w:?}"
        );
    }

    /// FP fixed: the same shadowing effect for a plain top-level proc
    /// redefinition (no namespace involved at all).
    #[test]
    fn top_level_proc_shadowing_builtin_suppresses_sink() {
        let w = taint_warnings_for("proc puts {x} { return }\nset u [gets stdin]\nputs $u");
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "puts is shadowed by a top-level proc, got {w:?}"
        );
    }

    /// FP fixed: a *global*-level shadowing proc suppresses the sink for
    /// bare calls made from *any* namespace, not just the global one —
    /// Tcl's unqualified-name lookup always falls back to global.
    #[test]
    fn global_proc_shadows_calls_from_other_namespaces_too() {
        let w = taint_warnings_for(
            "proc puts {x} { return }\n\
                 namespace eval ::myns {\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
        );
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "the global puts proc must shadow calls from ::myns too, got {w:?}"
        );
    }

    /// TN control: an unshadowed bare `puts` inside a namespace is still
    /// the real builtin sink.
    #[test]
    fn unshadowed_namespace_call_is_still_a_sink() {
        let w = taint_warnings_for(
            "namespace eval ::myns {\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 for an unshadowed puts call, got {w:?}"
        );
    }

    /// TN control: a shadowing proc in an unrelated namespace must not
    /// suppress a sink in a *different* namespace.
    #[test]
    fn shadow_in_unrelated_namespace_does_not_suppress_sink() {
        let w = taint_warnings_for(
            "namespace eval ::other {\n\
                 \x20 proc puts {x} { return }\n\
                 }\n\
                 namespace eval ::myns {\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "::other's shadow must not affect ::myns, got {w:?}"
        );
    }

    /// TN control: an explicit fully-qualified `::puts` call bypasses
    /// namespace resolution entirely, so it must still be flagged even
    /// when the bare name is locally shadowed.
    #[test]
    fn fully_qualified_call_bypasses_the_shadow() {
        let w = taint_warnings_for(
            "namespace eval ::myns {\n\
                 \x20 proc puts {x} { return }\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   ::puts $u\n\
                 \x20 }\n\
                 }",
        );
        assert!(
            w.iter()
                .any(|d| matches!(d.code, DiagCode::T100 | DiagCode::T101)),
            "an explicit ::puts must still be classified as a sink, got {w:?}"
        );
    }
}

/// `trace add variable` / `trace variable` conservatively forcing a
/// traced name to fully tainted (`apply_module_variable_traces`).
mod variable_trace_taint {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn taint_warnings_for(source: &str) -> Vec<TaintWarning> {
        let registry = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        find_taint_warnings_for_cu(&cu, &registry, None)
    }

    /// FN guard: installing a `write` trace on an already-tainted
    /// variable must not clear its taint. The trace-add call's own
    /// literal argument words — `add`, `variable`, `x`, `write`,
    /// `{handler}` — are all clean, so a generic "propagate from
    /// arguments" fallback would treat the trace-add's synthetic write to
    /// `x` as deriving from them.
    #[test]
    fn write_trace_does_not_clear_prior_taint() {
        let w = taint_warnings_for(
            "set x [gets stdin]\ntrace add variable x write {sanitize}\nputs $x",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "trace installation must not launder x's existing taint, got {w:?}"
        );
    }

    /// FN direction: a variable set to a literal (clean) value is
    /// conservatively treated as tainted once *any* trace (even a
    /// `read` trace) is attached — the handler could inject
    /// attacker-controlled content on every future access.
    #[test]
    fn traced_literal_value_is_conservatively_tainted() {
        // The handler must exist: an undefined one makes the read raise
        // in C Tcl, so `puts` never runs.
        let w = taint_warnings_for(
            "proc inject {name args} { upvar 1 $name v; set v [gets stdin] }\n\
                 set x \"safe-literal\"\ntrace add variable x read {inject}\nputs $x",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "a traced variable must be treated as tainted even from a literal set, got {w:?}"
        );
    }

    /// Cross-proc: a trace installed by one proc still taints a read of
    /// the same global variable in a different function (the module-wide
    /// `Module::traced_variables` over-approximation, not a per-function
    /// one).
    #[test]
    fn trace_installed_in_another_proc_still_taints_this_read() {
        let w = taint_warnings_for(
            "proc install_trace {} { trace add variable ::x write {cb} }\n\
                 set x [gets stdin]\n\
                 install_trace\n\
                 puts $x",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 despite the trace living in a different proc, got {w:?}"
        );
    }

    /// A dynamic trace target (`trace add variable $name …`) can't be
    /// resolved to a literal name, so — mirroring
    /// `Module::has_dynamic_variable_trace` — every name in the module is
    /// conservatively treated as traced (tainted).
    #[test]
    fn dynamic_trace_target_taints_every_name() {
        let w = taint_warnings_for(
            "set x \"safe-literal\"\ntrace add variable $somevar write {cb}\nputs $x",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "a dynamic trace target must conservatively taint every name, got {w:?}"
        );
    }

    /// TN control: an untraced variable behaves exactly as before — a
    /// clean literal stays clean.
    #[test]
    fn untraced_literal_value_stays_clean() {
        let w = taint_warnings_for("set x hello\nputs $x");
        assert_eq!(w.len(), 0, "no trace in scope, expected no warnings: {w:?}");
    }

    /// TN control: an untraced tainted variable is unaffected by this
    /// fix — still flagged exactly as the pre-existing behaviour.
    #[test]
    fn untraced_tainted_value_still_flagged() {
        let w = taint_warnings_for("set x [gets stdin]\nputs $x");
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 for a plain untraced tainted puts call, got {w:?}"
        );
    }
}

/// Unit tests for the pure [`apply_module_variable_traces`] helper,
/// isolated from the end-to-end plumbing tested by
/// `variable_trace_taint` above.
mod apply_module_variable_traces_tests {
    use super::*;
    use crate::compilation_unit::ModuleTraceFacts;

    #[test]
    fn forces_existing_entries_for_a_traced_name() {
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {} { trace add variable t write cb\nset t 1\nputs $t }",
            &CommandRegistry::build_default()
                .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap()),
            false,
        );
        assert!(cu.ir_module.traced_variables.contains("t"));
        let traces = ModuleTraceFacts {
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
        };

        let fu = cu.function("::p").unwrap();
        let patched = apply_module_variable_traces((*fu.taints).clone(), &fu.ssa, traces);
        let first_definitions: Vec<_> = patched
            .iter()
            .filter(|((symbol, version), _)| *version == 1 && fu.ssa.var_name(*symbol) == "t")
            .collect();
        assert!(
            !first_definitions.is_empty(),
            "the traced variable must have a represented first definition: {patched:?}"
        );
        assert!(
            first_definitions
                .iter()
                .all(|(_, taint)| taint.is_tainted()),
            "every canonical t#1 definition must be forced tainted: {patched:?}"
        );
    }

    #[test]
    fn leaves_untraced_names_unchanged() {
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {} { set t 1\nputs $t }",
            &registry,
            false,
        );
        assert!(!cu.ir_module.traced_variables.contains("t"));
        assert!(!cu.ir_module.has_dynamic_variable_trace);
        let traces = ModuleTraceFacts {
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
        };
        let fu = cu.function("::p").unwrap();
        let patched = apply_module_variable_traces((*fu.taints).clone(), &fu.ssa, traces);
        assert_eq!(
            patched, *fu.taints,
            "no trace in scope — map must be unchanged"
        );
    }
}
