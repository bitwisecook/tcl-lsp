# Editor installation

Install the editor-specific artefact from
[GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest).

**No editor needs Python.** The server is a self-contained native
`tcl-lsp-server` binary. The VS Code `.vsix` and the JetBrains `.zip`
each bundle one binary per platform (macOS/Linux/Windows on x64 and
arm64, plus Linux riscv64) and run the one matching your machine. The
Sublime Text package and the Zed extension download the matching binary
on first use.

The standalone editors (Neovim, Emacs, Helix, and any other LSP-capable
editor) need the `tcl-lsp-server-<target-triple>` binary from
[Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest) — see
[The server binary](#the-server-binary) below.

| Editor | Artefact | Install |
|--------|----------|---------|
| [VS Code](#vs-code) | `tcl-lsp-vscode-<v>-universal.vsix` (manual) or an auto-selected platform package | `code --install-extension`, or VS Code Marketplace |
| [Cursor / Windsurf / VSCodium / Theia / code-server / Gitpod / Codespaces](#vs-code) | `tcl-lsp-vscode-<v>-universal.vsix` | Sideload the `.vsix` (`code --install-extension` style) |
| [Sublime Text](#sublime-text) | `TclLsp.sublime-package` | Package Control: install **TclLsp**, or copy into `Installed Packages/` |
| [JetBrains](#jetbrains) | `tcl-lsp-jetbrains-<v>.zip` | Settings > Plugins > Install from Disk |
| [Neovim](#neovim) | `tcl-lsp-server-<triple>` | Lua config |
| [Emacs](#emacs) | `tcl-lsp-server-<triple>` | eglot / lsp-mode |
| [Helix](#helix) | `tcl-lsp-server-<triple>` | `languages.toml` |
| [Zed](#zed) | extension registry | `zed: extensions` |

**[VS Code-compatible editors](#vs-code-compatible-editors)** (Cursor,
Windsurf, VSCodium, code-server, Gitpod, Codespaces, Eclipse Theia)
install the same `.vsix` unchanged.

**[Other LSP-capable editors](#other-lsp-capable-editors)** (Vim,
Kate, Kakoune, Notepad++, Geany, Lite XL, micro, CudaText,
JupyterLab) point a generic LSP client at the `tcl-lsp-server` binary.

## The server binary

Skip this section for VS Code, VS Code-compatible editors, JetBrains,
Sublime Text, and Zed — those all obtain the server for you.

Every other editor needs the `tcl-lsp-server` binary on the host. Download
the asset for your platform from
[Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest); assets
are named `tcl-lsp-server-<target-triple>` (`.exe` on Windows), with no
version in the filename.

| Platform | Asset |
|---|---|
| macOS arm64 | `tcl-lsp-server-aarch64-apple-darwin` |
| macOS x86_64 | `tcl-lsp-server-x86_64-apple-darwin` |
| Linux x86_64 | `tcl-lsp-server-x86_64-unknown-linux-gnu` |
| Linux arm64 | `tcl-lsp-server-aarch64-unknown-linux-gnu` |
| Linux riscv64 | `tcl-lsp-server-riscv64gc-unknown-linux-gnu` |
| Windows x86_64 | `tcl-lsp-server-x86_64-pc-windows-msvc.exe` |
| Windows arm64 | `tcl-lsp-server-aarch64-pc-windows-msvc.exe` |

For macOS on Apple silicon:

```sh
base=https://github.com/bitwisecook/tcl-lsp/releases/latest/download
curl -fLO "$base/tcl-lsp-server-aarch64-apple-darwin"
install -m 0755 tcl-lsp-server-aarch64-apple-darwin ~/bin/tcl-lsp-server
```

Verify it against the release `SHA256SUMS` (see
[INSTALL-cli.md](INSTALL-cli.md#verify-downloads)), then use
`~/bin/tcl-lsp-server` as the command in the snippets below. The examples
use that path throughout; substitute your own.

## VS Code

Install from the VS Code Marketplace
(<https://marketplace.visualstudio.com/items?itemName=bitwisecook.tcl-lsp>),
which serves a small package containing only your platform's binary
automatically, or download and sideload the `-universal` package (bundles
every platform in one file, so it works regardless of your OS/architecture):

```sh
code --install-extension ~/Downloads/tcl-lsp-vscode-<v>-universal.vsix
```

Configure under **Settings > Extensions > Tcl**. No Python interpreter
is needed — the extension ships a native `tcl-lsp-server` binary for
your platform and launches it automatically. There is no Python backend:
to run against a local build, point `tclLsp.rustServerPath` at a
`tcl-lsp-server` binary or `tclLsp.serverPath` at a checkout.

### VS Code-compatible editors

The `-universal` package works in editors that cannot use the Microsoft
Marketplace (Cursor, Windsurf, VSCodium, Eclipse Theia, code-server /
Coder, Gitpod, GitHub Codespaces Theia builds) — it bundles every
platform's binary in one file, so there's no need to pick the right one
by hand. Download it from the GitHub release and sideload through the
editor's Extensions UI, or via the CLI:

```sh
cursor   --install-extension ~/Downloads/tcl-lsp-vscode-<v>-universal.vsix
codium   --install-extension ~/Downloads/tcl-lsp-vscode-<v>-universal.vsix
code-server --install-extension ~/Downloads/tcl-lsp-vscode-<v>-universal.vsix
```

(Windsurf, Theia, and Gitpod all surface the same drag-and-drop or
"Install from VSIX" entry in their Extensions panel.)

## Sublime Text

Requires Sublime Text 4 (build 4107+).

Install via **Package Control** (Command Palette → **Package Control:
Install Package** → search **TclLsp**), which serves the
`TclLsp.sublime-package` asset attached to each stable release of
`bitwisecook/tcl-lsp`.

For the manual sideload path, download `TclLsp.sublime-package` from
[Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest) and
drop it, name unchanged, into your Installed Packages directory
(pre-release tags ship it as `TclLsp-prerelease.sublime-package`, which
must be renamed to `TclLsp.sublime-package` before it is copied):

```sh
# macOS:   ~/Library/Application Support/Sublime Text/Installed Packages/
# Linux:   ~/.config/sublime-text/Installed Packages/
# Windows: %APPDATA%\Sublime Text\Installed Packages\
```

Syntaxes, snippets and symbol indexing work on their own. For language
server features, install the **LSP** package from Package Control too:
the first Tcl file you open then downloads the `tcl-lsp-server` build for
your platform into LSP's package storage, verified against the release's
`SHA256SUMS`. To use a server you manage yourself, set `server_path`
under **Preferences > Package Settings > TclLsp > LSP Settings**.

The package ships no key bindings — bind its commands yourself
(see [editors/sublime-text/README.md](editors/sublime-text/README.md)).

## JetBrains

Requires IDEA Ultimate 2024.1+ (free editions from 2025.3).
**Settings > Plugins > gear icon > Install Plugin from Disk…**,
select the zip, restart. Configure under **Settings > Tools > Tcl
Language Server**.

## Neovim

Install the server binary at `~/bin/tcl-lsp-server` (see
[The server binary](#the-server-binary)).

`~/.config/nvim/server/tcl_lsp.lua`:

```lua
return {
  cmd = { vim.fn.expand('~/bin/tcl-lsp-server') },
  filetypes = { 'tcl' },
  settings = { tclLsp = { dialect = 'tcl8.6' } },
}
```

`init.lua`:

```lua
vim.filetype.add({ extension = {
  tcl = 'tcl', tk = 'tcl', itcl = 'tcl', tm = 'tcl', tclspec = 'tcl',
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
               '(tcl-mode . ("~/bin/tcl-lsp-server"))))
(add-hook 'tcl-mode-hook #'eglot-ensure)
```

**lsp-mode**:

```elisp
(with-eval-after-load 'lsp-mode
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection
                     '("~/bin/tcl-lsp-server"))
    :activation-fn (lsp-activate-on "tcl")
    :server-id 'tcl-lsp)))
(add-hook 'tcl-mode-hook #'lsp)
```

## Helix

`~/.config/helix/languages.toml`:

```toml
[language-server.tcl-lsp]
command = "/home/you/bin/tcl-lsp-server"

[[language]]
name = "tcl"
scope = "source.tcl"
file-types = ["tcl", "tk", "itcl", "tm", "tclspec", "irul", "irule", "iapp", "iappimpl", "impl"]
language-servers = ["tcl-lsp"]
```

## Zed

Command Palette > **`zed: extensions`** > search **Tcl**. The
extension auto-downloads the LSP server on first use.

## VS Code-compatible editors

These editors load the VS Code `.vsix` unchanged. Download the
`-universal` package, `tcl-lsp-vscode-<v>-universal.vsix` (it bundles
every platform's binary in one file) from
[Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest)
and install with the editor's own CLI.

| Editor | Install command |
|--------|-----------------|
| **Cursor** | `cursor --install-extension ~/Downloads/tcl-lsp-vscode-<v>-universal.vsix` |
| **Windsurf** | `windsurf --install-extension ~/Downloads/tcl-lsp-vscode-<v>-universal.vsix` |
| **VSCodium** | `codium --install-extension ~/Downloads/tcl-lsp-vscode-<v>-universal.vsix` |
| **code-server** (Coder) | Drag-drop into the Extensions side panel, or `code-server --install-extension <path>` |
| **GitHub Codespaces** | Extensions side panel > `...` > **Install from VSIX…** |
| **Gitpod** | Same as Codespaces — open the workspace and install from VSIX |
| **Eclipse Theia** | Extensions side panel > **Install from VSIX…** |

Settings UI, keybindings, and the compiler-explorer / Tk preview
panels behave the same as in VS Code itself. The bundled native
`tcl-lsp-server` binary is used automatically; no Python interpreter
is involved.

## Other LSP-capable editors

These editors have a built-in or third-party generic LSP client.
Install the `tcl-lsp-server` binary somewhere stable (the examples below
use `~/bin/tcl-lsp-server` — see [The server binary](#the-server-binary))
and paste the snippet into the editor's config.

### Vim (classic, non-Neovim)

**vim-lsp** (`prabirshrestha/vim-lsp`) in `~/.vimrc`:

```vim
if executable(expand('~/bin/tcl-lsp-server'))
    augroup tcl_lsp_register
        au!
        au User lsp_setup call lsp#register_server({
            \ 'name': 'tcl-lsp',
            \ 'cmd': {server_info->[expand('~/bin/tcl-lsp-server')]},
            \ 'allowlist': ['tcl'],
            \ 'workspace_config': {'tclLsp': {'dialect': 'tcl8.6'}},
            \ })
    augroup END
endif

au BufRead,BufNewFile *.tcl,*.tk,*.itcl,*.tm,*.irul,*.irule,*.iapp,*.iappimpl,*.impl,*.exp,*.apl,*.test,*.irules,*.expect,*.tmsh,*.tclspec set filetype=tcl
```

**coc.nvim** (`neoclide/coc.nvim`) in `coc-settings.json`
(`:CocConfig`):

```json
{
  "languageserver": {
    "tcl-lsp": {
      "command": "/home/you/bin/tcl-lsp-server",
      "filetypes": ["tcl"],
      "settings": { "tclLsp": { "dialect": "tcl8.6" } }
    }
  }
}
```

`coc.nvim` only starts the server once Vim's `filetype` is already
`tcl`, so add the same extension mapping as the vim-lsp block to
`~/.vimrc` (otherwise `.irul`, `.irule`, `.iapp`, `.iappimpl`, and
`.impl` files won't trigger the server):

```vim
au BufRead,BufNewFile *.tcl,*.tk,*.itcl,*.tm,*.irul,*.irule,*.iapp,*.iappimpl,*.impl,*.exp,*.apl,*.test,*.irules,*.expect,*.tmsh,*.tclspec set filetype=tcl
```

### Kate

Kate ships with a built-in LSP client. Enable it under **Settings >
Configure Kate > Plugins > LSP Client**, then open **Settings >
Configure Kate > LSP Client > User Server Settings** and paste:

```json
{
  "servers": {
    "tcl": {
      "command": ["/home/you/bin/tcl-lsp-server"],
      "rootIndicationFileNames": ["pkgIndex.tcl", ".git"],
      "highlightingModeRegex": "^Tcl/Tk$",
      "settings": { "tclLsp": { "dialect": "tcl8.6" } }
    }
  }
}
```

### Kakoune

Install [`kak-lsp`](https://github.com/kakoune-lsp/kakoune-lsp),
then in `~/.config/kak-lsp/kak-lsp.toml`:

```toml
[language.tcl]
filetypes = ["tcl"]
roots = ["pkgIndex.tcl", ".git"]
command = "/home/you/bin/tcl-lsp-server"
settings_section = "tclLsp"

[language.tcl.settings.tclLsp]
dialect = "tcl8.6"
```

In `~/.config/kak/kakrc`:

```kak
eval %sh{kak-lsp --kakoune -s $kak_session}
hook global WinSetOption filetype=tcl %{ lsp-enable-window }
```

### Notepad++

1. **Plugins > Plugins Admin…** > install **nppLspClient**.
2. **Plugins > nppLspClient > Edit configuration** and add:

```json
{
  "servers": {
    "tcl": {
      "name": "tcl-lsp",
      "executable": "C:\\Users\\you\\tcl-lsp-server.exe",
      "args": "",
      "fileExtensions": [".tcl", ".tk", ".itcl", ".tm", ".irul", ".irule", ".iapp", ".iappimpl", ".impl", ".exp", ".apl", ".test", ".irules", ".expect", ".tmsh", ".tclspec"],
      "initOptions": { "tclLsp": { "dialect": "tcl8.6" } }
    }
  }
}
```

3. Restart Notepad++.

### Geany

Geany 2.0+ bundles `geany-lsp`. Enable it under **Tools > Plugin
Manager > LSP Client**, then edit
`~/.config/geany/plugins/server/lsp.conf`:

```ini
[Tcl]
cmd=/home/you/bin/tcl-lsp-server
use=true
rpc-log=
initialization-options-file=
```

### Lite XL

Install the `lsp` plugin via `lpm install lsp` (or copy from
[lite-xl-plugins](https://github.com/lite-xl/lite-xl-plugins)),
then in `~/.config/lite-xl/init.lua`:

```lua
local lsp = require "plugins.lsp"
lsp.add_server {
  name = "tcl-lsp",
  language = "tcl",
  file_patterns = { "%.tcl$", "%.tk$", "%.itcl$", "%.tm$", "%.irul$", "%.irule$", "%.iapp$", "%.iappimpl$", "%.impl$", "%.exp$", "%.apl$", "%.test$", "%.irules$", "%.expect$", "%.tmsh$", "%.tclspec$" },
  command = { "/home/you/bin/tcl-lsp-server" },
  settings = { tclLsp = { dialect = "tcl8.6" } },
}
```

### micro

Install [`micro-lsp`](https://github.com/AndCake/micro-plugin-lsp)
via `> plugin install lsp`, then in `~/.config/micro/settings.json`:

```json
{
  "lsp.server": "tcl=/home/you/bin/tcl-lsp-server",
  "lsp.formatOnSave": false
}
```

### CudaText

Install **cuda_lsp** via **Plugins > Addons Manager > Install**,
then create `settings/cuda_lsp/tcl.json` inside the CudaText
settings folder:

```json
{
  "lexers": { "Tcl": "tcl" },
  "cmd_unix": ["/home/you/bin/tcl-lsp-server"],
  "cmd_windows": ["C:\\Users\\you\\tcl-lsp-server.exe"],
  "work_dir": "",
  "tcp_port": 0
}
```

### JupyterLab

```sh
pip install jupyterlab-lsp jupyter-lsp
```

Then in `~/.jupyter/jupyter_server_config.py`:

```python
c.LanguageServerManager.language_servers = {
    "tcl-lsp": {
        "version": 2,
        "argv": ["/home/you/bin/tcl-lsp-server"],
        "languages": ["tcl"],
        "mime_types": ["text/x-tcl", "text/tcl"],
        "display_name": "Tcl LSP",
    },
}
```

Restart JupyterLab. `.tcl` files in the file browser now get
diagnostics, completion, and hover.

### Doom Emacs / Spacemacs

Both ship eglot and lsp-mode. Use the snippets in the [Emacs](#emacs)
section above, dropped into:

- **Doom Emacs:** `~/.doom.d/config.el` (then `doom sync`).
- **Spacemacs:** the `dotspacemacs/user-config` function in `~/.spacemacs`.
