# Namespace teardown, cached references and ensemble lookup

Namespace deletion separates fresh public-tree lookup from an original Tcl
namespace object's retained token. The runtime unpublishes a namespace after
its ordinary commands retire and before recursing into children, retaining its
display name and allocation for live frames and cached references. Original
delete operands use the shared two-pass `NamespaceDeleteBackend` operation.

The executable measurements in `native-ensemble-fix/receipt.json` identify the
native subject, selected profiles, binary SHA-256 and captured source patch.
`native-ensemble-fix/records.json` records actual matching-library process
results; `unit-records.json` records exact native regressions. Missing execution
policy is a host refusal, not a successful test or a guest Tcl result.

## Oracle behavior

`upstream-namespace-56.2.tcl` retains the exact upstream Tcl9.0.4 test body. Its
mutually entangled command-delete traces request the opposite child namespace
and then the empty current namespace. All five C references, with their real
matching init.tcl and Tcltest, report Total1, Passed1, Skipped0, Failed0. Commands,
library selection and input SHA-256 are in `original-oracle-records.json`.

C TclTeardownNamespace unlinks the parent's publication edge after its own
commands retire and before recursing into children (Tcl9.0.4
`generic/tclNamesp.c:1307–1338`). NamespaceDeleteCmd performs fresh
Tcl_FindNamespace lookup, validating all operands before mutation
(`:3391–3434`). A missing first operand therefore prevents an empty second
operand from deleting the global namespace. Changing empty-name grammar would
mask the lifecycle bug.

`cached-fresh-namespace-probe.tcl` compares a literal namespace object with a
newly constructed value (`set fresh ::P; append fresh ::C`). C 8.5–9.1 report
`OBS {{parent 1 1} {child 1 0}}`. GetNamespaceFromObj retains its nsName internal
representation while the selected leaf namespace, interpreter and reference
context remain valid (`generic/tclNamesp.c:2998–3029`); ancestor publication is
not revalidated. The child can therefore remain reachable through the literal
object while fresh lookup misses. A universal string memo cannot model this.

`parent-child-observation.tcl` measures ensemble command deletion callbacks.
C 8.5–9.1 report `{P 1 1} {PC 0 1 ::CE} {C 0 1}`. C 8.4 lacks namespace ensembles;
its capability error is preserved without a conformance claim.

## Actual native results

| Measurement | Runtime result | C reference |
| --- | --- | --- |
| Original namespace-56.2, real 8.5.19 init + Tcltest | Total1 Passed1 Failed0 | Total1 Passed1 Failed0 |
| Global survival, real 8.4/8.5/9.0/9.1 init | `0 set` | `0 set` |
| Cached/fresh, real9.0/9.1 init | `parent 1 1`, `child 1 0` | Same |
| Cached/fresh, real 8.5 init | `parent 1 1`, `child 0 0` | `parent 1 1`, `child 1 0` |
| Cached/fresh, explicit native core 8.5–9.1 | `parent 1 1`, `child 1 0` | Same |

The 8.5 library-initialized cache/literal-context difference remains open. The
passing core regression does not establish conformance after standard-library
initialization.

The original upstream test does not start on 8.4 (indexed-variable receiver
policy),8.6 (original command-entry epochs during init), or 9.0/9.1 (original
local alias table receipt). Exact refusal channels are retained in
`native-ensemble-fix/<patchlevel>/upstream-namespace-56.2.stderr`. Those four
captures provide no upstream test coverage.

## Implicit ensemble targets

`native-runner/ensemble-shadow-probe.tcl` measures namespace-local targets,
global shadows and a caller namespace. C 8.5 constructs the implicit command
head from the actual ensemble namespace's full name and parent state
(`generic/tclNamesp.c:6649–6656`), then dispatches globally. C 8.6–9.1 explicit
subcommand heads are bare and dispatch from the ensemble namespace. The adapter
uses the existing `NativeEnsembleDispatchProtocol` and native namespace token;
it does not infer identity from a rendered compatibility key.

All four C references select `::M::add` from both caller contexts despite
`::add` and `::Caller::add` shadows. With only global `::remove` present, C 8.5
reports `invalid command name "::M::remove"`; C 8.6–9.1 execute the global fallback.
The fixed native subject matches every control. Before-fix captures in
`native-runner/ensemble-shadow-before-*` preserve the misdispatch to `::add` on
8.5. C controls, inputs, raw channels and hashes are retained separately.

The deterministic regression
`implicit_ensemble_targets_preserve_the_selected_namespace_context` pins both
behaviors against these observed C results. Namespace teardown and original
object lifetime regressions use explicit native constructors; Logical fixture
measurements in `post-merge-baseline/` are separate historical subject captures.
