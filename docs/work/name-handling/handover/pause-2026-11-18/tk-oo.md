# Tk and TclOO area: pause hand-off, 18 November 2026

Session `session_017u5g3KsHnGzMmqgP3e6dJj`. Paused by the owner's instruction, at about 11:55 UTC. The tree is restored: code in flight is only in the patches below, never on `work`.

## (a) Landed

On 17 November, late:

- `4c131697b`: the clay definer check compares names through the naming owner.
- Fixture repairs: `4ba406e95` (`tcloo_navigation` calls the methods that capture callbacks), `b41526348` (commands tests load Tk), `a4f948098` (`namespace code` around the `after` callback), `8c09b5012` (require itcl).
- Scrub ledger: `9948462bc`, `8dcd07467`, `da457343a`, then `ef3eb2f50` for `current-state.md`.

On 18 November:

- `5017cf46a` and `e3fad96eb`: ledger rows for the callback repairs and the uncalled-body forms.
- `3a3dce282`: `tcl-lsp-server/tests/e2e/tcloo_navigation_uncalled.rs`, with the seven uncalled-method variants beside the called ones.
- `b803b3ae9`: widget ledger rows.
- `295474332`: top-level replay starts from the entry's command-world availability. Compiler fixed 2, new 0, gates 39/39.
- Tk widget methods on the typed registered-instance route. These are display tier only, gated on the owning package being loaded, and reviewed by lsp-features before each push:
  - completion: `cdfb73bc6`
  - hover: `6bf405846`, lsp-core fixed 3, new 0
  - semantic tokens: `017d6ccdc`
  - the shared selection is in `tcl-lsp-core/src/registered_instance.rs`
- `4c7c2f54c`: ledger and `current-state.md`.
- `1272d0681`: `docs/work/name-handling/patches/tk-oo-eval-proved-constant-v2.patch`, a reproducer for analyser-binding.
- `8ec2781cc`: ledger rows for the three Tk `command_prefix_integration` tests.

## (b) In flight

### 1. Package-presence rule for instance publication

Patch: `patches/tk-oo-package-presence-rule.patch`, 16 files, +229/-46. It applies cleanly to `work` as of this hand-off.

What it does:

- `AdviceGraph.loaded_packages` is filled by `PackageTransition::Require` with a literal package (`advice_graph.rs`). Branch, `catch` and loop bodies run on a cloned graph whose differences return only as possibilities, so their requires never count. A `namespace eval` body runs once and is adopted whole (`adopt_completed_body_world`), so its require counts.
- `registered_factory` refuses a factory whose `owning_package()` is neither ambient (`context().ambient_package`) nor loaded (`factory_package_present`, `registered_instance.rs`).
- The tape records refused factory call sites in `unloaded_factories`. This covers the named form and the `set lb [listbox .l]` handle form; the handle path now takes `&mut tape`.
- `source_factory_package_unproved_at` (`advice_queries.rs`) and `original_source_unloaded_factory` (`realm/.../class_advice.rs`) gate the presentation map: `analyser/commands/instance_classes.rs` `factory_package_proved`.
- `source_command_publication_at` is deliberately unchanged. It is the syntactic naming fact ("this call names `.t`"), not an instance proof.

Witnesses in the patch (`analyser/commands/tests.rs`) all pass:

- `a_widget_constructor_without_its_package_publishes_no_instance`: named and handle forms, tcl8.6 and tcl9.0.
- `a_widget_constructor_after_an_unproved_require_publishes_no_instance`: `catch`, `if`.
- `a_widget_constructor_publishes_where_tk_is_present`: the `tk` environment, and a require inside `namespace eval`.

tclsh evidence, with Tk built against each reference tclsh and run under Xvfb (see (g)):

| Script | 8.6.18 / 9.0.4 / 9.1.0 |
|---|---|
| `package require Tk; ttk::treeview .t; set lb [listbox .l]` | Treeview, `.l` Listbox |
| require inside `namespace eval ui {...}` | Treeview |
| `wish` with no require | Treeview |
| no require | `invalid command name "ttk::treeview"` |
| `catch {package require Tk}` where Tk is absent | `info commands ttk::treeview` is empty |

Fixture repairs in the same patch: each adds `package require Tk` (or, for the synthetic factories, `package require <factory_package>`):

- `analyser/diagnostics/widget_command.rs`: the `codes` helper, 6 tests.
- `command_binding/source_transition_advice/registered_instance.rs`: 7 tests.
- `.../source_procedure.rs`: 1 test.
- `tcl-lsp-core/src/diagnostic_subject/tests.rs`: 1 test.
- `tcl-lsp-core/tests/it_5/command_prefix_integration.rs`: 3 tests, already red at baseline.

Measurement:

- Baseline at `c1b5d422b`: compiler `--lib --tests` 1890 of 13156 failing; lsp-core `--lib --tests` 639 of 4912.
- With the rule and no repairs (`gate1`): compiler 1904 of 13160 (+4 new witnesses), lsp-core 621 of 4912.
  - New: 14 compiler and 1 lsp-core. Thirteen compiler and the one lsp-core are the Tk fixtures above.
  - The fourteenth, `registry_invocation::vendor_source_barriers::scaling_tests::nested_if_heads_take_barrier_work_linear_in_depth`, fails in isolation with and without the patch (6915 -> 18977 -> 55877 steps). It is not this change; it passed in the baseline run only.
  - lsp-core "fixed" 19 in that run, including iRules object refs and the workspace index. These are unrelated to this change and unverified: do not claim them.
- With the repairs: a targeted run of all 30 affected tests shows every repaired test green, plus `tk_scroll_callback_braced_widget_path_does_not_fire_w123`, which was red at baseline. Still red, and also red at baseline: `optional_named_factory_source_requires_its_present_original_operand`, `original_registered_factory_candidate_keys_delegate_jim_comparison_without_publication`, `tk_scale_command_bareword_head_is_a_callback_edge`, `tk_scroll_callback_bareword_undefined_head_fires_w123`.
- The full re-measurement with repairs (`gate2`) was killed at 1260 of 13160 for the pause. Not comparable.

What remains:

1. A full compiler and lsp-core run, against a fresh baseline on the then-current `work`. New must be 0 apart from the known flaky scaling test.
2. clippy for tcl-compiler and tcl-lsp-core, and `make xtask-gates`.
3. Review by analyser-binding (review only).
4. Push the rule and its repairs in one commit.
5. Send each owning session its rows (FIX-FIXTURE, repaired, with the tclsh evidence). The `widget_command`, advice and `diagnostic_subject` tests turned red only by the rule have no ledger row yet; add them with the push.
6. Write up the wider principle (all package-owned commands' registry facts) under `outstanding/`, as ruled, without building it.

### 2. `eval` of a proved constant (#1655)

Patch: `patches/tk-oo-eval-proved-constant.patch`, the same as `docs/work/name-handling/patches/tk-oo-eval-proved-constant-v2.patch`.

`original_delegated_script.rs` accepts `WordExpr::Variable` when `written_name_values[written_origin]` is current and equals the argument. It adds two it_4 `walk_blockers` tests. The TP passes; the negative fails on its second prefix:

```tcl
proc f {} {global test; set test {proc M {} {}}}
set test {set x 1}
f
eval $test
```

This is followed by the guarded class. tclsh 8.6.18, 9.0.4 and 9.1.0 give `procs=::M class=0`. The patch wrongly records `::M` as a class: `::test`'s contents epoch does not move across `f`.

analyser-binding owns that walk fault, accepted it, and is using the patch as its reproducer. Do not land (A) before their fix; then re-run both tests. After (A) comes the clay `foreach {flag test} {...} {... [eval $test]}` case (a proved finite set, walked member by member).

### 3. Widget-constructor completion declarations

Not started; there is no patch. analyser-binding asked for Normal + Error completion on the Tk constructors (walk Unknown routes block 14 Tk replay rows and 6 `callback_replay` tests). They then asked me to hold until they name the exact spec field and an existing example; that has not arrived.

Evidence gathered (scripts `ctor.tcl`, `ctor2.tcl`, `ctor3.tcl`, reproduced in (g)):

- Completion is Normal or Error. The validatecommand's break, return and error give constructor catch code 0. Errors seen: `window name "e" already exists in parent`, `unknown option "-nosuch"`.
- Synchronous scripts do run:
  - `entry` and `spinbox` with `-validate all -validatecommand S -textvariable v`, in either order, run S during creation; a proc S defines exists afterwards.
  - A read trace on `-textvariable` fires.
  - ttk::entry, ttk::combobox and ttk::spinbox (same options), scale, ttk::scale and radiobutton do not run anything synchronously.
  - Scroll commands run only after `update`.

The audit session proposed one fact family for Tk callback timing to analyser-binding. I agreed not to add a second vocabulary.

## (c) Queue, in order

1. Package-presence rule: finish (b)1. It is a ruling with a fixed landing shape, nearly done, and other areas' Tk rows depend on it.
2. Constructor completion declarations, once analyser-binding names the field. This unblocks 14 Tk replay rows and 6 `callback_replay` tests in other areas.
3. (A) eval of a proved constant, after analyser-binding's stale-global fix; then the clay `foreach` case (#1655).
4. Open ledger rows in `tk-oo.tsv`, in this order:
   - `uncalled-body-generic-entry` (7) and `uncalled-procedure-facts` in `references::tests::callback_targets` (9), which wait on the generic-entry ruling's implementation;
   - `tk_checks` declared-provides (3);
   - Tk ensemble splice (definition and hover).

## (d) Waiting on, and waited on by

Waiting on:

- analyser-binding: the stale-global walk fix (call-site contents epoch for a body writing a linked global); the completion-route spec field and an example; review of the package rule.
- The owner or coordinator: resume.

Others waiting on me:

- taint and audit: the constructor completion declarations.
- lsp-features: compiler-side package gating (#2324 follow-up) is on my list, which (b)1 delivers.
- The coordinator: the package-rule commit and its fixed/new counts.

## (e) Rulings not yet carried out

- Package-presence rule (coordinator, 18 November 09:07, `test-scrub/README.md`). A registered-instance publication counts only where the factory's owning package is ambient in the environment, or loaded by an unconditional `package require` (or another proved load, including through a followed `source` edge) earlier on the walk.
  - A conditional or caught require is not a load.
  - The call is not a proved error: do not mark later statements unreached.
  - W120 and its fix are unchanged.
  - Scope is `CREATES_WIDGET` and the registry naming factories only. The general principle goes in `outstanding/`.
  - It lands with the repair of every fixture it turns red, in one push, after analyser-binding's review, then gates, then fixed/new to the coordinator.
  - Not yet checked: a require proved through a followed `source` edge. The advice graph's `source_point.rs` clones the graph per sourced file; confirm whether `loaded_packages` comes back.
- #1655 eval-of-a-proved-constant walk (coordinator ruling, `current-state.md` line ~103): parked as above.

## (f) Last measured counts

- At `c1b5d422b` (base for the package rule): tcl-compiler `--lib --tests` 1890 failing of 13156; tcl-lsp-core `--lib --tests` 639 failing of 4912.
- Command (Jim oracle required), run from `rust/`:

  ```sh
  source scripts/dev/agent-build-env.sh
  export CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
    TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 \
    TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1 CARGO_BUILD_JOBS=2
  cargo nextest run -p tcl-compiler --lib --tests --no-fail-fast
  cargo nextest run -p tcl-lsp-core --lib --tests --no-fail-fast
  ```

  Failing list: `grep -E '^\s+FAIL' log | sed -E 's/.*\) //' | sort -u`.
- tcl-lsp-server e2e was not measured this session. Ledger rows 2–37 (`tcloo_navigation*`) are open or repaired by their last e2e run, not re-checked.

## (g) What a successor would get wrong

- Do not edit sources while a measurement is compiling. The lsp-core half rebuilds tcl-compiler from the tree. One baseline here had to be redone, and the orphaned `cargo-nextest` process survived `pkill -f "cargo nextest"` (the binary is `cargo-nextest`).
- Disk fills from `target/debug/incremental` and `/tmp/tcl-spectcl-cache-test-*`. Delete incremental sessions older than ~90 minutes, and the cache directories.
- Tk is built in `tmp/tk{8.6.18,9.0.4,9.1.0}/unix` (`--disable-xft --disable-xss`; 9.1 needs `libharfbuzz-dev` and `libxft-dev`, which are installed). `package require Tk` needs a pkgIndex. Use a directory holding `package ifneeded Tk <v> [list load <unix>/libtk8.6.so|libtcl9tk9.x.so Tk]`, then:

  ```sh
  TCLLIBPATH=<dir> xvfb-run -a env -u TCL_LIBRARY TK_LIBRARY=tmp/tk<v>/library tmp/tcl<v>/unix/tclsh script.tcl
  ```

  For wish, set `LD_LIBRARY_PATH` to the Tk unix directory. These builds live in `tmp/` and do not survive a fresh container.
- The lsp-core display tier gate is `DocumentFloor::advises_package`, which does not check order. The compiler rule does (earlier on the walk). Keep them consistent if either changes.
- `instance_classes` is presentation metadata, but taint reads `LocalInstanceClasses`. The proof fact is `source_registered_instance_words_at`.
- lsp-features must review every display-tier change in `tcl-lsp-core` before push. Completion, hover and tokens must stay off rename, references and edits.
- `nested_if_heads_take_barrier_work_linear_in_depth` is red on `work` in isolation, whatever the baseline run says.
