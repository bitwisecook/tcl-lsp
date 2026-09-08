# Value transfers — the registry's dataflow axis

How a command invocation transforms the compiler's abstract constant state,
declared once on its `CommandSpec` and consumed generically by SCCP, the
optimiser, the interprocedural summaries, dead-code elimination, branch
analysis, and every diagnostic that reads the lattice. This is the design
for [issue #1943](https://github.com/bitwisecook/tcl-lsp/issues/1943): it
replaces the per-command arms in `rust/tcl-compiler/src/sccp.rs` with a
registry-owned descriptor, states what a `.tclspec` pack can author on that
axis, and works through the consequences pass by pass. Read it before
touching a fold, a lattice transfer, or a constant-condition rewrite, and
before adding compile-time knowledge about any command to a consumer.

> **Status — a proposal, not a description of what is built.** The
> `ValueTransfer` descriptor, `value_transfer_for_call`, `CellFoldFn`,
> `DestructureFoldFn`, the `cell_fold` / `destructure_fold` hook families,
> `SccpResult::folded_types`, `TransferDecline`, and `cargo xtask
> value-transfers` name nothing in the workspace today. Every *existing*
> identifier this document cites was checked against the tree; the survey
> sections describe the code as it is, and the model sections describe what
> is proposed. Sequencing is at the end; decisions the owner must ratify are
> collected in the last section.

## The four motivating programs

Each of these is legal, common Tcl. What the compiler knows about it today
depends on which of seven independent constant evaluators happens to be
asked, and the answer is different for each consumer.

```tcl
set s [string range foobarbaz 3 6]      ;# (1) a pure fold: barb
set h [binary format H* 414243444546]   ;# (2) a pure fold with no folder: ABCDEF
set n 1; incr n; incr n 2               ;# (3) a read-modify-write chain: 4
set acc ""; append acc foo; append acc bar
switch -- $acc {                        ;# (4) a switch whose subject is known
    baz     { puts never }
    default { puts always }
}
```

| Program | Optimiser (O-codes) | Shared lattice the diagnostics read | Why they differ |
|---|---|---|---|
| (1) `string range` | O129 fires: `string range` carries `const_fold: Some(fold_range)` (`rust/tcl-registry/src/commands/tcl/string_.rs`) and the propagation pass re-runs SCCP with `BuiltinFoldInputs` | `s` is `Overdefined` — `FunctionUnit::build` calls `sccp_with_extra_escaping` with `folds = None` (`rust/tcl-compiler/src/compilation_unit.rs`) | the shared lattice's memo key does not carry the command-mutation trust fact (`rust/tcl-compiler/src/sccp.rs`, the `BuiltinFoldInputs` doc) |
| (2) `binary format` | nothing: `rust/tcl-registry/src/commands/tcl/binary_.rs` declares `pure: true` on the `format` subcommand and no fold | `Overdefined` | the registry can say *whether* a command is pure but not *what* it computes; there is no evaluator, though `tcl_cmd_core::binary::format` exists and the registry already depends on `tcl-cmd-core` |
| (3) `incr` chain | O100 forwards `4` into a later `$n` (`sccp_value_literal`); the `Statement::Incr` arm of `evaluate_def_with_folds` does the arithmetic | `4` — the same arm runs in both lattices | `incr` is the one read-modify-write command with a typed IR node and a hand-written transfer; `append acc …` hits `_ => LatticeValue::Overdefined` and `acc` is unknown everywhere |
| (4) `switch -- $acc` | O112 would fire if `acc` were constant (`structure_elimination.rs` resolves `$acc` by name from its own `Env`); it is not, because of (3) | no `ConstantBranch`, no I231, no O107 — even with a constant subject, `switch_subject_operand` lowers a whole-variable subject to `ExprNode::Raw`, which the expression evaluator rejects before consulting the environment | three implementations of `switch` semantics (`cfg_lower.rs`, `structure_elimination.rs`, `analyser/handlers.rs`) with two notions of subject resolution |

The design below makes one answer per program, computed once, visible to
every consumer, gated by the same soundness facts, and authorable by a pack
for a command the registry has never heard of.

## Where per-command knowledge lives today

The registry invariant ([command-registry.md](command-registry.md)) says
per-command knowledge is a `CommandSpec` fact and the compiler is a generic
consumer. On the dataflow axis the compiler crate is already close: a whole-
crate sweep finds 26 live name-keyed sites, and only the ones below touch
constants, values, or dataflow. They are the inventory the drift gate in
[§ The drift gate and the generated inventory](#the-drift-gate-and-the-generated-inventory)
must drive to zero or waive by name.

| Site | Shape | What it encodes |
|---|---|---|
| `sccp.rs` `try_fold_cmd_subst` | `trusted("list")`, `trusted("format")`, `cmd == "llength"`, `cmd == "string"` + `sub == "length"`, `cmd == "expr"` | each arm *is* that command's fold, run ahead of the registry engine "so single-hop results stay byte-identical" |
| `sccp.rs` `evaluate_def_with_folds` | `Statement::Incr { name, amount, .. }` | the `incr` lattice transfer: `Const(Int)` base, literal or lattice amount, `checked_add`, widen on overflow / wrong intrep / dynamic key |
| `sccp.rs` `evaluate_def_with_folds` | `matches!(command.as_str(), "foreach" \| "lmap")` | the loop-variable `ConstSet` transfer over a literal or folded list |
| `sccp.rs` `scan_defined_and_unset` | `command == "unset"` | the destroy fact the `info exists` post-pass needs |
| `optimiser/chain_fold.rs` | `"set"` / `"append"` / `"lappend"` | the O104 / O130 write-chain classifier ("the fold's per-command semantics … ARE the dispatch") |
| `optimiser/structure_elimination.rs` | `resolve_subject`, `pattern_matches` | O112's own subject resolution and arm matching for `switch` |
| `optimiser/propagation.rs` `fold_tail_statement_under_lattice` | `Statement::Incr` | "`incr` returns the value it assigned" for the implicit-return fold |
| `optimiser/elimination.rs` `assignment_safe_to_delete` | `Statement::Incr` | "the write is the whole observable effect" — `append` / `lappend` fall to `_ => false` and are never deletable |
| `optimiser/end_offset.rs` | `"lindex"`, `"lrange"`, `"lreplace"`, `"string index/range/replace"`, `"llength"`, `"string length"` | O128's length-position table |
| `static_loops.rs` `exec_statement` | `Statement::Incr` | a second, independent `incr` transfer for bounded `for` simulation |
| `intervals.rs` `transfer` | `Statement::Incr` | a third `incr` transfer, literal amounts only |
| `interval_bounds.rs` | `"lindex"`, `"lset"`, `"string index"`, `cmd.name() != "list"` | container-length recovery |
| `value_provenance.rs`, `script_arg.rs`, `auto_path_eval.rs` | `"list"`, `"info"`, `"file"` | three more private `[list …]` / path evaluators |
| `analyser/` | `const_strings`, `last_literal_set_value_for_var`, `infer_list_length_from_recent_set` | the analyser's own lexical constant store plus two backward source re-scans |

Three modules that the same sweep found **clean** are the exemplars the
design copies: `rust/tcl-compiler/src/existence_query.rs` recognises
`[info exists X]` through `SemanticOperationId::Intrinsic(IntrinsicId::InfoExists)`
and names no command; `rust/tcl-compiler/src/const_subst.rs` folds any
`[cmd …]` through `CommandSpec::const_fold` and states "No command name is
matched here"; `rust/tcl-compiler/src/world_state_ssa.rs` renames mutable
interpreter state from registry `StateTransition` facts alone. The transfer
descriptor is the same move applied to the value axis.

The three structural gaps the inventory reduces to:

1. `evaluate_def_with_folds` answers `Overdefined` for every `Statement::Call`
   that is not `foreach` / `lmap`, so no read-modify-write command other than
   `incr` has a lattice transfer.
2. The shared per-unit lattice is built without `BuiltinFoldInputs`, so every
   registry fold — 41 of them today — is invisible to every diagnostic.
3. The `switch` dispatch chain lowers a whole-variable subject to
   `ExprNode::Raw`, which cannot be evaluated, so a constant-subject switch
   never yields per-arm reachability.

Two stale statements in neighbouring design docs were corrected alongside
this survey: [sccp-core-analyses.md](sccp-core-analyses.md) said
`Statement::Incr` was not a shape `evaluate_def` folds (it has been since
the `incr` arm landed), and
[constant-folding-type-inference.md](constant-folding-type-inference.md)
named a propagation-side trace-and-alias gate that no longer existed under
that name — the live gate is `sccp::is_externally_mutable` over the
`var_observability::analyse_var_observability` escaping set plus the
`Module::traced_variables` / `has_dynamic_variable_trace` facts carried by
`sccp::TraceInputs`.

## Vocabulary

- **Value transfer** — for one statement, the function from the abstract
  state before it to the abstract state after it: the lattice value of each
  variable it writes and of its result. The registry declares it; SCCP
  applies it.
- **Cell** — a Tcl variable as storage: a scalar, an array element, or the
  whole array. A *cell transfer* reads the cell's current lattice value and
  writes a new one (`incr`, `append`, `lappend`, `lset`, `dict incr`, …).
- **Pure fold** — a transfer with no written cell: the result is a function
  of the literal argument values (`string range`, `binary format`, `format`,
  `list`, …). This is exactly what `const_fold` declares today.
- **Destructure** — a transfer that writes several cells from the literal
  arguments and also produces a result (`lassign`, `scan`, `regexp` with
  match variables, `binary scan`).
- **Destroy** — a transfer that unbinds cells (`unset`, `array unset`).
- **Iterate** — a transfer that binds loop variables to the elements of a
  literal list, producing a `ConstSet` (`foreach`, `lmap`, `dict for`,
  `array for`, the EDA `foreach_in_collection`).
- **Widening condition** — a stated reason a transfer declines and its
  targets go to `Overdefined`: a non-literal word, an untrusted head, an
  escaping or traced target, a wrong intrep, overflow under the target
  release, a dynamic key, a release-ambiguous answer, an over-sized result.
- **Branch fact** — a decision about a `Terminator::Branch` or a `switch`
  arm made from lattice values: taken, not taken, or open.

The lattice itself is unchanged: `LatticeValue::{Unknown, Const, ConstSet,
Overdefined}` over `ConstValue::{Int, Float, Bool, String}`
(`rust/tcl-compiler/src/analyses.rs`), joined by `sccp::join`, with
`MAX_CONSTSET_SIZE = 32`. What changes is who computes the value at each
statement.

## The model

### One descriptor, five kinds

```rust
/// `CommandSpec::value_transfer`, `SubCommand::value_transfer`,
/// `CommandForm::value_transfer`. Authored only where the derivation
/// below cannot produce it; consumers never read this field directly.
pub enum ValueTransfer {
    /// The result is a pure function of the literal argument values. The
    /// function is the spec's `const_fold` / `const_fold_versioned`.
    Pure,
    /// Read the cell named by the resolved `VarWrite` argument, apply
    /// `update`, write it back; the result is the new value.
    Cell { update: CellUpdate },
    /// Each resolved `VarWrite` argument receives a value computed from
    /// the literal arguments; the result is computed alongside.
    Destructure(DestructureFoldFn),
    /// Every resolved `VarWrite` argument is unbound afterwards.
    Destroy,
    /// Each `LoopVarList` group binds its variables to successive
    /// elements of the paired literal list; the loop body sees a
    /// `ConstSet`.
    Iterate,
    /// An algorithm the descriptor cannot express and the compiler keeps,
    /// dispatched by an exhaustive `match` the way `ReturnTypeHookId` is.
    Native(ValueTransferId),
}

/// The evaluator behind `Cell`: `old` is the target's proven value, `args`
/// the literal words after the target, `version` the target release when
/// the profile names one. `None` widens.
pub type CellFoldFn =
    fn(old: &str, args: &[&str], version: Option<TclVersion>) -> Option<String>;

/// The evaluator behind `Destructure`: the result plus one value per
/// written target, in resolved-role order. `None` widens every target.
pub type DestructureFoldFn =
    fn(args: &[&str], version: Option<TclVersion>) -> Option<DestructuredFold>;

pub struct DestructuredFold {
    pub result: String,
    pub writes: Vec<Option<String>>, // `None` = this target widens
}
```

`CellUpdate` is the enum `rust/tcl-registry/src/native_lowering.rs` already
owns for `NativeLowering::CellReadModifyWrite` — `Increment`, `Append`,
`ListAppend` — extended with the dict and list cell operations the intrinsic
catalogue names (`DictIncr`, `DictAppend`, `DictListAppend`, `DictSet`,
`DictUnset`, `ListSet`, `ListPop`). Each variant maps to one registry-owned
`CellFoldFn` in a new `rust/tcl-registry/src/cell_fold.rs`, beside
`const_fold.rs`, implemented over the shared cores in `tcl-cmd-core` where
one exists (`tcl_cmd_core::index`, `tcl_cmd_core::string`,
`tcl_cmd_core::binary`, `tcl_syntax::list`, the dict canonicaliser) so the
compile-time answer and the runtime answer come from one implementation.
That is the Family-B rule ([family-b-routing.md](../family-b-routing.md))
applied to folding.

`ValueTransferId` is deliberately small. Its first members are the two
transfers whose inputs are not literal words: `Expr` (the braced-versus-
quoted substitution model and the lattice environment, today's `cmd ==
"expr"` arm) and `ExistenceQuery` (today's `existence_constant_branches`
post-pass, which reads whole-body facts rather than arguments). A third
member is added only when a command's semantics cannot be written as a
function of values, and the drift gate lists every member with its reason.

### The central query

```rust
impl CommandSpec {
    /// The one derived answer every consumer asks. Mirrors
    /// `return_type_for_call`: explicit field first, derivation second,
    /// `None` third — and `None` means "widen every def", never "guess".
    pub fn value_transfer_for_call(&self, args: &[&str]) -> Option<ResolvedValueTransfer>;
}

pub struct ResolvedValueTransfer {
    pub kind: ValueTransfer,
    /// 0-based argument indices whose resolved role is `VarWrite`,
    /// from `arg_role_resolver` → `arg_roles` → `assigns_variable_at`.
    pub targets: Vec<u8>,
    /// Index of the first value word after the target(s).
    pub values_from: u8,
    /// The type the written cell and the result take, from
    /// `var_write_typing` and `return_type_for_call`.
    pub written_type: Option<TclType>,
    pub result_type: Option<TclType>,
}
```

The derivation, applied when `value_transfer` is unset, in order:

| Existing fact on the spec | Derived transfer |
|---|---|
| `const_fold` or `const_fold_versioned` set, no `VarWrite` role | `Pure` |
| `native_lowering == CellReadModifyWrite(update)` | `Cell { update }` |
| `semantic_operation == Intrinsic(id)` with `id` in the cell-operation table (`DictIncr`, `DictAppend`, `DictListAppend`, `DictSet`, `DictUnset`, `ListSet`) | `Cell { update: id.cell_update() }` |
| `var_write_typing == ElementsOf { container_arg }` (`lassign`) | `Destructure(elements_of)` |
| `Traits::DESTROYS_VARIABLE` | `Destroy` |
| `Traits::LOOP_LIST_HEADER`, or `HAS_LOOP_BODY` with a `LoopVarList` role | `Iterate` |
| anything else | `None` — every def widens, exactly today's `_ => Overdefined` |

`incr`, `append`, and `lappend` therefore need **no new authored fact**:
their `native_lowering` already names the update kind, and
`assigns_variable_at: Some(0)` plus `ArgRole::VarWrite` already name the
target. `dict incr` / `dict append` / `dict lappend` come from their
`semantic_operation`. The subcommand level matters here: `SubCommand` has
no `native_lowering` and no `assigns_variable_at` today, so the descriptor
and the target index must exist at subcommand and form level from the first
change, or `dict incr` cannot be described at all.

Authored fields stay private to the resolver where the crate layout allows
it, which is the [#1712](https://github.com/bitwisecook/tcl-lsp/issues/1712)
rule: a consumer that reads `const_fold` or `native_lowering` to decide a
fold instead of calling `value_transfer_for_call` is the raw-field
reconstruction that programme forbids, and the gate below flags it.

### The transfer function, over the lattice

For a statement `S` with SSA uses `U` and defs `D`, the transfer runs only
when its inputs are known:

1. **Resolve the invocation.** `Statement::Call { command, canonical_command,
   args, defs, .. }` projects directly. `Statement::Incr { name, amount, .. }`
   projects to the invocation view `incr name ?amount?`. The synthetic
   `foreach` / `lmap` / `dict for` header call the CFG builder emits carries
   `foreach_groups` and projects to `Iterate`. Resolution goes through the
   realm (`CommandBindingRealm::resolve_unpositioned(head).spec_name()`), so
   a proven alias gets its target's transfer and a taken-over spelling gets
   none.
2. **Trust the head.** `ModuleCommandMutations::trusts(name)` for a builtin,
   `trusts_proc_binding` for a proc — whole-module, flow-insensitive, as
   `const_subst.rs` requires. The shared lattice can only do this once the
   trust fact is in its memo key; see [§ Where the transfer runs](#where-the-transfer-runs-and-what-it-costs).
3. **Resolve the words.** Every non-target word becomes a literal value the
   way `ConstSubstCtx::literal_words_at_depth` does today: a clean literal
   token, a `$var` whose lattice entry at this statement's use version is
   `Const`, or a nested `[cmd …]` that folds recursively under the same
   context. A `ConstSet` word is handled by the rule in step 5. Anything
   else — a multi-token word, `{*}`, an unresolvable variable, JimTcl
   `$(…)` — widens.
4. **Read the target.** For `Cell`, the target's lattice value at the use
   version. `Unknown` propagates `Unknown` (SCCP's optimism, needed for the
   fixpoint); `Overdefined` widens; `Const(v)` calls the evaluator with
   `old = v`'s Tcl string form. A dynamic key (`incr a($i)`) widens rather
   than answering `Unknown`, for the reason the current arm documents: the
   miss is permanent and `join(prev, Unknown) = prev` would launder a stale
   element constant.
5. **Map over `ConstSet`.** When exactly one input is a `ConstSet` and the
   rest are `Const`, evaluate once per member and join the answers; a
   product over two `ConstSet` inputs is not attempted (widen) — the cap on
   `MAX_CONSTSET_SIZE` bounds the work and the join bounds the height.
6. **Apply and type.** A `Some` answer re-enters the lattice through
   `parse_literal_value` (leading-zero and sign spellings stay strings, as
   today), and the resolved `written_type` / `result_type` is recorded in a
   new `SccpResult::folded_types: HashMap<ValueKey, TclType>` that
   `type_infer` joins into its own lattice. Today `sccp.rs` discards the
   `return_type` that `ResolvedConstSubst` already carries; this is the fix.
7. **Record why not.** A decline carries a `TransferDecline` reason
   (`NonLiteralWord`, `UntrustedHead`, `TargetEscapes`, `TargetTraced`,
   `WrongIntrep`, `Overflow`, `DynamicKey`, `ReleaseAmbiguous`,
   `ResultTooLarge`, `DepthExceeded`, `NoTransfer`) that the Explorer's
   `sccp` view renders and the inventory counts, the way `NativeLowering`
   declines are typed rather than silent.

The transfer is monotone by construction — a deterministic function of
`Const` inputs answers `Const` or widens, never a different constant for the
same inputs — which is what SCCP's optimistic iteration needs to converge.
Determinism is therefore a *contract* on evaluators, not a hope: a native
evaluator is a pure Rust function, and a pack evaluator runs in the hook
sandbox whose whitelist has no clock, no I/O, and (once the gap in
[§ The execution host](#the-execution-host) is closed) no `rand`. The
`ConstSet` cap guarantees termination regardless; determinism guarantees
that what it converges to is true.

### Widening conditions, stated once

The current `incr` arm re-derives its own widening rules; `append` would
re-derive them again; a pack would forget one. The descriptor states them
once, and every kind inherits them:

| Condition | Who decides | Answer |
|---|---|---|
| a word is not a resolvable literal | the engine (step 3) | widen this statement's defs |
| the head is renamed, aliased, shadowed, or in an opaque namespace | `ModuleCommandMutations` | widen; the same fact that gates O129 today |
| the target is `::`-qualified, escaping, or the function has a dynamic trace | `sccp::is_externally_mutable` | the def is `Overdefined` before any transfer runs, as for every def today |
| the target is named anywhere in `Module::traced_variables` | `TraceInputs` | same |
| the target is an array-element base write | `element_write_base` | same |
| the old value has the wrong intrep for the update (`incr` of `abc`, `lappend` to an unbalanced list) | the evaluator returns `None` | widen — the program errors at run time, and a fold must never turn an error into a value |
| the arithmetic leaves the wide range | the evaluator, under `version` | 8.5+ promotes to a bignum string; 8.4 and `None` widen |
| the answer differs between target releases and the profile names none | the evaluator | widen — `NumberSyntax::unanimous` for indices, `StringCharacterModel` for counts, the leading-zero rule for numerals |
| the result exceeds `MAX_FOLD_OUTPUT_BYTES` (1 MiB today) | the engine | widen |
| nesting exceeds `MAX_CONST_SUBST_DEPTH` (16) | the engine | widen |
| the statement is a `Barrier` or `UpFrame` | SCCP | every tracked value widens, as today |

The `WrongIntrep` row is the one a pack author is most likely to get wrong,
and the sandbox makes it hard to get wrong: `incr` on `abc` *raises* in the
hook VM, and an error is an abstention.

### Answers to the issue's three questions

**Is the IR node one registry fact with the transfer, or two?** Two facts,
one identity. `LoweringHookId::Incr` describes IR *shape*, and the typed
`Statement::Incr` node is consumed across 43 files for codegen, native lowering,
α-renaming, span rebasing, and liveness; it stays. The lattice transfer is
`ValueTransfer::Cell { update: CellUpdate::Increment }`, *derived* from the
same `CellUpdate` the native lowering already declares, so a new
read-modify-write command adds one `CellUpdate` variant and gets both
consumers by construction — the same relationship `SemanticOperationId::StructuredLowering`
has to `LoweringHookId`. A contract test pins agreement: a spec with
`CellReadModifyWrite(u)` must resolve to `Cell { update: u }` on the same
target. The `Statement::Incr` sites that encode *semantics* rather than
shape — the transfer in `sccp.rs`, the simulator arm in `static_loops.rs`,
the interval arm in `intervals.rs`, the removability and hidden-read arms
in `elimination.rs`, the tail fold in `propagation.rs`, and the
global-write rule in `interprocedural.rs` — each become a consumer of the
resolved transfer; the sites that encode shape (`codegen`, `inlining`,
`lattice_rebase.rs`, `native_lowering`, `shimmer`) do not change.

**Is this a new hook or `const_fold` with an environment parameter?** A new
descriptor whose `Pure` arm *is* `const_fold`. The cell evaluator needs a
different signature because its contract differs — it reads a proven value
that is not a word, it must abstain on the wrong intrep, and its overflow
rule is versioned — but it is the same family of "pure function of values",
rides the same `pack_hooks` slot and thunk machinery, and shares the engine
(`ConstSubstCtx`) for word resolution, trust, nesting, and escapes. Giving
`const_fold` an extra `old` parameter would silently change 41 existing
folders' contracts; a second function type does not.

**Do the analyser's `analyser_hook` handlers belong on this axis?** No.
`AnalyserHookId` is scope and definition structure — what a `proc`, a
`namespace eval`, or a `dict for` *declares*. Its constant-string store
(`Analyser::const_strings`) is a *consumer* of constants that should read
`ConstSubstCtx` and, where the salsa unit is available, the lattice; it is
not a producer of transfers. The one overlap, `DictWith` binding the keys of
a constant dict, is a `Destructure` consumer. Keeping the axes apart is what
lets the analyser's isolated per-item pass stay sound with
`ModuleCommandMutations::distrust_all()` while the unit-level lattice folds.

## The registry surface

### Fields

| Field | On | Meaning |
|---|---|---|
| `value_transfer: Option<ValueTransfer>` | `CommandSpec`, `SubCommand`, `CommandForm` | the authored transfer; absent means "derive" |
| `assigns_variable_at: Option<u8>` | `SubCommand` (new), `CommandForm` (new) | the target index, today command-level only |
| `native_lowering` | unchanged | the derivation reads it; packs still cannot author it (`render_spectcl.rs` `GAPS`, `Excluded`) |
| `const_fold`, `const_fold_versioned` | unchanged | the `Pure` evaluator; `run_const_fold` stays the dispatch |

Everything else the transfer needs already exists: `arg_roles` /
`arg_role_resolver` for the targets, `var_write_typing` and
`return_type_for_call` for the types, `safe_on_uninit` for the unbound
case the compiler cannot yet prove, `result_stability` for whether a `Pure`
transfer may run at all (`ReferentiallyTransparent` only — a
`ReadsVersionedWorld` or `Volatile` command is never folded, whatever it
declares), and `Traits::READS_BEFORE_WRITE` as the membership predicate
`CommandRegistry::rmw_first_arg_variable` already exposes.

### Contract tests

Registry contract tests run through the front-ends with the registry as
oracle ([registry-contract-tests.md](../contracts/registry-contract-tests.md));
these are the additions, each an in-process sweep over every loadable
dialect plus the shipped `.tclspec` packs, in the shape of
`rust/tcl-registry/tests/analyser_hooks.rs`:

1. **Agreement.** `CellReadModifyWrite(u)` on a spec ⇒
   `value_transfer_for_call` resolves `Cell { update: u }` with a target
   whose resolved role is `VarWrite`; `const_fold` set ⇒ `Pure`;
   `DESTROYS_VARIABLE` ⇒ `Destroy`. A spec that declares
   `result_stability: Volatile` and a `const_fold` is an error.
2. **Typing.** For a fixture table of `(command, old, args)`, the evaluator's
   answer parses as the spec's `written_type` / `result_type` — an `incr`
   that returns a non-integer, or a `dict incr` whose dict does not
   canonicalise, is a bug caught here, not in shimmer.
3. **Oracle.** `rust/tcl-registry/tests/differential_fold.rs` already runs
   every `const_fold` against a real `tclsh9.0` and accepts only equality or
   `None`. It gains the cell evaluators (`set v OLD; CMD v ARGS; set v`) and
   the destructuring ones, per release the test can find on `PATH`, so the
   8.4 / 8.5 overflow and leading-zero rules are pinned by evidence rather
   than by reading `tclIncr`.
4. **Inventory.** The generated inventory below is regenerated and diffed.

## The SpecTcl surface

A `.tclspec` pack can already author `const_fold { … }` as a Tcl body run
in the hook sandbox ([spec-dsl-examples/README.md](../spec-dsl-examples/README.md)
§ *Hooks*), which is the `Pure` kind. This section adds the rest, following
the rule design E fixed for `constraints`: **declarative data first, a hook
only where data cannot say it, and the hook may only report values — the
compiler owns every gate.**

### What a pack writes

```tcl
command counter::bump {
    arity 1..2
    arg 0 -role VarWrite
    traits {READS_BEFORE_WRITE FIRST_ARG_VARNAME}
    return_type Int

    # The shipped increment evaluator, by name: no body runs.
    cell_fold -native Increment
}

command log::push {
    arity 2
    arg 0 -role VarWrite
    return_type List

    # An authored evaluator. `words` are the argument words after the
    # command name with every non-target word already resolved to its
    # constant; the target's proven value arrives in ctx.
    cell_fold {words ctx} {
        set old [dict get $ctx target-value]
        lappend old "[lindex $words 1]"
        fold $old
    }
}

command kv::split3 {
    arity 4
    arg_role_resolver {words ctx} { role 1 VarWrite; role 2 VarWrite; role 3 VarWrite }
    arg_role_resolver_roles {VarWrite}
    return_type Int

    # Writes three cells and returns a count. Silence widens every target.
    destructure_fold {words ctx} {
        lassign [split [lindex $words 0] :] a b c
        write 1 $a
        write 2 $b
        write 3 $c
        fold 3
    }
}
```

`Destroy` and `Iterate` are never authored: they derive from
`DESTROYS_VARIABLE` and the `LoopVarList` role, both of which a pack
already states. `Native` is not authorable at all — like `completion`, it
names compiler code.

The two new statements follow `hook_source`'s existing grammar exactly
(`FIELD ?-inputs {…}? {params} {body}` or `FIELD -native ID`), so the loader
change is two arms in the property-statement dispatch, at command,
subcommand, and form scope. The parameter list stays `{words ctx}`; the
target's value travels in `ctx`, which is the ruling the option-arity hook
already established ("the fix is `ctx`, not a third parameter").

### `ctx` keys and verbs

| Family | Extra `ctx` keys | Verbs | Silence means |
|---|---|---|---|
| `cell_fold` | `target-index` (0-based, after the command name), `target-value` (the cell's proven value) | `fold VALUE` | widen the target and the result |
| `destructure_fold` | `targets` (the resolved `VarWrite` indices) | `fold VALUE`, `write IDX VALUE` | widen every target and the result; a `write` to an index not in `targets` raises, and raising is an abstention |

`answer_of` in `rust/tcl-spec-hooks/src/emit.rs` is the exhaustive match that
forces both decisions for every new family, and the same change touches
`HookFamily`, `HOOK_FAMILIES`, `verbs()`, `silence()`,
`requires_all_literal()`, `field()`, a thunk, a `slot_tables!` row, an
accessor, and the `bind_command` / `bind_subcommand` arms in
`rust/tcl-spectcl/src/hooks.rs` — eleven mechanical edit sites, all in two
files, which is the cost of a family and the reason there are two rather
than four.

### Preconditions, restated for transfers

The README's normative rule — a fold body runs only when every word's
`kinds` entry is `literal` — holds unchanged, with one clarification: the
*engine* resolves `$var` words to their lattice constants before the hook
sees them, so from the body's point of view every non-target word is a
literal value, and the target word is a literal *name*. A body never reads
`kinds`; the precondition is the loader's and the engine's, stated once.

The version-invariance rule holds too: an unversioned `cell_fold` body must
answer the same for every release it accepts, or return. `Increment`'s
overflow rule is why the shipped kind is `-native`: it is versioned by
nature, like `string is`.

### `-native ID` must mean one thing

Today `-native ID` resolves for the closed catalogues (`lowering_hook -native
Switch` reaches `LoweringHookId::Switch`) but **not** for the body families:
`const_fold_versioned -native string::is` in
`docs/design/spec-dsl-examples/string.tclspec` installs the family's
abstention and nothing else, because no name→`fn` table exists for const
folders and `HookSource::Native` is consumed nowhere but reports. A pack
that names a shipped folder silently loses it.

The transfer families fix this rather than inherit it. `cell_fold -native
KIND` resolves through the `CellUpdate` catalogue (`Increment`, `Append`,
`ListAppend`, `DictIncr`, …), the loader table is pinned by
`native_hook_tables_cover_their_catalogues`, and an unknown kind is dropped
with a load notice. `const_fold -native ID` gets the same treatment in the
same change — a `const_fold::by_name` table over the shipped folders — so
the DSL's promise that `-native` means "the engine, by name" becomes true
for every family at once. The renderer's synthesised
`FIELD -native <command>::<field>` spelling, which names nothing, is then a
`GAPS` entry until the draft can recover the real name.

### Caching and cost

A transfer hook declares `-inputs {words target-value}` and earns
`CacheMode::Content`: the shape key's content hash covers the words and the
target's value, so an unchanged call site with an unchanged incoming
constant is answered from the thread-local cache at the 24.5 ns the
measured shape-cached path costs, and only a genuinely new `(words, old)`
pair enters the VM (about 28 µs, the measured uncached resolver). Two facts
bound the worst case: a transfer runs only when its inputs are `Const` or a
bounded `ConstSet`, never on `Unknown` or `Overdefined`, and shipped
commands keep native evaluators, so only pack-declared commands ever enter
the VM. The counter `RelationCheckStats::hook_entries` established for
`constraints` gets a sibling, `transfer_hook_entries`, with the same
contract: it stays 0 on a corpus of shipped commands.

`dialect` is outside the shape key today, so a transfer body that declares
it is uncacheable; the key should widen to include the profile name before
a shipped pack relies on `dialect` in a transfer.

### The four surfaces

[command-spec-studio.md](../contracts/command-spec-studio.md) § *Parity with
native specs* makes four surfaces move together or carry a `GAPS` entry:

1. **Registry** — the fields above, and `rust/tcl-spec-studio/src/coverage.rs`'s
   exhaustive destructuring witness, which fails to compile until each new
   field is surfaced or `Surface::Excluded`.
2. **Loader** — `cell_fold` / `destructure_fold` at command, subcommand,
   and form scope; `assigns_variable_at` at subcommand scope (a
   `LoaderGap` today); the coverage matrix rows in the frozen-syntax memo.
3. **Renderer and export** — `render_spectcl.rs` emits the two statements
   through the existing `native_hook` / `catalogue_hook` helpers, and
   `export.rs` round-trips bodies verbatim as it does for `const_fold`;
   the `spectcl_roundtrip` gate covers both.
4. **Studio** — a `FieldKind::Enum` picker over the `CellUpdate` catalogue
   for the `-native` form and a body box for the authored form, in the
   "Purity and folding" cluster; help text; `relations.rs` linking the
   target role to the transfer. Two known studio limits apply and should
   be closed alongside: carry-forward of a hook body is top-level only, so
   a subcommand transfer body is dropped on a form edit; and there is no
   "try it" affordance, although `HookHost::install_pack_hooks` plus a
   synthetic `HookCall` is all a sample-inputs box would need.

`tcl-mcp`'s `spectcl_check` already reports every hook's family,
cacheability, and the keys its body reads beyond its `-inputs`; the two
families join that report so an AI author sees a transfer that reads
`target-value` without declaring it.

### Inference for the spec-author skill

`ai/claude/skills/spec-author/SKILL.md` infers arity, roles, traits, hover,
and packages from a library's sources, and lists "side effects, taint,
version history" as the questions only the author can answer. Transfers are
inferable for a large class of private commands: a proc the interprocedural
summary marks `pure` whose body is loop-free and uses only whitelisted
commands *is* its own `const_fold` body; a proc whose body is `upvar 1 $name
v; lappend v …` is a `cell_fold` with `ListAppend` semantics. The skill
emits the stub, runs it through `spectcl_check`, and the differential test
in the pack's corpus proves it against the library's real behaviour. That
is [spec-packs.md](../spec-packs.md)'s observation — for a command
implemented in pure Tcl, "fold" can mean running the implementation on the
literal arguments — made mechanical.

## The execution host

Pack transfers run where pack folds run: `tcl-spec-hooks` builds one
`tcl-vm` engine per pack per thread, compiles each body once to a proc
named `::spectcl::unit::N`, whitelists thirty commands, and enforces a
budget of 100,000 commands, 250 ms, and 16 MiB per invocation; an error is
an abstention logged once, a budget overrun quarantines the hook, and a
caught panic poisons the pack ([spec-dsl-examples/README.md](../spec-dsl-examples/README.md)
§ *Purity and the sandbox*). Three properties of that host matter to a
transfer, and one gap must close first:

- **Everything a transfer body needs is whitelisted** — `incr`, `lappend`,
  `string`, `dict`, `binary`, `format`, `scan`, `regexp`, `regsub`, `expr`
  — and `foldlist` is `tcl_registry::const_fold::fold_list` itself, so a
  body's list rendering cannot drift from the registry's.
- **Determinism is by whitelist.** The pack-evaluation sandbox strips
  `rand` and `srand`; the hook sandbox keeps no `tcl::mathfunc::` prefix at
  all, so `expr {abs($x)}` in a body raises and abstains. A transfer body
  that needs a math function is therefore unwritable today. The fix is to
  give the hook whitelist the same audited `tcl::mathfunc::` retention as
  `pack_eval.rs`, minus the non-deterministic pair — a small change, and a
  precondition for the `Increment`-like bodies a pack will naturally write.
- **A wrong answer is bounded.** The README's asymmetry argument for why
  `const_fold` is authorable and `completion` is not — a wrong fold is a
  wrong value in one expression, diffable by the corpus gate; a wrong
  completion is a wrong control-flow graph with no output to diff — holds
  for a cell transfer, because the answer is a *value* that enters the
  lattice through the same `join` and the same gates as any `set`. It
  holds for `Destroy` only because the compiler derives it from a trait
  and never lets a body say it: an authored "unbind" would be a claim about
  existence, which is closer to completion than to a value, and is
  excluded for that reason.

**The salsa overlay gap.** `Analyser::with_pack_overlay` and the
semantic-token queries read the pack-specialised registry, but
`TclDb::registry(dialect)` — and through it `compilation_unit`, the shared
SCCP lattice, and `function_optimisations` — resolve the un-overlaid
registry. A pack's `const_fold` is visible to the non-salsa optimiser path
the `optimiseDocument` command takes and invisible to the memoised
diagnostics path. That is tolerable for a fold nobody sees in a diagnostic;
it is not tolerable for a transfer whose purpose is to reach every
diagnostic. Threading `spec_pack_key` into the `compilation_unit` query is
a prerequisite of phase 4, and the same edit closes the gap for the 41
existing folds.

## Where the transfer runs, and what it costs

Today there are two lattices: the shared per-unit one (`FunctionUnit::sccp`,
no fold inputs, read by every diagnostic and by O112, O107, I230, I231,
W124, W230–W233, the shimmer and taint families) and the optimiser's
re-run (`sccp_with_builtin_folds` in `propagation.rs`, with
`BuiltinFoldInputs`, gated on a function containing a command-substitution
assignment). Native transfers belong in the shared lattice, or the
diagnostics never see them.

The blocker is the memo key, not the work: the shared lattice's salsa key
(`FnLatticeKey` in `rust/tcl-lsp-db/src/lib.rs`) cannot carry
`ModuleCommandMutations`, so folds that need the trust fact are held back.
`CommandTrustSnapshot` already exists as the hashable form of exactly that
fact, and `FnLatticeKey` already carries two whole-module facts of the same
kind (`traced_variables`, `has_dynamic_variable_trace`). Adding the snapshot
to the key is the change; a `rename` appearing anywhere in the file then
invalidates every per-procedure lattice in it, which is the correct
sensitivity and the one the analyser's deferred-body memo already has.

Cost is bounded by construction: a transfer runs per statement only when
every input is `Const` or a bounded `ConstSet`; native evaluators are
microseconds; `existence_constant_branches` is today computed twice (once
into `constant_branches`, once again for I230) and becomes one
`Native(ExistenceQuery)` transfer run once; and the optimiser's re-run
collapses into the shared lattice for every function whose trust snapshot
is unchanged, which is the common case. The one new cost centre is pack
hooks, bounded as described above and metered by `transfer_hook_entries`.

## Soundness gates, as contract rules

Every rule below is enforced by the engine or SCCP, never by an evaluator
or a pack, and each names the fact that answers it today.

1. **Trust before transfer.** No transfer runs for a head
   `ModuleCommandMutations::trusts` rejects, a proc `trusts_proc_binding`
   rejects or `redefined_procedures` contains, or a name in a namespace
   `changes_command_resolution` / `opaque_namespaces` made opaque. A
   consumer with no whole-module view uses `distrust_all()`.
2. **The realm resolves the head.** The invocation view carries the
   realm-resolved spec name, so an alias gets its target's transfer and a
   rebound spelling gets none (`RealmBinding::is_rebound()`).
3. **Escaping cells never hold constants.** `is_externally_mutable` runs
   before any def is evaluated; a transfer cannot re-narrow a cell it
   widened.
4. **Traces widen.** `Module::traced_variables` and
   `has_dynamic_variable_trace` feed the escaping set, and `TraceInputs`
   is threaded into every SCCP entry point, including the interprocedural
   re-run.
5. **Barriers widen everything but seeds.** `Statement::Barrier` and
   `UpFrame` keep their current treatment; `existence_constant_branches`'s
   whole-function bail on a barrier carries over to the
   `ExistenceQuery` transfer.
6. **Dynamic names abstain.** `DynamicNameBarrier::writes` / `destroys`
   block value motion; `reads` blocks store deletion; a dynamic-key target
   widens rather than answering `Unknown`.
7. **Element writes keep the base opaque.** An array-element transfer
   writes the element's value and refreshes the base as a may-def, exactly
   as `set arr(k) v` does today.
8. **Release facts are inputs, not defaults.** `FoldPolicy`'s six axes and
   `DialectProfile::const_fold_version` reach every evaluator; a profile
   that names no release gets the unanimous answer or a widen.
9. **Errors are not values.** An evaluator that would raise at run time
   returns `None`; the NaN-branch, integer-divide-by-zero, and
   quoted-versus-braced `expr` declines carry over unchanged.
10. **Types travel with values.** A folded value carries the registry's
    `written_type` / `result_type` into `folded_types`; a byte-array result
    is typed `ByteArray` even though its lattice form is a string, so
    shimmer never reports a conversion the runtime does not perform.
11. **A folded value has no source spelling.** `value_provenance` records it
    with `literal_span: None` and rename abstains; `regex_source` does the
    same for a folded pattern.
12. **Rewrites are gated separately from folds.** Entering the lattice
    authorises propagation and branch decisions; proposing a source edit
    additionally requires the O-code's own presentation gates (printable
    ASCII, size, `is_value_safe_bare_word`, quoting through
    `render_propagation_word`). A `binary format` result of non-printable
    bytes folds, decides branches, and is never spliced into source.
13. **Spans rebase.** Any new span-carrying lattice fact is added to
    `lattice_rebase.rs` in the same change, or memoised units drift.
14. **Interprocedural seeds keep their gates.** A folded argument may seed a
    callee only under `declines_seeding`, `trusts_proc_binding`, the
    trailing-`args` break, and caller-enumerability, exactly as a literal
    does today.
15. **Method bodies keep the dispatch barrier.** `MethodDispatchBarrier::allows_locals`
    gates propagation in a `TclOO` method as it does today; a transfer on
    an instance variable is widened by the escaping set before it runs.
16. **Depth caps answer conservatively.** Every cap the engine and the
    passes carry (`MAX_CONST_SUBST_DEPTH`, `MAX_BRACKET_TEXT_DEPTH`,
    `MAX_EXPR_NODE_DEPTH`) keeps its "past the cap, assume the unsafe
    answer" direction.

## Folding

A pure fold today reaches the lattice only for a whole-word `[cmd …]` on
the right-hand side of an assignment (`fold_assign_value`), a `foreach`
list, and — through the five name-keyed arms — `list`, `format`, `llength`,
`string length`, and `expr`. Under the model:

- **Every `Pure` transfer folds in the shared lattice**, through the one
  engine, with trust from the memo-keyed snapshot. The five arms are
  deleted; `list`, `format`, `llength`, and `string length` already carry
  registry folders, and the `string length` arm's character-model gate
  moves into the registry folder through `FoldPolicy::characters`, which
  the `Pure` arm receives alongside `version`. Single-hop results stay
  byte-identical, which the existing `evaluate_def_*` tests pin.
- **`expr` is `Native(Expr)`**, because its inputs are the lattice
  environment and the substitution model, not literal words. The arm's
  logic moves unchanged behind the ID.
- **Nested words fold.** `word_subst.rs` already lifts the `[cmd …]`
  substitutions nested inside a statement's words for the detectors that
  need them; the engine folds a nested substitution in a composite word
  (`set y "a[string range foobarbaz 3 6]z"`) to a partial constant string
  through `rendered_properties`' post-substitution model, and the whole
  word only when every part folds.
- **`binary format` gets a native folder** over `tcl_cmd_core::binary::format`,
  typed `ByteArray`; `binary scan` gets a `Destructure` over
  `tcl_cmd_core::binary::scan`; `binary encode` / `decode` fold over the
  existing base64, hex, and uuencode cores. The result of `binary format`
  is a Tcl string whose characters are U+0000–U+00FF; the lattice stores
  that string and `folded_types` says `ByteArray`, which is what keeps S110
  honest.
- **`regexp` gets a `Destructure`** over `tcl-regex`, which `regsub`'s
  folder already uses: the match flag as the result, and each match
  variable's value as a write (an unmatched group writes the empty string;
  `-inline` and `-all` type through the existing `ReturnTypeHookId::Regexp`).
- **`lassign` derives** from `VarWriteTyping::ElementsOf`; `scan` from its
  existing folder plus the `Destructured` typing; `dict with` and `dict
  update` bind keys of a constant dict as a `Destructure` the analyser's
  `DictWith` handler then consumes instead of re-deriving.
- **Results are typed** (`folded_types`), which is what lets `type_infer`
  stop re-splitting `Const(String)` lists to recover element facts, and
  lets `common_aot_plan` use a folded constant as a materialisable slot
  (it refuses a constant without a singleton `TypeShape` today).

What does *not* fold, by rule: anything `ResultStability` does not mark
`ReferentiallyTransparent` (`clock`, `pid`, every `HTTP::…` reader, every
data-group lookup, `file normalize` — which `auto_path_eval.rs` already
declines except on an anchored path), anything whose head is untrusted, and
anything whose release-sensitive answer the profile cannot fix.

## Propagation

The propagation pass is the largest consumer and changes least in shape:

- **`sccp_value_literal` sees more.** After `append acc foo; append acc bar`,
  `$acc` at a later use is `Const("foobar")`; O100 forwards it, and because
  the defining statement is a computed write the code stays O100, not O102,
  exactly the split #1934 fixed for `incr`. `UseKind::VariableName`
  positions are never rewritten, so `append acc baz` is never turned into
  `append foobar baz`.
- **O129 / O116 / O118 fire with lattice arguments**, not only literal
  ones: `[string length $acc]` folds because `lookup_var` answers from the
  lattice, and the folded value's own use sites fold onward. The code
  selection by head spelling (`list` → O116, `lindex` → O118, else O129)
  is diagnostic granularity, kept.
- **`binary format`, `regexp`, `lassign`, `scan`, `dict incr`, `lset` join
  O129's surface** with no new code: `try_o129_fold` is name-agnostic
  below the two special cases.
- **The optimiser's SCCP re-run becomes a fallback**, needed only when the
  shared lattice was built under a different trust snapshot (a stale memo)
  or with a proven `TclOO` frame (`defining_class`), which the shared
  lattice never has.
- **Call-site seeding admits folded arguments.** `params_constants_from_call_sites`
  seeds only a *source literal* every caller agrees on
  (`uniform_literal_at`); a call `f [string range foobarbaz 3 6]` seeds
  nothing even when the argument folds at the call site. With the transfer
  in the shared lattice, the call-site evidence can carry the folded
  constant under the same gates (rule 14). The same change fixes a
  pre-existing typing bug: the seed is inserted as
  `Const(String("3"))`, and the `incr` transfer requires `Const(Int)`, so
  `proc f {n} {incr n}` called uniformly as `f 3` widens at the `incr`
  today — the seed must go through `parse_literal_value` like every other
  literal.
- **The lowering const map stays.** `Lowerer::const_map_stack` runs before
  SSA exists, records only `set var {literal}` inside a proc, and is
  cleared by any control flow; it resolves `proc $name …` and body words at
  lowering time, and nothing in the lattice can replace it. It is listed
  so that nobody tries.
- **The analyser's constant store migrates onto the engine.**
  `Analyser::const_strings` keeps its scope-chain lexical semantics (the
  analyser must stay sound with no whole-module view), but every consumer
  that today re-derives a constant from source — `last_literal_set_value_for_var`
  for W304, W300, W103; `infer_list_length_from_recent_set` for W231; the
  verbatim `args[idx]` read in W303's pattern collection; the raw `args`
  read in W121 — asks `lookup_const_string` or, where the unit is
  available, the lattice, and the four private re-scanners retire. The
  seven-evaluator table becomes two: the lattice for unit-level facts, and
  `const_strings` for the analyser's own walk, both folding through
  `ConstSubstCtx`.

## Inlining and the interprocedural summaries

- **`ProcSummary` never sees a lattice.** `constant_return` and
  `can_fold_static_calls` come from `classify_return`, a syntactic test:
  `return 42` is `Literal`, `return $s` where `s` is not a parameter is
  `Other`. A transfer that makes `s` constant inside the callee changes
  nothing here. The argument-*sensitive* O103 path is different:
  `evaluate_proc_with_constants` re-runs SCCP on the callee with the call's
  literal arguments, so `proc f {} {set s abc; append s def; return $s}`
  — which does not fold today — folds to `abcdef` at every `[f]` site the
  moment `append` has a transfer, with no change to the summary. To let
  the argument-*independent* fold and the bare-statement hint see it too,
  `summarise_returns` should consult a per-proc SCCP run with an empty seed
  and take `resolve_return_constant`'s answer; the trust and redefinition
  gates that today sit at the call site move with it.
- **`fold_tail_statement_under_lattice` goes generic.** Its
  `Statement::Incr` arm encodes "`incr` returns the value it assigned";
  `append`, `lappend`, `lset`, and `dict incr` return their new value too
  and are not handled. A `Cell` transfer answers "the result is the
  written value" for all of them.
- **The general proc inliner is unwired** (`rust/tcl-compiler/src/inlining/`
  is a pre-codegen IR transform whose only consumer is WASM codegen, and no
  production caller invokes it); the production inliner is
  `inline_uplevel.rs`, which splices `uplevel 1 $body` passthroughs before
  CFG construction. Neither reads constants. Where they meet the transfer:
  the inliner's three `Statement::Incr` sites are *shape* (α-renaming and
  eligibility) and stay; `capture_implicit_return_value` recognises only
  `AssignConst` / `AssignValue` tails and would accept a folded tail; and
  once a body is spliced, its `incr` / `append` on the caller's cells run
  the transfer in the caller's lattice, which is how a folded constant
  reaches across a call boundary without a summary.
- **`apply` lambdas** fold through the same argument-sensitive path once
  `lambda_literal.rs` hands the body to a fresh `FunctionUnit`; a computed
  `apply $fn` withdraws every seed in the unit, as today.
- **Method bodies** keep `method_barrier.rs`; a `Cell` transfer on an
  instance variable never runs because the escaping set widened the cell
  first (rule 15).

## Dead-code elimination

- **O107 (unreachable)** is "not in `SccpResult::executable_blocks`" and
  needs no change; it fires more as more branches decide, including the
  arm bodies of a constant-subject `switch` once the operand gap closes.
- **O108 (ADCE)** is a backward closure over def-use seeded by O109 / O126,
  pinning any def with a `PhiIncoming` or `Terminator` use, with side
  effects as the negative roots through `assignment_safe_to_delete_with_effect`.
  That predicate's `Statement::Incr` arm ("the write is the whole
  observable effect; deletable when the cell is dead and the amount word is
  pure") generalises to every `Cell` transfer: an `append` to a cell that
  is dead is removable when its value words have no observable side
  effect, which today is blocked purely by `_ => false`. A `Pure` transfer
  in statement position (`string range foobarbaz 3 6` on a line of its
  own) is a removable no-op for the same reason. A `Destroy` statement is
  never removed: `unset` of an unbound cell raises, and deleting it would
  remove an error.
- **O109 / O126 (dead stores, unused assignments)** keep their twelve
  guards; a `Cell` transfer changes only guard 2 (the purity of the
  right-hand side) and shrinks `collect_rmw_hidden_reads`'s job, because
  the read-modify-write read is now an SSA use with `UseKind::VariableName`
  rather than a textual diff.
- **O112 (constant-condition compounds)** evaluates against its own
  name-keyed `Env` projected from the lattice; more constants mean more
  firings, and its `switch` arms are subsumed by the branch facts below.
  Two of its limits are worth removing in passing: `try_eliminate_if`
  abandons the whole `if` when the first clause it meets is unfoldable,
  and `walk_statement` descends into `catch` bodies with the enclosing
  `Env` although the CFG models a `catch` body as one opaque call in the
  default build.
- **O124 (unused iRule procs)** is a call-graph fact and is unaffected,
  except that a `switch` proven to take `default` no longer counts the
  dead arms' calls as edges once the call graph reads reachability.
- **The Explorer's liveness dead-store view** (`dead_stores.rs`) deliberately
  excludes `Statement::Incr` and answers a different question from O109;
  it stays separate, per [optimisation-passes.md](optimisation-passes.md).

## Branch analysis

Branch facts are where the transfer pays for itself, and where the current
code has the sharpest asymmetry.

### `if`, `elseif`, `while`, `for`

Conditions are parsed expressions, so `evaluate_branch` already binds
`$var` operands from the lattice. Two gaps:

- **Command substitutions inside a condition are opaque.** `ExprNode::Command`
  is rejected by the evaluator, so `if {[string length $acc] == 6}` never
  decides even when `acc` is constant. The engine becomes the evaluator's
  command resolver: an `ExprNode::Command` text is folded through
  `ConstSubstCtx` under the branch's use versions, and the result takes
  part in the expression. This is one change in `evaluate_branch` and
  reaches every `Pure` transfer at once — `info exists`, `string is
  integer`, `dict exists`, `lsearch`, `regexp` without match variables.
- **`ConstSet` operands do not decide.** `env_from_uses` binds only a
  single `Const`. When exactly one operand is a `ConstSet`, the condition
  is evaluated per member: all true → taken, all false → not taken, mixed
  → open. This is what turns `set x [expr {$c ? "a" : "b"}]` followed by
  `if {$x eq "c"}` into a decided branch, and it is the mechanism behind
  the dead-arm finding below.

Loop-carried values still widen at the header phi; the `Cell` transfer
does not change that, and the bounded enumeration a `ConstSet` could give
(`i ∈ {0..4}` for a five-trip loop) is left to a later phase because
`static_loops.rs` already answers the post-loop question for `for` by
simulation. What the transfer does fix is the three disagreeing `incr`
models: `static_loops::exec_statement`, `intervals::transfer`, and the SCCP
arm resolve `incr x $n`, leading-zero amounts, and overflow differently
today, and all three become consumers of one `CellUpdate::Increment`
evaluator (the interval domain applies it as an interval add).

### `switch`

The CFG builder flattens only an exact, case-sensitive, no-fall-through
`switch` into a dispatch chain of `StrEq` branches; glob, regexp,
`-nocase`, and any fall-through arm stay one opaque `Statement::Switch`.
Even in the flattened form a whole-variable subject lowers to
`ExprNode::Raw` — deliberately, because `Raw` is the only operand that
preserves a backslash-bearing `${…}` name under both 8.x and 9.x close
rules — and `Raw` cannot be evaluated. The result, by form:

| Form | O112 (structured IR) | O107 / I231 (CFG) |
|---|---|---|
| exact, literal subject | fires | fires |
| exact, `$var` subject | fires when `var` is constant | never — `Raw` |
| exact + `-nocase`, or a fall-through arm | fires | never — opaque |
| `-glob` | fires (`pattern_matches` is glob-aware) | never — opaque |
| `-regexp` | never (bails) | never |

The design closes this with one side-car and one operand rule:

1. **`evaluate_branch` resolves a whole-variable `Raw` operand from the
   lattice.** The CFG operand shape does not change; the analysis-side
   evaluator, which already collects the condition's variables through
   `var_refs::vars_in_expr`, substitutes a `Const` for a `Raw` that is
   exactly a whole-variable reference. The flattened form then decides
   per arm for a constant subject, `executable_blocks` drops the dead arm
   bodies, O107 and I231 fire, and O101 stays suppressed on the synthetic
   chain as today.
2. **Opaque switches get arm decisions as a post-pass**, the way
   `existence_constant_branches` contributes today. For a `Statement::Switch`
   whose subject is `Const` or a `ConstSet`, each arm's pattern is tested
   under the registry's `case_list` descriptor — exact and `-nocase`
   through the string comparison, `-glob` through `string match`'s folder,
   `-regexp` through `tcl-regex` — and the decisions are recorded as
   `ConstantBranch`-shaped facts against the arm's `pattern_span`, with
   `-matchvar` / `-indexvar` written through a `Destructure` when the
   match is decided. O112 and the analyser's `switch_body_is_selected`
   then consume the same decisions instead of each re-implementing
   `pattern_matches`, and the `static_loops` simulator, which ignores the
   mode entirely, stops disagreeing.

Because `case_list` is an authorable descriptor, a private dispatch-table
command — an Expect-like `case` or a vendor `switch` wrapper — gets the
same arm analysis from its `.tclspec` with no hook at all; that is the one
way a pack reaches branch analysis, and it is data, not code.

Three findings follow from decided arms:

- **"Only ever lands on `default`"** is O112's existing whole-statement
  replacement ("matches no pattern; keep default") plus I231 on every arm,
  now for every form.
- **"This arm can never match"** — a `ConstSet` subject that misses an
  arm — is I231 today for the flattened form only. Deleting one arm while
  keeping the switch has no edit shape yet: `structure_elimination` only
  replaces the whole statement, and `statement_delete_rewrite_range`
  only deletes a statement. A new `Dce`-category code, provisionally
  **O131 — eliminate a switch arm that can never match**, owns the edit
  over the arm's `pattern_span` plus `body_span`; it joins the `full`
  profile beside O107 and O112.
- **"`default` is unreachable"** — a `ConstSet` subject every arm covers —
  is I231 on the default body.

### `catch`, `try`, and completion

A `catch` body is one opaque `Call` in the default build (`emit_opaque_catch`),
so nothing inside it enters the lattice; a `try` body enters only when its
handler shape permits. Transfers do not change that, and must not be used
to reason across it: the exception edge is a completion fact, which the
DSL excludes for exactly the reason the README gives. `catch {expr {1/0}}`
stays undecided because integer division by zero already declines to
fold. `error`, `throw`, `exit`, and `tailcall` promote to `Return`
terminators through `TERMINATES_BLOCK`, and a decided `switch` arm that
ends in one keeps that promotion.

### Existence, later

`Destroy` transfers make a flow-sensitive bound / unbound fact possible
(`info exists x` after `unset x` decides false; the `parameter is present`
fold survives an `unset` on another path), replacing the flow-insensitive
`scan_defined_and_unset` scan and its `command == "unset"` site. It is a
third lattice rung rather than a value, so it is sequenced after the value
transfers, with the same gates the existence post-pass applies today.

### Predicate refinement, later

Inside the taken arm of `if {$x eq "a"}` or the `a` arm of an exact
`switch $x`, `x` is `"a"` even when it was `Overdefined` before the test.
The existence-guard narrowing (`collect_existence_guards`, dominance-based)
is the precedent; a general edge-refinement for equality with a literal,
`string is CLASS`, and `switch` arms would feed both the value and the
type lattices. It needs a block-qualified lookup the per-value map does
not have, so it is recorded as a follow-on, not a phase.

## Every analysis, and what changes for it

Paths are relative to `rust/tcl-compiler/src/`. "Reads the lattice" means
`SccpResult::values`; "reads reachability" means `executable_blocks` /
`executable_edges` only. The last column is the change under this design,
or "none" where the pass is a beneficiary with no edit.

| Module | Fact it produces | Constant relationship today | Under this design |
|---|---|---|---|
| `analyses.rs` | the lattice vocabulary | — | unchanged; `analyses::ConstantBranch`, `ReadBeforeSet`, `UnusedVariable` are dead duplicates and go |
| `sccp.rs` | the lattice, reachability, constant branches | the seven name-keyed sites | the transfer dispatcher replaces `try_fold_cmd_subst`'s arms and the `Incr` / `foreach` / `unset` arms; `folded_types` added; `evaluate_branch` gains the command resolver and `ConstSet` evaluation |
| `const_subst.rs` | `[cmd …]` folds through `const_fold` | the engine | becomes the engine for every kind: word resolution, trust, nesting, escapes; gains the cell and destructure entry points |
| `type_infer.rs` | the type lattice | reads `values` for `lindex` indices and list literals; types `Statement::Incr` as `Int` | joins `folded_types`; stops re-splitting `Const(String)` lists; the two `mathfunc` name arms move to the registry `mathfunc` table |
| `types.rs`, `value_shapes.rs`, `word_expr.rs` | vocabulary and word helpers | — | none |
| `intervals.rs` | per-value integer intervals | seeds from `Const(Int)` / `Const(Bool)`; its own `Incr` arm, literal amounts only | seeds from `ConstSet` too; the `Incr` arm becomes the `Increment` evaluator applied to intervals; folded `llength` / `string length` results seed tight intervals |
| `interval_bounds.rs` | W230–W233 findings | through `intervals`; its own container-length map and name table | container lengths come from folded `list` / `split` / `lrange` values; the name table goes |
| `native_integer_proof.rs` | native-add evidence | reads `values` directly; rejects `ConstSet` | more provable additions; no edit beyond accepting `ConstSet` ranges |
| `value_provenance.rs` | written constants reaching a use, with source spans | its own φ / copy walk and `[list …]` folder | reads the lattice; a folded value enters with `literal_span: None` (rule 11); the `list` folder goes |
| `rendered_properties.rs` | may/must string-content flags | reads reachability | a folded constant gives exact flags; none required |
| `representation_plan.rs` | representation obligations | — | consumes `folded_types` for byte-array literals |
| `tcl_expr_eval.rs` | expression evaluation under `FoldPolicy` | the evaluator | gains the command-substitution resolver callback; `rand` / `srand` stay the one name check (non-determinism is an expression fact) |
| `word_subst.rs` | nested `[cmd …]` lift | `"expr"` check for the lifted form | the lift feeds nested-word folding; the check moves to the `EXPR_CONCATENATES_ARGS` trait, as `end_offset.rs` already resolves `expr` |
| `subst_nocommands.rs` | `[subst -nocommands]` evaluation | its own const map | reads the lattice through `lookup_var` |
| `static_loops.rs` | post-loop environment for a bounded `for` | its own `StaticValue` lattice and `Incr` arm; `exec_switch` ignores the mode | `exec_statement` dispatches on the resolved transfer; `exec_switch` consumes the arm decisions |
| `loops.rs` | the natural-loop forest | reads reachability | none |
| `dynamic_names.rs` | the name-blindness barrier | a gate | unchanged; every transfer runs under it (rule 6) |
| `existence_query.rs` | `[info exists]` recognition through `IntrinsicId` | the exemplar | unchanged; becomes the `ExistenceQuery` transfer's front |
| `var_observability.rs`, `var_refs.rs`, `var_resolve.rs`, `var_scoping.rs` | alias / trace lattice, reference scanning, place resolution | gates and helpers | unchanged; `var_scoping.rs`'s `upvar` / `namespace upvar` index refinement stays until `FrameArgLayout` carries it |
| `var_escape/` | local-versus-frame tagging for WASM | none | none; `info_subcommands.rs`'s audited allow-list is a `SubCommand` trait candidate on another axis |
| `place.rs`, `place_bridge.rs` | storage places and overlap | `Statement::Incr` read modelling | the bridge reads the transfer's target instead of the node; `namespace upvar` / `trace add variable` literals move to `FrameArgLayout` / `ESTABLISHES_VARIABLE_TRACE` (they are effect facts, not this axis) |
| `memory_ssa.rs`, `effect_ssa.rs`, `state_ssa.rs`, `world_state_ssa.rs` | versioned memory, effects, world state | none; already name-free | none |
| `def_use.rs` | def-use chains, `UseKind` | `VariableName` from #1934 | a `Cell` transfer's read is a `VariableName` use by construction |
| `dead_stores.rs` | the Explorer's liveness view | reads reachability; excludes `Incr` | none, deliberately |
| `dataflow_graph.rs` | the Explorer's data-flow graph | renders `LatticeValue` | renders `folded_types` and `TransferDecline` reasons |
| `side_effects.rs` | effect classification | name-free | none |
| `taint.rs`, `taint_interproc.rs` | taint colours | reads reachability and φ edges; never `values`; `Incr` passthrough | a folded value is untainted only when every input was; the transfer engine propagates colour through `Cell` and `Destructure` writes the way the `Incr` arm does today |
| `interprocedural.rs` | `ProcSummary`, `MethodSummary` | syntactic `classify_return`; `global` / `variable` / `upvar` name sets | `summarise_returns` consults a seedless SCCP run; the alias name sets move to `Traits::CREATES_SCOPE_ALIAS` + `FrameArgLayout` |
| `unit_scope.rs` | call-site seeding | literal-only, string-typed seeds | folded arguments seed under the same gates; seeds go through `parse_literal_value` |
| `command_binding.rs`, `alias.rs`, `realm.rs`, `registry_invocation.rs`, `dispatch_proof.rs` | trust, aliases, binding realm, dispatch stability | name-free gates | unchanged; `realm.rs`'s `namespace import` scan is a `StateTransition::Namespace` consumer candidate |
| `object_types.rs` | object-handle provenance | reads the type lattice | benefits from `folded_types` on constructor results; none required |
| `lambda_literal.rs`, `inline_uplevel.rs`, `inlining/` | lambda splitting, passthrough inlining, the unwired IR inliner | shape only | none, except `inlining/`'s `break` synthesis stays as is |
| `shimmer/` | S100–S103, S110 | reads `values` and reachability; `ArgTypeHint::transparent_from` | reads `folded_types` so a folded `binary format` is `ByteArray`; the `Incr` `Int` read comes from the transfer's `written_type` |
| `path_concat.rs`, `uri_split.rs`, `regex_source.rs`, `scan_predicate.rs`, `script_arg.rs`, `auto_path_eval.rs` | W201, IRULE3103, regex source spans, `scan` no-match proof, list-built scripts, `auto_path` evaluation | four private evaluators and a `scan` conversion table | `uri_split` already reads `values` and simply sees more; `script_arg` and `auto_path_eval`'s `[list]` / `file` / `info` arms become `Pure` folds (`file join`, `file dirname`, `file tail` gain registry folders — platform-conditional, so versioned by the profile's platform axis); `scan_predicate`'s table is `scan`'s own format grammar and stays |
| `common_aot_plan.rs`, `mixed_region_plan.rs`, `semantic_optimisation.rs` | AOT evidence and plans | reads `values` as evidence; refuses a constant without a singleton type | `folded_types` supplies the type; the boundary rule stands — a folded value never authorises live intrinsic dispatch |
| `slot_allocation.rs`, `signature_scan/`, `lattice_rebase.rs`, `environment_ingress.rs` | slots, signature scan, span rebasing, dialect ingress | — | `lattice_rebase.rs` shifts any new span-carrying fact (rule 13) |
| `lowering/` const map | `proc $name` and body-word resolution before SSA | its own literal-only map | unchanged (see § Propagation) |
| `cfg_builder/` | blocks, terminators, loop nodes | the `switch` dispatch chain and `Raw` subject | unchanged operand shape; `while` gains a `LoopNode` when the bounded-loop phase needs it |

## Every optimisation, and what changes for it

Producers are under `rust/tcl-compiler/src/optimiser/` unless named. The
profile column is the generated catalogue's; a new code joins a profile by
its `OptCategory` row in `rust/tcl-core-types/src/diag_code.rs`.

| Code | Producer | Constants consumed today | Under this design |
|---|---|---|---|
| O100 | `propagation.rs` (five sites), `branch_folding.rs` | `sccp_constants_for`, `sccp_value_literal`, `command_mutations`, operand types | fires on every new constant; the computed-write fallback stays O100 |
| O101 | `branch_folding.rs`, `expr_simplify.rs`, `propagation.rs` | `constant_branches`, the constants projection, `trusts("expr")` | more branches decide; `is_switch_dispatch` suppression unchanged |
| O102 | `propagation.rs` `run_load_forwarding` | def-use chains, `UseKind::Operand`, `is_externally_mutable`, `TraceInputs`; never `values` | unchanged — a literal load is a literal load |
| O103 | `propagation.rs` two shapes | `ProcSummary`, `trusts_proc_binding`, `evaluate_proc_with_constants` | the argument-sensitive path folds string-building callees at once; the summary path after `summarise_returns` reads a lattice |
| O104, O130 | `chain_fold.rs` | none — textual, strictly consecutive, literal-only | the classifier dispatches on the `Cell` kind (`Append` / `ListAppend`) instead of three names; non-consecutive chains and lattice-constant operands become foldable through the lattice value at the last write |
| O105, O106 | `gvn.rs` | reachability | unchanged; more reachability |
| O107 | `elimination.rs` | `executable_blocks` | more arms decide (see branch analysis) |
| O108 | `elimination.rs` ADCE | def-use, `assignment_safe_to_delete_with_effect` | `Cell` and statement-position `Pure` transfers become removable |
| O109, O126 | `elimination.rs`, `manager.rs` coupling | def-use, twelve guards | guard 2 generalises; `collect_rmw_hidden_reads` shrinks |
| O110, O113, O117, O120 | `branch_folding.rs`, `expr_simplify.rs` | operand types | benefit from `folded_types` on every version |
| O111 | `rust/tcl-lsp-server/src/lib.rs` | none | none |
| O112 | `structure_elimination.rs` | its own `Env` projection; `resolve_subject`, `pattern_matches` | consumes the arm decisions; the first-unfoldable-clause and `catch`-descent limits are removed |
| O114 | `pattern_recognition.rs` | all-versions `Int` typing | benefits from `folded_types`; none required |
| O115 | four sites | `trusts("expr")` | none |
| O116, O118, O129 | `propagation.rs` `try_o129_fold` | `ConstSubstCtx` over the re-run lattice | fire with lattice arguments; `binary format`, `regexp`, `lassign`, `scan`, `dict incr`, `lset` join O129 with no new code; presentation gates (rule 12) decide what is spliced |
| O119 | `pattern_recognition.rs` | none | none |
| O121–O123 | `tail_call.rs` | none | none |
| O124 | `unused_procs.rs` | `ProcSummary::calls`, `has_barrier` | none required; a reachability-aware call graph is a follow-on |
| O125 | `code_sinking.rs` | side-effect-free assignment shapes | a `Cell` statement is sinkable under the same rule as `Incr` today; optional |
| O127 | `propagation.rs` `run_store_to_load_forwarding` | def-use, memory SSA, "not SCCP-constant" | unchanged; a constant use now takes O100 instead |
| O128 | `end_offset.rs` | its own `lindex` / `lrange` / `lreplace` / `string …` / `llength` table | the length-position table becomes `ArgRole::Index` plus `ReturnElements` on the registry — another axis, listed as debt |
| **O131** (new) | `structure_elimination.rs` | arm decisions | eliminate a `switch` arm that can never match; `Dce`, `full` |

The `Optimisation` record (`code`, `message`, `span`, `replacement`,
`group`, `hint_only`) and the LSP surface — one `HINT` diagnostic per
finding with the replacement in `Diagnostic.data`, applied by
`apply_optimisations` through the `optimiseDocument` command — do not
change. The Explorer's `opt` view shows the new findings, and its `sccp`
view shows `folded_types` and decline reasons, which is the visibility the
code-review checklist asks for.

## Every diagnostic, and what changes for it

The catalogue has 197 diagnostic codes in fourteen sections
(`docs/generated/diagnostic_codes.md`); most are syntax, scope, version,
dialect, or protocol facts with no constant input, and this design does not
touch them. The rows below are the ones with a constant relationship.

### Codes that read the lattice today and see more

| Code | Producer | What it reads | Under this design |
|---|---|---|---|
| I230, I231 | `analyser/diagnostics/dataflow.rs` | `constant_branches` + `executable_blocks`; the existence fold re-run | every decided arm, every form of `switch`, conditions with command substitutions; the existence fold runs once |
| O100 (hint) | `compiler_checks.rs` `from_constant_branch` | every `ConstantBranch` | more branches |
| W124 | `dataflow.rs` `emit_invalid_ip_diagnostics` | every `Const(String)` in `values` | folded `format` / `string cat` / `append` results; the literal-substring anchoring with whole-statement fallback stays the model for a folded value |
| W233, W230–W232 (dynamic half) | `dataflow.rs` → `interval_bounds.rs` | `values` + reachability | tighter intervals from `incr` chains and folded lengths; **more true findings, and the same "whole interval outside the range" rule** |
| W210, W211, W213, W214, W220, H300 | `dataflow.rs` | reachability | fewer false positives in dead arms; a `Destroy` transfer later gives read-after-`unset` precision |
| W126 | `dataflow.rs` | the type lattice | `folded_types` |
| W123, W307, W308 | `var_command.rs`, `helpers.rs` | `Const` / `ConstSet` strings for `$cmd` dispatch and `dict with` | more resolvable dispatch heads; a folded head is `rename_safe: false` (rule 11) |
| S100, S101 | `shimmer/use_site.rs`, `shimmer/expr.rs`, `shimmer/commit.rs` | `values` + reachability; `is_valid_instance_of` suppresses on a known-valid constant | `folded_types` prevents a phantom conversion on a folded `binary format`; more suppressions on known-valid lists and dicts |
| S102, S103, S110 | `shimmer/thunking.rs`, `sharing.rs`, `byte_array.rs` | reachability | `S110` reads `folded_types` for a byte-array literal |
| T100–T106, IRULE3001–3004, W313 | `taint.rs` | reachability and φ edges; never `values` | unchanged; colour flows through the transfer engine |
| IRULE3101 | `taint.rs` `find_setter_constraint_warnings` | reachability + a literal-only prefix check | a `Const(String)` subject is checked directly, removing the false positive on `set p /a; HTTP::path $p` |
| IRULE3103 | `uri_split.rs` | `values` (`Const(String)` only) | folded operands |
| IRULE1005–1008, 1201, 1202, 3102, 4002, 4004, 5002, 5004 | `irules_checks.rs` | reachability | fewer findings in dead arms |
| W201 | `path_concat.rs` via `compiler_checks.rs` | reachability, rendered properties, taints | exact flags on folded values |

### Codes that are literal-only today and would gain

| Code | Bail today | Gain |
|---|---|---|
| W121 | raw `args` scan | asymmetric with W124; `set m 255.0.255.0; IP::addr $ip mask $m` |
| W127, W137, W141 | `value.contains('$') \|\| value.contains('[')` | a constant-propagated or `[string tolower CONST]`-folded option value |
| W146 | `LiteralValidationDecline::NonLiteralArgument` | the decline reason is exactly "not a statically known value"; a lattice-backed `lookup_var` converts declines into findings |
| W145, W147, W152 | literal option spellings | a folded option name |
| W303 | verbatim `args[idx]` in pattern collection | `set re {(a+)+$}; regexp $re $s` — the analyser already resolves that shape for *highlighting* and not for the ReDoS check |
| W230, W232 (syntactic half) | `has_subst` / `!is_literal_index` | `set l {a b c}; lindex $l 9` through a folded container length |
| W240–W242 | "intentionally shallow" literal condition text | `set n 0; while {$n} {…}` |
| W200, W138 | literal format strings | a folded `format` / `binary format` template |
| IRULE4004 | `value.contains('$') \|\| value.contains('[')` | a `set x [string range CONST 0 3]` in a per-request event becomes hoistable |

### Codes at risk from an unsound fold, in order of blast radius

1. **O100 / O101 / O102 / O112 / O129** rewrite source; a wrong fold is a
   miscompile of the suggestion. Rule 12 and the existing presentation
   gates apply.
2. **I230 / I231 + O107 / O108**: a wrong branch decision marks live code
   unreachable, and `executable_blocks` gates taint, shimmer, W210 / W211 /
   W220, and the IRULE flow checks — a spurious "unreachable" silently
   deletes a whole family's findings for that block. This is the
   highest-consequence silent failure and the reason evaluators may only
   widen.
3. **W124 / W233 / W230–W232**: a wrong `Const(String)` fabricates a finding
   at a span the user never wrote.
4. **W123 / W307 / W308**: a wrong constant resolves a command to the wrong
   target, and rename keys off it.
5. **S100 / S101**: a wrong constant *suppresses* a real finding.
6. **T100–T106**: taint never reads `values`, so a value fold cannot create
   a false negative; an unsound `executable_edges` could drop a tainted φ
   incoming.

Every gate in the contract rules exists to keep these six from moving in
the unsound direction; none of them is new, and the drift gate below is
what stops a future per-command arm from bypassing them.

### Families with no constant input

The E-codes (syntax and recovery), W001–W004, the usage and style codes
W100–W120 not listed above, the version-gate codes W135, W136, W139, W144,
W149, and W150, the scope codes W215–W218, W250, the security codes
W300–W312 other than W303, H301, TK1001–TK1003, BIGIP6xxx, IAPP7xxx,
SSLIC1xxx, and the IRULE event and structure checks not listed above are
unaffected. They are listed so that "every diagnostic" is answered rather
than implied.

<!-- value-transfers-tier-inventory -->

## Third-party commands

The registry describes about 2,200 commands across its packs; 17 command
modules carry a fold — 16 in the core `tcl` pack and one in the `spectcl`
pack that describes the DSL itself. The inventory by pack, counting
a module "pure-ish" when it declares `pure: true`, `Traits::PURE`,
`CSE_CANDIDATE`, or `ReferentiallyTransparent`:

| Pack | Modules | Pure-ish | With a fold |
|---|---|---|---|
| `tcl` | 164 | 52 | 16 |
| `tcllib` | 244 | 120 | 0 |
| `irules` | 1017 | 63 | 0 |
| `stdlib` | 247 | 23 | 0 |
| `tk` | 65 | 20 | 0 |
| `expect` | 36 | 1 | 0 |
| EDA (`specs/*.tclspec`) | 346 commands | — | 0 |
| `bpf`, `iapps`, `itcl`, `argparse`, `sslictcl`, `ticklecharts` | 97 | 0 | 0 |

Three tiers reach them, and a pack author picks by where the implementation
lives.

**Tier 1 — a native evaluator over a shared core.** For a command whose
runtime handler already has a Rust core the registry can reach
(`tcl-cmd-core`, `tcl-regex`, `tcl-syntax`), the fold is a thin wrapper and
the differential test proves it. This is the `binary` family, `regexp`,
`lassign`, `dict incr` / `append` / `lappend` / `set` / `unset`, `lset`,
and `file join` / `dirname` / `tail` / `extension` / `rootname` /
`split` (platform-conditional through the profile). For iRules, the
same tier covers the pure functions whose runtime handlers exist for WASM
parity and should be lifted to a core under the Family-B rule rather than
duplicated: `b64encode` / `b64decode` (the base64 core exists),
`crc32`, `md5`, `sha1`, `sha256`, `sha384`, `sha512` (a digest core shared
with the runtime), `htonl` / `htons` / `ntohl` / `ntohs`, `findstr`,
`getfield`, `substr`, `domain`, `URI::basename` / `path` / `query` /
`host` / `port` / `protocol` / `decode` / `encode` / `escape` /
`compare`, and `IP::addr A equals B` with literal operands. These matter
in `RULE_INIT` bodies (`set static::key [b64encode "…"]`) and for literal
arguments; a `switch -glob [HTTP::uri]` never folds because `HTTP::uri`
reads versioned world state, and that is the correct answer. Note that
`b64encode`'s spec today declares only a `Global`-side read effect and no
purity — the inventory makes such under-declared specs visible.

**Tier 2 — a `.tclspec` body.** For a command implemented in Tcl, or one
whose pack is already SpecTcl, the pack author writes `const_fold`,
`cell_fold`, or `destructure_fold` bodies that *call the real command*
inside the sandbox — `fold [string range $s $first $last]` is the whole
body, as `string.tclspec` shows — and the pack's corpus gate proves them.
The EDA packs are already `.tclspec`, so a vendor helper that is pure
(`get_property` is not; a string-formatting utility is) is authorable
today; most of what EDA scripts gain is the transfer on `lappend opts …`
chains and `switch $tool {…}` on a constant, which needs no pack change.
tcllib's Rust specs are the biggest tier-2 candidate set once they move to
SpecTcl: `base32`, `ip::normalize` / `prefix` / `mask` / `equal` /
`version` / `type` / `contract` / `collapse`, `uri::canonicalize` /
`isrelative`, `textutil::*`, `html::html_entities`, `json::json2dict` /
`list2json`, `csv::split` / `join`, `struct::list`, `struct::set`,
`math::statistics::{mean,median,min,max,…}`, `mime::*` decoders,
`fileutil::relative` / `lexnormalize` / `stripn`, `otp`, `ripemd`, `md4`,
`md5crypt`. Until then they are typed pure-but-unfolded, and the inventory
says so per command.

**Tier 3 — a user proc, through the interprocedural path.** A private
proc needs no spec at all: the argument-sensitive O103 path re-runs SCCP
on the callee under the call's constants, so every transfer the callee's
body uses folds through it (`proc pad {s} {append s "!!"; return $s}`
folds `[pad hi]` to `hi!!` once `append` has a transfer). A future
*transfer summary* — deriving a `Cell` transfer for a proc whose body is
`upvar 1 $name v; lappend v …` — would let callers apply it without
re-running the callee, and is what the `spec-author` inference emits as a
`cell_fold` today when the library is packaged.

What no tier reaches, by rule: anything reading versioned world state or
volatile sources (`clock`, `pid`, `RESOLV::lookup`, `class match` on a data
group, `table`, `winfo`, `font measure`, every `HTTP::` / `TCP::` / `SSL::`
reader, `AES::*` with generated keys), anything whose head is untrusted,
and anything a `Volatile` `result_stability` names. `static::` variables
are cross-event state and escape.

## The drift gate and the generated inventory

One `cargo xtask value-transfers` command, in the `make xtask-check`
family, with three halves in the shape of its neighbours:

1. **A source lint**, after `number-drift` and `segmentation-drift`: a Tcl
   command or subcommand name used to *recognise an invocation* — as the
   operand of `==` / `!=` / `matches!` on a `command`, `canonical_command`,
   `cmd`, `head`, or `sub` binding, as a `match` arm on such a binding, or
   as a `&str` constant compared against one — anywhere under
   `rust/tcl-compiler/src/` outside the registry-dispatch owners. A
   reviewed site carries `// value-transfer-ok: <reason>` on the line or
   in the comment block above it; the reason must name the axis the fact
   belongs to or state why it is irreducible. The `rand` / `srand`
   check in the expression evaluator, `scan_predicate.rs`'s conversion
   table, and the inliner's synthesised `break` are the expected waivers.
2. **A registry enumeration**, after `callback-inventory`: every command,
   subcommand, and form is resolved through `value_transfer_for_call`
   with a representative argument shape, and the result is written to
   `docs/generated/value-transfers.md` — pack, command, kind (`Pure` /
   `Cell(update)` / `Destructure` / `Destroy` / `Iterate` /
   `Native(id)` / none), the evaluator's origin (native, `.tclspec` body,
   derived), the target role, and a *gap* column for a command that
   declares `VarWrite` roles, `READS_BEFORE_WRITE`, `pure`, or
   `ReferentiallyTransparent` and resolves to none. The `append`,
   `lappend`, `dict incr` gap the issue names is a row in that file until
   it is closed, and the 183 pure-ish third-party commands are rows too.
   `--check` fails on drift, on an unclassified `VarWrite` command whose
   kind is none and whose gap is not waived, and on a stale waiver.
3. **A pinned-set test**, after `rust/tcl-registry/tests/analyser_hooks.rs`:
   the set of specs carrying each kind, swept over every loadable dialect
   and the shipped `.tclspec` packs, so a transfer cannot appear, vanish,
   or move without the test changing beside it.

The owner-resolution manifest in
[shared-utility-contracts-rust.md](../contracts/shared-utility-contracts-rust.md)
gains a row — surface *constant folding and value transfers*, owner
`tcl-registry` (`const_fold.rs`, `cell_fold.rs`) with `tcl-compiler`'s
`const_subst.rs` as the engine, gate `xtask-value-transfers` — so
`owner-resolution` proves the doc's claims resolve to live code and a
registered gate. The Explorer's `sccp` view and `tcl explore --show sccp`
render each statement's transfer kind and decline reason, which keeps the
generated inventory and what a contributor sees in the tool the same
artefact.

## Sequencing

Each phase lands with its tests, its KCS notes for any new or changed code,
`make codegen` for the regenerated catalogues, and the design-doc updates
named; each is independently shippable.

1. **The descriptor and the query, behaviour-preserving.** `ValueTransfer`,
   `value_transfer_for_call`, the derivation, the contract tests, the
   inventory and its gate, and the SCCP dispatcher re-expressing the five
   fold arms, the `Incr` arm, the `foreach` / `lmap` arm, and the `unset`
   scan as resolved transfers. Every existing `evaluate_def_*` and
   `sccp_with_builtin_folds` test pins byte-identical results. The two
   stale design-doc sentences are corrected. Exit: `sccp.rs` matches no
   command name; the inventory shows `incr` as `Cell(Increment)` and
   `append` / `lappend` as gaps.
2. **Cell transfers in the shared lattice.** `cell_fold.rs` with
   `Increment`, `Append`, `ListAppend`; `CommandTrustSnapshot` in
   `FnLatticeKey`; `folded_types`; the call-site seed typed through
   `parse_literal_value`; `assignment_safe_to_delete`,
   `fold_tail_statement_under_lattice`, `chain_fold`, `static_loops`, and
   `intervals` consuming the resolved kind. Exit: program (3) folds in every
   consumer; the three `incr` models agree on `incr x $n`, leading zeros,
   and overflow; O104 / O130 fold a non-consecutive chain.
3. **Branch facts.** The `Raw` whole-variable resolution in
   `evaluate_branch`; the command resolver for `ExprNode::Command`;
   `ConstSet` evaluation; the opaque-switch arm post-pass over `case_list`
   with `string match` and `tcl-regex`; O112 and the analyser consuming the
   decisions; O131. Exit: program (4) yields O112, I231 on the dead arm,
   and O107 on its body, for every switch form.
4. **Pure folds that are missing, and destructuring.** `binary format` /
   `scan` / `encode` / `decode`, `regexp`, `lassign`, `dict` cell
   operations, `lset`, the `file` string operations; `Destructure`
   consumers in the analyser (`dict with`) and W303 / W121 / W127 / W146
   reading the engine; the `word_subst` nested-word fold. Exit: program (2)
   folds and is typed `ByteArray`; the literal-only diagnostics in the
   table above read constants.
5. **SpecTcl.** The `cell_fold` / `destructure_fold` families across the
   four surfaces; `-native ID` resolution for every body family; the hook
   sandbox's `tcl::mathfunc::` retention; `spec_pack_key` threaded into
   the `compilation_unit` query; the studio picker and body carry-forward
   for subcommands; `spectcl_check` reporting; the `spec-author`
   inference; `transfer_hook_entries`. Exit: the `string.tclspec` port's
   `-native string::is` line does what it says, and a workspace pack's
   `cell_fold` reaches a diagnostic on the memoised path.
6. **Third-party evaluators.** The iRules pure functions over lifted cores,
   the tcllib candidates as their specs move to SpecTcl, and the
   interprocedural `summarise_returns` reading a seedless lattice so the
   argument-independent O103 sees folded returns. Exit: the inventory's
   gap column is empty for every command that declares purity.

Follow-ons recorded but not phased: the existence rung, predicate
refinement, bounded-loop enumeration, and proc-level transfer summaries.

## Decisions to ratify

1. **Name.** `ValueTransfer` / `value_transfer` for the descriptor,
   `cell_fold` / `destructure_fold` for the DSL families. The issue's
   working name "dataflow hook" describes the axis, not the field.
2. **`CellUpdate` is the shared identity** between native lowering and the
   lattice transfer, extended with the dict and list cell operations, with
   a contract test pinning agreement — rather than a second enum, or a
   single authored field that native lowering derives from.
3. **Subcommand and form scope from the first change**, including
   `assigns_variable_at` there, or `dict incr` cannot be described.
4. **`Destroy` and `Native` are never authorable**; `Iterate` is derived.
5. **`-native ID` resolves for every family**, closing the existing
   `const_fold -native` gap in the same change.
6. **Transfers run in the shared lattice**, keyed by `CommandTrustSnapshot`;
   the optimiser's re-run becomes the `defining_class` fallback.
7. **`folded_types` is a side map**, not a `ConstValue` variant, so `join`
   and every consumer of `LatticeValue` are untouched.
8. **O131 is the one new code**; "constant `binary format`", "constant
   `string range`", "`incr` chain", and "switch always default" are
   existing codes with wider inputs.
9. **The bignum rung** stays a canonical decimal `String` in the lattice,
   as `TclValue::Big` maps today; a versioned evaluator produces it and
   `native_integer_proof` declines it.
10. **`spec_pack_key` in the `compilation_unit` query** is accepted as the
    cost of pack hooks reaching diagnostics.

## File-path anchors

- `rust/tcl-compiler/src/sccp.rs` — the transfer function, `evaluate_def_with_folds`, `evaluate_branch`, `existence_constant_branches`, `BuiltinFoldInputs`, `TraceInputs`, `is_externally_mutable`
- `rust/tcl-compiler/src/const_subst.rs` — `ConstSubstCtx`, the fold engine every kind rides
- `rust/tcl-compiler/src/analyses.rs` — `LatticeValue`, `ConstValue`, `MAX_CONSTSET_SIZE`
- `rust/tcl-compiler/src/command_binding.rs` — `ModuleCommandMutations`, `CommandTrustSnapshot`
- `rust/tcl-compiler/src/var_observability.rs`, `rust/tcl-compiler/src/dynamic_names.rs` — the escaping and dynamic-name gates
- `rust/tcl-compiler/src/cfg_builder/cfg_lower.rs` — `lower_switch`, `switch_subject_operand`, `lower_opaque_switch`
- `rust/tcl-compiler/src/optimiser/propagation.rs`, `branch_folding.rs`, `structure_elimination.rs`, `elimination.rs`, `chain_fold.rs`, `end_offset.rs`, `manager.rs` — the consumers
- `rust/tcl-compiler/src/static_loops.rs`, `intervals.rs`, `interval_bounds.rs`, `type_infer.rs`, `unit_scope.rs`, `interprocedural.rs` — the analyses with private `incr` or literal-only models
- `rust/tcl-registry/src/spec.rs` — `CommandSpec`, `SubCommand`, `CommandForm`, `run_const_fold`
- `rust/tcl-registry/src/const_fold.rs` — the shipped pure folders; `cell_fold.rs` beside it is proposed
- `rust/tcl-registry/src/native_lowering.rs` — `NativeLowering`, `CellUpdate`
- `rust/tcl-registry/src/intrinsic.rs`, `semantic_operation.rs` — `IntrinsicId`, `SemanticOperationId`
- `rust/tcl-registry/src/pack_hooks.rs` — `HookFamily`, `HookInputs`, `CacheMode`, the slot tables
- `rust/tcl-spec-hooks/src/emit.rs`, `host.rs`, `sandbox.rs` — `answer_of`, `HookHost`, `SANDBOX_COMMANDS`
- `rust/tcl-spectcl/src/hooks.rs`, `loader.rs`, `export.rs` — the pack seam, `hook_source`, the native-ID tables
- `rust/tcl-spec-studio/src/coverage.rs`, `render_spectcl.rs`, `schema.rs` — the four-surface gates
- `rust/tcl-lsp-db/src/lib.rs` — `FnLatticeKey`, the `compilation_unit` and `registry` queries
- `rust/xtask/src/callback_inventory.rs`, `number_drift.rs`, `owner_resolution.rs` — the gate shapes to copy
- `ai/claude/skills/spec-author/SKILL.md` — the inference surface

## Test anchors

- `rust/tcl-compiler/src/sccp.rs` — the `evaluate_def_incr_*`, `evaluate_def_assign_value_folds_*`, and `evaluate_def_foreach_*` tests pin today's arms and become the phase-1 byte-identity gate
- `rust/tcl-compiler/src/optimiser/branch_folding.rs` — `switch_dispatch_branches_are_skipped`
- `rust/tcl-compiler/src/optimiser/propagation.rs` — `o103_folds_implicit_return_proc_cmd_subst` and `o103_folds_arg_sensitive_passthrough_cmd_subst`, the argument-sensitive path
- `rust/tcl-registry/tests/differential_fold.rs` — every fold against a real `tclsh9.0`
- `rust/tcl-registry/tests/analyser_hooks.rs` — the pinned-set shape
- `rust/tcl-spec-hooks/tests/const_fold_e2e.rs` — O129 driven by a `.tclspec` body
- `rust/tcl-spectcl/tests/spec_corpus.rs` — every shipped pack's hooks through the sandboxed host
- `rust/tcl-spectcl/src/loader.rs` — `native_hook_tables_cover_their_catalogues`
- `rust/tcl-spec-studio/tests/spectcl_roundtrip.rs` — the four-surface round trip
- `rust/tcl-vm/tests/dict_canonicalisation_parity.rs` — the list-rendering parity the folders depend on

## Related docs

- [sccp-core-analyses.md](sccp-core-analyses.md) — the lattice, the drivers, and the existence post-pass this design generalises
- [constant-folding-type-inference.md](constant-folding-type-inference.md) — the fold-versus-rewrite separation and the type lattice
- [command-registry.md](command-registry.md) — the `CommandSpec` field reference and the hook catalogues
- [lowering-dispatch.md](lowering-dispatch.md) — why `Statement::Incr` exists and stays
- [optimisation-passes.md](optimisation-passes.md) — pass ownership and the semantic-AOT boundary
- [interprocedural-analysis.md](interprocedural-analysis.md), [interprocedural-call-site-seeding.md](interprocedural-call-site-seeding.md) — the summaries and the seeds
- [pass-fact-ownership-matrix.md](pass-fact-ownership-matrix.md) — producer and consumer ownership this design adds a row to
- [precision-limitations.md](precision-limitations.md) — where the deliberate imprecision is recorded
- [../spec-packs.md](../spec-packs.md), [../spec-dsl-examples/README.md](../spec-dsl-examples/README.md), [../spectcl-design-e-deep-dive.md](../spectcl-design-e-deep-dive.md) — the DSL, its hook contract, and the `constraints` precedent
- [../contracts/command-spec-studio.md](../contracts/command-spec-studio.md) — the four-surface parity rule
- [../contracts/shared-utility-contracts-rust.md](../contracts/shared-utility-contracts-rust.md) — the owner manifest
- [../family-b-routing.md](../family-b-routing.md) — the shared-core rule the native evaluators follow
- [compiler design index](README.md), [design docs index](../README.md)
