# Optimisation passes

The Rust optimiser consumes the retained `CompilationUnit` facts after CFG,
SSA, SCCP, type, effect, and interprocedural analysis. Its implementation is
under `rust/tcl-compiler/src/optimiser/`, with GVN in `src/gvn.rs`.

## Pass ownership

- `manager.rs` orchestrates the pass sequence and groups findings.
- `propagation.rs` performs constant, copy, load, and command-substitution
  propagation, including O100–O103 and related literal folds.
- `expr_simplify.rs`, `branch_folding.rs`, and `pattern_recognition.rs`
  implement expression and structural rewrites.
- `elimination.rs` owns dead-code, dead-store, and scope-aware elimination;
  optimiser-authoritative O109 findings also feed Explorer dead-store views.
- `code_sinking.rs`, `tail_call.rs`, `unused_procs.rs`, `chain_fold.rs`, and
  `end_offset.rs` implement their named specialised rewrites.
- `gvn.rs` handles value-numbering and CSE candidates. A Tcl command call is
  only eligible when registry purity, result stability, completion, mutable
  world state, dispatch dependencies, and trace policy are all proven.

The stable O-code catalogue and priorities are registry/compiler data consumed
by these modules. Add a pass beside its Rust owner, add focused tests, and
surface any durable result through the generic Explorer contract.

## Catalogue and implementation ownership

`tcl-core-types::DiagCode` owns each O-code's stable spelling, category,
description, and optimisation-profile membership. `cargo xtask diag-tables`
projects that metadata into
[`docs/generated/optimisation_codes.md`](../../generated/optimisation_codes.md),
and its check mode plus unit test reject drift.

`optimiser::PassId` owns selectable pass ordering. The modules above own the
implementations and may share an O-code when they report the same user-visible
rewrite from different IR shapes. Consequently there is deliberately no second
hand-maintained one-code-to-one-module catalogue. Per-code KCS pages own the
user explanation; this design document owns pass-level architecture.

Backend semantic AOT transforms are a different control surface. They remain
independently disableable and off by default under
[`semantic-aot-optimisation.md`](semantic-aot-optimisation.md), and do not
receive O-codes unless they also produce a source-level optimisation finding.
The implemented guarded `string length` and sealed native-i64-add selections
therefore appear as typed code-generation/region-plan evidence, not as a new
O-code or as an implicit member of `readability`, `standard`, `full`, or
`aggressive`.

## Reuse by semantic AOT

The generated catalogue is the authority for the meanings and profile
membership of O100–O130. The groups below describe shared proof inputs; they
are not a second code-to-module catalogue. In particular, `PassId` covers the
nine selectable source-rewrite passes, while GVN, compiler checks, and the
paired O111 hint also have production sites outside that enum.

| Catalogue entries | Shared input relevant to semantic AOT | Current relationship |
|---|---|---|
| O100–O103 | SSA values and uses, SCCP constants, type facts, command binding, variable observability, and interprocedural call facts | Material inputs to the common direct-call, slot, and native-integer proofs. AOT consumes the retained analyses, never the emitted rewrite or its O-code. |
| O105–O106 | Registry-derived invocation legality, effects, mutable-world barriers, dominance, and loop structure | The invocation-legality primitives also serve executable semantic analysis. The common AOT selector does not consume GVN, PRE, CSE, or LICM candidates. |
| O107–O109, O112, and O126 | SCCP reachability, SSA def-use, place/alias facts, and effect tests | Inputs overlap with conservative AOT reasoning, but DCE, dead-store, and structure-elimination results are not AOT evidence. Current AOT does not use an O107 reachability decision to erase a region or an O109 result to erase storage. |
| O110, O113, and O114 | Tcl expression parsing, type facts, and integer semantics | Native-integer proof reuses the common expression/type/range substrate. It does not consume expression rewrites or the `incr` suggestion. |
| O116, O118, and O129 | Registry const-fold identities, result stability, command-binding trust, traces, and shared Tcl primitives | Guarded intrinsic planning overlaps with those registry and dispatch facts. A compile-time folded value or O129 finding cannot authorise live intrinsic dispatch. |
| O104, O111, O115, O117, O119, O120, O128, and O130 | Primarily source-form, readability, or local pattern evidence; O104/O130 also consult variable observability before folding writes | No material result currently feeds semantic AOT. A later selector may reuse a common parser or primitive, but must construct its own typed proof. |
| O121–O125 and O127 | Call graph, CFG, def-use, purity, and use-placement facts | Some inputs are common, but tail-call, recursion, unused-procedure, sinking, and single-use-inline findings do not select AOT regions or frame plans. |

The current fact boundary is:

| Fact owner | Source-optimiser use | Semantic-AOT use |
|---|---|---|
| `CompilationUnit` / `FunctionUnit` SSA and def-use | Propagation, elimination, GVN, and data-flow diagnostics | Exact caller values, result identities, and materialisable-slot candidates |
| SCCP value lattice | Constant propagation, branch folding, and elimination | Constants and interval seeds; the source rewrite is irrelevant |
| Type lattice and `intervals` | Expression and idiom legality | Actual-argument types and non-wrapping integer ranges |
| `command_binding`, registry invocation facts, and module trace/mutation summaries | Builtin folding, propagation, GVN legality, and trace-sensitive rewrites | Direct-procedure identity, internal operation identity, dispatch dependencies, and guard obligations |
| `var_escape`, `var_observability`, and place facts | Dead-store/load-forwarding and chain-fold suppression, plus diagnostics | Frame-private and materialisation premises; neither fact alone proves frame elision |
| Interprocedural summaries and call-site evidence | O103 and unused-procedure/call analyses | Binding-safe direct callers and joined actual types/ranges |
| GVN/PRE/LICM findings | O105/O106 suggestions | Not consumed |
| DCE/dead-store findings | O107–O109/O112/O126 suggestions and Explorer reporting | Not consumed |
| Liveness results | Dead-store reporting and slot coalescing | Not consumed by the common AOT plan today; liveness cannot substitute for escape, frame-observer, completion, or deoptimisation proof |

`PassContext` scratch state and `Optimisation` edit groups remain owned by the
source optimiser. Semantic AOT owns typed evidence, decline reasons, guard
dependencies, and slow-path/materialisation obligations. A backend consumes an
immutable selected plan; it must not reinterpret an O-code as execution
authorisation.

## Remaining shared-utility debt

The audit found the following bounded duplication. These are consolidation
targets, not permission to broaden a proof while moving code:

- `common_aot_plan` has a local function iterator although
  `CompilationUnit::functions` already supplies deterministic traversal.
- Direct-call collection and native-integer caller recovery traverse the same
  CFG/SSA statement again. Nested command parsing is already shared through
  `value_shapes`, but the call-site/actual-value carrier should become one
  common executable-word fact rather than two walkers over flattened text.
- `native_integer_proof` repeats interval join, finite-bound, checked-add, and
  `i128`-to-interval helpers because the corresponding `intervals` operations
  are private or narrower. Those operations should move to the central range
  lattice before another numeric consumer appears.
- `intervals` still has its own radix/literal parser, while native proof uses
  the dialect-aware Tcl expression conversion policy. Centralising literal
  conversion is required before interval seeding itself can carry
  version-sensitive numeric authority.
- The Explorer liveness dead-store detector and optimiser O109 elimination
  deliberately answer different questions, and slot allocation has a separate
  name-level liveness representation. They should continue sharing SSA, place,
  and observability primitives, but their outputs must not be treated as
  interchangeable.

## Soundness rule

Missing alias, frame, dispatch, completion, trace, or world-state evidence
causes a pass to abstain. Purity alone does not authorise command-call CSE or
code motion. Optimisation findings are facts and edit plans; consumers do not
reconstruct pass logic.

## Related

- [Compiler pipeline overview](compiler-pipeline-overview.md)
- [Common semantic compiler contract](common-semantic-compiler.md)
- [Explorer coverage contract](../contracts/explorer-compiler-coverage.md)
