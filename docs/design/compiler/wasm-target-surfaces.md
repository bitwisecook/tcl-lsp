# WASM target surfaces: WASI and the browser

`runtime/rust` selects its capability host independently of Tcl naming,
compiler admission and original-object ownership. The bytecode VM browser module
and browser LSP/explorer modules have separate hosts and do not embed this runtime.

## 1. Target and host selection

`wasm32-unknown-unknown` selects `BrowserHost`; `target_os = "wasi"` selects
`WasiHost`. Both implement the shared [`Host`](../../../rust/tcl-platform/src/lib.rs)
contract in [host_wasm.rs](../../../runtime/rust/src/host_wasm.rs). A target triple
alone does not supply a filesystem, clock, process, socket, native object or
interpreter receipt.

The runtime crate has a WASM cdylib build. Numeric-tower availability is selected
by its build inputs and `build.rs`; an unavailable tower does not become a
supported numeric operation. The [runtime build instructions](wasm-codegen.md)
and [real-link tests](../../../rust/tcl-compiler/tests/wasm_real_link.rs) describe
that integration. Imports, exports and module size belong to the actual built
artifact; they are not constants of the host interface.

## 2. Current host capability boundary

| Capability | BrowserHost | WasiHost |
| --- | --- | --- |
| Clock | Seconds and milliseconds are fixed at zero | Uses the same fixed clock |
| Standard output/error | Discarded | Rust stdio writes and flushes through WASI |
| Environment | Empty; writes are ignored | Mutable virtual environment |
| Working directory | Fixed `/`; chdir is unsupported | Mutable virtual working directory |
| Filesystem | Absent | `wasm_stdlib` supplies seeded `MemFs`; otherwise absent |
| Sockets/processes/threads | No optional capability | No optional capability in this host |

`BrowserHost` exposes no JavaScript host-import facade. Its default optional
capability methods remain absent. `WasiHost`'s stdlib filesystem is an in-memory
store, independently of an operating-system filesystem or WASI preopens.

A fixed clock supplies no elapsed-time progression. Timer/event code and a
platform sleep primitive do not turn this host into a real clock. Command
availability, operation prerequisites and typed unsupported results remain
separate from whether a module builds or links.

## 3. Environment and library ownership

With `wasm_stdlib`, `WasiHost::new` seeds its own `MemFs` with the embedded Tcl
library and supplies its mount through `$TCL_LIBRARY`. Library loading still
requires that actual filesystem/environment and the runtime's independently
selected interpreter entry. `BrowserHost` neither seeds nor exposes this store,
so the feature flag alone supplies no browser library access.

The embedded scripts are runtime backing, not proof of native C Tcl or Jim
conformance. Oracle tests retain each selected interpreter's original matching
`init.tcl` and `tcltest.tcl`; a runtime copy cannot replace those inputs.
`Interp::set_exit` and `take_exit` retain the guest exit request for the actual
embedding host, independently of process termination.

## 4. Recorded build measurements

[Original build and size observations](../../../experiments/wasm-target-surfaces/recorded-build-measurements.md)
retain their source descriptions and exact recorded values. They have limited
provenance and establish no current source revision, Native execution or
cross-target performance claim. A new build has its own artifact and input
closure; source-level direct emission does not supply a measured timing result.

See [WASM code generation](wasm-codegen.md), [runtime primitives](wasm-runtime-primitives.md)
and [optional embedded library backing](wasm-extensions.md) for the current
compiler/runtime interfaces.
