# Editor installation guide

Each editor needs the same thing: a Python 3.10+ interpreter on the
host, plus the editor-specific artefact from
[GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest).

The `.vsix`, `.sublime-package`, and `.zip` plugin archives bundle the
LSP server and every Python dependency — Python is the only thing you
need to install separately.

| Editor | Artefact | Install method |
|--------|----------|----------------|
| [VS Code](#vs-code) | `tcl-lsp-vscode-<version>.vsix` | `code --install-extension` |
| [Sublime Text](#sublime-text) | `Tcl.sublime-package` | Copy into `Installed Packages/` |
| [JetBrains](#jetbrains) | `tcl-lsp-jetbrains-<version>.zip` | Settings > Plugins > Install from Disk |
| [Neovim](#neovim) | `tcl-lsp-server-<version>.pyz` + Lua snippet | Drop on disk, point LSP config at it |
| [Emacs](#emacs) | `tcl-lsp-server-<version>.pyz` + elisp snippet | eglot or lsp-mode |
| [Helix](#helix) | `tcl-lsp-server-<version>.pyz` + TOML snippet | `languages.toml` |
| [Zed](#zed) | extension registry or `tcl-lsp-zed-<version>.zip` | `zed: install dev extension` |

## Python prerequisite

Install Python 3.10+ on the host (3.14 recommended):

```sh
# macOS
brew install python@3.14

# Debian/Ubuntu
sudo apt install python3            # 22.04+ ships 3.10+; older: use deadsnakes PPA

# RHEL/Rocky/Alma 9 (system python3 is 3.9 — install alongside)
sudo dnf install python3.11

# Fedora / Arch / Alpine
sudo dnf install python3            # / pacman -S python / apk add python3
```

`python3 --version` must report 3.10 or newer.  If you have multiple
interpreters, each editor has a setting for picking the one to use —
see the editor's section below.

## VS Code

```sh
code --install-extension ~/Downloads/tcl-lsp-vscode-<version>.vsix
```

Restart VS Code.  Configure under **Settings > Extensions > Tcl**.  To
pin a specific Python interpreter, set `tclLsp.pythonPath` to its full
path (default `"auto"` auto-discovers Python 3.10+ on `PATH`).

## Sublime Text

Drop the package into your Sublime Text Installed Packages directory.
**It must be named `Tcl.sublime-package`** — Sublime derives the
package name from the filename.

```sh
# macOS
cp ~/Downloads/Tcl.sublime-package \
   ~/Library/Application\ Support/Sublime\ Text/Installed\ Packages/

# Linux
cp ~/Downloads/Tcl.sublime-package ~/.config/sublime-text/Installed\ Packages/

# Windows (PowerShell)
Copy-Item "$env:USERPROFILE\Downloads\Tcl.sublime-package" `
    "$env:APPDATA\Sublime Text\Installed Packages\"
```

Restart Sublime Text.  Install the **LSP** package from Package
Control for full features.  To pin a Python interpreter, set
`python_path` in **Preferences > Package Settings > LSP-Tcl >
Settings**.

## JetBrains

Requires IntelliJ IDEA Ultimate 2024.1+ (or any paid JetBrains IDE).
From 2025.3 onward the LSP API is available to free editions too.

1. Download `tcl-lsp-jetbrains-<version>.zip`.
2. **Settings > Plugins > gear icon > Install Plugin from Disk…** and
   select the zip.
3. Restart the IDE.

Configure under **Settings > Tools > Tcl Language Server**.

## Neovim

Download `tcl-lsp-server-<version>.pyz` from
[Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest) and
put it somewhere readable (e.g. `~/bin/tcl-lsp-server.pyz`).

Neovim 0.11+ native LSP config (drop into `~/.config/nvim/lsp/tcl_lsp.lua`):

```lua
return {
  cmd = { 'python3', vim.fn.expand('~/bin/tcl-lsp-server.pyz') },
  filetypes = { 'tcl' },
  settings = { tclLsp = { dialect = 'tcl8.6' } },
}
```

In your `init.lua`:

```lua
vim.filetype.add({
  extension = {
    tcl = 'tcl', tk = 'tcl', itcl = 'tcl', tm = 'tcl',
    irul = 'tcl', irule = 'tcl', iapp = 'tcl', iappimpl = 'tcl', impl = 'tcl',
  },
})
vim.lsp.enable('tcl_lsp')
```

See [editors/neovim/README.md](editors/neovim/README.md) for the
nvim-lspconfig and manual autocommand variants.

## Emacs

Download `tcl-lsp-server-<version>.pyz`.

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

Replace `"python3"` with the full path to a 3.10+ interpreter if your
default `python3` is older.

## Helix

Download `tcl-lsp-server-<version>.pyz`, then add to
`~/.config/helix/languages.toml` (or `%APPDATA%\helix\languages.toml`
on Windows):

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

Easiest: install from the Zed extension registry — open Zed, run
**`zed: extensions`** from the Command Palette, search for **Tcl**.

Or install the release artefact as a dev extension:

```sh
unzip ~/Downloads/tcl-lsp-zed-<version>.zip -d /tmp/tcl-lsp-zed
# In Zed: Command Palette > "zed: install dev extension" > /tmp/tcl-lsp-zed
```

The extension auto-downloads the LSP server zipapp on first use and
auto-discovers Python 3.10+ on `$PATH`.

Optional configuration (`settings.json`):

```json
{
  "lsp": {
    "tcl-lsp": {
      "settings": { "tclLsp": { "dialect": "tcl8.6" } }
    }
  }
}
```
