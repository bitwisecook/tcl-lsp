-- tcl-lsp: Neovim LSP configuration
--
-- Copy this file to ~/.config/nvim/lsp/tcl_lsp.lua  (Neovim 0.11+)
-- then enable with:   vim.lsp.enable('tcl_lsp')
--
-- For older Neovim or nvim-lspconfig, see README.md.

return {
  -- Native Rust server (default). Build it with `make rust-server` (or
  -- `cargo build -p tcl-lsp-server`) and point at the binary:
  cmd = { '/path/to/tcl-lsp/target/release/tcl-lsp-server' },

  filetypes = { 'tcl', 'tcl-apl' },
  root_markers = { '.git' },
  single_file_support = true,

  settings = {
    tclLsp = {
      -- <generated: neovim-dialects>
      -- Valid dialects: bpf, expect, f5-bigip, f5-iapps, f5-irules, f5-tmsh,
      -- jim, spectcl, sslictcl, tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1,
      -- cadence-eda-tcl, intel-quartus-eda-tcl, mentor-eda-tcl,
      -- microchip-libero-eda-tcl, synopsys-eda-tcl, tk, xilinx-eda-tcl
      -- </generated>
      dialect = 'tcl8.6',
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
