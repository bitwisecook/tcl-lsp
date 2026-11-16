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

//! What the scan records for a call, opaque callers included, and how the evidence merges, slices and converges.

use super::*;

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
    let unknown = super::evidence(
        "proc helper {x} {return $x}\nhelper prod\nafter idle {$::runtime_command $::runtime_value}",
    );
    assert!(
        unknown
            .get("::helper")
            .is_some_and(|site| site.opaque_caller)
    );
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
