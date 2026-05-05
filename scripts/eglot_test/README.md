# Eglot debugging tools for tcl-lsp

Two scripts here, both pointed at GitHub issue #333 (highlighting goes
stale until file reload in Emacs).

## 1. `tcl-lsp-record-bug.el` — bug recorder for end-users

A self-contained Elisp file users can load in their Emacs to record what
happens around a tcl-lsp highlighting bug. The recorder captures:

- emacs / eglot / jsonrpc / eldoc / flymake / lsp-mode versions
- system info (`uname`, `lsb_release`, `system-type`, locale, tty/graphic)
- python / uv / tclsh versions
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

## 2. `test_issue333.el` + `run.sh` — automated reproducer

A batch-mode harness that:

1. Auto-installs eglot 1.23 from GNU ELPA into `tmp/elpa/`
2. Spins up `uv run python -m lsp` per scenario
3. Performs a sequence of in-buffer edits (sending didChange to eglot,
   which then exchanges semanticTokens/full and /full/delta with the
   server)
4. Snapshots `face` text-properties at the post-edit point
5. Kills the buffer and reopens (forces a fresh /full request)
6. Snapshots again and diffs

Mismatches reproduce the bug. Run with:

```sh
scripts/eglot_test/run.sh
```

Current scenarios: `rename-only`, `insert-line-top`,
`delete-line-middle`, `rapid-fire-no-wait`, `user-issue-code`,
`many-small-edits`. As of 2026-05-05 all six pass — eglot 1.23's delta
application matches a fresh full open in every case we've tried.

## 3. `exercise_recorder.el` — smoke-test for the recorder

A self-driving emacs that loads the recorder, makes some edits against
a live LSP, calls every public command, and dumps the resulting `.eld`
back to ensure it's readable. Mainly used to verify the recorder still
works after changes.

Run with:

```sh
emacs -Q -batch \
  -L tmp/elpa/eglot-1.23 -L tmp/elpa/jsonrpc-1.0.28 \
  -L tmp/elpa/eldoc-1.16.0 -L tmp/elpa/flymake-1.4.5 \
  -l scripts/eglot_test/exercise_recorder.el
```
