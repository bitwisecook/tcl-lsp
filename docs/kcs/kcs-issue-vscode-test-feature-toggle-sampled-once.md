# KCS: a VS Code feature-toggle test samples the provider once and is flaky

> **Audience:** Contributor
> **Type:** Issue

## Applies to

VS Code

## Question

A VS Code extension test disables a `tclLsp.features.*` toggle, waits for the
config change, then queries the provider exactly once and asserts on the
result — and it occasionally (or, worse, deterministically) sees the
*pre-toggle* answer. How do you fix the wait, and how do you tell that apart
from the toggle genuinely not working?

## Symptoms

- A test shaped like: disable a feature, `await waitForFeatureToggle(...)`,
  then a single `await vscode.commands.executeCommand("vscode.execute*Provider",
  ...)` immediately asserted against.
- The failure (when it happens) shows the *old* answer — the depth, item, or
  symbol the provider returned before the toggle — not an error and not an
  empty result.
- It reproduces rarely in isolation but more often under load or inside the
  full suite, because the race window it depends on is short.

## Answer

This is [issue #1295](https://github.com/bitwisecook/tcl-lsp/issues/1295):
`waitForFeatureToggle` (and `waitForEffectiveConfig` underneath it) only
proves the **server's effective config** changed — it says nothing about
whether a provider request issued right *now* will be answered under the new
config. There is no LSP event that announces "the next request reflects the
new toggle", so a single sample taken immediately after the config barrier is
racing an unobserved transition.

The fix is a **bounded wait on the result itself**, not a longer sleep:

```ts
const after = await waitForProviderResult(
  docUri,
  () => selectionRangesAt(docUri, pos),
  (r) => chainDepth(r) < depthBefore,
  { timeout: 10_000, label: "selection range depth to drop after disabling the feature" },
);
```

`waitForProviderResult` (`editors/vscode/src/test/helper.ts`) re-pulls the
provider on every `onDidChangeDiagnostics` publish plus a tight backstop
interval, and **rejects** rather than resolving with a stale value if the
predicate never holds. That makes the fixed test fast in the common case (it
resolves the instant the new answer lands) and loud in the broken case — a
timeout that names the label and the last value seen, not a silent pass
against the wrong result. `pollUntil` is the same shape for a query with no
useful diagnostics-publish signal to key off (e.g. a raw config read).

Two outcomes once converted:

- **It passes reliably.** The toggle worked; the bug was the single sample.
  This was true for every case fixed under #1295 in this codebase
  (`selectionRange`, `documentSymbols` — see the note on that one below —
  and a `folding` test tightened for consistency).
- **It fails deterministically at the bound.** Before concluding the feature
  toggle itself is broken, rule out the VS Code test-instrument problem
  below — it produces exactly this symptom and is not a product bug.

### A deterministic failure can still be a test bug, not a product bug

Converting `disabling features.documentSymbols removes LSP document symbols`
to a bounded wait initially failed *deterministically*, including in
isolation under `MOCHA_GREP`, with the provider always answering the
pre-toggle symbol set. That looked exactly like "the toggle is not applied to
in-flight provider registrations" — but the server-side e2e contract test
(`disabling_document_symbols_suppresses_provider`,
`rust/tcl-lsp-server/tests/e2e/config.rs`) already passed, and a raw
`textDocument/documentSymbol` request sent directly over the LSP connection
(bypassing the VS Code command) returned the correct, empty answer
immediately.

The command `vscode.executeDocumentSymbolProvider` goes through VS Code's own
outline/breadcrumbs model, which caches the last answer for a document and
does not invalidate it on a bare configuration change with no document edit —
the same class of problem already documented for
`vscode.executeFoldingRangeProvider` (see `foldingRangeViaLsp` in
`editors/vscode/src/test/configSettings.test.ts`). The fix was the same one:
send the raw LSP request via `getClient().sendRequest(...)` instead of the VS
Code command, which is what the registered provider actually receives.

**Rule of thumb:** if a bounded provider-result wait fails deterministically,
check whether an independent oracle agrees before concluding the toggle is
broken —

1. the Rust e2e contract test for that feature in
   `rust/tcl-lsp-server/tests/e2e/config.rs` (`TOGGLEABLE_FEATURES`), and
2. a raw `client.sendRequest("textDocument/...", ...)` call instead of the
   `vscode.execute*Provider` command.

If both agree with the raw LSP answer and only the VS Code command disagrees,
the bug is in the test's oracle, not the product. If the raw LSP request
*also* shows the stale answer, that is a real product bug and must be fixed
in the server (or the extension's feature gating), not worked around in the
test.

### Auditing for the same shape elsewhere

This is a shape, not a one-off: any test that establishes a baseline with a
bounded wait but samples the "after" state exactly once has the same defect,
whatever provider it queries. Search for a single `await
vscode.commands.executeCommand("vscode.execute*Provider", ...)` (or
`client.sendRequest`) sitting directly after a `waitForFeatureToggle` /
`waitForEffectiveConfig` call with no wait wrapped around it — that pairing
is the signature to grep for.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-issue-vscode-test-runner-reports-false-hang.md](kcs-issue-vscode-test-runner-reports-false-hang.md)
  — the companion fix (issue #1293) for the suite's launch-to-exit budget.
- [kcs-issue-vscode-test-timed-out-on-didopen.md](kcs-issue-vscode-test-timed-out-on-didopen.md)
  — a different, per-test wait timeout with its own three-way verdict.
