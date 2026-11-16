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

//! The module mutation summary: frame opacity, trust snapshots and procedure-binding trust.

use super::*;

/// A `namespace import` that shadows a proc declared in the same
/// namespace must distrust that binding — tclsh answers `EVIL` for the
/// sheet below, so folding `::ns::answer` with the declared body is
/// wrong. A *globbed* import elsewhere must not distrust the source
/// namespace it imports *from*, or an ordinary `namespace import ::lib::*`
/// would stop every fold in `::lib`.
#[test]
fn an_import_shadowing_a_declared_proc_distrusts_only_its_namespace() {
    let reg = CommandRegistry::build_default();
    let shadowed = "namespace eval ::evil { proc answer {} {return EVIL} ; namespace export answer }\n             namespace eval ::ns { proc answer {} { return 42 } ; namespace import -force ::evil::answer }\n";
    let cu = CompilationUnit::build_for(shadowed, &reg, false);
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(
        !m.trusts_proc_binding("::ns::answer"),
        "the import shadows the declared proc"
    );
    assert!(
        m.trusts_proc_binding("::evil::answer"),
        "the namespace imported *from* is untouched"
    );

    // A wildcard import makes only its target namespace opaque.
    let wildcard = "namespace eval ::lib { proc helper {} { return 1 } ; namespace export helper }\n             namespace eval ::app { namespace import ::lib::* }\n";
    let cu = CompilationUnit::build_for(wildcard, &reg, false);
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.trusts_proc_binding("::app::helper"));
    assert!(
        m.trusts_proc_binding("::lib::helper"),
        "the imported-from namespace still folds"
    );
}

/// A `TclOO` method runs in the receiver's namespace, chosen at run time,
/// so a command it names relatively may resolve to an object-local
/// implementation. That uncertainty belongs to the frame: it disqualifies
/// facts derived inside the body, and leaves the names the top level and
/// the procedures resolve untouched.
#[test]
fn an_ordinary_method_body_scopes_its_opacity_to_its_own_frame() {
    let reg = CommandRegistry::build_default();
    for body in [
        "puts hi",
        "my helper",
        "next",
        "set y 1",
        "if {1} { puts hi }",
    ] {
        let src = format!(
            "oo::class create ::A {{\n    method m {{}} {{ {body} }}\n}}\nproc p {{}} {{ return 1 }}\n"
        );
        let cu = CompilationUnit::build_for(&src, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            m.has_runtime_selected_frames(),
            "{body}: the frame itself stays opaque"
        );
        assert!(
            m.trusts("string"),
            "{body}: a builtin the module never touches must stay trusted"
        );
        assert!(
            m.trusts_proc_binding("::p"),
            "{body}: a proc declared outside the class must stay trusted"
        );
    }
}

/// A binding change from such a frame names its subject where this scan
/// cannot look, so it carries past the frame and no name in the module
/// survives it.
#[test]
fn a_binding_change_in_a_runtime_selected_frame_distrusts_the_module() {
    let reg = CommandRegistry::build_default();
    for body in [
        "rename puts myputs",
        "proc helper {} { return 1 }",
        "interp alias {} shout {} puts",
        "namespace import ::lib::*",
    ] {
        let src = format!(
            "oo::class create ::A {{\n    method m {{}} {{ {body} }}\n}}\nproc p {{}} {{ return 1 }}\n"
        );
        let cu = CompilationUnit::build_for(&src, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts_proc_binding("::p"),
            "{body}: an unspellable subject must distrust the module"
        );
    }
}

/// A rebinding does not stop carrying past the frame because it sits in a
/// statically lowered `eval` or `uplevel`. Those lower to
/// `Statement::Block` / `Statement::UpFrame`, whose bodies are part of the
/// frame like any other nested script: `m` really does remove `::string`,
/// so a fold of `[string length x]` elsewhere would be a miscompile.
#[test]
fn a_rebinding_inside_a_lowered_eval_or_uplevel_still_distrusts_the_module() {
    let reg = CommandRegistry::build_default();
    for body in [
        "puts hi; eval {rename ::string ::saved}",
        "puts hi; uplevel 1 {rename ::string ::saved}",
        "puts hi; if {1} { eval {rename ::string ::saved} }",
    ] {
        let src = format!(
            "oo::class create ::A {{\n    method m {{}} {{ {body} }}\n}}\nproc p {{}} {{ return 1 }}\n"
        );
        let cu = CompilationUnit::build_for(&src, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts("string"),
            "{body}: a rebinding in a lowered body must distrust the module"
        );
    }
}

/// A body whose commands all resolve absolutely is namespace-invariant, so
/// it takes the ordinary scan and leaves no frame opacity behind.
#[test]
fn an_absolute_only_method_body_leaves_no_frame_opacity() {
    let reg = CommandRegistry::build_default();
    let src = "oo::class create ::A {\n    method m {} { ::puts hi }\n}\nproc p {} { return 1 }\n";
    let cu = CompilationUnit::build_for(src, &reg, false);
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.has_runtime_selected_frames());
    assert!(m.trusts_proc_binding("::p"));
    assert!(m.trusts("puts"));
}

/// The snapshot is the per-procedure lattice memo's trust fact, so every
/// field of the summary must survive the round trip and take part in the
/// key: a memo taken under one binding state must never answer for
/// another. The literal names every field, so a field added to the
/// summary fails to compile here until it is carried.
#[test]
fn opaque_namespace_imports_preserve_empty_command_tails() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    let mutations = ModuleCommandMutations {
        opaque_namespaces: ["::ns".to_owned()].into_iter().collect(),
        ..ModuleCommandMutations::default()
    };
    for name in ["ns::", "::ns::", "::ns::::", "::ns::ordinary"] {
        assert!(!mutations.trusts_proc_binding(name), "{name}");
    }
    assert!(mutations.trusts_proc_binding("::ns"));
    assert!(mutations.trusts_proc_binding("::other::"));
}

#[test]
fn the_trust_snapshot_round_trips_every_mutation_field() {
    let every = ModuleCommandMutations {
        names: std::iter::once("llength".to_owned()).collect(),
        rebound: std::iter::once("p".to_owned()).collect(),
        dynamic: true,
        rebinding_subjects: RebindingSubjects::SomeUnnameable,
        runtime_selected_frames: true,
        resolution_changed: true,
        opaque_namespaces: std::iter::once("::ns".to_owned()).collect(),
        precision_written: PrecisionWrites::MayBeWritten,
    };
    assert_eq!(every.snapshot().to_mutations(), every);
    let base = ModuleCommandMutations::default();
    let one_field: [ModuleCommandMutations; 8] = [
        ModuleCommandMutations {
            names: every.names.clone(),
            ..base.clone()
        },
        ModuleCommandMutations {
            rebound: every.rebound.clone(),
            ..base.clone()
        },
        ModuleCommandMutations {
            dynamic: true,
            ..base.clone()
        },
        ModuleCommandMutations {
            rebinding_subjects: RebindingSubjects::SomeUnnameable,
            ..base.clone()
        },
        ModuleCommandMutations {
            runtime_selected_frames: true,
            ..base.clone()
        },
        ModuleCommandMutations {
            resolution_changed: true,
            ..base.clone()
        },
        ModuleCommandMutations {
            opaque_namespaces: every.opaque_namespaces.clone(),
            ..base.clone()
        },
        ModuleCommandMutations {
            precision_written: PrecisionWrites::MayBeWritten,
            ..base.clone()
        },
    ];
    for m in &one_field {
        assert_eq!(&m.snapshot().to_mutations(), m);
        assert_ne!(m.snapshot(), base.snapshot(), "{m:?} is part of the key");
    }

    // A scanned summary answers both trust stances alike from the
    // snapshot: `rename $a {}` names no subject, so no builtin is
    // claimable (#2168), and an unknown head raises only the top.
    let reg = CommandRegistry::build_default();
    for src in [
        "set a llength\nrename $a {}\n",
        "someUnknownLibraryCall x\n",
        "proc llength {l} { return 99 }\n",
    ] {
        let cu = CompilationUnit::build_for(src, &reg, false);
        let scanned = scan_module_command_mutations(&cu.ir_module, &reg);
        let restored = scanned.snapshot().to_mutations();
        assert_eq!(restored, scanned, "{src:?}");
        for name in ["llength", "list", "set"] {
            assert_eq!(
                restored.observed_binding_is_the_builtin(name),
                scanned.observed_binding_is_the_builtin(name),
                "{src:?} {name}"
            );
            assert_eq!(
                restored.trusts(name),
                scanned.trusts(name),
                "{src:?} {name}"
            );
        }
    }
}

/// The frame fact is part of the memo key: a summary taken with every
/// frame statically nameable must not be reused once one is not.
#[test]
fn the_frame_fact_survives_a_snapshot_round_trip() {
    let reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = "oo::class create ::A {\n    method m {} { puts hi }\n}\n";
    let cu = CompilationUnit::build_for(src, &reg, false);
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(m.has_runtime_selected_frames());
    assert!(m.snapshot().to_mutations().has_runtime_selected_frames());
    assert_ne!(
        m.snapshot(),
        ModuleCommandMutations::default().snapshot(),
        "the frame fact must distinguish the two summaries"
    );
}

#[test]
fn independently_analysed_summaries_are_semantically_equal() {
    let (cu, reg) = analyse("interp alias {} e {} expr\ne {1 + 1}");
    let first = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    let second = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

    assert!(!Arc::ptr_eq(&first.bindings, &second.bindings));
    assert!(!Arc::ptr_eq(
        &first.root_boundary_bindings,
        &second.root_boundary_bindings
    ));
    assert_eq!(first, second);
    assert!(
        !first.same_state(&second),
        "the analysis-local fast path deliberately requires one shared baseline Arc"
    );
}

#[test]
fn module_mutations_distrust_rebound_builtins_only() {
    let reg = CommandRegistry::build_default();
    // A builtin renamed inside a proc body is distrusted everywhere
    // (over-approximation); a fresh user proc untrusts nothing.
    let cu = CompilationUnit::build_for(
        "proc clobber {} { rename string {} }\nproc myproc {} { return 1 }",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.trusts("string"), "string is rebound in a proc body");
    assert!(m.trusts("lappend"), "an untouched builtin stays trusted");
    assert!(m.trusts("myproc"), "a fresh user proc untrusts nothing");

    // An actual unknown Value formal keeps the mutation subject unknown.
    let cu2 = CompilationUnit::build_for("proc clobber {x} {rename $x bar}", &reg, false);
    let m2 = scan_module_command_mutations(&cu2.ir_module, &reg);
    assert!(!m2.trusts("string") && !m2.trusts("lappend"));

    // A frozen absent subject errors before moving any command. Its
    // written substitution does not turn the subject into a dynamic one.
    let cu3 = CompilationUnit::build_for("set x foo\nrename $x bar", &reg, false);
    let m3 = scan_module_command_mutations(&cu3.ir_module, &reg);
    assert!(m3.trusts("string") && m3.trusts("lappend"));
}

#[test]
fn trusts_proc_binding_true_for_untouched_proc() {
    // TP control: a proc never named by any `rename` / `interp alias`
    // is trusted.
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for("proc myproc {} { return 1 }", &reg, false);
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(m.trusts_proc_binding("myproc"));
    assert!(m.trusts_proc_binding("::myproc"));
}

#[test]
fn trusts_proc_binding_false_for_rename_source_and_target() {
    // FP guard: `rename triple double` perturbs BOTH names — `triple`
    // (vacated, no longer denotes what it did) and `double` (now
    // denotes `triple`'s body, not `double`'s own declaration).
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple double\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.trusts_proc_binding("double"), "rename target");
    assert!(!m.trusts_proc_binding("triple"), "rename source");
}

#[test]
fn trusts_proc_binding_false_for_interp_alias_name() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc answer {} { return 42 }\nproc other {} { return 99 }\ninterp alias {} answer {} other\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.trusts_proc_binding("answer"));
    // The alias *target* itself is untouched — still trusted.
    assert!(m.trusts_proc_binding("other"));
}

#[test]
fn trusts_proc_binding_follows_alias_resolved_mutations() {
    let reg = CommandRegistry::build_default();
    let renamed = CompilationUnit::build_for(
        "proc p {} {return P}\nproc q {} {return Q}\n\
             interp alias {} r {} rename\n\
             r p oldp\nr q p\n",
        &reg,
        false,
    );
    let mutations = scan_module_command_mutations(&renamed.ir_module, &reg);
    for name in ["p", "q", "oldp"] {
        assert!(
            !mutations.trusts_proc_binding(name),
            "alias-resolved rename touched {name}"
        );
    }

    let aliased = CompilationUnit::build_for(
        "proc p {} {return P}\nproc q {} {return Q}\n\
             interp alias {} replace {} interp alias {} p {} q\n\
             replace\n",
        &reg,
        false,
    );
    let mutations = scan_module_command_mutations(&aliased.ir_module, &reg);
    assert!(!mutations.trusts_proc_binding("p"));
    assert!(mutations.trusts_proc_binding("q"));
}

#[test]
fn trusts_proc_binding_unaffected_by_unrelated_rename() {
    // TN control: renaming a DIFFERENT proc must not untrust this one —
    // `trusts_proc_binding` is per-name, unlike the whole-module
    // `dynamic` wildcard.
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple somethingElse\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(m.trusts_proc_binding("double"));
}

/// A fully-qualified `proc ::n::expr` shadows the builtin for bodies that
/// resolve in `::n`, so the builtin is no longer foldable (#2159).
///
/// The two spellings used to disagree: `namespace eval ::n { proc expr … }`
/// distrusted `expr`, while `proc ::n::expr` did not, because the recorded
/// name is `::n::expr` and only a name whose *own* default is a builtin was
/// collected. Measured harm before the fix — `tcl explore --show opt`
/// rewrote
///
/// ```text
/// proc ::n::f {r} { return [expr {$r ** 2}] }
/// ```
///
/// into `[expr {$r * $r}]`, and with `proc ::n::expr {s} {return "SHADOW:$s"}`
/// that changes the program's output from `SHADOW:$r ** 2` to
/// `SHADOW:$r * $r` on tclsh 9.0.4.
#[test]
fn qualified_proc_shadowing_a_builtin_distrusts_it() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::n {}\nproc ::n::expr {s} { return $s }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(
        !m.trusts("expr"),
        "a qualified shadow distrusts the builtin"
    );
}

/// A qualified shadow installed through an alias is distrusted too.
///
/// `interp alias {} make {} proc ::n::expr` carries the prepended name,
/// so the source scan never sees the spelling `proc ::n::expr` — but
/// `ModuleCommandBindings` recovers the definition, and the same tail
/// projection applies there. tclsh 9.0.4 prints `SHADOW:$r ** 2` for the
/// program below; without this, O110 still rewrote the body to
/// `[expr {$r * $r}]`.
#[test]
fn an_alias_installed_qualified_shadow_is_distrusted() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::n {}\ninterp alias {} make {} proc ::n::expr\nmake {s} { return \"SHADOW:$s\" }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(
        !m.trusts("expr"),
        "an alias-installed qualified shadow distrusts the builtin",
    );
}

/// And an alias installing something whose tail is not a builtin leaves
/// trust alone.
#[test]
fn an_alias_installing_a_non_builtin_tail_keeps_trust() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::n {}\ninterp alias {} make {} proc ::n::helper\nmake {s} { return $s }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(m.trusts("expr"), "an unrelated alias keeps trust");
}

/// The same, for a command other than `expr` — the scan is keyed on the
/// tail having a builtin default, not on any one name.
#[test]
fn qualified_shadow_distrust_is_not_specific_to_expr() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::n {}\nproc ::n::llength {s} { return 99 }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(!m.trusts("llength"), "any shadowed builtin tail distrusts");
    assert!(m.trusts("expr"), "and only the one that was shadowed");
}

/// A qualified `proc` whose tail is *not* a builtin changes nothing —
/// the guard must not withdraw folding from every namespaced file.
#[test]
fn qualified_proc_with_a_non_builtin_tail_keeps_trust() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::n {}\nproc ::n::helper {s} { return $s }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(m.trusts("expr"), "an unrelated qualified proc keeps trust");
}

#[test]
fn rebound_candidates_include_relative_qualified_names() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    let namespace = crate::ir::ExecutionNamespace::exact("::ns");
    for (written, expected) in [
        ("inner::p", vec!["::inner::p", "::ns::inner::p"]),
        ("p", vec!["::ns::p", "::p"]),
        ("::inner::p", vec!["::inner::p"]),
    ] {
        let mut bindings = ModuleCommandBindings::default();
        bindings.record_proc_rebound_candidates(written, &namespace);
        assert_eq!(
            bindings.proc_rebound_names,
            expected.into_iter().map(String::from).collect()
        );
        assert!(bindings.bindings.is_empty());
    }
}

#[test]
fn trusts_proc_binding_false_for_namespace_relative_rename() {
    // FP guard (reported in code review): a bare `rename` argument
    // inside a namespaced proc resolves relative to THAT proc's own
    // namespace, not the global namespace — `rename triple double`
    // inside `proc ::ns::doit` renames `::ns::triple` onto
    // `::ns::double`. An earlier version always rooted the bare names
    // globally (`::triple`/`::double`), so it never distrusted the
    // actually-affected namespaced names.
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "namespace eval ::ns {\n    proc double {n} { expr {$n * 2} }\n    proc triple {n} { expr {$n * 3} }\n}\nproc ::ns::doit {} { rename triple double }\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(
        !m.trusts_proc_binding("::ns::double"),
        "namespace-relative rename target"
    );
    assert!(
        !m.trusts_proc_binding("::ns::triple"),
        "namespace-relative rename source"
    );
    // The scan is flow-insensitive (it can't know whether `double`/
    // `triple` already existed as GLOBAL commands at the point
    // `::ns::doit` runs), so it conservatively distrusts the
    // global-rooted candidate too — a deliberate, sound
    // over-approximation (a missed fold, never a wrong one),
    // mirroring `collect_tampered_builtins`'s existing philosophy.
    assert!(
        !m.trusts_proc_binding("::double"),
        "global-rooted candidate"
    );
    assert!(
        !m.trusts_proc_binding("::triple"),
        "global-rooted candidate"
    );
}

// Regression: `proc max {...}` must not be distrusted. `max`/`min` read
// like `tcl::mathop` operator words, but real Tcl never registered them
// there (verified against tclsh 8.6/9.0 — `info commands
// ::tcl::mathop::*` never lists them); they exist only as unrelated
// `expr` math functions. A now-fixed registry bug once carried bare
// `max`/`min` `CommandSpec` entries as if they were `tcl::mathop`
// members, which made `default_binding` treat them as pre-existing
// builtins — so a completely ordinary `proc max {...}` looked like it
// was "renaming a builtin", silently blocking O103 from folding calls to
// it (caught by `tests/optimiser.rs::interprocedural_constant_folding`).
#[test]
fn module_mutations_do_not_distrust_proc_named_like_mathop_word() {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc max {a b} {\n    if {$a > $b} { return $a } else { return $b }\n}\nset v [max 3 7]\n",
        &reg,
        false,
    );
    let m = scan_module_command_mutations(&cu.ir_module, &reg);
    assert!(
        m.trusts("max"),
        "a plain proc sharing a name with an (incorrectly bare-registered) \
             tcl::mathop-lookalike must not be distrusted"
    );
}
