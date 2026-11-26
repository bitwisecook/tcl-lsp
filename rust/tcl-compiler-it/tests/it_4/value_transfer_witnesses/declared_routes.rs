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

//! Declared implementations and pack routes: folding through the driver, folded types, arity, rebinding and shadowing.

use super::*;

/// The bounded host's stand-in for `tenant::label`'s body `{name} { fold
/// [string cat "tenant:" $name] }`: one exact argument in, `tenant:` before
/// it out, under the release the call is pinned to.
struct LabelHost;

impl tcl_registry::pack_hooks::PackHookHost for LabelHost {
    fn invoke(
        &self,
        _slot: tcl_registry::pack_hooks::HookSlot,
        call: &tcl_registry::pack_hooks::HookCall<'_>,
    ) -> tcl_registry::pack_hooks::HookAnswer {
        use tcl_registry::pack_hooks::{EvaluationAnswer, HookAnswer};
        match call.words {
            [name] if call.dialect == Some("tcl9.0") => HookAnswer::Evaluation(EvaluationAnswer {
                result: Some(format!("tenant:{}", name.value)),
                stores: Vec::new(),
            }),
            _ => HookAnswer::Abstain,
        }
    }
}

/// A `tcl9.0` registry holding `tenant::label` as a pack declares it —
/// `arity 1`, `evaluate -implementation tenant.label.v1 -host bounded_tcl`
/// reading `arg 0 exact` — its body bound to an `evaluate` slot.
fn tenant_label_registry() -> tcl_registry::CommandRegistry {
    use tcl_registry::pack_hooks::{self, HookFamily, HookInput, HookInputs};
    use tcl_registry::spec::CommandSpec;
    use tcl_registry::value_transfer::{
        CompletionSupport, ContextDependency, DeclaredEvaluation, DeclaredImplementation,
        DeclaredInput, DeclaredSemantics, DeclaredStructure, EvaluatorCapability, Exactness,
        HostKind, ImplementationBudget, ImplementationIdentity, Needs, SemanticsDeclaration,
    };

    let slot = pack_hooks::allocate(
        HookFamily::Evaluate,
        &HookInputs::declared([HookInput::Words]),
    )
    .expect("an evaluate slot");
    let label: &'static DeclaredSemantics = Box::leak(Box::new(DeclaredSemantics {
        scope: "tenant::label",
        structure: DeclaredStructure::default(),
        evaluation: DeclaredEvaluation::Implementation(DeclaredImplementation {
            capability: EvaluatorCapability {
                identity: ImplementationIdentity {
                    pack: "tenant",
                    id: "tenant.label.v1",
                    content_hash: 1,
                },
                host: HostKind::BoundedTcl,
                target: Needs::NONE,
                inputs: &[DeclaredInput::Operand {
                    index: 0,
                    exactness: Exactness::Exact,
                }],
                depends: &[
                    ContextDependency::TclProfile,
                    ContextDependency::ImplementationIdentity,
                ],
                budget: ImplementationBudget {
                    commands: Some(2000),
                    wall_clock_ms: Some(20),
                    value_bytes: Some(65536),
                },
                completion: CompletionSupport::NormalOnly,
            },
            slot: Some(slot),
            extension: None,
        }),
        option_declines: &[],
    }));
    let mut base = tcl_registry::CommandRegistry::build_default();
    base.insert(CommandSpec {
        name: "tenant::label",
        semantics: SemanticsDeclaration::Declared(label),
        ..CommandSpec::DEFAULT
    });
    base.project_for_profile(tcl_dialect::DialectProfile::find("tcl9.0").expect("the profile"))
}

/// A declared implementation runs through the driver as a shipped route
/// does, from the pack's declaration alone: `tenant::label acme` folds to
/// `tenant:acme` through the body's `fold`, and the entry counts as an
/// implementation; an argument the analysis does not know declines
/// `NotExact` before any body runs; and a worker with no host declines
/// `Transient` — a state of the worker, never a verdict on the inputs. The
/// host here stands in for the bounded one, which runs the body in
/// `tcl-spec-hooks`' own witnesses.
#[test]
pub(super) fn a_declared_implementation_folds_through_the_driver() {
    use tcl_registry::pack_hooks;

    let registry = tenant_label_registry();
    let source = "proc p {x} {\n\
                  set known [tenant::label acme]\n\
                  set unknown [tenant::label $x]\n\
                  return $known$unknown\n\
                  }\n";
    let routes = |unit: &CompilationUnit| -> Vec<String> {
        unit.procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure")
            .sccp
            .explanations
            .iter()
            .filter(|explanation| explanation.command == "tenant::label")
            .map(|explanation| explanation.route.clone())
            .collect()
    };

    pack_hooks::install_host(std::rc::Rc::new(LabelHost));
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    assert_eq!(
        value_at(&unit, "::p", "known", 1),
        Some(text("tenant:acme"))
    );
    assert_eq!(
        value_at(&unit, "::p", "unknown", 1),
        Some(LatticeValue::Overdefined)
    );
    assert_eq!(
        answers_for(&unit, "::p", "tenant::label"),
        ["evaluated", "declined: not-exact"]
    );
    assert_eq!(
        routes(&unit),
        [
            "implementation tenant.label.v1",
            "implementation tenant.label.v1"
        ]
    );
    let tally = unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("the procedure")
        .sccp
        .route_tally;
    assert_eq!(tally.implementation, 2, "both calls enter the route");
    assert_eq!(tally.direct, 0);

    // The same worker without its host: the known call cannot run, and
    // says so as a transient decline; the unknown one still declines on
    // its input first.
    pack_hooks::clear_host();
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    assert_eq!(
        value_at(&unit, "::p", "known", 1),
        Some(LatticeValue::Overdefined)
    );
    assert_eq!(
        answers_for(&unit, "::p", "tenant::label"),
        ["declined: transient", "declined: not-exact"]
    );
}

/// The value-transfer design's executable example: a private command
/// a workspace pack declares under its own name, a second name, and a
/// subcommand form whose operand sits one word later.
const TENANT_PACK: &str =
    include_str!("../../../../tcl-compiler/tests/fixtures/value_transfers/tenant.tclspec");

/// The example's three spellings, each taking the name as its last word.
const TENANT_SPELLINGS: [&str; 3] = ["tenant::label", "tenant::tag", "tenant label"];

/// What a vendor runtime provides for the example: the three spellings as
/// real commands, written for every release from 8.4 (no `string cat`, no
/// `namespace ensemble`), so a program the analysis never sees the
/// definitions of runs under each `tclsh`.
const TENANT_RUNTIME: &str = "namespace eval ::tenant {\n\
                              \x20   proc label {name} {return \"tenant:$name\"}\n\
                              \x20   proc tag {name} {return \"tenant:$name\"}\n\
                              }\n\
                              proc ::tenant {subcommand name} {\n\
                              \x20   if {$subcommand ne \"label\"} {error \"bad subcommand $subcommand\"}\n\
                              \x20   return [::tenant::label $name]\n\
                              }\n";

/// The example loaded as a workspace loads it: the pack set, its hook plan
/// published and this thread's host built from it, as the language server
/// and the CLI do on a pack load.
fn tenant_workspace() -> tcl_spectcl::PackSet {
    pack_workspace("tenant", TENANT_PACK)
}

/// Program (2) of the interface page: `binary format` declares a
/// registry-owned route, so `set h [binary format H* 414243444546]` is
/// `ABCDEF` in the shared lattice under every profile, typed a byte array by
/// construction, and neither S100 nor S110 reports a conversion for it. A
/// computed byte array has no lossless source spelling, so no rewrite writes
/// it into the program (`SccpResult::materialises`): the optimised program
/// keeps the command that builds it and prints what every release prints.
#[test]
fn program_two_folds_and_is_a_byte_array() {
    use tcl_compiler::shimmer::{find_byte_array_warnings_for_cu, find_shimmer_warnings_for_cu};
    use tcl_compiler::value_transfer::FoldedType;
    use tcl_registry::TclType;
    use tcl_registry::value_transfer::RepresentationEvidence;
    let source = "proc p {} {\n    set h [binary format H* 414243444546]\n    puts $h\n}\n\
                  proc b {} {\n    set g [binary format H* 4748]\n    return $g\n}\np\nputs [b]\n";
    let byte_array = FoldedType {
        intrep: Some(TclType::ByteArray),
        shape: None,
        representation: RepresentationEvidence::Constructed(TclType::ByteArray),
    };
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "h", 1),
            Some(text("ABCDEF")),
            "{dialect}"
        );
        assert_eq!(
            folded_at(&unit, "::p", "h", 1),
            Some(byte_array.clone()),
            "{dialect}"
        );
        let registry = static_context_for(dialect).commands();
        let shimmers = find_shimmer_warnings_for_cu(&unit, registry);
        assert!(shimmers.is_empty(), "{dialect}: {shimmers:?}");
        let damage = find_byte_array_warnings_for_cu(&unit, registry);
        assert!(damage.is_empty(), "{dialect}: {damage:?}");
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            !rewritten.contains("ABCDEF") && !rewritten.contains("GH"),
            "{dialect}: a byte array was written into the source:\n{rewritten}"
        );
    }
    prints_under_every_release(source, "ABCDEF\nGH\n");
}

/// The shared lattice keeps what each evaluation states of its
/// value's type beside the value itself (`SccpResult::folded_types`). A
/// result and a write carry the type facts and the representation the route
/// constructed — `string length` and `incr` build an int, `list` a list,
/// `append` a string; a copy shares its source's; a φ keeps what its arms
/// state alike; a literal states nothing; an opaque barrier (`eval
/// $script`) widens every value and forgets what they stated. A pack's declared implementation states its
/// `result -semantic` type and no representation, and the type lattice
/// takes it where the static typing knows nothing of a pack command's
/// result.
#[test]
fn folded_types_state_what_each_route_constructed() {
    use tcl_compiler::value_transfer::FoldedType;
    use tcl_registry::TclType;
    use tcl_registry::value_transfer::RepresentationEvidence;
    let built = |ty: TclType| {
        Some(FoldedType {
            intrep: Some(ty),
            shape: None,
            representation: RepresentationEvidence::Constructed(ty),
        })
    };
    let unit = unit_of(
        "proc p {c} {\n    set s abc\n    set n [string length $s]\n    set l [list a b]\n    \
         incr n\n    append s x\n    set m $n\n    \
         if {$c} {set k [llength $l]} else {set k [string length $s]}\n    \
         return $m$k$l$s\n}\n\
         proc b {script} {\n    set s abc\n    set n [string length $s]\n    eval $script\n    \
         return $n\n}\n",
        "tcl9.0",
    );
    assert_eq!(folded_at(&unit, "::p", "s", 1), None, "a literal");
    assert_eq!(folded_at(&unit, "::p", "n", 1), built(TclType::Int));
    assert_eq!(folded_at(&unit, "::p", "l", 1), built(TclType::List));
    assert_eq!(folded_at(&unit, "::p", "n", 2), built(TclType::Int), "incr");
    assert_eq!(
        folded_at(&unit, "::p", "s", 2),
        built(TclType::String),
        "append"
    );
    assert_eq!(
        folded_at(&unit, "::p", "m", 1),
        built(TclType::Int),
        "a copy"
    );
    assert_eq!(
        folded_at(&unit, "::p", "k", 1),
        built(TclType::Int),
        "the φ"
    );
    assert_eq!(
        built(TclType::ByteArray)
            .map(|folded| folded.label())
            .as_deref(),
        Some("bytearray (constructed)")
    );
    let barrier = unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::b"))
        .expect("the procedure");
    assert!(
        barrier.sccp.folded_types.is_empty(),
        "{:?}",
        barrier.sccp.folded_types
    );

    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = tenant_workspace();
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &packs);
    let unit = CompilationUnit::build_for_dialect(
        "proc p {} {\n    set r [tenant::label acme]\n    return $r\n}\n",
        &registry,
        false,
        "tcl9.0",
    );
    let stated = Some(FoldedType {
        intrep: Some(TclType::String),
        shape: None,
        representation: RepresentationEvidence::Unknown,
    });
    assert_eq!(folded_at(&unit, "::p", "r", 1), stated);
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("the procedure");
    let r = function.ssa.var_symbol("r").expect("the variable");
    assert_eq!(
        function.types.get(&(r, 1)),
        Some(&tcl_compiler::types::TypeLattice::of(TclType::String)),
        "the type lattice takes the declared result type"
    );
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// The step-1 completion test (`docs/design/compiler/value-transfers.md`
/// § *The completion test*): one private command, renamed, and given a
/// subcommand form with its operand one word later, reaches the analysis
/// and the optimiser from its pack's declarations alone — the fixture is the
/// only file that names any of the three spellings.
///
/// - Each spelling folds `acme` to `tenant:acme` through the analysis, on
///   the implementation route, and an argument the analysis does not know
///   declines `not-exact`; without the pack the same program folds nothing,
///   so the answer is the declaration's.
/// - The body runs under the analysed release: it folds under 8.6, 9.0 and
///   9.1, and under 8.4, 8.5 and iRules' 8.4 base it raises and declines,
///   because `string cat` is 8.6's — the releases `tclsh` itself runs `puts
///   [string cat tenant: acme]` under. A profile naming no release has no
///   engine to run it on and declines.
/// - The optimiser forwards the folded constant (O100), and the vendor
///   runtime — [`TENANT_RUNTIME`], which the analysis never sees — prints
///   what the optimised program prints under every `tclsh` on `PATH`.
#[test]
fn the_completion_test_needs_no_consumer_edit() {
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = tenant_workspace();
    for (dialect, folds) in [
        ("tcl8.4", false),
        ("tcl8.5", false),
        ("tcl8.6", true),
        ("tcl9.0", true),
        ("tcl9.1", true),
        ("f5-irules", false),
        ("tcl", false),
    ] {
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, &packs);
        for spelling in TENANT_SPELLINGS {
            every_spelling_answers(&registry, dialect, spelling, folds);
        }
    }
    the_vendor_runtime_prints_what_the_optimiser_forwards(&packs);
}

/// `spelling acme` and `spelling $x` in one procedure under `dialect`: both
/// calls enter the implementation route; the known argument folds to
/// `tenant:acme` where the body runs (`folds`) and declines `unsupported`
/// where it cannot; the unknown one declines `not-exact`. The same program
/// against the registry without the pack folds nothing.
fn every_spelling_answers(
    registry: &tcl_registry::CommandRegistry,
    dialect: &str,
    spelling: &str,
    folds: bool,
) {
    let head = spelling.split(' ').next().expect("a head");
    let source = format!(
        "proc p {{x}} {{\n    set known [{spelling} acme]\n    set unknown [{spelling} $x]\n    return $known$unknown\n}}\n"
    );
    let unit = CompilationUnit::build_for_dialect(&source, registry, false, dialect);
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("the procedure");
    let routes: Vec<&str> = function
        .sccp
        .explanations
        .iter()
        .filter(|explanation| explanation.command == head)
        .map(|explanation| explanation.route.as_str())
        .collect();
    assert_eq!(
        routes,
        [
            "implementation tenant.label.v1",
            "implementation tenant.label.v1"
        ],
        "{dialect} {spelling}"
    );
    let (value, answer) = if folds {
        (text("tenant:acme"), "evaluated")
    } else {
        (LatticeValue::Overdefined, "declined: unsupported")
    };
    assert_eq!(
        value_at(&unit, "::p", "known", 1),
        Some(value),
        "{dialect} {spelling}"
    );
    assert_eq!(
        value_at(&unit, "::p", "unknown", 1),
        Some(LatticeValue::Overdefined),
        "{dialect} {spelling}"
    );
    assert_eq!(
        answers_for(&unit, "::p", head),
        [answer, "declined: not-exact"],
        "{dialect} {spelling}"
    );
    assert_eq!(
        function.sccp.route_tally.implementation, 2,
        "{dialect} {spelling}: both calls enter the route"
    );
    let bare = static_context_for(dialect).commands();
    let unit = CompilationUnit::build_for_dialect(&source, bare, false, dialect);
    assert_eq!(
        value_at(&unit, "::p", "known", 1),
        Some(LatticeValue::Overdefined),
        "{dialect} {spelling}: without the pack the name is nobody's"
    );
}

/// The optimiser forwards each spelling's folded constant (O100) where the
/// body runs and rewrites nothing where it cannot; `tclsh` runs the body's
/// `string cat` exactly where the analysis folds; and with the vendor
/// runtime ([`TENANT_RUNTIME`]) the original and the optimised program
/// print the same under every `tclsh` on `PATH`.
fn the_vendor_runtime_prints_what_the_optimiser_forwards(packs: &tcl_spectcl::PackSet) {
    let program = "proc p {} {\n    set a [tenant::label acme]\n    set b [tenant::tag acme]\n    set c [tenant label acme]\n    puts $a\n    puts $b\n    puts $c\n}\np\n";
    let optimise = |dialect: &str| {
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, packs);
        let profile = resolve_environment(dialect).analyser_profile();
        optimise_source_multipass(program, &registry, Some(profile), PASSES)
    };
    let (rewritten, rewrites) = optimise("tcl9.0");
    assert_eq!(
        rewritten.matches("puts tenant:acme").count(),
        3,
        "{rewritten}\n{rewrites:#?}"
    );
    assert!(
        rewrites
            .iter()
            .any(|rewrite| rewrite.code.as_str() == "O100"),
        "{rewrites:#?}"
    );
    let (unchanged, _) = optimise("tcl8.4");
    assert!(
        !unchanged.contains("puts tenant:acme"),
        "the body cannot run under 8.4:\n{unchanged}"
    );
    for (series, tclsh) in releases_on_path() {
        let body_runs = run_script(&tclsh, "puts [string cat tenant: acme]")
            == Some((true, "tenant:acme\n".to_owned()));
        assert_eq!(
            body_runs,
            matches!(series, "8.6" | "9.0" | "9.1"),
            "tclsh{series} runs the body exactly where the analysis folds"
        );
        let (rewritten, rewrites) = optimise(&dialect_of(series));
        for candidate in [program, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, &format!("{TENANT_RUNTIME}{candidate}")),
                Some((true, "tenant:acme\ntenant:acme\ntenant:acme\n".to_owned())),
                "tclsh{series}:\n{candidate}\n{rewrites:#?}"
            );
        }
    }
}

/// A pack command that writes the variable it names, from that variable's
/// incoming value: `acc::add VAR PIECE` appends `PIECE` to what `VAR`
/// holds. Its `write` needs the variable's prior value as an input
/// (`target 0 incoming`), which the analysis can read only where the
/// lowering records a use of it: the word must carry the `VarWrite` role
/// and the command the `READS_BEFORE_WRITE` trait. `acc::put` declares the
/// same body without the trait, and `acc::store VAR VALUE` writes its
/// argument without reading the variable.
const ACC_PACK: &str = "speclib acc 2.2 {
    command acc::add {
        arity 2
        arg 0 -role VarWrite
        traits {READS_BEFORE_WRITE}
        semantics {
            stores -targets {0} -outcome write
            result -semantic string
        }
        evaluate -implementation acc.add.v1 -host bounded_tcl {
            inputs {target 0 incoming arg 1 exact}
            body {prior piece} { set joined $prior$piece; write 0 $joined; fold $joined }
        }
    }
    command acc::put {
        arity 2
        arg 0 -role VarWrite
        semantics {
            stores -targets {0} -outcome write
            result -semantic string
        }
        evaluate -implementation acc.put.v1 -host bounded_tcl {
            inputs {target 0 incoming arg 1 exact}
            body {prior piece} { set joined $prior$piece; write 0 $joined; fold $joined }
        }
    }
    command acc::store {
        arity 2
        arg 0 -role VarWrite
        semantics {
            stores -targets {0} -outcome write
            result -semantic string
        }
        evaluate -implementation acc.store.v1 -host bounded_tcl {
            inputs {arg 1 exact}
            body {value} { write 0 $value; fold $value }
        }
    }
}
";

/// A pack's `write` through an incoming target reaches the real driver in
/// statement position: `acc::add s cd` over `s` = `ab` leaves `abcd` in the
/// next version of `s`, run by the tclvm host from the loaded pack. In value
/// position neither shape is a value, and neither route runs: the source
/// gate refuses a command with a `VarWrite` role whose writes do not land
/// (the effect-free policy of a substitution) and says why — `[acc::add s
/// ef]` reads `s` before it writes it, a read nothing proves there
/// (`not-exact`), and `[acc::store u xy]` writes storage the substitution
/// cannot own (`stateful-nested`) — and their hosts stay overdefined. Without
/// `READS_BEFORE_WRITE` nothing records the variable's prior value at the
/// call, so the incoming target is not exact and `acc::put` declines.
#[test]
fn a_pack_write_through_an_incoming_target_reaches_the_driver() {
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = pack_workspace("acc", ACC_PACK);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &packs);
    let source = "proc p {} {\n    set s ab\n    acc::add s cd\n    set r [acc::add s ef]\n    \
                  set w [acc::store u xy]\n    return $s$r$w\n}\n\
                  proc q {} {\n    set t ab\n    acc::put t cd\n    return $t\n}\n";
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    assert_eq!(
        value_at(&unit, "::p", "s", 2),
        Some(text("abcd")),
        "the statement's write lands on the next version of `s`"
    );
    for value in ["r", "w"] {
        assert_eq!(
            value_at(&unit, "::p", value, 1),
            Some(LatticeValue::Overdefined),
            "`{value}`: a writing call is never a value in value position"
        );
    }
    assert_eq!(
        answers_for(&unit, "::p", "acc::add"),
        ["evaluated", "declined: not-exact"]
    );
    assert_eq!(
        answers_for(&unit, "::p", "acc::store"),
        ["declined: stateful-nested"]
    );
    assert_eq!(
        value_at(&unit, "::q", "t", 2),
        Some(LatticeValue::Overdefined),
        "no use of `t` is recorded at the call without the trait"
    );
    assert_eq!(
        answers_for(&unit, "::q", "acc::put"),
        ["declined: not-exact"]
    );
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// The no-match preserve (#2051's program): a `regexp` that cannot
/// match leaves its match variables as they were, so the store feeding one
/// stays — no O109 deletes it, no W220 calls it unread, no W210 reports the
/// read — and the original and optimised programs print `before` under
/// every release. The same commands in a condition or a word keep the
/// stores their targets may preserve: `tcl opt` had deleted `set v before`
/// from both, and the optimised programs failed with `can't read "v": no
/// such variable`.
#[test]
fn a_no_match_keeps_the_store_it_preserves() {
    use tcl_compiler::analyser::Analyser;
    let programs = [
        (
            "proc p {} {\n    set a before\n    regexp {(x)(y)} zz a b\n    puts $a\n}\np\n",
            "a",
            "before\n",
        ),
        (
            "proc q {s} {\n    set v before\n    if {[regexp {(x)} $s -> v]} {\n        \
             puts matched\n    }\n    puts $v\n}\nq abc\n",
            "v",
            "before\n",
        ),
        (
            "proc r {} {\n    set v before\n    puts [regexp {x} y v]\n    puts $v\n}\nr\n",
            "v",
            "0\nbefore\n",
        ),
    ];
    for (source, kept, printed) in programs {
        let named = format!("'{kept}'");
        for dialect in DIALECTS {
            let rewrites = rewrites_of(source, dialect);
            assert!(
                !rewrites
                    .iter()
                    .any(|rewrite| rewrite.code == DiagCode::O109),
                "{dialect}: {source}{rewrites:#?}"
            );
            let reported: Vec<(DiagCode, String)> = Analyser::new()
                .analyse(source, dialect)
                .diagnostics
                .into_iter()
                .filter(|d| matches!(d.code, DiagCode::W210 | DiagCode::W220))
                .map(|d| (d.code, d.message))
                .collect();
            assert!(
                !reported.iter().any(|(_, message)| message.contains(&named)),
                "{dialect}: {source}{reported:?}"
            );
        }
        prints_under_every_release(source, printed);
    }
}

/// A pack command's declared preserve is a preserved definition like a
/// builtin's: `keep::miss VAR PIECE` declares `write_or_preserve`
/// on its target and its body preserves it, so the definition holds the
/// version before the call — the undefined root in `p`, the `set` in `q` —
/// which is what W210 reads. The pack spells no trait: the outcome alone
/// makes the target a conditional write.
#[test]
fn a_pack_declared_preserve_holds_the_prior_version() {
    const KEEP_PACK: &str = "speclib keep 2.2 {
    command keep::miss {
        arity 2
        arg 0 -role VarWrite
        semantics {
            stores -targets {0} -outcome write_or_preserve
            result -semantic int
        }
        evaluate -implementation keep.miss.v1 -host bounded_tcl {
            inputs {arg 1 exact}
            body {piece} { preserve 0; fold 0 }
        }
    }
}
";
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = pack_workspace("keep", KEEP_PACK);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &packs);
    let source = "proc p {} {\n    keep::miss v x\n    return $v\n}\n\
                  proc q {} {\n    set v before\n    keep::miss v x\n    return $v\n}\n";
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    for (proc, prior) in [("::p", 0), ("::q", 1)] {
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, proc))
            .expect("the procedure");
        let symbol = function.ssa.var_symbol("v").expect("the variable");
        assert_eq!(
            function.sccp.preserved.get(&(symbol, prior + 1)),
            Some(&prior),
            "{proc}: {:?}",
            function.sccp.preserved
        );
        assert_eq!(answers_for(&unit, proc, "keep::miss"), ["evaluated"]);
    }
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// A word that raises ends the invocation before the command runs, whichever
/// operands its route reads: `review::constant [error boom]` declares
/// `inputs {}`, yet Tcl substitutes the word first and the command never
/// runs, so `catch` is not proven to complete normally — `tcl opt` had
/// rewritten `puts $code` and `puts $message` to `puts 0` and
/// `puts answer`, where every tclsh prints `1` and `boom`. A call whose
/// words complete still folds.
#[test]
fn a_raising_argument_ends_the_invocation_before_its_implementation_runs() {
    const CONSTANT_PACK: &str = "speclib review 2.2 {
    command review::constant {
        arity 1
        semantics {
            effects {no_store_writes no_external_io}
            result -semantic string
        }
        evaluate -implementation review.constant.v1 -host bounded_tcl {
            inputs {}
            depends {tcl_profile implementation_identity}
            body {} {fold answer}
        }
    }
}
";
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = pack_workspace("review", CONSTANT_PACK);
    let dialect = "tcl8.6";
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, &packs);
    let profile = resolve_environment(dialect).analyser_profile();
    let source = "set code [catch {review::constant [error boom]} message]\nputs $code\n\
                  puts $message\nproc p {} {\n    set v start\n    \
                  catch {set v [review::constant [error boom]]} m\n    return \"$v $m\"\n}\n\
                  proc q {} {\n    set w [review::constant abc]\n    return $w\n}\n";
    let (rewritten, rewrites) = optimise_source_multipass(source, &registry, Some(profile), PASSES);
    for kept in ["puts $code", "puts $message", "return \"$v $m\""] {
        assert!(
            rewritten.contains(kept),
            "{kept}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    assert!(rewritten.contains("return answer"), "{rewritten}");
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// A declared route answers only a call its resolved arity accepts:
/// `review::label acme extra` against `arity 1` raises `wrong # args` in
/// every tclsh, so neither the explicit implementation nor the one derived
/// from the reference body runs, and the catch facts stay unproven, where
/// `tcl opt` had rewritten `puts $code` to `puts 0`; an expression route
/// declines the same way. A call the arity accepts still folds.
#[test]
fn a_declared_route_refuses_a_call_outside_its_arity() {
    const EXPLICIT: &str = "speclib review 2.2 {
    command review::label {
        arity 1
        semantics {
            effects {no_store_writes no_external_io}
            result -semantic string
        }
        evaluate -implementation review.label.v1 -host bounded_tcl {
            inputs {arg 0 exact}
            depends {tcl_profile implementation_identity}
            body {name} {fold $name}
        }
    }
}
";
    const DERIVED: &str = "speclib review 2.2 {
    command review::label {
        arity 1
        runtime_backing tcl-body {-pack-text {proc review::label {name} {return $name}} -evaluate}
    }
}
";
    const EXPRESSION: &str = "speclib review 2.2 {
    command review::label {
        arity 1
        evaluate -expression tcl.expr
    }
}
";
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dialect = "tcl8.6";
    for (pack, good, folded) in [
        (EXPLICIT, "review::label acme", text("acme")),
        (DERIVED, "review::label acme", text("acme")),
        (
            EXPRESSION,
            "review::label {1 + 2}",
            LatticeValue::Const(ConstValue::int(3)),
        ),
    ] {
        // The derived pack's one notice is the pack-text advisory.
        let packs = pack_workspace_noting("review", pack);
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, &packs);
        let source = format!(
            "proc p {{}} {{\n    set code [catch {{review::label acme extra}} message]\n    \
             set ok [{good}]\n    return $code$message$ok\n}}\n"
        );
        let unit = CompilationUnit::build_for_dialect(&source, &registry, false, dialect);
        assert!(
            !matches!(
                value_at(&unit, "::p", "code", 1),
                Some(LatticeValue::Const(_))
            ),
            "{pack}: {:?}",
            value_at(&unit, "::p", "code", 1)
        );
        assert_eq!(value_at(&unit, "::p", "ok", 1), Some(folded), "{pack}");
        let profile = resolve_environment(dialect).analyser_profile();
        let (rewritten, _) = optimise_source_multipass(&source, &registry, Some(profile), PASSES);
        assert!(
            rewritten.contains("return $code$message"),
            "{pack}: {rewritten}"
        );
    }
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// A declared implementation's `binding` dependencies are checked at the
/// call: `review::length` declares that its body rests on `string`, so in a
/// module that renames `string` and defines its own — tclsh prints
/// `rebound` — the body's builtin `string length` is not the command the
/// module calls, and the call declines where `tcl opt` had rewritten
/// `puts $x` to `puts 3`. The same call in a module that keeps the builtin
/// still folds.
#[test]
fn a_declared_binding_the_module_rebinds_declines_the_call() {
    const PACK: &str = "speclib review 2.2 {
    command review::length {
        arity 1
        semantics {
            effects {no_store_writes no_external_io}
            result -semantic int
        }
        evaluate -implementation review.length.v1 -host bounded_tcl {
            inputs {arg 0 exact}
            depends {tcl_profile implementation_identity binding string}
            body {value} {fold [string length $value]}
        }
    }
}
";
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dialect = "tcl8.6";
    let packs = pack_workspace("review", PACK);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, &packs);
    let profile = resolve_environment(dialect).analyser_profile();
    let call = "set x [review::length abc]\nputs $x\n";
    let rebound =
        format!("rename string saved_string\nproc string args {{return rebound}}\n{call}");
    let (rewritten, _) = optimise_source_multipass(&rebound, &registry, Some(profile), PASSES);
    assert!(rewritten.contains("puts $x"), "{rewritten}");
    assert!(!rewritten.contains("puts 3"), "{rewritten}");
    let (kept, _) = optimise_source_multipass(call, &registry, Some(profile), PASSES);
    assert!(kept.contains("puts 3"), "{kept}");
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// A pack's `evaluate -direct ID` runs the registry evaluator the id names
/// when it is one a pack may name: `review::list a b` on `ListOfArgs` is
/// `a b` in the shared lattice, so `if {$x eq {a b}}` decides (I230), and
/// the answer is `evaluated` on the direct route. A subcommand scope reads
/// its arguments after the subcommand word: `review pair a b` is `a b` too.
#[test]
fn a_pack_direct_route_runs_the_evaluator_it_names() {
    use tcl_compiler::analyser::Analyser;
    const DIRECT_PACK: &str = "speclib review 2.2 {
    command review::list {
        arity 0..
        semantics {
            effects {no_store_writes no_external_io}
            result -semantic list
        }
        evaluate -direct ListOfArgs
    }
    command review {
        arity 1..
        subcommand pair {
            arity 2
            evaluate -direct ListOfArgs
        }
    }
}
";
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = pack_workspace("review", DIRECT_PACK);
    let dialect = "tcl8.6";
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, &packs);
    let source = "proc p {} {\n    set x [review::list a b]\n    set y [review pair a b]\n    \
                  if {$x eq {a b}} {puts selected} else {puts unreachable}\n    return $x$y\n}\n";
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, dialect);
    assert_eq!(value_at(&unit, "::p", "x", 1), Some(text("a b")));
    assert_eq!(value_at(&unit, "::p", "y", 1), Some(text("a b")));
    assert_eq!(
        answers_for(&unit, "::p", "review::list"),
        ["evaluated"],
        "the direct route runs"
    );
    let decided = Analyser::new()
        .with_pack_overlay(packs.key)
        .analyse(source, dialect)
        .diagnostics
        .into_iter()
        .any(|d| d.code == DiagCode::I230);
    assert!(decided, "the condition decides");
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// A procedure of the module that takes a package command's name replaces
/// it for every caller: `proc base32::encode` and `namespace eval
/// base32 {proc encode …}` print `shadowed` on tclsh 8.5.19 to 9.1.0, so the
/// `base32` route never answers through them — the call folds to its
/// procedure's constant (O103), as before the route — and no condition
/// reading the call is decided (I230). The route still answers where the
/// package is required and nothing shadows it.
#[test]
fn a_shadowing_procedure_stops_a_package_route() {
    let shadows = [
        (
            "namespace eval base32 {}\nproc base32::encode {x} {return shadowed}\nputs [base32::encode abc]\n",
            "shadowed\n",
        ),
        (
            "namespace eval base32 {\n    proc encode {x} {return shadowed}\n}\nputs [base32::encode abc]\nputs [base32::encode xyz]\n",
            "shadowed\nshadowed\n",
        ),
        (
            "namespace eval base32 {}\nproc base32::encode {x} {return shadowed}\nif {[base32::encode abc] eq \"shadowed\"} {puts yes} else {puts no}\n",
            "yes\n",
        ),
    ];
    for (source, printed) in shadows {
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let (rewritten, rewrites) = optimised(source, dialect);
            assert!(
                !rewritten.contains("MFRGG===") && !rewritten.contains("if {0}"),
                "{dialect}: the module's procedure answers\n{rewritten}\n{rewrites:#?}"
            );
            assert!(
                !reports(source, dialect, DiagCode::I230),
                "{dialect}: no condition on the shadowed call is decided"
            );
        }
        prints_under_releases_from(source, printed, "8.5");
    }
    let required = "package require base32\nset h [base32::encode abc]\nputs $h\n";
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let (rewritten, rewrites) = optimised(required, dialect);
        assert!(
            rewritten.contains("puts MFRGG==="),
            "{dialect}: the route answers\n{rewritten}\n{rewrites:#?}"
        );
    }
}

/// A route's decline is final: the interpolation rewriter never takes
/// a word the lexer reads as one `[…]` apart, so `[string match -nocase
/// {a]€} {€a}]`, whose route declines the collation, stays as written where
/// the rewriter had cut it at the `]` inside the braces and emitted a
/// program no tclsh parses. A declined route inside interpolation text
/// leaves the word as written, and an answered one still folds. tclsh 8.4.20
/// to 9.1.0 print the same before and after `tcl opt`.
#[test]
fn a_declined_route_leaves_its_word_as_written() {
    for (source, printed, kept) in [
        ("puts [string match -nocase {a]€} {€a}]\n", "0\n", true),
        ("puts [string match -nocase {ba]😀[€} {€a}]\n", "0\n", true),
        (
            "puts \"x[string match -nocase {a€} {€a}]y\"\n",
            "x0y\n",
            true,
        ),
        ("puts [string match {ba]😀[€} {€a}]\n", "0\n", false),
        ("puts \"x[string match {a*} abc]y\"\n", "x1y\n", false),
    ] {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, rewrites) = optimised(source, dialect);
            if kept {
                assert_eq!(
                    rewritten.lines().next(),
                    source.lines().next(),
                    "{dialect}: the declined word stays\n{rewrites:#?}"
                );
            }
        }
        prints_under_every_release(source, printed);
    }
}
