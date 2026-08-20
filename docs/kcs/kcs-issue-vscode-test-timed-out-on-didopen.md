# KCS: a VS Code test timed out waiting for the server to drain didOpen

> **Audience:** Contributor
> **Type:** Issue

## Applies to

VS Code

## Question

A VS Code extension test failed with `timed out awaiting the server to drain
didOpen`, and you want to know whether the server was wedged, one document was
wedged, or the machine was simply slow.

## Symptoms

- One or more tests fail with a `VSCODE-WAIT-TIMEOUT` message naming
  `the server to drain didOpen for <file> (flush hover 1 of 2)`.
- The message says the machine looked healthy: `PROBE: could not confirm
  starvation`.
- Often several consecutive tests on the *same* fixture fail and the rest of
  the suite is unaffected.

## Answer

Read the `LIVENESS:` line that follows the probe note. The wait asks three
independent questions before it gives up, and the verdict names which of them
answered:

1. **`SERVER WEDGED`** — *none* of the three questions answered, including one
   that touches no document at all. The fault is the server process or the
   connection, not the test. Look for a panic or a deadlock in the server log;
   the stuck test is a symptom, not the cause. This is the one verdict that
   latches: the runner skips every remaining test rather than have each of them
   re-pay a full wait budget to rediscover the same dead server, so the report
   arrives in about the time a healthy run takes.
2. **`DOCUMENT-FREE REQUEST SLOW, TRANSPORT ALIVE`** — the document-free request
   did not answer inside its (deliberately short) probe budget, but a document
   hover did. A hover's reply travels the whole client → server → client path,
   so it contradicts a wedge outright; the run continues. Treat it as latency,
   and read the per-question timings below the verdict to see how close the
   budget was. Before
   [issue #1600](https://github.com/bitwisecook/tcl-lsp/issues/1600) this
   combination reported `SERVER WEDGED` and skipped the rest of the run — once
   at 687/899, where a byte-identical re-run then passed 899/899.
3. **`DOCUMENT PIPELINE WEDGED`** — the server answers a document-free request
   but no document request at all. Document intake is stuck for every document,
   so suspect the shared intake path rather than anything the failing test did.
4. **`THIS DOCUMENT'S QUEUE WEDGED`** — another document answers and this one
   still does not. The stall is specific to this document — the hypothesis
   [issue #1294](https://github.com/bitwisecook/tcl-lsp/issues/1294) was filed
   to test. A run that reports this verdict is the evidence that ticket asked
   for; attach the whole failure block to it.
5. **`REQUEST DROPPED, NOT WEDGED`** — a retry of the same request on the same
   document answered. The queue is draining and the original request was lost
   or merely slower than its budget, so treat it as a latency problem, not a
   hang.

Each line below the verdict reports how long that question took, so a verdict
of "answered after 3900ms" on a 4000ms budget reads as a slow machine rather
than a healthy one, even when the starvation probe cannot confirm it.

## What the server was doing

Below the three per-question lines is a capture of the process itself, which is
what turns a `SERVER WEDGED` verdict from "re-run it" into a diagnosis. Read it
in this order:

1. **`extension host: a 250ms timer woke Nx late`.** If that is `2x` or more,
   the extension host's own event loop was blocked and the request may never
   have left it — the server is not necessarily implicated at all.
2. **`server process: pid N, state S, T thread(s) … burned C CPU tick(s), read R
   byte(s) and wrote W byte(s)`.** These are deltas over a quarter-second, and
   the line after them says which shape they are:
   - CPU moving → the server is running. Suspect a spin, or a long computation
     holding a lock every other request needs.
   - no CPU but bytes moving → the transport is alive and the work is parked.
     Suspect a barrier (`edits_settled`) or a mutex, not the pipe.
   - neither moving → nothing is being read from stdin or written to stdout.
     That is the stdin-reader-stopped shape; see `transport_liveness.rs`.
   - `state Z` → the process is a zombie; it already exited.
3. **`server log: last N of M line(s)`.** The server's own `[timing]` markers,
   which name the last thing it got through before it went quiet.

If the capture says `pid unavailable`, the language client no longer exposes its
child process where `serverProcessId` looks (`helper.ts`) — a
vscode-languageclient upgrade is the usual cause. `waitDiscipline.test.ts` has
an assertion that fails on exactly that, so it should not reach you silently.

## Why the first question carries no URI

The first question is asked with **no URI** (`tcl-lsp.getEffectiveConfig` with
an empty argument), and that is load-bearing. Given a document URI the same
command runs `read_document`, which waits on the server's global `EditOrder`
barrier and then takes the `documents` lock and the salsa `db` mutex — every one
of which can be held by exactly the stall being diagnosed. It would then be a
document question wearing a transport question's label. If you add a liveness
check of your own, pass `""`.

If the verdict is missing entirely, the failing wait is not one that carries a
liveness probe. Add one at the call site with
`serverLivenessDiagnostic(docUri)` (`editors/vscode/src/test/helper.ts`), which
is the `diagnose` hook `bounded` accepts.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-issue-lsp-features-are-missing.md](kcs-issue-lsp-features-are-missing.md)
