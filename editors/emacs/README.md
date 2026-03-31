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
uv run --directory /path/to/tcl-lsp --no-dev python -m server

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
                             "--no-dev" "python" "-m" "server"))))

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
                       "--no-dev" "python" "-m" "server"))
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

## Bracket matching and auto-pairs

Emacs's `tcl-mode` provides bracket matching out of the box via
`show-paren-mode` (enabled by default in Emacs 29+).  For automatic
insertion of closing brackets and quotes, enable `electric-pair-mode`:

```elisp
(add-hook 'tcl-mode-hook #'electric-pair-mode)
```

This auto-closes `{}`, `[]`, `()`, and `""` as you type.

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
