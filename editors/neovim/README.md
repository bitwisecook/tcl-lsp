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

tcl-lsp is not yet in
[`neovim/nvim-lspconfig`](https://github.com/neovim/nvim-lspconfig)'s
built-in server list, so configure it with one of the forms below.

## Neovim 0.11+ (native LSP)

1. Copy `tcl_lsp.lua` to `~/.config/nvim/server/tcl_lsp.lua`.
2. Edit the `cmd` line to point at your server.
3. Register the filetype and enable the server in your `init.lua`:

```lua
vim.filetype.add({
  extension = {
    -- <generated: neovim-extensions>
    -- Tcl
    tcl = 'tcl', tk = 'tcl', itcl = 'tcl', tm = 'tcl', test = 'tcl',
    -- Expect
    exp = 'tcl', expect = 'tcl',
    -- F5 BIG-IP
    scf = 'tcl',
    -- F5 iApps
    iapp = 'tcl', iappimpl = 'tcl', impl = 'tcl',
    -- F5 iRules
    irul = 'tcl', irule = 'tcl', irules = 'tcl',
    -- F5 tmsh Scripts
    tmsh = 'tcl',
    -- SpecTcl
    tclspec = 'tcl',
    -- SslicTcl
    sslictcl = 'tcl',
    -- Cadence Genus / Innovus / Xcelium
    globals = 'tcl',
    -- Intel Quartus Prime
    qsf = 'tcl', qpf = 'tcl', qip = 'tcl',
    -- Siemens Questa / ModelSim
    ['do'] = 'tcl',
    -- Synopsys DC / PrimeTime / ICC2 / Formality
    sdc = 'tcl', upf = 'tcl',
    -- Xilinx Vivado
    xdc = 'tcl',
    -- iApp APL
    apl = 'tcl-apl',
    -- </generated>
  },
  pattern = {
    -- <generated: neovim-shebangs>
    -- Scripts named by their interpreter rather than an extension.
    ['.*'] = {
      function(_, bufnr)
        local first = vim.api.nvim_buf_get_lines(bufnr, 0, 1, false)[1] or ''
        for _, word in ipairs({ 'expect', 'jimsh', 'tclsh', 'wish' }) do
          if first:find('^#!.-%f[%w]' .. vim.pesc(word)) then
            return 'tcl'
          end
        end
      end,
      { priority = -math.huge },
    },
    -- </generated>
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

<!-- <generated: neovim-dialects> -->
`dialect` takes any of `bpf`, `expect`, `f5-bigip`, `f5-iapps`, `f5-irules`,
`f5-tmsh`, `jim`, `spectcl`, `sslictcl`, `tcl8.4`, `tcl8.5`, `tcl8.6`,
`tcl9.0`, `tcl9.1`, `cadence-eda-tcl`, `intel-quartus-eda-tcl`,
`mentor-eda-tcl`, `microchip-libero-eda-tcl`, `synopsys-eda-tcl`, `tk`, and
`xilinx-eda-tcl`.
<!-- </generated> -->

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
