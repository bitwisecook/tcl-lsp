# Editor installation

Every editor needs Python 3.10+ on the host plus the editor-specific
artefact from [GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest).
The `.vsix`, `.sublime-package`, and `.zip` plugins bundle the LSP
server; the standalone editors (Neovim/Emacs/Helix) point at
`tcl-lsp-server-<version>.pyz` instead.

| Editor | Artefact | Install |
|--------|----------|---------|
| [VS Code](#vs-code) | `tcl-lsp-vscode-<v>.vsix` | `code --install-extension`, or VS Code Marketplace |
| [Cursor / Windsurf / VSCodium / Theia / code-server / Gitpod / Codespaces](#vs-code) | same `.vsix` | Open VSX Registry, or sideload the `.vsix` |
| [Sublime Text](#sublime-text) | `Tcl.sublime-package` | Package Control: install **Tcl-LSP**, or copy into `Installed Packages/` |
| [JetBrains](#jetbrains) | `tcl-lsp-jetbrains-<v>.zip` | Settings > Plugins > Install from Disk |
| [Neovim](#neovim) | `tcl-lsp-server-<v>.pyz` | Lua config |
| [Emacs](#emacs) | `tcl-lsp-server-<v>.pyz` | eglot / lsp-mode |
| [Helix](#helix) | `tcl-lsp-server-<v>.pyz` | `languages.toml` |
| [Zed](#zed) | extension registry | `zed: extensions` |

## Python

`python3 --version` must report 3.10 or newer:

```sh
brew install python@3.14            # macOS
sudo apt install python3            # Debian/Ubuntu 22.04+
sudo dnf install python3.11         # RHEL/Rocky/Alma 9 (system python3 is 3.9)
sudo dnf install python3            # Fedora
sudo pacman -S python               # Arch
sudo apk add python3                # Alpine
```

Each editor has a setting for pinning the interpreter when multiple
are installed — see the per-editor sections below.

## VS Code

Install from the VS Code Marketplace
(<https://marketplace.visualstudio.com/items?itemName=bitwisecook.tcl-lsp>),
or sideload the bundled `.vsix`:

```sh
code --install-extension ~/Downloads/tcl-lsp-vscode-<v>.vsix
```

Configure under **Settings > Extensions > Tcl**. Pin an interpreter
with `tclLsp.pythonPath` (default `"auto"`).

### VS Code-compatible editors (Open VSX)

The same extension is published to the [Open VSX Registry](https://open-vsx.org/extension/bitwisecook/tcl-lsp),
which is the default extension source for editors that cannot use the
Microsoft Marketplace:

- **Cursor**, **Windsurf** — Extensions panel; search "Tcl/Tk".
- **VSCodium** — Extensions panel; search "Tcl/Tk".
- **Eclipse Theia**, **code-server** / **Coder**, **Gitpod**,
  **GitHub Codespaces (Theia builds)** — same Extensions UI; search
  "Tcl/Tk".

Sideloading the `.vsix` works in all of them as a fallback.

## Sublime Text

Install via **Package Control** (Command Palette → **Package Control:
Install Package** → search **Tcl-LSP**), or sideload manually. The
Package Control entry pulls from a dedicated mirror repo, so tagged
releases of `bitwisecook/tcl-lsp` appear within ~1 hour of the
maintainer running `make publish-sublime`.

For the manual sideload path, drop the package —
**filename must be `Tcl.sublime-package`** — into your Installed
Packages directory:

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
