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
runtime change. COMPILE-CHECK

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

GOLDENS

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

RESULTS

## 3. Compiler-side root causes (reported to the coordinator)

COMPILER

## 4. Not done / remaining

REMAINING
