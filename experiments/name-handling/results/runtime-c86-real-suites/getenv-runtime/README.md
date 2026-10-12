# Selected C8.6 current-host getenv worker

The registry admits `::tcl::clock::getenv` only for the selected native C8.6
profile, with measured NoHook metadata independent of purity and explicit
shipped backing. The runtime consumes the existing original byte conversion,
CString extent, result construction, usage and shared refusal owners. Unix
NativeEnv supplies an optional current byte environment view through the
platform Env seam: actual process bytes plus its existing per-host overrides.
Missing entries become fresh empty strings; unavailable capability remains an
operational refusal. The consumer checks entered-operation currency after the
host read and before result publication.

ParseFormatArgs and getenv share the same actual Rc pool address. Each genuine
object-command registration owns one clone and its once-only delete callback
releases that clone. Retiring either one retains the pool; retiring the last
implemented worker releases its literal references while returned lists retain
their original objects. The five other support workers remain absent. This
does not establish the original seven-worker constructor, full shared pool
lifetime, clock map order, or another release's worker conformance.

The [source and original controls](../getenv-question/README.md) identify the
real all.tcl stop at GetSystemTimeZone's relative `getenv TCL_TZ`, and measure
the current Unix byte channel, missing/arity behaviour, read-trace bypass and
environment-array mutation boundaries in fresh C providers. Those captures
remain unchanged. The existing Unicode-only browser/WASI adapters offer no
native byte capability; Unix evidence does not authenticate Windows handling.
Normal guest env element write/unset synchronisation remains a separate gap;
this getter reads current host data, not the bootstrap env array snapshot.
F5 remains unmeasured.

Focused checks passed against the frozen implementation:

- NativeHost's real child-process byte primitive covers ASCII, empty, absent,
  raw-name/raw-value bytes, actual override currency and separate host scopes.
- Runtime getter controls cover original counted-NUL/raw byte objects, raw
  results, dynamic arity after rename, seeded success errorCode, read-trace
  bypass and host changes after env-array bootstrap.
- Actual BrowserHost checks operational refusal past catch and genuine failed
  object-command creation releasing its pool clone while retaining the cause.
- Both pool deletion orders retain original result members and unchanged
  relative compiler epochs; the prior eleven ParseFormatArgs rows remain a
  regression obligation.
- Selected-profile assertions compare both direct registry shipped backing
  declarations to actual RegisteredBacking::Builtin and NoHook token recipes,
  and assert intentional absence on the other supported C profiles. The
  existing C9-only backing xtask supplies no C8.6 parity evidence.

The runtime bootstrap target passed7tests; NativeHost primitive and registry
getter unit each passed1test. The genuine child-process checks additionally
ran their single child test successfully. Exact commands, log/binary hashes,
outcomes and dependency scopes are in
[validation-02/validation.json](validation-02/validation.json). The first native
test compilation failed E0433 because the browser-host module is wasm32-only;
[that failure remains retained](validation-01/getenv-runtime-bootstrap-tests.log).
The corrected native-only test imports the existing BrowserHost source at file
scope and does not replace its environment data with a simulator.

The [fresh capture](capture-01/provenance.json) records rebuilt standalone
runner and matching C8.6 controls with unchanged real init/tcltest and original
all.tcl. Getter's first five arity/value/errorCode rows plus read-trace row
match byte-for-byte. Later normal env write/unset/whole-array replacement rows
diverge through the separate host synchronisation gap; their exact raw rows
are retained, not counted as getter conformance passes. ParseFormatArgs's
eleven rows match both fresh C and the prior original C8.6 output: four
successes and seven caught worker errors.

Fresh C object controls confirm two NULL hooks and identical original client
data. They explicitly retire the other five original support workers before
observing the two surviving owners. In both retirement orders, format literal
refcounts are two for one retained list, three for two lists, three after the
first worker retires, and two after the final worker retires. The surviving
worker still runs and retained values remain usable. These measurements do not
establish the original seven-worker constructor or inventory equivalence.

Fresh original C all.tcl selects var.test and exits0 with110total,90passed,
20skipped and0failed. The rebuilt runtime passes init, harness and orchestration
markers, gets past getenv, then
[exits2 before any suite](capture-01/runtime/all-tcl-var/stderr.txt):
`run_script: execution refused: native string bytes cannot be represented as
host Unicode at byte 3`. This is an operational refusal, not a caught worker
error; no runtime tcltest suite pass is claimed.

Only the five approved Rust owners changed: platform/lib.rs,
host-native/lib.rs, registry clock_.rs and tcl/mod.rs, and runtime cmd_clock.rs.
Targeted rustfmt and whitespace checks completed. Scoped dependency snapshots
before/after validation were unchanged: the standalone local path closure has
15packages, and the separately refrozen registry dev closure has25packages.
The latter includes the three temporary compiler diagnostic source files and
is labelled a diagnostic-frozen test build, not a normal production binary.
Standalone runner dependencies do not include compiler/LSP owners. Independent
xtask owner-resolution edits are outside both dependency claims. No complete
rust-check/prep-pr or cross-provider conformance pass is asserted.

Replay into a fresh destination with a freshly rebuilt runner:

```sh
python experiments/name-handling/results/runtime-c86-real-suites/getenv-runtime/capture.py \
  --output-dir /tmp/getenv-runtime-NEW \
  --runner /absolute/path/to/fresh/run_script
```
