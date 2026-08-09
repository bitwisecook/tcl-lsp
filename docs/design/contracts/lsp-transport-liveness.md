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
request registration, cancellation, and exit side effects, but polls only four
ordinary returned handler futures at a time. It also forwards the inner
service's `poll_ready` state before each call. This matters during `initialize`:
later messages must not be admitted until initialisation has completed.

The resulting responsibilities are:

| Layer | Bound | Purpose |
|---|---:|---|
| stdin admission after initialisation | no finite bound | Client responses can always be routed. |
| active application handlers | 4 | Shared server state retains the upstream concurrency limit. |
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

`exit` and `$/cancelRequest` bypass the four-handler application limit. They
exist to stop queued or running work, so putting either behind that work would
make shutdown and cancellation least effective precisely when the server is
busy.

## Example

Suppose four workspace-folder notifications all pull configuration. The editor
delays those replies, then sends 400 hover requests. Under the old topology,
the first four handlers waited, the 100-item queue filled, and the reader never
reached the configuration replies behind the hover burst.

Under the production topology, the hover futures wait outside the four-handler
application limit. The reader continues to the replies, the four configuration
handlers finish, and the queued hover requests then run four at a time.

## Limits

- The pending-future set can grow with a request burst. This is deliberate: a
  finite input-admission bound recreates the deadlock at a different threshold.
  The peer is the local editor process, not an internet-facing client.
- This does not make a slow or faulty handler fast. Timeouts remain appropriate
  where the awaited operation has cancellation-safe cleanup. The configuration
  timeout predates this transport change and can retain one dependency waiter
  until a late reply arrives; it must not be copied to periodic requests.
- The stdout pump remains required. Unbounded input admission does not prevent
  a blocked stdout writer from propagating pressure through the response path.
- The wrapper is for the LSP service, whose returned futures are `'static`.
  Services which borrow from `call` need a different admission boundary.
- During `initialize`, the wrapped LSP service deliberately retains its own
  readiness barrier. The current initialise handler does not make a
  server-to-client request, so it cannot form the reply cycle described here.

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
