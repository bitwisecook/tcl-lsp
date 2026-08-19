# Emacs

tcl-lsp works with Emacs via **eglot** (built-in since Emacs 29) or **lsp-mode**.

## Prerequisites

The server is the native `tcl-lsp-server` binary — no Python, interpreter,
or runtime dependencies are required. Download the binary for your platform
from the
[latest release](https://github.com/bitwisecook/tcl-lsp/releases/latest),
or build it from source with `make rust-server` (or
`cargo build -p tcl-lsp-server`, producing `target/release/tcl-lsp-server`).

See the [Installation Guide](../../INSTALL-editors.md) for full details.

Point the server command at the binary — either its name (`tcl-lsp-server`)
if it is on your PATH, or an absolute path to it.

## eglot (Emacs 29+)

Add to your `init.el`:

```elisp
;; Dialect-specific derived modes so eglot sends a distinct `languageId` per
;; file family. Routing `.irul` / `.iapp` / `.apl` / `.exp` through plain
;; `tcl-mode` sends languageId "tcl", which the server maps to tcl8.6 — so the
;; F5 iRules / iApps and Expect dialects never engage. The `:language-id` in
;; each `eglot-server-programs` entry sets the id the server keys its dialect
;; on (see `dialect_from_language_id`).
(define-derived-mode f5-irules-mode tcl-mode "iRules")
(define-derived-mode f5-iapps-mode  tcl-mode "iApp")
(define-derived-mode expect-mode    tcl-mode "Expect")

;; The extensions each profile owns in the dialect catalog.
(add-to-list 'auto-mode-alist '("\\.irul\\(es?\\)?\\'" . f5-irules-mode)) ; .irul / .irule / .irules
(add-to-list 'auto-mode-alist '("\\.\\(iapp\\|iappimpl\\|impl\\)\\'" . f5-iapps-mode))
(add-to-list 'auto-mode-alist '("\\.apl\\'"            . f5-iapps-mode)) ; the iApp presentation language
(add-to-list 'auto-mode-alist '("\\.\\(exp\\|expect\\)\\'" . expect-mode))

(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '(tcl-mode . ("/path/to/tcl-lsp-server")))
  (add-to-list 'eglot-server-programs
               '((f5-irules-mode :language-id "f5-irules") . ("/path/to/tcl-lsp-server")))
  (add-to-list 'eglot-server-programs
               '((f5-iapps-mode :language-id "f5-iapps") . ("/path/to/tcl-lsp-server")))
  (add-to-list 'eglot-server-programs
               '((expect-mode :language-id "expect") . ("/path/to/tcl-lsp-server"))))

;; Auto-start on Tcl and the dialect modes
(dolist (h '(tcl-mode-hook f5-irules-mode-hook f5-iapps-mode-hook expect-mode-hook))
  (add-hook h #'eglot-ensure))
```

## lsp-mode

```elisp
(with-eval-after-load 'lsp-mode
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("/path/to/tcl-lsp-server"))
    :activation-fn (lsp-activate-on "tcl")
    :server-id 'tcl-lsp)))

(add-hook 'tcl-mode-hook #'lsp)
```

## Settings

Pass settings via eglot workspace configuration:

```elisp
(setq-default eglot-workspace-configuration
              '(:tclLsp (:dialect "tcl8.6"   ;; tcl8.4 | tcl8.5 | tcl8.6 | tcl9.0 | tcl9.1 | f5-irules | f5-iapps | f5-tmsh | f5-bigip | bpf | expect | spectcl | cadence-eda-tcl | intel-quartus-eda-tcl | mentor-eda-tcl | microchip-libero-eda-tcl | synopsys-eda-tcl | xilinx-eda-tcl
                         :formatting (:indentSize 4 :maxLineLength 120))))
```

`.apl` (and `.irul` / `.irule` / `.irules` / `.iapp` / `.iappimpl` / `.impl` /
`.exp` / `.expect`) files are handled by the dialect
derived modes in the eglot setup above, which send the correct `languageId` —
do **not** also map `.apl` to plain `tcl-mode`, or it would analyse as tcl8.6.

## Known issues

### Eglot semantic-tokens highlighting goes stale until file reload

**Symptoms:** after making edits, syntax highlighting becomes wrong —
identifiers show colors that don't match their actual token type, or
highlighting visibly degrades the more you edit. Saving (`C-x C-s`)
does not fix it. Reverting the buffer (`M-x revert-buffer`) or closing
and reopening the file does fix it.

**Cause:** Tracked at
[bitwisecook/tcl-lsp#333](https://github.com/bitwisecook/tcl-lsp/issues/333).
This is an upstream eglot painter bug: `eglot--semtok-font-lock-2`
repaints from stale local properties with `add-face-text-property`
(which *appends* rather than replaces) while a semantic-tokens response
is in flight, so each repaint stacks another `eglot-semantic-*` face on
the same character until a fresh full paint (buffer revert / reopen)
clears them. It shows up most on large files, where the round-trip is
slow enough for eglot to repaint several times before the response
lands. The server serves correct tokens throughout — verified by a
spec-correct reference client driven through the same edits
(`rust/tcl-lsp-server/tests/e2e/semantic_tokens_reference_client.rs`) — so
this is purely how eglot paints them.

The accumulation was reproduced against real Tcl code and measured
under several fixes (see `scripts/eglot_test/prove_fix.el`): the
**client-side painter advice below collapses a 7-deep face stack to a
single correct face**, whereas a purely server-side capability tweak
made **no difference** — confirming the bug lives entirely in eglot's
painter. The same accumulation reproduces with rust-analyzer and clangd,
so it is not tcl-lsp-specific.

The server does its part to *shrink* that stale window: it implements
proper `semanticTokens/full/delta`, so a keystroke transmits only the
changed tokens (a few bytes) instead of the whole document — the same
incremental behaviour rust-analyzer uses. That reduces how often eglot
is caught mid-refresh, but the definitive fix is still the painter
advice below. Lowering `eglot-send-changes-idle-time` also helps, by
keeping the round-trip short.

**Workarounds (pick one):**

1. **Apply this advice in your `init.el`** (recommended — this is the
   one that actually stops it). It strips stale `eglot-semantic-*` faces
   before each repaint, preserving every other face:

   ```elisp
   (with-eval-after-load 'eglot
     (defun tcl-lsp--eglot-semantic-face-p (x)
       (and (symbolp x)
            (string-prefix-p "eglot-semantic-" (symbol-name x))))

     (defun tcl-lsp--eglot-strip-semantic-faces (face)
       (cond
        ((null face) nil)
        ((tcl-lsp--eglot-semantic-face-p face) nil)
        ((symbolp face) face)
        ((and (consp face) (keywordp (car face))) face)
        ((consp face)
         (let ((new (delq nil (mapcar #'tcl-lsp--eglot-strip-semantic-faces face))))
           (cond ((null new) nil)
                 ((null (cdr new)) (car new))
                 (t new))))
        (t face)))

     (defun tcl-lsp--eglot-clear-semantic-face-properties (beg end)
       (with-silent-modifications
         (let ((pos beg))
           (while (< pos end)
             (let* ((next (or (next-single-property-change pos 'face nil end) end))
                    (face (get-text-property pos 'face))
                    (new-face (tcl-lsp--eglot-strip-semantic-faces face)))
               (if new-face
                   (put-text-property pos next 'face new-face)
                 (remove-text-properties pos next '(face nil)))
               (setq pos next))))
         (remove-list-of-text-properties
          beg end '(eglot--semtok-token eglot--semtok-faces eglot--semtok-names))))

     (advice-add 'eglot--semtok-font-lock-1 :before
                 (lambda (beg end &rest _)
                   (tcl-lsp--eglot-clear-semantic-face-properties beg end)))
     (advice-add 'eglot--semtok-font-lock-2 :before
                 (lambda (beg end &rest _)
                   (tcl-lsp--eglot-clear-semantic-face-properties beg end))))

   (defun tcl-lsp-repair-eglot-highlighting ()
     "Manually clean up accumulated eglot-semantic-* faces in the
   current buffer.  Useful if the advice above isn't installed and
   you want a one-off fix without reverting the buffer."
     (interactive)
     (tcl-lsp--eglot-clear-semantic-face-properties (point-min) (point-max))
     (when (boundp 'eglot--semtok-state) (setq eglot--semtok-state nil))
     (font-lock-flush)
     (font-lock-ensure))
   ```

   Originally posted by @georgtree on
   [issue #333](https://github.com/bitwisecook/tcl-lsp/issues/333#issuecomment-4380920940).

2. **Disable eglot semantic tokens for `tcl-mode`**, falling back to
   Emacs's built-in `tcl-mode` font-lock keywords (loses LSP-derived
   highlighting like distinguishing user procs from builtins):

   ```elisp
   (with-eval-after-load 'eglot
     (add-to-list 'eglot-stay-out-of 'eglot-semantic-tokens-mode))
   ```

3. **Revert the buffer** (`M-x revert-buffer`) whenever highlighting
   visibly degrades. Discards unsaved changes — only viable if you've
   just saved.

## Bracket matching and auto-pairs

Emacs's `tcl-mode` provides bracket matching out of the box via
`show-paren-mode` (enabled by default in Emacs 29+).  For automatic
insertion of closing brackets and quotes, enable `electric-pair-mode`:

```elisp
(add-hook 'tcl-mode-hook #'electric-pair-mode)
```

This auto-closes `{}`, `[]`, `()`, and `""` as you type.

## Debugging

### Viewing LSP logs

`M-x eglot-events-buffer` opens the `*EVENTS for <project>*` buffer
showing all JSON-RPC messages between Emacs and the server.

`M-x eglot-stderr-buffer` opens the `*STDERR for <project>*` buffer
showing server-side log output.

### Verbose logging

By default eglot truncates large messages.  To capture full
request/response payloads, add to your `init.el`:

```elisp
(setq eglot-events-buffer-config '(:size 2000000 :format full))
```

Or set it interactively in a running session:

    M-x set-variable RET eglot-events-buffer-config RET (:size 2000000 :format full)

### Viewing diagnostics

- `C-h .` — show the diagnostic at point (hover)
- `M-x flymake-show-buffer-diagnostics` — list all diagnostics in the
  current buffer
- `M-x flymake-show-project-diagnostics` — list diagnostics across the
  project

### Restarting the server

If the server gets into a bad state:

- `M-x eglot-shutdown` — stop the server for the current project
- `M-x eglot-reconnect` — restart without closing buffers


## Configuration File

tcl-lsp reads a platform-native configuration file for editor-agnostic
defaults (diagnostics, optimiser, shimmer, features, formatting):

| Platform | Default path |
|----------|-------------|
| Linux / BSD / WSL2 | `~/.config/tcl-lsp/config.ini` |
| macOS | `~/Library/Application Support/tcl-lsp/config.ini` |
| Windows | `%APPDATA%\tcl-lsp\config.ini` |
| MSYS2 / Cygwin | `~/.config/tcl-lsp/config.ini` |

`$XDG_CONFIG_HOME` overrides the default on every platform.

Settings from the config file are applied as baseline defaults.  Emacs
workspace configuration via `eglot-workspace-configuration` or
`lsp-mode` settings override the config file — so you can set shared
defaults in the config file and per-project overrides in Emacs.

Use the `tcl-lsp.exportConfig` command via `workspace/executeCommand` to
write current settings to the config file.

See [docs/design/contracts/xdg-config.md](../../docs/design/contracts/xdg-config.md) for
the full reference.
