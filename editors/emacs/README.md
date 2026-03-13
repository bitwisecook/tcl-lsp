# Emacs

tcl-lsp works with Emacs via **eglot** (built-in since Emacs 29) or **lsp-mode**.

## Prerequisites

The server needs to be accessible via one of:

```sh
# Option A — run from source (requires uv)
uv run --directory /path/to/tcl-lsp --no-dev python -m server

# Option B — standalone zipapp
python3 /path/to/tcl-lsp-server.pyz
```

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
              '(:tclLsp (:dialect "tcl8.6"
                         :formatting (:indentSize 4 :maxLineLength 120))))
```
