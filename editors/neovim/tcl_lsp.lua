-- tcl-lsp: Neovim LSP configuration
--
-- Copy this file to ~/.config/nvim/lsp/tcl_lsp.lua  (Neovim 0.11+)
-- then enable with:   vim.lsp.enable('tcl_lsp')
--
-- For older Neovim or nvim-lspconfig, see README.md.

return {
  cmd = { 'uv', 'run', '--directory', '/path/to/tcl-lsp', '--no-dev', 'python', '-m', 'server' },
  -- Alternative: use the standalone zipapp (no uv required):
  -- cmd = { 'python3', '/path/to/tcl-lsp-server.pyz' },

  filetypes = { 'tcl' },
  root_markers = { '.git' },
  single_file_support = true,

  settings = {
    tclLsp = {
      dialect = 'tcl8.6',       -- tcl8.4 | tcl8.5 | tcl8.6 | tcl9.0 | f5-irules | f5-iapps | eda-tools
      extraCommands = {},
      libraryPaths = {},

      formatting = {
        indentSize = 4,
        indentStyle = 'spaces',   -- spaces | tabs
        braceStyle = 'k_and_r',
        maxLineLength = 120,
      },
    },
  },
}
