# Neovim

tcl-lsp works with Neovim's built-in LSP client. No plugin is required.

## Prerequisites

The server is the native `tcl-lsp-server` binary — no Python, interpreter,
or runtime dependencies are required. Download the binary for your platform
from the
[latest release](https://github.com/bitwisecook/tcl-lsp/releases/latest),
or build it from source with `make rust-server` (or
`cargo build -p tcl-lsp-server`, producing `target/release/tcl-lsp-server`).

See the [Installation Guide](../../INSTALL-editors.md) for full details.

Point `cmd` at the binary — either its name (`tcl-lsp-server`) if it is on
your PATH, or an absolute path to it.

## Via nvim-lspconfig (recommended once merged upstream)

Once the tcl-lsp config is merged into
[`neovim/nvim-lspconfig`](https://github.com/neovim/nvim-lspconfig), the
setup is a one-liner:

```lua
require('lspconfig').tcl_lsp.setup({})
```

The config expects the `tcl-lsp-server` binary to be on your PATH.
Download it from the
[latest release](https://github.com/bitwisecook/tcl-lsp/releases/latest)
and drop it somewhere on PATH.

## Neovim 0.11+ (native LSP)

1. Copy `tcl_lsp.lua` to `~/.config/nvim/server/tcl_lsp.lua`.
2. Edit the `cmd` line to point at your server.
3. Register the filetype and enable the server in your `init.lua`:

```lua
vim.filetype.add({
  extension = {
    tcl = 'tcl', tk = 'tcl', itcl = 'tcl', tm = 'tcl', tclspec = 'tcl',
    irul = 'tcl', irule = 'tcl', irules = 'tcl',
    iapp = 'tcl', iappimpl = 'tcl', impl = 'tcl', tmsh = 'tcl',
    apl = 'tcl-apl', exp = 'tcl', expect = 'tcl',
    -- EDA vendor scripts; the server picks the vendor dialect from the
    -- extension (`.globals` is Innovus/Genus, `.do` is ModelSim/Questa —
    -- `do` is a Lua keyword, hence the bracket form).
    globals = 'tcl', qsf = 'tcl', qpf = 'tcl', qip = 'tcl',
    ['do'] = 'tcl', sdc = 'tcl', upf = 'tcl', xdc = 'tcl',
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
      cmd = { '/path/to/tcl-lsp-server' },
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
      cmd  = { '/path/to/tcl-lsp-server' },
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

The eighteen dialect profiles `dialect` accepts: `tcl8.4`, `tcl8.5`,
`tcl8.6`, `tcl9.0`, `tcl9.1`, `f5-irules`, `f5-iapps`, `f5-tmsh`,
`f5-bigip`, `bpf`, `expect`, `spectcl`, `cadence-eda-tcl`,
`intel-quartus-eda-tcl`, `mentor-eda-tcl`, `microchip-libero-eda-tcl`,
`synopsys-eda-tcl`, `xilinx-eda-tcl`.

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

See [docs/design/contracts/xdg-config.md](../../docs/design/contracts/xdg-config.md) for
the full reference.
