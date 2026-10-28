# H5 slice 2 design: owned application backend (private VM execution, candidate admission, returned-code/literal closure)

Status: DESIGN FOR REVIEW (Astra). Nothing here is implemented. Builds on slice 1 as reviewed
(owned evaluator, request gate, private store, `select_application_folds`), WORK 2026-10-28.

## Goal

A genuine closed consumer for pack-hook folds: one private worker owns the evaluator, registry,
compiler and VM; it selects folds (slice 1), answers them with the owned evaluator, builds a
candidate rewrite, admits it against the original, executes only the admitted candidate in a
private VM, retires everything, and returns detached bytes:

    fold_execute_owned_application_source(packs, source) -> Result<ClosedSourceOutput, ClosedSourceRefusal>

It implements the existing `tcl_runtime_api::closed_native_consumer::ClosedNativeBackend` contract
and is driven through the existing `ClosedNativeConsumerOwner` (`new` → `scope` → `select` →
`execute`). It grants nothing to the ordinary optimiser, CU, SourceNormal or Native handlers.
MAP/RANGE are out of scope (blocked by opaque `subst` and interprocedural constants).

## Components and crate placement

Crate direction forbids `tcl-engine-tclvm → tcl-spectcl`, so the backend lives in `tcl-spectcl`
(`hooks/owned_application.rs`, beside `owned_evaluator.rs`). Engine naming stays in
`tcl-spec-hooks` (re-export of the private VM type), as slice 1 did.

1. **Rewrite plan (tcl-compiler, `optimiser/closed_consumer/application.rs`).**
   `ApplicationRewritePlan::new(source, registry, profile, answers) -> Result<_, Refusal>`.
   - Selection narrowed to the ordinary O129 visitor's sites (`Statement::Call`, `Statement::AssignValue`)
     — review precondition A2. Answers rendered with `render_propagation_word` (made `pub(crate)`)
     and filtered by `admits_answer` (`folded_literal_reads_apart`).
   - Candidate = original with each selected `[...]` span replaced by its rendered word; spans
     non-overlapping, outermost only.
   - `accepts_candidate(candidate)`: candidate equals the planned rewrite or the original (byte
     equality); re-lexes the candidate under the same profile and checks the command count, every
     unchanged command's byte span, and that each replaced word now lexes as one literal word with
     the planned bytes. No other text may change.
   - Command and scalar inventory: canonical command targets of every *remaining* command, and
     scalar roots written, extracted with the retained-binding machinery (as `prepare_closed_source`
     does). Any remaining command that is not registry-backed or is a pack command with no runtime
     backing → `Source`.
2. **Private application VM (tcl-engine-tclvm, from the VM draft, corrected).** Owned C9.1 core,
   memory I/O (`MemoryHost`), fixed canonical compiler restricted to {original, candidate} scripts
   (from the compiler draft's `OwnedApplicationCompileService`), command limit 256, wall clock 250 ms,
   value limit 4096. `restrict_closed_native_commands(commands, scalars)` before the first
   observation. Sticky limit: `vm.limit_exceeded().is_some()` refuses even after a Tcl `catch`
   (review finding on the VM draft, kept).
3. **Current-service preparation (tcl-vm, from the current-preparation draft).**
   `Vm::prepare_owned_application_source` compiles through THIS VM's installed service with the
   current entry and returns an opaque `PreparedOwnedApplicationSource`; `execute_owned_application_preparation`
   runs exactly that module (`run_current_module_at`, CurrentService provenance). The public
   `try_run_module` path (AdmittedForeign provenance) is not used — review finding: no
   Foreign→CurrentService retag.
4. **Backend (tcl-spectcl).** `ApplicationBackend` owns, in drop order: VM, compiler inputs,
   rewrite plan, `OwnedApplicationEvaluator`, private registry. Its
   `Currency = (RegistrySemanticKey, EvaluatorGeneration, plan Arc address, ABI)`; numeric state
   checked as "no pending ERANGE/EDOM" (slice 1 decision, reviewed).

## Returned-code and literal closure (the core of this slice)

The candidate module returned by the installed service is checked before admission and again
before execution (`check_function`, from the compiler draft, extended):

- **Exhaustive destructuring** of `FunctionAsm` and `Instruction` (no `..`): a new field forces review.
- **Opcode allow-list**: NOP, PUSH1/4, POP, DUP, LOAD_SCALAR1/4/STK, STORE_SCALAR1/4/STK,
  INVOKE_STK1/4, START_CMD, STR_CONCAT1, DONE, RETURN_IMM/STK, BEGIN_CATCH4/END_CATCH and the
  catch result pushes. Anything else → `Source`.
- **PUSH is not inert.** `exec.rs` substitutes a literal when `!push_verbatim`
  (`subst_word_bytes_as(..., instr.evaluation)`), running getters and nested commands. This slice
  refuses every PUSH with `push_verbatim == false`, any `evaluation` other than the direct verbatim
  form, and any literal whose `NativeLiteralAction` is not the plain verbatim action. Nested
  script/template compilation therefore cannot be reached; the compiler service also refuses
  substitution templates (empty template list).
- **No procedures, procedure bindings, site claims, loop targets, compiled-local layout, native
  compilation failure, or nested script commands** (`script_command`, `release_body*`,
  `entered_command` all `None`).
- **Literals**: every literal's bytes are within the source or the rendered answers; total literal
  bytes ≤ 32 KiB.
- **INVOKE targets** must be in the plan's remaining command inventory; `restrict_closed_native_commands`
  enforces that only those commands exist in the VM.

## Cache, getter, COW and observer closure

- **Cache**: the review found runtime script-cache hits precede compiler callbacks
  (`interp.rs` ~28290). The backend uses a fresh VM per call, never `eval_source`; only
  `prepare_owned_application_source` (one compile, no cache lookup) and
  `execute_owned_application_preparation`. Before preparation the backend asserts the VM's script
  cache is empty (new read-only `Vm::script_cache_is_empty`, vm-runtime hunk).
- **Getters**: LOAD_* only on scalars in the plan's inventory, which `observe_closed_native_state`
  proves initially absent ordinary cells with no traces; candidate stores precede loads (checked
  by the plan's ordered inventory).
- **COW / value escape**: no value leaves the worker. The transcript is copied out of `MemoryHost`
  after the VM, compiler and evaluator are dropped.
- **Observers**: `NativeCompilationEntry` checks of `ClosedNativeConsumerOwner::check_native_state`
  (no variable observers, no command resolvers or callbacks, closed namespace tables) plus memory
  I/O quiet before execution.

## Installed-program lineage

Every answered fold records `(HookSlot, declaration ptr, file, pack, origin)` from
`OwnedHookRegistry::owns_binding` at answer time. The backend's `validate_rewrite` re-checks that
each planned replacement's answer came from a still-current binding (same evaluator generation,
same plan Arc, same receiver). The detached output adds a receipt listing these rows (strings only).

## Flow

1. Worker: private store + owned evaluator (slice 1).
2. `select_application_folds` (narrowed) → owned answers → `ApplicationRewritePlan`.
3. Construct the VM and compiler inputs {original, candidate}; build `ApplicationBackend`.
4. `ClosedNativeConsumerOwner::new(backend)` (captures state, validates the original).
5. `scope.select(candidate)` → `owner.execute(rewrite)`: validate again, prepare via the installed
   service, check module closure, execute once, drop everything, return transcript + rewritten source.

The original is validated but never executed (no double execution or replay). The fold's
correctness rests on the pack hook's declared contract, as for the ordinary O129 fold (user
decision 2026-10-22: declared-effect hooks fold only when pure).

## Tests planned

Positive: length hook pack `puts [owner::f abcde]` → transcript `5\n`, rewritten `puts 5`;
`set y [owner::f abc]; puts $y` → `3\n`. N17 `$arr($a:b)` hook → `good\n`.
Negative: a non-verbatim PUSH in the candidate (quoted word with `$x`) → `Source`; caught limit
(`catch {while 1 {}}`) → `Execution`; remaining unbacked pack command → `Source`; trace/rename in
the source → refusal; candidate drift between `select` and `execute` → `Selection`/`Currency`;
prepopulated foreign host → `NativeContext`; quarantined hook during answer → `Currency`.

## Questions for the reviewer

1. Is the candidate-only execution argument acceptable, with the hook's declared contract as the
   fold authority, or must a runtime-backed pack command's original also be cross-checked (which
   would be double execution)?
2. Is refusing every non-verbatim PUSH sufficient closure for this first domain, given that
   `evaluation` and `push_verbatim` are the only executor branches into substitution?
3. Is a fresh VM plus an empty-script-cache assertion sufficient cache ownership, or must the
   compiler generation also be pinned across `prepare` → `execute`?
4. Ownership hunks outside my files: tcl-vm (`prepare_owned_application_source`,
   `execute_owned_application_preparation`, `script_cache_is_empty`), tcl-engine-tclvm
   (`PrivateApplicationVm`) and tcl-spec-hooks (re-export). These would be N17/H5 spans, reviewed
   with vm-runtime.
