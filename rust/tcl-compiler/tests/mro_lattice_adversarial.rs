// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Adversarial check for the class-lattice dispatch resolver
//! (`analyser::class_lattice`, EXPERIMENT).
//!
//! The contract under test: on dynamic-dispatch shapes the resolver must
//! **abstain** (⊤) rather than mis-resolve.  A confident wrong answer is
//! worse than an honest abstention.  Each case asserts either an
//! `Abstain(reason)` verdict or — for the one shape the lattice is
//! *designed* to handle (a control-flow join of two known classes) — a
//! correct `Resolved` set.

use tcl_compiler::analyser::class_lattice::TopReason;
use tcl_compiler::analyser::class_lattice::{
    AblationConfig, DispatchVerdict, NsContext, SiteReport, analyse_dispatch,
};
use tcl_compiler::analyser::state::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

/// Analyse `src`, then run the resolver over its `$obj method` sites with
/// the full model against the file's own class index.
fn resolve(src: &str) -> Vec<SiteReport> {
    let reg = CommandRegistry::build_default();
    let mut a = Analyser::new();
    let result = a.analyse(src, "tcl8.6");
    let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(
        &reg,
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
    );
    let ns = NsContext::from_result(&result);
    let (reports, _stats) = analyse_dispatch(
        &cu,
        &result.all_classes,
        &a.var_command_sites,
        &ns,
        &AblationConfig::full(),
    );
    reports
}

/// The verdict for the first `$<var> <method>` site, or panic.
fn verdict_for(src: &str, var: &str, method: &str) -> DispatchVerdict {
    let reports = resolve(src);
    reports
        .into_iter()
        .find(|r| r.var == var && r.method.as_deref() == Some(method))
        .unwrap_or_else(|| panic!("no dispatch site for ${var} {method}"))
        .verdict
}

const CLASSES: &str = r#"
oo::class create Animal { method speak {} { return "..." } }
oo::class create Dog { superclass Animal; method speak {} { return "woof" } }
oo::class create Cat { superclass Animal; method speak {} { return "meow" } }
"#;

fn with_classes(body: &str) -> String {
    format!("{CLASSES}\n{body}\n")
}

#[test]
fn reassigned_to_string_abstains_dynamic() {
    let src = with_classes("set o [Dog new]\nset o \"a string\"\n$o speak");
    assert_eq!(
        verdict_for(&src, "o", "speak"),
        DispatchVerdict::Abstain(TopReason::DynamicAssign),
    );
}

#[test]
fn factory_return_abstains() {
    let src = with_classes("proc make {} { return [Dog new] }\nset o [make]\n$o speak");
    // Either an honest FactoryReturn abstention, or — if interprocedural
    // return typing ever binds it — a *correct* resolution to Dog.  What
    // is forbidden is a confident *wrong* class.
    match verdict_for(&src, "o", "speak") {
        DispatchVerdict::Abstain(TopReason::FactoryReturn) => {}
        DispatchVerdict::Resolved {
            classes,
            method_known,
            ..
        } => {
            assert!(
                classes.iter().all(|c| c == "::Dog"),
                "must not name a wrong class: {classes:?}"
            );
            assert!(method_known);
        }
        v @ DispatchVerdict::Abstain(_) => panic!("unexpected verdict: {v:?}"),
    }
}

#[test]
fn introspection_dynamic_class_abstains() {
    let src = with_classes("set cls [info object class $x]\nset o [$cls new]\n$o speak");
    assert_eq!(
        verdict_for(&src, "o", "speak"),
        DispatchVerdict::Abstain(TopReason::Introspection),
    );
}

#[test]
fn oo_copy_abstains() {
    let src = with_classes("set o [oo::copy $other]\n$o speak");
    assert_eq!(
        verdict_for(&src, "o", "speak"),
        DispatchVerdict::Abstain(TopReason::Introspection),
    );
}

#[test]
fn per_object_mixin_abstains() {
    let src = with_classes(
        "set o [Dog new]\noo::objdefine $o { method fetch {} { return stick } }\n$o fetch",
    );
    assert_eq!(
        verdict_for(&src, "o", "fetch"),
        DispatchVerdict::Abstain(TopReason::PerObjectMixin),
    );
}

#[test]
fn bare_parameter_receiver_abstains_unknown() {
    let src = with_classes("proc go {obj} { $obj speak }");
    assert_eq!(
        verdict_for(&src, "obj", "speak"),
        DispatchVerdict::Abstain(TopReason::Unknown),
    );
}

#[test]
fn control_flow_join_resolves_the_set_correctly() {
    // The one case the lattice is *designed* to nail: a join of two known
    // classes, both of which implement the method.
    let src = with_classes("if {$flag} { set o [Dog new] } else { set o [Cat new] }\n$o speak");
    match verdict_for(&src, "o", "speak") {
        DispatchVerdict::Resolved {
            classes,
            method_known,
            ..
        } => {
            assert!(classes.contains("::Dog"), "classes={classes:?}");
            assert!(classes.contains("::Cat"), "classes={classes:?}");
            assert!(method_known, "both Dog and Cat implement speak");
        }
        v @ DispatchVerdict::Abstain(_) => panic!("expected a resolved join, got {v:?}"),
    }
}

#[test]
fn concrete_binding_resolves() {
    // Positive control: a plain local constructor resolves cleanly.
    let src = with_classes("set o [Dog new]\n$o speak");
    match verdict_for(&src, "o", "speak") {
        DispatchVerdict::Resolved {
            classes,
            method_known,
            providers,
        } => {
            assert_eq!(classes.iter().cloned().collect::<Vec<_>>(), vec!["::Dog"]);
            assert!(method_known);
            assert!(providers.contains("::Dog"));
        }
        v @ DispatchVerdict::Abstain(_) => panic!("expected resolved, got {v:?}"),
    }
}

#[test]
fn unknown_method_on_known_class_is_resolved_not_abstained() {
    // Class is known, method is not: this is the W308 candidate — the
    // resolver *names the class* (not ⊤) but flags the method unknown.
    let src = with_classes("set o [Dog new]\n$o fly");
    match verdict_for(&src, "o", "fly") {
        DispatchVerdict::Resolved {
            method_known,
            classes,
            ..
        } => {
            assert!(classes.contains("::Dog"));
            assert!(!method_known, "Dog has no `fly` and no unknown handler");
        }
        v @ DispatchVerdict::Abstain(_) => {
            panic!("expected resolved-but-unknown-method, got {v:?}")
        }
    }
}
