# Installation Guide

Step-by-step instructions for installing tcl-lsp from
[GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases) on
macOS and Windows.

Each release publishes these artefacts:

| File | Editor |
|------|--------|
| `tcl-lsp-vscode-VERSION.vsix` | VS Code |
| `Tcl.sublime-package` | Sublime Text (ready to install) |
| `tcl-lsp-sublime-VERSION.sublime-package` | Sublime Text (versioned; rename to `Tcl.sublime-package`) |
| `tcl-lsp-jetbrains-VERSION.zip` | JetBrains IDEs |
| `tcl-lsp-server-VERSION.pyz` | Standalone LSP server (Neovim, Emacs, Helix, Zed) |

---

## VS Code

The `.vsix` bundles the server — no extra dependencies required.

### macOS

```bash
# Download the .vsix from GitHub Releases, then:
code --install-extension ~/Downloads/tcl-lsp-vscode-VERSION.vsix
```

### Windows

```powershell
# Download the .vsix from GitHub Releases, then:
code --install-extension "$env:USERPROFILE\Downloads\tcl-lsp-vscode-VERSION.vsix"
```

Restart VS Code after installation. Settings are available under
**Settings > Extensions > Tcl**.

---

## Sublime Text

**Prerequisites**: Sublime Text 4 (build 4107+), Python 3.10+ on PATH.

The `.sublime-package` bundles the server. For full LSP features, also
install the **LSP** package from Package Control.

### macOS

```bash
# Copy the ready-to-install package (no rename needed):
cp ~/Downloads/Tcl.sublime-package \
   ~/Library/Application\ Support/Sublime\ Text/Installed\ Packages/

# Or, if you downloaded the versioned filename, rename it:
cp ~/Downloads/tcl-lsp-sublime-VERSION.sublime-package \
   ~/Library/Application\ Support/Sublime\ Text/Installed\ Packages/Tcl.sublime-package
```

### Windows

```powershell
# Copy the ready-to-install package (no rename needed):
Copy-Item "$env:USERPROFILE\Downloads\Tcl.sublime-package" `
    "$env:APPDATA\Sublime Text\Installed Packages\"

# Or, if you downloaded the versioned filename, rename it:
Copy-Item "$env:USERPROFILE\Downloads\tcl-lsp-sublime-VERSION.sublime-package" `
    "$env:APPDATA\Sublime Text\Installed Packages\Tcl.sublime-package"
```

> **Important**: The file **must** be named `Tcl.sublime-package` in the
> `Installed Packages` directory. Sublime Text derives the package name from
> the filename, and the plugin expects to be loaded as `Tcl`.

Restart Sublime Text after installation.

---

## JetBrains (IntelliJ IDEA, PyCharm, WebStorm, etc.)

**Prerequisites**: IntelliJ IDEA Ultimate 2024.1+ (or other paid JetBrains
IDE), Python 3.10+ on PATH.

> Starting with IntelliJ IDEA 2025.3, the LSP API is available to all users
> including free editions.

The `.zip` bundles the server — no extra configuration needed.

### macOS and Windows

1. Download `tcl-lsp-jetbrains-VERSION.zip` from GitHub Releases
2. Open your JetBrains IDE
3. **Settings > Plugins > gear icon > Install Plugin from Disk...**
4. Select the downloaded `.zip` file
5. Restart the IDE

Configure via **Settings > Tools > Tcl Language Server**.

---

## Neovim

**Prerequisites**: Neovim 0.11+ (or 0.8+ with nvim-lspconfig), Python 3.10+.

Neovim does not use a packaged extension — instead, download the standalone
server and point your LSP config at it.

### macOS

```bash
# 1. Download the server zipapp
cp ~/Downloads/tcl-lsp-server-VERSION.pyz ~/bin/tcl-lsp-server.pyz
chmod +x ~/bin/tcl-lsp-server.pyz

# 2. Copy the LSP config (Neovim 0.11+)
mkdir -p ~/.config/nvim/lsp
cp editors/neovim/tcl_lsp.lua ~/.config/nvim/lsp/tcl_lsp.lua
```

Edit `~/.config/nvim/lsp/tcl_lsp.lua` and set the `cmd` line:

```lua
cmd = { 'python3', os.getenv('HOME') .. '/bin/tcl-lsp-server.pyz' },
```

Then add to your `init.lua`:

```lua
vim.filetype.add({
  extension = {
    tcl = 'tcl', tk = 'tcl', itcl = 'tcl', tm = 'tcl',
    irul = 'tcl', irule = 'tcl', iapp = 'tcl', iappimpl = 'tcl', impl = 'tcl',
  },
})

vim.lsp.enable('tcl_lsp')
```

### Windows

```powershell
# 1. Download the server zipapp to a known location
Copy-Item "$env:USERPROFILE\Downloads\tcl-lsp-server-VERSION.pyz" `
    "$env:LOCALAPPDATA\tcl-lsp\tcl-lsp-server.pyz"

# 2. Copy the LSP config (Neovim 0.11+)
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\nvim\lsp"
Copy-Item editors\neovim\tcl_lsp.lua "$env:LOCALAPPDATA\nvim\lsp\tcl_lsp.lua"
```

Edit `%LOCALAPPDATA%\nvim\lsp\tcl_lsp.lua` and set the `cmd` line:

```lua
cmd = { 'python3', vim.fn.expand('$LOCALAPPDATA') .. '/tcl-lsp/tcl-lsp-server.pyz' },
```

Then add the same `vim.filetype.add` and `vim.lsp.enable` blocks to your
`init.lua` (see macOS above).

See [editors/neovim/README.md](editors/neovim/README.md) for nvim-lspconfig
and manual autocommand alternatives.

---

## Emacs

**Prerequisites**: Emacs 29+ (for eglot) or lsp-mode, Python 3.10+.

Download `tcl-lsp-server-VERSION.pyz` from GitHub Releases and place it
somewhere on your system.

### macOS and Windows

Add to your `init.el` (eglot):

```elisp
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '(tcl-mode . ("python3" "/path/to/tcl-lsp-server.pyz"))))

(add-hook 'tcl-mode-hook #'eglot-ensure)
```

Or with lsp-mode:

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

Replace `/path/to/tcl-lsp-server.pyz` with the actual path where you saved
the file.

See [editors/emacs/README.md](editors/emacs/README.md) for settings and
running from source.

---

## Helix

**Prerequisites**: Helix editor, Python 3.10+.

Download `tcl-lsp-server-VERSION.pyz` from GitHub Releases.

### macOS

Add to `~/.config/helix/languages.toml`:

```toml
[language-server.tcl-lsp]
command = "python3"
args = ["/path/to/tcl-lsp-server.pyz"]

[[language]]
name = "tcl"
scope = "source.tcl"
file-types = ["tcl", "tk", "itcl", "tm", "irul", "irule", "iapp", "iappimpl", "impl"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
```

### Windows

Add to `%APPDATA%\helix\languages.toml`:

```toml
[language-server.tcl-lsp]
command = "python3"
args = ["C:/path/to/tcl-lsp-server.pyz"]

[[language]]
name = "tcl"
scope = "source.tcl"
file-types = ["tcl", "tk", "itcl", "tm", "irul", "irule", "iapp", "iappimpl", "impl"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
```

See [editors/helix/README.md](editors/helix/README.md) for workspace settings.

---

## Zed

**Prerequisites**: Zed editor, Python 3.10+.

Zed currently requires installing the extension from source (a dev extension).
There is no standalone release artefact.

### macOS and Windows

1. Clone the tcl-lsp repository
2. Open Zed, then **Command Palette > Extensions: Install Dev Extension**
3. Point it at the `editors/zed/` directory

The extension compiles to WebAssembly automatically.

Ensure `tcl-lsp-server.pyz` is on your `$PATH`, or configure the server path
in your Zed `settings.json`:

```json
{
  "lsp": {
    "tcl-lsp": {
      "settings": {
        "tclLsp": {
          "dialect": "tcl8.6"
        }
      }
    }
  }
}
```

See [editors/zed/README.md](editors/zed/README.md) for details.
