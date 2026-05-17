# Helix

Helix has built-in LSP support. Add the following to your `languages.toml`
(typically `~/.config/helix/languages.toml`).

## Prerequisites

**Python 3.10+** is required. We recommend the latest stable Python
(currently 3.14). Install via [Homebrew](https://docs.brew.sh/Homebrew-and-Python)
(`brew install python@3.14`) or [python.org](https://www.python.org/downloads/).

The `.pyz` zipapp bundles all Python dependencies internally — no
`pip install` is needed. You only need a Python interpreter on your system.

See the [Installation Guide](../../INSTALL-editors.md#python) for
full details on Python setup across platforms.

## Upstream integration (after merge)

Once tcl-lsp is added to
[`helix-editor/helix`](https://github.com/helix-editor/helix)'s default
`languages.toml`, Helix users only need the `tcl-lsp-server.pyz` zipapp
on their PATH — no per-user `languages.toml` edit is required.

## Configuration (until upstream merges)

```toml
[language-server.tcl-lsp]
command = "uv"
args = ["run", "--directory", "/path/to/tcl-lsp", "--no-dev", "python", "-m", "lsp"]

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
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }
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

Settings from the config file are applied as baseline defaults.  Helix
`config` settings in `languages.toml` override the config file — so you
can set shared defaults in the config file and per-project overrides in
Helix.

See [docs/kcs/kcs-xdg-config.md](../../docs/kcs/kcs-xdg-config.md) for
the full reference.
