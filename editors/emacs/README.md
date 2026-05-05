# Emacs

tcl-lsp works with Emacs via **eglot** (built-in since Emacs 29) or **lsp-mode**.

## Prerequisites

**Python 3.10+** is required. We recommend the latest stable Python
(currently 3.14). Install via [Homebrew](https://docs.brew.sh/Homebrew-and-Python)
(`brew install python@3.14`) or [python.org](https://www.python.org/downloads/).

The `.pyz` zipapp bundles all Python dependencies internally — no
`pip install` is needed. You only need a Python interpreter on your system.

See the [Installation Guide](../../INSTALL.md#python-prerequisite) for
full details on Python setup across platforms.

The server needs to be accessible via one of:

```sh
# Option A — run from source (requires uv)
uv run --directory /path/to/tcl-lsp --no-dev python -m lsp

# Option B — standalone zipapp (just needs Python 3.10+)
python3 /path/to/tcl-lsp-server.pyz
```

To point to a specific Python interpreter, use its full path as the first
element of the command list (e.g. `"/opt/homebrew/bin/python3.14"`).

## eglot (Emacs 29+)

Add to your `init.el`:

```elisp
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '(tcl-mode . ("uv" "run" "--directory" "/path/to/tcl-lsp"
                             "--no-dev" "python" "-m" "lsp"))))

;; Auto-start on Tcl files
(add-hook 'tcl-mode-hook #'eglot-ensure)
```

Or with the standalone zipapp:

```elisp
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '(tcl-mode . ("python3" "/path/to/tcl-lsp-server.pyz"))))
```

## lsp-mode

```elisp
(with-eval-after-load 'lsp-mode
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection
                     '("uv" "run" "--directory" "/path/to/tcl-lsp"
                       "--no-dev" "python" "-m" "lsp"))
    :activation-fn (lsp-activate-on "tcl")
    :server-id 'tcl-lsp)))

(add-hook 'tcl-mode-hook #'lsp)
```

## Settings

Pass settings via eglot workspace configuration:

```elisp
(setq-default eglot-workspace-configuration
              '(:tclLsp (:dialect "tcl8.6"   ;; tcl8.4 | tcl8.5 | tcl8.6 | tcl9.0 | f5-irules | f5-iapps | f5-tmsh | f5-bigip | synopsys-eda-tcl | cadence-eda-tcl | xilinx-eda-tcl | intel-quartus-eda-tcl | mentor-eda-tcl | expect
                         :formatting (:indentSize 4 :maxLineLength 120))))

;; Register .apl files for tcl-mode so eglot activates
(add-to-list 'auto-mode-alist '("\\.apl\\'" . tcl-mode))
```

## Known issues

### Eglot semantic-tokens highlighting goes stale until file reload

**Symptoms:** after making edits, syntax highlighting becomes wrong —
identifiers show colors that don't match their actual token type, or
highlighting visibly degrades the more you edit. Saving (`C-x C-s`)
does not fix it. Reverting the buffer (`M-x revert-buffer`) or closing
and reopening the file does fix it.

**Cause:** an upstream bug in `eglot-semantic-tokens-mode`'s painter
(`eglot--semtok-font-lock-1`). The painter applies new token faces
with `add-face-text-property` but never strips its prior
`eglot-semantic-*` faces from the buffer's `face' text-property
before re-applying. Each repaint accumulates a copy, and Emacs renders
based on the *first* face in the list — which is the oldest one, not
the freshest. Tracked at
[bitwisecook/tcl-lsp#333](https://github.com/bitwisecook/tcl-lsp/issues/333);
present in eglot 1.21–1.23 inclusive.

**Workarounds (pick one):**

1. **Apply this advice in your `init.el`** (strips stale
   `eglot-semantic-*` faces before each repaint, preserving everything
   else):

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

The proper fix is in eglot upstream. If you're affected, please
upvote / report at the appropriate emacs-devel channel.

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

See [docs/kcs/kcs-xdg-config.md](../../docs/kcs/kcs-xdg-config.md) for
the full reference.
