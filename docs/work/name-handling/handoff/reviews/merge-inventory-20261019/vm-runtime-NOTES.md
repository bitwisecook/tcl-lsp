# vm-runtime: Sol-48 verification and SpecTcl execution (21–22 Oct)

## Scope and setup

Branch `work`, starting at `54256b96a3`. I made no commits myself. The coordinator's WIP
snapshots (about every 30 minutes, for example `6afd791002`) committed my edits as they
stood. Every change is described below. `vm-runtime.patch` is the diff of the paths I
touched against `54256b96a3`.

Owned paths:
- rust/tcl-vm
- rust/tcl-vm-cli
- rust/tcl-vm-wasm
- rust/tcl-engine-tclvm
- rust/tcl-cmd-core
- runtime/rust
- rust/tcl-spectcl

At the coordinator's direction I also edited:
- `docs/design/spec-dsl-examples/*.tclspec` (the SpecTcl example ports);
- their README;
- two `rust/tcl-spec-studio` tests. That crate has no owner.

Run environment:
- Every run: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
  TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`.
- SpecTcl targets also: `TCL_LSP_TCLSH90=tclsh9.0 TCL_REQUIRE_SPECTCL_COMPAT=1`, as in
  `make test-spectcl-compat`.
- One test binary at a time; each binary was deleted after its run.
- Free disk ranged from 0.7 to 2.8 GB. Builds waited for ≥ 2.5 GB, and later for ≥ 2 GB,
  as directed.
- The load average was about 8 on 4 CPUs.
- tcl-compiler and tcl-registry were rebuilt by other agents throughout, so each result
  includes their working tree at that moment.

## 1. Changes

### 1.1 SpecTcl static fast path folds braced backslash-newline (`rust/tcl-spectcl/src/loader/eval.rs`)

**Symptom.** Validation row 9a: eval_loader's all-shipped-packs selector failed on
`specs/eda_microchip.tclspec` `add_probe_insertion_point`'s `example`. The static fast route
kept the `\`-newline inside the braced word. The interpreter route folded it to one space.

**Root cause.** `drive_static` pushed CST rows whose braced-word text is the raw bytes
between the braces. A braced word's Tcl *value* folds each backslash-newline, together with
the following whitespace, to one space at any brace depth.

`54256b96a3` did not leak raw text. It changes only the interpreter route, which was already
correct. The old `add_library` line failure had masked this divergence, because the
snapshot comparison reports only its first difference.

**Fix.** A new `static_row()`:
- A static row whose braced word changes under `BraceBackslashNewline::Folds` is captured
  from its folded values through `capture_stmt`.
- That is exactly what the interpreter route does: `VerbatimIndex::row` never matches a CST
  row whose text differs from the value.
- Bodies are still re-read from the verbatim text, so line numbers stay those of the
  original source.
- It applies to the `default` arm and the catch-all row arm.

**C Tcl evidence.** I ran `command x {hover {example {add_probe_insertion_point -net
{count_out_c[0]} \<nl>    -driver {D} -pin {H5}}}}` through a `hover` proc that prints the
`example` value. tclsh 8.4.20, 8.5.19, 8.6, 9.0.4 and 9.1.0 all print
`…{count_out_c[0]}  -driver …` (the continuation folded to one space).

### 1.2 codegen_stamps test harness: `PlainCounting` forwards every compile entry (`rust/tcl-spectcl/tests/codegen_stamps.rs`)

`PlainCounting` wraps the default `BytecodeCompileService`. It implemented only the older
trait methods, so the entry-aware and byte-ingress methods the VM now calls fell back to the
trait defaults:
- some refused with "CompileService does not expose entry-aware byte parsing" (4 tests);
- some silently compiled plain (`compile_script_with_entry` defaults to
  `compile_plain_script_for_profile`), which corrupted the plain-compile count.

The wrapper now forwards every remaining method to the inner service:
- `compile_script_with_entry`
- `compile_procedure_with_entry`
- `compile_substitution_with_entry`
- `compile_script_bytes_*`
- `compile_plain_script_bytes_*`
- `compile_procedure_bytes_*`
- `script_command_plan_bytes_*`
- `compile_failure_for_profile`

It counts the plain variants (and Plain procedure dispatch) and forges optimised modules,
exactly as the existing methods do. No assertion changed.

### 1.3 tcl-engine-tclvm: the wall-clock budget test's doc comment (`rust/tcl-engine-tclvm/src/lib.rs`)

Sol-48's commands-budget change charges inline-compiled commands at their start, as C's
`INST_START_CMD` does. The doc comment on `the_wall_clock_budget_stops_a_loop_the_command_budget_cannot_see`
said the command budget never fires on an inlined loop, which is no longer true.

The comment now says the inlined commands do count, but a budget sized for real work is far
from spent when the 50 ms wall clock fires. The test name and its assertions are unchanged.

### 1.4 tcl-vm `package` version conflict uses the protocol's release (`rust/tcl-vm/src/cmd_package.rs`)

Flagged by the owner-resolution gate. `version_conflict` spelled the requirement with
`vm.runtime_version()`. It now takes the `release` of the selected C package protocol, which
both callers already hold:
- `pkg_require` gets it from `NativePackageProtocol::C(release)`;
- `after_package_unknown` gets it from `PackageRequire.release`.

So the message now uses the same release that validated the requirement.

No test was added, because there is no cheap fixture:
- `Vm::set_runtime_version` re-pins the dialect profile, so the protocol release and the
  runtime version always move together.
- The only divergence is an authored package provider
  (`Vm::set_logical_package_provider`). That needs an actual engine profile and the
  authored-8.4 naming policy.

### 1.5 runtime/rust: redundant `#[path]` attributes removed (owner-resolution gate)

Each attribute named the default location of a child module of a non-`mod.rs` file, and no
`dir/mod.rs` twin exists. The gate skips modules with non-plain attributes, so these hid
owner registrations from it.

Removed:
- `native_body_artifact.rs`: `native_dictionary_scope` plus the nine other non-test top-level
  ones (`native_dictionary`, `native_dictionary_mutation`, `native_error`, `native_named`,
  `native_switch`, `native_try`, `native_lappend`, `native_oo_next`,
  `native_control_preparation`);
- `value_ops.rs:51` (`native_jim_cat`);
- `native_variable_names.rs:13` (`native_unset`).

The `#[cfg(test)]` modules' attributes and the unflagged ones are untouched. There is no
runtime change. **Not yet compile-checked**: the runtime build is the next step after the pause.

### 1.6 SpecTcl example ports and goldens

- **`oo-class.tclspec`, ported to the current vocabulary** (coordinator decision). The pack
  had stopped installing `oo::class` at all (`commands 0`). The reason: its `world_effects`
  rows `callback` / `dynamic_fallback`, and its `state_transitions.resolver -native …` ids,
  became semantic-class "not loadable" since `9d4f72634f`.
  - A pack cannot author those facts:
    - `world_effects` loads only `composition`;
    - `STATE_TRANSITION_RESOLVER_NATIVE` is empty;
    - a resolver body states variable-cell aliases only (README, "What a pack cannot
      author").
  - The port therefore drops the pack-level `descriptor world_effects class-factory-effects`,
    the three `world_effects` references and the three `state_transitions` blocks. It keeps
    the legacy `side_effect InterpState -writes` / `defines_command_at` facts. A header
    comment says why.
  - Writing an empty descriptor, or a resolver-less transitions block, would claim "no
    transitions" for `oo::class create foo`, which is false.
- **`upvar.tclspec`.**
  - `frame_effect -level-word Upvar`, replacing `ArityParity`. The dialect-aware
    `FrameLevelWord::Upvar` came in with `944d156922`, and the DSL accepts it.
  - tclsh evidence: `proc p {} {catch {upvar 1 b} m; …}` gives "wrong # args" in 8.4.20 and
    8.5.19, and aliases in 8.6, 9.0 and 9.1, so `ArityParity` misread 8.4/8.5. The comments
    and README line 1288 are updated to match.
  - The hover description is now the shipped text verbatim.
- **`lsort.tclspec`**: `arity 1 -positionals`, `reserved_trailing_words 1` and
  `script_lookup_scope InvokingFrame`, matching `lsort_.rs`.
- **`switch.tclspec`**: `arity 3.. -step 2 -also 2 -positionals`.
- **`foreach.tclspec`**: `representation_effect {CoerceOrdinaryListPairs 0}`.
- **`return.tclspec`**: `reserved_trailing_words 1`. `side_effect InterpState -writes` is
  dropped, because the shipped spec moved that write into a typed `world_effects` access
  (`944d156922`) that a pack cannot author. The iRules `EventControl` row stays.
- **`string.tclspec`**: `is` gains `refine value { arity 2; side_effects none; world_effects
  none; state_transitions {}; result_stability ReferentiallyTransparent; traits {PURE
  CSE_CANDIDATE} }`. This is the shipped two-word, pure form, which has no `-failindex`.

Goldens were regenerated twice. Each time I used a temporary probe that runs exactly
`golden::render(evaluate_pack(source))` → `golden::golden_path` over
`golden::shipped_packs`, which is what `rust/xtask/src/pack_goldens.rs` does. I did not
run `cargo xtask pack-goldens` itself, because of disk; CI's golden check is the backstop.
The orphan check found 25 goldens and 25 packs, matched by name.

**Pass 1** (the coordinator accepted every class):
- **1367 command lines are digest-only.** I dropped the CommandSpec fields added since the
  golden commit `ca698d87c8` and restored the old field order (CommandSpec went from 125
  to 143 fields, and 37 structs changed). The xxh3 of that rendering reproduces the
  recorded digest exactly. The struct changes come from the `spec.rs` commits on `work`:
  `944d156922`, `9d4f72634f`, `4d727e4fe1`, `6c7b06f97f`, `b9fd2b9ac0`, `660d2858bc`,
  `fd63ff1d26`, `22572db4c6`, `4a9d6c2ae6`, `a1332643cb`, `a3e3d7949c`, `60500bb166`,
  `191284784c`, `fe7fe4d3a5`, `ab88e9f0df`, `f1e36d8d60`, `5a8dfde489`, `a3c6e6856b`,
  `3fd8d08e4a`.
- **148 command lines are the fold of §1.1** (eda_microchip 50, eda_xilinx 58, upf 40).
  They are exactly the set of commands with a backslash-newline inside a braced row word.
- **foreach**: the notice moves from line 49 to 52. 52 is the physical `lowering_hook` row;
  the old golden was wrong.
- **oo-class**: the port changes listed above. Registrations go 3→2, notices 9→0,
  `commands 1`.
- The per-pack table is in `vm-runtime/gold/table.md`, and the scripts are
  `vm-runtime/gold/*.py`.

**Pass 2, after the port pass.** Only foreach, lsort, return, string, switch and upvar
changed, and only in their intended spec digests plus the line shifts of the comment rows
I added. lsort's header moved from `speclib tcl 1.1` to `2.1`, because `-positionals` (2.1)
and `script_lookup_scope` (2.0) drew "newer vocabulary" notices under 1.1. Its spec digest
is identical with and without the bump.

### 1.8 Thread-exit abort (priority bug from the lsp agent)

**Symptom.** A tokio worker exits and its `pack_hooks` `HOST` thread-local drops
`HookHost<TclVmEngine>`. That drops the `Vm` and its `Value`s, and
`native_retirement::release` calls `PENDING.with` on the already-destroyed retirement
queue. The resulting panic inside a destructor aborts the process (SIGABRT). It killed
the tcl-lsp-server lib test process after about 221 tests.

**Fix.** Every thread-local with a destructor that a drop path touches now uses
`LocalKey::try_with`:
- `native_retirement::release`: if the queue is gone, the detached header is leaked
  (`mem::forget`), because dropping it would recurse into dead TLS. `RetirementDrain` and
  the pop loop are fallible in the same way.
- `native_jim_pool` `ScopeOwner::drop` and `construct` (`ENTERED`).
- `DoubleFormatContext::for_dialect` (`DOUBLE_PRECISIONS`), which falls back to a fresh
  cell at the release default.

Thread-locals that are const `Cell`s without a destructor (`DRAINING`, tcl_syntax number
`SYNTAX`/`PRINT`) are always accessible, so they were left alone.

**Regression test.** `rust/tcl-vm/tests/thread_exit_retirement.rs`, 2/2.
- A thread-local `RefCell<Option<Vm>>` is registered before the VM's own thread-locals, so
  it drops after them. It holds a 64-deep native list.
- A control test checks that a VM dropped while the thread lives still retires its values.
- **Negative control:** with only `release` reverted to `PENDING.with`, the binary aborts
  with exactly the reported "cannot access a Thread Local Storage value during or after
  destruction: AccessError" (exit 134).

### 1.9 UPF hook coverage (review C.2)

New test `rust/tcl-spectcl/tests/upf_hooks.rs`, 2/2. It takes all 17 executable bodies of
`specs/upf.tclspec` through the production plan (`hooks::plan_for` over bundled `specs/`)
and runs them on the production `tclvm_host` with exact expected answers:
- **15 option-arity hooks with an optional `TRUE|FALSE` value**:
  - TRUE/FALSE → `consume 1`;
  - "maybe", a following `-option`, or the end of the words → `consume 0`.
- **`-feedthrough`, which takes an optional name**: a name → `consume 1`; `-x` or the end →
  `consume 0`.
- **The `define_power_model` arg-role resolver**: it marks the trailing word as Body (the
  1- and 3-index cases), and abstains for a one-word call.

The test also asserts that there are exactly 16+1 bodies and that none is declined.

### 1.7 tcl-spec-studio tests (no owner; coordinator direction)

- **`tests/pack_store.rs`.** I re-derived the expectation from the run: all 12 examples now
  take the splice path. `oo-class` was the one exception, because carry-forward reclaimed its
  `world_effects class-factory-effects` reference. That names a pack-level descriptor which
  `PackStore::accepts`' isolated-block check cannot see. The descriptor is gone, so the
  assertion is now `examples().len()` and the comment says why.
- **`tests/spectcl_ports.rs`.** Every remaining field the DSL cannot express is a documented
  `unequal` / `unequal_subcommand` row, with a shared reason constant:
  - `OO_CLASS_FACTORY_DESCRIPTORS`: oo::class create/new/createWithNamespace
    `__unrenderable`, the shipped `world_effects`/`state_transitions`.
  - `NATIVE_COMPILATION`: the shipped `native_compilation` recipe (lsort, switch, oo::class
    `__unrenderable`). No DSL surface exists.
  - `SUCCESSFUL_HANDLER`: shipped `successful_handler`, on foreach, if, switch, string
    length/map/range and HTTP::header value/values. The DSL recognises it but excludes it
    from pack-authored strong analysis (README), so authoring it would exclude the command.
  - string `map` `__unrenderable`: the same completion and dispatch descriptors already
    documented for `length`.
  - HTTP::header now compares each of its 14 subcommands field by field
    (`subcommand_subset` lists them all). Only the two `successful_handler` rows are
    excused, rather than the whole `subcommands` key.

## 2. Verification

| Target | Before | After | Notes |
|---|---|---|---|
| eval_loader | 24/28 (Sol-48); 27/1/28 (validation, c4fa23c53d) | **28/28** | §1.1 |
| golden_packs | 1/2/3 (validation) | **3/3** | regenerated goldens, §1.6 |
| pack_source_e2e | 3/6 | 3/6 | compiler-side, §3; same 3 tests |
| spec_corpus | not run | 2/5 | the 3 failures are compiler-side or a watchdog budget verdict, §3 |
| pack_is_real_tcl | not run | **1/1** | |
| codegen_stamps | 1/28 | 5 ok / 23 failed / 1 capped at 1500 s | +4 from §1.2, no regressions; the rest are compiler-side |
| upf_hooks (new) | — | **2/2** | §1.9 |
| tcl-spec-studio spectcl_ports | 10/12 | **12/12** | §1.7 |
| tcl-spec-studio pack_store | 6/7 | **7/7** | §1.7 |
| tcl-vm thread_exit_retirement (new) | — | **2/2** | §1.8 |
| jim_document | — | 9/17/26 | no baseline; Jim class analysis, not yet triaged |

Not run, because I was paused at 13:09Z:
- jim_own_surface, jim_surface_roster, jim_generation_zero, jim_seed_after_publish;
- eda_loadables, eda_environment_packs;
- runtime/rust, both configurations;
- tcl-vm lib (Sol-48 baseline 1107/1387, per-test statuses in
  `vm-runtime/sol48-baseline/`);
- tcl-engine-tclvm lib, embed_api_e2e;
- clippy.

The per-test statuses of every run are in `vm-runtime/runs/`.


## 3. Compiler-side root causes (reported to the coordinator)

Each item was reported to the coordinator, who routed it.

1. **Default compile refuses everything.** `BytecodeCompileService::default().compile(src)`
   returns `NativeCompilationAdmission(NativePreflightRequired)` for every source, even
   `set x 1`. codegen_stamps' `prepare()` hits this.
2. **Whole-module compile loses procedures and pack specialisation.** `compile_for_profile`,
   on both the plain and the overlay service, emits no procedures or procedure provenance
   and binds nothing. So:
   - pack alias/codegen_hook stamps produce no `BuiltinAlias` claim;
   - `const_fold` produces no `PackFacts` claim;
   - nothing is inlined.

   That is 16 codegen_stamps tests at "original compiled fixture procedure". The raw
   lowering pipeline gives `ProviderRequired`, and the VM refuses it.
3. **Pack const_fold hooks never run.** The optimiser (`optimise_raw`) never invokes
   installed pack const_fold hooks: no O129, with empty host error and crash logs. This
   breaks pack_source_e2e's fold test and spec_corpus's hostile pack ("0/0 attempted").
4. **The engine sandbox refuses some string bodies.** It refuses `string cat` and
   `string is` with `NativePreflightRequired` on the entry-aware route, while
   `string length`, `string match`, `lindex` and `expr` work. A plain
   `compile_procedure_for_profile` of the same bodies passes. This breaks pack_source_e2e's
   reference and derived body tests.
5. **Shared analysis misses I230.** spec_corpus `shared_unit_matches_legacy…`: for tcl8.6
   `if {$value}`, legacy gives I230 and shared gives nothing.
6. **codegen_stamps matrix test is too slow.** `every_shape_of_inlined_body…` takes more
   than 25 min, because every `compile_script_with_entry` re-assembles `ContextRegistry`.
   This is lowering cost.
7. **N17**: codegen `$a:b` variable grammar → wrong registration.
8. **`install.rs` native_compilation for pack rows**: compiler-codegen's change, which most
   remaining codegen_stamps failures wait on.


## 4. Not done / remaining

- Everything listed under "Not run" in §2. The Resume section of
  `docs/work/name-handling/outstanding/vm-runtime.md` gives the exact commands.
- Triage of the `jim_document` failures (`vm-runtime/runs/after8.jim_document.failures.txt`).
- `cmd_package`: there is no test for the protocol-release change, because no cheap fixture
  exists. `set_runtime_version` re-pins the profile, so the two releases only diverge for
  an authored package provider.
- `spectcl_ports`: the report prints `[stale]` rows only on failure. I did not check
  whether any documented row now matches.
- The previous agent's §2.3 parked items stay parked under the scope decision.

