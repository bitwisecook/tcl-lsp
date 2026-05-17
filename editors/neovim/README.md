# Neovim

tcl-lsp works with Neovim's built-in LSP client. No plugin is required.

## Prerequisites

**Python 3.10+** is required. We recommend the latest stable Python
(currently 3.14). Install via [Homebrew](https://docs.brew.sh/Homebrew-and-Python)
(`brew install python@3.14`) or [python.org](https://www.python.org/downloads/).

The `.pyz` zipapp bundles all Python dependencies internally — no
`pip install` is needed. You only need a Python interpreter on your system.

See the [Installation Guide](../../INSTALL-editors.md#python) for
full details on Python setup across platforms.

The server needs to be accessible via one of:

```sh
# Option A — run from source (requires uv)
uv run --directory /path/to/tcl-lsp --no-dev python -m lsp

# Option B — standalone zipapp (just needs Python 3.10+)
python3 /path/to/tcl-lsp-server.pyz
```

To point to a specific Python interpreter, use the full path as the first
element of `cmd` in your LSP config (e.g.
`'/opt/homebrew/bin/python3.14'`).

## Via nvim-lspconfig (recommended once merged upstream)

Once the tcl-lsp config is merged into
[`neovim/nvim-lspconfig`](https://github.com/neovim/nvim-lspconfig), the
setup is a one-liner:

```lua
require('lspconfig').tcl_lsp.setup({})
```

The config expects the `tcl-lsp-server.pyz` zipapp to be on your PATH.
Download the zipapp from the
[latest release](https://github.com/bitwisecook/tcl-lsp/releases/latest)
and drop it somewhere on PATH (renamed or symlinked to
`tcl-lsp-server.pyz` so the executable bit is set).

Maintainer note: the upstream submission flow is documented in
[`docs/kcs/kcs-howto-publish-editor-extensions.md`](../../docs/kcs/kcs-howto-publish-editor-extensions.md);
the upstream PR body lives in `editors/neovim/lspconfig.lua`.

## Neovim 0.11+ (native LSP)

1. Copy `tcl_lsp.lua` to `~/.config/nvim/lsp/tcl_lsp.lua`.
2. Edit the `cmd` line to point at your server.
3. Register the filetype and enable the server in your `init.lua`:

```lua
vim.filetype.add({
  extension = {
    tcl = 'tcl', tk = 'tcl', itcl = 'tcl', tm = 'tcl',
    irul = 'tcl', irule = 'tcl', iapp = 'tcl', iappimpl = 'tcl', impl = 'tcl',
    apl = 'tcl-apl', exp = 'tcl',
  },
})

vim.lsp.enable('tcl_lsp')
```

## nvim-lspconfig (Neovim 0.8+)

If you use [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig):

```lua
local lspconfig = require('lspconfig')
local configs   = require('lspconfig.configs')

if not configs.tcl_lsp then
  configs.tcl_lsp = {
    default_config = {
      cmd = { 'uv', 'run', '--directory', '/path/to/tcl-lsp', '--no-dev', 'python', '-m', 'server' },
      filetypes = { 'tcl', 'tcl-apl' },
      root_dir = lspconfig.util.root_pattern('.git'),
      single_file_support = true,
    },
  }
end

lspconfig.tcl_lsp.setup({
  settings = {
    tclLsp = {
      dialect = 'tcl8.6',
    },
  },
})
```

## Manual autocommand (any Neovim with LSP)

```lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'tcl',
  callback = function()
    vim.lsp.start({
      name = 'tcl-lsp',
      cmd  = { 'uv', 'run', '--directory', '/path/to/tcl-lsp', '--no-dev', 'python', '-m', 'server' },
      root_dir = vim.fs.dirname(vim.fs.find({ '.git' }, { upward = true })[1]),
      settings = { tclLsp = { dialect = 'tcl8.6' } },
    })
  end,
})
```

## Bracket matching and auto-pairs

Neovim's built-in `matchparen` plugin highlights matching `{}`, `[]`,
and `()` pairs automatically — no configuration needed.

For auto-closing brackets and quotes as you type, use a plugin such as
[nvim-autopairs](https://github.com/windwp/nvim-autopairs):

```lua
require('nvim-autopairs').setup({})
```

Or with [mini.pairs](https://github.com/echasnovski/mini.pairs):

```lua
require('mini.pairs').setup()
```

Both handle `{}`, `[]`, `()`, and `""` out of the box.

## Settings reference

Settings are sent under the `tclLsp` namespace. Key options:

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `dialect` | string | `tcl8.6` | Language dialect |
| `extraCommands` | string[] | `[]` | Custom command names to treat as known |
| `libraryPaths` | string[] | `[]` | Paths to scan for Tcl packages |
| `formatting.indentSize` | integer | `4` | Spaces per indent level |
| `formatting.indentStyle` | string | `spaces` | `spaces` or `tabs` |
| `formatting.braceStyle` | string | `k_and_r` | `k_and_r` |
| `formatting.maxLineLength` | integer | `120` | Maximum line length |

See the top-level README for the full list of formatting, diagnostic, and optimiser settings.

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

Settings from the config file are applied as baseline defaults.  Neovim
`settings` passed via `lspconfig.setup()` or `vim.lsp.start()` override
the config file — so you can set shared defaults in the config file and
project-specific overrides in your Neovim config.

Use the `tcl-lsp.exportConfig` command via `workspace/executeCommand` to
write current settings to the config file.

See [docs/kcs/kcs-xdg-config.md](../../docs/kcs/kcs-xdg-config.md) for
the full reference.
