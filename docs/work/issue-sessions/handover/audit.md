# Hand-over: the `audit` issue session (#1712, #1710)

Written 18 November 2026 by the `audit` session (session_017u5g3MNKJTTq44wzXNszit), paused on the owner's instruction. It replaces the 17 November hand-over. A fresh session in a clean container should be able to continue from this file alone.

## Issues

### #1712 — closed (completed)

Typed completeness for every facts domain, and purity derived from it.

- Closing comment 6972855523. Correction comment 6972933688: the F5 device check was renumbered from 32 to 33, because 32 was already the `lassign` check.
- The commits are listed under *On `work`* below. In order: `Knowledge<T>` for side effects, world effects, completion, result stability, state transitions, callbacks (derived from world effects) and taint; purity derived from the four domains, with `Traits::PURE` retired; C-evidence backfill; generated coverage matrix.
- **F5 ruling (parent, "Gap A", option (a)):** the 36 core builtins that are pure at Tcl 8.4 but have no F5 evidence stay impure under the F5 dialects. They are listed in `docs/generated/semantics-coverage.md`, section "Pure under Tcl 8.4, not evidenced for F5". The device check that would settle them is **check 33** in `docs/work/name-handling/bigip-name-checks.md`. Measured cost to iRules users: the sweep of the iRules corpus was identical before and after (5,432 firings both times; control probe identical). Hover, code actions, codegen/inlining and the explorer were not measured.
- **"Gap B" ruling:** state transitions, taint and callbacks are wrapped in `Knowledge`, landed in two stages (see *How to measure*).

### #1710 — open

The issue's acceptance list has nine items:

| # | Item | State | Commits / where |
|---|---|---|---|
| 1 | Corpus in `docs/CORPUS.md` | done (before this session) | triage comment 6894828097 |
| 2 | Audit Tk 8.4–9.1 command, option, subcommand, method, lifecycle, hover, trait, taint and side-effect facts | options and methods done: classic options `d5a61981`, ttk options and methods `b85f7ffe` + `8457c455`, classic methods `5c7d01a84f`. Not done: a per-row audit of lifecycle, hover, trait and side-effect facts; nobody has asked for one | comments 6899695906, 6900451162 |
| 3 | Per-command facts in `tcl-registry`; consumers resolve generically | done | |
| 4 | Executable argument timing and variable links in CommandSpec; validation/event substitutions as user input | registry side done (`bf127e07`, comment 6897380736). The compiler-side replay is the **open callback-replay fault** below | |
| 5 | Geometry conflicts, missing parents, unsupported options | done (registry `tk_geometry`, before this session) | |
| 6 | Static `TkUiModel` across LSP, MCP, VS Code, JetBrains | done (before this session) | |
| 7 | SpecTcl and Spec Studio flow for every new property | done (`ffc6af66`, #1714) | |
| 8 | Tk AI guidance: `[list]` callbacks, validation, DPI-aware layout, preview limits | done: `3c904cf697` (`ai/claude/skills/_prompts/tk_system.md`, "Validation" and "DPI-aware, responsive layout") | |
| 9 | Tests; regenerate references; measure idle and callback-bearing cost | tests and regeneration done with each commit. **Cost measurement must be redone** (see below) | comment 6903504605 has the first, invalid table |

The issue closes when the six `callback_replay` tests and `taint`'s replay rows pass on `work`, and the cost is re-measured and posted.

## The callback-replay fault

### The failing tests

These six `tcl-compiler` lib tests are this session's (ledger `docs/work/test-scrub/ledger/taint.tsv`, cause `callback-replay-no-warnings`):

- `taint::tests::callback_replay::callback_list_replay_uses_the_selected_captured_builder_prefix`
- `…::callback_list_replay_uses_the_selected_static_expansion_owner`
- `…::callback_replays_batch_into_one_extra_interprocedural_build`
- `…::callback_replay_batches_past_the_first_batch_without_truncating_later_sinks`
- `…::callback_replay_name_search_is_single_pass_under_collision_stress`
- `…::command_substitution_callbacks_stop_before_replay_when_original_owner_is_refused`

The `taint` session has two replay-side fixes parked behind this fault: (a) build the replay unit under `cu.ir_module.dialect_profile`, and (b) `is_actual_formal_entry` refusing the seed for an uncalled replay proc. They also have an `entry_after` fix. `taint` measured that (a)+(b) alone change nothing, and that they don't touch candidate collection.

### Diagnosis (corrected)

`collect_callback_replay_candidates` (`rust/tcl-compiler/src/taint/callback_replay.rs`, owned by `taint`) resolves each registering statement in the original module. After `entry .one -validatecommand X`, the next statement no longer resolves: it gets `<caller-frame-opaque>`, so there is one candidate where there should be two.

Probe: a temporary test, reverted and never committed, under `tcl8.6` with the `loaded_tk_callback_unit` fixture (Tk 8.6.13 provider). It printed each statement's resolution and `(source_callback_retention, requires_world_barrier)`:

| first statement | `entry .two -validatecommand {eval %S}` |
|---|---|
| `entry .one -validatecommand {set x 1}` | unresolved |
| `entry .one -validatecommand {eval %P}` | unresolved |
| `entry .one -validatecommand {puts hi}` | unresolved |
| `entry .one -validatecommand [list puts hi]` | unresolved |
| `entry .one` | resolves |
| `label .one` | resolves |
| `after 10 {eval $x}` | resolves |
| `after 10 {puts hi}` | resolves |

`entry .one` has the same facts with or without the option: retention `Unproven`, barrier `true`. `after` is `MayRetain`/`true`. So the guard inputs don't separate the cases. An earlier report blamed `requires_reentry_guard`'s retention clause (`command_binding/source_walk/completion_receipts.rs:299`); that was **wrong** and has been corrected with the parent.

What triggers the withholding is a **Body-valued option word on the constructor**. `after` declares `body_execution: Some(DeferredGlobalScript)` (`tcl/after_.rs`), so its flow is `ScriptBodyFlow::Deferred` and it doesn't withhold. `entry` declares no `body_execution`. Only `target_walk.rs` calls `withhold_command_world`; the exact branch is for `analyser-binding` to name.

Real Tk 9.0.3 (tkinter under xvfb; script `rust/tcl-registry/tests/data/tk_callback_dispatch/reg.tcl` on `issues/audit-22`):

```
tk 9.0.3 after-one=1 two=1 three-vcmd after-three
```

So `entry .one -validate all -validatecommand {…; rename entry {}}` does not run the callback at creation; `entry` survives and `entry .two` works. `entry .three -textvariable v -validate all -validatecommand {…}`, with `v` existing, runs it synchronously during creation.

### Parent's soundness ruling

Do not ship the relaxation alone. There must be two facts:

1. **Registration time:** per callback surface, whether registering can run the callback synchronously. This depends on the arguments.
2. **Dispatch time:** per command, whether it can run already-retained callbacks synchronously, and whose.

A command with no fact keeps today's withholding while a callback is outstanding: Unknown is not "does not dispatch". The walk ignores retention at a registration only when the surface is deferred under the invocation's arguments, and withholds at every dispatch point while a callback is pending.

Five witnesses, each checked against wish:
1. `entry .one -vcmd {…}; entry .two …` resolves both.
2. With `update` between them, `.two` is withheld.
3. With `.one insert 0 x` between them (and `-validate key`), withheld.
4. With `destroy .one` after `bind .one <Destroy> {rename entry {}}`, withheld.
5. The `-textvariable` forced-validation case is withheld at creation (a positive witness, not a note).

### Coordinator's conditions

1. One fact family, agreed with `analyser-binding` (session_017u5g3HAPLcJWWX93pMpk32, owns the walk and decides the shape) and `tk-oo` (session_017u5g3KsHnGzMmqgP3e6dJj) first.
2. Then this session writes the walk half as an **announced commit that `analyser-binding` reviews before push**. Tell `irules-registry-tooling` (session_017u5g3GuvSUJA9Qanj2AVvk) what registry data is added.
3. Acceptance:
   - the six tests;
   - `taint`'s three-line repro and its 14 Tk-replay rows, re-run (tell `taint`, session_017u5g3HGxE6CxLNW1DgfVr9, when it lands);
   - full `tcl-compiler` and `tcl-lsp-core` with 0 new failures;
   - all 39 gates;
   - `analyser-binding`'s review.
4. Order: `tk-oo`'s constructor completion routes (Normal|Error) land first, then this session's registry rows, then the walk.

### Agreed shape (`analyser-binding`, 11:31–11:43)

- **Completion** (`tk-oo`'s): constructor `InvocationCompletion` routes are Normal|Error only.
- **Registration** = the existing body-execution axis (`body_execution.rs` `BodyExecutionSpec` → `script_body_flow.rs` `ScriptBodyFlow`). Add one variant, e.g. `DeferredGlobalScriptUnlessForced(&'static ForcedEntrySpec)`, on classic `entry` and `spinbox` only. Forcing set: `-textvariable` present AND `-validate` ∉ {none}. Forcing provably absent → `Deferred`; present or undecidable → `Unknown(bodies)`, so the walk withholds at creation. `ttk::entry`, `button`, `after`, `bind` and `trace` stay plain Deferred. Other widget constructors with Body options should get plain `DeferredGlobalScript`.
- **Dispatch** = option (B): `RetainedDispatch` is its **own `Knowledge`-typed field** on `CommandSpec` / `SubCommand` (and `CommandForm` if needed), not inside `CallbackEffect`. A stated `CallbackEffect` would widen what `gvn`, `const_subst` and `effect_ssa` believe, and constructors with linked-variable options do fire traces. Reader rule: Unknown → `AllPending`. The doc comment must say it asserts nothing about callbacks run through traces (those stay with `world_effects` / the variable-observer model).
  - `AllPending`: `update`, `update idletasks`, `vwait`, `tkwait` (all forms).
  - `TargetWindow`: `event generate` (no `-when`; mapped is assumed); `destroy`; receiver methods `entry`/`spinbox` `insert`/`delete`/`validate` (declared unconditionally); `invoke` on button, checkbutton, radiobutton, spinbox, `ttk::button` and other ttk buttons, and `menu`; `ttk::entry insert`.
  - `None`: `after cancel`/`info`, `focus -force`, `grab`, `scale set`, `scrollbar set`, `listbox insert`, `configure` (see the exception below), `winfo`, `wm`, `bind` query.
- **Variable writes**: forced validation on a later `set v y` is the `-textvariable` link acting as a write observer. It belongs in the variable-observer modelling, not `RetainedDispatch`. `checkbutton -variable` installs no command-running observer.
- Evidence goes under `rust/tcl-registry/tests/data/tk_callback_dispatch/`, cited from each row.

### Questions still open with `analyser-binding` (unanswered at pause)

1. **Option database** (raised 11:46). The Tk option database can supply both forcing options, so "absent from argv" does not prove Deferred. In wish 8.4.20, 8.6.18 and 9.1.0 (`optdb.tcl` and `registration.tcl` on `issues/audit-22`):
   - `option add *Entry.textVariable v; option add *Entry.validate all; entry .e -validatecommand vcmd` runs vcmd at creation;
   - `-textvar v -validate al` (abbreviated) runs it;
   - an explicit `-validate none` in argv overrides the database and stays deferred.

   On X11 the database is also seeded from the user's X resources. Options offered:
   - (i) strict: Deferred only for literal `-validate` none-prefix or empty `-textvariable`, so the six tests stay red;
   - (ii) the walk models the option database as world state: `option add` / `option readfile` sites that could match make the selection Unknown, and external X resources are a documented assumption (this session's lean);
   - (iii) ignore the database as an explicit assumption.
2. **Default for unstated rows** (raised 11:52). With Unknown → AllPending, every Tcl-core, tcllib and unstated Tk command becomes a withholding point while a callback is pending. That regresses `after 10 {…}; set x 1; entry .two`, which resolves today. C audit (Tcl and Tk 8.6.18 and 9.1.0, `generic/` + `unix/`): the only command implementations reaching `Tcl_DoOneEvent` / `Tcl_ServiceAll` / `TclServiceIdle` / `Tcl_ServiceEvent` are:
   - `Tcl_UpdateObjCmd`, `Tcl_VwaitObjCmd`, `Tk_UpdateObjCmd`, `Tk_TkwaitObjCmd`;
   - `TkSelGetSelection` (`selection get`, `clipboard get` on X11, which wait with `Tcl_DoOneEvent(0)` when another client owns the selection);
   - `EatGrabEvents` (grab) and `TkScrollWindow` (canvas/text scrolling). Both run under a restrict proc that only discards or defers, so no user callbacks run, which matches wish.

   Proposal: state None as Complete {8.4 9.1} on Tcl-core and Tk rows outside that list, guarded by a test that holds the C list. Alternative (B'): a group default for Tcl and Tk. `analyser-binding` to choose.

### `tk-oo`'s evidence (8.6.18 / 9.0.4 / 9.1.0, their scratch `tkw/ctor*.tcl`)

- `entry`/`spinbox` run `-vcmd` at creation only with `-textvariable` (either order).
- `ttk::entry`, `ttk::combobox`, `ttk::spinbox`, `scale`/`ttk::scale -command` and `radiobutton -command` don't.
- A `-vcmd` returning break, return or error leaves the constructor's catch code at 0.
- A proc defined by the `-vcmd` exists after the constructor returns.
- A read trace on `-textvariable` fires during creation.
- Scroll commands run only after `update`.

`tk-oo` will add no per-option timing vocabulary of its own.

### Wish evidence gathered (all on `issues/audit-22`)

Scripts and per-release outputs (`<script>.tk<version>.txt`) are under `rust/tcl-registry/tests/data/tk_callback_dispatch/`, captured with Tk 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Results are identical across releases unless noted; ttk is 8.5+.

**Registration** (`registration.tcl`, `dispatch_points.tcl`). The callback runs at creation only for classic `entry`/`spinbox` with an existing `-textvariable` and `-validate` ∈ {all, key, focus}, whether those come from argv, from abbreviations, or from the option database. Everything else is deferred:
- an unset `-textvariable`, or `-validate none`;
- `ttk::entry`/`ttk::combobox`;
- `scale`, `ttk::scale`, `checkbutton`, `radiobutton` and `spinbox` `-command`, with or without their variable set;
- `listbox -yscrollcommand` with `-listvariable`; `entry -xscrollcommand`;
- `button -command`, `after`, `after idle`, `bind`, `trace add`.

**Dispatch: runs before return** (`dispatch_points*.tcl`, `after_and_waits.tcl`, `menu_invoke.tcl`, `configure_receiver.tcl`):
- `update`; `update idletasks` (idle callbacks only, not timers); `vwait`; `tkwait variable`/`window`/`visibility`;
- `event generate` to a mapped window (virtual and `<Key>`);
- `destroy` (`<Destroy>`);
- `entry`/`spinbox` `insert`/`delete` under `-validate key`, and `.e validate`; `ttk::entry insert` under `-validate key`;
- `invoke` on button, checkbutton, radiobutton, spinbox, `ttk::button` and `menu`. On 8.x a menu's entry 0 is the default tearoff entry; Tk 9.0 changed the X11 default to no tearoff;
- `trace add variable` read/write/unset firing;
- classic `entry` `-textvariable` write under `-validate all`;
- **`configure -textvariable` on classic `entry` and `spinbox` with validation on, and `scale configure -variable`, run the widget's own callback.** So `configure` is Receiver on entry, spinbox and scale, and None elsewhere.

**Dispatch: does not run before return:**
- `update idletasks` for timers;
- `after ms`, `after ms script`, `after idle script`, `after cancel`, `after info`;
- `event generate -when tail`, or to an unmapped window;
- `focus -force`, `grab`;
- `scale set` (runs at the next idle), `scrollbar set`, `listbox insert` with `-yscrollcommand`;
- `checkbutton -variable` write (no `-command`); `ttk::entry` `-textvariable` write;
- `selection get` / `clipboard get` when this application owns the selection;
- `entry configure -validate all`, `cget`;
- 70 forms in `no_dispatch.tcl` with a timer, an idle callback and a `<Configure>` binding pending: all 36 classic and ttk widget constructors; `pack`, `pack forget`, `grid`, `place`; `bind` set/query, `bindtags`; `bell`, `clipboard`, `event add`/`info`; `focus`; `font create`/`measure`; `image create`; `option add`; `raise`, `lower`; `selection clear`; `winfo`; `wm title`/`geometry`; `tk scaling`/`appname`; `ttk::style configure`; `configure`/`cget`; plain `insert`/`create`.

**Still to gather:**
- the remaining widget subcommands (anything not listed above is Unknown);
- `tk_*` dialogs and `tk_popup` (they run local event loops through `vwait`/`tkwait` in their library scripts; that should be stated from `library/*.tcl`);
- `send`.

### Work in progress (`issues/audit-22`, tip `279875ba60`; not reviewed, not built after the rows)

- `rust/tcl-registry/src/retained_dispatch.rs`:
  - `RetainedDispatch {None, TargetWindow(DispatchTarget), AllPending}` and `DispatchTarget {Receiver, Argument(u8), ArgumentsFrom(u8)}`. Argument indices are post-command, so they include the subcommand word;
  - `RetainedDispatch::read` (Unknown → AllPending);
  - constants `EVIDENCED` (Tcl 8.4 onward), `NONE`, `ALL_PENDING`, `RECEIVER`.
- The field `retained_dispatch: Knowledge<RetainedDispatch>` on `CommandSpec` and `SubCommand` (default Unknown), resolved in `operation_resolution.rs`. A selected subcommand uses only its own fact and never borrows the command's, so a constructor's None can't leak into `.b invoke`. It is exposed as `InvocationFacts::retained_dispatch`.
- `closed_evidence.rs` states None (Complete, at the closed windows) on every C-evidenced closed row, since a closed row runs no script.
- Rows:
  - `update`, `vwait`, `tkwait` and its subcommands: AllPending;
  - `destroy`: `TargetWindow(ArgumentsFrom(0))`;
  - `event generate`: `TargetWindow(Argument(1))`; `event add`/`delete`/`info`: None;
  - `selection get`, `clipboard get`: AllPending; `selection clear`/`own`, `clipboard append`/`clear`: None;
  - `after` and its `cancel`/`idle`/`info` subcommands: None.
- `tcl-registry` type-checked before the rows were added; the rows themselves are unchecked.
- Not started:
  - the SpecTcl row and Spec Studio parity (follow `299f39fbcc`, the taint field: `spec.rs`, `spec/subcommand.rs`, `spec/defaults.rs`, `tcl-spectcl/src/loader/command/effect_rows.rs` + `body_statements.rs` + `subcommand.rs`, the studio's `coverage`/`help`/`relations`/`render_rs`/`render_spectcl`/`schema`/`examples`/`draft`, `docs/references/command-spec/fields.md`, `docs/design/spec-dsl-examples/README.md`, goldens);
  - the matrix column in `xtask/src/semantics_matrix.rs`;
  - the widget rows; `ForcedEntrySpec`; the walk.

## Callback cost measurement

The table in comment 6903504605 is invalid: the sink rows produced no warnings. To re-measure, the six tests and `taint`'s replay rows must pass first. Then time `taint_warning_report_for_cu` (base plus replay) against the base pass on Tk 8.6 fixtures of 8, 32 and 128 statements, in three shapes (idle, benign `-validatecommand {set value %P}`, sink `{eval %P}`), averaged over 3 runs in the dev profile, and post the table on #1710.

## On `work` (this session's commits, oldest first)

| SHA | Change |
|---|---|
| c19768337d | `Knowledge` for world effects |
| a6c54c6f65 | `Knowledge` for completion |
| 51a9b7ffdb | `Knowledge` for result stability |
| ea1198999b | namespace qualifiers/tail look no namespace up |
| aaf1c7159e | C evidence of what each core command's implementation can touch |
| 2ae49f1b0b | close the purity domains of evidenced rows |
| 509e237900 | derive purity from the four domains; retire `Traits::PURE` |
| bfaba16a69 | iRules `event … disable` effects stated Unknown |
| b022e509e8 | generated purity coverage matrix |
| 0511d6e439 | regenerate value-transfers after a moved fold site |
| bcf4bd77fb | list the core builtins pure at 8.4 but not evidenced for F5 (check 33) |
| e2fa614556 | `Knowledge` for state transitions |
| 0215dd44d3 | callbacks known through the world effects that carry them |
| 299f39fbcc | `Knowledge` for taint |
| 576480a5cb | renumber the TMM core-builtins check to 33 |
| 3c904cf697 | Tk AI guidance: validation and DPI-aware layout (#1710 item 8) |
| 5c7d01a84f | Tk classic widget methods follow each release's wish (#1710 item 2) |

Earlier #1710 commits from the previous holder: `bf127e07`, `5f97b4d7`, `d5a61981`, `b85f7ffe`, `8457c455`, `d9f49ecc`, `ffc6af66`.

## Only on branches

- `issues/audit-22` (`279875ba60`): **the only branch with unlanded work** (the WIP above). Keep it.
- Safe to delete. Every other commit on these is an earlier pre-fixup copy of a commit since landed under the same title, which is why `git cherry` shows them as unique: `issues/audit-2` … `issues/audit-21`.
- `issues/audit` (`1f44162812`) is the previous holder's branch. Its 210 commits predate the 17 November re-import (`8d12353794`), so they don't match by patch. Spot-check before deleting.
- The parent will put the deletions to the owner; don't delete them yourself.

## How to measure

- Environment: `source scripts/dev/agent-build-env.sh` (target dir `/home/user/tcl-lsp/target`), then `export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0`. Dev/test profile only, never `--release`. One cargo build at a time, and no source edits while one runs.
- Tests: `cargo nextest run … --no-fail-fast --status-level fail --final-status-level none --failure-output never`, skipping `-E 'not test(/codegen_stamps::every_shape/)'`. Registry group: `-p tcl-registry -p tcl-spectcl -p tcl-spec-studio -p tcl-cshim`.
- Tk-related workspace filter used for #1710: `-E 'test(/tk|widget|canvas|menu|panedwindow|text_widget|wish|callback_replay/) & not package(tcl-registry)'`. At `5c7d01a84f`: 300 run, 39 failing, the same 39 as before it. Their owners, routed by the parent:
  - `tk_gate_*`: `analysis-performance.tsv`;
  - the `tk_checks` provides tests: `tk-oo`;
  - `tk_ensemble_configure_splice`: `tk-oo` / `lsp-features`;
  - `command_prefix` `tk_scale`/`tk_scroll`: `tk-oo`, fix the fixtures;
  - the six `callback_replay` tests: this session.
- Baseline comparison: run the same filter with the change reverted (`git checkout HEAD~1 -- <files>`, run, `git checkout HEAD -- <files>`), then `diff` the sorted FAIL lists.
- Before pushing to `work`:
  - `cargo check --workspace --all-targets`;
  - `cargo clippy --workspace --all-targets -- -D warnings -A clippy::assert_is_empty`;
  - `make xtask-gates` (39 gates);
  - zero new failures.
  - After registry changes, regenerate `cargo xtask value-transfers`, `semantics-matrix`, `callback-inventory` and `pack-goldens` as the gates demand.
- **Two-stage landing (parent's ruling):** stage 1, one full baseline run plus one tip run. Stage 2, rebase; if clean, regenerate, check, clippy, gates and the purity groups, push at once, and verify with `git merge-base --is-ancestor <sha> origin/work`. Land on `work` only by fast-forward after rebasing onto `origin/work`; never force-push. Keep WIP on new `issues/audit-N` names (force-push is denied).
- **wish builds:**
  ```bash
  for v in 8.4.20 8.5.19 8.6.18 9.0.4 9.1.0; do
    mkdir -p tk$v && (cd tk$v && \
      /home/user/tcl-lsp/tmp/tk$v/unix/configure --with-tcl=/home/user/tcl-lsp/tmp/tcl$v/unix --disable-shared && \
      make -j4 wish && rm -f *.o)
  done
  ```
  Run this in a scratch dir. It is about 36 MB in total; Tk 9.1 needs `apt-get install libharfbuzz-dev libxft-dev libfontconfig1-dev`. Run each case as `TCL_LIBRARY=/home/user/tcl-lsp/tmp/tcl$v/library TK_LIBRARY=/home/user/tcl-lsp/tmp/tk$v/library xvfb-run -a ./wish script.tcl`. Tcl/Tk sources come from the session-start hook (`tmp/tcl*`, `tmp/tk*`); tclsh 8.4–9.1 are in `~/.local/bin`.
- Classic method tables: `scripts/dev/tk_classic_methods.tcl` under each wish → `rust/tcl-registry/tests/data/tk_option_tables/tk<v>.classic-methods.txt`. Test `classic_widget_methods_match_each_releases_wish`.

## Rulings in force

- The 36 core builtins stay impure under F5 until device check 33 is run.
- Purity comes only from Complete domains; Unknown is never "pure" or "effect-free".
- Callback replay: registration-time and dispatch-time facts, with Unknown keeping today's withholding, as set out above. Shape: (B), agreed with `analyser-binding`. Order: `tk-oo` completion, then registry rows, then the walk under `analyser-binding`'s review.
- Don't touch `taint/callback_replay.rs` or `taint/propagation.rs` (`taint`'s), or `command_binding/` without `analyser-binding`'s review.
- Tell consumer sessions before pushing changes to registry fields or traits they read:
  - values (session_017u5g3KpPQPTbtTswiHYkaw);
  - dataflow-optimiser (session_017u5g3MFyobSx3WBcgjea2s);
  - irules-registry-tooling (session_017u5g3GuvSUJA9Qanj2AVvk).
- Tests name their Tcl releases (8.4–9.1). Never weaken an assertion, never `#[ignore]`, no `#[allow]`. One cause per commit, with evidence. F5 behaviour that needs a device goes into a numbered check, never a guess.

## Dead ends

- Blaming `requires_reentry_guard`'s retention clause for the withholding: wrong, because plain `entry .one` has identical facts.
- Blaming #1796 for the `taint::tests` failures: wrong (comment 6949734041).
- Assuming forced validation is decided by argv alone: the option database and abbreviations both force it.
- Mapping `scale set` / `scrollbar set` to Receiver: wish shows they don't dispatch before returning.
- Reading menu `invoke 0` on 8.x as deferred: entry 0 is the tearoff.

## Next steps, in order

1. Get `analyser-binding`'s answers on the option database and the unstated-row default.
2. Wait for `tk-oo`'s constructor completion routes to land.
3. Finish the registry half on top of `issues/audit-22`:
   - the SpecTcl/Studio parity for `retained_dispatch`;
   - the widget rows (constructors None; receiver methods; `configure` Receiver on entry, spinbox and scale);
   - the None coverage agreed in step 1, with a test holding the C notifier list;
   - plain `DeferredGlobalScript` on widget constructors with Body options, and `DeferredGlobalScriptUnlessForced` + `ForcedEntrySpec` on `entry`/`spinbox`;
   - a test per witness against the evidence files.

   Tell `irules-registry-tooling`, values and dataflow-optimiser before pushing. Land it with the two-stage ruling.
4. Write the walk half in `target_walk.rs`: Deferred registration doesn't withhold; withhold at dispatch points whose scope covers a pending callback, and at Unknown(bodies) registrations. Announce it, get `analyser-binding`'s review, then land.
5. Re-run the six tests, `taint`'s repro and its 14 rows, plus full `tcl-compiler` and `tcl-lsp-core`; tell `taint`.
6. Re-measure the callback cost, post it on #1710, and close #1710.

## Hazards

- **Disk.** The session allowance is ~39 GB and `target/debug/build` grows past 20 GB across differing builds. Prefer `CARGO_INCREMENTAL=0`. On ENOSPC, delete `target/debug/incremental`, `target/debug/examples`, and stale `target/debug/build/*/<hash>` dirs no current build is using (deleting live ones forces a rebuild).
- **`/tmp/tcl-spectcl-cache-test-*` leak.** `tcl-spectcl` tests leave ~7 MB per run; 1,105 of them (~8 GB) filled the disk. Delete them when no test is running. It is with the coordinator for its owner.
- **`cargo fmt --all`** reformats upstream files and sweeps them into your commit. Run `rustfmt --edition 2024 <file>` on files you edited only.
- `git reset --hard` and force-push are denied in this environment. To squash, use `git reset --soft` + `commit`.
- Stop hooks nag about unpushed/uncommitted work. Push WIP to a fresh `issues/audit-N` rather than to `work`.
