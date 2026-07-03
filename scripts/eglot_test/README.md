# Eglot debugging tools for tcl-lsp

Tools pointed at GitHub issue #333 (highlighting goes stale until file
reload in Emacs).

**None of this runs in CI or `make test`.** It needs a real Emacs, network
access to install eglot from GNU ELPA, and it exercises upstream eglot
internals we can't fix from the server side. Run it by hand.

## What issue #333 actually is

The reporter sees `eglot-semantic-*` faces *accumulate* on characters after
edits — a `keyword` char ends up wearing `keyword string variable function
namespace …` all at once — cleared only by reverting/reopening the file.

It is an **upstream eglot painter bug**, not a server bug:
`eglot--semtok-font-lock-2` ("repaint from stale-but-not-that-much local
properties") applies faces with `add-face-text-property`, which *appends*
to the existing `face` list instead of replacing it, and never strips the
prior `eglot-semantic-*` faces first. That path runs whenever eglot's
cached tokens are stale relative to the buffer's current version — i.e.
while a `semanticTokens` response is in flight. On large files the
round-trip is slow enough that eglot repaints several times before the
response lands, so the faces stack. (The reporter's "hang for a second" is
the *diagnostics* pipeline, not tokens — the server's `semanticTokens/full`
answers a 6000-line file in tens of milliseconds; see
`rust/tcl-lsp-server/tests/e2e/semantic_tokens_reference_client.rs`.)

The server can't fix eglot's painter, but it *shrinks the stale window*
by implementing proper `semanticTokens/full/delta`: a keystroke sends
only the changed tokens (like rust-analyzer), not the whole document.
This harness verifies that delta path end-to-end through real eglot.
(An earlier experiment added an "eglot-compatibility mode" that dropped
`full/delta`/`range`; the data showed it made no difference to the
accumulation, so it was removed in favour of the deltas.)

## 1. `tcl-lsp-record-bug.el` — bug recorder for end-users

A self-contained Elisp file users can load in their Emacs to record what
happens around a tcl-lsp highlighting bug. The recorder captures:

- emacs / eglot / jsonrpc / eldoc / flymake / lsp-mode versions
- system info (`uname`, `lsb_release`, `system-type`, locale, tty/graphic)
- tclsh versions
- the LSP server's advertised capabilities
- every `after-change-functions` event for the recorded buffer
- every JSON-RPC request, notification, and incoming message
- run-length-encoded snapshots of `face` text-properties at user-marked
  "this looks wrong" moments
- eglot's internal semantic-tokens state (`:data` length, `:docver`,
  region markers, etc.)

By default no source code or inserted text is recorded — only sizes and
positions. Set `tcl-lsp-bug-record-include-source` to `t` if you're
willing to share the buffer text.

### How to use it

```elisp
M-x load-file RET .../scripts/eglot_test/tcl-lsp-record-bug.el RET
;; open the Tcl file where you can reproduce the bug, with eglot running
M-x tcl-lsp-bug-record-start
;; ...edit until the highlighting glitches...
M-x tcl-lsp-bug-record-mark RET highlighting wrong here RET
M-x tcl-lsp-bug-record-stop
M-x tcl-lsp-bug-record-save RET ~/tcl-lsp-bug.eld RET
```

Attach the resulting `.eld` file to the issue. It is a plain Elisp
data file you can inspect before sending.

## 2. `test_issue333.el` + `run.sh` — automated reproducer + delta proof

A batch-mode harness that:

1. Auto-installs a semantic-tokens-capable eglot (>= 1.20) from GNU ELPA
   into `tmp/elpa/` if one isn't already there.
2. Builds (or reuses) the native `tcl-lsp-server` binary and launches it
   per scenario. Set `TCL_LSP_SERVER_BIN` to reuse a prebuilt binary.
3. Per edit scenario: performs in-buffer edits (sending didChange to
   eglot, which exchanges `semanticTokens/full` and `/full/delta` with the
   server), snapshots `face` text-properties, then kills+reopens the file
   (fresh `/full`) and snapshots again, diffing the two.
4. Runs `painter-accumulation` — a deterministic, client-only reproducer of
   the upstream eglot painter bug.
5. Runs `semantic-tokens-delta` — the real gate for our server change.

Run with:

```sh
scripts/eglot_test/run.sh
```

### The `semantic-tokens-delta` test

End-to-end proof of the server's incremental `full/delta`. It connects
real eglot, asserts the server advertises `full: {delta: true}` + `range`,
makes a token-changing edit, then confirms — from eglot's jsonrpc events
buffer — that the `full/delta` **response carried `edits`** rather than a
full `data` re-send. (`eglot--semtok-apply-delta-edits` is a `defsubst`
inlined into byte-compiled eglot, so the response is inspected directly
instead of advising that symbol.)

Unlike the painter scenarios, `semantic-tokens-delta` failing **fails the
suite** — it guards the server's delta implementation, which we control.
The CI-side counterpart lives in
`rust/tcl-lsp-server/tests/e2e/semantic_tokens_reference_client.rs`
(`server_returns_real_delta_not_full_resend`).

### Edit scenarios

`rename-only`, `insert-line-top`, `delete-line-middle`,
`rapid-fire-no-wait`, `user-issue-code`, `many-small-edits`.

Several are marked `:xfail` because they reproduce known upstream eglot
behaviour rather than a server bug:

- `rapid-fire-no-wait` exercises eglot's didChange request coalescing,
  timing-sensitive and known to drop intermediate edits on some
  eglot/jsonrpc combinations.
- `rename-only` / `insert-line-top` / `delete-line-middle` /
  `user-issue-code` trigger the same painter accumulation as
  `painter-accumulation` under CPU pressure — the delta paint leaves stale
  `eglot-semantic-*` faces the fresh-reload snapshot doesn't have.
- `painter-accumulation` is a deterministic reproducer for issue #333.

XFAIL scenarios that unexpectedly PASS are reported as XPASS so we remember
to drop the marker once eglot ships a fix.

The scenario diff deduplicates accumulated `eglot-semantic-*` faces before
comparing, so any positional mismatch flagged as `FAIL [diff]` reflects a
real LSP delta-correctness bug on our side. `INFO [accum-edited]` /
`INFO [accum-reload]` lines flag the upstream painter accumulation — they
are informational only.

If `painter-accumulation` ever PASSes, the upstream bug is fixed: drop its
`:xfail` marker *and* the accumulation-tolerance in `t333-diff` / the `INFO`
logging in `t333-run-scenario`, so real accumulation regressions are then
caught.

### CI-friendly counterpart

The correctness half of this — proving the *server* never drifts under
harsh editing — lives in a pure-Rust, no-Emacs test that does run in CI:
`rust/tcl-lsp-server/tests/e2e/semantic_tokens_reference_client.rs`. It drives
a spec-correct reference semantic-tokens client through brutal edit
sequences (mixed edits, edit-then-undo, rapid-fire bursts, a 5000+ line
file) and asserts both the server's `full` response and the reference
client's `full/delta` reconstruction always equal a cold reopen. As long as
that passes, any staleness this eglot harness shows is provably eglot's.

## 3. Diagnostic experiments (manual, non-CI)

Three standalone scripts used to characterise issue #333. They load the
harness helpers via `TCL_LSP_T333_NOEXEC=1` and print/write their own
report. None run in CI.

- **`compare_langs.el`** — drives eglot against several servers
  (`tcl-lsp`, `pyright`, `rust-analyzer`) on similar-sized (~6k line)
  inputs and reports how semantic tokens *appear* (connect → first
  tokens) and *update* (edit → fresh tokens) for each. Answers "is
  tcl-lsp's semantic-token timing typical for eglot or an outlier?".
  Measured here: `rust-analyzer` appear≈0.15s / update≈0.19s; `tcl-lsp`
  ≈0.55s / ≈0.47s; `pyright` emits no eglot semantic tokens at all. So
  tcl-lsp is sub-second and in the same ballpark as the reference
  server — the reporter's "several seconds" is not the token request.

  ```sh
  TCL_LSP_REPO=$PWD TCL_LSP_SERVER_BIN=$PWD/target/release/tcl-lsp-server \
    emacs -Q -batch -L tmp/elpa/eglot-1.24 -L tmp/elpa/jsonrpc-1.0.29 \
    -l scripts/eglot_test/compare_langs.el
  ```

- **`experiment_333.el`** — on a ~6k line Tcl file, runs the same edit
  sequence with eglot-compat OFF and ON, reporting connect→first-tokens
  and per-edit update latency plus accumulated-face counts, then a
  rapid edit/undo "stale window" stress. Batch has no redisplay, so it
  reports timings and (usually) zero accumulation — the accumulation
  itself only appears under a real GUI (see below).

- **`repro_interactive.el`** — the actual #333 reproducer. Needs a
  *real GUI Emacs* (batch never repaints; a software `xvfb` frame is too
  slow). Runs the rapid edit/undo burst with compat OFF and ON under
  real redisplay and self-classifies the outcome
  (`PROVEN` / `PARTIAL` / `NO-DIFFERENCE` / `NOT-REPRODUCED`). Use this
  to check, on your own machine, whether the compat mode actually
  changes the accumulation. See its header for the invocation.

## 4. `exercise_recorder.el` — smoke-test for the recorder

A self-driving emacs that loads the recorder, makes some edits against a
live LSP, calls every public command, and dumps the resulting `.eld` back
to ensure it's readable. Mainly used to verify the recorder still works
after changes.

Run with (adjust the ELPA version dirs to whatever `run.sh` installed):

```sh
emacs -Q -batch \
  -L tmp/elpa/eglot-1.24 -L tmp/elpa/jsonrpc-1.0.29 \
  -l scripts/eglot_test/exercise_recorder.el
```
