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
file-types = ["tcl", "tk", "itcl", "tm", "irul", "irule", "iapp", "iappimpl", "impl"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
```

## Settings

Pass workspace settings via the `config` key:

```toml
[language-server.tcl-lsp.config.tclLsp]
dialect = "tcl8.6"

[language-server.tcl-lsp.config.tclLsp.formatting]
indentSize = 4
maxLineLength = 120
```
