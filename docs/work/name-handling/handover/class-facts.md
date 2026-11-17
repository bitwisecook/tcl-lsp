# Hand-over: class and object facts

Outgoing session `session_017u5g3JzuPiZKPQt1QkNjnk`, 17 November 2026. Paths are on the restructured tree unless marked.

## Scope

Class, object and method facts in `tcl-compiler`:

- Class factories: `tests/it_4` `analyser::class_factories::*`, including the tcllib clay corpus test.
- Class definition scripts (`oo::class create C {…}`, `oo::define C {…}`): the definers bind, ordinary commands resolve as in the enclosing namespace.
- Method bodies of TclOO, snit and itcl classes: read-before-set, class-declared variables as one place per class, callers and callback selectors inside bodies the analysis never entered.
- Named instances and class cards (`instance_classes`, `OriginalProjectClassCards`).
- The lenient `tcl` environment's display facts.
- Registry widget constructors as receivers (`[listbox .l] curselection`).

Not this area: the execution walk itself (the `command_binding.rs` facade and `command_binding/source_walk/`, owned by analyser-binding), the diagnostic emitters (diagnostics), lowering and SSA (dataflow-optimiser). This area did edit small spots in each; they are listed under Held work.

## How to measure

```bash
source scripts/dev/agent-build-env.sh
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0
cargo nextest run -p tcl-compiler --no-run            # cold: about 25 minutes
cargo nextest run -p tcl-compiler --no-fail-fast --status-level fail --final-status-level none --failure-output never > run.log 2>&1   # about 11 minutes
grep -E "^\s+(FAIL|TIMEOUT|SIGABRT|SIGSEGV|ABORT)" run.log | sed -E 's/^\s+\S+ \[[^]]*\] *(\([ 0-9/]+\) *)?//' | sort -u > run.fail
comm -23 base.fail run.fail   # fixed
comm -13 base.fail run.fail   # new
```

Run each as a background command with a two-hour timeout. A rebuild after a library edit is about 11 minutes, so a change costs about 22 minutes to measure. Since the file split a test path gained a module, so compare the list below with a new run by last path segment. No test was excluded; the suite has no 1,200-second test.

Fast iteration: a second target directory with `CARGO_TARGET_DIR=<dir> CARGO_INCREMENTAL=1 CARGO_PROFILE_TEST_OPT_LEVEL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0` and one small integration test rebuilds in 30 to 60 seconds (cold: about 3.5 minutes). The probe this session used, as an untracked `rust/tcl-compiler/tests/zz_probe.rs` (list it in `.git/info/exclude`, and run `rustfmt` on it or `cargo fmt --all -- --check` fails):

```rust
use tcl_compiler::analyser::Analyser;

#[test]
fn probe() {
    let Ok(path) = std::env::var("TCL_PROBE") else {
        return;
    };
    let src = std::fs::read_to_string(path).unwrap();
    let dialects = std::env::var("TCL_PROBE_DIALECTS").unwrap_or_else(|_| "tcl8.6".into());
    for dialect in dialects.split(',') {
        let r = Analyser::new().analyse(&src, dialect);
        println!("=== {dialect}");
        for d in &r.diagnostics {
            println!("diag {} {:?} {}", d.code, d.span, d.message);
        }
        let mut classes: Vec<_> = r.all_classes.iter().collect();
        classes.sort_by_key(|(k, _)| (*k).clone());
        for (name, c) in classes {
            let mut methods: Vec<_> = c.methods.keys().collect();
            methods.sort();
            println!(
                "class {name} supers={:?} unknown={} methods={methods:?} factory={}",
                c.superclasses,
                c.inheritance_unknown,
                c.factory.is_some()
            );
        }
        let mut inst: Vec<_> = r.instance_classes.iter().collect();
        inst.sort();
        println!("instances {inst:?}");
        match tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards::capture(&r) {
            None => println!("cards None"),
            Some(cards) => {
                let rows: Vec<_> = cards
                    .named_instance_display_reports()
                    .into_iter()
                    .map(|(label, class)| (label, class.qualified_name.clone()))
                    .collect();
                println!("cards {rows:?}");
            }
        }
        if std::env::var("TCL_PROBE_IR").is_ok() {
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            let cu =
                tcl_compiler::compilation_unit::CompilationUnit::build_for(&src, registry, false);
            for m in cu.ir_module.methods.values() {
                println!("method {} ivars={:?}", m.reporting_label(), m.instance_vars);
                for st in &m.body.statements {
                    let d = format!("{st:?}");
                    println!("   {}", &d[..d.len().min(400)]);
                    if let Some(t) = st.tokens() {
                        println!(
                            "   accesses {:?}",
                            t.variable_accesses
                                .iter()
                                .map(|a| a.original_spelling.clone())
                                .collect::<Vec<_>>()
                        );
                    }
                }
            }
        }
        let mut procs: Vec<_> = r.all_procs.keys().collect();
        procs.sort();
        println!("procs {procs:?}");
    }
}
```

Run it with `TCL_PROBE=<file.tcl> TCL_PROBE_DIALECTS=tcl8.6,tcl <binary> --nocapture`; `TCL_PROBE_IR=1` adds the method IR and recorded variable reads. Useful traces on a debug build (they are `debug_assertions` or `cfg(test)` code): `TCL_LSP_TRACE_BODY_SYNTAX` (per command: proved targets, `unknown`, `absent`), `TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE` (the walk's world per statement, `unclosed=`), `TCL_LSP_TRACE_JIM_SOURCE_OWNER` (constructor and presence queries), `TCL_LSP_TRACE_RBS_SOURCE_OWNER` (read-before-set).

`tcl-lsp-core` was never measured by this session.

## Current numbers

| Commit | Date | Run | Failing | `class_factories` failing |
| --- | --- | --- | --- | --- |
| `c8099ccc` (start) | 16 November | 13,123 | 2,355 | 50 |
| `a2df29e1` (first batch pushed, pre-restructure) | 16 November | 13,129 | 2,247 | 48 |

No run on the restructured tree.

## Baseline list

`docs/work/name-handling/lists/class-facts-failing-2026-11-16.txt`: the 2,247 failing tests of the whole `tcl-compiler` suite at `a2df29e1`, one per line, pre-restructure paths.

## Queue

1. **Re-apply and land the held second batch** (Held work, patch 1). It needs its hunks re-applied by hand on the restructured tree, then a full run, clippy and the gates. Expected against the list: fixed `lowering::tests::extracts_oo_class_methods`, `analyser::diagnostics::tests::method_body_read_before_set_now_flags_w210`, and `class_factories::{a_bare_name_inside_the_metaclass_namespace_does_resolve, an_unreadable_cross_file_prologue_marks_inheritance_unknown, the_cross_file_result_matches_the_same_file_result, the_per_item_walk_agrees_with_the_whole_file_walk_cross_file}`; new none (check `objdefine_method_body_unbound_read_now_flags_w210` in particular).
2. **Every command after `oo::define` is unknown.** Fixture (tcl8.6, 9.0): `oo::class create C {method a {} {}}` / `oo::define C {method b {} {return 1}}` / `C create obj` / `obj nosuch` / `puts hi`. After the `oo::define` every head reads `targets=[] unknown=true` and no W308 is drawn; tclsh 9.0.4 raises `unknown method "nosuch": must be a, b, c or destroy` and changes nothing but the class. Cause: `oo::class create` completes through `register_bounded_methods` and `record_bounded_class`, and an object-layer definer through `walk_closed_own_object_method_configuration` (both called from `prepare_native_definition`, `command_binding/source_walk/native_definition.rs`), but a class-layer `Configure` has no closed route, so its script is entered as an ordinary body where `method` fails the compile preflight. Patch 2 is a first route. Still to do after it: keep the class definition instead of dropping it (the `Configure` arm of `apply_declared_binding_transitions` removes `class_definitions[identity]`), so the instance stays typed. The LSP area expects this to be the widest cause in the `tcloo_navigation` end-to-end group.
3. **A relative head in a TclOO method body.** `oo::class create C {method m {} {return $neverBound}}; C create c; c m` reports nothing; tclsh 8.6.18 and 9.0.4 raise `can't read "neverBound"`. `build_method_units` (`compilation_unit/body_units.rs`) builds a `trivial_guarded` unit when the body's namespace is runtime-selected and `requires_runtime_command_namespace` (`ir_helpers/nested_bodies.rs`) sees any relative head. With a known command world an uncalled body already proves such heads (`list` and `return` read `::list` and `::return` in the trace) through `original_receiver_command_lookup` (`command_binding/source_name_lookup.rs`); the guard does not read that proof. Question open with analyser-binding (see Open questions).
4. **A class made by a user metaclass publishes no factory.** `class_factories::a_computed_metaclass_name_resolves_through_a_literal_call_site` (second assertion), the `a_replayed_creation_*` and `a_post_pass_*` tests. `user_metaclass_of_class` needs a relation provider on each `superclass` word; `original_class_definition_body_in_analysis` (`analyser/handlers/classes/class_definitions.rs`) only owns a definition body under a registry definer's grammar, so the body of `Meta create X {superclass Meta}` has no owner and the relation has no provider. Needs a definition-body owner derived from the constructor call and the manufacturer layout.
5. **Cross-file consumers** after patch 1 and the namespace hand-off below: `a_workspace_indexed_metaclass_creates_real_classes`, `a_metaclass_file_publishes_exactly_one_factory`, `unknown_dispatch_*`. With `namespace eval ::tk {}` written first in `META_FILE` (an experiment, not a fixture change) patch 1 turned the consumer tests green, so the remaining cause is the hand-off.
6. Not started: `[listbox .l] curselection` (no recorded invocation for the substituted head), `package require Tk` under a logical input, `clay_inventory_refuses_computed_expanded_and_replaced_definers`, the clay corpus test, `namespace eval` bodies under the logical policy (the first batch adopts them for the native policy only), `tcl-lsp-core` measurements.

## Held work

Both patches are against the **pre-restructure** layout and do not apply to the restructured tree (a cherry-pick of the first conflicts in six files); re-apply the hunks by hand in the modules named here.

1. `docs/work/name-handling/patches/handover-class-facts-batch2.patch`: `git format-patch` of `6a149988` on `a2df29e1`. Contents and where each hunk now belongs:
   - `seed_declared_receiver_variables` in `command_binding/receiver_self.rs`, called from `seed_deferred_inputs` (`command_binding/deferred_entries.rs`): in an uncalled method, constructor or destructor body the class's declared variables become cells of the declaring class with unknown contents.
   - `written_object_storage` beside `written_namespace_storage` (`analyser/diagnostics/dataflow/dead_stores.rs`), read by the W220 visibility check and the W211 loop.
   - `extract_one_member` (`lowering/declarations/class_members.rs`): a member with no body word no longer sets `unretained_executable_roots`.
   - New module `analyser/workspace_class_factory.rs` and its call in `resolve_class_creation` (`analyser/handlers/classes/class_factories.rs`); `foreign_factory_at_call` becomes `pub(super)`.
   - `method_body_read_before_set_now_flags_w210` names its release (`codes_for_in(src, "tcl9.0")`).
   - `it_4` test `analyser::method_bodies::a_class_variable_is_object_state_in_every_body_of_its_class`, and a class-facts section for `current-state.md` (superseded by this file).

   Verified: a full run on `a2df29e1` of an earlier form gave fixed 5, new 1 (`objdefine_method_body_unbound_read_now_flags_w210`). The patch then changed three things that no run has covered: bodies with no single class definition are left untouched (the fix for that regression), the workspace lookup moved into its own module, and the fixture names its release. The new `it_4` test passed on a debug build before those edits. No clippy, no gates.
2. `docs/work/name-handling/patches/handover-class-facts-oo-define.patch`: `git diff` from `6a149988` to `92922906`, so it applies after patch 1 on the pre-restructure layout. New module `command_binding/closed_class_configuration.rs`, two helpers made `pub(super)` and `configuration_target(facts, layer)` in `command_binding/own_object_configuration.rs`, and the route in `prepare_native_definition`. It compiles on a debug build. It was never probed or tested: whether the command world stays closed after `transfer_native_definition` is the first thing to check with the fixture of queue item 2.

Pushed and not held: `a2df29e1` and its three parents (namespace-body classes, display labels, `instance_classes` under `tcl`, their tests).

## Open questions and rulings

- Unanswered, to analyser-binding: may an uncalled receiver method resolve relative heads through the fresh-receiver path (object namespace holding only `my`, then `::oo::Helpers`, then `::`) in the end-of-load world, refused when the document can put a command into a receiver namespace? The "world after load" decision of 21 October excluded receiver methods.
- Working under: the display and proof policy of `fixing-guide.md`. This session applied it once on its own judgement and reported it to the coordinator without objection: `named_instance_display_reports` reads `labels_complete` (every unread operation is a construction of, or a call on, a source class of the catalogue), while class cards keep the strict `complete`.
- Working under: `a_dynamic_metaclass_name_still_abstains` is honoured as written. The workspace lookup refuses a head that is not a written word, although tclsh would dispatch `$meta create …`.
- Working under: no Rust file over 2,000 lines; new code goes in a named module.

## Hand-offs

Sent to analyser-binding (`session_017u5g3J3rxgBLfx8ijRxPu9`), none answered:

- TclOO creation under a namespace that does not exist withdraws the command world. `oo::class create ::nope::X {method m {} {return 1}}` then `oo::class create Y {}`: the second `oo::class` is unknown. tclsh 8.6.18, 9.0.4, 9.1.0: the creation succeeds and `namespace exists ::nope` turns 1 (also `::nope::X create ::other::deep::obj`, `oo::object create ::o3::obj`, `X create rel::obj`), where `proc ::p4::p {} {}` raises `unknown namespace`. Refused in `selected_class_publication` (`command_binding/command_publication.rs`) through `publication_key_at`, and in `OriginalNamedManufacture::capture` (`command_binding/named_manufacture.rs`).
- `next` in an entered TclOO method body draws W123. `oo::class create B {method m {} {return 1}}` / `oo::class create D {superclass B; method m {} {next}}` / `D create d` / `d m`. The presence query (`diagnostic_command_presence`, `command_binding/command_presence.rs`) looks the head up from `::` without `::oo::Helpers`. Keeps `class_factories::a_provably_direct_body_keeps_method_checks_live` failing.

Sent as notices: diagnostics (the `written_object_storage` change and the fixture), dataflow-optimiser (the `extract_one_member` fix, and that `definer_invocation_may_supply_body` answers true for every head with no registry candidate, so one call to a user procedure flags the whole module).

Received and not started: none.

## Hazards

- `origin/work` is force-updated and the clone is shallow, so a plain rebase onto it replays unrelated history. Move `work` to the fetched head and cherry-pick your own commits.
- `origin/work` did not compile once after a fetch (two sessions' commits met in `tcl-registry`). Check that the tree builds before measuring, and fetch again before fixing: another session pushed the same fix first.
- Do not edit the checkout while a suite is compiling from it. Use a second worktree with its own target directory for experiments.
- The format check reads untracked test files too.
- A debug build with three jobs beside another build restarted the container once. Two jobs each held.
- Disk: 11 GB were left with one optimised target, one debug target and the xtask build; `target/debug/incremental` held 5.7 GB of stale data.
- Never write a document through an unquoted shell here-document: backticked text is executed. This session did that once while writing this file; nothing was changed, but it could have been.
- The profile-less `Analyser::new()` fixtures (`codes_for`) prove nothing about a release. `method_body_read_before_set_now_flags_w210` looked like a code failure and was partly a fixture one.
- A fallback that widens "to be safe" can remove a true diagnostic: setting the whole activation unknown for a body with no class definition silenced the `oo::objdefine` read-before-set test.
- `c_source_namespace_class_after_unentered_definition_body_keeps_unknown_mutation` pinned an unknown mutation after a members-only class body; that expectation was wrong for tclsh and was changed in the first batch. Its `uplevel` row is right and stays.

## Oracle habits

`tclsh8.4`, `tclsh8.5`, `tclsh8.6` (8.6.18), `tclsh9.0` (9.0.4) and `tclsh9.1` (9.1.0) are in `/root/.local/bin`; there is no `jimsh`. Write the fixture to a file and run it under each release in a loop. For a question about what a command leaves behind, wrap each step in `catch` and print `info commands`, `namespace exists` and the message, one line per release, and put the script's result in the test comment. For TclOO only 8.6 to 9.1 apply.
