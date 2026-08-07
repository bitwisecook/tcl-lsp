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

1. **`SERVER WEDGED`** — the server did not answer even a request that touches
   no document. The fault is the server process or the connection, not the
   test. Look for a panic or a deadlock in the server log; the stuck test is a
   symptom, not the cause.
2. **`DOCUMENT PIPELINE WEDGED`** — the server answers a document-free request
   but no document request at all. Document intake is stuck for every document,
   so suspect the shared intake path rather than anything the failing test did.
3. **`THIS DOCUMENT'S QUEUE WEDGED`** — another document answers and this one
   still does not. The stall is specific to this document — the hypothesis
   [issue #1294](https://github.com/bitwisecook/tcl-lsp/issues/1294) was filed
   to test. A run that reports this verdict is the evidence that ticket asked
   for; attach the whole failure block to it.
4. **`REQUEST DROPPED, NOT WEDGED`** — a retry of the same request on the same
   document answered. The queue is draining and the original request was lost
   or merely slower than its budget, so treat it as a latency problem, not a
   hang.

Each line below the verdict reports how long that question took, so a verdict
of "answered after 3900ms" on a 4000ms budget reads as a slow machine rather
than a healthy one, even when the starvation probe cannot confirm it.

If the verdict is missing entirely, the failing wait is not one that carries a
liveness probe. Add one at the call site with
`serverLivenessDiagnostic(docUri)` (`editors/vscode/src/test/helper.ts`), which
is the `diagnose` hook `bounded` accepts.

## Related

- [KCS index](../README.md)
- [Glossary](../../GLOSSARY.md)
- [kcs-issue-lsp-features-are-missing.md](kcs-issue-lsp-features-are-missing.md)
