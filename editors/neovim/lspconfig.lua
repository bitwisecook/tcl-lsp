-- tcl_lsp — nvim-lspconfig configuration
--
-- This file is the canonical config for tcl-lsp under nvim-lspconfig.
-- Drop it into `lua/lspconfig/configs/tcl_lsp.lua` when submitting the
-- one-time PR to https://github.com/neovim/nvim-lspconfig.
--
-- After upstream merge, users install via:
--
--   require('lspconfig').tcl_lsp.setup({})
--
-- and the server is auto-discovered from a `tcl-lsp-server.pyz` zipapp on
-- their PATH (or via the explicit `cmd` override).

local util = require('lspconfig.util')

return {
  default_config = {
    cmd = { 'tcl-lsp-server.pyz' },
    filetypes = {
      'tcl',
      'tk',
      'expect',
      'irule',
      'iapp',
    },
    root_dir = function(fname)
      return util.root_pattern('.git', 'pkgIndex.tcl', 'tclpkg.tcl')(fname)
        or util.path.dirname(fname)
    end,
    single_file_support = true,
    settings = {
      tclLsp = {
        dialect = 'tcl8.6',
      },
    },
  },
  docs = {
    description = [[
https://github.com/bitwisecook/tcl-lsp

Language server for Tcl (8.4–9.0), F5 iRules / iApps / TMSH, and EDA tool
dialects (Synopsys, Cadence, Xilinx, Intel Quartus, Mentor).

Install the standalone zipapp from
https://github.com/bitwisecook/tcl-lsp/releases (single-file Python zipapp;
requires Python 3.10+ on the system PATH). Either rename / symlink it as
`tcl-lsp-server.pyz` on your PATH, or set `cmd` explicitly:

```lua
require('lspconfig').tcl_lsp.setup({
  cmd = { 'python3', '/path/to/tcl-lsp-server.pyz' },
  settings = {
    tclLsp = {
      dialect = 'f5-irules',
    },
  },
})
```

See https://github.com/bitwisecook/tcl-lsp/blob/main/editors/neovim/README.md
for the full settings reference.
]],
  },
}
