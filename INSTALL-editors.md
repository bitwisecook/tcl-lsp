# Editor installation

Every editor needs Python 3.10+ on the host plus the editor-specific
artefact from [GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest).
The `.vsix`, `.sublime-package`, and `.zip` plugins bundle the LSP
server; the standalone editors (Neovim/Emacs/Helix) point at
`tcl-lsp-server-<version>.pyz` instead.

| Editor | Artefact | Install |
|--------|----------|---------|
| [VS Code](#vs-code) | `tcl-lsp-vscode-<v>.vsix` | `code --install-extension` |
| [Sublime Text](#sublime-text) | `Tcl.sublime-package` | Copy into `Installed Packages/` |
| [JetBrains](#jetbrains) | `tcl-lsp-jetbrains-<v>.zip` | Settings > Plugins > Install from Disk |
| [Neovim](#neovim) | `tcl-lsp-server-<v>.pyz` | Lua config |
| [Emacs](#emacs) | `tcl-lsp-server-<v>.pyz` | eglot / lsp-mode |
| [Helix](#helix) | `tcl-lsp-server-<v>.pyz` | `languages.toml` |
| [Zed](#zed) | extension registry | `zed: extensions` |

## VS Code

```sh
code --install-extension ~/Downloads/tcl-lsp-vscode-<v>.vsix
```

Configure under **Settings > Extensions > Tcl**. Pin an interpreter
with `tclLsp.pythonPath` (default `"auto"`).

## Sublime Text

Drop the package — **filename must be `Tcl.sublime-package`** — into
your Installed Packages directory:

```sh
# macOS:   ~/Library/Application Support/Sublime Text/Installed Packages/
# Linux:   ~/.config/sublime-text/Installed Packages/
# Windows: %APPDATA%\Sublime Text\Installed Packages\
```

Install the **LSP** package from Package Control. Pin an interpreter
via **Preferences > Package Settings > LSP-Tcl > Settings**.

## JetBrains

Requires IDEA Ultimate 2024.1+ (free editions from 2025.3).
**Settings > Plugins > gear icon > Install Plugin from Disk…**,
select the zip, restart. Configure under **Settings > Tools > Tcl
Language Server**.

## Neovim

Drop `tcl-lsp-server-<v>.pyz` at `~/bin/tcl-lsp-server.pyz`.

`~/.config/nvim/lsp/tcl_lsp.lua`:

```lua
return {
  cmd = { 'python3', vim.fn.expand('~/bin/tcl-lsp-server.pyz') },
  filetypes = { 'tcl' },
  settings = { tclLsp = { dialect = 'tcl8.6' } },
}
```

`init.lua`:

```lua
vim.filetype.add({ extension = {
  tcl = 'tcl', tk = 'tcl', itcl = 'tcl', tm = 'tcl',
  irul = 'tcl', irule = 'tcl', iapp = 'tcl', iappimpl = 'tcl', impl = 'tcl',
}})
vim.lsp.enable('tcl_lsp')
```

See [editors/neovim/README.md](editors/neovim/README.md) for
nvim-lspconfig and autocommand variants.

## Emacs

**eglot** (Emacs 29+):

```elisp
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '(tcl-mode . ("python3" "/path/to/tcl-lsp-server.pyz"))))
(add-hook 'tcl-mode-hook #'eglot-ensure)
```

**lsp-mode**:

```elisp
(with-eval-after-load 'lsp-mode
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection
                     '("python3" "/path/to/tcl-lsp-server.pyz"))
    :activation-fn (lsp-activate-on "tcl")
    :server-id 'tcl-lsp)))
(add-hook 'tcl-mode-hook #'lsp)
```

## Helix

`~/.config/helix/languages.toml`:

```toml
[language-server.tcl-lsp]
command = "python3"
args = ["/path/to/tcl-lsp-server.pyz"]

[[language]]
name = "tcl"
scope = "source.tcl"
file-types = ["tcl", "tk", "itcl", "tm", "irul", "irule", "iapp", "iappimpl", "impl"]
language-servers = ["tcl-lsp"]
```

## Zed

Command Palette > **`zed: extensions`** > search **Tcl**. The
extension auto-downloads the LSP server on first use.
