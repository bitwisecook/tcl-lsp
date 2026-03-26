# Installation Guide

Step-by-step instructions for installing tcl-lsp from
[GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases) on
macOS, Linux, and Windows.

Each release publishes these artefacts:

| File | Editor |
|------|--------|
| `tcl-lsp-vscode-VERSION.vsix` | VS Code |
| `Tcl.sublime-package` | Sublime Text (ready to install) |
| `tcl-lsp-sublime-VERSION.sublime-package` | Sublime Text (versioned; rename to `Tcl.sublime-package`) |
| `tcl-lsp-jetbrains-VERSION.zip` | JetBrains IDEs |
| `tcl-lsp-server-VERSION.pyz` | Standalone LSP server (Neovim, Emacs, Helix, Zed) |

---

## Python prerequisite

tcl-lsp requires **Python 3.10 or newer**. We recommend installing the
**latest stable Python** (currently 3.14) for the best performance and
security updates.

> **Bundled extensions (VS Code, Sublime Text, JetBrains):** The `.vsix`,
> `.sublime-package`, and `.zip` plugin archives bundle all Python
> *dependencies* (pygls, lsprotocol, etc.) inside the package — you do **not**
> need to `pip install` anything. However, a Python 3.10+ **interpreter**
> must still be installed on your system, because the bundled server runs as a
> Python zipapp (`.pyz`) that is executed by your local interpreter.

### Installing Python

#### macOS (Homebrew — recommended)

```bash
brew install python@3.14
```

Homebrew installs to `/opt/homebrew/bin/python3` (Apple Silicon) or
`/usr/local/bin/python3` (Intel). The extension auto-discovers both
locations.

See the [Homebrew Python documentation](https://docs.brew.sh/Homebrew-and-Python)
for more details.

#### macOS / Windows (python.org)

Download the latest installer from
[python.org/downloads](https://www.python.org/downloads/) and run it.
On Windows, check **"Add python.exe to PATH"** during installation.

#### Linux

Use your distribution's package manager:

```bash
# Debian / Ubuntu
sudo apt install python3

# Fedora
sudo dnf install python3

# Arch
sudo pacman -S python
```

### Verifying your installation

```bash
python3 --version   # Should print Python 3.10 or newer
```

### Pointing the extension to a specific interpreter

If you have multiple Python versions or a non-standard install location,
you can tell the extension exactly which interpreter to use:

| Editor | Setting |
|--------|---------|
| VS Code | `tclLsp.pythonPath` in Settings (default: `"auto"`) |
| Sublime Text | `python_path` in `LSP-Tcl.sublime-settings` |
| JetBrains | **Settings > Tools > Tcl Language Server > Python path** |
| Neovim / Emacs / Helix | First element of the `cmd` array in your LSP config |
| Zed | Discovered automatically from PATH |

Set the value to the full path of your Python interpreter, e.g.
`/opt/homebrew/bin/python3.14` or `C:\Python313\python.exe`.
When set to `"auto"` (the default for VS Code, Sublime Text, and
JetBrains), the extension scans PATH and well-known locations for the
highest available Python 3.10+ version.

---

## VS Code

The `.vsix` bundles the server and all Python dependencies — only a
Python 3.10+ interpreter is required (see [above](#python-prerequisite)).

If no suitable interpreter is found, the extension shows an error
notification with a link to this guide.

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

To use a specific Python interpreter, set **`tclLsp.pythonPath`** in
VS Code settings to the full path (e.g. `/opt/homebrew/bin/python3.14`).
The default `"auto"` scans PATH and well-known locations automatically.

---

## Sublime Text

**Prerequisites**: Sublime Text 4 (build 4107+), Python 3.10+ on PATH
(see [Python prerequisite](#python-prerequisite)).

The `.sublime-package` bundles the server and all Python dependencies.
For full LSP features, also install the **LSP** package from Package
Control.

If no suitable interpreter is found, Sublime Text shows an error in the
status bar with guidance to install Python.

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

To use a specific Python interpreter, set `python_path` in
**Preferences > Package Settings > LSP-Tcl > Settings**:

```json
{
    "python_path": "/opt/homebrew/bin/python3.14"
}
```

---

## JetBrains (IntelliJ IDEA, PyCharm, WebStorm, etc.)

**Prerequisites**: IntelliJ IDEA Ultimate 2024.1+ (or other paid JetBrains
IDE), Python 3.10+ on PATH
(see [Python prerequisite](#python-prerequisite)).

> Starting with IntelliJ IDEA 2025.3, the LSP API is available to all users
> including free editions.

The `.zip` bundles the server and all Python dependencies — only a Python
interpreter is needed.

If no suitable interpreter is found, the IDE shows a notification balloon
with guidance to install Python.

### macOS and Windows

1. Download `tcl-lsp-jetbrains-VERSION.zip` from GitHub Releases
2. Open your JetBrains IDE
3. **Settings > Plugins > gear icon > Install Plugin from Disk...**
4. Select the downloaded `.zip` file
5. Restart the IDE

Configure via **Settings > Tools > Tcl Language Server**. To use a
specific Python interpreter, set the **Python path** field to the full
path (e.g. `/opt/homebrew/bin/python3.14`).

---

## Neovim

**Prerequisites**: Neovim 0.11+ (or 0.8+ with nvim-lspconfig), Python 3.10+
(see [Python prerequisite](#python-prerequisite)).

Neovim does not use a packaged extension — instead, download the standalone
server and point your LSP config at it. The `.pyz` zipapp bundles all
Python dependencies; only a Python 3.10+ interpreter is needed.

### macOS

```bash
# 1. Download the server zipapp
cp ~/Downloads/tcl-lsp-server-VERSION.pyz ~/bin/tcl-lsp-server.pyz
chmod +x ~/bin/tcl-lsp-server.pyz

# 2. Copy the LSP config (Neovim 0.11+)
mkdir -p ~/.config/nvim/lsp
cp editors/neovim/tcl_lsp.lua ~/.config/nvim/lsp/tcl_lsp.lua
```

Edit `~/.config/nvim/lsp/tcl_lsp.lua` and set the `cmd` line to your
Python interpreter and server path:

```lua
cmd = { '/opt/homebrew/bin/python3', os.getenv('HOME') .. '/bin/tcl-lsp-server.pyz' },
```

If `python3` on your PATH is 3.10+, you can simply use `'python3'` as the
first element.

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

**Prerequisites**: Emacs 29+ (for eglot) or lsp-mode, Python 3.10+
(see [Python prerequisite](#python-prerequisite)).

Download `tcl-lsp-server-VERSION.pyz` from GitHub Releases and place it
somewhere on your system. The `.pyz` zipapp bundles all Python
dependencies; only a Python 3.10+ interpreter is needed.

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

Replace `"python3"` with the full path to your Python 3.10+ interpreter
if `python3` on your PATH is too old or absent (e.g.
`"/opt/homebrew/bin/python3.14"`). Replace `/path/to/tcl-lsp-server.pyz`
with the actual path where you saved the file.

See [editors/emacs/README.md](editors/emacs/README.md) for settings and
running from source.

---

## Helix

**Prerequisites**: Helix editor, Python 3.10+
(see [Python prerequisite](#python-prerequisite)).

Download `tcl-lsp-server-VERSION.pyz` from GitHub Releases. The `.pyz`
zipapp bundles all Python dependencies; only a Python 3.10+ interpreter
is needed.

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

Replace `"python3"` with the full path to your interpreter if needed
(e.g. `"/opt/homebrew/bin/python3.14"`).

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

**Prerequisites**: Zed editor, Python 3.10+
(see [Python prerequisite](#python-prerequisite)).

Zed currently requires installing the extension from source (a dev extension).
There is no standalone release artefact.

If no suitable Python interpreter is found, the extension shows an error
notification with guidance to install Python.

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
