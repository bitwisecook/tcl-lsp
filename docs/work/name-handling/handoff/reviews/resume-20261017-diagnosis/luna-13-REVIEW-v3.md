# Luna-13 delta review v2 → v3: layer-a-v3.patch (sha256 ed83a1de…1340)

**Baseline.** HEAD 0703a8cd5. The six `la3/a` files are byte-identical to HEAD.

**Mechanical checks:**
- `git apply --check` passes;
- rustfmt 2024 is clean on all six files;
- the patch adds no `#[allow]`;
- v3 touches the same file set as v2, and `native_compilation.rs` and `original_namespace_binding.rs` are unchanged from v2.

**Method.** I diffed v2 against v3 with the hunk headers stripped. Only the B1 delta changed.

## Verdict: ADMIT

### The delta

v2's `ensemble_reconfigured` bool became `ensembles: EnsembleManagement { reconfigured, created }`.

- **Derives.** The struct derives `Copy`, `Default`, `Eq` and `Hash`, so the derives on `NamespaceResolutionProjection` still hold.
- **`record`.** `Exists` sets nothing. `Create` sets `created`. Everything else (Configure, Other, unresolved) sets `reconfigured`, which is unchanged from v2.
  - The `match` runs on `&Option<_>` with default binding modes.
  - `tcl_registry::state_transition` is `pub mod` (tcl-registry/src/lib.rs:245).
- **`join`.** It ORs each bit and reports a change.
- **N1 is fixed.** `NonBindingObservationStamp` now carries `namespace_ensembles`.
- **Consumers of `reconfigured`** are unchanged from v2: `original_ensemble_mapping_holds` and `with_late_lookup`.

### Consumers of `created`

1. **`original_named_selection_is_closed`** returns false, so the selection falls back to `select_for_facts`, which is HEAD behaviour. This is the option (b) I recommended.

2. **`successful_handler_transfer`** gains a guarded arm: `Required(_) | RequiredPath(_) if created => return None`. It sits before the existing `Required` and `RequiredPath` arms, and the `return` leaves the `and_then` closure, whose type is `Option<HandlerSourceTransfer>`.
   - **Sound.** It only removes a bounded route. The caller then takes the existing global residual: `residual.mark_opaque_binding_mutation()` in the `else` branch of `walk_unknown_compiler_handler`, command_binding.rs:9306-9311.
   - **Necessary.** Without it, c9's body `string repeat` (compiled → `Unknown` → `EnsembleLeaf` handler transfer) proves the stock worker against a namespace-named ensemble that replaced `::string`. HEAD and v2 have that same hole. So v3 closes more than v2 did: it is not merely a no-regression fix.
   - **Limited to compiled bodies.** The only caller is command_binding.rs:9267 (`walk_unknown_compiler_handler`), which runs only when `compiled.unknown` is set. Under the authoring `Direct` mode, `select_invocation` returns the default selection (compiled_invocation.rs:1017-1019), so `unknown` stays false. Root precision is therefore unchanged.
   - **Precision cost.** It applies only after an `ensemble create`, and only to worker-requiring members in compiled bodies. This is the same as HEAD's residual for those members except `string repeat`, `binary`, and the other `EnsembleLeaf` members, which become a residual after a create. That is acceptable.

### Are the c6 and c9 tests genuine?

- **c6** (create `-command ::namespace` inside `::fake`, after the prelude rename; then a body `namespace export h`; `incr x` must not be proved). Under v2 the gate admitted the export and proved `incr` (REVIEW-v2 B1). Under v3, gate 1 refuses it. Nothing else in the fixture refuses it: the prelude is renamed, an export needs no preload, and `::a` is fresh. **It discriminates.**
- **c9** (`namespace eval ::string {proc repeat …; namespace export repeat; namespace ensemble create}`, then `namespace eval ::b {string repeat a 2}`):
  - The earlier export runs before the create, so it passes.
  - The body `string repeat` goes gate 1 → `Unknown` → `successful_handler_transfer` → None (gate 2) → residual.

  Under v2 or HEAD, the `EnsembleLeaf` route proves it. **It discriminates the second consumer.**
- **Both are tclsh-proved hijacks** in 8.6 and 9.0 (`probe/v2-results.txt`: v2_c6 and v2_c9).
- **c7** shares the same `Create` operation, so it is covered without a test of its own.

### Anything else?

Nothing outside the B1 delta: the comments, the stamp field and the two test sources.

## Note (non-blocking): broaden the tracked gap

`namespace ensemble create -command X` can replace **any** command, not only a stock ensemble. tclsh 8.6 and 9.0, `probe/v3_c11_create_over_incr.tcl`:

```
namespace eval ::fake {namespace ensemble create -command ::incr -map {x ::fake::go}}
```

Afterwards `incr x` runs `::fake::go`, whose `return -code return` gives after=0. Two things follow:

- **Not a v3 regression.** HEAD does not model a create as a definition, at the root or in bodies, for Generic or inline commands, and v3 is no worse.
- **Wording to fix.** The tracked-gap line "Root Direct dispatch and inline members ignore a replaced or reconfigured ensemble" should say *any command replaced by an ensemble create*.

The registry accessor for the created or configured command (the follow-up both NOTES give) would let `create -command <literal>` be modelled as a definition and close this gap.

Everything else from REVIEW-v2 stands (N2, N3, N4): the configure precision cost, the root gaps, and the decision on the withheld fixtures.
