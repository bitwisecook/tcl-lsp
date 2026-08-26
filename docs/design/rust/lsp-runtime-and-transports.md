# The LSP runtime seam and its three transports

The Tcl language server has one protocol core and three ways of driving it. The
core — `LspService<Backend>` in `rust/tcl-lsp-server` — decides everything
about Tcl and nothing about transport: it never names stdin, `postMessage`, or a
socket. What differs between the native binary, the browser worker, and the WASI
command is *how a message reaches it* and *what kind of async runtime is
underneath it*.

Those two questions are answered in two different places, and keeping them
separate is the point of this design:

| Question | Owner | File |
|---|---|---|
| What does `spawn` / `sleep` / `spawn_blocking` mean here? | the runtime seam | `rust/tcl-lsp-server/src/rt.rs` |
| How does a message get in and out? | the transport | `main.rs`, `tcl-lsp-server-wasm`, `tcl-lsp-server-wasi` |

A handler body calls `crate::rt::spawn` and never learns which arm it got. A
transport picks the arm implicitly, by choosing a target, and then owes that
arm a runtime shaped the way its documentation says.

---

## Part 1 — the runtime seam, in three arms

`rt.rs` is the single place the server expresses "there is more than one kind of
async runtime under me". Everything else in the crate — the `spawn_blocking`
call sites, the `JoinSet` fan-outs, the `sleep_until` deadlines — is written
once, against `rt`'s names.

The arms are selected by `cfg`, and the split is on `target_os` rather than
`target_family` for a reason recorded in the module: `wasm-bindgen` only binds
real JavaScript under `not(target_os = "wasi")`, so a wasip1 build that took the
browser arm links cleanly and then aborts on its first timer.

```
                    not(target_family = "wasm")        ->  native
    target_family = "wasm", target_os = "unknown"      ->  browser
    target_family = "wasm", target_os = "wasi"         ->  wasi
```

### Native — "nothing changed"

Plain re-exports of Tokio's own items. Same types, same semantics, same
scheduling as before the seam existed. `available_parallelism` is the one
function with a body, and it is the expression that used to be written out at
each of its call sites. The rule is deliberate: the native server must not be
able to regress because of a port.

### Browser (wasm32-unknown-unknown) — "same shapes, single thread"

There is one thread *and* no operating system: no clock a Rust program can
read, and nothing for a timer wheel to wait on. So the browser arm borrows the
host's event loop wholesale.

| Item | Browser implementation |
|---|---|
| `spawn` | `wasm_bindgen_futures::spawn_local`, output routed back over a oneshot so the handle still awaits to `Result<T, JoinError>` |
| `spawn_blocking` | runs the closure *inline* and hands back an already-finished handle |
| `JoinSet` | an ordered-arrival `FuturesUnordered`; every call site is spawn-all-then-drain, so the outputs match |
| `Instant` | `Date.now()` |
| `sleep` / `timeout` | `gloo_timers`' `setTimeout` futures |
| `available_parallelism` | `1` |

### WASI (wasm32-wasip1) — "Tokio, minus the thread pool"

wasip1 has no threads either, but it *does* have an OS: a real monotonic clock
and a `poll_oneoff` the Tokio timer driver can wait on. So `spawn`, `JoinSet`,
`yield_now`, `Instant`, `sleep`, `sleep_until` and `timeout` are Tokio's own,
unchanged.

`spawn_blocking` is the single exception. wasip1 has no thread-spawn syscall, so
Tokio's own compiles and then fails *at run time* (`os error 58`) the first time
the blocking pool tries to grow. The closure runs inline instead, as it does in
the browser, with its value handed to a real `tokio::task::spawn` so the return
type stays Tokio's and every call site compiles untouched.

### What a wasi host owes this arm

These are not style preferences. Each one is a process abort if ignored.

1. **`Builder::new_current_thread().enable_time()`.** `rt-multi-thread` cannot
   start a worker.
2. **Never let the runtime park with nothing pending.** wasip1's `std` has no
   condvar. Tokio's `park_timeout(d)` has a wasm branch that becomes
   `thread::sleep(d)` and is fine; its `park()` — reached when the ready queue
   is empty, nothing is deferred, and no timer exists — goes to
   `Condvar::wait`, which panics with `condvar wait not supported` and, because
   wasip1 cannot unwind, aborts the process.
3. **Panics are fatal.** `panic = "abort"`. There is no `catch_unwind` recovery
   to write, because the process is gone before one could run. The same applies
   to the salsa `Cancelled` unwinding and the condvar-blocking `set_text` wait
   the native server relies on: both are sound on wasip1 today *only* because
   the inline `spawn_blocking` means no analysis snapshot ever survives across
   an await. A transport that reintroduces real concurrency there turns
   cancellations into aborts.

---

## Part 2 — the three transports

| | `tcl-lsp-server` (`main.rs`) | `tcl-lsp-server-wasm` | `tcl-lsp-server-wasi` |
|---|---|---|---|
| Target | host native | wasm32-unknown-unknown | wasm32-wasip1 |
| Artefact | executable | cdylib + bindgen glue + worker | WASI command module |
| Wire format | Content-Length over stdio | one JSON-RPC string per `postMessage` | Content-Length over stdio |
| Event loop from | Tokio multi-thread + a blocking-stdin thread | the browser's | its own — see Part 3 |
| File source | `vfs::NativeStore` | `vfs::MemoryStore`, filled by the host | `vfs::NativeStore`, over preopens |
| Threads | many | one | one |
| Stack budget | 64 MiB per worker thread | 64 MiB, `-zstack-size` | 64 MiB, `-zstack-size` |
| `wasm-opt` | — | **no** (breaks the externref table) | **yes** (no bindgen glue to break) |

Three properties are shared by all three, and a fourth by two of them:

- **The two protocol shims.** `normalise_request_uris` on the way in,
  `inject_type_hierarchy_provider` on the way out. Every transport applies both,
  at the message boundary.
- **Detached dispatch.** A request is handed to the service and the driver goes
  straight back to reading. It has to: during `initialized` the server issues
  `workspace/configuration` and awaits the client's reply, and that reply
  arrives on the same input channel the driver would otherwise have stopped
  reading.
- **`poll_ready` gating.** `LspService::poll_ready` is `Pending` while an
  `initialize` is in flight; that is the transport's way of holding everything
  else behind it.
- **The 64 MiB stack** (both wasm transports). The analyser's `analyse_body`
  recursion and the CFG builder's `lower_script` recursion cap their nesting
  depth, but a cap on the *number* of frames says nothing about how much stack
  those frames need — a 2 MiB stack overflows around nesting depth 130-140, well
  inside the cap (issue #996). rust-lld's default wasm stack is 1 MiB, and a
  wasm stack overflow is not a clean panic: it corrupts the shadow stack or
  traps with `unreachable`. Both wasm builds match the native budget with
  `-C link-arg=-zstack-size=67108864`.

### Why the WASI transport is not the browser one with different framing

The browser worker never has to solve the hard problem, because it never owns
the wait. `postMessage` delivers on the JS event loop; between deliveries, the
microtask queue and `setTimeout` keep detached work and timers moving *for
free*. The WASI transport has one thread, no reactor, and a `read` that blocks
the entire process. Everything the browser gets from its host, it has to
manufacture.

---

## Part 3 — the WASI driver

`rust/tcl-lsp-server-wasi/src/driver.rs`. One loop, five steps, in this order:

```
   ┌─────────────────────────────────────────────────────────────┐
   │ 1. route    every complete frame the decoder holds:          │
   │             requests -> service.call(), detached             │
   │             replies  -> the socket's response sink           │
   │ 2. flush    every queued server->client message to stdout    │
   │ 3. yield    rt::yield_now().await                            │
   │ 4. flush    whatever step 3 produced                         │
   │ 5. wait     poll_oneoff([fd_read(stdin), clock(slice)])      │
   └─────────────────────────────────────────────────────────────┘
                              ^                     │
                              └─────────────────────┘
```

### Step 3 is what makes step 5 safe

Tokio's `yield_now` does not wake the driver's waker inline; it *defers* it to
the current-thread scheduler's deferred list. So `block_on` does what it always
does with a pending top-level future — runs the entire ready queue — and then,
because a task was deferred, takes its **non-blocking** park
(`park_timeout(0)`), which fires every expired timer before polling the driver
again.

Two guarantees fall out of that, and both matter:

- By the time step 5 runs, every CPU-ready task has run and every due timer has
  *fired* — waker called, task queued — so blocking the thread costs nothing
  that was ready to make progress.

  One yield is not quite one full drain, and the gap is worth naming. `block_on`
  polls the top-level future — the driver — *before* the tasks the park just
  woke, so a timer that fires during the park hands its continuation to a queue
  that is not serviced until the next pass through the loop. A timer's
  continuation can therefore run up to about **two** slices after it was due,
  not one. That bounds the continuation, not the firing, and nothing in the
  server needs the tighter bound.
- The runtime is never asked to park *empty*. The loop's only `.await` is
  `yield_now`, and a deferred waker means `park_timeout(0)`, never `park()`.
  This is the caveat-2 abort, closed structurally rather than by hoping a timer
  happens to be pending.

**The invariant a change to this file must preserve: the driver must never
return `Pending` without having arranged its own wake.** Adding an `.await` on
anything that can genuinely be pending — a channel receive, a oneshot, or
`poll_ready` — reintroduces the abort. That is why `Driver::admit` polls
`poll_ready` with a no-op waker and re-probes on the next pass instead of
awaiting it, and why client replies are queued to a relay task rather than
awaited into the sink.

### Step 5's deadline is what keeps the timers moving

Tokio's timer wheel only advances when the runtime is driven, and the runtime is
not driven while `poll_oneoff` is inside the host. The deadline is therefore the
dominant term in how late a timer can be — it bounds the *waiting*, but not the
inline analysis a pass may run before it reaches the next wait, which on a large
document is the larger number, nor the extra pass noted above:

| Slice | Value | When |
|---|---:|---|
| `ACTIVE_SLICE` | 5 ms | within 500 ms of the last byte in either direction |
| `IDLE_SLICE` | 100 ms | otherwise |

Both are chosen against the deadlines the server actually keeps: the tightest is
the 50 ms diagnostics debounce, and the longest is the 10 s
`workspace/configuration` timeout. An idle session wakes ten times a second to
find nothing, which is the price of those long deadlines expiring on a silent
connection.

### The alternatives, and why they lose

**Blocking `read`, with an explicit drain first.** Keeps step 3's guarantee and
loses step 5's. With the thread parked in `read`, a timer that comes due while
the client is quiet does not fire until the client next speaks. The 10 s
configuration deadline on an idle session cannot be met at all, and a
`publishDiagnostics` sitting behind its 50 ms debounce is delivered only when
the user happens to type again.

**`tokio::time::sleep` as the wait**, sampling stdin non-blockingly around it.
This inverts the trade: Tokio parks for the true minimum of its own deadlines,
so timers are exact — but stdin is then only sampled once per slice and *every
request* pays that latency. A late timer is invisible to the client; a late
request is not. The wait belongs on stdin.

### Measured behaviour

The starvation scenario is the one that separates the designs, so it is worth
recording what each actually does. With the chosen driver, on the fixture
session in `test/e2e.mjs`: `didOpen` is dispatched, the client sends nothing at
all, and `publishDiagnostics` arrives — carrying a real `E003` finding, so the
analysis behind it ran to completion, not just the notification path. With the
naive blocking-`read` driver, the same session produces the `didOpen`
acknowledgement and then nothing: the process sits in `read`, and neither the
detached analysis nor its debounce timer ever advances. The client waits
forever for diagnostics the server has all the information to produce.

### `poll_oneoff` under a real host

The readiness wait is the crate's entire `unsafe` budget: one call, in
`src/wasi_poll.rs`, wrapped by a safe function that owns both the subscription
array and the event array. Verified against wasmtime 47.0.3:

| Situation | Result |
|---|---|
| stdin already has bytes | returns in ~0.3 ms with the `fd_read` event |
| stdin quiet, data 1 s later | returns at 0.99 s with the `fd_read` event |
| stdin quiet past the deadline | returns exactly at the clock deadline |
| the write end of stdin closes | returns readable; the following `read` reports 0 bytes, which is how end-of-input is detected |

A host that refuses an `fd_read` subscription on stdin is handled rather than
assumed away, though it is worth being blunt about how much is left. Only a
permanent refusal latches the degraded mode — `ENOTSUP`, `EINVAL` or `EBADF`,
the errnos that mean the host will never poll stdin; `EINTR` and other
transient failures are retried a bounded number of times and otherwise reported
as "nothing arrived", because treating a passing failure as a permanent one
would throw away the good path for the rest of the session.

Once latched, the wait becomes a clock-only `poll_oneoff` followed by a read —
and since `fill_buf` on wasip1 stdin blocks, that read parks the thread until
the client speaks. On such a host this *is* the naive blocking driver: both
requests and timers wait for the next client byte, so a debounce or a deadline
that comes due while stdin is quiet does not fire until then. The slice bounds
only how long the first pass takes to notice bytes that were already waiting.
This path is a floor that keeps an unpollable host usable at all, not a second
correct mode; wasmtime is not such a host, and neither is any host tcl-lsp
currently ships against.

### Reading and writing

stdin is read through `fill_buf`/`consume`, always consuming the whole buffer,
so `std`'s own buffering can never hold bytes that the readiness poll cannot
see. Frames are decoded incrementally (`src/framing.rs`): a partial frame yields
nothing and leaves the decoder ready for the rest, which a driver that has to
hand the thread back after every read cannot do without.

stdout has exactly one writer. Responses from detached calls and server-initiated
requests both land on one unbounded queue that the driver drains, so two
messages can never interleave on the wire, and the driver sees every byte that
leaves — which is what feeds the active/idle slice decision.

---

## Part 4 — the WASI host contract

A host running `tcl-lsp-server-wasi.wasm` (wasmtime, `@vscode/wasm-wasi`, any
other preview1 host) owes it four things.

1. **Preopen the workspace.** WASI preopens are what make `std::fs` real, and
   `vfs::NativeStore` is a literal delegation to `std::fs`. A directory the host
   grants — `wasmtime --dir HOST::/w`, or a `MapDir` — is a directory the server
   can walk, so folder scans, `source` resolution, the package database, and
   spec-pack discovery all work as they do natively. Anything outside every
   preopen is `NotFound`, which is exactly the store's documented contract for a
   missing file.
2. **Drain stdout.** There is one thread. A host that stops reading stdout
   eventually blocks the server inside `write`. The native transport solves this
   with a second task (`stdio_pump`, and the wedge history in
   [`contracts/lsp-transport-liveness.md`](../contracts/lsp-transport-liveness.md))
   precisely because it *can*; wasip1 cannot, so the obligation moves to the
   host.
3. **Provide a monotonic clock.** Both the driver's wait and the server's own
   deadlines need `poll_oneoff`'s `CLOCKID_MONOTONIC`.
4. **Expect the exit codes.** `exit` after `shutdown` ends the process with 0;
   `exit` without one ends it with 1, as the protocol prescribes. A closed stdin
   ends it with 0, matching what the native transport does when its stream ends.

Diagnostics go to stderr — stdout carries the protocol, and there is no console
to reach for.

---

## Part 5 — building and testing

```
make lsp-server-wasi        # release link + wasm-opt -Os -> dist/
make lsp-server-wasi-test   # the above, then the scripted sessions under wasmtime
```

`wasm-opt -Os` is run here and *not* for the browser build. The browser crate
skips it because binaryen rebinds wasm-bindgen's `__wbindgen_externrefs` export
from the growable externref table onto the fixed-size funcref table, which breaks
`Table.grow` at run time. This module has no wasm-bindgen glue, no externref
table, and no JavaScript to rebind against, so none of that applies.

The e2e harness (`test/e2e.mjs`) is a real LSP client: it spawns `wasmtime run`,
writes framed JSON-RPC to the child's stdin, and reads it back off stdout. Its
scenarios are chosen to pin the driver rather than the server:

| Scenario | What would break without the driver's design |
|---|---|
| `workspace/configuration` round-trips during `initialized` | a driver that awaited its own handler would deadlock on the reply |
| `didOpen` publishes diagnostics with the client silent | a blocking `read` starves the detached analysis and its debounce |
| the 10 s configuration deadline expires on a silent session | no deadline on the wait means no timer service while idle |
| ten idle seconds do not abort the process | the empty-park abort (caveat 2) |
| `shutdown`+`exit` → 0, bare `exit` → 1, closed stdin → 0 | the exit paths |
| go-to-definition into a `source`d sibling that is only on disk | `NativeStore` over preopens — the reason this transport exists rather than a re-framed browser one |

CI runs it in the `lsp-server-wasi` job, gated by the same path filter as
`lsp-server-wasm` because the two transports share their entire dependency
closure. It is the only place in CI where the wasip1 arm of `rt.rs` is
*executed* rather than type-checked.
