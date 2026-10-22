# First-diagnostic latency blocks ILX e2e

Source: `aeecf397`; isolated 4-CPU/16-GiB workspace. Pinned C Tcl
9.0.4/8.6.18 and Jim 0.84. No deadlines, assertions or load-scaling changed.

Build `cargo test --locked -p tcl-lsp-server --test e2e --no-run`, then run
the produced e2e executable with `issue1707_ilx_methods --test-threads=1
--nocapture` and delete it within the same command. All 16 selected ILX
tests fail before content assertions, waiting for the first version-1
`publishDiagnostics` within the harness's load-scaled 30-second barrier.
The scheduling probes report healthy conditions. Full receipt:
`server-e2e-20261022.txt` (the same invocation also passes all five XC
command cases and BIG-IP/APL W109).

The control `issue1707_ilx_methods::ordinary_tcl_named_ilx_call_is_untouched`
also times out. It opens language `tcl` with ordinary Tcl procedures named
`ILX::init` and `ILX::call`; it contains no `when` and should select no
Registry iRules spec. Minimal source preserving that fixture shape:

```tcl
proc ILX::init {a b} { return [list $a $b] }
proc ILX::call {h m} { return $m }
set handle [ILX::init ilx_plaintcl my_extension]
set reply [ILX::call $handle my_js_function]
```

The complete workspace/setup is in the existing test, including its
extension file. The source above alone has not been separately timed.
The server child consumed CPU during the barrier; this receipt does not
identify a specific hot compiler/provider function. Coordinator: route
latency investigation to the generic diagnostic/compiler/performance
owners. Required contract: an opened version receives its first current
diagnostic report within the existing barrier, including plain Tcl and
iRules. Keep the existing deadlines and content assertions. Once fixed,
BIG-IP must re-run all 16 ILX cases; the independent constructor/read
source-owner contract remains in `ilx-handle-owner-repro.md`.
