# KCS: `make test-ext` reports "mocha never completed (likely hung)" on a green run

> **Audience:** Contributor
> **Type:** Issue

## Applies to

VS Code

## Question

`make test-ext` (or `npm test` under `editors/vscode`) exits 1 with `mocha
never completed (likely hung)`, but the log shows every test passed — how do
you tell a genuine hang apart from a run that was simply slow?

## Symptoms

- The run prints test results with `0 failed` (or a full green summary), then
  ends with `VS Code test runner did not exit within <N>ms after launch` or
  `mocha did not complete. Treating as failure.`
- The failure looks intermittent — it happens on some runs and not others,
  or only under load (several build trees, CI runners, or agent sessions
  sharing the machine).
- A run that fails this way and is simply re-run often passes.

## Answer

The watchdog (`editors/vscode/src/test/runnerWatchdog.ts`) bounds
**lack of progress**, not elapsed time. It reads the heartbeat file the
extension host writes every 2s
(`.vscode-test/mocha-heartbeat.json`, or
`.vscode-test/mocha-heartbeat-multifolder.json` for `test:multi-folder`) and
gives up only when `completed + failed` and the in-flight test title have not
moved for the no-progress window. A run that is still completing tests, or
still failing tests, or has just moved on to a new in-flight test, is never
killed however long it has taken. A generous absolute ceiling remains as a
backstop for a run that never stalls but also never finishes.

1. Read the message after `--- watchdog report ---`. It names which of three
   things happened:
   - **`no test completed for Ns (last progress: N completed, M failed, in
     flight: <title>)`** — a genuine stall. `<title>` names the test that was
     stuck; the heartbeat's server-probe line says whether the language
     server was still answering (suspect the test/extension host) or not
     (suspect the server).
   - **`absolute ceiling reached while still completing tests`** — the run
     was still making progress but took far longer than the generous
     ceiling allows. This is never reported as "likely hung"; check whether
     the suite has grown, or the machine was extremely loaded (the message
     includes the measured load factor).
   - **`no heartbeat file was ever written`** — the extension host failed to
     start, or `run()` never reached `mocha.run`. Look earlier in the log
     for an activation error, not at the watchdog itself.
2. If the verdict is the ceiling one and the suite has genuinely grown, the
   fix is usually a bigger no-progress window or ceiling, not a special
   case — see the env vars below.
3. To stretch (or, at `0`, disable) the absolute ceiling for one run:
   `TCL_LSP_VSCODE_TEST_EXIT_TIMEOUT_MS=<ms>`. To stretch (or disable) just
   the no-progress window: `TCL_LSP_VSCODE_TEST_NO_PROGRESS_TIMEOUT_MS=<ms>`.
   Both are base values — they are still scaled by measured machine load
   the same way every other wait in the suite is (see
   `editors/vscode/src/test/signal.ts`).

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [a VS Code test timed out draining `didOpen`](kcs-issue-vscode-test-timed-out-on-didopen.md)
  — a different, per-test wait timeout with its own three-way verdict.
- [a feature-toggle test samples the provider once and is flaky](kcs-issue-vscode-test-feature-toggle-sampled-once.md)
  — a single-sample "after" read racing an unobserved config transition.
