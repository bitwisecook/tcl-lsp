# Helix

Helix has built-in LSP support. Add the following to your `languages.toml`
(typically `~/.config/helix/languages.toml`):

## Configuration

```toml
[language-server.tcl-lsp]
command = "uv"
args = ["run", "--directory", "/path/to/tcl-lsp", "--no-dev", "python", "-m", "server"]

# Or with the standalone zipapp:
# command = "python3"
# args = ["/path/to/tcl-lsp-server.pyz"]

[[language]]
name = "tcl"
scope = "source.tcl"
file-types = ["tcl", "tk", "itcl", "tm", "irul", "irule", "iapp", "iappimpl", "impl", "apl", "exp"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
```

## Settings

Pass workspace settings via the `config` key:

```toml
[language-server.tcl-lsp.config.tclLsp]
# Valid dialects: tcl8.4, tcl8.5, tcl8.6, tcl9.0, f5-irules, f5-iapps,
# f5-bigip, f5-tmsh, synopsys-eda-tcl, cadence-eda-tcl, xilinx-eda-tcl,
# intel-quartus-eda-tcl, mentor-eda-tcl, expect
dialect = "tcl8.6"

[language-server.tcl-lsp.config.tclLsp.formatting]
indentSize = 4
maxLineLength = 120
```

## Configurable Settings

tcl-lsp supports per-diagnostic, optimiser, shimmer, and XC diagnostic
toggles. These can be configured via `~/.config/tcl-lsp/config.ini` (XDG
config), which works across all editors. See
[docs/kcs/kcs-xdg-config.md](../../docs/kcs/kcs-xdg-config.md) for the
full reference.
