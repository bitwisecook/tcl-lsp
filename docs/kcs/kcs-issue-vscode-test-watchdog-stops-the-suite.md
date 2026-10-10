# KCS: Why does the VS Code test watchdog stop the suite?

> **Audience:** Contributor
> **Type:** Issue

## Applies to

VS Code

## Question

`make test-ext` (or `npm test` under `editors/vscode`) exits 1 with a
watchdog report. How do you identify a stalled test, a slow suite, or an
extension host that never started?

## Symptoms

- The runner prints `--- watchdog report ---` and exits with a failure.
- The verdict names a lack of progress, the absolute ceiling, or a missing
  heartbeat, followed by `mocha did not complete. Treating as failure.`
- The report includes the last observed test counts and in-flight title
  when a heartbeat is available.

## Answer

The watchdog (`editors/vscode/src/test/runnerWatchdog.ts`) checks
progress and an absolute ceiling. It reads the heartbeat file the
extension host writes every 2s
(`.vscode-test/mocha-heartbeat.json`, or
`.vscode-test/mocha-heartbeat-multifolder.json` for `test:multi-folder`) and
reports a stall when `completed + failed` and the in-flight test title have
not moved for the no-progress window. Progress resets that window. An
absolute ceiling also bounds a run that keeps making progress without
finishing.

1. Read the message after `--- watchdog report ---`. It names which of three
   things happened:
   - **`no test completed for Ns (last progress: N completed, M failed, in
     flight: <title>)`** — a genuine stall. `<title>` names the test that was
     stuck; the heartbeat's server-probe line says whether the language
     server was still answering (suspect the test/extension host) or not
     (suspect the server).
   - **`absolute ceiling reached while still completing tests`** — the run
     was still making progress but took far longer than the generous
     ceiling allows. Check whether the suite has grown, or the machine was extremely loaded (the message
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
