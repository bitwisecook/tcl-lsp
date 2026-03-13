# Neovim

tcl-lsp works with Neovim's built-in LSP client. No plugin is required.

## Prerequisites

The server needs to be accessible via one of:

```sh
# Option A — run from source (requires uv)
uv run --directory /path/to/tcl-lsp --no-dev python -m server

# Option B — standalone zipapp (just needs Python 3.10+)
python3 /path/to/tcl-lsp-server.pyz
```

## Neovim 0.11+ (native LSP)

1. Copy `tcl_lsp.lua` to `~/.config/nvim/lsp/tcl_lsp.lua`.
2. Edit the `cmd` line to point at your server.
3. Register the filetype and enable the server in your `init.lua`:

```lua
vim.filetype.add({
  extension = {
    tcl = 'tcl', tk = 'tcl', itcl = 'tcl', tm = 'tcl',
    irul = 'tcl', irule = 'tcl', iapp = 'tcl', iappimpl = 'tcl', impl = 'tcl',
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
      filetypes = { 'tcl' },
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
