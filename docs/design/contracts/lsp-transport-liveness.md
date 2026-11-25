# LSP transport liveness

This contract keeps the language server able to read editor input even when
application handlers or stdout are slow.

## The two independent cycles

`tower-lsp-server` 0.23 joins one stdin reader, a bounded handler queue, a
four-handler pool, and one stdout writer. Two different forms of backpressure
can stop the only stdin reader:

```text
stdin reader -> task queue (100) -> active handlers (4) -> stdout
      ^                                      |
      |                                      |
      +---------- client replies ------------+
```

1. A client which stops reading stdout blocks outbound messages. Backpressure
   then fills the task queue and stops stdin.
2. Four handlers can wait for server-to-client replies. A request burst fills
   the task queue before stdin reaches those replies, so the handlers wait for
   input which the server no longer reads.

The cycles need separate fixes. `stdio_pump` owns the stdout side.
`transport_liveness` owns handler admission.

## Production topology

The production binary configures the transport's `buffer_unordered` limit as
`usize::MAX`. `FuturesUnordered` does not reserve that many entries. It means
the transport absorbs every request future already read from the local editor,
rather than stopping after four pending futures.

`DeferredConcurrency` calls the inner LSP service immediately, preserving its
request registration, cancellation, and notification side effects. It polls
only four ordinary request futures at a time; every JSON-RPC notification (a
message with no `id`) bypasses that limit. It also forwards the inner service's
`poll_ready` state before each call. This matters during `initialize`: later
messages must not be admitted until initialisation has completed.

The resulting responsibilities are:

| Layer | Bound | Purpose |
|---|---:|---|
| stdin admission after initialisation | no finite bound | Client responses can always be routed. |
| active request handlers | 4 | Shared server state retains the upstream concurrency limit. |
| notification handlers | no transport limit | Protocol progress cannot be overtaken by later requests. |
| stdout pump | no finite message bound | A slow reader cannot stop stdin. |
| configuration request | 10 seconds | A client which never replies releases its handler permit. |

The existing configuration timeout is defence in depth for application
capacity; stdin routing does not depend on it firing. It is intentionally not
generalised to every server-to-client request. In `tower-lsp-server` 0.23,
dropping a timed-out response future leaves its pending-map sender registered
until a late reply arrives. Applying that pattern to periodic refresh requests
would turn a non-responsive editor into unbounded retained state. A general
request timeout therefore needs cancellation-safe pending-request tracking in
the transport dependency first.

Every JSON-RPC notification bypasses the four-request application limit.
`exit` and `$/cancelRequest` exist to stop queued or running work, so putting
either behind that work would make shutdown and cancellation least effective
precisely when the server is busy. Document-sync notifications have the same
ordering requirement: `didOpen`, `didChange`, and `didClose` draw their
`EditOrder` ticket when their handler is first polled. A following read waits
only for tickets already drawn; delaying the preceding notification behind
saturated requests lets that read snapshot the barrier first and answer from
text older than an edit the server has already received.

## Example

Suppose four workspace-folder notifications all pull configuration. The editor
delays those replies, then sends 400 hover requests. With the upstream topology
alone, the first four handlers wait, the 100-item queue fills, and the reader
never reaches the configuration replies behind the hover burst.

Under the production topology, the hover futures wait outside the four-handler
application limit. The reader continues to the replies, the four configuration
handlers finish, and the queued hover requests then run four at a time.

## Limits

- The pending-future set can grow with a request burst. This is deliberate: a
  finite input-admission bound recreates the deadlock at a different threshold.
  The peer is the local editor process, not an internet-facing client.
- Notification futures can grow with a notification burst for the same reason.
  This is a deliberate protocol-ordering trade-off, not a new work queue:
  document mutations serialise through `EditOrder`, and notification paths that
  trigger expensive follow-on work must coalesce or bound it locally rather
  than relying on request admission for backpressure.
- This does not make a slow or faulty handler fast. Timeouts remain appropriate
  where the awaited operation has cancellation-safe cleanup. The configuration
  timeout can retain one dependency waiter until a late reply arrives; it must
  not be copied to periodic requests.
- The stdout pump remains required. Unbounded input admission does not prevent
  a blocked stdout writer from propagating pressure through the response path.
- The wrapper is for the LSP service, whose returned futures are `'static`.
  Services which borrow from `call` need a different admission boundary.
- During `initialize`, the wrapped LSP service deliberately retains its own
  readiness barrier. The current initialise handler does not make a
  server-to-client request, so it cannot form the reply cycle described here.

## Async mutex audit

The server denies Clippy's `await_holding_lock` lint for blocking mutexes.
The table records why Tokio mutex guards and their `TrackedMutexGuard`
wrappers may span additional awaits. A compound operation can take more than
one of these guards. Acquire them in the order used by the existing owner.

| Guard | Reason for holding it across an await |
| --- | --- |
| `db`, `db_files` | Analysis, navigation, provider capture and publication transactions pair a Salsa database view with the matching source handles or project inputs. The pair must stay current while the next guard in the transaction is acquired. |
| `folder_db_configs`, `folder_db_config_tombstones` | Folder settings publish the current configuration and retired handles together. Analysis and diagnostic capture keep the folder selection stable while acquiring the global fallback. |
| `db_project_members`, `db_project` | Folder configuration updates keep the project's member set and corresponding project input together while acquiring the active configuration. |
| `default_dialect`, `warned_session_dialect` | `setDialect` publishes the name with its explicitness bit. Configured-dialect changes serialise the same pair with the last warned spelling. |
| `rehoming_gate`, including `rehomed_index_guard` and guarded symbol snapshots | Navigation, rename, diagnostics, class factories, scans and source publication retain the selected index view while acquiring related read or publication guards. Rehoming cannot retire the viewed source generation midway through that operation. |
| `disk_publication_gate` | Disk publication and workspace folder changes must order rehoming and source publication before another disk transaction can publish. |
| `live_publication_gate`, including the carried `publication` guard in `pause_open_publication` | Deferred opens, live buffer changes, disk publication and proposal checks serialise the publication generation with its source inputs. The retry path releases the guard before joining a contended dependency's fair queue, then reacquires it and rechecks the generation. |
| `diag_slots`, `diagnostic_publisher.completion`, `pull_diag_cache` | Diagnostic scheduling and publication pair the current job, completion receipt and cached result. Closed-badge pruning pairs the pull cache with its generation records while it applies the open-document check. |
| `original_declaration_registry` | Logical lenses and hierarchy issue handles inside a transaction that checks the retained declaration and source generations. |
| `original_source_view_build_gate` | Source-view reconciliation admits one builder through asynchronous capture, validation and table publication. |
| `workspace_folders` | Workspace rename holds the active root selection through its comparison with the index and source-store receipt. |
| `workspace_scan_gate`, `watched_file_change_gate` | The scan guard admits one workspace fill. The watch guard orders a batch of file-change notifications through the asynchronous refresh steps it triggers. |
| `rehomed_source_seeds` | Workspace scan and library indexing reconcile pending rehoming with the autoloaded-library roster. |
| `spec_pack_reload`, `studio_sources`, `spec_packs` | Reload serialises discovery and publication. Publication and configuration reports retain the Studio snapshot and installed pack generation together while joining the current analyser inputs. Notices keep the installed set through enqueue and release it before waiting for delivery. |
| `external_pack_watch_globs` | While unregistering the previous watcher ID, the guard keeps the compared watch set current. The client request has a deadline. |

To refresh the inventory, run
`cargo clippy --locked -p tcl-lsp-server --lib -- -W clippy::await_holding_invalid_type`
with a copy of the repository's `clippy.toml` in `CLIPPY_CONF_DIR` and add
`await-holding-invalid-types = ["tokio::sync::MutexGuard", "tcl_lsp_server::tracked_locks::TrackedMutexGuard"]`.
Inspect each `await_holding_invalid_type` result in source. An explicit `drop`
can still appear in this inventory when the later await is inside the same
lexical scope. Prefer a narrower scope for a temporary that only supplies a
cloned value. Async transaction and publication owners stay documented here.

## Regression coverage

The tests use both controls and the production topology so a passing result
cannot come from a fixture which never reaches the fault:

| Case | Expected result | Guard |
|---|---|---|
| Raw upstream topology, client reply after 400 queued requests | four handlers start, no handler completes | Positive fault control using the identical reply fixture |
| Raw upstream topology, handlers finish after a short delay | all 400 requests drain | Negative control |
| Production topology, client reply after 400 queued requests | reply is routed and all handlers finish | False-negative guard for the cycle break |
| Deferred wrapper with twelve handlers and a limit of two | peak inner polling is exactly two | False-positive guard against unbounded application concurrency |
| Inner service is initially not ready | `call` runs only after readiness changes | Protocol-ordering guard |
| Generic no-id notification with a saturated request pool | its inner future is polled immediately | Protocol-progress admission guard |
| `didChange` followed by a read with a saturated request pool | the read observes the notification's state | Document ordering guard |
| `exit` and `$/cancelRequest` with a saturated application pool | both reach the inner service immediately | Shutdown/cancellation ordering guard |
| Request cancelled while queued for a handler permit | cancellation response is returned and the handler is never polled | Queued-request cancellation guard |

The paired upstream control is also a dependency-upgrade sentinel. If a later
`tower-lsp-server` routes the response itself, that control will fail and the
local admission workaround should be re-evaluated rather than kept by habit.

The consolidated LSP end-to-end suite repeats the real client shape with four
workspace configuration pulls and a 400-request burst. The VS Code suite is a
separate negative control: it sends a larger-than-queue provider burst through
the packaged extension and confirms that ordinary concurrent work remains
responsive under the new admission layer. VS Code's language-client test API
does not provide a safe hook for delaying its built-in configuration replies,
so the Rust end-to-end and paired transport fixtures carry the fault injection.

## Other transports

This contract is about the *native* transport, where the runtime has threads to
spare and the failures are backpressure cycles between two of them. The browser
and WASI transports drive the same `LspService<Backend>` with one thread each,
so their liveness problem is a different one: not "which of my threads blocked
the other" but "what keeps the single thread turning at all". The WASI driver in
particular has to manufacture an event loop that the browser gets free from
JavaScript and the native binary gets free from Tokio's blocking pool.

It reaches the same stdin-independence property this contract requires — a
handler waiting on a client reply can never stop that reply being routed — by a
different mechanism: `poll_ready` is probed rather than awaited. Requests that
the service will not yet admit do still queue, in the driver's own `held` deque
rather than in the service; what does *not* queue behind initialisation is
stdin routing and the delivery of client replies, which is exactly the part
this contract is about. See
[`rust/lsp-runtime-and-transports.md`](../rust/lsp-runtime-and-transports.md)
for the runtime seam's three arms, the driver loop, and the WASI host contract.
